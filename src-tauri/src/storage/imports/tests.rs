//! Prüft Import-Deduplizierung, überlappende Belege und zugehörige Persistenzregeln.

use super::*;
use crate::storage::*;

#[test]
fn suspicious_duplicates_block_saving_until_explicitly_kept_or_skipped() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::test_storage(directory.path().join("suspects.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();
    let transaction = |reference: &str| crate::importers::ParsedTransaction {
        booking_date: "2026-08-24".into(),
        value_date: Some("2026-08-24".into()),
        description: "Steuerverwaltung des Kantons Bern".into(),
        amount_minor: -59_000,
        currency: "CHF".into(),
        confidence: 1.0,
        source_row: 1,
        reference_namespace: Some("camt-entry".into()),
        external_reference: Some(reference.into()),
        ..Default::default()
    };
    let statement = |reference: &str| crate::importers::ParsedStatement {
        provider: "ubs".into(),
        format: "CAMT053".into(),
        account_name: "Privatkonto".into(),
        transactions: vec![transaction(reference)],
        ..Default::default()
    };
    let first_source = directory.path().join("first.xml");
    fs::write(&first_source, b"first").unwrap();
    save_import_to(
        &storage,
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: first_source.to_string_lossy().into(),
            account_name: "Privatkonto".into(),
            statement: statement("BANK-1"),
            duplicate_resolutions: vec![],
        },
    )
    .unwrap();
    let account_id = storage
        .connect()
        .unwrap()
        .query_row("SELECT id FROM accounts LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    let request = |name: &str, reference: &str| {
        let source = directory.path().join(name);
        fs::write(&source, name.as_bytes()).unwrap();
        SaveImportRequest {
            account_ids: BTreeMap::from([("CHF".into(), account_id)]),
            source_path: source.to_string_lossy().into(),
            account_name: "Privatkonto".into(),
            statement: statement(reference),
            duplicate_resolutions: vec![],
        }
    };

    let same_comment_distinct_reference = request("distinct-stored.xml", "BANK-2");
    let check = duplicate_check(
        &storage.connect().unwrap(),
        &same_comment_distinct_reference,
        "distinct-stored",
    )
    .unwrap();
    assert_eq!(check.suspected_transactions.len(), 1);

    let mut skipped = request("skip.xml", "BANK-2");
    skipped.statement.transactions[0].booking_date = "2026-08-25".into();
    skipped.statement.transactions[0].value_date = Some("2026-08-25".into());
    skipped.statement.transactions[0].reference_namespace = None;
    skipped.statement.transactions[0].external_reference = None;
    let check = duplicate_check(&storage.connect().unwrap(), &skipped, "skip").unwrap();
    assert_eq!(check.suspected_transactions.len(), 1);
    assert_eq!(check.suspected_transactions[0].match_source, "stored");

    let mut missing_comment = request("missing-comment.xml", "NO-COMMENT");
    missing_comment.statement.transactions[0].description = String::new();
    let check = duplicate_check(
        &storage.connect().unwrap(),
        &missing_comment,
        "missing-comment",
    )
    .unwrap();
    assert_eq!(check.suspected_transactions.len(), 1);

    let mut sibling_payments = request("siblings.xml", "SIBLING-1");
    sibling_payments.statement.transactions = vec![
        crate::importers::ParsedTransaction {
            booking_date: "2026-01-03".into(),
            value_date: Some("2026-01-03".into()),
            description: "Nils Wüthrich Dauerauftrag Referenz: NONREF//9992503LK6809612".into(),
            amount_minor: -3_000,
            currency: "CHF".into(),
            confidence: 1.0,
            source_row: 12,
            reference_namespace: Some("ubs-csv".into()),
            external_reference: Some("9992503LK6809612".into()),
            ..Default::default()
        },
        crate::importers::ParsedTransaction {
            booking_date: "2026-01-03".into(),
            value_date: Some("2026-01-03".into()),
            description: "Jasper Wüthrich Dauerauftrag Referenz: NONREF//9992503LK6809618".into(),
            amount_minor: -3_000,
            currency: "CHF".into(),
            confidence: 1.0,
            source_row: 13,
            reference_namespace: Some("ubs-csv".into()),
            external_reference: Some("9992503LK6809618".into()),
            ..Default::default()
        },
    ];
    assert!(
        duplicate_check(&storage.connect().unwrap(), &sibling_payments, "siblings")
            .unwrap()
            .suspected_transactions
            .is_empty()
    );

    let mut distinct_references = request("sbb.xml", "SBB-1");
    distinct_references.statement.transactions = vec![
        crate::importers::ParsedTransaction {
            booking_date: "2024-02-21".into(),
            description: "SBB Contact Center Swiss Pass E-BILL PayNet-Auftrag · Referenz: NONREF//3892052DJ4905339".into(),
            amount_minor: -7_500,
            currency: "CHF".into(),
            ..Default::default()
        },
        crate::importers::ParsedTransaction {
            booking_date: "2024-02-21".into(),
            description: "SBB Contact Center Swiss Pass E-BILL PayNet-Auftrag · Referenz: NONREF//3792052DJ4905335".into(),
            amount_minor: -7_500,
            currency: "CHF".into(),
            ..Default::default()
        },
    ];
    assert!(duplicate_check(
        &storage.connect().unwrap(),
        &distinct_references,
        "distinct-references",
    )
    .unwrap()
    .suspected_transactions
    .is_empty());
    let mut matching_text_references = distinct_references.clone();
    matching_text_references.statement.transactions[1].description =
        matching_text_references.statement.transactions[0]
            .description
            .clone();
    assert_eq!(
        duplicate_check(
            &storage.connect().unwrap(),
            &matching_text_references,
            "matching-text-references",
        )
        .unwrap()
        .suspected_transactions
        .len(),
        1
    );
    assert!(save_import_to(&storage, skipped.clone())
        .unwrap_err()
        .contains("vollständig geprüft"));
    skipped
        .duplicate_resolutions
        .push(crate::storage::imports::models::DuplicateResolution {
            transaction_index: 0,
            action: "skip".into(),
        });
    assert_eq!(
        save_import_to(&storage, skipped)
            .unwrap()
            .inserted_transactions,
        0
    );

    let mut kept = request("keep.xml", "BANK-3");
    kept.statement.transactions[0].booking_date = "2026-08-25".into();
    kept.statement.transactions[0].value_date = Some("2026-08-25".into());
    kept.statement.transactions[0].reference_namespace = None;
    kept.statement.transactions[0].external_reference = None;
    kept.duplicate_resolutions
        .push(crate::storage::imports::models::DuplicateResolution {
            transaction_index: 0,
            action: "keep".into(),
        });
    assert_eq!(
        save_import_to(&storage, kept)
            .unwrap()
            .inserted_transactions,
        1
    );
    assert_eq!(
        storage
            .connect()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );

    let mut same_day_same_amount = request("same-amount.xml", "BANK-4");
    same_day_same_amount.statement.transactions[0].description = "Andere Gegenpartei".into();
    let check = duplicate_check(
        &storage.connect().unwrap(),
        &same_day_same_amount,
        "same-amount",
    )
    .unwrap();
    assert!(check.suspected_transactions.is_empty());

    let mut stale_decision = request("stale.xml", "BANK-5");
    stale_decision.statement.transactions[0].amount_minor = -58_000;
    stale_decision.duplicate_resolutions.push(
        crate::storage::imports::models::DuplicateResolution {
            transaction_index: 0,
            action: "skip".into(),
        },
    );
    assert!(save_import_to(&storage, stale_decision)
        .unwrap_err()
        .contains("ungültig"));
}

