//! Prüft gespeicherte Referenzen und überlappende Buchungen vor der Importfreigabe.
use super::identity::transaction_identity;
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::{DuplicateCheck, SaveImportRequest, TransactionIdentity};
use crate::storage::rules::categorization::apply_categories;
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
        return Ok(DuplicateCheck {
            exact_file,
            matching_transactions: request.statement.transactions.len() - insertions.len(),
            total_transactions: request.statement.transactions.len(),
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
        total_transactions: request.statement.transactions.len(),
    })
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
        if request.statement.format == "CAMT053" && identity.external_reference.is_some() {
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
                return Err("camt.053: Eine bereits importierte Bankreferenz hat ein anderes Buchungsdatum, einen anderen Betrag oder eine andere Währung. Bitte die Auszüge prüfen.".into());
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
        if identity.external_reference.is_some() || exceeds_legacy_count {
            indices.push((index, identity.clone()));
        }
    }
    Ok(indices)
}
