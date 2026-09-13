use super::*;

#[test]
fn category_priority_is_manual_then_merchant_then_industry_then_description() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_schema(&connection).unwrap();
    connection.execute_batch(
        "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'ubs','UBS','bank','2026-01-01');
         INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Kreditkarte','credit_card','CHF','2026-01-01');
         INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(1,1,'card.csv','CSV','industry-test','2026-01-01',3,'[]');
         INSERT INTO transactions(id,account_id,import_id,booking_date,description,industry,amount_minor,currency,confidence,source_row,category_id,category_manual,category_source)
         SELECT 1,1,1,'2026-01-01','Restaurant im Namen','Lebensmittelgeschäfte',-100,'CHF',1,1,NULL,0,'description'
         UNION ALL SELECT 2,1,1,'2026-01-01','Manuell','Lebensmittelgeschäfte',-100,'CHF',1,2,id,1,'manual' FROM categories WHERE category_key='housing'
         UNION ALL SELECT 3,1,1,'2026-01-01','Shop 123','Lebensmittelgeschäfte',-100,'CHF',1,3,NULL,0,'description';
         INSERT INTO merchant_category_rules(merchant_key,category_id) SELECT 'shop',id FROM categories WHERE category_key='leisure';"
    ).unwrap();
    apply_categories(&connection).unwrap();
    let rows = connection.prepare("SELECT c.category_key,t.category_source FROM transactions t JOIN categories c ON c.id=t.category_id ORDER BY t.id").unwrap()
        .query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).unwrap()
        .collect::<Result<Vec<_>,_>>().unwrap();
    assert_eq!(
        rows,
        vec![
            ("groceries".into(), "industry".into()),
            ("housing".into(), "manual".into()),
            ("leisure".into(), "merchant".into())
        ]
    );
}

#[test]
fn restaurants_are_separate_from_leisure_and_sport() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_schema(&connection).unwrap();
    connection.execute_batch(
        "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'ubs','UBS','bank','2026-01-01');
         INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Kreditkarte','credit_card','CHF','2026-01-01');
         INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(1,1,'card.csv','CSV','category-split','2026-01-01',6,'[]');
         INSERT INTO transactions(id,account_id,import_id,booking_date,description,industry,amount_minor,currency,confidence,source_row,category_id,category_manual,category_source)
         VALUES(1,1,1,'2026-01-01','Restaurant Muster',NULL,-100,'CHF',1,1,NULL,0,'description'),
               (2,1,1,'2026-01-01','Kino Muster',NULL,-100,'CHF',1,2,NULL,0,'description'),
               (3,1,1,'2026-01-01','Kartenzahlung','Fast-Food Restaurants',-100,'CHF',1,3,NULL,0,'description'),
               (4,1,1,'2026-01-01','Restaurant manuell',NULL,-100,'CHF',1,4,(SELECT id FROM categories WHERE category_key='leisure'),1,'manual'),
               (5,1,1,'2026-01-01','Fitnesscenter Muster',NULL,-100,'CHF',1,5,NULL,0,'description'),
               (6,1,1,'2026-01-01','Restaurant Altbestand',NULL,-100,'CHF',1,6,(SELECT id FROM categories WHERE category_key='leisure'),0,'description');"
    ).unwrap();

    apply_categories(&connection).unwrap();
    let rows = connection
        .prepare("SELECT c.category_key FROM transactions t JOIN categories c ON c.id=t.category_id ORDER BY t.id")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            "restaurants",
            "leisure",
            "restaurants",
            "leisure",
            "leisure",
            "restaurants",
        ]
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT label FROM categories WHERE category_key='leisure'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "Freizeit & Sport"
    );
}

#[test]
fn credit_card_settlements_have_their_own_category() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_schema(&connection).unwrap();
    connection.execute_batch(
        "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'ubs','UBS','bank','2026-01-01');
         INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at)
           VALUES(1,1,'Privatkonto','checking','CHF','2026-01-01'),
                 (2,1,'Kreditkarte','credit_card','CHF','2026-01-01');
         INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json)
           VALUES(1,1,'bank.mt940','MT940','credit-card-category','2026-01-01',4,'[]');
         INSERT INTO transactions(id,account_id,import_id,booking_date,description,amount_minor,currency,confidence,source_row,category_id,category_manual,category_source)
           VALUES(1,1,1,'2026-01-01','UBS Switzerland AG VIS1W WIDERSPRUCH AN UBS INNERT 30 TAGEN',-10000,'CHF',1,1,NULL,0,'description'),
                 (2,1,1,'2026-01-02','Kreditkartenabrechnung August',-20000,'CHF',1,2,(SELECT id FROM categories WHERE category_key='other'),0,'description'),
                 (3,2,1,'2026-01-03','Credit Card Payment Einkauf',-30000,'CHF',1,3,NULL,0,'description'),
                 (4,1,1,'2026-01-04','Kreditkarten-Abrechnung manuell',-40000,'CHF',1,4,(SELECT id FROM categories WHERE category_key='housing'),1,'manual');"
    ).unwrap();

    apply_categories(&connection).unwrap();
    let rows = connection
        .prepare("SELECT c.category_key FROM transactions t JOIN categories c ON c.id=t.category_id ORDER BY t.id")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec!["credit_card", "credit_card", "other", "housing"]);
    assert_eq!(
        connection
            .query_row(
                "SELECT label FROM categories WHERE category_key='credit_card'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "Kreditkarte"
    );
}

