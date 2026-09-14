//! Lokale SQLite-Persistenz und Initialisierung des aktuellen Datenmodells.
mod card_settlements;
pub mod categories;
pub mod market_data;
pub mod reconciliation;
pub mod security;
pub mod tax_history;

use crate::importers::ParsedStatement;
use chrono::{Duration, Local, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, collections::HashMap, collections::HashSet, fs, path::PathBuf};
use tauri::State;
#[cfg(test)]
mod duplicate_tests;
mod merchant_rules;

pub struct Storage {
    path: PathBuf,
    session: std::sync::RwLock<security::Session>,
    _lock: Option<fs::File>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRun {
    id: i64,
    source_name: String,
    source_format: String,
    provider: String,
    accounts: String,
    imported_at: String,
    transaction_count: i64,
    first_date: Option<String>,
    last_date: Option<String>,
}

#[tauri::command]
pub fn list_imports(storage: State<'_, Storage>) -> Result<Vec<ImportRun>, String> {
    imports_from(&storage)
}

fn imports_from(storage: &Storage) -> Result<Vec<ImportRun>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection.prepare("SELECT ir.id, ir.source_name, ir.source_format, i.name,
        (SELECT group_concat(a.name || ' (' || a.currency || ')', ', ') FROM accounts a
          WHERE a.id = ir.account_id OR EXISTS(SELECT 1 FROM transactions t WHERE t.import_id = ir.id AND t.account_id = a.id)
          OR EXISTS(SELECT 1 FROM balance_snapshots b WHERE b.import_id = ir.id AND b.account_id = a.id)),
        ir.imported_at, (SELECT COUNT(*) FROM transactions t WHERE t.import_id = ir.id),
        (SELECT MIN(booking_date) FROM transactions t WHERE t.import_id = ir.id),
        (SELECT MAX(booking_date) FROM transactions t WHERE t.import_id = ir.id)
        FROM import_runs ir JOIN accounts main ON main.id = ir.account_id
        JOIN institutions i ON i.id = main.institution_id ORDER BY ir.imported_at DESC, ir.id DESC").map_err(db_error)?;
    let rows = query
        .query_map([], |row| {
            Ok(ImportRun {
                id: row.get(0)?,
                source_name: row.get(1)?,
                source_format: row.get(2)?,
                provider: row.get(3)?,
                accounts: row.get(4)?,
                imported_at: row.get(5)?,
                transaction_count: row.get(6)?,
                first_date: row.get(7)?,
                last_date: row.get(8)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

#[tauri::command]
pub fn delete_imports(storage: State<'_, Storage>, ids: Vec<i64>) -> Result<usize, String> {
    delete_imports_from(&storage, ids)
}

fn delete_imports_from(storage: &Storage, ids: Vec<i64>) -> Result<usize, String> {
    let ids: std::collections::BTreeSet<i64> = ids.into_iter().collect();
    if ids.is_empty() {
        return Err("Bitte mindestens einen Import auswählen.".into());
    }
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    for id in &ids {
        // Foreign-key cascades remove every associated transaction and snapshot,
        // including all currency accounts. Account records and source files stay.
        if transaction
            .execute("DELETE FROM import_runs WHERE id = ?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("Ein ausgewählter Import existiert nicht mehr. Bitte die Liste aktualisieren; es wurde nichts gelöscht.".into());
        }
    }
    transaction.commit().map_err(db_error)?;
    Ok(ids.len())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportRequest {
    #[serde(default)]
    pub account_ids: BTreeMap<String, i64>,
    pub source_path: String,
    pub account_name: String,
    pub statement: ParsedStatement,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportResult {
    pub import_id: i64,
    pub account_id: i64,
    pub inserted_transactions: usize,
    pub duplicate: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCheck {
    exact_file: bool,
    matching_transactions: usize,
    total_transactions: usize,
}

#[tauri::command]
pub fn check_import_duplicates(
    storage: State<'_, Storage>,
    request: SaveImportRequest,
) -> Result<DuplicateCheck, String> {
    let connection = storage.connect().map_err(db_error)?;
    let bytes = fs::read(&request.source_path)
        .map_err(|_| "Die Quelldatei ist nicht mehr verfügbar.".to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    duplicate_check(&connection, &request, &hash)
}

#[tauri::command]
pub fn is_file_imported(storage: State<'_, Storage>, path: String) -> Result<bool, String> {
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

fn duplicate_check(
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
    let mut counts = BTreeMap::new();
    for row in &request.statement.transactions {
        if let Some(account) = request.account_ids.get(&row.currency) {
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
    let mut matching_transactions = 0;
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

type TransactionKey = (i64, String, Option<String>, i64, String, String);

fn transaction_indices_to_insert(
    connection: &Connection,
    request: &SaveImportRequest,
    account_ids: &std::collections::BTreeMap<&str, i64>,
) -> Result<Vec<usize>, String> {
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

    let mut encountered = BTreeMap::<TransactionKey, usize>::new();
    let mut indices = Vec::new();
    for (index, row) in request.statement.transactions.iter().enumerate() {
        let key = (
            account_ids[row.currency.as_str()],
            row.booking_date.clone(),
            row.value_date.clone(),
            row.amount_minor,
            row.currency.clone(),
            row.description.trim().to_string(),
        );
        let occurrence = encountered.entry(key.clone()).or_default();
        if *occurrence >= existing_counts[&key] {
            indices.push(index);
        }
        *occurrence += 1;
    }
    Ok(indices)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub path: String,
    pub accounts: i64,
    pub imports: i64,
    pub transactions: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardAccount {
    pub id: i64,
    pub provider: String,
    pub provider_key: String,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub balance_currency: String,
    pub balance_minor: Option<i64>,
    pub balance_date: Option<String>,
    pub include_in_net_worth: bool,
    pub logo_data_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAccount {
    pub id: i64,
    pub institution_id: i64,
    pub provider: String,
    pub provider_key: String,
    pub institution_type: String,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
    pub is_active: bool,
    pub include_in_net_worth: bool,
    pub balance_minor: Option<i64>,
    pub balance_date: Option<String>,
    pub balance_currency: String,
    pub import_count: i64,
    pub manual_valuation_count: i64,
    pub manual_quantity: Option<f64>,
    pub manual_unit_price_minor: Option<i64>,
    pub manual_quote_currency: Option<String>,
    pub manual_exchange_rate: Option<f64>,
    pub logo_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub institution_name: String,
    pub institution_type: String,
    pub account_name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
    pub is_active: bool,
    pub include_in_net_worth: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualValuationRequest {
    pub id: Option<i64>,
    pub account_id: i64,
    pub label: String,
    pub valuation_date: String,
    pub amount_minor: i64,
    pub quantity: Option<f64>,
    pub unit_price_minor: Option<i64>,
    pub quote_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub asset_type: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier: Option<String>,
    pub holding_start_date: Option<String>,
    pub holding_end_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualPosition {
    pub id: i64,
    pub account_id: i64,
    pub label: String,
    pub valuation_date: String,
    pub amount_minor: i64,
    pub value_currency: String,
    pub quantity: Option<f64>,
    pub unit_price_minor: Option<i64>,
    pub quote_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub asset_type: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier: Option<String>,
    pub price_source: Option<String>,
    pub holding_start_date: Option<String>,
    pub holding_end_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardProvider {
    pub provider: String,
    pub provider_key: String,
    pub balance_minor: i64,
    pub account_count: i64,
    pub logo_data_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentImport {
    pub source_name: String,
    pub provider: String,
    pub account_name: String,
    pub imported_at: String,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total_balance_minor: i64,
    pub currency: String,
    pub accounts: Vec<DashboardAccount>,
    pub providers: Vec<DashboardProvider>,
    pub recent_import: Option<RecentImport>,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthHistoryPoint {
    pub date: String,
    pub total_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthBreakdown {
    pub key: String,
    pub label: String,
    pub amount_minor: i64,
    pub account_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthData {
    pub currency: String,
    pub current_total_minor: i64,
    pub first_total_minor: Option<i64>,
    pub change_minor: Option<i64>,
    pub history: Vec<WealthHistoryPoint>,
    pub by_type: Vec<WealthBreakdown>,
    pub by_provider: Vec<WealthBreakdown>,
    pub accounts: Vec<ManagedAccount>,
    pub excluded_account_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySpend {
    pub key: String,
    pub label: String,
    pub color: String,
    pub amount_minor: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySpend {
    pub month: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryPoint {
    pub date: String,
    pub total_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedTransaction {
    pub id: i64,
    pub booking_date: String,
    pub description: String,
    pub industry: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub category_key: String,
    pub category_label: String,
    pub category_color: String,
    pub category_source: String,
    pub provider: String,
    pub provider_key: String,
    pub account_name: String,
    pub excluded_from_totals: bool,
    pub is_card_settlement: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionAnalysis {
    pub total_income_minor: i64,
    pub income_count: i64,
    pub income_transactions: Vec<AnalyzedTransaction>,
    pub total_spend_minor: i64,
    pub transaction_count: i64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub categories: Vec<CategorySpend>,
    pub months: Vec<MonthlySpend>,
    pub history: Vec<TransactionHistoryPoint>,
    pub transactions: Vec<AnalyzedTransaction>,
    pub providers: Vec<DashboardProvider>,
}

fn initialize_schema(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS app_settings (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS institutions (
           id INTEGER PRIMARY KEY, provider_key TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
           institution_type TEXT NOT NULL, logo_data_url TEXT, created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS accounts (
           id INTEGER PRIMARY KEY, institution_id INTEGER NOT NULL REFERENCES institutions(id),
           name TEXT NOT NULL, account_type TEXT NOT NULL,
           currency TEXT NOT NULL CHECK(length(currency)=3), external_reference TEXT,
           is_active INTEGER NOT NULL DEFAULT 1, include_in_net_worth INTEGER NOT NULL DEFAULT 1,
           created_at TEXT NOT NULL, UNIQUE(institution_id,name,currency)
         );
         CREATE TABLE IF NOT EXISTS import_runs (
           id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id),
           source_name TEXT NOT NULL, source_format TEXT NOT NULL, source_hash TEXT NOT NULL UNIQUE,
           imported_at TEXT NOT NULL, transaction_count INTEGER NOT NULL, warnings_json TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS categories (
           id INTEGER PRIMARY KEY, category_key TEXT NOT NULL UNIQUE, label TEXT NOT NULL,
           color TEXT NOT NULL, sort_order INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS transactions (
           id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id),
           import_id INTEGER NOT NULL REFERENCES import_runs(id) ON DELETE CASCADE,
           booking_date TEXT NOT NULL, value_date TEXT, description TEXT NOT NULL, industry TEXT,
           amount_minor INTEGER NOT NULL, balance_minor INTEGER, currency TEXT NOT NULL,
           confidence REAL NOT NULL, source_row INTEGER NOT NULL,
           category_id INTEGER REFERENCES categories(id), category_manual INTEGER NOT NULL DEFAULT 0,
           category_source TEXT NOT NULL DEFAULT 'description', UNIQUE(import_id,source_row)
         );
         CREATE INDEX IF NOT EXISTS idx_transactions_account_date ON transactions(account_id,booking_date);
         CREATE TABLE IF NOT EXISTS balance_snapshots (
           id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id),
           import_id INTEGER NOT NULL REFERENCES import_runs(id) ON DELETE CASCADE,
           balance_date TEXT NOT NULL, amount_minor INTEGER NOT NULL, currency TEXT NOT NULL,
           recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
           UNIQUE(import_id,account_id,balance_date)
         );
         CREATE TABLE IF NOT EXISTS industry_category_rules (
           industry_key TEXT PRIMARY KEY, industry_label TEXT NOT NULL,
           category_id INTEGER NOT NULL REFERENCES categories(id)
         );
         CREATE TABLE IF NOT EXISTS merchant_category_rules (
           merchant_key TEXT PRIMARY KEY, category_id INTEGER NOT NULL REFERENCES categories(id)
         );

         CREATE TABLE IF NOT EXISTS instruments (
           id INTEGER PRIMARY KEY, name TEXT NOT NULL, asset_type TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS instrument_identifiers (
           id INTEGER PRIMARY KEY, instrument_id INTEGER NOT NULL REFERENCES instruments(id) ON DELETE CASCADE,
           identifier_type TEXT NOT NULL, identifier TEXT NOT NULL,
           UNIQUE(identifier_type,identifier)
         );
         CREATE TABLE IF NOT EXISTS instrument_listings (
           id INTEGER PRIMARY KEY, instrument_id INTEGER NOT NULL REFERENCES instruments(id) ON DELETE CASCADE,
           exchange_mic TEXT, market_symbol TEXT,
           quote_currency TEXT CHECK(quote_currency IS NULL OR length(quote_currency)=3),
           preferred_price_source TEXT,
           UNIQUE(instrument_id,exchange_mic,market_symbol,quote_currency)
         );
         CREATE TABLE IF NOT EXISTS instrument_prices (
           id INTEGER PRIMARY KEY, listing_id INTEGER NOT NULL REFERENCES instrument_listings(id) ON DELETE CASCADE,
           price_date TEXT NOT NULL, price_at TEXT,
           price_type TEXT NOT NULL CHECK(price_type IN ('eod_close','adjusted_close','official_close','nav','bid','ask','last','manual_valuation')),
           price_amount INTEGER NOT NULL, price_scale INTEGER NOT NULL CHECK(price_scale BETWEEN 0 AND 9),
           currency TEXT NOT NULL CHECK(length(currency)=3), source TEXT NOT NULL, fetched_at TEXT NOT NULL,
           UNIQUE(listing_id,price_date,price_type,source)
         );
         CREATE INDEX IF NOT EXISTS idx_instrument_prices_listing_date ON instrument_prices(listing_id,price_date);
         CREATE TABLE IF NOT EXISTS portfolio_positions (
           id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
           listing_id INTEGER REFERENCES instrument_listings(id), label TEXT NOT NULL, asset_type TEXT NOT NULL,
           holding_start_date TEXT NOT NULL, holding_end_date TEXT,
           created_at TEXT NOT NULL, updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_portfolio_positions_account ON portfolio_positions(account_id);
         CREATE TABLE IF NOT EXISTS position_quantities (
           id INTEGER PRIMARY KEY, position_id INTEGER NOT NULL REFERENCES portfolio_positions(id) ON DELETE CASCADE,
           valid_from TEXT NOT NULL, quantity_amount INTEGER NOT NULL,
           quantity_scale INTEGER NOT NULL CHECK(quantity_scale BETWEEN 0 AND 9),
           source TEXT NOT NULL, recorded_at TEXT NOT NULL, UNIQUE(position_id,valid_from)
         );
         CREATE TABLE IF NOT EXISTS manual_position_values (
           id INTEGER PRIMARY KEY, position_id INTEGER NOT NULL REFERENCES portfolio_positions(id) ON DELETE CASCADE,
           value_date TEXT NOT NULL, amount_minor INTEGER NOT NULL,
           currency TEXT NOT NULL CHECK(length(currency)=3), source TEXT NOT NULL,
           recorded_at TEXT NOT NULL, UNIQUE(position_id,value_date,source)
         );
         CREATE TABLE IF NOT EXISTS fx_rates (
           id INTEGER PRIMARY KEY, base_currency TEXT NOT NULL CHECK(length(base_currency)=3),
           quote_currency TEXT NOT NULL CHECK(length(quote_currency)=3), rate_date TEXT NOT NULL,
           rate_amount INTEGER NOT NULL, rate_scale INTEGER NOT NULL CHECK(rate_scale BETWEEN 0 AND 12),
           source TEXT NOT NULL, fetched_at TEXT NOT NULL,
           UNIQUE(base_currency,quote_currency,rate_date,source)
         );
         CREATE INDEX IF NOT EXISTS idx_fx_rates_pair_date ON fx_rates(base_currency,quote_currency,rate_date);
         CREATE TABLE IF NOT EXISTS daily_valuations (
           id INTEGER PRIMARY KEY, position_id INTEGER NOT NULL REFERENCES portfolio_positions(id) ON DELETE CASCADE,
           valuation_date TEXT NOT NULL, quantity_amount INTEGER, quantity_scale INTEGER,
           instrument_price_id INTEGER REFERENCES instrument_prices(id),
           fx_rate_id INTEGER REFERENCES fx_rates(id), value_minor INTEGER NOT NULL,
           currency TEXT NOT NULL CHECK(length(currency)=3), calculated_at TEXT NOT NULL,
           UNIQUE(position_id,valuation_date)
         );
         CREATE INDEX IF NOT EXISTS idx_daily_valuations_position_date ON daily_valuations(position_id,valuation_date);
         CREATE TABLE IF NOT EXISTS market_sync_state (
           id INTEGER PRIMARY KEY CHECK(id=1), last_attempt_at TEXT, last_success_at TEXT
         );
         CREATE TABLE IF NOT EXISTS annual_tax_snapshots (
           id INTEGER PRIMARY KEY,
           tax_year INTEGER NOT NULL UNIQUE CHECK(tax_year BETWEEN 1990 AND 2100),
           valuation_date TEXT NOT NULL,
           gross_assets_minor INTEGER NOT NULL CHECK(gross_assets_minor >= 0),
           liabilities_minor INTEGER NOT NULL CHECK(liabilities_minor >= 0),
           taxable_wealth_minor INTEGER NOT NULL CHECK(
             taxable_wealth_minor >= 0 AND gross_assets_minor - liabilities_minor = taxable_wealth_minor
           ),
           canton_taxable_wealth_minor INTEGER CHECK(
             canton_taxable_wealth_minor IS NULL OR
             (canton_taxable_wealth_minor >= 0 AND canton_taxable_wealth_minor <= taxable_wealth_minor)
           ),
           currency TEXT NOT NULL DEFAULT 'CHF' CHECK(currency='CHF'),
           source_name TEXT NOT NULL,
           source_hash TEXT NOT NULL,
           parser_version TEXT NOT NULL,
           extraction_confidence REAL NOT NULL,
           imported_at TEXT NOT NULL
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_annual_tax_snapshots_source_hash
           ON annual_tax_snapshots(source_hash);
         CREATE TABLE IF NOT EXISTS annual_tax_snapshot_breakdowns (
           snapshot_id INTEGER PRIMARY KEY REFERENCES annual_tax_snapshots(id) ON DELETE CASCADE,
           securities_and_cash_minor INTEGER NOT NULL CHECK(securities_and_cash_minor >= 0),
           real_estate_minor INTEGER NOT NULL CHECK(real_estate_minor >= 0),
           other_assets_minor INTEGER NOT NULL
         );

         INSERT OR IGNORE INTO categories(category_key,label,color,sort_order) VALUES
           ('housing','Wohnen','#5B7CFA',10),
           ('furnishing','Möbel & Einrichtung','#B7814B',12),
           ('electronics','Elektronik','#527B91',14),
           ('groceries','Lebensmittel & Haushalt','#18A77B',20),
           ('health','Gesundheit','#E25D6A',30),
           ('leisure','Freizeit & Sport','#A66DD4',40),
           ('restaurants','Restaurants','#E07A5F',42),
           ('telecom','Internet & Mobilfunk','#26949A',46),
           ('digital_subscriptions','Digitale Abos','#6474D8',45),
           ('transport','Mobilität','#3D9ED8',50),
           ('credit_card','Kreditkarte','#7C5CFC',55),
           ('travel','Reisen','#F19A3E',60),
           ('taxes','Steuern','#8B6F5A',70),
           ('alimony','Alimente','#B66B87',75),
           ('saving','Sparen & Vorsorge','#087A5B',80),
           ('income','Einkommen','#2F8F62',90),
           ('other','Sonstiges','#8390A1',100);
         COMMIT;",
    )?;
    categories::apply_redirects(connection)?;
    apply_categories(connection)
}

fn apply_categories(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "UPDATE transactions SET category_id = (
           SELECT id FROM categories WHERE category_key = CASE
             WHEN amount_minor > 0 THEN 'income'
             WHEN lower(description) LIKE '%miete%' OR lower(description) LIKE '%hypothek%' THEN 'housing'
             WHEN lower(description) LIKE '%lebensmittel%' OR lower(description) LIKE '%coop%' OR lower(description) LIKE '%migros%' OR lower(description) LIKE '%haushalt%' THEN 'groceries'
             WHEN lower(description) LIKE '%krankenkasse%' OR lower(description) LIKE '%apotheke%' OR lower(description) LIKE '%arzt%' THEN 'health'
             WHEN lower(description) LIKE '%restaurant%' OR lower(description) LIKE '%restaur%'
               OR lower(description) LIKE '%fast-food%' OR lower(description) LIKE '%fast food%'
               OR lower(description) LIKE '%takeaway%' OR lower(description) LIKE '%take-away%'
               OR lower(description) LIKE '%pizzeria%' OR lower(description) LIKE '%bistro%'
               OR lower(description) LIKE '%cafÃ©%' OR lower(description) LIKE '%cafe%'
               OR lower(description) LIKE '%gastronomie%' THEN 'restaurants'
             WHEN lower(description) LIKE '%freizeit%' OR lower(description) LIKE '%kino%'
               OR lower(description) LIKE '%sport%' OR lower(description) LIKE '%fitness%'
               OR lower(description) LIKE '%gym%' OR lower(description) LIKE '%spielst%' THEN 'leisure'
             WHEN lower(description) LIKE '%sbb%' OR lower(description) LIKE '%tanken%' OR lower(description) LIKE '%mobilität%' OR lower(description) LIKE '%mobilitat%' THEN 'transport'
             WHEN lower(description) LIKE '%ferien%' OR lower(description) LIKE '%hotel%' OR lower(description) LIKE '%flug%' THEN 'travel'
             WHEN lower(description) LIKE '%steuer%' THEN 'taxes'
             WHEN lower(description) LIKE '%depot%' OR lower(description) LIKE '%vorsorge%' OR lower(description) LIKE '%säule%' OR lower(description) LIKE '%saule%' THEN 'saving'
             ELSE 'other' END
         ), category_source = 'description' WHERE category_id IS NULL;
         UPDATE transactions
           SET category_id=(SELECT id FROM categories WHERE category_key='restaurants'),
               category_source='description'
         WHERE amount_minor < 0 AND category_manual=0 AND category_source='description'
           AND category_id=(SELECT id FROM categories WHERE category_key='leisure')
           AND (lower(description) LIKE '%restaurant%' OR lower(description) LIKE '%restaur%'
             OR lower(description) LIKE '%fast-food%' OR lower(description) LIKE '%fast food%'
             OR lower(description) LIKE '%takeaway%' OR lower(description) LIKE '%take-away%'
             OR lower(description) LIKE '%pizzeria%' OR lower(description) LIKE '%bistro%'
             OR lower(description) LIKE '%cafÃ©%' OR lower(description) LIKE '%cafe%'
             OR lower(description) LIKE '%gastronomie%');
         UPDATE transactions SET category_id = (
           SELECT id FROM categories WHERE category_key = CASE
             WHEN lower(industry) LIKE '%lebensmittel%' OR lower(industry) LIKE '%supermarkt%' THEN 'groceries'
             WHEN lower(industry) LIKE '%restaurant%' OR lower(industry) LIKE '%restaur%'
               OR lower(industry) LIKE '%fast-food%' OR lower(industry) LIKE '%fast food%'
               OR lower(industry) LIKE '%gastronomie%' OR lower(industry) LIKE '%cafÃ©%'
               OR lower(industry) LIKE '%cafe%' THEN 'restaurants'
             WHEN lower(industry) LIKE '%spielst%' OR lower(industry) LIKE '%freizeit%'
               OR lower(industry) LIKE '%sport%' OR lower(industry) LIKE '%fitness%'
               OR lower(industry) LIKE '%gym%' THEN 'leisure'
             WHEN lower(industry) LIKE '%taxi%' OR lower(industry) LIKE '%transport%' OR lower(industry) LIKE '%tankstelle%' THEN 'transport'
             WHEN lower(industry) LIKE '%digitale güter%' OR lower(industry) LIKE '%digitale gueter%' THEN 'digital_subscriptions'
             WHEN lower(industry) LIKE '%hotel%' OR lower(industry) LIKE '%reise%' OR lower(industry) LIKE '%flug%' THEN 'travel'
             WHEN lower(industry) LIKE '%apotheke%' OR lower(industry) LIKE '%medizin%' OR lower(industry) LIKE '%gesundheit%' THEN 'health'
             WHEN lower(industry) LIKE '%elektronik%' THEN 'electronics'
             WHEN lower(industry) LIKE '%möbel%' OR lower(industry) LIKE '%moebel%' THEN 'furnishing'
             WHEN lower(industry) LIKE '%telekommunikation%' THEN 'telecom'
             ELSE 'other' END
         ), category_source = 'industry'
         WHERE amount_minor < 0 AND category_manual = 0 AND industry IS NOT NULL AND trim(industry) <> ''
           AND (lower(industry) LIKE '%lebensmittel%' OR lower(industry) LIKE '%supermarkt%'
             OR lower(industry) LIKE '%restaurant%' OR lower(industry) LIKE '%restaur%'
             OR lower(industry) LIKE '%fast-food%' OR lower(industry) LIKE '%fast food%'
             OR lower(industry) LIKE '%gastronomie%' OR lower(industry) LIKE '%cafÃ©%' OR lower(industry) LIKE '%cafe%'
             OR lower(industry) LIKE '%spielst%' OR lower(industry) LIKE '%freizeit%'
             OR lower(industry) LIKE '%sport%' OR lower(industry) LIKE '%fitness%' OR lower(industry) LIKE '%gym%'
             OR lower(industry) LIKE '%taxi%' OR lower(industry) LIKE '%transport%' OR lower(industry) LIKE '%tankstelle%'
             OR lower(industry) LIKE '%digitale güter%' OR lower(industry) LIKE '%digitale gueter%'
             OR lower(industry) LIKE '%hotel%' OR lower(industry) LIKE '%reise%' OR lower(industry) LIKE '%flug%'
             OR lower(industry) LIKE '%apotheke%' OR lower(industry) LIKE '%medizin%' OR lower(industry) LIKE '%gesundheit%'
             OR lower(industry) LIKE '%elektronik%' OR lower(industry) LIKE '%möbel%' OR lower(industry) LIKE '%moebel%'
             OR lower(industry) LIKE '%telekommunikation%');
         UPDATE transactions SET category_id = (
           SELECT category_id FROM industry_category_rules WHERE industry_key=lower(trim(transactions.industry))
         ), category_source = 'industry'
         WHERE amount_minor < 0 AND category_manual=0
           AND lower(trim(industry)) IN (SELECT industry_key FROM industry_category_rules);
         UPDATE transactions SET category_id = (SELECT id FROM categories WHERE category_key = 'digital_subscriptions')
           , category_source = 'description'
         WHERE amount_minor < 0 AND category_manual = 0
           AND category_id = (SELECT id FROM categories WHERE category_key = 'other')
           AND (lower(description) LIKE '%apple.com/bill%'
             OR lower(description) LIKE '%itunes.com%'
             OR lower(description) LIKE '%apple music%'
             OR lower(description) LIKE '%apple tv%'
             OR lower(description) LIKE '%icloud%'
             OR lower(description) LIKE '%google%youtube%'
             OR lower(description) LIKE '%google%one%'
             OR lower(description) LIKE '%google%storage%'
             OR lower(description) LIKE '%google%play%'
             OR lower(description) LIKE '%youtube premium%'
             OR lower(description) LIKE '%paramount+%'
             OR lower(description) LIKE '%paramountplus%'
             OR lower(description) LIKE '%netflix%'
             OR lower(description) LIKE '%disney+%'
             OR lower(description) LIKE '%disneyplus%'
             OR lower(description) LIKE '%disney plus%');
         UPDATE transactions SET category_id=(SELECT id FROM categories WHERE category_key='telecom')
           , category_source = 'description'
         WHERE amount_minor < 0 AND category_manual=0
           AND category_id=(SELECT id FROM categories WHERE category_key='other')
           AND (lower(description) LIKE '%sunrise%' OR lower(description) LIKE '%swisscom%');
         UPDATE transactions
           SET category_id=(SELECT id FROM categories WHERE category_key='credit_card'),
               category_source='description'
         WHERE amount_minor < 0 AND category_manual=0
           AND account_id IN (SELECT id FROM accounts WHERE account_type<>'credit_card')
           AND (
             (lower(description) LIKE '%ubs%vis1w%' AND
               (lower(description) LIKE '%widerspruch%' OR lower(description) LIKE '%lastschrift%'))
             OR lower(description) LIKE '%kreditkartenabrechnung%'
             OR lower(description) LIKE '%kreditkarten-abrechnung%'
             OR lower(description) LIKE '%credit card payment%'
           );",
    )?;
    categories::apply_redirects(connection)?;
    merchant_rules::apply(connection)
}

#[tauri::command]
pub fn save_import(
    storage: State<'_, Storage>,
    request: SaveImportRequest,
) -> Result<SaveImportResult, String> {
    if request.account_ids.is_empty() {
        return Err(
            "Bitte ein bestehendes Konto auswählen. Neue Konten unter Banken & Konten anlegen."
                .into(),
        );
    }
    save_import_to(&storage, request)
}

fn save_import_to(
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
        let mut statement = transaction
            .prepare("INSERT INTO transactions(account_id, import_id, booking_date, value_date, description, industry, amount_minor, balance_minor, currency, confidence, source_row) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)")
            .map_err(db_error)?;
        for index in &transaction_indices {
            let row = &request.statement.transactions[*index];
            statement
                .execute(params![
                    account_ids[row.currency.as_str()],
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
    transaction.commit().map_err(db_error)?;
    Ok(SaveImportResult {
        import_id,
        account_id,
        inserted_transactions,
        duplicate: inserted_transactions == 0,
    })
}

#[tauri::command]
pub fn database_status(storage: State<'_, Storage>) -> Result<DatabaseStatus, String> {
    let connection = storage.connect().map_err(db_error)?;
    Ok(DatabaseStatus {
        path: connection.path().unwrap_or_default().to_string(),
        accounts: count(&connection, "accounts")?,
        imports: count(&connection, "import_runs")?,
        transactions: count(&connection, "transactions")?,
    })
}

#[tauri::command]
pub fn dashboard_data(storage: State<'_, Storage>) -> Result<DashboardData, String> {
    dashboard_from(&storage)
}

#[tauri::command]
pub fn list_accounts(storage: State<'_, Storage>) -> Result<Vec<ManagedAccount>, String> {
    accounts_from(&storage)
}

#[tauri::command]
pub fn wealth_data(
    storage: State<'_, Storage>,
    account_ids: Option<Vec<i64>>,
) -> Result<WealthData, String> {
    wealth_from(&storage, account_ids.as_deref())
}

#[tauri::command]
pub fn transaction_analysis(
    storage: State<'_, Storage>,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
) -> Result<TransactionAnalysis, String> {
    let connection = storage.connect().map_err(db_error)?;
    analyze_transactions(&connection, from, to, provider_key, account_id)
}

fn analyze_transactions(
    connection: &Connection,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
) -> Result<TransactionAnalysis, String> {
    card_settlements::prepare(connection).map_err(db_error)?;
    let history = transaction_history(connection, &provider_key, account_id).map_err(db_error)?;
    // Category analysis describes what the money was spent on. Card statement
    // debits are only transfers to settle the card and would double-count the
    // individual card purchases, so exclude settlements and include card rows.
    let category_filter = "t.amount_minor < 0 AND a.is_active = 1 AND t.currency = 'CHF'
                  AND (?1 IS NULL OR t.booking_date >= ?1)
                  AND (?2 IS NULL OR t.booking_date <= ?2)
                  AND (?3 IS NULL OR i.provider_key = ?3)
                  AND (?4 IS NULL OR a.id = ?4)
                  AND t.id NOT IN (SELECT id FROM card_settlement_transactions)";
    let mut category_query = connection.prepare(&format!(
        "SELECT c.category_key, c.label, c.color, SUM(-t.amount_minor), COUNT(*)
         FROM transactions t JOIN accounts a ON a.id=t.account_id JOIN institutions i ON i.id=a.institution_id
         JOIN categories c ON c.id=t.category_id WHERE {category_filter}
         GROUP BY c.id ORDER BY SUM(-t.amount_minor) DESC"
    )).map_err(db_error)?;
    let categories = category_query
        .query_map(params![from, to, provider_key, account_id], |row| {
            Ok(CategorySpend {
                key: row.get(0)?,
                label: row.get(1)?,
                color: row.get(2)?,
                amount_minor: row.get(3)?,
                transaction_count: row.get(4)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut month_query = connection
        .prepare(&format!(
            "SELECT substr(t.booking_date,1,7), SUM(-t.amount_minor) FROM transactions t
         JOIN accounts a ON a.id=t.account_id JOIN institutions i ON i.id=a.institution_id
         WHERE {category_filter} GROUP BY substr(t.booking_date,1,7) ORDER BY 1"
        ))
        .map_err(db_error)?;
    let months = month_query
        .query_map(params![from, to, provider_key, account_id], |row| {
            Ok(MonthlySpend {
                month: row.get(0)?,
                amount_minor: row.get(1)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let detail_filter = "t.amount_minor <> 0 AND a.is_active = 1 AND t.currency = 'CHF'
                  AND (?1 IS NULL OR t.booking_date >= ?1)
                  AND (?2 IS NULL OR t.booking_date <= ?2)
                  AND (?3 IS NULL OR i.provider_key = ?3)
                  AND (?4 IS NULL OR a.id = ?4)";
    let mut transaction_query = connection.prepare(&format!(
        "SELECT t.id,t.booking_date,t.description,t.industry,t.amount_minor,t.currency,c.category_key,c.label,c.color,
                CASE WHEN t.category_manual=1 THEN 'manual' ELSE t.category_source END,i.name,i.provider_key,a.name,
                (?4 IS NULL AND a.account_type='credit_card'),
                t.id IN (SELECT id FROM card_settlement_transactions)
         FROM transactions t JOIN accounts a ON a.id=t.account_id JOIN institutions i ON i.id=a.institution_id
         JOIN categories c ON c.id=t.category_id WHERE {detail_filter} ORDER BY t.booking_date DESC,t.id DESC"
    )).map_err(db_error)?;
    let transactions = transaction_query
        .query_map(params![from, to, provider_key, account_id], |row| {
            Ok(AnalyzedTransaction {
                id: row.get(0)?,
                booking_date: row.get(1)?,
                description: row.get(2)?,
                industry: row.get(3)?,
                amount_minor: row.get(4)?,
                currency: row.get(5)?,
                category_key: row.get(6)?,
                category_label: row.get(7)?,
                category_color: row.get(8)?,
                category_source: row.get(9)?,
                provider: row.get(10)?,
                provider_key: row.get(11)?,
                account_name: row.get(12)?,
                excluded_from_totals: row.get(13)?,
                is_card_settlement: row.get(14)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let (income_transactions, transactions): (Vec<_>, Vec<_>) = transactions
        .into_iter()
        .partition(|item| item.amount_minor > 0);
    let providers = {
        let mut query = connection.prepare("SELECT i.name,i.provider_key,COUNT(a.id) FROM institutions i JOIN accounts a ON a.institution_id=i.id WHERE a.is_active=1 GROUP BY i.id ORDER BY i.name").map_err(db_error)?;
        let providers = query
            .query_map([], |row| {
                Ok(DashboardProvider {
                    provider: row.get(0)?,
                    provider_key: row.get(1)?,
                    account_count: row.get(2)?,
                    balance_minor: 0,
                    logo_data_url: None,
                })
            })
            .map_err(db_error)?;
        let result = providers.collect::<Result<Vec<_>, _>>().map_err(db_error)?;
        result
    };
    // Cash-flow totals deliberately keep the opposite perspective: the bank
    // account settlement is real cash movement, while card details are not
    // added a second time unless the card account itself is selected.
    let (total_spend_minor, transaction_count) = connection
        .query_row(
            "SELECT COALESCE(SUM(-t.amount_minor),0), COUNT(*)
             FROM transactions t
             JOIN accounts a ON a.id=t.account_id
             JOIN institutions i ON i.id=a.institution_id
             WHERE t.amount_minor<0 AND a.is_active=1 AND t.currency='CHF'
               AND (?1 IS NULL OR t.booking_date>=?1)
               AND (?2 IS NULL OR t.booking_date<=?2)
               AND (?3 IS NULL OR i.provider_key=?3)
               AND (?4 IS NULL OR a.id=?4)
               AND (?4 IS NOT NULL OR a.account_type<>'credit_card')",
            params![from, to, provider_key, account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(db_error)?;
    let (total_income_minor, income_count, first_date, last_date) =
        income_summary(&connection, &from, &to, &provider_key, account_id).map_err(db_error)?;
    Ok(TransactionAnalysis {
        total_income_minor,
        income_count,
        income_transactions,
        total_spend_minor,
        transaction_count,
        first_date,
        last_date,
        categories,
        months,
        history,
        transactions,
        providers,
    })
}

fn transaction_history(
    db: &Connection,
    provider: &Option<String>,
    account: Option<i64>,
) -> rusqlite::Result<Vec<TransactionHistoryPoint>> {
    let mut query = db.prepare(
        "SELECT 'account:' || bs.account_id,bs.balance_date,bs.amount_minor
         FROM balance_snapshots bs
         JOIN accounts a ON a.id=bs.account_id
         JOIN institutions i ON i.id=a.institution_id
         WHERE bs.currency='CHF' AND a.is_active=1
           AND date(bs.balance_date)<=date('now','localtime')
           AND (?1 IS NULL OR i.provider_key=?1)
           AND (?2 IS NULL OR a.id=?2)
           AND (?2 IS NOT NULL OR a.account_type<>'credit_card')
           AND bs.id=(SELECT latest.id FROM balance_snapshots latest
             WHERE latest.account_id=bs.account_id AND latest.balance_date=bs.balance_date
             ORDER BY latest.id DESC LIMIT 1)
         ORDER BY bs.balance_date,bs.account_id",
    )?;
    let snapshots = query
        .query_map(params![provider, account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut dates = BTreeMap::<String, Vec<(String, i64)>>::new();
    for (source, date, amount) in snapshots {
        dates.entry(date).or_default().push((source, amount));
    }
    let (Some(first), Some(last)) = (dates.keys().next(), dates.keys().next_back()) else {
        return Ok(Vec::new());
    };
    let mut day = NaiveDate::parse_from_str(first, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let last = NaiveDate::parse_from_str(last, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let mut latest = HashMap::<String, i64>::new();
    let mut history = Vec::new();
    while day <= last {
        let date = day.format("%Y-%m-%d").to_string();
        if let Some(values) = dates.get(&date) {
            for (source, amount) in values {
                latest.insert(source.clone(), *amount);
            }
        }
        history.push(TransactionHistoryPoint {
            date,
            total_minor: latest.values().sum(),
        });
        day += Duration::days(1);
    }
    Ok(history)
}

fn income_summary(
    db: &Connection,
    from: &Option<String>,
    to: &Option<String>,
    provider: &Option<String>,
    account: Option<i64>,
) -> rusqlite::Result<(i64, i64, Option<String>, Option<String>)> {
    db.query_row("SELECT COALESCE(SUM(CASE WHEN t.amount_minor>0 THEN t.amount_minor ELSE 0 END),0),
        COUNT(CASE WHEN t.amount_minor>0 THEN 1 END), MIN(t.booking_date),MAX(t.booking_date)
        FROM transactions t JOIN accounts a ON a.id=t.account_id JOIN institutions i ON i.id=a.institution_id
        WHERE a.is_active=1 AND t.currency='CHF' AND t.amount_minor<>0
          AND (?1 IS NULL OR t.booking_date>=?1) AND (?2 IS NULL OR t.booking_date<=?2)
          AND (?3 IS NULL OR i.provider_key=?3) AND (?4 IS NULL OR a.id=?4)
          AND (?4 IS NOT NULL OR a.account_type <> 'credit_card')",
        params![from,to,provider,account],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
}

#[cfg(test)]
mod income_tests {
    use super::*;
    #[test]
    fn statement_payment_without_counterpart_is_not_counted_twice() {
        let make_db = || {
            let db = Connection::open_in_memory().unwrap();
            db.execute_batch("CREATE TABLE institutions(id INTEGER,name TEXT,provider_key TEXT);
                INSERT INTO institutions VALUES(1,'UBS','ubs');
                CREATE TABLE accounts(id INTEGER,institution_id INTEGER,name TEXT,account_type TEXT,is_active INTEGER);
                INSERT INTO accounts VALUES(1,1,'Privatkonto','checking',1),(2,1,'Kreditkarte','credit_card',1);
                CREATE TABLE categories(id INTEGER,category_key TEXT,label TEXT,color TEXT);
                INSERT INTO categories VALUES(1,'other','Sonstiges','#000');
                CREATE TABLE transactions(id INTEGER,account_id INTEGER,category_id INTEGER,amount_minor INTEGER,currency TEXT,booking_date TEXT,description TEXT,industry TEXT,category_manual INTEGER DEFAULT 0,category_source TEXT NOT NULL DEFAULT 'description');
                INSERT INTO transactions(id,account_id,category_id,amount_minor,currency,booking_date,description) VALUES
                  (1,1,1,-414680,'CHF','2026-08-27','UBS Switzerland AG VIS1W WIDERSPRUCH AN UBS INNERT 30 TAGEN'),
                  (2,2,1,-4090,'CHF','2026-08-05','APPLE.COM Einkauf'),
                  (3,2,1,-1265,'CHF','2026-08-05','SBB Restaurant Einkauf'),
                  (4,1,1,-1600,'CHF','2026-08-31','Saldo Dienstleistungspreisabschluss'),
                  (5,1,1,-3000,'CHF','2026-08-15','Versicherung LSV'),
                  (6,2,1,500,'CHF','2026-08-06','Erstattung Einkauf'),
                  (7,2,1,9900,'CHF','2026-08-10','LSV-ZAHLUNG');
                CREATE TABLE balance_snapshots(id INTEGER PRIMARY KEY,account_id INTEGER,balance_date TEXT,amount_minor INTEGER,currency TEXT);
                INSERT INTO balance_snapshots(account_id,balance_date,amount_minor,currency) VALUES
                  (1,'2026-08-14',1000000,'CHF'),
                  (1,'2026-08-15',997000,'CHF'),
                  (1,'2026-08-27',582320,'CHF'),
                  (1,'2026-08-31',580720,'CHF'),
                  (2,'2026-08-04',0,'CHF'),
                  (2,'2026-08-05',-5355,'CHF'),
                  (2,'2026-08-06',-4855,'CHF'),
                  (2,'2026-08-10',5045,'CHF');").unwrap();
            db
        };
        let db = make_db();
        let result = analyze_transactions(
            &db,
            Some("2026-08-01".into()),
            Some("2026-08-31".into()),
            Some("ubs".into()),
            None,
        )
        .unwrap();
        assert_eq!(result.total_spend_minor, 419280);
        assert_eq!(result.transaction_count, 3);
        assert_eq!(result.transactions.len(), 5);
        assert_eq!(
            result
                .transactions
                .iter()
                .filter(|transaction| !transaction.excluded_from_totals)
                .map(|transaction| -transaction.amount_minor)
                .sum::<i64>(),
            419280
        );
        assert!(result
            .transactions
            .iter()
            .any(|transaction| transaction.id == 1
                && !transaction.excluded_from_totals
                && transaction.is_card_settlement));
        // Spending analysis replaces the card settlement with its purchases.
        assert_eq!(
            result
                .categories
                .iter()
                .map(|c| c.amount_minor)
                .sum::<i64>(),
            9955
        );
        assert_eq!(
            result
                .categories
                .iter()
                .map(|c| c.transaction_count)
                .sum::<i64>(),
            4
        );
        assert_eq!(result.months[0].amount_minor, 9955);
        assert_eq!(
            result
                .transactions
                .iter()
                .filter(|transaction| !transaction.is_card_settlement)
                .map(|transaction| -transaction.amount_minor)
                .sum::<i64>(),
            9955
        );
        assert_eq!(result.history.len(), 18);
        assert_eq!(result.history.first().unwrap().date, "2026-08-14");
        assert_eq!(result.history.last().unwrap().total_minor, 580720);
        assert_eq!(result.total_income_minor, 0);
        assert_eq!(result.income_transactions.len(), 2);
        assert!(result
            .income_transactions
            .iter()
            .all(|transaction| transaction.excluded_from_totals));
        // An individual account still shows its actual debits and credits.
        let db = make_db();
        let account = analyze_transactions(&db, None, None, None, Some(1)).unwrap();
        assert_eq!(account.total_spend_minor, 419280);
        assert_eq!(account.transaction_count, 3);
        assert_eq!(account.categories[0].amount_minor, 4600);
        assert_eq!(account.history.last().unwrap().total_minor, 580720);

        let db = make_db();
        let card = analyze_transactions(&db, None, None, None, Some(2)).unwrap();
        assert_eq!(card.total_spend_minor, 5355);
        assert_eq!(card.categories[0].amount_minor, 5355);
        assert_eq!(card.history.len(), 7);
        assert_eq!(card.history.last().unwrap().total_minor, 5045);
    }

    #[test]
    fn income_respects_filters_and_excludes_credit_card_details() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE institutions(id INTEGER,provider_key TEXT); INSERT INTO institutions VALUES(1,'ubs'),(2,'other');
    CREATE TABLE accounts(id INTEGER,institution_id INTEGER,is_active INTEGER); INSERT INTO accounts VALUES(1,1,1),(2,1,1),(3,2,1),(4,1,0);
    CREATE TABLE transactions(id INTEGER,account_id INTEGER,amount_minor INTEGER,currency TEXT,booking_date TEXT);
    INSERT INTO transactions VALUES(1,1,800000,'CHF','2026-01-01'),(2,1,-100000,'CHF','2026-01-02'),(3,2,100000,'CHF','2026-01-03'),(4,2,-20000,'CHF','2026-01-04'),(5,3,50000,'CHF','2026-01-05'),(6,1,30000,'EUR','2026-01-06'),(7,4,40000,'CHF','2026-01-07'),(8,1,60000,'CHF','2025-01-01');
    ALTER TABLE accounts ADD COLUMN account_type TEXT NOT NULL DEFAULT 'checking';
    UPDATE accounts SET account_type='credit_card' WHERE id=2;").unwrap();
        let from = Some("2026-01-01".into());
        let result = income_summary(&db, &from, &None, &Some("ubs".into()), None).unwrap();
        assert_eq!(
            result,
            (
                800000,
                1,
                Some("2026-01-01".into()),
                Some("2026-01-02".into())
            )
        );
        assert_eq!(
            income_summary(&db, &from, &None, &None, None).unwrap().0,
            850000
        );
        assert_eq!(
            income_summary(&db, &from, &None, &None, Some(2)).unwrap().0,
            100000
        );
        assert_eq!(
            income_summary(&db, &Some("2027-01-01".into()), &None, &None, None).unwrap(),
            (0, 0, None, None)
        );
    }
}

#[tauri::command]
pub fn set_transaction_category(
    storage: State<'_, Storage>,
    transaction_id: i64,
    category_key: String,
) -> Result<usize, String> {
    let mut connection = storage.connect().map_err(db_error)?;
    merchant_rules::learn(&mut connection, transaction_id, &category_key)
}

#[tauri::command]
pub fn create_account(
    storage: State<'_, Storage>,
    request: CreateAccountRequest,
) -> Result<i64, String> {
    validate_account(
        &request.account_name,
        &request.currency,
        &request.account_type,
    )?;
    if request.institution_name.trim().is_empty() {
        return Err("Bitte einen Namen für die Bank oder den Anbieter eingeben.".to_string());
    }
    let connection = storage.connect().map_err(db_error)?;
    let provider_key = unique_provider_key(&connection, &request.institution_name)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT OR IGNORE INTO institutions(provider_key, name, institution_type, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![provider_key, request.institution_name.trim(), request.institution_type, now],
    ).map_err(db_error)?;
    let institution_id: i64 = connection
        .query_row(
            "SELECT id FROM institutions WHERE provider_key = ?1",
            [&provider_key],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    connection.execute(
        "INSERT INTO accounts(institution_id, name, account_type, currency, created_at, external_reference) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![institution_id, request.account_name.trim(), request.account_type, request.currency.trim().to_uppercase(), now, clean_optional(request.external_reference)],
    ).map_err(|error| match error {
        rusqlite::Error::SqliteFailure(_, _) => "Dieses Konto ist bei diesem Anbieter bereits vorhanden.".to_string(),
        other => db_error(other),
    })?;
    Ok(connection.last_insert_rowid())
}

#[tauri::command]
pub fn update_account(
    storage: State<'_, Storage>,
    request: UpdateAccountRequest,
) -> Result<(), String> {
    validate_account(&request.name, &request.currency, &request.account_type)?;
    let connection = storage.connect().map_err(db_error)?;
    let changed = connection.execute(
        "UPDATE accounts SET name = ?1, account_type = ?2, currency = ?3, external_reference = ?4, is_active = ?5, include_in_net_worth = ?6 WHERE id = ?7",
        params![request.name.trim(), request.account_type, request.currency.trim().to_uppercase(), clean_optional(request.external_reference), request.is_active, request.include_in_net_worth, request.id],
    ).map_err(db_error)?;
    if changed == 0 {
        Err("Das Konto wurde nicht gefunden.".to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn delete_account(storage: State<'_, Storage>, account_id: i64) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    let institution_id: i64 = transaction
        .query_row(
            "SELECT institution_id FROM accounts WHERE id = ?1",
            [account_id],
            |row| row.get(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "Das Konto wurde nicht gefunden.".to_string(),
            other => db_error(other),
        })?;
    let related_data: i64 = transaction
        .query_row(
            "SELECT (SELECT COUNT(*) FROM import_runs WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM transactions WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM balance_snapshots WHERE account_id = ?1)
                  + (SELECT COUNT(*) FROM portfolio_positions WHERE account_id = ?1)",
            [account_id],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if related_data > 0 {
        return Err("Dieses Konto enthält Importdaten und kann deshalb nicht gelöscht werden. Du kannst es stattdessen archivieren.".to_string());
    }
    transaction
        .execute("DELETE FROM accounts WHERE id = ?1", [account_id])
        .map_err(db_error)?;
    transaction
        .execute(
            "DELETE FROM institutions WHERE id = ?1 AND NOT EXISTS (SELECT 1 FROM accounts WHERE institution_id = ?1)",
            [institution_id],
        )
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)
}

#[tauri::command]
pub fn save_manual_valuation(
    storage: State<'_, Storage>,
    request: ManualValuationRequest,
) -> Result<(), String> {
    let start = request
        .holding_start_date
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(request.valuation_date.trim());
    NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|_| "Das Einstandsdatum ist ungültig.".to_string())?;
    if request.label.trim().is_empty() {
        return Err("Bitte eine Bezeichnung für die Position eingeben.".into());
    }
    if let Some(end) = request
        .holding_end_date
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "Das Verkaufsdatum ist ungültig.".to_string())?;
        if end < start {
            return Err("Das Verkaufsdatum darf nicht vor dem Einstandsdatum liegen.".into());
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
    if account_type != "manual_asset" {
        return Err(
            "Manuelle Bewertungen sind nur für manuell verwaltete Positionen möglich.".into(),
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

    transaction
        .execute(
            "DELETE FROM position_quantities WHERE position_id=?1",
            [position_id],
        )
        .map_err(db_error)?;
    if let Some(quantity) = request.quantity {
        transaction.execute(
            "INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(?1,?2,?3,6,'manual',?4)",
            params![position_id, start, (quantity * 1_000_000.0).round() as i64, now],
        ).map_err(db_error)?;
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
            params![position_id, request.valuation_date.trim(), request.amount_minor, account_currency, now],
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
            params![listing_id, request.valuation_date.trim(), price, currency, now],
        ).map_err(db_error)?;
        if currency != account_currency {
            if let Some(rate) = request.exchange_rate {
                transaction.execute(
                    "INSERT INTO fx_rates(base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at)
                     VALUES(?1,?2,?3,?4,9,'manual',?5)
                     ON CONFLICT(base_currency,quote_currency,rate_date,source) DO UPDATE SET
                       rate_amount=excluded.rate_amount,rate_scale=excluded.rate_scale,fetched_at=excluded.fetched_at",
                    params![currency, account_currency, request.valuation_date.trim(), (rate * 1_000_000_000.0).round() as i64, now],
                ).map_err(db_error)?;
            }
        }
    }
    let rebuild_now = listing_id.is_none() || request.unit_price_minor.is_some();
    transaction.commit().map_err(db_error)?;
    if rebuild_now {
        rebuild_daily_valuations(&storage)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_manual_positions(
    storage: State<'_, Storage>,
    account_id: i64,
) -> Result<Vec<ManualPosition>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection.prepare(
        "SELECT p.id,p.account_id,p.label,
                COALESCE((SELECT valuation_date FROM daily_valuations d WHERE d.position_id=p.id ORDER BY valuation_date DESC LIMIT 1),p.holding_start_date),
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
                p.holding_start_date,p.holding_end_date
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
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(positions)
}

#[tauri::command]
pub fn delete_manual_position(storage: State<'_, Storage>, position_id: i64) -> Result<(), String> {
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

fn listing_parts(identifier: &str) -> (Option<String>, Option<String>) {
    identifier
        .split_once(':')
        .map_or((None, Some(identifier.to_string())), |(mic, symbol)| {
            (Some(mic.to_string()), Some(symbol.to_string()))
        })
}

fn exchange_currency(mic: Option<&str>) -> Option<&'static str> {
    match mic {
        Some("XASX") => Some("AUD"),
        Some("XSWX") => Some("CHF"),
        Some("XLON") => Some("GBP"),
        Some("XETR") => Some("EUR"),
        Some("XNAS" | "XNYS" | "XASE") => Some("USD"),
        _ => None,
    }
}

fn decimal_value(amount: i64, scale: i64) -> f64 {
    amount as f64 / 10_f64.powi(scale as i32)
}

pub(super) fn rebuild_daily_valuations(storage: &Storage) -> Result<(), String> {
    rebuild_daily_valuations_with_mode(storage, false)
}

pub(super) fn rebuild_daily_valuations_incremental(storage: &Storage) -> Result<(), String> {
    rebuild_daily_valuations_with_mode(storage, true)
}

fn rebuild_daily_valuations_with_mode(storage: &Storage, incremental: bool) -> Result<(), String> {
    let mut connection = storage.connect().map_err(db_error)?;
    let positions = {
        let mut statement = connection
            .prepare(
                "SELECT p.id,p.listing_id,p.holding_start_date,p.holding_end_date,a.currency
             FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id",
            )
            .map_err(db_error)?;
        let values = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        values
    };
    let transaction = connection.transaction().map_err(db_error)?;
    if !incremental {
        transaction
            .execute("DELETE FROM daily_valuations", [])
            .map_err(db_error)?;
    }
    let today = Local::now().date_naive();
    for (position_id, listing_id, start, end, _account_currency) in positions {
        let mut day = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
            .map_err(|_| "Ungültiges Einstandsdatum in der Datenbank.".to_string())?;
        if incremental {
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
#[tauri::command]
pub fn set_institution_logo(
    storage: State<'_, Storage>,
    institution_id: i64,
    data_url: Option<String>,
) -> Result<(), String> {
    let value = data_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = &value {
        let allowed = [
            "data:image/png;base64,",
            "data:image/jpeg;base64,",
            "data:image/webp;base64,",
            "data:image/svg+xml;base64,",
        ];
        if !allowed.iter().any(|prefix| value.starts_with(prefix)) {
            return Err("Bitte ein Logo im Format PNG, JPEG, WebP oder SVG auswählen.".to_string());
        }
        if value.len() > 2_800_000 {
            return Err("Das Logo darf maximal 2 MB gross sein.".to_string());
        }
    }
    let connection = storage.connect().map_err(db_error)?;
    let changed = connection
        .execute(
            "UPDATE institutions SET logo_data_url = ?1 WHERE id = ?2",
            params![value, institution_id],
        )
        .map_err(db_error)?;
    if changed == 0 {
        Err("Der Anbieter wurde nicht gefunden.".to_string())
    } else {
        Ok(())
    }
}

fn accounts_from(storage: &Storage) -> Result<Vec<ManagedAccount>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut statement = connection.prepare(
        "SELECT a.id, i.id, i.name, i.provider_key, i.institution_type, a.name, a.account_type,
                a.currency, a.external_reference, a.is_active, a.include_in_net_worth,
                CASE WHEN a.account_type = 'manual_asset' THEN (SELECT SUM(d.value_minor) FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id AND date(p.holding_start_date)<=date('now','localtime') AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))) WHEN bs.id IS NULL THEN NULL ELSE bs.amount_minor + COALESCE((
                  SELECT SUM(t.amount_minor) FROM transactions t
                  WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                    AND date(t.booking_date) <= date('now', 'localtime')
                ), 0) END,
                CASE WHEN a.account_type = 'manual_asset' THEN (SELECT MAX(d.valuation_date) FROM portfolio_positions p JOIN daily_valuations d ON d.position_id=p.id WHERE p.account_id=a.id AND date(d.valuation_date)<=date('now','localtime')) WHEN bs.id IS NULL THEN NULL ELSE COALESCE((
                  SELECT MAX(t.booking_date) FROM transactions t
                  WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                    AND date(t.booking_date) <= date('now', 'localtime')
                ), bs.balance_date) END,
                CASE WHEN a.account_type = 'manual_asset' THEN COALESCE((SELECT d.currency FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id LIMIT 1),a.currency) ELSE a.currency END,
                (SELECT COUNT(*) FROM import_runs ir WHERE ir.account_id = a.id OR EXISTS(SELECT 1 FROM balance_snapshots s WHERE s.import_id = ir.id AND s.account_id = a.id)),
                (SELECT COUNT(*) FROM portfolio_positions p WHERE p.account_id = a.id),
                NULL, NULL, NULL, NULL, i.logo_data_url
         FROM accounts a JOIN institutions i ON i.id = a.institution_id
         LEFT JOIN balance_snapshots bs ON bs.id = (SELECT latest.id FROM balance_snapshots latest WHERE latest.account_id = a.id AND date(latest.balance_date) <= date('now', 'localtime') ORDER BY latest.balance_date DESC, latest.id DESC LIMIT 1)
         ORDER BY i.name, a.is_active DESC, a.name"
    ).map_err(db_error)?;
    let accounts = statement
        .query_map([], |row| {
            Ok(ManagedAccount {
                id: row.get(0)?,
                institution_id: row.get(1)?,
                provider: row.get(2)?,
                provider_key: row.get(3)?,
                institution_type: row.get(4)?,
                name: row.get(5)?,
                account_type: row.get(6)?,
                currency: row.get(7)?,
                external_reference: row.get(8)?,
                is_active: row.get(9)?,
                include_in_net_worth: row.get(10)?,
                balance_minor: row.get(11)?,
                balance_date: row.get(12)?,
                balance_currency: row.get(13)?,
                import_count: row.get(14)?,
                manual_valuation_count: row.get(15)?,
                manual_quantity: row.get(16)?,
                manual_unit_price_minor: row.get(17)?,
                manual_quote_currency: row.get(18)?,
                manual_exchange_rate: row.get(19)?,
                logo_data_url: row.get(20)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(accounts)
}

fn wealth_from(storage: &Storage, account_ids: Option<&[i64]>) -> Result<WealthData, String> {
    let all_accounts = accounts_from(storage)?;
    let excluded_account_count = all_accounts
        .iter()
        .filter(|account| !account.is_active || !account.include_in_net_worth)
        .count();
    let available_accounts: Vec<ManagedAccount> = all_accounts
        .into_iter()
        .filter(|account| account.is_active && account.include_in_net_worth)
        .collect();
    let accounts: Vec<ManagedAccount> = available_accounts
        .iter()
        .filter(|account| account_ids.is_none_or(|ids| ids.contains(&account.id)))
        .cloned()
        .collect();
    let current_total_minor = accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
        .filter_map(|account| account.balance_minor)
        .sum();
    let connection = storage.connect().map_err(db_error)?;
    let regular_accounts = accounts
        .iter()
        .filter(|account| account.account_type != "manual_asset")
        .cloned()
        .collect::<Vec<_>>();
    let mut by_type = breakdown(&regular_accounts, |account| {
        (
            account.account_type.clone(),
            account_type_label(&account.account_type).to_string(),
        )
    });
    let selected_account_ids = accounts
        .iter()
        .map(|account| account.id)
        .collect::<HashSet<_>>();
    let mut manual_type_accounts = HashMap::<String, HashSet<i64>>::new();
    let mut manual_values = HashMap::<String, i64>::new();
    {
        let mut statement = connection.prepare(
            "SELECT p.account_id,p.asset_type,d.value_minor
             FROM portfolio_positions p
             JOIN accounts a ON a.id=p.account_id
             JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest
               WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime')
               ORDER BY latest.valuation_date DESC LIMIT 1)
             WHERE d.currency='CHF' AND date(p.holding_start_date)<=date('now','localtime')
               AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))"
        ).map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(db_error)?;
        for row in rows {
            let (account_id, asset_type, value_minor) = row.map_err(db_error)?;
            if !selected_account_ids.contains(&account_id) {
                continue;
            }
            *manual_values.entry(asset_type.clone()).or_default() += value_minor;
            manual_type_accounts
                .entry(asset_type)
                .or_default()
                .insert(account_id);
        }
    }
    for (asset_type, amount_minor) in manual_values {
        by_type.push(WealthBreakdown {
            key: format!("manual_asset_{asset_type}"),
            label: asset_type_label(&asset_type).to_string(),
            amount_minor,
            account_count: manual_type_accounts
                .get(&asset_type)
                .map_or(0, |ids| ids.len() as i64),
        });
    }
    by_type.sort_by_key(|item| std::cmp::Reverse(item.amount_minor));
    let by_provider = breakdown(&accounts, |account| {
        (account.provider_key.clone(), account.provider.clone())
    });

    let account_filter = account_ids.map_or(String::new(), |ids| {
        if ids.is_empty() {
            " AND 0".to_string()
        } else {
            format!(
                " AND a.id IN ({})",
                ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
            )
        }
    });
    let history_query = format!(
        "SELECT 'account:' || bs.account_id AS source_key,bs.balance_date,bs.amount_minor
         FROM balance_snapshots bs JOIN accounts a ON a.id=bs.account_id
         WHERE a.account_type<>'manual_asset' AND a.is_active=1
           AND a.include_in_net_worth=1 AND a.currency='CHF'
           AND date(bs.balance_date)<=date('now','localtime'){account_filter}
         UNION ALL
         SELECT 'position:' || d.position_id,d.valuation_date,d.value_minor
         FROM daily_valuations d JOIN portfolio_positions p ON p.id=d.position_id
         JOIN accounts a ON a.id=p.account_id
         WHERE a.is_active=1 AND a.include_in_net_worth=1
           AND d.currency='CHF' AND date(d.valuation_date)<=date('now','localtime'){account_filter}
         UNION ALL
         SELECT 'position:' || p.id,date(p.holding_end_date,'+1 day'),0
         FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id
         WHERE p.holding_end_date IS NOT NULL AND a.is_active=1
           AND a.include_in_net_worth=1
           AND EXISTS(SELECT 1 FROM daily_valuations ended
             WHERE ended.position_id=p.id AND ended.currency='CHF')
           AND date(p.holding_end_date,'+1 day')<=date('now','localtime'){account_filter}
         ORDER BY 2,1"
    );
    let mut statement = connection.prepare(&history_query).map_err(db_error)?;
    let snapshots = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut dates: BTreeMap<String, Vec<(String, i64)>> = BTreeMap::new();
    for (source_key, date, amount) in snapshots {
        let day = date.get(..10).unwrap_or(&date).to_string();
        dates.entry(day).or_default().push((source_key, amount));
    }
    let mut latest = HashMap::<String, i64>::new();
    let mut history = Vec::new();
    if let (Some(first), Some(last)) = (dates.keys().next(), dates.keys().next_back()) {
        let mut day = NaiveDate::parse_from_str(first, "%Y-%m-%d")
            .map_err(|error| format!("Ungültiges Bewertungsdatum: {error}"))?;
        let last = NaiveDate::parse_from_str(last, "%Y-%m-%d")
            .map_err(|error| format!("Ungültiges Bewertungsdatum: {error}"))?;
        while day <= last {
            let date = day.format("%Y-%m-%d").to_string();
            if let Some(values) = dates.get(&date) {
                for (source_key, amount) in values {
                    latest.insert(source_key.clone(), *amount);
                }
            }
            history.push(WealthHistoryPoint {
                date,
                total_minor: latest.values().sum(),
            });
            day += Duration::days(1);
        }
    }
    let first_total_minor = history.first().map(|point| point.total_minor);
    let change_minor = first_total_minor.map(|first| current_total_minor - first);
    Ok(WealthData {
        currency: "CHF".to_string(),
        current_total_minor,
        first_total_minor,
        change_minor,
        history,
        by_type,
        by_provider,
        accounts: available_accounts,
        excluded_account_count,
    })
}

fn breakdown<F>(accounts: &[ManagedAccount], key_and_label: F) -> Vec<WealthBreakdown>
where
    F: Fn(&ManagedAccount) -> (String, String),
{
    let mut values = HashMap::<String, WealthBreakdown>::new();
    for account in accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
    {
        let (key, label) = key_and_label(account);
        let entry = values.entry(key.clone()).or_insert(WealthBreakdown {
            key,
            label,
            amount_minor: 0,
            account_count: 0,
        });
        entry.amount_minor += account.balance_minor.unwrap_or(0);
        entry.account_count += 1;
    }
    let mut result: Vec<_> = values.into_values().collect();
    result.sort_by_key(|item| std::cmp::Reverse(item.amount_minor));
    result
}

fn account_type_label(value: &str) -> &'static str {
    match value {
        "cash" => "Konten",
        "savings" => "Sparkonten",
        "portfolio" => "Depots",
        "pillar3a" => "Säule 3a",
        "mortgage" => "Hypotheken",
        "credit_card" => "Kreditkarten",
        "manual_asset" => "Manuelle Positionen",
        _ => "Sonstiges",
    }
}

fn asset_type_label(value: &str) -> &'static str {
    match value {
        "stock" => "Aktien",
        "option" => "Optionen",
        "crypto" => "Kryptowährungen",
        "cash" => "Cash & Geldbeträge",
        "fund" => "Fonds & ETF",
        "bond" => "Obligationen",
        _ => "Sonstige Anlagen",
    }
}

fn validate_account(name: &str, currency: &str, account_type: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Bitte einen Kontonamen eingeben.".to_string());
    }
    if currency.trim().len() != 3 {
        return Err("Die Währung muss aus drei Buchstaben bestehen.".to_string());
    }
    if ![
        "cash",
        "savings",
        "portfolio",
        "pillar3a",
        "mortgage",
        "credit_card",
        "manual_asset",
    ]
    .contains(&account_type)
    {
        return Err("Der Kontotyp ist ungültig.".to_string());
    }
    Ok(())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn unique_provider_key(connection: &Connection, name: &str) -> Result<String, String> {
    if let Some(key) = connection
        .query_row(
            "SELECT provider_key FROM institutions WHERE lower(name) = lower(?1)",
            [name.trim()],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?
    {
        return Ok(key);
    }
    let generated = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let normalized = generated
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if normalized.is_empty() {
        "anbieter"
    } else {
        &normalized
    };
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while connection
        .query_row(
            "SELECT 1 FROM institutions WHERE provider_key = ?1",
            [&candidate],
            |_| Ok(()),
        )
        .optional()
        .map_err(db_error)?
        .is_some()
    {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

fn dashboard_from(storage: &Storage) -> Result<DashboardData, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection
        .prepare(
            "SELECT a.id, i.name, i.provider_key, a.name, a.account_type, a.currency,
                    CASE WHEN a.account_type = 'manual_asset' THEN (SELECT SUM(d.value_minor) FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id AND date(p.holding_start_date)<=date('now','localtime') AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))) WHEN bs.id IS NULL THEN NULL ELSE bs.amount_minor + COALESCE((
                      SELECT SUM(t.amount_minor) FROM transactions t
                      WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                        AND date(t.booking_date) <= date('now', 'localtime')
                    ), 0) END,
                    CASE WHEN a.account_type = 'manual_asset' THEN (SELECT MAX(d.valuation_date) FROM portfolio_positions p JOIN daily_valuations d ON d.position_id=p.id WHERE p.account_id=a.id AND date(d.valuation_date)<=date('now','localtime')) WHEN bs.id IS NULL THEN NULL ELSE COALESCE((
                      SELECT MAX(t.booking_date) FROM transactions t
                      WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                        AND date(t.booking_date) <= date('now', 'localtime')
                    ), bs.balance_date) END,
                    CASE WHEN a.account_type = 'manual_asset' THEN COALESCE((SELECT d.currency FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id LIMIT 1),a.currency) ELSE a.currency END,
                    a.include_in_net_worth, i.logo_data_url
             FROM accounts a
             JOIN institutions i ON i.id = a.institution_id
             LEFT JOIN balance_snapshots bs ON bs.id = (
               SELECT latest.id FROM balance_snapshots latest
               WHERE latest.account_id = a.id
                 AND date(latest.balance_date) <= date('now', 'localtime')
               ORDER BY latest.balance_date DESC, latest.id DESC LIMIT 1
             )
             WHERE a.is_active = 1
             ORDER BY COALESCE(bs.amount_minor, 0) DESC, i.name, a.name",
        )
        .map_err(db_error)?;
    let accounts = query
        .query_map([], |row| {
            Ok(DashboardAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                provider_key: row.get(2)?,
                name: row.get(3)?,
                account_type: row.get(4)?,
                currency: row.get(5)?,
                balance_minor: row.get(6)?,
                balance_date: row.get(7)?,
                balance_currency: row.get(8)?,
                include_in_net_worth: row.get(9)?,
                logo_data_url: row.get(10)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut providers = Vec::<DashboardProvider>::new();
    for account in &accounts {
        if !account.include_in_net_worth {
            continue;
        }
        let chf_balance = if account.balance_currency == "CHF" {
            account.balance_minor.unwrap_or(0)
        } else {
            0
        };
        if let Some(provider) = providers
            .iter_mut()
            .find(|provider| provider.provider_key == account.provider_key)
        {
            provider.balance_minor += chf_balance;
            provider.account_count += 1;
        } else {
            providers.push(DashboardProvider {
                provider: account.provider.clone(),
                provider_key: account.provider_key.clone(),
                balance_minor: chf_balance,
                account_count: 1,
                logo_data_url: account.logo_data_url.clone(),
            });
        }
    }
    providers.sort_by_key(|provider| std::cmp::Reverse(provider.balance_minor));
    let recent_import = connection
        .query_row(
            "SELECT ir.source_name, i.name, a.name, ir.imported_at, ir.transaction_count
             FROM import_runs ir
             JOIN accounts a ON a.id = ir.account_id
             JOIN institutions i ON i.id = a.institution_id
             ORDER BY ir.imported_at DESC, ir.id DESC LIMIT 1",
            [],
            |row| {
                Ok(RecentImport {
                    source_name: row.get(0)?,
                    provider: row.get(1)?,
                    account_name: row.get(2)?,
                    imported_at: row.get(3)?,
                    transaction_count: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(db_error)?;
    let transaction_count = count(&connection, "transactions")?;
    let total_balance_minor = accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
        .filter(|account| account.include_in_net_worth)
        .filter_map(|account| account.balance_minor)
        .sum();
    Ok(DashboardData {
        total_balance_minor,
        currency: "CHF".to_string(),
        accounts,
        providers,
        recent_import,
        transaction_count,
    })
}

fn count(connection: &Connection, table: &str) -> Result<i64, String> {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(db_error)
}

fn provider_metadata(provider: &str) -> (&'static str, &'static str, &'static str) {
    match provider {
        "ubs" => ("UBS", "bank", "cash"),
        "swissquote" => ("Swissquote", "bank", "cash"),
        "migros" => ("Migros Bank", "bank", "cash"),
        "raiffeisen" => ("Raiffeisen", "bank", "cash"),
        "generali" => ("Generali", "insurance", "pillar3a"),
        _ => ("Unbekannter Anbieter", "bank", "cash"),
    }
}

fn db_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::InvalidQuery) {
        return "Finanzblick ist gesperrt. Bitte erneut entsperren.".into();
    }
    format!("Die lokale Datenbank konnte nicht aktualisiert werden: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importers::{ParsedStatement, ParsedTransaction};

    #[test]
    fn initializes_only_the_current_normalized_valuation_schema() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_schema(&connection).unwrap();
        initialize_schema(&connection).unwrap();

        let tables = {
            let mut query = connection
                .prepare("SELECT name FROM sqlite_master WHERE type='table'")
                .unwrap();
            query
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<Result<HashSet<_>, _>>()
                .unwrap()
        };
        for required in [
            "instruments",
            "instrument_identifiers",
            "instrument_listings",
            "instrument_prices",
            "portfolio_positions",
            "position_quantities",
            "manual_position_values",
            "fx_rates",
            "daily_valuations",
            "annual_tax_snapshots",
            "annual_tax_snapshot_breakdowns",
        ] {
            assert!(tables.contains(required), "missing table {required}");
        }
        let price_columns = {
            let mut query = connection
                .prepare("PRAGMA table_info(instrument_prices)")
                .unwrap();
            query
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<HashSet<_>, _>>()
                .unwrap()
        };
        for required in [
            "price_date",
            "price_at",
            "price_type",
            "price_amount",
            "price_scale",
            "currency",
            "source",
            "fetched_at",
        ] {
            assert!(
                price_columns.contains(required),
                "missing column {required}"
            );
        }
        assert!(!price_columns.contains("captured_at"));
        assert!(!price_columns.contains("unit_price_minor"));
    }

    #[test]
    fn foreign_prices_require_and_reference_an_explicit_fx_rate() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-fx-valuation-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'broker','Broker','broker','2020-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Depot','manual_asset','AUD','2020-01-01');
             INSERT INTO instruments(id,name,asset_type) VALUES(1,'Testinstrument','stock');
             INSERT INTO instrument_listings(id,instrument_id,exchange_mic,market_symbol,quote_currency) VALUES(1,1,'XASX','XYZ','AUD');
             INSERT INTO instrument_prices(id,listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) VALUES(1,1,'2020-01-01','eod_close',1000,2,'AUD','test','2020-01-01');
             INSERT INTO portfolio_positions(id,account_id,listing_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,1,'Testposition','stock','2020-01-01','2020-01-02','2020-01-01','2020-01-01');
             INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(1,'2020-01-01',1000000,6,'manual','2020-01-01');",
        ).unwrap();
        drop(connection);

        rebuild_daily_valuations(&storage).unwrap();
        let connection = storage.connect().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM daily_valuations", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        connection.execute(
            "INSERT INTO fx_rates(id,base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at) VALUES(1,'AUD','CHF','2020-01-01',700000000,9,'test','2020-01-01')",
            [],
        ).unwrap();
        drop(connection);

        rebuild_daily_valuations(&storage).unwrap();
        let connection = storage.connect().unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*),MIN(value_minor),MAX(value_minor),COUNT(fx_rate_id) FROM daily_valuations",
                    [],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
                )
                .unwrap(),
            (2, 700, 700, 2)
        );
        drop(connection);
        let wealth = wealth_from(&storage, Some(&[1])).unwrap();
        assert_eq!(wealth.current_total_minor, 0);
        assert_eq!(
            wealth
                .history
                .iter()
                .find(|point| point.date == "2020-01-01")
                .map(|point| point.total_minor),
            Some(700)
        );
        assert_eq!(
            wealth
                .history
                .last()
                .map(|point| (point.date.as_str(), point.total_minor)),
            Some(("2020-01-03", 0))
        );
        storage
            .connect()
            .unwrap()
            .execute(
                "UPDATE portfolio_positions SET holding_start_date=date('now'),holding_end_date=NULL WHERE id=1",
                [],
            )
            .unwrap();
        rebuild_daily_valuations(&storage).unwrap();
        let dashboard = dashboard_from(&storage).unwrap();
        assert_eq!(dashboard.total_balance_minor, 700);
        assert_eq!(dashboard.accounts[0].currency, "AUD");
        assert_eq!(dashboard.accounts[0].balance_currency, "CHF");
        assert_eq!(dashboard.providers[0].balance_minor, 700);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn automatic_valuations_do_not_mix_market_price_sources() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-price-source-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'broker','Broker','broker','2020-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Krypto','manual_asset','USD','2020-01-01');
             INSERT INTO instruments(id,name,asset_type) VALUES(1,'Coin','crypto');
             INSERT INTO instrument_listings(id,instrument_id,market_symbol,quote_currency,preferred_price_source) VALUES(1,1,'COIN-USD','USD','yahoo');
             INSERT INTO instrument_prices(id,listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) VALUES
               (1,1,'2020-01-01','eod_close',1000,2,'USD','alpha_vantage','2020-01-01'),
               (2,1,'2020-01-02','eod_close',2000,2,'USD','alpha_vantage','2020-01-02'),
               (3,1,'2020-01-01','eod_close',10000,2,'USD','yahoo','2020-01-01');
             INSERT INTO portfolio_positions(id,account_id,listing_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,1,'Coin','crypto','2020-01-01','2020-01-02','2020-01-01','2020-01-01');
             INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(1,'2020-01-01',1000000,6,'manual','2020-01-01');
             INSERT INTO fx_rates(base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at) VALUES('USD','CHF','2020-01-01',1000000000,9,'test','2020-01-01');",
        ).unwrap();
        drop(connection);

        rebuild_daily_valuations(&storage).unwrap();
        let connection = storage.connect().unwrap();
        let values = connection
            .prepare("SELECT value_minor FROM daily_valuations ORDER BY valuation_date")
            .unwrap()
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(values, vec![10_000, 10_000]);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn deletes_imports_atomically_restores_balances_and_allows_reimport() {
        let directory =
            std::env::temp_dir().join(format!("finanzblick-delete-imports-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        let first_source = directory.join("first.xlsx");
        let second_source = directory.join("second.xlsx");
        fs::write(&first_source, b"first deletion fixture").unwrap();
        fs::write(&second_source, b"second deletion fixture").unwrap();
        let first = save_import_to(
            &storage,
            request(first_source.to_string_lossy().into_owned()),
        )
        .unwrap();
        let mut req = request(second_source.to_string_lossy().into_owned());
        req.statement.transactions[0].booking_date = "2026-09-01".into();
        req.statement.transactions[0].balance_minor = Some(50000);
        req.statement.closing_balance_minor = None;
        let mut usd = req.statement.transactions[0].clone();
        usd.currency = "USD".into();
        usd.source_row = 5;
        usd.balance_minor = Some(1000);
        req.statement.transactions.push(usd);
        let second = save_import_to(&storage, req).unwrap();
        let listed = imports_from(&storage).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed[0].accounts.contains("USD"));
        assert_eq!(listed[0].transaction_count, 2);
        assert_eq!(dashboard_from(&storage).unwrap().total_balance_minor, 50000);
        assert!(delete_imports_from(&storage, vec![first.import_id, 999999]).is_err());
        assert_eq!(imports_from(&storage).unwrap().len(), 2); // Entire batch rolls back.
        assert_eq!(
            delete_imports_from(&storage, vec![second.import_id]).unwrap(),
            1
        );
        assert_eq!(dashboard_from(&storage).unwrap().total_balance_minor, 98750);
        assert_eq!(count(&connection, "transactions").unwrap(), 1);
        assert_eq!(count(&connection, "balance_snapshots").unwrap(), 1);
        assert_eq!(count(&connection, "accounts").unwrap(), 2);
        assert!(second_source.exists());
        let mut reimport_request = request(second_source.to_string_lossy().into_owned());
        reimport_request.statement.transactions[0].booking_date = "2026-09-02".into();
        let reimport = save_import_to(&storage, reimport_request).unwrap();
        assert!(!reimport.duplicate);
        assert_eq!(
            delete_imports_from(
                &storage,
                vec![first.import_id, reimport.import_id, first.import_id]
            )
            .unwrap(),
            2
        );
        assert_eq!(count(&connection, "transactions").unwrap(), 0);
        assert_eq!(count(&connection, "balance_snapshots").unwrap(), 0);
        assert_eq!(count(&connection, "accounts").unwrap(), 2);
        assert_eq!(dashboard_from(&storage).unwrap().total_balance_minor, 0);
        assert!(delete_imports_from(&storage, vec![]).is_err());
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imports_into_selected_account_and_rejects_invalid_mapping() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-account-selection-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("first.xlsx");
        fs::write(&source, b"first selection test").unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        let first =
            save_import_to(&storage, request(source.to_string_lossy().into_owned())).unwrap();
        connection
            .execute(
                "UPDATE accounts SET name = 'Mein bestehendes Konto' WHERE id = ?1",
                [first.account_id],
            )
            .unwrap();
        let second_source = directory.join("second.xlsx");
        fs::write(&second_source, b"second selection test").unwrap();
        let make_request = || {
            let mut req = request(second_source.to_string_lossy().into_owned());
            req.account_name = "Dieser Name darf kein Konto erzeugen".into();
            req.account_ids.insert("CHF".into(), first.account_id);
            req
        };
        let mut invalid = make_request();
        invalid.account_ids.insert("CHF".into(), -1);
        assert!(save_import_to(&storage, invalid).is_err());
        let mut invalid = make_request();
        invalid.account_ids.clear();
        invalid.account_ids.insert("USD".into(), first.account_id);
        assert!(save_import_to(&storage, invalid).is_err());
        let mut invalid = make_request();
        invalid.statement.provider = "swissquote".into();
        assert!(save_import_to(&storage, invalid).is_err());
        let mut invalid = make_request();
        invalid.statement.account_type = Some("credit_card".into());
        assert!(save_import_to(&storage, invalid).is_err());
        connection
            .execute(
                "UPDATE accounts SET is_active = 0 WHERE id = ?1",
                [first.account_id],
            )
            .unwrap();
        assert!(save_import_to(&storage, make_request()).is_err());
        connection
            .execute(
                "UPDATE accounts SET is_active = 1 WHERE id = ?1",
                [first.account_id],
            )
            .unwrap();
        assert_eq!(count(&connection, "import_runs").unwrap(), 1);
        let second = save_import_to(&storage, make_request()).unwrap();
        assert_eq!(second.account_id, first.account_id);
        assert!(second.duplicate);
        assert_eq!(second.inserted_transactions, 0);
        assert_eq!(count(&connection, "accounts").unwrap(), 1);
        assert_eq!(count(&connection, "institutions").unwrap(), 1);
        assert_eq!(count(&connection, "transactions").unwrap(), 1);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn stores_separate_currencies_and_zero_activity_balance_atomically() {
        let directory =
            std::env::temp_dir().join(format!("finanzblick-currency-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("statement.pdf");
        fs::write(&source, b"synthetic multi currency statement").unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().unwrap()).unwrap();
        let mut req = request(source.to_string_lossy().into_owned());
        req.statement.provider = "swissquote".into();
        let mut usd = req.statement.transactions[0].clone();
        usd.currency = "USD".into();
        usd.balance_minor = Some(1234);
        usd.source_row = 5;
        req.statement.transactions.push(usd);
        req.statement.currency_balances = [("CHF", 98750), ("USD", 1234), ("EUR", 181)]
            .into_iter()
            .map(|(currency, balance)| crate::importers::CurrencyBalance {
                currency: currency.into(),
                opening_date: None,
                opening_balance_minor: 0,
                closing_balance_minor: balance,
                closing_date: "2026-08-31".into(),
            })
            .collect();
        req.statement.closing_balance_minor = None;
        let duplicate_request = request(source.to_string_lossy().into_owned());
        assert_eq!(
            save_import_to(&storage, req).unwrap().inserted_transactions,
            2
        );
        assert!(
            save_import_to(&storage, duplicate_request)
                .unwrap()
                .duplicate
        );
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap(); // Reopening the DB must preserve the snapshots.
        assert_eq!(count(&connection, "accounts").unwrap(), 3);
        assert_eq!(count(&connection, "balance_snapshots").unwrap(), 5);
        let mismatches: i64 = connection.query_row("SELECT COUNT(*) FROM transactions t JOIN accounts a ON a.id=t.account_id WHERE t.currency != a.currency", [], |r| r.get(0)).unwrap();
        assert_eq!(mismatches, 0);
        let accounts = accounts_from(&storage).unwrap();
        for (currency, balance) in [("CHF", 98750), ("USD", 1234), ("EUR", 181)] {
            let account = accounts.iter().find(|a| a.currency == currency).unwrap();
            assert_eq!(account.balance_minor, Some(balance));
            assert_eq!(account.import_count, 1);
        }
        assert_eq!(dashboard_from(&storage).unwrap().total_balance_minor, 98750);
        assert_eq!(
            dashboard_from(&storage).unwrap().providers[0].balance_minor,
            98750
        );
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn sold_position_remains_in_history_but_not_in_current_assets() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-sold-position-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'broker','Broker','broker','2020-01-01')", []).unwrap();
        connection.execute("INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Depot','manual_asset','CHF','2020-01-01')", []).unwrap();
        connection.execute("INSERT INTO portfolio_positions(id,account_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,'Historische Aktie','stock','2020-01-01','2020-01-02','2020-01-01','2020-01-01'),(2,1,'Historische Aktie 2','stock','2020-01-01','2020-01-02','2020-01-01','2020-01-01')", []).unwrap();
        connection.execute("INSERT INTO manual_position_values(position_id,value_date,amount_minor,currency,source,recorded_at) VALUES(1,'2020-01-01',10000,'CHF','manual','2020-01-01'),(1,'2020-01-02',12000,'CHF','manual','2020-01-02'),(2,'2020-01-01',6000,'CHF','manual','2020-01-01')", []).unwrap();
        drop(connection);
        rebuild_daily_valuations(&storage).unwrap();

        let wealth = wealth_from(&storage, None).unwrap();
        assert_eq!(wealth.current_total_minor, 0);
        assert_eq!(
            wealth
                .history
                .iter()
                .map(|point| (point.date.as_str(), point.total_minor))
                .collect::<Vec<_>>(),
            vec![
                ("2020-01-01", 16000),
                ("2020-01-02", 18000),
                ("2020-01-03", 0)
            ]
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shared_instrument_prices_value_each_position_by_its_holding_period() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-instrument-price-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        let connection = storage.connect().unwrap();
        initialize_schema(&connection).unwrap();
        connection.execute("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'broker','Broker','broker','2020-01-01')", []).unwrap();
        connection.execute("INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Depot','manual_asset','CHF','2020-01-01')", []).unwrap();
        connection
            .execute(
                "INSERT INTO instruments(id,name,asset_type) VALUES(1,'Testinstrument','stock')",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO instrument_identifiers(instrument_id,identifier_type,identifier) VALUES(1,'ticker','XASX:XYZ')", []).unwrap();
        connection.execute("INSERT INTO instrument_listings(id,instrument_id,exchange_mic,market_symbol,quote_currency) VALUES(1,1,'XASX','XYZ','CHF')", []).unwrap();
        connection.execute("INSERT INTO instrument_prices(id,listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) VALUES(1,1,'2020-08-01','eod_close',130,2,'CHF','test','2020-08-01'),(2,1,'2020-09-01','eod_close',100,2,'CHF','test','2020-09-01'),(3,1,'2020-09-02','eod_close',110,2,'CHF','test','2020-09-02'),(4,1,'2020-09-03','eod_close',120,2,'CHF','test','2020-09-03'),(5,1,'2020-09-04','eod_close',130,2,'CHF','test','2020-09-04')", []).unwrap();
        connection.execute("INSERT INTO portfolio_positions(id,account_id,listing_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,1,'Testposition 1','stock','2020-09-01','2020-09-03','2020-09-01','2020-09-01'),(2,1,1,'Testposition 2','stock','2020-09-02','2020-09-04','2020-09-02','2020-09-02'),(3,1,1,'Testposition 3','stock','2020-08-01','2020-08-31','2020-08-01','2020-08-01')", []).unwrap();
        connection.execute("INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(1,'2020-09-01',10000000,6,'manual','2020-09-01'),(1,'2020-09-03',20000000,6,'manual','2020-09-03'),(2,'2020-09-02',5000000,6,'manual','2020-09-02'),(3,'2020-08-01',2000000,6,'manual','2020-08-01')", []).unwrap();
        drop(connection);
        rebuild_daily_valuations(&storage).unwrap();

        let connection = storage.connect().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM daily_valuations", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            37
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM daily_valuations WHERE instrument_price_id IS NULL",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
        drop(connection);

        let wealth = wealth_from(&storage, Some(&[1])).unwrap();
        assert_eq!(wealth.history.len(), 36);
        for (date, expected) in [
            ("2020-08-01", 260),
            ("2020-08-15", 260),
            ("2020-09-01", 1000),
            ("2020-09-02", 1650),
            ("2020-09-03", 3000),
            ("2020-09-04", 650),
            ("2020-09-05", 0),
        ] {
            assert_eq!(
                wealth
                    .history
                    .iter()
                    .find(|point| point.date == date)
                    .map(|point| point.total_minor),
                Some(expected)
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    fn request(source_path: String) -> SaveImportRequest {
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path,
            account_name: "Privatkonto CHF".to_string(),
            statement: ParsedStatement {
                currency_balances: Vec::new(),
                account_type: None,
                provider: "ubs".to_string(),
                format: "XLSX".to_string(),
                account_name: "Privatkonto CHF".to_string(),
                transactions: vec![ParsedTransaction {
                    booking_date: "2026-08-01".to_string(),
                    value_date: Some("2026-08-01".to_string()),
                    description: "Testbuchung".to_string(),
                    industry: None,
                    amount_minor: -1250,
                    balance_minor: Some(98750),
                    currency: "CHF".to_string(),
                    confidence: 1.0,
                    source_row: 4,
                }],
                opening_balance_minor: Some(100000),
                closing_balance_minor: Some(98750),
                warnings: vec![],
            },
        }
    }

    #[test]
    fn persists_an_import_and_detects_the_same_file() {
        let directory =
            std::env::temp_dir().join(format!("finanzblick-storage-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create test directory");
        let source = directory.join("statement.xlsx");
        fs::write(&source, b"synthetic statement").expect("write test source");
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().expect("open database")).expect("initialize database");

        let first = save_import_to(&storage, request(source.to_string_lossy().into_owned()))
            .expect("save import");
        assert!(!first.duplicate);
        assert_eq!(first.inserted_transactions, 1);

        let second = save_import_to(&storage, request(source.to_string_lossy().into_owned()))
            .expect("detect duplicate");
        assert!(second.duplicate);
        assert_eq!(first.import_id, second.import_id);

        let connection = storage.connect().expect("reopen database");
        assert_eq!(count(&connection, "accounts").unwrap(), 1);
        assert_eq!(count(&connection, "import_runs").unwrap(), 1);
        assert_eq!(count(&connection, "transactions").unwrap(), 1);
        assert_eq!(count(&connection, "categories").unwrap(), 17);
        let category: String = connection
            .query_row(
                "SELECT c.category_key FROM transactions t JOIN categories c ON c.id=t.category_id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("transaction category");
        assert_eq!(category, "other");

        let dashboard = dashboard_from(&storage).expect("load dashboard");
        assert_eq!(dashboard.total_balance_minor, 98750);
        assert_eq!(dashboard.accounts.len(), 1);
        assert_eq!(dashboard.providers.len(), 1);
        assert_eq!(dashboard.transaction_count, 1);
        assert!(dashboard.recent_import.is_some());
        let wealth = wealth_from(&storage, None).expect("load wealth data");
        assert_eq!(wealth.current_total_minor, 98750);
        assert_eq!(wealth.history.len(), 1);
        assert_eq!(wealth.by_type[0].key, "cash");

        connection
            .execute(
                "INSERT INTO balance_snapshots(account_id,import_id,balance_date,amount_minor,currency) VALUES(?1,?2,'2999-12-31',999999,'CHF')",
                params![first.account_id, first.import_id],
            )
            .expect("insert future balance fixture");
        let dashboard = dashboard_from(&storage).expect("ignore future balance");
        assert_eq!(dashboard.accounts[0].balance_minor, Some(98750));
        assert_eq!(
            dashboard.accounts[0].balance_date.as_deref(),
            Some("2026-08-01")
        );
        assert_eq!(dashboard.total_balance_minor, 98750);
        let wealth = wealth_from(&storage, None).expect("ignore future wealth snapshot");
        assert_eq!(
            wealth.history.last().map(|point| point.date.as_str()),
            Some("2026-08-01")
        );
        assert_eq!(wealth.current_total_minor, 98750);

        connection
            .execute(
                "INSERT INTO transactions(account_id,import_id,booking_date,description,amount_minor,currency,confidence,source_row) VALUES(?1,?2,'2026-08-02','Spätere Gutschrift',1250,'CHF',1,99)",
                params![first.account_id, first.import_id],
            )
            .expect("insert transaction after latest balance");
        let dashboard = dashboard_from(&storage).expect("include transactions after balance");
        assert_eq!(dashboard.accounts[0].balance_minor, Some(100000));
        assert_eq!(
            dashboard.accounts[0].balance_date.as_deref(),
            Some("2026-08-02")
        );

        connection
            .execute(
                "UPDATE accounts SET include_in_net_worth = 0 WHERE id = ?1",
                [first.account_id],
            )
            .expect("exclude account from net worth");
        let managed = accounts_from(&storage).expect("load managed accounts");
        assert_eq!(managed.len(), 1);
        assert!(!managed[0].include_in_net_worth);
        assert_eq!(dashboard_from(&storage).unwrap().total_balance_minor, 0);
        assert_eq!(
            wealth_from(&storage, None).unwrap().excluded_account_count,
            1
        );

        drop(connection);
        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn stores_the_dated_opening_balance_for_the_real_balance_history() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-opening-balance-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create test directory");
        let source = directory.join("statement.mt940");
        fs::write(&source, b"synthetic MT940 statement").expect("write test source");
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().expect("open database")).expect("initialize database");

        let mut import = request(source.to_string_lossy().into_owned());
        import.statement.format = "MT940".to_string();
        import.statement.currency_balances = vec![crate::importers::CurrencyBalance {
            currency: "CHF".to_string(),
            opening_date: Some("2026-07-31".to_string()),
            opening_balance_minor: 100000,
            closing_balance_minor: 98750,
            closing_date: "2026-08-01".to_string(),
        }];

        save_import_to(&storage, import).expect("save MT940 import");

        let connection = storage.connect().expect("reopen database");
        let snapshots = connection
            .prepare(
                "SELECT balance_date,amount_minor FROM balance_snapshots ORDER BY balance_date,id",
            )
            .expect("prepare snapshot query")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("query snapshots")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("read snapshots");
        assert_eq!(
            snapshots,
            vec![
                ("2026-07-31".to_string(), 100000),
                ("2026-08-01".to_string(), 98750),
            ]
        );

        let history = transaction_history(&connection, &None, None).expect("load balance history");
        assert_eq!(history.first().map(|point| point.total_minor), Some(100000));
        assert_eq!(history.last().map(|point| point.total_minor), Some(98750));

        drop(connection);
        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn overlapping_import_inserts_only_new_transaction_occurrences() {
        let directory =
            std::env::temp_dir().join(format!("finanzblick-overlap-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create overlap test directory");
        let first_source = directory.join("monthly.xlsx");
        let combined_source = directory.join("combined.xlsx");
        fs::write(&first_source, b"monthly statement").unwrap();
        fs::write(&combined_source, b"combined statement").unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().unwrap()).unwrap();

        save_import_to(
            &storage,
            request(first_source.to_string_lossy().into_owned()),
        )
        .unwrap();
        let mut combined = request(combined_source.to_string_lossy().into_owned());
        let repeated = combined.statement.transactions[0].clone();
        combined.statement.transactions.push(repeated.clone());
        let mut new_row = repeated;
        new_row.booking_date = "2026-08-02".into();
        new_row.description = "Neue Buchung".into();
        new_row.source_row = 6;
        combined.statement.transactions.push(new_row);

        let result = save_import_to(&storage, combined).unwrap();
        assert!(!result.duplicate);
        assert_eq!(result.inserted_transactions, 2);
        let connection = storage.connect().unwrap();
        assert_eq!(count(&connection, "transactions").unwrap(), 3);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM transactions WHERE booking_date='2026-08-01' AND amount_minor=-1250 AND trim(description)='Testbuchung'",
                    [],
                    |row| row.get::<_, usize>(0),
                )
                .unwrap(),
            2
        );
        drop(connection);
        drop(storage);
        fs::remove_dir_all(directory).expect("remove overlap test directory");
    }

    #[test]
    fn rolling_mt940_import_skips_overlap_and_extends_balance_history() {
        let directory = std::env::temp_dir().join(format!(
            "finanzblick-rolling-mt940-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create rolling import test directory");
        let first_source = directory.join("september.mt940");
        let second_source = directory.join("december.mt940");
        fs::write(&first_source, b"first rolling MT940 statement").unwrap();
        fs::write(&second_source, b"second rolling MT940 statement").unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().unwrap()).unwrap();

        let transaction =
            |date: &str, description: &str, amount: i64, balance: i64, row| ParsedTransaction {
                booking_date: date.to_string(),
                value_date: Some(date.to_string()),
                description: description.to_string(),
                industry: None,
                amount_minor: amount,
                balance_minor: Some(balance),
                currency: "CHF".to_string(),
                confidence: 1.0,
                source_row: row,
            };

        let mut september = request(first_source.to_string_lossy().into_owned());
        september.statement.format = "MT940".to_string();
        september.statement.transactions = vec![
            transaction("2025-07-01", "Buchung A", -100, 9900, 1),
            transaction("2025-08-01", "Buchung B", 200, 10100, 2),
            transaction("2025-09-01", "Buchung C", -300, 9800, 3),
        ];
        september.statement.currency_balances = vec![crate::importers::CurrencyBalance {
            currency: "CHF".to_string(),
            opening_date: Some("2025-06-30".to_string()),
            opening_balance_minor: 10000,
            closing_balance_minor: 9800,
            closing_date: "2025-09-01".to_string(),
        }];
        let first = save_import_to(&storage, september).expect("save September statement");
        assert_eq!(first.inserted_transactions, 3);

        let mut december = request(second_source.to_string_lossy().into_owned());
        december.statement.format = "MT940".to_string();
        december.statement.transactions = vec![
            transaction("2025-08-01", "Buchung B", 200, 10100, 1),
            transaction("2025-09-01", "Buchung C", -300, 9800, 2),
            transaction("2025-10-01", "Buchung D", 500, 10300, 3),
            transaction("2025-12-01", "Buchung E", -100, 10200, 4),
        ];
        december.statement.currency_balances = vec![crate::importers::CurrencyBalance {
            currency: "CHF".to_string(),
            opening_date: Some("2025-07-31".to_string()),
            opening_balance_minor: 9900,
            closing_balance_minor: 10200,
            closing_date: "2025-12-01".to_string(),
        }];
        let second = save_import_to(&storage, december).expect("save December statement");
        assert_eq!(second.inserted_transactions, 2);
        assert!(!second.duplicate);

        let connection = storage.connect().unwrap();
        assert_eq!(count(&connection, "transactions").unwrap(), 5);
        let history = transaction_history(&connection, &None, None).unwrap();
        assert_eq!(
            history
                .first()
                .map(|point| (point.date.as_str(), point.total_minor)),
            Some(("2025-06-30", 10000))
        );
        assert_eq!(
            history
                .last()
                .map(|point| (point.date.as_str(), point.total_minor)),
            Some(("2025-12-01", 10200))
        );

        drop(connection);
        drop(storage);
        fs::remove_dir_all(directory).expect("remove rolling import test directory");
    }
}
