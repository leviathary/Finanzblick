//! Regressionstests für Konten, Bewertungen und gemeinsame Auswertungen.
use super::*;
use crate::importers::{ParsedSecurityDetails, ParsedStatement, ParsedTransaction};

#[test]
fn pillar3a_is_manually_valued_and_excluded_from_assets_by_default() {
    assert!(!default_include_in_net_worth("pillar3a"));
    assert!(default_include_in_net_worth("cash"));

    let directory =
        std::env::temp_dir().join(format!("finanzblick-pillar3a-test-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let storage = Storage::test_storage(directory.join("test.sqlite3"));
    let connection = storage.connect().unwrap();
    initialize_schema(&connection).unwrap();
    connection.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'pension-provider','Vorsorgeanbieter','pension',datetime('now'));
             INSERT INTO accounts(id,institution_id,name,account_type,currency,include_in_net_worth,created_at) VALUES(1,1,'Vorsorgekonto','pillar3a','CHF',0,datetime('now'));
             INSERT INTO portfolio_positions(id,account_id,label,asset_type,holding_start_date,created_at,updated_at) VALUES(1,1,'Vorsorgeguthaben','cash',date('now','localtime'),datetime('now'),datetime('now'));
             INSERT INTO manual_position_values(position_id,value_date,amount_minor,currency,source,recorded_at) VALUES(1,date('now','localtime'),1234500,'CHF','manual',datetime('now'));",
        ).unwrap();
    drop(connection);
    rebuild_daily_valuations(&storage).unwrap();

    let accounts = accounts_from(&storage).unwrap();
    assert_eq!(accounts[0].balance_minor, Some(1_234_500));
    assert_eq!(accounts[0].manual_valuation_count, 1);
    assert_eq!(wealth_from(&storage, None).unwrap().current_total_minor, 0);

    storage
        .connect()
        .unwrap()
        .execute("UPDATE accounts SET include_in_net_worth=1 WHERE id=1", [])
        .unwrap();
    assert_eq!(
        wealth_from(&storage, None).unwrap().current_total_minor,
        1_234_500
    );
    let wealth = wealth_from(&storage, None).unwrap();
    assert_eq!(wealth.by_type[0].key, "pillar3a");
    assert_eq!(wealth.by_type[0].label, "Vorsorgekonten");
    fs::remove_dir_all(directory).unwrap();
}

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
    let first = save_import_to(&storage, request(source.to_string_lossy().into_owned())).unwrap();
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
                ..ParsedTransaction::default()
            }],
            opening_balance_minor: Some(100000),
            closing_balance_minor: Some(98750),
            warnings: vec![],
            ..ParsedStatement::default()
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
fn stores_document_and_security_metadata_and_deduplicates_bank_references() {
    let directory = std::env::temp_dir().join(format!(
        "finanzblick-import-metadata-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    let first_source = directory.join("first.pdf");
    let second_source = directory.join("second.pdf");
    fs::write(&first_source, b"first advice").unwrap();
    fs::write(&second_source, b"second advice").unwrap();
    let storage = Storage::test_storage(directory.join("test.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();

    let mut first = request(first_source.to_string_lossy().into_owned());
    first.statement.provider = "zkb".into();
    first.statement.format = "PDF".into();
    first.statement.document_type = Some("advice".into());
    first.statement.document_date = Some("2026-08-01".into());
    first.statement.account_reference = Some("CH0000000000000000000".into());
    first.statement.transactions[0].transaction_kind = "dividend".into();
    first.statement.transactions[0].reference_namespace = Some("zkb_pdf".into());
    first.statement.transactions[0].external_reference = Some("REFERENCE-1".into());
    first.statement.transactions[0].security_details = Some(ParsedSecurityDetails {
        isin: Some("CH0000000001".into()),
        valor_number: Some("100001".into()),
        quantity: Some("12.5".into()),
        price: Some("100.25".into()),
        price_currency: Some("CHF".into()),
        exchange_rate: None,
        gross_amount_minor: Some(12531),
        fees_minor: Some(25),
        taxes_minor: Some(10),
        withholding_tax_minor: Some(4386),
        accrued_interest_minor: None,
    });
    assert_eq!(
        save_import_to(&storage, first)
            .unwrap()
            .inserted_transactions,
        1
    );

    let mut second = request(second_source.to_string_lossy().into_owned());
    second.statement.provider = "zkb".into();
    second.statement.transactions[0].description = "Anderer Belegtext".into();
    second.statement.transactions[0].reference_namespace = Some("zkb_pdf".into());
    second.statement.transactions[0].external_reference = Some("REFERENCE-1".into());
    let duplicate = save_import_to(&storage, second).unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.inserted_transactions, 0);

    let connection = storage.connect().unwrap();
    assert_eq!(count(&connection, "transactions").unwrap(), 1);
    assert_eq!(count(&connection, "security_transactions").unwrap(), 1);
    assert_eq!(
            connection
                .query_row(
                    "SELECT document_type || ':' || document_date FROM import_document_metadata WHERE document_type IS NOT NULL",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "advice:2026-08-01"
        );
    assert_eq!(
        connection
            .query_row(
                "SELECT transaction_kind || ':' || external_reference FROM transaction_metadata",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "dividend:REFERENCE-1"
    );
    drop(connection);
    fs::remove_dir_all(directory).unwrap();
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
        .prepare("SELECT balance_date,amount_minor FROM balance_snapshots ORDER BY balance_date,id")
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

    let first = save_import_to(
        &storage,
        request(first_source.to_string_lossy().into_owned()),
    )
    .unwrap();
    let mut combined = request(combined_source.to_string_lossy().into_owned());
    combined.account_ids.insert("CHF".into(), first.account_id);
    let repeated = combined.statement.transactions[0].clone();
    combined.statement.transactions.push(repeated.clone());
    let mut new_row = repeated;
    new_row.booking_date = "2026-08-02".into();
    new_row.description = "Neue Buchung".into();
    new_row.source_row = 6;
    combined.statement.transactions.push(new_row);

    let preview = duplicate_check(&storage.connect().unwrap(), &combined, "different").unwrap();
    assert_eq!(preview.matching_transactions, 1);
    assert_eq!(preview.total_transactions, 3);

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
            ..ParsedTransaction::default()
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
