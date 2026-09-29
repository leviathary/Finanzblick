//! Speichert Positionen und berechnet ihre täglichen Bewertungen.
use crate::domain::securities::position_snapshots::parse_quantity;
use crate::infrastructure::market_data;
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::securities::models::{
    DeleteManualValuationRequest, ManualPosition, ManualValuation, ManualValuationRequest,
    PositionQuantityChangeRequest, UpdateManualValuationRequest,
};
use chrono::{Duration, Local, NaiveDate, Utc};
use rusqlite::params;
use rusqlite::OptionalExtension;

pub(crate) fn save_manual_valuation(
    storage: &Storage,
    request: ManualValuationRequest,
) -> Result<(), String> {
    let start = request
        .holding_start_date
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(request.valuation_date.trim());
    let start_day = NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|_| "Das Einstandsdatum ist ungültig.".to_string())?;
    let valuation_date = request.valuation_date.trim();
    let valuation_day = NaiveDate::parse_from_str(valuation_date, "%Y-%m-%d")
        .map_err(|_| "Das Bewertungsdatum ist ungültig.".to_string())?;
    if valuation_day < start_day {
        return Err("Das Bewertungsdatum darf nicht vor dem Einstandsdatum liegen.".into());
    }
    if request.label.trim().is_empty() {
        return Err("Bitte eine Bezeichnung für die Position eingeben.".into());
    }
    if let Some(end) = request
        .holding_end_date
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let end_day = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "Das Verkaufsdatum ist ungültig.".to_string())?;
        if end < start {
            return Err("Das Verkaufsdatum darf nicht vor dem Einstandsdatum liegen.".into());
        }
        if valuation_day > end_day {
            return Err("Das Bewertungsdatum darf nicht nach dem Verkaufsdatum liegen.".into());
        }
    }
    if request
        .quantity
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("Die Menge bzw. Anzahl Einheiten ist ungültig.".into());
    }
    if request
        .exchange_rate
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err("Der Wechselkurs muss grösser als null sein.".into());
    }

    let identifier_type = request
        .identifier_type
        .as_deref()
        .map(str::trim)
        .filter(|value| ["isin", "ticker"].contains(value));
    let identifier = request
        .identifier
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_uppercase);
    if identifier.is_some() && identifier_type.is_none() {
        return Err("Bitte ISIN oder Ticker als Kennungsart auswählen.".into());
    }

    let mut connection = storage.connect().map_err(db_error)?;
    let (account_type, account_currency): (String, String) = connection
        .query_row(
            "SELECT account_type,currency FROM accounts WHERE id=?1",
            [request.account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "Das Konto wurde nicht gefunden.".into(),
            other => db_error(other),
        })?;
    if !crate::domain::banking::accounts::supports_positions(&account_type) {
        return Err(
            "Positionen sind nur für Depots, manuell verwaltete Positionen und Vorsorgekonten möglich.".into(),
        );
    }

    let transaction = connection.transaction().map_err(db_error)?;
    let listing_id = if let (Some(kind), Some(value)) = (identifier_type, identifier.as_deref()) {
        let normalized = market_data::normalize_market_identifier(value);
        let existing_instrument_id = transaction.query_row(
            "SELECT instrument_id FROM instrument_identifiers WHERE identifier_type=?1 AND identifier=?2",
            params![kind, normalized],
            |row| row.get::<_, i64>(0),
        ).optional().map_err(db_error)?;
        let instrument_id = if let Some(id) = existing_instrument_id {
            id
        } else {
            transaction
                .execute(
                    "INSERT INTO instruments(name,asset_type) VALUES(?1,?2)",
                    params![
                        request.label.trim(),
                        request.asset_type.as_deref().unwrap_or("other")
                    ],
                )
                .map_err(db_error)?;
            let id = transaction.last_insert_rowid();
            transaction.execute(
                "INSERT INTO instrument_identifiers(instrument_id,identifier_type,identifier) VALUES(?1,?2,?3)",
                params![id, kind, normalized],
            ).map_err(db_error)?;
            id
        };
        let (exchange_mic, market_symbol) = listing_parts(&normalized);
        let quote_currency = request
            .quote_currency
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_uppercase)
            .or_else(|| exchange_currency(exchange_mic.as_deref()).map(str::to_string));
        let existing = transaction
            .query_row(
                "SELECT id FROM instrument_listings WHERE instrument_id=?1",
                [instrument_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(db_error)?;
        Some(if let Some(id) = existing {
            transaction.execute(
                "UPDATE instrument_listings SET exchange_mic=?1,market_symbol=COALESCE(market_symbol,?2),quote_currency=COALESCE(?3,quote_currency) WHERE id=?4",
                params![exchange_mic, market_symbol, quote_currency, id],
            ).map_err(db_error)?;
            id
        } else {
            transaction.execute(
                "INSERT INTO instrument_listings(instrument_id,exchange_mic,market_symbol,quote_currency,preferred_price_source) VALUES(?1,?2,?3,?4,'alpha_vantage')",
                params![instrument_id, exchange_mic, market_symbol, quote_currency],
            ).map_err(db_error)?;
            transaction.last_insert_rowid()
        })
    } else {
        None
    };

    let previous_start = if let Some(id) = request.id {
        transaction
            .query_row(
                "SELECT holding_start_date FROM portfolio_positions WHERE id=?1 AND account_id=?2",
                params![id, request.account_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
    } else {
        None
    };
    let now = Utc::now().to_rfc3339();
    let position_id = if let Some(id) = request.id {
        let changed = transaction.execute(
            "UPDATE portfolio_positions SET listing_id=?1,label=?2,asset_type=?3,holding_start_date=?4,holding_end_date=?5,updated_at=?6 WHERE id=?7 AND account_id=?8",
            params![listing_id, request.label.trim(), request.asset_type.as_deref().unwrap_or("other"), start,
                request.holding_end_date.as_deref().filter(|value| !value.trim().is_empty()), now, id, request.account_id],
        ).map_err(db_error)?;
        if changed == 0 {
            return Err("Die Position wurde nicht gefunden.".into());
        }
        id
    } else {
        transaction.execute(
            "INSERT INTO portfolio_positions(account_id,listing_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",
            params![request.account_id, listing_id, request.label.trim(), request.asset_type.as_deref().unwrap_or("other"), start,
                request.holding_end_date.as_deref().filter(|value| !value.trim().is_empty()), now],
        ).map_err(db_error)?;
        transaction.last_insert_rowid()
    };

    let quantity_history_exists = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM position_quantities WHERE position_id=?1)",
            [position_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)?;
    if !quantity_history_exists {
        if let Some(quantity) = request.quantity {
            transaction.execute(
                "INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(?1,?2,?3,6,'manual',?4)",
                params![position_id, start, (quantity * 1_000_000.0).round() as i64, now],
            ).map_err(db_error)?;
        }
    }

    if listing_id.is_some() && request.unit_price_minor.is_none() {
        transaction
            .execute(
                "DELETE FROM daily_valuations WHERE position_id=?1",
                [position_id],
            )
            .map_err(db_error)?;
    }

    if listing_id.is_none() {
        transaction.execute(
            "INSERT INTO manual_position_values(position_id,value_date,amount_minor,currency,source,recorded_at)
             VALUES(?1,?2,?3,?4,'manual',?5)
             ON CONFLICT(position_id,value_date,source) DO UPDATE SET amount_minor=excluded.amount_minor,currency=excluded.currency,recorded_at=excluded.recorded_at",
            params![position_id, valuation_date, request.amount_minor, account_currency, now],
        ).map_err(db_error)?;
    } else if let (Some(listing_id), Some(price)) = (listing_id, request.unit_price_minor) {
        let currency = request
            .quote_currency
            .as_deref()
            .unwrap_or(&account_currency)
            .to_uppercase();
        transaction.execute(
            "INSERT INTO instrument_prices(listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at)
             VALUES(?1,?2,'manual_valuation',?3,2,?4,'manual',?5)
             ON CONFLICT(listing_id,price_date,price_type,source) DO UPDATE SET price_amount=excluded.price_amount,currency=excluded.currency,fetched_at=excluded.fetched_at",
            params![listing_id, valuation_date, price, currency, now],
        ).map_err(db_error)?;
        if currency != account_currency {
            if let Some(rate) = request.exchange_rate {
                transaction.execute(
                    "INSERT INTO fx_rates(base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at)
                     VALUES(?1,?2,?3,?4,9,'manual',?5)
                     ON CONFLICT(base_currency,quote_currency,rate_date,source) DO UPDATE SET
                       rate_amount=excluded.rate_amount,rate_scale=excluded.rate_scale,fetched_at=excluded.fetched_at",
                    params![currency, account_currency, valuation_date, (rate * 1_000_000_000.0).round() as i64, now],
                ).map_err(db_error)?;
            }
        }
    }
    let rebuild_from = previous_start
        .as_deref()
        .into_iter()
        .chain([start, valuation_date])
        .min()
        .unwrap_or(start)
        .to_string();
    transaction.commit().map_err(db_error)?;
    if listing_id.is_none() {
        rebuild_manual_position_valuations_from(storage, position_id, &rebuild_from)?;
    } else if request.unit_price_minor.is_some() {
        let rebuild_day = NaiveDate::parse_from_str(&rebuild_from, "%Y-%m-%d")
            .map_err(|_| "Das Bewertungsdatum ist ungültig.".to_string())?;
        rebuild_daily_valuations_for_position_from(storage, position_id, rebuild_day)?;
    }
    Ok(())
}

pub(crate) fn save_position_quantity_change(
    storage: &Storage,
    request: PositionQuantityChangeRequest,
) -> Result<(), String> {
    let effective_day = NaiveDate::parse_from_str(request.effective_date.trim(), "%Y-%m-%d")
        .map_err(|_| "Das Datum der Bestandsänderung ist ungültig.".to_string())?;
    if effective_day > Local::now().date_naive() {
        return Err("Das Datum der Bestandsänderung darf nicht in der Zukunft liegen.".into());
    }
    let change = parse_quantity(&request.quantity)?;
    if change.amount == 0 {
        return Err("Bitte eine Menge grösser als null eingeben.".into());
    }
    let direction = match request.change_type.as_str() {
        "buy" => 1_i128,
        "sell" => -1_i128,
        _ => return Err("Bitte Kauf oder Verkauf auswählen.".into()),
    };

    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let (account_type, start, listing_id, latest_date, current_amount, current_scale): (
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<u32>,
    ) = transaction
        .query_row(
            "SELECT a.account_type,p.holding_start_date,p.listing_id,
                    (SELECT q.valid_from FROM position_quantities q WHERE q.position_id=p.id ORDER BY date(q.valid_from) DESC,q.id DESC LIMIT 1),
                    (SELECT q.quantity_amount FROM position_quantities q WHERE q.position_id=p.id ORDER BY date(q.valid_from) DESC,q.id DESC LIMIT 1),
                    (SELECT q.quantity_scale FROM position_quantities q WHERE q.position_id=p.id ORDER BY date(q.valid_from) DESC,q.id DESC LIMIT 1)
             FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id WHERE p.id=?1",
            [request.position_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "Die Position wurde nicht gefunden.".into(),
            other => db_error(other),
        })?;
    if !crate::domain::banking::accounts::supports_positions(&account_type) {
        return Err("Bestandsänderungen sind für diesen Kontotyp nicht möglich.".into());
    }
    if listing_id.is_none() {
        return Err("Käufe und Verkäufe benötigen eine Position mit Wertpapierkennung.".into());
    }
    if effective_day
        < NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|_| "Das Einstandsdatum ist ungültig.".to_string())?
    {
        return Err("Das Datum darf nicht vor dem Einstandsdatum liegen.".into());
    }
    if latest_date
        .as_deref()
        .is_some_and(|date| date > request.effective_date.trim())
    {
        return Err("Die Bestandsänderung darf nicht vor dem letzten Mengenstand liegen.".into());
    }
    let (current_amount, current_scale) = current_amount
        .zip(current_scale)
        .ok_or("Für diese Position ist noch keine Menge erfasst.")?;
    let result_scale = current_scale.max(change.scale);
    let factor = |from: u32| 10_i128.pow(result_scale - from);
    let result = i128::from(current_amount) * factor(current_scale)
        + direction * i128::from(change.amount) * factor(change.scale);
    if result < 0 {
        return Err(
            "Es können nicht mehr Einheiten verkauft werden als aktuell vorhanden sind.".into(),
        );
    }
    let result =
        i64::try_from(result).map_err(|_| "Die resultierende Menge ist zu gross.".to_string())?;
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at)
         VALUES(?1,?2,?3,?4,'manual_trade',?5)
         ON CONFLICT(position_id,valid_from) DO UPDATE SET quantity_amount=excluded.quantity_amount,
           quantity_scale=excluded.quantity_scale,source=excluded.source,recorded_at=excluded.recorded_at",
        params![request.position_id, request.effective_date.trim(), result, result_scale, now],
    ).map_err(db_error)?;
    if result == 0 {
        transaction
            .execute(
                "UPDATE portfolio_positions SET holding_end_date=?1,updated_at=?2 WHERE id=?3",
                params![request.effective_date.trim(), now, request.position_id],
            )
            .map_err(db_error)?;
    } else {
        transaction
            .execute(
                "UPDATE portfolio_positions SET holding_end_date=NULL,updated_at=?1 WHERE id=?2",
                params![now, request.position_id],
            )
            .map_err(db_error)?;
    }
    transaction.commit().map_err(db_error)?;
    rebuild_daily_valuations_for_position_from(storage, request.position_id, effective_day)
}

