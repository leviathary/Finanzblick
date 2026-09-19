//! Persistiert manuelle Umbuchungs- und Ausgleichsmarkierungen atomar.
//! Salden und reale Bankbewegungen bleiben von den Markierungen unberührt.

pub(crate) use crate::domain::banking::transfers::TransferType;
use rusqlite::{params, Connection};

// The two existing flags encode all three states without a schema migration.
pub(in crate::storage) fn set_transfers(
    db: &mut Connection,
    ids: &[i64],
    kind: TransferType,
) -> Result<(), String> {
    if ids.is_empty() || ids.len() > 5000 {
        return Err("Bitte zwischen 1 und 5000 Buchungen auswählen.".into());
    }
    let tx = db.transaction().map_err(crate::storage::db_error)?;
    let excluded = !matches!(kind, TransferType::None);
    let settlement = matches!(kind, TransferType::CreditCardSettlement);
    for id in ids.iter().collect::<std::collections::BTreeSet<_>>() {
        let changed = tx
            .execute(
                "INSERT INTO transaction_reporting_flags
             (transaction_id,exclude_from_cashflow,is_settlement,is_manually_overridden)
             SELECT id,?2,?3,1 FROM transactions WHERE id=?1
             ON CONFLICT(transaction_id) DO UPDATE SET
             exclude_from_cashflow=excluded.exclude_from_cashflow,
             is_settlement=excluded.is_settlement,is_manually_overridden=1",
                params![id, excluded, settlement],
            )
            .map_err(crate::storage::db_error)?;
        if changed == 0 {
            return Err("Buchung nicht gefunden.".into());
        }
    }
    tx.commit().map_err(crate::storage::db_error)
}

