//! Prüft gespeicherte Referenzen und überlappende Buchungen vor der Importfreigabe.
use super::identity::transaction_identity;
use crate::importers::{CARD_PURCHASE_REFERENCE_NAMESPACE, PROVISIONAL_CARD_TRANSACTION_KIND};
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::{
    DuplicateCheck, SaveImportRequest, SuspectedDuplicate, TransactionIdentity,
};
use crate::storage::rules::categorization::apply_categories;
use chrono::NaiveDate;
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fs;
type TransactionKey = (i64, String, Option<String>, i64, String, String);

pub(crate) fn check_import_duplicates(
    storage: &Storage,
    request: SaveImportRequest,
) -> Result<DuplicateCheck, String> {
    let connection = storage.connect().map_err(db_error)?;
    let bytes = fs::read(&request.source_path)
        .map_err(|_| "Die Quelldatei ist nicht mehr verfügbar.".to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    duplicate_check(&connection, &request, &hash)
}

pub(crate) fn is_file_imported(storage: &Storage, path: String) -> Result<bool, String> {
    let connection = storage.connect().map_err(db_error)?;
    let bytes =
        fs::read(path).map_err(|_| "Die Quelldatei ist nicht mehr verfügbar.".to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM import_runs WHERE source_hash = ?1)",
            [hash],
            |row| row.get(0),
        )
        .map_err(db_error)
}

pub(crate) fn duplicate_check(
    connection: &Connection,
    request: &SaveImportRequest,
    hash: &str,
) -> Result<DuplicateCheck, String> {
    let exact_file = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM import_runs WHERE source_hash = ?1)",
            [hash],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if exact_file {
        let mut enriched = 0;
        for row in &request.statement.transactions {
            let (Some(industry), Some(account_id)) = (
                row.industry.as_deref(),
                request.account_ids.get(&row.currency),
            ) else {
                continue;
            };
            enriched += connection
                .execute(
                    "UPDATE transactions SET industry=?1
                 WHERE account_id=?2 AND booking_date=?3
                   AND COALESCE(value_date,'')=COALESCE(?4,'')
                   AND amount_minor=?5 AND currency=?6
                   AND trim(description)=?7 AND (industry IS NULL OR trim(industry)='')",
                    params![
                        industry,
                        account_id,
                        row.booking_date,
                        row.value_date,
                        row.amount_minor,
                        row.currency,
                        row.description.trim()
                    ],
                )
                .map_err(db_error)?;
        }
        if enriched > 0 {
            apply_categories(connection).map_err(db_error)?;
        }
    }
    if request
        .statement
        .transactions
        .iter()
        .all(|row| request.account_ids.contains_key(&row.currency))
    {
        let account_ids = request
            .account_ids
            .iter()
            .map(|(currency, account_id)| (currency.as_str(), *account_id))
            .collect();
        let insertions = transaction_indices_to_insert(connection, request, &account_ids)?;
        let updatable_transactions =
            count_updatable_card_transactions(connection, request, &account_ids)?;
        let suspected_transactions =
            suspected_duplicates(connection, request, &account_ids, &insertions)?;
        return Ok(DuplicateCheck {
            exact_file,
            matching_transactions: request.statement.transactions.len() - insertions.len(),
            updatable_transactions,
            total_transactions: request.statement.transactions.len(),
            suspected_transactions,
        });
    }
    let mut counts = BTreeMap::new();
    let mut identity_occurrences = BTreeMap::new();
    let mut matching_transactions = 0;
    for row in &request.statement.transactions {
        if let Some(account) = request.account_ids.get(&row.currency) {
            let base_identity = transaction_identity(&request.statement, row, *account, 0);
            let occurrence = identity_occurrences
                .entry((*account, base_identity.fallback_fingerprint.clone()))
                .or_insert(0usize);
            let identity = transaction_identity(&request.statement, row, *account, *occurrence);
            *occurrence += 1;
            if stored_identity_exists(connection, *account, &identity)? {
                matching_transactions += 1;
                continue;
            }
            *counts
                .entry((
                    *account,
                    row.booking_date.as_str(),
                    row.value_date.as_deref(),
                    row.amount_minor,
                    row.currency.as_str(),
                    row.description.trim(),
                ))
                .or_insert(0usize) += 1;
        }
    }
    let mut query = connection.prepare("SELECT COUNT(*) FROM transactions WHERE account_id = ?1 AND booking_date = ?2 AND COALESCE(value_date,'') = COALESCE(?3,'') AND amount_minor = ?4 AND currency = ?5 AND trim(description) = ?6").map_err(db_error)?;
    for ((account, date, value_date, amount, currency, description), incoming) in counts {
        let existing: usize = query
            .query_row(
                params![account, date, value_date, amount, currency, description],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        matching_transactions += incoming.min(existing);
    }
    Ok(DuplicateCheck {
        exact_file,
        matching_transactions,
        updatable_transactions: 0,
        total_transactions: request.statement.transactions.len(),
        suspected_transactions: Vec::new(),
    })
}

