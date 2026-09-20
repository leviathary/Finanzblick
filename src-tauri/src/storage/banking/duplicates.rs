//! Prüft den bestehenden Buchungsbestand auf mögliche Dubletten und verwaltet reversible Prüfentscheidungen.
//! Die Erkennung verändert keine Buchungen; Entfernen und Freigeben bleiben explizite Nutzeraktionen.
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateAuditTransaction {
    pub id: i64,
    pub booking_date: String,
    pub value_date: Option<String>,
    pub description: String,
    pub amount_minor: i64,
    pub currency: String,
    pub account_name: String,
    pub provider: String,
    pub source_name: String,
    pub imported_at: String,
    pub source_row: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateAuditGroup {
    pub match_kind: String,
    pub transactions: Vec<DuplicateAuditTransaction>,
}

#[derive(Clone)]
struct AuditRow {
    transaction: DuplicateAuditTransaction,
    account_id: i64,
    normalized_description: String,
    reference_namespace: Option<String>,
    external_reference: Option<String>,
}

pub(crate) fn ignore(storage: &Storage, transaction_id: i64) -> Result<(), String> {
    let connection = storage.connect().map_err(db_error)?;
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM transactions WHERE id=?1)",
            [transaction_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)?;
    if !exists {
        return Err("Buchung nicht gefunden.".into());
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO ignored_duplicate_transactions(transaction_id) VALUES(?1)",
            [transaction_id],
        )
        .map_err(db_error)?;
    Ok(())
}

pub(crate) fn audit(storage: &Storage) -> Result<Vec<DuplicateAuditGroup>, String> {
    let connection = storage.connect().map_err(db_error)?;
    audit_from(&connection)
}

pub(crate) fn dismiss_candidate_group(
    storage: &Storage,
    mut transaction_ids: Vec<i64>,
) -> Result<(), String> {
    transaction_ids.sort_unstable();
    transaction_ids.dedup();
    if transaction_ids.len() < 2 {
        return Err(
            "Die geprüfte Duplikatgruppe ist nicht mehr aktuell. Bitte erneut prüfen.".into(),
        );
    }
    let mut connection = storage.connect().map_err(db_error)?;
    let groups = audit_from(&connection)?;
    let is_current_group = groups.iter().any(|group| {
        let mut current = group
            .transactions
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        current.sort_unstable();
        transaction_ids
            .iter()
            .all(|id| current.binary_search(id).is_ok())
    });
    if !is_current_group {
        return Err(
            "Die geprüfte Duplikatgruppe ist nicht mehr aktuell. Bitte erneut prüfen.".into(),
        );
    }
    let transaction = connection.transaction().map_err(db_error)?;
    for (position, left) in transaction_ids.iter().enumerate() {
        for right in transaction_ids.iter().skip(position + 1) {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO duplicate_review_exclusions(left_transaction_id,right_transaction_id) VALUES(?1,?2)",
                    params![left, right],
                )
                .map_err(db_error)?;
        }
    }
    transaction.commit().map_err(db_error)
}

