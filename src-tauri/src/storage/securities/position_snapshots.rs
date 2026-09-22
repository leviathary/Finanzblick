//! Vergleicht und speichert anbieterneutrale Depotbestände mit datierten Mengen.
//! Mengenänderungen werden datiert fortgeschrieben; nicht mehr enthaltene Positionen erhalten am Snapshot-Datum die Menge null.

use crate::domain::securities::position_snapshots::{
    parse_quantity, quantity_string, PositionSnapshot, PositionSnapshotRow, ScaledQuantity,
    SnapshotScope,
};
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::securities::positions::rebuild_daily_valuations;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSnapshotChange {
    pub position_id: Option<i64>,
    pub symbol: String,
    pub market_symbol: String,
    pub previous_quantity: Option<String>,
    pub quantity: String,
    pub action: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSnapshotPreview {
    #[serde(flatten)]
    pub snapshot: PositionSnapshot,
    pub changes: Vec<PositionSnapshotChange>,
    pub already_imported: bool,
    pub eligible_account_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePositionSnapshotResult {
    pub import_id: i64,
    pub created_positions: usize,
    pub updated_positions: usize,
    pub zeroed_positions: usize,
    pub unchanged_positions: usize,
    pub duplicate: bool,
}

#[derive(Clone, Debug)]
struct ExistingPosition {
    id: i64,
    instrument_id: Option<i64>,
    symbol: Option<String>,
    asset_type: String,
    holding_end_date: Option<String>,
    quantity: Option<ScaledQuantity>,
}

pub(crate) fn preview(
    storage: &Storage,
    account_id: Option<i64>,
    snapshot: PositionSnapshot,
    source_hash: &str,
    normalize: &dyn Fn(&str) -> String,
) -> Result<PositionSnapshotPreview, String> {
    snapshot.validate()?;
    let connection = storage.connect().map_err(db_error)?;
    let eligible_account_ids = eligible_accounts(&connection, &snapshot, normalize)?;
    let already_imported = account_id
        .map(|id| duplicate_result(&connection, id, source_hash))
        .transpose()?
        .flatten()
        .is_some();
    let changes = if let Some(account_id) = account_id {
        if !eligible_account_ids.contains(&account_id) {
            return Err("Bitte ein passendes aktives Anlagekonto auswählen.".into());
        }
        if already_imported || snapshot.snapshot_date.is_none() {
            Vec::new()
        } else {
            validate_snapshot_order(&connection, account_id, snapshot.date()?)?;
            changes_from(&connection, account_id, &snapshot)?
        }
    } else {
        Vec::new()
    };
    Ok(PositionSnapshotPreview {
        snapshot,
        changes,
        already_imported,
        eligible_account_ids,
    })
}

pub(crate) fn save(
    storage: &Storage,
    account_id: i64,
    snapshot: PositionSnapshot,
    source_name: &str,
    source_hash: &str,
    normalize: &dyn Fn(&str) -> String,
) -> Result<SavePositionSnapshotResult, String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let result = persist(
        &mut connection,
        account_id,
        &snapshot,
        source_name,
        source_hash,
        normalize,
    )?;
    rebuild_daily_valuations(storage)?;
    Ok(result)
}

fn persist(
    connection: &mut Connection,
    account_id: i64,
    snapshot: &PositionSnapshot,
    source_name: &str,
    source_hash: &str,
    normalize: &dyn Fn(&str) -> String,
) -> Result<SavePositionSnapshotResult, String> {
    snapshot.validate()?;
    let snapshot_date = snapshot.date()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(db_error)?;
    if !eligible_accounts(&transaction, snapshot, normalize)?.contains(&account_id) {
        return Err("Bitte ein passendes aktives Anlagekonto auswählen.".into());
    }
    if let Some(result) = duplicate_result(&transaction, account_id, source_hash)? {
        return Ok(result);
    }
    validate_snapshot_order(&transaction, account_id, snapshot_date)?;
    let changes = changes_from(&transaction, account_id, snapshot)?;
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO position_snapshot_imports(account_id,provider,source_name,source_format,source_hash,snapshot_date,scope,account_reference,imported_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![account_id,snapshot.provider,source_name,snapshot.format,source_hash,snapshot_date,
            if snapshot.scope == SnapshotScope::FullPortfolio { "fullPortfolio" } else { "partial" },
            snapshot.account_reference,now],
    ).map_err(db_error)?;
    let import_id = transaction.last_insert_rowid();
    let incoming = snapshot
        .positions
        .iter()
        .map(|position| (position.market_symbol.as_str(), position))
        .collect::<BTreeMap<_, _>>();
    let mut counts = BTreeMap::from([
        ("new", 0usize),
        ("update", 0usize),
        ("zero", 0usize),
        ("unchanged", 0usize),
    ]);
    for change in &changes {
        let position_id = if change.action == "new" {
            let incoming_position = incoming
                .get(change.market_symbol.as_str())
                .ok_or("Neue Position fehlt im eingelesenen Bestand.")?;
            create_position(
                &transaction,
                account_id,
                incoming_position,
                snapshot_date,
                &now,
            )?
        } else {
            change
                .position_id
                .ok_or("Position konnte nicht zugeordnet werden.")?
        };
        if let Some(row) = incoming.get(change.market_symbol.as_str()) {
            let instrument_id: i64 = transaction.query_row(
                "SELECT l.instrument_id FROM portfolio_positions p JOIN instrument_listings l ON l.id=p.listing_id WHERE p.id=?1",
                [position_id], |record| record.get(0),
            ).map_err(db_error)?;
            store_identifiers(&transaction, instrument_id, row)?;
        }
        let quantity = parse_quantity(&change.quantity)?;
        {
            transaction
                .execute(
                    "INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at)
                     VALUES(?1,?2,?3,?4,'position_snapshot',?5)
                     ON CONFLICT(position_id,valid_from) DO UPDATE SET
                       quantity_amount=excluded.quantity_amount,quantity_scale=excluded.quantity_scale,
                       source=excluded.source,recorded_at=excluded.recorded_at",
                    params![position_id, snapshot_date, quantity.amount, quantity.scale, now],
                )
                .map_err(db_error)?;
        }
        if quantity.amount == 0 {
            transaction
                .execute(
                    "UPDATE portfolio_positions SET holding_end_date=?1,updated_at=?2 WHERE id=?3",
                    params![snapshot_date, now, position_id],
                )
                .map_err(db_error)?;
        } else if incoming.contains_key(change.market_symbol.as_str()) && quantity.amount > 0 {
            transaction
                .execute(
                    "UPDATE portfolio_positions SET holding_end_date=NULL,updated_at=?1 WHERE id=?2",
                    params![now, position_id],
                )
                .map_err(db_error)?;
        }
        transaction
            .execute(
                "INSERT INTO position_snapshot_import_rows(import_id,position_id,symbol,quantity_amount,quantity_scale,action)
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    import_id,
                    position_id,
                    change.market_symbol,
                    quantity.amount,
                    quantity.scale,
                    change.action
                ],
            )
            .map_err(db_error)?;
        let count = counts
            .get_mut(change.action.as_str())
            .ok_or("Unbekannte Abgleichsaktion.")?;
        *count += 1;
    }
    transaction.commit().map_err(db_error)?;
    Ok(SavePositionSnapshotResult {
        import_id,
        created_positions: counts["new"],
        updated_positions: counts["update"],
        zeroed_positions: counts["zero"],
        unchanged_positions: counts["unchanged"],
        duplicate: false,
    })
}