pub(crate) fn count_updatable_card_transactions(
    connection: &Connection,
    request: &SaveImportRequest,
    account_ids: &std::collections::BTreeMap<&str, i64>,
) -> Result<usize, String> {
    let candidates = request
        .statement
        .transactions
        .iter()
        .filter(|row| {
            row.reference_namespace.as_deref() == Some(CARD_PURCHASE_REFERENCE_NAMESPACE)
                && row.transaction_kind != PROVISIONAL_CARD_TRANSACTION_KIND
                && row.external_reference.is_some()
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(0);
    }
    let mut query = connection
        .prepare(
            "SELECT EXISTS(
               SELECT 1 FROM transaction_metadata
               WHERE account_id=?1 AND reference_namespace=?3
                 AND external_reference=?2
                 AND transaction_kind=?4
             )",
        )
        .map_err(db_error)?;
    let mut count = 0;
    for row in candidates {
        let reference = row.external_reference.as_deref().unwrap();
        let account_id = account_ids[row.currency.as_str()];
        let exists: bool = query
            .query_row(
                params![
                    account_id,
                    reference,
                    CARD_PURCHASE_REFERENCE_NAMESPACE,
                    PROVISIONAL_CARD_TRANSACTION_KIND
                ],
                |result| result.get(0),
            )
            .map_err(db_error)?;
        count += usize::from(exists);
    }
    Ok(count)
}

pub(crate) fn suspected_duplicates(
    connection: &Connection,
    request: &SaveImportRequest,
    account_ids: &std::collections::BTreeMap<&str, i64>,
    insertions: &[(usize, TransactionIdentity)],
) -> Result<Vec<SuspectedDuplicate>, String> {
    let candidate_indices = insertions
        .iter()
        .map(|(index, _)| *index)
        .collect::<std::collections::BTreeSet<_>>();
    let mut query = connection
        .prepare(
            "SELECT rowid,booking_date,description,amount_minor,currency FROM transactions
             WHERE account_id=?1 AND amount_minor=?2 AND currency=?3
               AND date(booking_date) BETWEEN date(?4,'-2 day') AND date(?4,'+2 day')
             ORDER BY ABS(julianday(booking_date)-julianday(?4)), rowid DESC",
        )
        .map_err(db_error)?;
    let mut result = Vec::new();
    for index in candidate_indices {
        let row = &request.statement.transactions[index];
        let account_id = account_ids[row.currency.as_str()];
        let stored = query
            .query_map(
                params![account_id, row.amount_minor, row.currency, row.booking_date],
                |stored| {
                    Ok((
                        stored.get::<_, i64>(0)?,
                        stored.get::<_, String>(1)?,
                        stored.get::<_, String>(2)?,
                        stored.get::<_, i64>(3)?,
                        stored.get::<_, String>(4)?,
                    ))
                },
            )
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?
            .into_iter()
            .find(|(_, date, description, _, _)| {
                suspicious_match(&row.booking_date, &row.description, date, description)
            });
        if let Some((id, date, description, amount, currency)) = stored {
            result.push(suspect(
                index,
                "stored",
                Some(id),
                None,
                date,
                description,
                amount,
                currency,
                &row.booking_date,
            ));
            continue;
        }
        if let Some((compared_index, previous)) = request
            .statement
            .transactions
            .iter()
            .enumerate()
            .take(index)
            .rev()
            .find(|(_, previous)| {
                request.account_ids.get(&previous.currency) == Some(&account_id)
                    && previous.amount_minor == row.amount_minor
                    && previous.currency == row.currency
                    && suspicious_match(
                        &row.booking_date,
                        &row.description,
                        &previous.booking_date,
                        &previous.description,
                    )
            })
        {
            result.push(suspect(
                index,
                "current_file",
                None,
                Some(compared_index),
                previous.booking_date.clone(),
                previous.description.clone(),
                previous.amount_minor,
                previous.currency.clone(),
                &row.booking_date,
            ));
        }
    }
    Ok(result)
}

