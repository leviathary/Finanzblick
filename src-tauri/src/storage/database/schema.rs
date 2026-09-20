//! Initialisiert das aktuelle Datenmodell und seine Standardkategorien.
use crate::storage::rules::categorization::apply_categories;
use crate::storage::{categories, rules::settlement_rules};
use rusqlite::Connection;

pub(crate) fn initialize_schema(connection: &Connection) -> Result<(), rusqlite::Error> {
    settlement_rules::initialize(connection)?;
    crate::storage::database::schema::initialize_reporting_flags(connection)?;
    connection.execute_batch(
        "PRAGMA secure_delete=ON;
         BEGIN;
         CREATE TABLE IF NOT EXISTS app_settings (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS finance_chat_settings (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
         DELETE FROM finance_chat_settings;
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
         CREATE TABLE IF NOT EXISTS import_mapping_profiles (
           id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
           header_fingerprint TEXT NOT NULL, mapping_json TEXT NOT NULL,
           updated_at TEXT NOT NULL
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
         CREATE TABLE IF NOT EXISTS ignored_duplicate_transactions (
           transaction_id INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
           ignored_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS duplicate_review_exclusions (
           left_transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
           right_transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
           reviewed_at TEXT NOT NULL DEFAULT (datetime('now')),
           PRIMARY KEY(left_transaction_id,right_transaction_id),
           CHECK(left_transaction_id < right_transaction_id)
         );
         CREATE TABLE IF NOT EXISTS import_document_metadata (
           import_id INTEGER PRIMARY KEY REFERENCES import_runs(id) ON DELETE CASCADE,
           source_type TEXT NOT NULL, provider TEXT NOT NULL, document_type TEXT,
           document_date TEXT, value_date TEXT, account_reference TEXT,
           record_definition_id TEXT
         );
         CREATE TABLE IF NOT EXISTS transaction_metadata (
           transaction_id INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
           account_id INTEGER NOT NULL REFERENCES accounts(id),
           transaction_kind TEXT NOT NULL DEFAULT 'cash_transaction',
           reference_namespace TEXT, external_reference TEXT,
           fallback_fingerprint TEXT NOT NULL, counterparty_name TEXT,
           remittance_information TEXT,
           CHECK((reference_namespace IS NULL) = (external_reference IS NULL))
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_transaction_external_reference
           ON transaction_metadata(account_id,reference_namespace,external_reference)
           WHERE external_reference IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_transaction_fallback_fingerprint
           ON transaction_metadata(account_id,fallback_fingerprint)
           WHERE external_reference IS NULL;
         CREATE TABLE IF NOT EXISTS security_transactions (
           transaction_id INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
           isin TEXT, valor_number TEXT, quantity TEXT, price TEXT, price_currency TEXT,
           exchange_rate TEXT, gross_amount_minor INTEGER, fees_minor INTEGER,
           taxes_minor INTEGER, withholding_tax_minor INTEGER, accrued_interest_minor INTEGER
         );
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

pub(in crate::storage) fn initialize_reporting_flags(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS transaction_reporting_flags (
        transaction_id INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
        exclude_from_cashflow INTEGER NOT NULL DEFAULT 0 CHECK(exclude_from_cashflow IN (0,1)),
        is_settlement INTEGER NOT NULL DEFAULT 0 CHECK(is_settlement IN (0,1)),
        is_manually_overridden INTEGER NOT NULL DEFAULT 0 CHECK(is_manually_overridden IN (0,1)),
        CHECK(is_settlement=0 OR exclude_from_cashflow=1)
    );
    CREATE TABLE IF NOT EXISTS card_credit_decisions(
      transaction_id INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
      kind TEXT NOT NULL CHECK(kind IN ('REFUND','UNKNOWN'))
    );",
    )
}