fn duplicate_result(
    connection: &Connection,
    account_id: i64,
    source_hash: &str,
) -> Result<Option<SavePositionSnapshotResult>, String> {
    connection
        .query_row(
            "SELECT i.id,
              SUM(CASE WHEN r.action='new' THEN 1 ELSE 0 END),
              SUM(CASE WHEN r.action='update' THEN 1 ELSE 0 END),
              SUM(CASE WHEN r.action='zero' THEN 1 ELSE 0 END),
              SUM(CASE WHEN r.action='unchanged' THEN 1 ELSE 0 END)
             FROM (
               SELECT id,account_id,source_hash,0 AS legacy FROM position_snapshot_imports
               UNION ALL SELECT id,account_id,source_hash,1 FROM swissquote_position_imports
             ) i LEFT JOIN (
               SELECT import_id,action,0 AS legacy FROM position_snapshot_import_rows
               UNION ALL SELECT import_id,action,1 FROM swissquote_position_import_rows
             ) r ON r.import_id=i.id AND r.legacy=i.legacy
             WHERE i.account_id=?1 AND i.source_hash=?2 GROUP BY i.id,i.legacy",
            params![account_id, source_hash],
            |row| {
                Ok(SavePositionSnapshotResult {
                    import_id: row.get(0)?,
                    created_positions: row.get::<_, Option<usize>>(1)?.unwrap_or(0),
                    updated_positions: row.get::<_, Option<usize>>(2)?.unwrap_or(0),
                    zeroed_positions: row.get::<_, Option<usize>>(3)?.unwrap_or(0),
                    unchanged_positions: row.get::<_, Option<usize>>(4)?.unwrap_or(0),
                    duplicate: true,
                })
            },
        )
        .optional()
        .map_err(db_error)
}