pub(crate) fn set_settlement(db: &Connection, id: i64, settlement: bool) -> Result<(), String> {
    let changed = db
        .execute(
            "INSERT INTO transaction_reporting_flags
         (transaction_id,exclude_from_cashflow,is_settlement,is_manually_overridden)
         SELECT id,?2,?2,1 FROM transactions WHERE id=?1
         ON CONFLICT(transaction_id) DO UPDATE SET
         exclude_from_cashflow=excluded.exclude_from_cashflow,
         is_settlement=excluded.is_settlement,is_manually_overridden=1",
            params![id, settlement],
        )
        .map_err(crate::storage::db_error)?;
    if changed == 0 {
        return Err("Buchung nicht gefunden.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::reporting::consumption::prepare;

    #[test]
    fn batch_transfers_restore_atomically_and_preserve_balances() {
        let mut db = fixture();
        assert!(set_transfers(&mut db, &[1, 999], TransferType::InternalTransfer).is_err());
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transaction_reporting_flags",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        set_transfers(&mut db, &[3, 4, 3], TransferType::InternalTransfer).unwrap();
        let listed =
            crate::storage::query_transaction_transfers(&db, None, None, None, "".into()).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(matches!(
            listed[0].transfer_type,
            TransferType::InternalTransfer
        ));
        assert_eq!(
            crate::storage::query_transaction_transfers(
                &db,
                Some(1),
                Some("2026-02-01".into()),
                Some("2026-02-01".into()),
                "PAYMENT".into()
            )
            .unwrap()
            .len(),
            1
        );
        assert!(crate::storage::query_transaction_transfers(
            &db,
            None,
            Some("2027-01-01".into()),
            None,
            "".into()
        )
        .unwrap()
        .is_empty());
        prepare(&db).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT SUM(expense_minor) FROM reporting_transactions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            8000
        );
        assert_eq!(db.query_row("SELECT COUNT(*) FROM transaction_reporting_flags WHERE exclude_from_cashflow=1 AND is_settlement=0 AND is_manually_overridden=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
        set_transfers(&mut db, &[3], TransferType::CreditCardSettlement).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT is_settlement FROM transaction_reporting_flags WHERE transaction_id=3",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap(),
            true
        );
        set_transfers(&mut db, &[3, 4], TransferType::None).unwrap();
        assert!(
            crate::storage::query_transaction_transfers(&db, None, None, None, "".into())
                .unwrap()
                .is_empty()
        );
        crate::storage::initialize_schema(&db).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM transaction_reporting_flags WHERE exclude_from_cashflow=0 AND is_settlement=0 AND is_manually_overridden=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
        assert_eq!(
            db.query_row(
                "SELECT amount_minor FROM transactions WHERE id=3",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            -8000
        );
        assert!(set_transfers(&mut db, &[], TransferType::None).is_err());
        assert!(serde_json::from_str::<TransferType>("\"UNKNOWN\"").is_err());
    }

    #[test]
    fn analysis_multiselect_combines_accounts_and_intersects_providers() {
        use crate::storage::banking::transactions::analyze_transactions_filtered;
        let db = fixture();
        set_settlement(&db, 3, true).unwrap();
        set_settlement(&db, 4, true).unwrap();
        let analyze = |providers, accounts| analyze_transactions_filtered(&db, None, None, None, None, providers, accounts).unwrap();
        let all = analyze(vec![], vec![]);
        let combined = analyze(vec!["bank".into(), "other".into()], vec![1, 2, 2]);
        assert_eq!(combined.total_spend_minor, all.total_spend_minor);
        assert_eq!(combined.total_income_minor, all.total_income_minor);
        assert_eq!(combined.total_spend_minor, 8000);
        let card = analyze(vec!["bank".into()], vec![2]);
        assert_eq!(card.total_income_minor, 0);
        assert_eq!(card.total_spend_minor, 8000);
        let bank = analyze(vec![], vec![1]);
        assert_eq!(bank.total_income_minor, 50000);
        assert_eq!(bank.total_spend_minor, 0);
        let mismatch = analyze(vec!["unknown".into()], vec![1,2]);
        assert_eq!(mismatch.transaction_count, 0);
        assert_eq!(mismatch.income_count, 0);
        let unknown = analyze(vec![], vec![999]);
        assert_eq!(unknown.total_income_minor, 0);
    }

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        crate::storage::initialize_schema(&db).unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;
          INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'bank','Bank','bank','2026-01-01');
          INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES
          (1,1,'Bank','checking','CHF','2026-01-01'),(2,1,'Card','credit_card','CHF','2026-01-01');
          INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json)
          VALUES(1,1,'sample','CSV','hash','2026-01-01',5,'[]');
          INSERT INTO transactions(id,account_id,import_id,booking_date,description,amount_minor,currency,confidence,source_row,category_id) VALUES
          (1,2,1,'2026-01-02','Groceries',-10000,'CHF',1,1,(SELECT id FROM categories WHERE category_key='groceries')),
          (2,2,1,'2026-01-03','Refund',2000,'CHF',1,2,(SELECT id FROM categories WHERE category_key='groceries')),
          (3,1,1,'2026-02-01','Rechnung / Payment / Paiement',-8000,'CHF',1,3,(SELECT id FROM categories WHERE category_key='other')),
          (4,2,1,'2026-02-03','Ausgleich / Settlement / Solde',8000,'CHF',1,4,(SELECT id FROM categories WHERE category_key='other')),
          (5,1,1,'2026-01-01','Salary',50000,'CHF',1,5,(SELECT id FROM categories WHERE category_key='income'));
          INSERT INTO card_credit_decisions(transaction_id,kind) VALUES(2,'REFUND');").unwrap();
        db
    }

    #[test]
    fn explicit_flags_are_language_neutral_and_refunds_reduce_categories() {
        let db = fixture();
        set_settlement(&db, 3, true).unwrap();
        set_settlement(&db, 4, true).unwrap();
        for description in [
            "Kreditkartenabrechnung",
            "Payment received",
            "Paiement reçu",
            "無関係",
        ] {
            db.execute("UPDATE transactions SET description=?1", [description])
                .unwrap();
            let result = crate::storage::analyze_transactions(&db, None, None, None, None).unwrap();
            assert_eq!(result.total_spend_minor, 8000);
            assert_eq!(result.total_income_minor, 50000);
            assert_eq!(result.categories.len(), 1);
            assert_eq!(result.categories[0].key, "groceries");
            assert_eq!(result.categories[0].amount_minor, 8000);
            assert_eq!(result.months[0].amount_minor, 8000);
            assert_eq!(
                result
                    .transactions
                    .iter()
                    .map(|r| r.expense_minor)
                    .sum::<i64>(),
                8000
            );
        }
        let feb =
            crate::storage::analyze_transactions(&db, Some("2026-02-01".into()), None, None, None)
                .unwrap();
        assert_eq!(feb.total_spend_minor, 0);
        assert_eq!(feb.total_income_minor, 0);
        let card = crate::storage::analyze_transactions(&db, None, None, None, Some(2)).unwrap();
        assert_eq!(card.total_spend_minor, 8000);
        assert_eq!(card.total_income_minor, 0);
        assert_eq!(
            db.query_row(
                "SELECT amount_minor FROM transactions WHERE id=3",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            -8000
        );
    }

    #[test]
    fn decisions_survive_schema_initialization_and_can_be_reversed() {
        let db = fixture();
        set_settlement(&db, 3, true).unwrap();
        set_settlement(&db, 4, true).unwrap();
        crate::storage::initialize_schema(&db).unwrap();
        set_settlement(&db, 3, false).unwrap();
        crate::storage::initialize_schema(&db).unwrap();
        assert_eq!(db.query_row("SELECT exclude_from_cashflow,is_settlement,is_manually_overridden FROM transaction_reporting_flags WHERE transaction_id=3",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?,r.get::<_,bool>(2)?))).unwrap(),(false,false,true));
        assert_eq!(
            crate::storage::analyze_transactions(&db, None, None, None, None)
                .unwrap()
                .total_spend_minor,
            16000
        );
        assert!(set_settlement(&db, 999, true).is_err());
        assert!(db.execute("UPDATE transaction_reporting_flags SET is_settlement=1,exclude_from_cashflow=0 WHERE transaction_id=3",[]).is_err());
        db.execute("DELETE FROM transactions WHERE id=3", [])
            .unwrap();
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM transaction_reporting_flags WHERE transaction_id=3",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn text_does_not_implicitly_neutralize_and_refunds_can_exceed_purchases() {
        let db = fixture();
        prepare(&db).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT expense_minor FROM reporting_transactions WHERE id=3",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            8000
        );
        set_settlement(&db, 3, true).unwrap();
        set_settlement(&db, 4, true).unwrap();
        db.execute("UPDATE transactions SET amount_minor=12000 WHERE id=2", [])
            .unwrap();
        assert_eq!(
            crate::storage::analyze_transactions(&db, None, None, None, None)
                .unwrap()
                .total_spend_minor,
            -2000
        );
    }
}
