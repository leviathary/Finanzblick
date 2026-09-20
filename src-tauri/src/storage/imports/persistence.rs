//! Speichert freigegebene Importe, Buchungen und Salden gemeinsam.
use super::deduplication::{suspected_duplicates, transaction_indices_to_insert};
use crate::importers::{CARD_PURCHASE_REFERENCE_NAMESPACE, PROVISIONAL_CARD_TRANSACTION_KIND};
use crate::storage::banking::cards;
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::{SaveImportRequest, SaveImportResult};
use crate::storage::rules::categorization::apply_categories;
use crate::storage::rules::settlement_rules;
use chrono::Utc;
use rusqlite::params;
use rusqlite::OptionalExtension;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;

pub(crate) fn save_import_to(
    storage: &Storage,
    request: SaveImportRequest,
) -> Result<SaveImportResult, String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let source = PathBuf::from(&request.source_path);
    let bytes =
        fs::read(&source).map_err(|_| "Die Quelldatei ist nicht mehr verfügbar.".to_string())?;
    let source_hash = format!("{:x}", Sha256::digest(&bytes));

    if let Some((import_id, account_id, count)) = connection
        .query_row(
            "SELECT id, account_id, transaction_count FROM import_runs WHERE source_hash = ?1",
            [&source_hash],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)?)),
        )
        .optional()
        .map_err(db_error)?
    {
        return Ok(SaveImportResult {
            import_id,
            account_id,
            inserted_transactions: count as usize,
            updated_transactions: 0,
            duplicate: true,
        });
    }

    let transaction = connection.transaction().map_err(db_error)?;
    let now = Utc::now().to_rfc3339();
    let (institution_name, institution_type, account_type) =
        provider_metadata(&request.statement.provider);
    let account_type = request
        .statement
        .account_type
        .as_deref()
        .unwrap_or(account_type);
    transaction
        .execute(
            "INSERT OR IGNORE INTO institutions(provider_key, name, institution_type, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                request.statement.provider,
                institution_name,
                institution_type,
                now
            ],
        )
        .map_err(db_error)?;
    let institution_id: i64 = transaction
        .query_row(
            "SELECT id FROM institutions WHERE provider_key = ?1",
            [&request.statement.provider],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let currency = request
        .statement
        .currency_balances
        .first()
        .map(|row| row.currency.as_str())
        .or_else(|| {
            request
                .statement
                .transactions
                .first()
                .map(|row| row.currency.as_str())
        })
        .unwrap_or("CHF");
    let mut currencies: std::collections::BTreeSet<&str> = request
        .statement
        .transactions
        .iter()
        .map(|row| row.currency.as_str())
        .collect();
    currencies.extend(
        request
            .statement
            .currency_balances
            .iter()
            .map(|balance| balance.currency.as_str()),
    );
    currencies.insert(currency);
    let mut account_ids = std::collections::BTreeMap::new();
    for currency in currencies {
        if !request.account_ids.is_empty() {
            let id = *request
                .account_ids
                .get(currency)
                .ok_or_else(|| format!("Bitte ein Konto für {currency} auswählen."))?;
            let valid: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1 AND institution_id = ?2 AND currency = ?3 AND is_active = 1 AND (?4 IS NULL OR account_type = ?4))",
                params![id, institution_id, currency, request.statement.account_type], |row| row.get(0),
            ).map_err(db_error)?;
            if !valid {
                return Err(format!("Das gewählte {currency}-Konto passt nicht zu Anbieter, Kontotyp oder Währung oder ist archiviert."));
            }
            validate_account_reference(
                &transaction,
                id,
                request.statement.account_reference.as_deref(),
            )?;
            account_ids.insert(currency, id);
            continue;
        }
        transaction
        .execute(
            "INSERT OR IGNORE INTO accounts(institution_id, name, account_type, currency, created_at, external_reference)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![institution_id, request.account_name.trim(), account_type, currency, now, request.statement.account_reference.as_deref()],
        )
        .map_err(db_error)?;
        let account_id: i64 = transaction
            .query_row(
                "SELECT id FROM accounts WHERE institution_id = ?1 AND name = ?2 AND currency = ?3",
                params![institution_id, request.account_name.trim(), currency],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        validate_account_reference(
            &transaction,
            account_id,
            request.statement.account_reference.as_deref(),
        )?;
        account_ids.insert(currency, account_id);
    }
    let account_id = account_ids[currency];
    let updated_transactions =
        reconcile_provisional_card_transactions(&transaction, &request, &account_ids)?;
    let transaction_indices = transaction_indices_to_insert(&transaction, &request, &account_ids)?;
    let suspected =
        suspected_duplicates(&transaction, &request, &account_ids, &transaction_indices)?;
    let suspected_indices = suspected
        .iter()
        .map(|item| item.transaction_index)
        .collect::<std::collections::BTreeSet<_>>();
    let mut resolutions = std::collections::BTreeMap::new();
    for resolution in &request.duplicate_resolutions {
        if !matches!(resolution.action.as_str(), "keep" | "skip")
            || !suspected_indices.contains(&resolution.transaction_index)
        {
            return Err(
                "Eine Duplikatentscheidung ist ungültig. Bitte die Vorschau erneut prüfen.".into(),
            );
        }
        if resolutions
            .insert(resolution.transaction_index, resolution.action.as_str())
            .is_some()
        {
            return Err("Eine verdächtige Buchung wurde mehrfach entschieden. Bitte die Vorschau erneut prüfen.".into());
        }
    }
    if suspected
        .iter()
        .any(|item| !resolutions.contains_key(&item.transaction_index))
    {
        return Err("Mögliche Duplikate müssen vor dem Import vollständig geprüft werden.".into());
    }
    let transaction_indices = transaction_indices
        .into_iter()
        .filter(|(index, _)| resolutions.get(index).copied() != Some("skip"))
        .collect::<Vec<_>>();
    let inserted_transactions = transaction_indices.len();
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Import");
    let mut import_warnings = request.statement.warnings.clone();
    if updated_transactions > 0 {
        import_warnings.push(format!(
            "{updated_transactions} vorläufige Kreditkartenbuchungen wurden mit den endgültigen Abrechnungsdaten aktualisiert."
        ));
    }
    let warnings_json =
        serde_json::to_string(&import_warnings).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO import_runs(account_id, source_name, source_format, source_hash, imported_at, transaction_count, warnings_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![account_id, source_name, request.statement.format, source_hash, now, inserted_transactions as i64, warnings_json],
        )
        .map_err(db_error)?;
    let import_id = transaction.last_insert_rowid();
    let source_type = format!(
        "{}_{}",
        request.statement.provider.to_lowercase(),
        request
            .statement
            .format
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
    );
    transaction
        .execute(
            "INSERT INTO import_document_metadata(import_id,source_type,provider,document_type,document_date,value_date,account_reference,record_definition_id)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                import_id,
                source_type,
                request.statement.provider,
                request.statement.document_type,
                request.statement.document_date,
                request.statement.document_value_date,
                request.statement.account_reference,
                request.statement.record_definition_id
            ],
        )
        .map_err(db_error)?;
    {
        let mut snapshot = transaction
            .prepare(
                "INSERT OR REPLACE INTO balance_snapshots(account_id, import_id, balance_date, amount_minor, currency) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(db_error)?;
        for balance in &request.statement.currency_balances {
            if let Some(opening_date) = &balance.opening_date {
                snapshot
                    .execute(params![
                        account_ids[balance.currency.as_str()],
                        import_id,
                        opening_date,
                        balance.opening_balance_minor,
                        balance.currency
                    ])
                    .map_err(db_error)?;
            }
        }
    }
    {
        let mut insert_transaction = transaction
            .prepare("INSERT INTO transactions(account_id, import_id, booking_date, value_date, description, industry, amount_minor, balance_minor, currency, confidence, source_row) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)")
            .map_err(db_error)?;
        let mut insert_metadata = transaction
            .prepare("INSERT INTO transaction_metadata(transaction_id,account_id,transaction_kind,reference_namespace,external_reference,fallback_fingerprint,counterparty_name,remittance_information) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)")
            .map_err(db_error)?;
        let mut insert_security = transaction
            .prepare("INSERT INTO security_transactions(transaction_id,isin,valor_number,quantity,price,price_currency,exchange_rate,gross_amount_minor,fees_minor,taxes_minor,withholding_tax_minor,accrued_interest_minor) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)")
            .map_err(db_error)?;
        for (index, identity) in &transaction_indices {
            let row = &request.statement.transactions[*index];
            let target_account_id = account_ids[row.currency.as_str()];
            insert_transaction
                .execute(params![
                    target_account_id,
                    import_id,
                    row.booking_date,
                    row.value_date,
                    row.description,
                    row.industry,
                    row.amount_minor,
                    row.balance_minor,
                    row.currency,
                    row.confidence,
                    row.source_row as i64
                ])
                .map_err(db_error)?;
            let transaction_id = transaction.last_insert_rowid();
            insert_metadata
                .execute(params![
                    transaction_id,
                    target_account_id,
                    row.transaction_kind,
                    identity.namespace,
                    identity.external_reference,
                    identity.fallback_fingerprint,
                    row.counterparty_name,
                    row.remittance_information
                ])
                .map_err(db_error)?;
            if let Some(details) = &row.security_details {
                insert_security
                    .execute(params![
                        transaction_id,
                        details.isin,
                        details.valor_number,
                        details.quantity,
                        details.price,
                        details.price_currency,
                        details.exchange_rate,
                        details.gross_amount_minor,
                        details.fees_minor,
                        details.taxes_minor,
                        details.withholding_tax_minor,
                        details.accrued_interest_minor
                    ])
                    .map_err(db_error)?;
            }
        }
    }
    {
        let mut snapshot = transaction
            .prepare(
                "INSERT OR REPLACE INTO balance_snapshots(account_id, import_id, balance_date, amount_minor, currency) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(db_error)?;
        for row in &request.statement.transactions {
            if let Some(balance) = row.balance_minor {
                snapshot
                    .execute(params![
                        account_ids[row.currency.as_str()],
                        import_id,
                        row.booking_date,
                        balance,
                        row.currency
                    ])
                    .map_err(db_error)?;
            }
        }
    }
    if request.statement.currency_balances.is_empty() {
        if let Some(balance) = request.statement.closing_balance_minor {
            let balance_date = request
                .statement
                .transactions
                .last()
                .map(|row| row.booking_date.as_str())
                .unwrap_or(now.as_str());
            transaction.execute(
            "INSERT OR REPLACE INTO balance_snapshots(account_id, import_id, balance_date, amount_minor, currency) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account_id, import_id, balance_date, balance, currency],
        ).map_err(db_error)?;
        }
    } else {
        for balance in &request.statement.currency_balances {
            transaction.execute(
                "INSERT OR REPLACE INTO balance_snapshots(account_id, import_id, balance_date, amount_minor, currency) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![account_ids[balance.currency.as_str()], import_id, balance.closing_date, balance.closing_balance_minor, balance.currency],
            ).map_err(db_error)?;
        }
    }
    apply_categories(&transaction).map_err(db_error)?;
    cards::apply_import(&transaction, import_id)?;
    settlement_rules::apply_import(&transaction, import_id)?;
    transaction.commit().map_err(db_error)?;
    Ok(SaveImportResult {
        import_id,
        account_id,
        inserted_transactions,
        updated_transactions,
        duplicate: inserted_transactions == 0
            && updated_transactions == 0
            && request.statement.currency_balances.is_empty(),
    })
}

fn validate_account_reference(
    connection: &rusqlite::Transaction<'_>,
    account_id: i64,
    statement_reference: Option<&str>,
) -> Result<(), String> {
    let Some(expected) = statement_reference
        .map(normalize_account_reference)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let stored = connection
        .query_row(
            "SELECT external_reference FROM accounts WHERE id=?1",
            [account_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(db_error)?
        .map(|value| normalize_account_reference(&value));
    if stored.as_deref() != Some(expected.as_str()) {
        return Err("Die Kontokennung im Auszug stimmt nicht mit der beim gewählten Konto hinterlegten IBAN oder Kontoreferenz überein. Bitte die Kontozuordnung prüfen.".into());
    }
    Ok(())
}

fn normalize_account_reference(value: &str) -> String {
    let trimmed = value.trim();
    let without_label = if trimmed
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("IBAN"))
    {
        trimmed[4..].trim_start_matches([':', ' '])
    } else {
        trimmed
    };
    without_label
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

fn reconcile_provisional_card_transactions(
    connection: &rusqlite::Transaction<'_>,
    request: &SaveImportRequest,
    account_ids: &std::collections::BTreeMap<&str, i64>,
) -> Result<usize, String> {
    let mut find = connection
        .prepare(
            "SELECT transaction_id FROM transaction_metadata
             WHERE account_id=?1 AND reference_namespace=?3
               AND external_reference=?2
               AND transaction_kind=?4",
        )
        .map_err(db_error)?;
    let mut updated = 0;
    for row in &request.statement.transactions {
        if row.reference_namespace.as_deref() != Some(CARD_PURCHASE_REFERENCE_NAMESPACE)
            || row.transaction_kind == PROVISIONAL_CARD_TRANSACTION_KIND
        {
            continue;
        }
        let Some(reference) = row.external_reference.as_deref() else {
            continue;
        };
        let account_id = account_ids[row.currency.as_str()];
        let transaction_id = find
            .query_row(
                params![
                    account_id,
                    reference,
                    CARD_PURCHASE_REFERENCE_NAMESPACE,
                    PROVISIONAL_CARD_TRANSACTION_KIND
                ],
                |result| result.get::<_, i64>(0),
            )
            .optional()
            .map_err(db_error)?;
        let Some(transaction_id) = transaction_id else {
            continue;
        };
        connection
            .execute(
                "UPDATE transactions SET
                   booking_date=?1,value_date=?2,description=?3,industry=?4,
                   amount_minor=?5,balance_minor=?6,currency=?7,confidence=?8
                 WHERE id=?9",
                params![
                    row.booking_date,
                    row.value_date,
                    row.description,
                    row.industry,
                    row.amount_minor,
                    row.balance_minor,
                    row.currency,
                    row.confidence,
                    transaction_id
                ],
            )
            .map_err(db_error)?;
        connection
            .execute(
                "UPDATE transaction_metadata SET
                   transaction_kind=?1,counterparty_name=?2,remittance_information=?3
                 WHERE transaction_id=?4",
                params![
                    row.transaction_kind,
                    row.counterparty_name,
                    row.remittance_information,
                    transaction_id
                ],
            )
            .map_err(db_error)?;
        updated += 1;
    }
    Ok(updated)
}

pub(crate) fn provider_metadata(provider: &str) -> (&'static str, &'static str, &'static str) {
    match provider {
        "zkb" => ("Zürcher Kantonalbank", "bank", "cash"),
        "ubs" => ("UBS", "bank", "cash"),
        "swissquote" => ("Swissquote", "bank", "cash"),
        "migros" => ("Migros Bank", "bank", "cash"),
        "raiffeisen" => ("Raiffeisen", "bank", "cash"),
        "generali" => ("Generali", "insurance", "pillar3a"),
        _ => ("Unbekannter Anbieter", "bank", "cash"),
    }
}

#[cfg(test)]
mod reference_tests {
    use super::{normalize_account_reference, validate_account_reference};
    use rusqlite::Connection;

    #[test]
    fn normalizes_iban_labels_case_and_spacing() {
        assert_eq!(
            normalize_account_reference(" iban: ch26 0029 2292 6049 4440 d "),
            "CH260029229260494440D"
        );
    }

    #[test]
    fn rejects_selected_accounts_with_a_different_or_missing_reference() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE accounts(id INTEGER PRIMARY KEY, external_reference TEXT);
                 INSERT INTO accounts VALUES(1, 'CH26 0029 2292 6049 4440 D');
                 INSERT INTO accounts VALUES(2, NULL);",
            )
            .unwrap();
        let transaction = connection.transaction().unwrap();

        assert!(
            validate_account_reference(&transaction, 1, Some("IBAN: CH260029229260494440D"))
                .is_ok()
        );
        assert!(
            validate_account_reference(&transaction, 1, Some("CH9300762011623852957")).is_err()
        );
        assert!(
            validate_account_reference(&transaction, 2, Some("CH260029229260494440D")).is_err()
        );
        assert!(validate_account_reference(&transaction, 2, None).is_ok());
    }
}