fn eligible_accounts(
    connection: &Connection,
    snapshot: &PositionSnapshot,
    normalize: &dyn Fn(&str) -> String,
) -> Result<Vec<i64>, String> {
    let mut query = connection
        .prepare(
            "SELECT a.id,a.account_type,a.external_reference FROM accounts a
         JOIN institutions i ON i.id=a.institution_id
         WHERE a.is_active=1 AND i.provider_key=?1",
        )
        .map_err(db_error)?;
    let accounts = query
        .query_map([&snapshot.provider], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    // A provider may express a shared trading reference on its associated cash account.
    let shared_reference_matches = snapshot
        .account_reference
        .as_deref()
        .is_none_or(|reference| {
            accounts.iter().any(|(_, _, saved)| {
                saved
                    .as_deref()
                    .is_some_and(|v| normalize(v) == normalize(reference))
            })
        });
    Ok(accounts
        .into_iter()
        .filter(|(_, kind, saved)| {
            let matches = snapshot
                .account_reference
                .as_deref()
                .is_none_or(|reference| {
                    if snapshot.reference_is_shared {
                        shared_reference_matches
                    } else {
                        saved
                            .as_deref()
                            .is_some_and(|value| normalize(value) == normalize(reference))
                    }
                });
            matches && crate::domain::banking::accounts::supports_positions(kind)
        })
        .map(|(id, _, _)| id)
        .collect())
}

fn validate_snapshot_order(
    connection: &Connection,
    account_id: i64,
    snapshot_date: &str,
) -> Result<(), String> {
    let latest = connection
        .query_row(
            "SELECT MAX(snapshot_date) FROM (SELECT snapshot_date FROM position_snapshot_imports WHERE account_id=?1 UNION ALL SELECT snapshot_date FROM swissquote_position_imports WHERE account_id=?1 UNION ALL SELECT q.valid_from FROM position_quantities q JOIN portfolio_positions p ON p.id=q.position_id WHERE p.account_id=?1)",
            [account_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(db_error)?;
    if latest.as_deref().is_some_and(|date| date > snapshot_date) {
        return Err("Der Stichtag liegt vor einem bereits gespeicherten Mengenstand. Importiere Positionsbestände in zeitlicher Reihenfolge.".into());
    }
    Ok(())
}

fn changes_from(
    connection: &Connection,
    account_id: i64,
    snapshot: &PositionSnapshot,
) -> Result<Vec<PositionSnapshotChange>, String> {
    let existing = existing_positions(connection, account_id)?;
    let mut by_symbol = BTreeMap::new();
    for position in existing.iter().filter(|position| position.symbol.is_some()) {
        let key = canonical_market_symbol(
            position.symbol.as_deref().unwrap(),
            &position.asset_type,
            None,
        );
        if by_symbol.insert(key.clone(), position).is_some() {
            return Err(format!("Das Depot enthält mehrere Positionen für {key}. Bitte bereinige die Zuordnung zuerst."));
        }
    }
    let mut seen = BTreeSet::new();
    let mut changes = Vec::new();
    for row in &snapshot.positions {
        let key = canonical_market_symbol(
            &row.market_symbol,
            &row.asset_type,
            Some(&row.quote_currency),
        );
        let instrument = find_instrument(connection, row)?;
        let matched = existing
            .iter()
            .filter(|position| instrument.is_some() && position.instrument_id == instrument)
            .collect::<Vec<_>>();
        if matched.len() > 1 {
            return Err(
                "Das Depot enthält mehrere Positionen für dieselbe Instrumentkennung.".into(),
            );
        }
        let position = matched
            .first()
            .copied()
            .or_else(|| by_symbol.get(&key).copied());
        let quantity = parse_quantity(&row.quantity)?;
        if let Some(position) = position {
            if !seen.insert(position.id) {
                return Err("Mehrere Importzeilen verweisen auf dieselbe Depotposition.".into());
            }
            let unchanged =
                position.quantity == Some(quantity) && position.holding_end_date.is_none();
            changes.push(PositionSnapshotChange {
                position_id: Some(position.id),
                symbol: row.symbol.clone(),
                market_symbol: row.market_symbol.clone(),
                previous_quantity: position.quantity.map(quantity_string),
                quantity: row.quantity.clone(),
                action: if unchanged { "unchanged" } else { "update" }.into(),
            });
        } else {
            changes.push(PositionSnapshotChange {
                position_id: None,
                symbol: row.symbol.clone(),
                market_symbol: row.market_symbol.clone(),
                previous_quantity: None,
                quantity: row.quantity.clone(),
                action: "new".into(),
            });
        }
    }
    for position in existing {
        if snapshot.scope != SnapshotScope::FullPortfolio {
            continue;
        }
        let Some(symbol) = position.symbol else {
            return Err("Der vollständige Depotbestand enthält eine bestehende Position ohne Instrumentzuordnung. Bitte diese zuerst zuordnen.".into());
        };
        if seen.contains(&position.id) {
            continue;
        }
        let is_already_closed = position
            .quantity
            .is_some_and(|quantity| quantity.amount == 0)
            && position.holding_end_date.is_some();
        changes.push(PositionSnapshotChange {
            position_id: Some(position.id),
            symbol: display_symbol(&symbol, &position.asset_type),
            market_symbol: symbol,
            previous_quantity: position.quantity.map(quantity_string),
            quantity: "0".into(),
            action: if is_already_closed {
                "unchanged"
            } else {
                "zero"
            }
            .into(),
        });
    }
    Ok(changes)
}

fn existing_positions(
    connection: &Connection,
    account_id: i64,
) -> Result<Vec<ExistingPosition>, String> {
    let mut query = connection
        .prepare(
            "SELECT p.id,COALESCE(l.market_symbol,
               (SELECT ii.identifier FROM instrument_identifiers ii WHERE ii.instrument_id=l.instrument_id AND ii.identifier_type='ticker' LIMIT 1)),
               p.asset_type,p.holding_end_date,
               (SELECT q.quantity_amount FROM position_quantities q WHERE q.position_id=p.id ORDER BY q.valid_from DESC,q.id DESC LIMIT 1),
               (SELECT q.quantity_scale FROM position_quantities q WHERE q.position_id=p.id ORDER BY q.valid_from DESC,q.id DESC LIMIT 1),
               l.instrument_id
             FROM portfolio_positions p LEFT JOIN instrument_listings l ON l.id=p.listing_id
             WHERE p.account_id=?1 ORDER BY p.id",
        )
        .map_err(db_error)?;
    let positions = query
        .query_map([account_id], |row| {
            Ok(ExistingPosition {
                id: row.get(0)?,
                instrument_id: row.get(6)?,
                symbol: row.get(1)?,
                asset_type: row.get(2)?,
                holding_end_date: row.get(3)?,
                quantity: row
                    .get::<_, Option<i64>>(4)?
                    .zip(row.get::<_, Option<u32>>(5)?)
                    .map(|(amount, scale)| ScaledQuantity { amount, scale }),
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(positions)
}

fn create_position(
    transaction: &Transaction<'_>,
    account_id: i64,
    row: &PositionSnapshotRow,
    snapshot_date: &str,
    now: &str,
) -> Result<i64, String> {
    let existing_instrument_id = find_instrument(transaction, row)?;
    let instrument_id = if let Some(id) = existing_instrument_id {
        id
    } else {
        transaction
            .execute(
                "INSERT INTO instruments(name,asset_type,created_at) VALUES(?1,?2,?3)",
                params![row.symbol, row.asset_type, now],
            )
            .map_err(db_error)?;
        transaction.last_insert_rowid()
    };
    store_identifiers(transaction, instrument_id, row)?;
    let existing_listing_id = transaction
        .query_row(
            "SELECT id FROM instrument_listings WHERE instrument_id=?1 AND market_symbol=?2 LIMIT 1",
            params![instrument_id, row.market_symbol],
            |record| record.get::<_, i64>(0),
        )
        .optional()
        .map_err(db_error)?;
    let listing_id = if let Some(id) = existing_listing_id {
        id
    } else {
        transaction
            .execute(
                "INSERT INTO instrument_listings(instrument_id,market_symbol,quote_currency,preferred_price_source) VALUES(?1,?2,?3,?4)",
                params![instrument_id, row.market_symbol, (!row.quote_currency.is_empty()).then_some(row.quote_currency.as_str()), row.price_source],
            )
            .map_err(db_error)?;
        transaction.last_insert_rowid()
    };
    transaction
        .execute(
            "INSERT INTO portfolio_positions(account_id,listing_id,label,asset_type,holding_start_date,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?6)",
            params![account_id, listing_id, row.symbol, row.asset_type, snapshot_date, now],
        )
        .map_err(db_error)?;
    Ok(transaction.last_insert_rowid())
}

fn row_identifiers(row: &PositionSnapshotRow) -> Vec<(&str, &str)> {
    [
        ("isin", row.isin.as_deref()),
        ("valor", row.valor.as_deref()),
        ("ticker", Some(row.market_symbol.as_str())),
    ]
    .into_iter()
    .filter_map(|(kind, value)| value.map(|v| (kind, v)))
    .collect()
}

fn find_instrument(db: &Connection, row: &PositionSnapshotRow) -> Result<Option<i64>, String> {
    let mut ids = BTreeSet::new();
    for (kind, value) in row_identifiers(row) {
        let found = db.query_row(
            "SELECT instrument_id FROM instrument_identifiers WHERE identifier_type=?1 AND identifier=?2",
            params![kind,value], |r| r.get::<_,i64>(0),
        ).optional().map_err(db_error)?;
        if let Some(id) = found {
            ids.insert(id);
        }
    }
    if ids.len() > 1 {
        return Err("Die Instrumentkennungen verweisen auf unterschiedliche Wertpapiere.".into());
    }
    let found = ids.into_iter().next();
    if let Some(id) = found {
        validate_identifiers(db, id, row)?;
    }
    Ok(found)
}

fn validate_identifiers(
    db: &Connection,
    instrument: i64,
    row: &PositionSnapshotRow,
) -> Result<(), String> {
    for (kind, value) in row_identifiers(row)
        .into_iter()
        .filter(|(kind, _)| *kind != "ticker")
    {
        let conflict: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM instrument_identifiers WHERE instrument_id=?1 AND identifier_type=?2 AND identifier<>?3)",
            params![instrument,kind,value], |record| record.get(0),
        ).map_err(db_error)?;
        if conflict {
            return Err(
                "Die Instrumentkennungen verweisen auf unterschiedliche Wertpapiere.".into(),
            );
        }
    }
    Ok(())
}

fn store_identifiers(
    db: &Connection,
    instrument: i64,
    row: &PositionSnapshotRow,
) -> Result<(), String> {
    validate_identifiers(db, instrument, row)?;
    for (kind, value) in row_identifiers(row) {
        let owner = db.query_row(
            "SELECT instrument_id FROM instrument_identifiers WHERE identifier_type=?1 AND identifier=?2",
            params![kind,value], |record| record.get::<_,i64>(0),
        ).optional().map_err(db_error)?;
        if owner.is_some_and(|id| id != instrument) {
            return Err(
                "Die Instrumentkennungen verweisen auf unterschiedliche Wertpapiere.".into(),
            );
        }
        db.execute(
            "INSERT OR IGNORE INTO instrument_identifiers(instrument_id,identifier_type,identifier) VALUES(?1,?2,?3)",
            params![instrument,kind,value],
        ).map_err(db_error)?;
    }
    Ok(())
}

fn canonical_market_symbol(symbol: &str, asset_type: &str, quote_currency: Option<&str>) -> String {
    let symbol = symbol.trim().to_ascii_uppercase();
    if asset_type == "crypto" && !symbol.contains('-') {
        format!(
            "{symbol}-{}",
            quote_currency
                .filter(|value| !value.is_empty())
                .unwrap_or("USD")
        )
    } else {
        symbol
    }
}

fn display_symbol(symbol: &str, asset_type: &str) -> String {
    if asset_type == "crypto" {
        symbol.strip_suffix("-USD").unwrap_or(symbol).to_string()
    } else {
        symbol.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::securities::position_snapshots::normalize_reference;
    use crate::storage::initialize_schema;

    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        initialize_schema(&db).unwrap();
        db.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES
             (1,'swissquote','Swissquote','bank','now'),(2,'example_broker','Example Broker','bank','now');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,external_reference,created_at) VALUES
             (1,1,'Depot','manual_asset','CHF','111','now'),
             (2,2,'Depot','manual_asset','CHF','222','now'),
             (3,2,'Zweites Depot','manual_asset','CHF','333','now');"
        ).unwrap();
        db
    }

    fn row(symbol: &str, kind: &str, quantity: &str) -> PositionSnapshotRow {
        PositionSnapshotRow {
            symbol: symbol.into(),
            market_symbol: symbol.into(),
            isin: None,
            valor: None,
            quantity: quantity.into(),
            category: kind.into(),
            asset_type: kind.into(),
            quote_currency: "CHF".into(),
            price_source: Some("yahoo".into()),
            source_row: 1,
        }
    }

    fn snapshot(
        provider: &str,
        date: &str,
        positions: Vec<PositionSnapshotRow>,
    ) -> PositionSnapshot {
        PositionSnapshot {
            provider: provider.into(),
            format: "TEST".into(),
            snapshot_date: Some(date.into()),
            scope: SnapshotScope::FullPortfolio,
            account_reference: None,
            reference_is_shared: false,
            positions,
            warnings: vec![],
        }
    }

    fn save_test(
        db: &mut Connection,
        account: i64,
        snapshot: &PositionSnapshot,
        hash: &str,
    ) -> SavePositionSnapshotResult {
        persist(db, account, snapshot, "test", hash, &normalize_reference).unwrap()
    }

    fn quantity_at(db: &Connection, account: i64, symbol: &str, date: &str) -> Option<String> {
        db.query_row(
            "SELECT q.quantity_amount,q.quantity_scale FROM position_quantities q
             JOIN portfolio_positions p ON p.id=q.position_id JOIN instrument_listings l ON l.id=p.listing_id
             WHERE p.account_id=?1 AND l.market_symbol=?2 AND q.valid_from<=?3 ORDER BY q.valid_from DESC LIMIT 1",
            params![account,symbol,date],
            |r| Ok(ScaledQuantity { amount:r.get(0)?,scale:r.get(1)? }),
        ).optional().unwrap().map(quantity_string)
    }

    #[test]
    fn depot_accounts_are_import_targets_for_each_provider_but_cash_is_not() {
        let mut db = database();
        db.execute("UPDATE accounts SET account_type='portfolio' WHERE id IN (1,2)", []).unwrap();
        db.execute("UPDATE accounts SET account_type='cash' WHERE id=3", []).unwrap();
        for (id, provider, reference) in [(1, "swissquote", "111"), (2, "example_broker", "222")] {
            let mut input = snapshot(provider, "2026-01-02", vec![row("AAA", "stock", "2")]);
            input.account_reference = Some(reference.into());
            assert_eq!(eligible_accounts(&db, &input, &normalize_reference).unwrap(), vec![id]);
            assert_eq!(save_test(&mut db, id, &input, "depot").created_positions, 1);
            assert_eq!(quantity_at(&db, id, "AAA", "2026-01-01"), None);
            assert_eq!(quantity_at(&db, id, "AAA", "2026-01-02").as_deref(), Some("2"));
            input.account_reference = Some("wrong".into());
            assert!(eligible_accounts(&db, &input, &normalize_reference).unwrap().is_empty());
            db.execute("UPDATE accounts SET is_active=0 WHERE id=?1", [id]).unwrap();
            input.account_reference = Some(reference.into());
            assert!(eligible_accounts(&db, &input, &normalize_reference).unwrap().is_empty());
        }
        let input = snapshot("example_broker", "2026-01-02", vec![row("AAA", "stock", "2")]);
        assert!(persist(&mut db, 3, &input, "test", "cash", &normalize_reference).is_err());
    }

    #[test]
    fn same_workflow_for_two_providers_and_effective_dates() {
        let mut db = database();
        for (account, provider) in [(1, "swissquote"), (2, "example_broker")] {
            let first = snapshot(
                provider,
                "2026-01-02",
                vec![row("AAA", "stock", "1.123456789")],
            );
            assert_eq!(
                save_test(&mut db, account, &first, "first").created_positions,
                1
            );
            assert_eq!(quantity_at(&db, account, "AAA", "2026-01-01"), None);
            assert_eq!(
                quantity_at(&db, account, "AAA", "2026-01-02").as_deref(),
                Some("1.123456789")
            );
            let next = snapshot(provider, "2026-02-01", vec![row("AAA", "stock", "2")]);
            assert_eq!(
                save_test(&mut db, account, &next, "next").updated_positions,
                1
            );
            assert_eq!(
                quantity_at(&db, account, "AAA", "2026-01-31").as_deref(),
                Some("1.123456789")
            );
            assert_eq!(
                quantity_at(&db, account, "AAA", "2026-02-01").as_deref(),
                Some("2")
            );
            assert!(save_test(&mut db, account, &next, "next").duplicate);
            let unchanged = snapshot(provider, "2026-03-01", next.positions);
            assert_eq!(
                save_test(&mut db, account, &unchanged, "unchanged").unchanged_positions,
                1
            );
            assert_eq!(
                db.query_row(
                    "SELECT COUNT(*) FROM position_snapshot_imports WHERE account_id=?1",
                    [account],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                3
            );
            assert!(persist(
                &mut db,
                account,
                &first,
                "test",
                "backdated",
                &normalize_reference
            )
            .is_err());
        }
    }

    #[test]
    fn full_snapshot_closes_missing_asset_classes_and_empty_portfolios() {
        let mut db = database();
        let first = snapshot(
            "example_broker",
            "2026-01-01",
            vec![row("AAA", "stock", "2"), row("BOND", "bond", "3")],
        );
        save_test(&mut db, 2, &first, "first");
        let stocks = snapshot(
            "example_broker",
            "2026-02-01",
            vec![row("AAA", "stock", "2")],
        );
        assert_eq!(save_test(&mut db, 2, &stocks, "stocks").zeroed_positions, 1);
        assert_eq!(
            quantity_at(&db, 2, "BOND", "2026-02-01").as_deref(),
            Some("0")
        );
        let empty = snapshot("example_broker", "2026-03-01", vec![]);
        assert_eq!(save_test(&mut db, 2, &empty, "empty").zeroed_positions, 1);
        assert_eq!(
            quantity_at(&db, 2, "AAA", "2026-03-01").as_deref(),
            Some("0")
        );
        let again = snapshot(
            "example_broker",
            "2026-04-01",
            vec![row("AAA", "stock", "4")],
        );
        save_test(&mut db, 2, &again, "again");
        assert_eq!(
            quantity_at(&db, 2, "AAA", "2026-03-20").as_deref(),
            Some("0")
        );
        assert_eq!(
            quantity_at(&db, 2, "AAA", "2026-04-01").as_deref(),
            Some("4")
        );
    }

    #[test]
    fn partial_snapshot_preserves_other_positions_and_accounts_are_validated() {
        let mut db = database();
        let mut first = snapshot(
            "example_broker",
            "2026-01-01",
            vec![row("AAA", "stock", "2"), row("BOND", "bond", "3")],
        );
        first.account_reference = Some("222".into());
        assert_eq!(
            eligible_accounts(&db, &first, &normalize_reference).unwrap(),
            vec![2]
        );
        assert!(persist(&mut db, 1, &first, "test", "wrong", &normalize_reference).is_err());
        assert!(persist(&mut db, 3, &first, "test", "wrong", &normalize_reference).is_err());
        save_test(&mut db, 2, &first, "first");
        let mut partial = snapshot(
            "example_broker",
            "2026-02-01",
            vec![row("AAA", "stock", "5")],
        );
        partial.scope = SnapshotScope::Partial;
        assert_eq!(
            save_test(&mut db, 2, &partial, "partial").zeroed_positions,
            0
        );
        assert_eq!(
            quantity_at(&db, 2, "BOND", "2026-02-01").as_deref(),
            Some("3")
        );
        partial.snapshot_date = None;
        assert!(persist(
            &mut db,
            2,
            &partial,
            "test",
            "missing",
            &normalize_reference
        )
        .is_err());
        partial.snapshot_date = Some("2026-02-30".into());
        assert!(persist(
            &mut db,
            2,
            &partial,
            "test",
            "invalid",
            &normalize_reference
        )
        .is_err());
    }

    #[test]
    fn matches_isin_across_ticker_changes_and_rolls_back_invalid_imports() {
        let mut db = database();
        let mut first = snapshot(
            "example_broker",
            "2026-01-01",
            vec![row("OLD", "stock", "1")],
        );
        first.positions[0].isin = Some("CH0000000001".into());
        save_test(&mut db, 2, &first, "first");
        let mut next = snapshot(
            "example_broker",
            "2026-02-01",
            vec![row("NEW", "stock", "2")],
        );
        next.positions[0].isin = Some("CH0000000001".into());
        let result = save_test(&mut db, 2, &next, "next");
        assert_eq!(result.updated_positions, 1);
        assert_eq!(result.created_positions, 0);
        assert_eq!(result.zeroed_positions, 0);
        // Failure after writing the import header must roll back both header and new instruments.
        db.execute_batch("CREATE TRIGGER fail_snapshot_row BEFORE INSERT ON position_snapshot_import_rows BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        let bad = snapshot(
            "example_broker",
            "2026-03-01",
            vec![row("FAIL", "stock", "1")],
        );
        assert!(persist(&mut db, 2, &bad, "test", "bad", &normalize_reference).is_err());
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM position_snapshot_imports WHERE source_hash='bad'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM instruments WHERE name='FAIL'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn legacy_import_evidence_is_read_without_migration() {
        let mut db = database();
        db.execute("INSERT INTO swissquote_position_imports(account_id,source_name,source_format,source_hash,snapshot_date,imported_at) VALUES(1,'old','XLSX','old-hash','2026-02-01','now')",[]).unwrap();
        let same = snapshot("swissquote", "2026-02-01", vec![row("AAA", "stock", "1")]);
        assert!(save_test(&mut db, 1, &same, "old-hash").duplicate);
        assert!(validate_snapshot_order(&db, 1, "2026-01-01").is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM position_snapshot_imports", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM swissquote_position_imports",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn valuation_starts_at_snapshot_and_uses_dated_quantities() {
        let directory = std::env::temp_dir().join(format!(
            "saldonaut-position-snapshot-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        {
            let db = storage.connect().unwrap();
            initialize_schema(&db).unwrap();
            db.execute_batch("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'example_broker','Example','bank','now');
              INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Depot','manual_asset','CHF','now');").unwrap();
        }
        let first = snapshot(
            "example_broker",
            "2026-01-02",
            vec![row("AAA", "stock", "2")],
        );
        save(&storage, 1, first, "first", "first", &normalize_reference).unwrap();
        {
            let db = storage.connect().unwrap();
            db.execute("INSERT INTO instrument_prices(listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) SELECT id,'2026-01-01','eod_close',1000,2,'CHF','yahoo','now' FROM instrument_listings",[]).unwrap();
        }
        let next = snapshot(
            "example_broker",
            "2026-02-01",
            vec![row("AAA", "stock", "3")],
        );
        save(&storage, 1, next, "next", "next", &normalize_reference).unwrap();
        let db = storage.connect().unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM daily_valuations WHERE valuation_date<'2026-01-02'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT value_minor FROM daily_valuations WHERE valuation_date='2026-01-02'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2000
        );
        assert_eq!(
            db.query_row(
                "SELECT value_minor FROM daily_valuations WHERE valuation_date='2026-02-01'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            3000
        );
        drop(db);
        drop(storage);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
