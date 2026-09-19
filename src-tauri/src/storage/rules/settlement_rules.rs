//! Verwaltet ausdrücklich bestätigte Kartenausgleichsregeln mit Vorschau und geschützten manuellen Entscheidungen.
use crate::storage::{db_error, Storage};
use rusqlite::{params, Connection};
use serde::Serialize;

fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(in crate::storage) fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS settlement_rules(
        id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
        currency TEXT NOT NULL, direction INTEGER NOT NULL CHECK(direction IN (-1,1)),
        prefix TEXT NOT NULL, UNIQUE(account_id,currency,direction,prefix));")
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
    prefix: String,
    account_name: String,
    currency: String,
    direction: i64,
    matches: Vec<MatchRow>,
    protected_count: usize,
}

pub(in crate::storage) fn preview(
    db: &Connection,
    id: i64,
    prefix: Option<String>,
) -> Result<(i64, Preview), String> {
    let (account, currency, amount, description, name): (i64, String, i64, String, String) = db
        .query_row(
            "SELECT t.account_id,t.currency,t.amount_minor,t.description,a.name FROM transactions t
         JOIN accounts a ON a.id=t.account_id WHERE t.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(db_error)?;
    let prefix = normalized(&prefix.unwrap_or_else(|| {
        description
            .split('·')
            .next()
            .unwrap_or(&description)
            .trim()
            .into()
    }));
    if amount == 0
        || prefix.chars().count() < 8
        || prefix.len() > 2000
        || !normalized(&description).starts_with(&prefix)
    {
        return Err("Bitte einen passenden Textanfang mit mindestens 8 Zeichen wählen.".into());
    }
    let direction = amount.signum();
    let mut q=db.prepare("SELECT t.id,t.booking_date,t.description,t.amount_minor,
        COALESCE(f.is_manually_overridden,0) FROM transactions t
        LEFT JOIN transaction_reporting_flags f ON f.transaction_id=t.id
        WHERE t.account_id=?1 AND t.currency=?2 AND sign(t.amount_minor)=?3 ORDER BY t.booking_date DESC,t.id DESC").map_err(db_error)?;
    let rows = q
        .query_map(params![account, currency, direction], |r| {
            Ok((
                MatchRow {
                    id: r.get(0)?,
                    booking_date: r.get(1)?,
                    description: r.get(2)?,
                    amount_minor: r.get(3)?,
                },
                r.get::<_, bool>(4)?,
            ))
        })
        .map_err(db_error)?;
    let mut matches = Vec::new();
    let mut protected_count = 0;
    for row in rows {
        let (row, manual) = row.map_err(db_error)?;
        if normalized(&row.description).starts_with(&prefix) {
            if manual && row.id != id {
                protected_count += 1;
            } else {
                matches.push(row);
            }
        }
    }
    Ok((
        account,
        Preview {
            prefix,
            account_name: name,
            currency,
            direction,
            matches,
            protected_count,
        },
    ))
}

pub fn preview_settlement_rule(
    storage: &Storage,
    transaction_id: i64,
    prefix: Option<String>,
) -> Result<Preview, String> {
    let db = storage.connect().map_err(db_error)?;
    preview(&db, transaction_id, prefix).map(|(_, p)| p)
}

fn confirm(
    db: &mut Connection,
    id: i64,
    prefix: String,
    expected_ids: Vec<i64>,
    future: bool,
) -> Result<usize, String> {
    let tx = db.transaction().map_err(db_error)?;
    let count = apply_confirmed(&tx, id, prefix, expected_ids, true, future)?;
    tx.commit().map_err(db_error)?;
    Ok(count)
}

pub(crate) fn apply_confirmed(
    db: &Connection,
    id: i64,
    prefix: String,
    expected_ids: Vec<i64>,
    past: bool,
    future: bool,
) -> Result<usize, String> {
    let (account, p) = preview(db, id, Some(prefix))?;
    let expected: std::collections::BTreeSet<_> = expected_ids.into_iter().collect();
    let actual: std::collections::BTreeSet<_> = p.matches.iter().map(|r| r.id).collect();
    if actual != expected || actual.is_empty() || actual.len() > 5000 {
        return Err("Die Treffer haben sich geändert oder überschreiten 5000 Buchungen. Bitte Vorschau neu laden.".into());
    }
    for id in &actual {
        if past {
            crate::storage::banking::reporting_flags::set_settlement(db, *id, true)?;
        }
    }
    if future {
        db.execute("INSERT OR IGNORE INTO settlement_rules(account_id,currency,direction,prefix) VALUES(?1,?2,?3,?4)",
            params![account,p.currency,p.direction,p.prefix]).map_err(db_error)?;
    }
    Ok(if past { actual.len() } else { 0 })
}

pub fn confirm_settlement_rule(
    storage: &Storage,
    transaction_id: i64,
    prefix: String,
    expected_ids: Vec<i64>,
    future: bool,
) -> Result<usize, String> {
    let mut db = storage.connect().map_err(db_error)?;
    confirm(&mut db, transaction_id, prefix, expected_ids, future)
}

pub(in crate::storage) fn apply_import(db: &Connection, import_id: i64) -> Result<(), String> {
    let mut q=db.prepare("SELECT t.id,t.description,r.prefix FROM transactions t
        JOIN settlement_rules r ON r.account_id=t.account_id AND r.currency=t.currency AND r.direction=sign(t.amount_minor)
        LEFT JOIN transaction_reporting_flags f ON f.transaction_id=t.id
        WHERE t.import_id=?1 AND COALESCE(f.is_manually_overridden,0)=0").map_err(db_error)?;
    let rows = q
        .query_map([import_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    for (id, text, prefix) in rows {
        if normalized(&text).starts_with(&prefix) {
            db.execute("INSERT INTO transaction_reporting_flags(transaction_id,exclude_from_cashflow,is_settlement,is_manually_overridden)
                VALUES(?1,1,1,0) ON CONFLICT(transaction_id) DO UPDATE SET exclude_from_cashflow=1,is_settlement=1
                WHERE transaction_reporting_flags.is_manually_overridden=0",[id]).map_err(db_error)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    id: i64,
    account_id: i64,
    account_name: String,
    currency: String,
    direction: i64,
    prefix: String,
}

pub fn list_settlement_rules(storage: &Storage) -> Result<Vec<Rule>, String> {
    let db = storage.connect().map_err(db_error)?;
    rules_from(&db)
}

fn rules_from(db: &Connection) -> Result<Vec<Rule>, String> {
    let mut q = db
        .prepare(
            "SELECT r.id,a.name,r.currency,r.direction,r.prefix,r.account_id FROM settlement_rules r
        JOIN accounts a ON a.id=r.account_id ORDER BY a.name,r.id",
        )
        .map_err(db_error)?;
    let rows = q
        .query_map([], |r| {
            Ok(Rule {
                id: r.get(0)?,
                account_id: r.get(5)?,
                account_name: r.get(1)?,
                currency: r.get(2)?,
                direction: r.get(3)?,
                prefix: r.get(4)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub fn delete_settlement_rule(storage: &Storage, rule_id: i64) -> Result<(), String> {
    storage
        .connect()
        .map_err(db_error)?
        .execute("DELETE FROM settlement_rules WHERE id=?1", [rule_id])
        .map_err(db_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn listed_rules_preserve_account_ids_even_for_identical_names() {
        let db = fixture();
        db.execute("UPDATE accounts SET name='Family card'", []).unwrap();
        db.execute("INSERT INTO settlement_rules(account_id,currency,direction,prefix) VALUES(1,'CHF',1,'payment'),(2,'CHF',1,'payment')", []).unwrap();
        let rules = rules_from(&db).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].account_name, rules[1].account_name);
        assert_eq!(rules[0].account_id, 1);
        assert_eq!(rules[1].account_id, 2);
        let json = serde_json::to_value(&rules[0]).unwrap();
        assert_eq!(json["accountId"], 1);
        db.execute("DELETE FROM settlement_rules WHERE account_id=1", []).unwrap();
        let remaining = rules_from(&db).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].account_id, 2);
    }
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE accounts(id INTEGER PRIMARY KEY,name TEXT);
          INSERT INTO accounts VALUES(1,'Bank'),(2,'Other');
          CREATE TABLE transactions(id INTEGER PRIMARY KEY,account_id INTEGER,currency TEXT,amount_minor INTEGER,description TEXT,booking_date TEXT,import_id INTEGER);
          INSERT INTO transactions VALUES
          (1,1,'CHF',-100,'Card payment · ref 11','2026-01-01',1),
          (2,1,'CHF',-200,'CARD  PAYMENT · ref 22','2026-02-01',1),
          (3,2,'CHF',-100,'Card payment · ref 33','2026-01-01',1),
          (4,1,'EUR',-100,'Card payment · ref 44','2026-01-01',1),
          (5,1,'CHF',100,'Card payment · ref 55','2026-01-01',1),
          (6,1,'CHF',-100,'Card purchase · ref 66','2026-01-01',1),
          (7,1,'CHF',-100,'Card payment · ref 77','2026-01-01',1);").unwrap();
        crate::storage::database::schema::initialize_reporting_flags(&db).unwrap();
        initialize(&db).unwrap();
        crate::storage::banking::reporting_flags::set_settlement(&db, 7, false).unwrap();
        db
    }
    #[test]
    fn preview_scopes_account_currency_direction_and_preserves_manual_choices() {
        let mut db = fixture();
        let (_, p) = preview(&db, 1, None).unwrap();
        assert_eq!(p.prefix, "card payment");
        assert_eq!(
            p.matches.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(p.protected_count, 1);
        assert_eq!(confirm(&mut db, 1, p.prefix, vec![1, 2], true).unwrap(), 2);
        db.execute("INSERT INTO transactions VALUES(8,1,'CHF',-350,'Card payment · ref 88','2026-03-01',2)",[]).unwrap();
        apply_import(&db, 2).unwrap();
        assert_eq!(db.query_row("SELECT is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=8",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?))).unwrap(),(true,false));
        crate::storage::banking::reporting_flags::set_settlement(&db, 8, false).unwrap();
        apply_import(&db, 2).unwrap();
        assert_eq!(db.query_row("SELECT is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=8",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?))).unwrap(),(false,true));
        db.execute("DELETE FROM settlement_rules", []).unwrap();
        db.execute("INSERT INTO transactions VALUES(9,1,'CHF',-400,'Card payment · ref 99','2026-04-01',3)",[]).unwrap();
        apply_import(&db, 3).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transaction_reporting_flags WHERE transaction_id=9",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
    #[test]
    fn stale_preview_is_rejected_atomically_and_future_is_opt_in() {
        let mut db = fixture();
        assert!(confirm(&mut db, 1, "card payment".into(), vec![1], true).is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM settlement_rules", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transaction_reporting_flags",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(preview(&db, 1, Some("card".into())).is_err());
        assert!(preview(&db, 1, Some("unrelated text".into())).is_err());
        confirm(&mut db, 1, "card payment".into(), vec![1, 2], false).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM settlement_rules", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