fn suspect(
    transaction_index: usize,
    match_source: &str,
    existing_transaction_id: Option<i64>,
    compared_transaction_index: Option<usize>,
    booking_date: String,
    description: String,
    amount_minor: i64,
    currency: String,
    incoming_date: &str,
) -> SuspectedDuplicate {
    SuspectedDuplicate {
        transaction_index,
        match_source: match_source.into(),
        existing_transaction_id,
        compared_transaction_index,
        match_kind: if booking_date == incoming_date {
            "same_day"
        } else {
            "nearby_day"
        }
        .into(),
        booking_date,
        description,
        amount_minor,
        currency,
    }
}

fn dates_are_close(left: &str, right: &str) -> bool {
    let (Ok(left), Ok(right)) = (
        NaiveDate::parse_from_str(left, "%Y-%m-%d"),
        NaiveDate::parse_from_str(right, "%Y-%m-%d"),
    ) else {
        return left == right;
    };
    (left - right).num_days().abs() <= 2
}

fn suspicious_match(
    incoming_date: &str,
    incoming_description: &str,
    compared_date: &str,
    compared_description: &str,
) -> bool {
    if !dates_are_close(incoming_date, compared_date) {
        return false;
    }
    let incoming = normalized_description(incoming_description);
    let compared = normalized_description(compared_description);
    incoming.is_empty() || compared.is_empty() || incoming == compared
}

fn normalized_description(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn stored_identity_exists(
    connection: &Connection,
    account_id: i64,
    identity: &TransactionIdentity,
) -> Result<bool, String> {
    if let (Some(namespace), Some(reference)) = (
        identity.namespace.as_deref(),
        identity.external_reference.as_deref(),
    ) {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transaction_metadata
                 WHERE account_id=?1 AND reference_namespace=?2 AND external_reference=?3)",
                params![account_id, namespace, reference],
                |row| row.get(0),
            )
            .map_err(db_error)
    } else {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transaction_metadata
                 WHERE account_id=?1 AND external_reference IS NULL AND fallback_fingerprint=?2)",
                params![account_id, identity.fallback_fingerprint],
                |row| row.get(0),
            )
            .map_err(db_error)
    }
}

