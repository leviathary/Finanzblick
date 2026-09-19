//! Speichert allgemeine Umbuchungsregeln und bestätigt historische Treffer atomar.
//! Bestehende Kartenregeln bleiben ohne Datenumschreibung als Kartenausgleich lesbar.
use crate::domain::banking::transfers::{normalized_rule_text, TransferRuleInput};
use crate::storage::{db_error, Storage};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    id: i64,
    name: String,
    account_id: i64,
    account_name: String,
    currency: String,
    direction: i64,
    prefix: String,
    transfer_type: String,
}

pub fn list_transfer_rules(storage: &Storage) -> Result<Vec<Rule>, String> {
    let db = storage.connect().map_err(db_error)?;
    let mut query = db
        .prepare(
            "SELECT r.id,COALESCE(d.name,''),r.account_id,a.name,r.currency,r.direction,r.prefix,
        COALESCE(d.transfer_type,'CREDIT_CARD_SETTLEMENT') FROM settlement_rules r
        JOIN accounts a ON a.id=r.account_id LEFT JOIN transfer_rule_details d ON d.rule_id=r.id
        ORDER BY a.name,r.id",
        )
        .map_err(db_error)?;
    let rows = query
        .query_map([], |r| {
            Ok(Rule {
                id: r.get(0)?,
                name: r.get(1)?,
                account_id: r.get(2)?,
                account_name: r.get(3)?,
                currency: r.get(4)?,
                direction: r.get(5)?,
                prefix: r.get(6)?,
                transfer_type: r.get(7)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub(super) fn check_conflicts(
    db: &Connection,
    id: Option<i64>,
    account: i64,
    currency: &str,
    direction: i64,
    prefix: &str,
    kind: &str,
) -> Result<(), String> {
    let mut query = db
        .prepare(
            "SELECT r.prefix,COALESCE(d.transfer_type,'CREDIT_CARD_SETTLEMENT')
        FROM settlement_rules r LEFT JOIN transfer_rule_details d ON d.rule_id=r.id
        WHERE r.account_id=?1 AND r.currency=?2 AND r.direction=?3 AND (?4 IS NULL OR r.id<>?4)",
        )
        .map_err(db_error)?;
    let rows = query
        .query_map(params![account, currency, direction, id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(db_error)?;
    for row in rows {
        let (other, action) = row.map_err(db_error)?;
        if action != kind && (prefix.starts_with(&other) || other.starts_with(prefix)) {
            return Err("Eine überlappende Regel verwendet einen anderen Regeltyp. Bitte die Bedingungen präzisieren.".into());
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchRow {
    id: i64,
    booking_date: String,
    description: String,
    amount_minor: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    matches: Vec<MatchRow>,
    protected_count: usize,
}

fn preview(db: &Connection, input: &TransferRuleInput) -> Result<Preview, String> {
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
            [input.account_id],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    if !exists {
        return Err("Konto nicht gefunden.".into());
    }
    if let Some(id) = input.id {
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM settlement_rules WHERE id=?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if !exists {
            return Err("Regel nicht gefunden. Bitte die Liste neu laden.".into());
        }
    }
    check_conflicts(
        db,
        input.id,
        input.account_id,
        &input.currency,
        input.direction,
        &input.prefix,
        input.type_key(),
    )?;
    let duplicate: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM settlement_rules
        WHERE account_id=?1 AND currency=?2 AND direction=?3 AND prefix=?4 AND (?5 IS NULL OR id<>?5))",
        params![input.account_id,input.currency,input.direction,input.prefix,input.id],|r|r.get(0)).map_err(db_error)?;
    if duplicate {
        return Err("Eine Regel mit diesen Bedingungen existiert bereits. Bitte die bestehende Regel bearbeiten.".into());
    }
    let mut query = db.prepare("SELECT t.id,t.booking_date,t.description,t.amount_minor,COALESCE(f.is_manually_overridden,0)
        FROM transactions t LEFT JOIN transaction_reporting_flags f ON f.transaction_id=t.id
        WHERE t.account_id=?1 AND t.currency=?2 AND sign(t.amount_minor)=?3 ORDER BY t.booking_date DESC,t.id DESC").map_err(db_error)?;
    let rows = query
        .query_map(
            params![input.account_id, input.currency, input.direction],
            |r| {
                Ok((
                    MatchRow {
                        id: r.get(0)?,
                        booking_date: r.get(1)?,
                        description: r.get(2)?,
                        amount_minor: r.get(3)?,
                    },
                    r.get::<_, bool>(4)?,
                ))
            },
        )
        .map_err(db_error)?;
    let mut result = Preview {
        matches: vec![],
        protected_count: 0,
    };
    for row in rows {
        let (row, manual) = row.map_err(db_error)?;
        if normalized_rule_text(&row.description).starts_with(&input.prefix) {
            if manual {
                result.protected_count += 1;
            } else {
                result.matches.push(row);
            }
        }
    }
    Ok(result)
}

pub fn preview_transfer_rule(
    storage: &Storage,
    mut input: TransferRuleInput,
) -> Result<Preview, String> {
    input.validate()?;
    let db = storage.connect().map_err(db_error)?;
    preview(&db, &input)
}

fn save(
    db: &mut Connection,
    mut input: TransferRuleInput,
    expected_ids: Vec<i64>,
    past: bool,
    future: bool,
) -> Result<usize, String> {
    input.validate()?;
    if !past && !future {
        return Err("Bitte bestehende Buchungen oder zukünftige Importe auswählen.".into());
    }
    let tx = db.transaction().map_err(db_error)?;
    let current = preview(&tx, &input)?;
    if past {
        let actual: BTreeSet<_> = current.matches.iter().map(|r| r.id).collect();
        if actual != expected_ids.into_iter().collect() || actual.len() > 5000 {
            return Err("Die Treffer haben sich geändert oder überschreiten 5000 Buchungen. Bitte Vorschau neu laden.".into());
        }
        for id in actual {
            tx.execute("INSERT INTO transaction_reporting_flags(transaction_id,exclude_from_cashflow,is_settlement,is_manually_overridden)
                VALUES(?1,1,?2,0) ON CONFLICT(transaction_id) DO UPDATE SET exclude_from_cashflow=1,is_settlement=excluded.is_settlement
                WHERE transaction_reporting_flags.is_manually_overridden=0",params![id,input.type_key()=="CREDIT_CARD_SETTLEMENT"]).map_err(db_error)?;
        }
    }
    if future {
        let id = if let Some(id) = input.id {
            tx.execute("UPDATE settlement_rules SET account_id=?2,currency=?3,direction=?4,prefix=?5 WHERE id=?1",
                params![id,input.account_id,input.currency,input.direction,input.prefix]).map_err(db_error)?;
            id
        } else {
            tx.execute("INSERT INTO settlement_rules(account_id,currency,direction,prefix) VALUES(?1,?2,?3,?4)",
                params![input.account_id,input.currency,input.direction,input.prefix]).map_err(db_error)?;
            tx.last_insert_rowid()
        };
        tx.execute("INSERT INTO transfer_rule_details(rule_id,name,transfer_type) VALUES(?1,?2,?3)
            ON CONFLICT(rule_id) DO UPDATE SET name=excluded.name,transfer_type=excluded.transfer_type",params![id,input.name,input.type_key()]).map_err(db_error)?;
    } else if let Some(id) = input.id {
        tx.execute("DELETE FROM settlement_rules WHERE id=?1", [id])
            .map_err(db_error)?;
    }
    tx.commit().map_err(db_error)?;
    Ok(if past { current.matches.len() } else { 0 })
}

pub fn save_transfer_rule(
    storage: &Storage,
    input: TransferRuleInput,
    expected_ids: Vec<i64>,
    past: bool,
    future: bool,
) -> Result<usize, String> {
    let mut db = storage.connect().map_err(db_error)?;
    save(&mut db, input, expected_ids, past, future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::banking::transfers::TransferType;

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE accounts(id INTEGER PRIMARY KEY,name TEXT);
            INSERT INTO accounts VALUES(1,'Private'),(2,'Household');
            CREATE TABLE transactions(id INTEGER PRIMARY KEY,account_id INTEGER,currency TEXT,amount_minor INTEGER,description TEXT,booking_date TEXT,import_id INTEGER);
            INSERT INTO transactions VALUES
            (1,1,'CHF',-100,'Household transfer January','2026-01-01',1),
            (2,1,'CHF',-200,'HOUSEHOLD   TRANSFER February','2026-02-01',1),
            (3,2,'CHF',100,'Household transfer January','2026-01-01',1),
            (4,1,'EUR',-100,'Household transfer January','2026-01-01',1),
            (5,1,'CHF',100,'Household transfer January','2026-01-01',1);").unwrap();
        crate::storage::database::schema::initialize_reporting_flags(&db).unwrap();
        super::super::settlement_rules::initialize(&db).unwrap();
        db
    }
    fn input() -> TransferRuleInput {
        TransferRuleInput {
            id: None,
            name: "Household".into(),
            account_id: 1,
            currency: "CHF".into(),
            direction: -1,
            prefix: "household transfer".into(),
            transfer_type: TransferType::InternalTransfer,
        }
    }
    fn flags(db: &Connection, id: i64) -> (bool, bool, bool) {
        db.query_row("SELECT exclude_from_cashflow,is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap()
    }
    #[test]
    fn scoped_rules_apply_to_history_and_imports_without_changing_money() {
        let mut db = fixture();
        let p = preview(&db, &input()).unwrap();
        assert_eq!(
            p.matches.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert!(save(&mut db, input(), vec![1], true, true).is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM settlement_rules", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(save(&mut db, input(), vec![1, 2], true, true).unwrap(), 2);
        assert_eq!(flags(&db, 1), (true, false, false));
        assert_eq!(
            db.query_row(
                "SELECT amount_minor FROM transactions WHERE id=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            -100
        );
        db.execute("INSERT INTO transactions VALUES(6,1,'CHF',-350,'Household transfer March','2026-03-01',2)",[]).unwrap();
        super::super::settlement_rules::apply_import(&db, 2).unwrap();
        assert_eq!(flags(&db, 6), (true, false, false));
        crate::storage::banking::reporting_flags::set_settlement(&db, 6, false).unwrap();
        super::super::settlement_rules::apply_import(&db, 2).unwrap();
        assert_eq!(flags(&db, 6), (false, false, true));
        assert!(preview(&db, &input()).is_err()); // Creation would duplicate the saved rule.
        let mut edited = input();
        edited.id = Some(1);
        assert_eq!(preview(&db, &edited).unwrap().protected_count, 1);
    }
    #[test]
    fn editing_and_deactivation_do_not_reset_existing_marks() {
        let mut db = fixture();
        save(&mut db, input(), vec![1, 2], true, true).unwrap();
        let mut edited = input();
        edited.id = Some(1);
        edited.name = "New name".into();
        edited.prefix = "household transfer january".into();
        edited.transfer_type = TransferType::CreditCardSettlement;
        save(&mut db, edited.clone(), vec![], false, true).unwrap();
        assert_eq!(flags(&db, 1), (true, false, false));
        save(&mut db, edited.clone(), vec![1], true, true).unwrap();
        assert_eq!(flags(&db, 1), (true, true, false));
        assert_eq!(flags(&db, 2), (true, false, false));
        save(&mut db, edited, vec![1], true, false).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM settlement_rules", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM transfer_rule_details", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(flags(&db, 1), (true, true, false));
    }
    #[test]
    fn conflicting_actions_and_stale_manual_decisions_are_rejected() {
        let mut db = fixture();
        save(&mut db, input(), vec![], false, true).unwrap();
        let mut conflicting = input();
        conflicting.prefix = "household transfer january".into();
        conflicting.transfer_type = TransferType::CreditCardSettlement;
        assert!(save(&mut db, conflicting, vec![1], true, true).is_err());
        assert!(super::super::settlement_rules::apply_confirmed(
            &db,
            1,
            "household transfer".into(),
            vec![1, 2],
            true,
            true
        )
        .is_err());
        crate::storage::banking::reporting_flags::set_settlement(&db, 1, false).unwrap();
        let mut edit = input();
        edit.id = Some(1);
        assert!(save(&mut db, edit.clone(), vec![1, 2], true, true).is_err());
        save(&mut db, edit, vec![2], true, true).unwrap();
        assert_eq!(flags(&db, 1), (false, false, true));
        let mut invalid = input();
        invalid.prefix = "short".into();
        assert!(save(&mut db, invalid, vec![], false, true).is_err());
        assert!(save(&mut db, input(), vec![], false, false).is_err());
    }
    #[test]
    fn future_only_rule_needs_no_historical_match_and_scopes_the_credit_side() {
        let mut db = fixture();
        let mut rule = input();
        rule.account_id = 2;
        rule.direction = 1;
        rule.prefix = "  SAVINGS   TRANSFER  ".into();
        assert_eq!(save(&mut db, rule, vec![], false, true).unwrap(), 0);
        assert_eq!(
            db.query_row("SELECT prefix FROM settlement_rules", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "savings transfer"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM transaction_reporting_flags",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        db.execute("INSERT INTO transactions VALUES(6,2,'CHF',150,'Savings transfer monthly','2026-03-01',2)",[]).unwrap();
        super::super::settlement_rules::apply_import(&db, 2).unwrap();
        assert_eq!(flags(&db, 6), (true, false, false));
    }
}