fn audit_from(connection: &Connection) -> Result<Vec<DuplicateAuditGroup>, String> {
    let mut query = connection
        .prepare(
            "SELECT t.id,t.account_id,t.booking_date,t.value_date,t.description,t.amount_minor,
                    t.currency,a.name,i.name,r.source_name,r.imported_at,t.source_row,
                    m.reference_namespace,m.external_reference
             FROM transactions t
             JOIN accounts a ON a.id=t.account_id
             JOIN institutions i ON i.id=a.institution_id
             JOIN import_runs r ON r.id=t.import_id
             LEFT JOIN transaction_metadata m ON m.transaction_id=t.id
             LEFT JOIN ignored_duplicate_transactions ignored ON ignored.transaction_id=t.id
             WHERE ignored.transaction_id IS NULL
             ORDER BY t.account_id,t.currency,t.amount_minor,t.booking_date,t.id",
        )
        .map_err(db_error)?;
    let rows = query
        .query_map([], |row| {
            let description = row.get::<_, String>(4)?;
            Ok(AuditRow {
                transaction: DuplicateAuditTransaction {
                    id: row.get(0)?,
                    booking_date: row.get(2)?,
                    value_date: row.get(3)?,
                    description: description.clone(),
                    amount_minor: row.get(5)?,
                    currency: row.get(6)?,
                    account_name: row.get(7)?,
                    provider: row.get(8)?,
                    source_name: row.get(9)?,
                    imported_at: row.get(10)?,
                    source_row: row.get(11)?,
                },
                account_id: row.get(1)?,
                normalized_description: normalize(&description),
                reference_namespace: row.get(12)?,
                external_reference: row.get(13)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let exclusions = connection
        .prepare("SELECT left_transaction_id,right_transaction_id FROM duplicate_review_exclusions")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .map_err(db_error)?;

    let mut buckets = BTreeMap::<(i64, String, i64), Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        buckets
            .entry((
                row.account_id,
                row.transaction.currency.clone(),
                row.transaction.amount_minor,
            ))
            .or_default()
            .push(index);
    }
    let mut parents = (0..rows.len()).collect::<Vec<_>>();
    for indices in buckets.values() {
        for (position, left_index) in indices.iter().enumerate() {
            for right_index in indices.iter().skip(position + 1) {
                let left = &rows[*left_index];
                let right = &rows[*right_index];
                let day_distance = day_distance(
                    &left.transaction.booking_date,
                    &right.transaction.booking_date,
                );
                if day_distance.is_some_and(|distance| distance > 2) {
                    break;
                }
                let exact_match = rows_match_exact(left, right);
                let possible_match = left.transaction.booking_date
                    == right.transaction.booking_date
                    || (day_distance.is_some_and(|distance| distance <= 2)
                        && descriptions_are_similar(
                            &left.normalized_description,
                            &right.normalized_description,
                        ));
                if !exact_match && !possible_match {
                    continue;
                }
                let id_pair = ordered_pair(left.transaction.id, right.transaction.id);
                if exclusions.contains(&id_pair) {
                    continue;
                }
                union(&mut parents, *left_index, *right_index);
            }
        }
    }
    let mut members = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..rows.len() {
        let root = find(&mut parents, index);
        members.entry(root).or_default().push(index);
    }
    let mut groups = members
        .into_values()
        .filter(|indices| indices.len() > 1)
        .map(|indices| {
            let exact = indices.iter().enumerate().all(|(position, left)| {
                indices
                    .iter()
                    .skip(position + 1)
                    .all(|right| rows_match_exact(&rows[*left], &rows[*right]))
            });
            let mut transactions = indices
                .into_iter()
                .map(|index| rows[index].transaction.clone())
                .collect::<Vec<_>>();
            transactions.sort_by(|left, right| {
                right
                    .booking_date
                    .cmp(&left.booking_date)
                    .then_with(|| right.id.cmp(&left.id))
            });
            DuplicateAuditGroup {
                match_kind: if exact { "exact" } else { "possible" }.into(),
                transactions,
            }
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        (left.match_kind != "exact")
            .cmp(&(right.match_kind != "exact"))
            .then_with(|| {
                right.transactions[0]
                    .booking_date
                    .cmp(&left.transactions[0].booking_date)
            })
    });
    Ok(groups)
}

fn rows_match_exact(left: &AuditRow, right: &AuditRow) -> bool {
    (left.external_reference.is_some()
        && left.external_reference == right.external_reference
        && left.reference_namespace == right.reference_namespace)
        || (left.transaction.booking_date == right.transaction.booking_date
            && !left.normalized_description.is_empty()
            && left.normalized_description == right.normalized_description)
}

fn ordered_pair(left: i64, right: i64) -> (i64, i64) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn day_distance(left: &str, right: &str) -> Option<i64> {
    let left = NaiveDate::parse_from_str(left, "%Y-%m-%d").ok()?;
    let right = NaiveDate::parse_from_str(right, "%Y-%m-%d").ok()?;
    Some((left - right).num_days().abs())
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn descriptions_are_similar(left: &str, right: &str) -> bool {
    let left = left
        .split_whitespace()
        .filter(|token| token.len() > 1)
        .collect::<BTreeSet<_>>();
    let right = right
        .split_whitespace()
        .filter(|token| token.len() > 1)
        .collect::<BTreeSet<_>>();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let common = left.intersection(&right).count();
    common >= 2 && common * 4 >= left.len().max(right.len()) * 3
}

fn find(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find(parents, parents[index]);
    }
    parents[index]
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left = find(parents, left);
    let right = find(parents, right);
    if left != right {
        parents[right] = left;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database::schema::initialize_schema;

    #[test]
    fn audit_groups_legacy_duplicates_and_remembers_review_decisions() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::test_storage(directory.path().join("audit.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'ubs','UBS','bank','2026-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Privatkonto','checking','CHF','2026-01-01');
             INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json)
               VALUES(1,1,'alt.pdf','PDF','old-a','2026-08-25',2,'[]'),(2,1,'neu.xml','CAMT053','old-b','2026-08-26',2,'[]');
             INSERT INTO transactions(id,account_id,import_id,booking_date,value_date,description,amount_minor,currency,confidence,source_row)
               VALUES(1,1,1,'2026-08-24','2026-08-24','Steuerverwaltung des Kantons Bern',-59000,'CHF',1,1),
                     (2,1,2,'2026-08-24','2026-08-24','Steuerverwaltung des Kantons Bern',-59000,'CHF',1,1),
                     (3,1,1,'2026-07-01','2026-07-01','STWEG Panoramastrasse',-171400,'CHF',1,2),
                     (4,1,2,'2026-07-01','2026-07-01','Andere Bezeichnung',-171400,'CHF',1,2);",
        )
        .unwrap();
        drop(connection);

        let groups = audit(&storage).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].match_kind, "exact");
        assert_eq!(groups[1].match_kind, "possible");

        dismiss_candidate_group(&storage, vec![3, 4]).unwrap();
        let groups = audit(&storage).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].transactions.len(), 2);

        ignore(&storage, 2).unwrap();
        assert!(audit(&storage).unwrap().is_empty());
    }
}