#[test]
fn ignored_duplicate_stays_deduplicated_and_can_be_restored_from_its_import() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::test_storage(directory.path().join("ignored-duplicate.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();
    let source = directory.path().join("statement.xml");
    fs::write(
        &source,
        include_str!("../../importers/fixtures/camt053.xml"),
    )
    .unwrap();
    let parsed = crate::importers::parse_statement(
        source.to_string_lossy().into(),
        Some("zkb".into()),
        None,
    )
    .unwrap();
    let request = SaveImportRequest {
        account_ids: BTreeMap::new(),
        source_path: source.to_string_lossy().into(),
        account_name: "Testkonto".into(),
        statement: parsed.clone(),
        duplicate_resolutions: vec![],
    };
    let saved = save_import_to(&storage, request).unwrap();
    let before = crate::storage::banking::transactions::transaction_analysis(
        &storage,
        None,
        None,
        None,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let transaction_id = before.transactions[0].id;

    crate::storage::banking::duplicates::ignore(&storage, transaction_id).unwrap();
    let analysis = crate::storage::banking::transactions::transaction_analysis(
        &storage,
        None,
        None,
        None,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    assert_eq!(analysis.transactions.len() + 1, before.transactions.len());
    assert_eq!(
        storage
            .connect()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    let imports = imports_from(&storage).unwrap();
    assert_eq!(imports[0].ignored_duplicate_count, 1);

    let repeated = save_import_to(
        &storage,
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: source.to_string_lossy().into(),
            account_name: "Testkonto".into(),
            statement: parsed,
            duplicate_resolutions: vec![],
        },
    )
    .unwrap();
    assert!(repeated.duplicate);
    assert_eq!(
        restore_import_duplicates(&storage, saved.import_id).unwrap(),
        1
    );
    assert_eq!(
        storage
            .connect()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM ignored_duplicate_transactions",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let restored = crate::storage::banking::transactions::transaction_analysis(
        &storage,
        None,
        None,
        None,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    assert_eq!(restored.transactions.len(), before.transactions.len());
}

#[test]
fn camt_import_roundtrip_distinguishes_references_and_skips_overlap() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::test_storage(directory.path().join("camt.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();
    let xml = include_str!("../../importers/fixtures/camt053.xml");
    let save = |name: &str, content: &str| {
        let source = directory.path().join(name);
        fs::write(&source, content).unwrap();
        let parsed = crate::importers::parse_statement(
            source.to_string_lossy().into(),
            Some("zkb".into()),
            None,
        )
        .unwrap();
        let mut request = SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: source.to_string_lossy().into(),
            account_name: "Testkonto".into(),
            statement: parsed,
            duplicate_resolutions: vec![],
        };
        if let Ok(account_id) = storage.connect().unwrap().query_row(
            "SELECT id FROM accounts WHERE currency='CHF' ORDER BY id LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            request.account_ids.insert("CHF".into(), account_id);
        }
        let check = duplicate_check(&storage.connect().unwrap(), &request, "test-check").unwrap();
        request.duplicate_resolutions = check
            .suspected_transactions
            .into_iter()
            .map(
                |item| crate::storage::imports::models::DuplicateResolution {
                    transaction_index: item.transaction_index,
                    action: "keep".into(),
                },
            )
            .collect();
        save_import_to(&storage, request).unwrap()
    };
    let first = save("one.xml", xml);
    assert_eq!(first.inserted_transactions, 2);
    let original_id: i64 = storage
        .connect()
        .unwrap()
        .query_row("SELECT id FROM transactions ORDER BY id LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    crate::storage::banking::reporting_flags::set_settlement(
        &storage.connect().unwrap(),
        original_id,
        true,
    )
    .unwrap();
    let overlap = save("two.xml", &xml.replace("TEST-MESSAGE", "ANOTHER-MESSAGE"));
    assert_eq!(overlap.inserted_transactions, 0);
    let reopened = storage.connect().unwrap();
    assert_eq!(reopened.query_row(
        "SELECT is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=?1",
        [original_id], |r| Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?))).unwrap(), (true,true));
    crate::storage::banking::reporting_flags::set_settlement(&reopened, original_id, false)
        .unwrap();
    drop(reopened);
    assert!(save("repeat.xml", xml).duplicate);
    assert_eq!(
        storage
            .connect()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM transactions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(storage.connect().unwrap().query_row(
        "SELECT is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=?1",
        [original_id], |r| Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?))).unwrap(), (false,true));
    // Same amount, dates and narrative but distinct bank IDs are real postings.
    let other = save(
        "three.xml",
        &xml.replace("BANK-1", "BANK-NEW")
            .replace("ENTRY-2", "ENTRY-NEW"),
    );
    assert_eq!(other.inserted_transactions, 2);
    let conflicting_source = directory.path().join("conflict.xml");
    fs::write(
        &conflicting_source,
        xml.replace("150.00", "151.00")
            .replace("1130.00", "1131.00"),
    )
    .unwrap();
    let conflicting = crate::importers::parse_statement(
        conflicting_source.to_string_lossy().into(),
        Some("zkb".into()),
        None,
    )
    .unwrap();
    let error = save_import_to(
        &storage,
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: conflicting_source.to_string_lossy().into(),
            account_name: "Testkonto".into(),
            statement: conflicting,
            duplicate_resolutions: vec![],
        },
    )
    .unwrap_err();
    assert!(error.contains("Bankreferenz"));
    let db = storage.connect().unwrap();
    assert_eq!(count(&db, "transaction_metadata").unwrap(), 4);
    assert_eq!(
        db.query_row(
            "SELECT document_date FROM import_document_metadata WHERE import_id=?1",
            [first.import_id],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "2026-09-02"
    );
    assert_eq!(db.query_row("SELECT amount_minor FROM balance_snapshots WHERE import_id=?1 AND balance_date='2026-08-31'", [first.import_id], |row| row.get::<_, i64>(0)).unwrap(), 100000);
    drop(db);
    delete_imports_from(
        &storage,
        vec![first.import_id, overlap.import_id, other.import_id],
    )
    .unwrap();
    let db = storage.connect().unwrap();
    assert_eq!(count(&db, "transaction_metadata").unwrap(), 0);
    assert_eq!(count(&db, "import_document_metadata").unwrap(), 0);
}

#[test]
fn balance_backed_import_skips_exact_duplicate_but_keeps_repeated_payments() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::test_storage(directory.path().join("balanced-dedup.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();
    let source = directory.path().join("ubs-statement.pdf");
    fs::write(&source, b"first statement").unwrap();
    let tax = crate::importers::ParsedTransaction {
        booking_date: "2026-08-24".into(),
        value_date: Some("2026-08-24".into()),
        description: "Steuerverwaltung des Kantons Bern\ne-banking-Verguetungsauftrag".into(),
        amount_minor: -59_000,
        balance_minor: Some(3_534_091),
        currency: "CHF".into(),
        confidence: 0.99,
        source_row: 1,
        ..crate::importers::ParsedTransaction::default()
    };
    let first_sbb = crate::importers::ParsedTransaction {
        booking_date: "2026-08-24".into(),
        value_date: Some("2026-08-24".into()),
        description: "SBB Contact Center Swiss Pass\nE-BILL, PayNet-Auftrag".into(),
        amount_minor: -8_500,
        balance_minor: Some(3_615_551),
        currency: "CHF".into(),
        confidence: 0.99,
        source_row: 2,
        ..crate::importers::ParsedTransaction::default()
    };
    let second_sbb = crate::importers::ParsedTransaction {
        balance_minor: Some(3_624_051),
        source_row: 3,
        ..first_sbb.clone()
    };
    let result = save_import_to(
        &storage,
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: source.to_string_lossy().into(),
            account_name: "Privatkonto".into(),
            statement: crate::importers::ParsedStatement {
                provider: "ubs".into(),
                format: "PDF".into(),
                account_name: "Privatkonto".into(),
                transactions: vec![tax.clone(), tax, first_sbb, second_sbb],
                opening_balance_minor: Some(3_683_051),
                closing_balance_minor: Some(3_534_091),
                warnings: Vec::new(),
                ..crate::importers::ParsedStatement::default()
            },
            duplicate_resolutions: vec![],
        },
    )
    .unwrap();

    assert_eq!(result.inserted_transactions, 3);
    let connection = storage.connect().unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        3
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE description LIKE 'Steuerverwaltung%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(DISTINCT balance_minor) FROM transactions WHERE description LIKE 'SBB Contact%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        2
    );
}

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
fn settlement_text_does_not_override_categories() {
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
    assert_eq!(rows, vec!["other", "other", "other", "housing"]);
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
    connection.execute_batch("CREATE TABLE import_runs(source_hash TEXT); INSERT INTO import_runs VALUES ('known'); CREATE TABLE transactions(account_id INTEGER, booking_date TEXT, value_date TEXT, amount_minor INTEGER, currency TEXT, description TEXT); INSERT INTO transactions VALUES(1, '2026-09-01', NULL, -100, 'CHF', 'Coffee'); CREATE TABLE transaction_metadata(transaction_id INTEGER PRIMARY KEY,account_id INTEGER,reference_namespace TEXT,external_reference TEXT,fallback_fingerprint TEXT);").unwrap();
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

#[test]
fn finalized_card_export_atomically_replaces_matching_provisional_purchase() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::test_storage(directory.path().join("card-overlay.sqlite3"));
    initialize_schema(&storage.connect().unwrap()).unwrap();
    let source_open = directory.path().join("card-open.csv");
    let source_final = directory.path().join("card-final.csv");
    fs::write(&source_open, b"open").unwrap();
    fs::write(&source_final, b"final").unwrap();
    let statement = |provisional: bool| crate::importers::ParsedStatement {
        provider: "ubs".into(),
        format: "CSV".into(),
        account_name: "Mastercard".into(),
        account_type: Some("credit_card".into()),
        transactions: vec![crate::importers::ParsedTransaction {
            booking_date: if provisional {
                "2026-09-19".into()
            } else {
                "2026-09-22".into()
            },
            description: "Karteneinkauf · Einkauf: 19.09.2026".into(),
            amount_minor: if provisional { -24_940 } else { -25_105 },
            currency: "CHF".into(),
            confidence: 0.95,
            source_row: 3,
            transaction_kind: if provisional {
                "provisional_card_transaction".into()
            } else {
                "cash_transaction".into()
            },
            reference_namespace: Some("credit-card-purchase".into()),
            external_reference: Some("stable-purchase-id".into()),
            ..Default::default()
        }],
        warnings: vec![],
        ..Default::default()
    };
    let first = save_import_to(
        &storage,
        SaveImportRequest {
            account_ids: BTreeMap::new(),
            source_path: source_open.to_string_lossy().into(),
            account_name: "Mastercard".into(),
            statement: statement(true),
            duplicate_resolutions: vec![],
        },
    )
    .unwrap();
    assert_eq!(first.inserted_transactions, 1);
    assert_eq!(first.updated_transactions, 0);

    let request = SaveImportRequest {
        account_ids: BTreeMap::from([("CHF".into(), first.account_id)]),
        source_path: source_final.to_string_lossy().into(),
        account_name: "Mastercard".into(),
        statement: statement(false),
        duplicate_resolutions: vec![],
    };
    let check = duplicate_check(&storage.connect().unwrap(), &request, "final").unwrap();
    assert_eq!(check.matching_transactions, 1);
    assert_eq!(check.updatable_transactions, 1);
    assert!(check.suspected_transactions.is_empty());

    let second = save_import_to(&storage, request).unwrap();
    assert_eq!(second.inserted_transactions, 0);
    assert_eq!(second.updated_transactions, 1);
    assert!(!second.duplicate);
    let connection = storage.connect().unwrap();
    let stored = connection
        .query_row(
            "SELECT t.booking_date,t.amount_minor,m.transaction_kind
             FROM transactions t JOIN transaction_metadata m ON m.transaction_id=t.id",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        stored,
        ("2026-09-22".into(), -25_105, "cash_transaction".into())
    );
    assert_eq!(count(&connection, "transactions").unwrap(), 1);
}
