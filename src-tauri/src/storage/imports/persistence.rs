//! Speichert freigegebene Importe, Buchungen und Salden gemeinsam.
use super::deduplication::transaction_indices_to_insert;
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
            account_ids.insert(currency, id);
            continue;
        }
        transaction
        .execute(
            "INSERT OR IGNORE INTO accounts(institution_id, name, account_type, currency, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![institution_id, request.account_name.trim(), account_type, currency, now],
        )
        .map_err(db_error)?;
        let account_id: i64 = transaction
            .query_row(
                "SELECT id FROM accounts WHERE institution_id = ?1 AND name = ?2 AND currency = ?3",
                params![institution_id, request.account_name.trim(), currency],
                |row| row.get(0),
            )
            .map_err(db_error)?;
        account_ids.insert(currency, account_id);
    }
    let account_id = account_ids[currency];
    let transaction_indices = transaction_indices_to_insert(&transaction, &request, &account_ids)?;
    let inserted_transactions = transaction_indices.len();
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Import");
    let warnings_json =
        serde_json::to_string(&request.statement.warnings).map_err(|error| error.to_string())?;
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
        duplicate: inserted_transactions == 0,
    })
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