#[test]
fn digital_categories_cover_existing_and_new_rows_and_preserve_overrides() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE categories(id INTEGER PRIMARY KEY, category_key TEXT); INSERT INTO categories VALUES(1,'other'),(2,'digital_subscriptions'),(3,'income'),(4,'leisure'),(5,'telecom'); CREATE TABLE accounts(id INTEGER PRIMARY KEY,account_type TEXT); INSERT INTO accounts VALUES(1,'checking'); CREATE TABLE transactions(id INTEGER PRIMARY KEY, account_id INTEGER DEFAULT 1, description TEXT, industry TEXT, amount_minor INTEGER, category_id INTEGER, category_manual INTEGER DEFAULT 0, category_source TEXT NOT NULL DEFAULT 'description'); CREATE TABLE industry_category_rules(industry_key TEXT PRIMARY KEY,industry_label TEXT,category_id INTEGER);").unwrap();
    for description in [
        "APPLE.COM/BILL CORK IRL",
        "Google YouTube",
        "Google One",
        "PARAMOUNT+ Berlin DEU",
        "NETFLIX.COM",
        "DISNEYPLUS",
        "Google Play",
    ] {
        connection
            .execute(
                "INSERT INTO transactions(description,amount_minor,category_id) VALUES(?1,-100,1)",
                [description],
            )
            .unwrap();
    }
    connection.execute_batch("INSERT INTO transactions(id,description,amount_minor,category_id,category_manual) VALUES(20,'Apple Store Zurich',-100,1,0),(21,'Disney Store',-100,1,0),(22,'Netflix',-100,1,1),(23,'Netflix',-100,4,0),(24,'Netflix',100,NULL,0),(25,'NETFLIX',-100,NULL,0),(26,'Sunrise Rechnung',-100,1,0),(27,'SWISSCOM',-100,NULL,0);").unwrap();
    apply_categories(&connection).unwrap();
    apply_categories(&connection).unwrap();
    let digital: usize = connection
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE category_id=2",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(digital, 8);
    for (id, expected) in [
        (20, 1),
        (21, 1),
        (22, 1),
        (23, 4),
        (24, 3),
        (26, 5),
        (27, 5),
    ] {
        assert_eq!(
            connection
                .query_row(
                    "SELECT category_id FROM transactions WHERE id=?1",
                    [id],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            expected
        );
    }
}

#[test]
fn checks_hash_account_and_transaction_multiplicity() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE import_runs(source_hash TEXT); INSERT INTO import_runs VALUES ('known'); CREATE TABLE transactions(account_id INTEGER, booking_date TEXT, value_date TEXT, amount_minor INTEGER, currency TEXT, description TEXT); INSERT INTO transactions VALUES(1, '2026-09-01', NULL, -100, 'CHF', 'Coffee');").unwrap();
    let mut request: SaveImportRequest = serde_json::from_value(serde_json::json!({
        "sourcePath": "unused", "accountName": "", "accountIds": {"CHF": 1},
        "statement": {"provider": "ubs", "format": "CSV", "accountName": "", "warnings": [], "openingBalanceMinor": null, "closingBalanceMinor": null,
        "transactions": [{"bookingDate": "2026-09-01", "valueDate": null, "description": "Coffee", "amountMinor": -100, "balanceMinor": null, "currency": "CHF", "confidence": 1.0, "sourceRow": 1}]}
    })).unwrap();
    assert!(
        duplicate_check(&connection, &request, "known")
            .unwrap()
            .exact_file
    );
    assert_eq!(
        duplicate_check(&connection, &request, "new")
            .unwrap()
            .matching_transactions,
        1
    );
    request
        .statement
        .transactions
        .push(request.statement.transactions[0].clone());
    let partial = duplicate_check(&connection, &request, "new").unwrap();
    assert_eq!(partial.matching_transactions, 1);
    assert_eq!(partial.total_transactions, 2);
    request.account_ids.insert("CHF".into(), 2);
    assert_eq!(
        duplicate_check(&connection, &request, "new")
            .unwrap()
            .matching_transactions,
        0
    );
    request.account_ids.insert("CHF".into(), 1);
    for row in &mut request.statement.transactions {
        row.description = "Other purchase".into();
    }
    assert_eq!(
        duplicate_check(&connection, &request, "new")
            .unwrap()
            .matching_transactions,
        0
    );
}
