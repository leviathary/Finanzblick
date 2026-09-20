//! Regressionstests für Import, Konsumrechnung und atomare Karteneinrichtung.
use super::*;
use crate::domain::banking::cards::CreditDecision;
use crate::storage::banking::cards::{apply_import, decide};
use rusqlite::Connection;
#[test]
fn ubs_csv_import_persists_payment_without_counting_it_as_income_or_refund() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("card.csv");
    std::fs::write(&source,"Kontonummer;Kartennummer;Einkaufsdatum;Buchungstext;Betrag;Originalwährung;Währung;Belastung;Gutschrift;Buchung\n1;123;01.08.2026;2002 LSV-ZAHLUNG;100;CHF;CHF;;100;02.08.2026\n1;123;01.08.2026;Shop;25;CHF;CHF;25;;02.08.2026\n").unwrap();
    let parsed = crate::importers::parse_statement(
        source.to_string_lossy().into(),
        Some("ubs".into()),
        None,
    )
    .unwrap();
    assert_eq!(parsed.account_type.as_deref(), Some("credit_card"));
    assert_eq!(parsed.transactions[0].transaction_kind, "card_settlement");
    let storage = Storage::test_storage(dir.path().join("test.sqlite3"));
    crate::storage::initialize_schema(&storage.connect().unwrap()).unwrap();
    let result = crate::storage::save_import_to(
        &storage,
        crate::storage::SaveImportRequest {
            account_ids: std::collections::BTreeMap::new(),
            source_path: source.to_string_lossy().into(),
            account_name: "Card".into(),
            statement: parsed,
            duplicate_resolutions: vec![],
        },
    )
    .unwrap();
    let db = storage.connect().unwrap();
    assert_eq!(result.inserted_transactions, 2);
    let analysis = crate::storage::analyze_transactions(&db, None, None, None, None).unwrap();
    assert_eq!(analysis.total_income_minor, 0);
    assert_eq!(analysis.total_spend_minor, 2500);
    assert_eq!(
        list_rows(&db, result.account_id)
            .unwrap()
            .iter()
            .filter(|r| r.kind == "SETTLEMENT")
            .count(),
        1
    );
}
fn fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    crate::storage::initialize_schema(&db).unwrap();
    db.execute_batch("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'ubs','Test','bank','2026-01-01');
          INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES
          (1,1,'Bank','checking','CHF','2026-01-01'),(2,1,'Card','credit_card','CHF','2026-01-01');
          INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json)
          VALUES(1,2,'sample','CSV','hash','2026-01-01',4,'[]');
          INSERT INTO transactions(id,account_id,import_id,booking_date,description,amount_minor,currency,confidence,source_row,category_id) VALUES
          (1,2,1,'2026-01-01','Shop',-10000,'CHF',1,1,(SELECT id FROM categories WHERE category_key='groceries')),
          (2,2,1,'2026-01-02','2002 LSV-ZAHLUNG · ref 1',10000,'CHF',1,2,(SELECT id FROM categories WHERE category_key='income')),
          (3,1,1,'2026-01-03','Card payment · ref 2',-10000,'CHF',1,3,(SELECT id FROM categories WHERE category_key='other')),
          (4,2,1,'2026-01-04','Unknown credit',2000,'CHF',1,4,(SELECT id FROM categories WHERE category_key='income'));
          INSERT INTO transaction_metadata(transaction_id,account_id,transaction_kind,fallback_fingerprint)
          VALUES(2,2,'card_settlement','two');").unwrap();
    db
}
fn request() -> SetupRequest {
    SetupRequest {
        card_id: 2,
        card_rule: Some(SetupRule {
            transaction_id: 2,
            prefix: "2002 LSV-ZAHLUNG".into(),
            expected_ids: vec![2],
        }),
        bank_rule: Some(SetupRule {
            transaction_id: 3,
            prefix: "Card payment".into(),
            expected_ids: vec![3],
        }),
        decisions: vec![],
        past: true,
        future: true,
    }
}
#[test]
fn unknown_credits_are_not_income_or_refunds_and_imports_keep_original_amounts() {
    let db = fixture();
    let list = list_rows(&db, 2).unwrap();
    assert_eq!(list.iter().find(|r| r.id == 2).unwrap().kind, "UNKNOWN");
    assert!(
        list.iter()
            .find(|r| r.id == 2)
            .unwrap()
            .suggested_settlement
    );
    let before = crate::storage::analyze_transactions(&db, None, None, None, Some(2)).unwrap();
    assert_eq!(before.total_income_minor, 0);
    assert_eq!(before.total_spend_minor, 10000);
    apply_import(&db, 1).unwrap();
    assert_eq!(
        list_rows(&db, 2)
            .unwrap()
            .iter()
            .find(|r| r.id == 2)
            .unwrap()
            .kind,
        "SETTLEMENT"
    );
    assert_eq!(
        db.query_row(
            "SELECT amount_minor FROM transactions WHERE id=2",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        10000
    );
    crate::storage::banking::reporting_flags::set_settlement(&db, 2, false).unwrap();
    apply_import(&db, 1).unwrap();
    assert_eq!(
        list_rows(&db, 2)
            .unwrap()
            .iter()
            .find(|r| r.id == 2)
            .unwrap()
            .kind,
        "UNKNOWN"
    );
    decide(
        &db,
        &CreditDecision {
            transaction_id: 4,
            kind: "REFUND".into(),
            category_key: Some("groceries".into()),
        },
        Some(2),
    )
    .unwrap();
    assert_eq!(
        crate::storage::analyze_transactions(&db, None, None, None, Some(2))
            .unwrap()
            .total_spend_minor,
        8000
    );
}
#[test]
fn setup_failure_rolls_back_rules_flags_and_refund_categories() {
    let mut db = fixture();
    let mut r = request();
    r.decisions = vec![
        CreditDecision {
            transaction_id: 4,
            kind: "REFUND".into(),
            category_key: Some("groceries".into()),
        },
        CreditDecision {
            transaction_id: 1,
            kind: "REFUND".into(),
            category_key: Some("groceries".into()),
        },
    ];
    assert!(apply_setup(&mut db, r).is_err());
    for table in [
        "settlement_rules",
        "transaction_reporting_flags",
        "card_credit_decisions",
    ] {
        let count: i64 = db
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "partial setup changes in {table}");
    }
    let category: String = db.query_row("SELECT c.category_key FROM transactions t JOIN categories c ON c.id=t.category_id WHERE t.id=4", [], |row| row.get(0)).unwrap();
    assert_eq!(category, "income");
    let amounts: i64 = db
        .query_row("SELECT SUM(amount_minor) FROM transactions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(amounts, -8000);
}

#[test]
fn setup_commits_both_sides_atomically_and_future_only_does_not_reclassify_history() {
    let mut db = fixture();
    let mut r = request();
    r.bank_rule.as_mut().unwrap().expected_ids = vec![999];
    assert!(apply_setup(&mut db, r).is_err());
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM settlement_rules", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM transaction_reporting_flags",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let mut r = request();
    r.past = false;
    apply_setup(&mut db, r).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM transaction_reporting_flags",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM settlement_rules", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    apply_setup(&mut db, request()).unwrap();
    assert_eq!(db.query_row("SELECT COUNT(*) FROM transaction_reporting_flags WHERE is_settlement=1 AND is_manually_overridden=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    let mut r = request();
    r.card_id = 1;
    assert!(apply_setup(&mut db, r).is_err());
}