pub(crate) fn transaction_indices_to_insert(
    connection: &Connection,
    request: &SaveImportRequest,
    account_ids: &std::collections::BTreeMap<&str, i64>,
) -> Result<Vec<(usize, TransactionIdentity)>, String> {
    let mut existing_counts = BTreeMap::<TransactionKey, usize>::new();
    let mut count_query = connection
        .prepare(
            "SELECT COUNT(*) FROM transactions
         WHERE account_id = ?1 AND booking_date = ?2 AND amount_minor = ?3
           AND COALESCE(value_date,'') = COALESCE(?4,'')
           AND currency = ?5 AND trim(description) = ?6",
        )
        .map_err(db_error)?;
    for row in &request.statement.transactions {
        let key = (
            account_ids[row.currency.as_str()],
            row.booking_date.clone(),
            row.value_date.clone(),
            row.amount_minor,
            row.currency.clone(),
            row.description.trim().to_string(),
        );
        if existing_counts.contains_key(&key) {
            continue;
        }
        let count = count_query
            .query_row(
                params![key.0, key.1, key.3, key.2, key.4, key.5],
                |result| result.get(0),
            )
            .map_err(db_error)?;
        existing_counts.insert(key, count);
    }

    let mut identities = Vec::with_capacity(request.statement.transactions.len());
    let mut identity_occurrences = BTreeMap::new();
    for row in &request.statement.transactions {
        let account_id = account_ids[row.currency.as_str()];
        let base_identity = transaction_identity(&request.statement, row, account_id, 0);
        let occurrence = identity_occurrences
            .entry((account_id, base_identity.fallback_fingerprint.clone()))
            .or_insert(0usize);
        identities.push(transaction_identity(
            &request.statement,
            row,
            account_id,
            *occurrence,
        ));
        *occurrence += 1;
    }

    let mut encountered = BTreeMap::<TransactionKey, usize>::new();
    let mut encountered_identities = std::collections::BTreeSet::new();
    let mut indices = Vec::new();
    for (index, row) in request.statement.transactions.iter().enumerate() {
        let account_id = account_ids[row.currency.as_str()];
        let key = (
            account_id,
            row.booking_date.clone(),
            row.value_date.clone(),
            row.amount_minor,
            row.currency.clone(),
            row.description.trim().to_string(),
        );
        let occurrence = encountered.entry(key.clone()).or_default();
        let exceeds_legacy_count = *occurrence >= existing_counts[&key];
        *occurrence += 1;
        let identity = &identities[index];
        if matches!(request.statement.format.as_str(), "CAMT053" | "CAMT054")
            && identity.external_reference.is_some()
        {
            let previous: Option<(i64, String, String)> = connection
                .query_row(
                    "SELECT t.amount_minor,t.currency,t.booking_date FROM transaction_metadata m
                 JOIN transactions t ON t.id=m.transaction_id
                 WHERE m.account_id=?1 AND m.reference_namespace=?2 AND m.external_reference=?3",
                    params![account_id, identity.namespace, identity.external_reference],
                    |result| Ok((result.get(0)?, result.get(1)?, result.get(2)?)),
                )
                .optional()
                .map_err(db_error)?;
            if previous.is_some_and(|(amount, currency, date)| {
                amount != row.amount_minor || currency != row.currency || date != row.booking_date
            }) {
                return Err("camt: Eine bereits importierte Bankreferenz hat ein anderes Buchungsdatum, einen anderen Betrag oder eine andere Währung. Bitte die Dateien prüfen.".into());
            }
        }
        let identity_key = if identity.external_reference.is_some() {
            (
                account_id,
                identity.namespace.clone(),
                identity.external_reference.clone(),
                None,
            )
        } else {
            (
                account_id,
                None,
                None,
                Some(identity.fallback_fingerprint.clone()),
            )
        };
        if stored_identity_exists(connection, account_id, identity)?
            || !encountered_identities.insert(identity_key)
        {
            continue;
        }
        // A new bank reference is a distinct posting even when date, amount
        // and description coincide with another posting.
        let stable_card_reference =
            identity.namespace.as_deref() == Some(CARD_PURCHASE_REFERENCE_NAMESPACE);
        if (identity.external_reference.is_some() && !stable_card_reference) || exceeds_legacy_count
        {
            indices.push((index, identity.clone()));
        }
    }
    Ok(indices)
}