fn rebuild_manual_position_valuations_from(
    storage: &Storage,
    position_id: i64,
    rebuild_from: &str,
) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let (listing_id, start, end): (Option<i64>, String, Option<String>) = connection
        .query_row(
            "SELECT listing_id,holding_start_date,holding_end_date FROM portfolio_positions WHERE id=?1",
            [position_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(db_error)?;
    if listing_id.is_some() {
        return Err("Die Position wird automatisch bewertet.".into());
    }
    let rebuild_day = NaiveDate::parse_from_str(rebuild_from, "%Y-%m-%d")
        .map_err(|_| "Das Bewertungsdatum ist ungültig.".to_string())?;
    let holding_start = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
        .map_err(|_| "Das Einstandsdatum ist ungültig.".to_string())?;
    let today = Local::now().date_naive();
    let last = end
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map_or(today, |value| value.min(today));
    let transaction = connection.transaction().map_err(db_error)?;
    transaction
        .execute(
            "DELETE FROM daily_valuations WHERE position_id=?1 AND date(valuation_date)>=date(?2)",
            params![position_id, rebuild_from],
        )
        .map_err(db_error)?;
    let calculated_at = Utc::now().to_rfc3339();
    let mut day = rebuild_day.max(holding_start);
    while day <= last {
        let date = day.format("%Y-%m-%d").to_string();
        if let Some(value) = transaction
            .prepare_cached(
                "SELECT amount_minor,currency FROM manual_position_values WHERE position_id=?1 AND date(value_date)<=date(?2) ORDER BY date(value_date) DESC,id DESC LIMIT 1",
            )
            .map_err(db_error)?
            .query_row(params![position_id, date], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .optional()
            .map_err(db_error)?
        {
            transaction
                .prepare_cached(
                    "INSERT INTO daily_valuations(position_id,valuation_date,value_minor,currency,calculated_at) VALUES(?1,?2,?3,?4,?5)",
                )
                .map_err(db_error)?
                .execute(params![position_id, date, value.0, value.1, calculated_at])
                .map_err(db_error)?;
        }
        day += Duration::days(1);
    }
    transaction.commit().map_err(db_error)
}

pub(crate) fn list_manual_valuations(
    storage: &Storage,
    position_id: i64,
) -> Result<Vec<ManualValuation>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection
        .prepare(
            "SELECT v.id,v.position_id,v.value_date,v.amount_minor,v.currency
             FROM manual_position_values v
             JOIN portfolio_positions p ON p.id=v.position_id
             WHERE v.position_id=?1 AND v.source='manual' AND p.listing_id IS NULL
             ORDER BY date(v.value_date) DESC,v.id DESC",
        )
        .map_err(db_error)?;
    let valuations = query
        .query_map([position_id], |row| {
            Ok(ManualValuation {
                id: row.get(0)?,
                position_id: row.get(1)?,
                value_date: row.get(2)?,
                amount_minor: row.get(3)?,
                currency: row.get(4)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(valuations)
}

pub(crate) fn update_manual_valuation(
    storage: &Storage,
    request: UpdateManualValuationRequest,
) -> Result<(), String> {
    let value_date = request.value_date.trim();
    let value_day = NaiveDate::parse_from_str(value_date, "%Y-%m-%d")
        .map_err(|_| "Das Bewertungsdatum ist ungültig.".to_string())?;
    if request.amount_minor < 0 {
        return Err("Der Bewertungswert darf nicht negativ sein.".into());
    }
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let valuation = transaction
        .query_row(
            "SELECT v.value_date,p.holding_start_date,p.holding_end_date,p.listing_id,v.source,a.currency
             FROM manual_position_values v
             JOIN portfolio_positions p ON p.id=v.position_id
             JOIN accounts a ON a.id=p.account_id
             WHERE v.id=?1 AND v.position_id=?2",
            params![request.id, request.position_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| "Die Bewertung wurde nicht gefunden.".to_string())?;
    if valuation.3.is_some() || valuation.4 != "manual" {
        return Err("Nur manuell eingegebene Bewertungen können korrigiert werden.".into());
    }
    let holding_start = NaiveDate::parse_from_str(&valuation.1, "%Y-%m-%d")
        .map_err(|_| "Das Einstandsdatum ist ungültig.".to_string())?;
    if value_day < holding_start {
        return Err("Das Bewertungsdatum darf nicht vor dem Einstandsdatum liegen.".into());
    }
    if let Some(end) = valuation.2.as_deref() {
        let holding_end = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "Das Verkaufsdatum ist ungültig.".to_string())?;
        if value_day > holding_end {
            return Err("Das Bewertungsdatum darf nicht nach dem Verkaufsdatum liegen.".into());
        }
    }
    let duplicate: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM manual_position_values WHERE position_id=?1 AND value_date=?2 AND source='manual' AND id<>?3)",
            params![request.position_id, value_date, request.id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if duplicate {
        return Err("Für diesen Stichtag besteht bereits eine Bewertung.".into());
    }
    transaction
        .execute(
            "UPDATE manual_position_values SET value_date=?1,amount_minor=?2,currency=?3,recorded_at=?4 WHERE id=?5 AND position_id=?6 AND source='manual'",
            params![value_date, request.amount_minor, valuation.5, Utc::now().to_rfc3339(), request.id, request.position_id],
        )
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)?;
    let rebuild_from = valuation.0.as_str().min(value_date);
    rebuild_manual_position_valuations_from(storage, request.position_id, rebuild_from)
}

pub(crate) fn delete_manual_valuation(
    storage: &Storage,
    request: DeleteManualValuationRequest,
) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let value_date = transaction
        .query_row(
            "SELECT v.value_date
             FROM manual_position_values v
             JOIN portfolio_positions p ON p.id=v.position_id
             WHERE v.id=?1 AND v.position_id=?2 AND v.source='manual' AND p.listing_id IS NULL",
            params![request.id, request.position_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| "Die Bewertung wurde nicht gefunden.".to_string())?;
    let valuation_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM manual_position_values WHERE position_id=?1 AND source='manual'",
            [request.position_id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if valuation_count <= 1 {
        return Err("Die einzige Bewertung einer Position kann nicht gelöscht werden.".into());
    }
    let changed = transaction
        .execute(
            "DELETE FROM manual_position_values WHERE id=?1 AND position_id=?2 AND source='manual'",
            params![request.id, request.position_id],
        )
        .map_err(db_error)?;
    if changed != 1 {
        return Err("Die Bewertung wurde nicht gefunden.".into());
    }
    transaction.commit().map_err(db_error)?;
    rebuild_manual_position_valuations_from(storage, request.position_id, &value_date)
}

pub(crate) fn list_manual_positions(
    storage: &Storage,
    account_id: i64,
) -> Result<Vec<ManualPosition>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection.prepare(
        "SELECT p.id,p.account_id,p.label,
                COALESCE(
                    (SELECT value_date FROM manual_position_values m WHERE m.position_id=p.id ORDER BY date(value_date) DESC,m.id DESC LIMIT 1),
                    (SELECT valuation_date FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1),
                    p.holding_start_date
                ),
                COALESCE((SELECT value_minor FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1),0),
                COALESCE((SELECT currency FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1),a.currency),
                (SELECT quantity_amount FROM position_quantities q WHERE q.position_id=p.id ORDER BY valid_from DESC LIMIT 1),
                (SELECT quantity_scale FROM position_quantities q WHERE q.position_id=p.id ORDER BY valid_from DESC LIMIT 1),
                (SELECT ip.price_amount FROM instrument_prices ip WHERE ip.id=(SELECT instrument_price_id FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1)),
                (SELECT ip.price_scale FROM instrument_prices ip WHERE ip.id=(SELECT instrument_price_id FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1)),
                l.quote_currency,
                (SELECT fx.rate_amount FROM fx_rates fx WHERE fx.id=(SELECT fx_rate_id FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1)),
                (SELECT fx.rate_scale FROM fx_rates fx WHERE fx.id=(SELECT fx_rate_id FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1)),
                p.asset_type,ii.identifier_type,ii.identifier,l.preferred_price_source,
                p.holding_start_date,p.holding_end_date,
                p.listing_id IS NULL AND NOT EXISTS(SELECT 1 FROM daily_valuations history WHERE history.position_id=p.id)
         FROM portfolio_positions p
         JOIN accounts a ON a.id=p.account_id
         LEFT JOIN instrument_listings l ON l.id=p.listing_id
         LEFT JOIN instrument_identifiers ii ON ii.id=(SELECT identifier.id FROM instrument_identifiers identifier
           WHERE identifier.instrument_id=l.instrument_id
           ORDER BY CASE identifier.identifier_type WHEN 'ticker' THEN 1 WHEN 'isin' THEN 2 ELSE 3 END,identifier.id LIMIT 1)
         WHERE p.account_id=?1 ORDER BY p.label,p.id"
    ).map_err(db_error)?;
    let positions = query
        .query_map([account_id], |r| {
            let quantity_amount = r.get::<_, Option<i64>>(6)?;
            let quantity_scale = r.get::<_, Option<i64>>(7)?;
            let price_amount = r.get::<_, Option<i64>>(8)?;
            let price_scale = r.get::<_, Option<i64>>(9)?;
            let fx_amount = r.get::<_, Option<i64>>(11)?;
            let fx_scale = r.get::<_, Option<i64>>(12)?;
            Ok(ManualPosition {
                id: r.get(0)?,
                account_id: r.get(1)?,
                label: r.get(2)?,
                valuation_date: r.get(3)?,
                amount_minor: r.get(4)?,
                value_currency: r.get(5)?,
                quantity: quantity_amount
                    .zip(quantity_scale)
                    .map(|(amount, scale)| decimal_value(amount, scale)),
                unit_price_minor: price_amount
                    .zip(price_scale)
                    .map(|(amount, scale)| (decimal_value(amount, scale) * 100.0).round() as i64),
                quote_currency: r.get(10)?,
                exchange_rate: fx_amount
                    .zip(fx_scale)
                    .map(|(amount, scale)| decimal_value(amount, scale)),
                asset_type: r.get(13)?,
                identifier_type: r.get(14)?,
                identifier: r.get(15)?,
                price_source: r.get(16)?,
                holding_start_date: r.get(17)?,
                holding_end_date: r.get(18)?,
                can_delete: r.get(19)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(positions)
}

pub(crate) fn delete_manual_position(storage: &Storage, position_id: i64) -> Result<(), String> {
    let connection = storage.connect().map_err(db_error)?;
    let preserves_history: bool = connection.query_row(
        "SELECT listing_id IS NOT NULL OR EXISTS(SELECT 1 FROM daily_valuations d WHERE d.position_id=portfolio_positions.id) FROM portfolio_positions WHERE id=?1",
        [position_id], |row| row.get(0),
    ).unwrap_or(false);
    if preserves_history {
        return Err("Diese Wertpapierposition besitzt eine Kurshistorie. Bitte trage beim Bearbeiten das Verkaufs- oder Enddatum ein, damit die Historie erhalten bleibt.".into());
    }
    connection
        .execute("DELETE FROM portfolio_positions WHERE id=?1", [position_id])
        .map_err(db_error)?;
    Ok(())
}

pub(crate) fn listing_parts(identifier: &str) -> (Option<String>, Option<String>) {
    identifier
        .split_once(':')
        .map_or((None, Some(identifier.to_string())), |(mic, symbol)| {
            (Some(mic.to_string()), Some(symbol.to_string()))
        })
}

pub(crate) fn exchange_currency(mic: Option<&str>) -> Option<&'static str> {
    match mic {
        Some("XASX") => Some("AUD"),
        Some("XSWX") => Some("CHF"),
        Some("XLON") => Some("GBP"),
        Some("XETR") => Some("EUR"),
        Some("XNAS" | "XNYS" | "XASE") => Some("USD"),
        _ => None,
    }
}

pub(crate) fn decimal_value(amount: i64, scale: i64) -> f64 {
    amount as f64 / 10_f64.powi(scale as i32)
}

#[cfg(test)]
pub(crate) fn rebuild_daily_valuations(storage: &Storage) -> Result<(), String> {
    rebuild_daily_valuations_with_scope(storage, ValuationRebuildScope::Full)
}

pub(crate) fn rebuild_daily_valuations_incremental(storage: &Storage) -> Result<(), String> {
    rebuild_daily_valuations_with_scope(storage, ValuationRebuildScope::Incremental)
}

fn rebuild_daily_valuations_for_position_from(
    storage: &Storage,
    position_id: i64,
    rebuild_from: NaiveDate,
) -> Result<(), String> {
    rebuild_daily_valuations_with_scope(
        storage,
        ValuationRebuildScope::PositionFrom(position_id, rebuild_from),
    )
}

pub(crate) fn rebuild_account_valuations_from(
    storage: &Storage,
    account_id: i64,
    rebuild_from: NaiveDate,
) -> Result<(), String> {
    rebuild_daily_valuations_with_scope(
        storage,
        ValuationRebuildScope::AccountFrom(account_id, rebuild_from),
    )
}

#[derive(Clone, Copy)]
enum ValuationRebuildScope {
    #[cfg(test)]
    Full,
    Incremental,
    PositionFrom(i64, NaiveDate),
    AccountFrom(i64, NaiveDate),
}

fn rebuild_daily_valuations_with_scope(
    storage: &Storage,
    scope: ValuationRebuildScope,
) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let positions = {
        let mut statement = connection
            .prepare(
                "SELECT p.id,p.account_id,p.listing_id,p.holding_start_date,p.holding_end_date,a.currency
             FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id",
            )
            .map_err(db_error)?;
        let values = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        values
    };
    let transaction = connection.transaction().map_err(db_error)?;
    match scope {
        #[cfg(test)]
        ValuationRebuildScope::Full => transaction
            .execute("DELETE FROM daily_valuations", [])
            .map_err(db_error)?,
        ValuationRebuildScope::PositionFrom(position_id, rebuild_from) => transaction
            .execute(
                "DELETE FROM daily_valuations WHERE position_id=?1 AND date(valuation_date)>=date(?2)",
                params![position_id, rebuild_from.format("%Y-%m-%d").to_string()],
            )
            .map_err(db_error)?,
        ValuationRebuildScope::AccountFrom(account_id, rebuild_from) => transaction
            .execute(
                "DELETE FROM daily_valuations WHERE position_id IN (SELECT id FROM portfolio_positions WHERE account_id=?1) AND date(valuation_date)>=date(?2)",
                params![account_id, rebuild_from.format("%Y-%m-%d").to_string()],
            )
            .map_err(db_error)?,
        ValuationRebuildScope::Incremental => 0,
    };
    let rebuild_from = match scope {
        ValuationRebuildScope::PositionFrom(_, date)
        | ValuationRebuildScope::AccountFrom(_, date) => Some(date),
        _ => None,
    };
    let incremental = matches!(scope, ValuationRebuildScope::Incremental);
    let today = Local::now().date_naive();
    for (position_id, account_id, listing_id, start, end, _account_currency) in positions {
        match scope {
            ValuationRebuildScope::PositionFrom(target_id, _) if target_id != position_id => {
                continue;
            }
            ValuationRebuildScope::AccountFrom(target_id, _) if target_id != account_id => {
                continue;
            }
            _ => {}
        }
        let mut day = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|_| "Ungültiges Einstandsdatum in dem Finanzprofil.".to_string())?;
        if let Some(rebuild_from) = rebuild_from {
            day = day.max(rebuild_from);
        } else if incremental {
            if listing_id.is_none() {
                continue;
            }
            let latest_valuation: Option<String> = transaction
                .query_row(
                    "SELECT MAX(valuation_date) FROM daily_valuations WHERE position_id=?1",
                    [position_id],
                    |row| row.get(0),
                )
                .map_err(db_error)?;
            if let Some(latest) = latest_valuation
                .as_deref()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
            {
                day = latest;
            }
            transaction
                .execute(
                    "DELETE FROM daily_valuations WHERE position_id=?1 AND date(valuation_date)>=date(?2)",
                    params![position_id, day.format("%Y-%m-%d").to_string()],
                )
                .map_err(db_error)?;
        }
        let last = end
            .as_deref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
            .map_or(today, |value| value.min(today));
        let calculated_at = Utc::now().to_rfc3339();
        if let Some(listing_id) = listing_id {
            let valuation_currency = "CHF";
            while day <= last {
                let date = day.format("%Y-%m-%d").to_string();
                let quantity = transaction
                    .prepare_cached(
                        "SELECT quantity_amount,quantity_scale FROM position_quantities
                     WHERE position_id=?1 AND date(valid_from)<=date(?2)
                     ORDER BY date(valid_from) DESC,id DESC LIMIT 1",
                    )
                    .map_err(db_error)?
                    .query_row(params![position_id, date], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                    })
                    .optional()
                    .map_err(db_error)?;
                let price = transaction.prepare_cached(
                    "SELECT id,price_amount,price_scale,currency FROM instrument_prices
                     WHERE listing_id=?1 AND date(price_date)<=date(?2)
                       AND price_type IN ('official_close','eod_close','nav','manual_valuation','last')
                       AND (price_type='manual_valuation'
                         OR source=(SELECT preferred_price_source FROM instrument_listings WHERE id=?1)
                         OR (SELECT preferred_price_source FROM instrument_listings WHERE id=?1) IS NULL)
                     ORDER BY date(price_date) DESC,
                       CASE price_type WHEN 'official_close' THEN 1 WHEN 'eod_close' THEN 2 WHEN 'nav' THEN 3 WHEN 'last' THEN 4 ELSE 5 END,
                       CASE source WHEN 'marketstack' THEN 1 WHEN 'alpha_vantage' THEN 2 WHEN 'yahoo' THEN 3 ELSE 4 END,id DESC LIMIT 1"
                ).map_err(db_error)?.query_row(
                    params![listing_id,date],
                    |row| Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?,row.get::<_,String>(3)?)),
                ).optional().map_err(db_error)?;
                if let (
                    Some((quantity_amount, quantity_scale)),
                    Some((price_id, price_amount, price_scale, price_currency)),
                ) = (quantity, price)
                {
                    let fx = if price_currency == valuation_currency {
                        Some((None, 1.0))
                    } else {
                        transaction.prepare_cached(
                            "SELECT id,rate_amount,rate_scale FROM fx_rates
                             WHERE base_currency=?1 AND quote_currency=?2 AND date(rate_date)<=date(?3)
                             ORDER BY date(rate_date) DESC,id DESC LIMIT 1"
                        ).map_err(db_error)?.query_row(
                            params![price_currency,valuation_currency,date],
                            |row| Ok((Some(row.get::<_,i64>(0)?),decimal_value(row.get(1)?,row.get(2)?))),
                        ).optional().map_err(db_error)?
                    };
                    if let Some((fx_rate_id, rate)) = fx {
                        let value_minor = (decimal_value(quantity_amount, quantity_scale)
                            * decimal_value(price_amount, price_scale)
                            * rate
                            * 100.0)
                            .round() as i64;
                        transaction.prepare_cached(
                            "INSERT INTO daily_valuations(position_id,valuation_date,quantity_amount,quantity_scale,instrument_price_id,fx_rate_id,value_minor,currency,calculated_at)
                             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)"
                        ).map_err(db_error)?.execute(
                            params![position_id,date,quantity_amount,quantity_scale,price_id,fx_rate_id,value_minor,valuation_currency,calculated_at],
                        ).map_err(db_error)?;
                    }
                }
                day += Duration::days(1);
            }
        } else {
            while day <= last {
                let date = day.format("%Y-%m-%d").to_string();
                if let Some(value) = transaction.prepare_cached(
                    "SELECT amount_minor,currency FROM manual_position_values WHERE position_id=?1 AND date(value_date)<=date(?2) ORDER BY date(value_date) DESC,id DESC LIMIT 1",
                ).map_err(db_error)?.query_row(
                    params![position_id,date], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?)),
                ).optional().map_err(db_error)? {
                    transaction.prepare_cached(
                        "INSERT INTO daily_valuations(position_id,valuation_date,value_minor,currency,calculated_at) VALUES(?1,?2,?3,?4,?5)"
                    ).map_err(db_error)?.execute(
                        params![position_id,date,value.0,value.1,calculated_at],
                    ).map_err(db_error)?;
                }
                day += Duration::days(1);
            }
        }
    }
    transaction.commit().map_err(db_error)
}
