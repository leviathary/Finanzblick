use super::*;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationRow {
    source: String,
    account: String,
    currency: String,
    first_date: String,
    last_date: String,
    debits: i64,
    credits: i64,
    payments: i64,
    unmatched_credits: i64,
    closing_balance: Option<i64>,
    payment_dates: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmatchedPayment {
    date: String,
    account: String,
    currency: String,
    amount: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reconciliation {
    statements: Vec<ReconciliationRow>,
    unmatched_payments: Vec<UnmatchedPayment>,
}
#[tauri::command]
pub fn card_reconciliation(storage: State<'_, Storage>) -> Result<Reconciliation, String> {
    let db = storage.connect().map_err(db_error)?;
    card_settlements::prepare(&db).map_err(db_error)?;
    let mut q=db.prepare("SELECT ir.source_name,a.name,t.currency,MIN(t.booking_date),MAX(t.booking_date),
 SUM(CASE WHEN t.amount_minor<0 THEN -t.amount_minor ELSE 0 END),
 SUM(CASE WHEN t.amount_minor>0 THEN t.amount_minor ELSE 0 END),
 SUM(CASE WHEN m.credit_id IS NOT NULL THEN t.amount_minor ELSE 0 END),
 SUM(CASE WHEN m.credit_id IS NULL AND t.amount_minor>0 AND
 (lower(t.description) LIKE '%lsv%zahlung%' OR lower(t.description) LIKE '%zahlung erhalten%' OR lower(t.description) LIKE '%payment received%' OR lower(t.description) LIKE '%rechnungsausgleich%') THEN t.amount_minor ELSE 0 END),
  (SELECT b.amount_minor FROM balance_snapshots b WHERE b.import_id=ir.id AND b.account_id=a.id AND b.currency=t.currency ORDER BY b.balance_date DESC,b.id DESC LIMIT 1),
 GROUP_CONCAT(DISTINCT d.booking_date)
 FROM transactions t JOIN accounts a ON a.id=t.account_id JOIN import_runs ir ON ir.id=t.import_id
 LEFT JOIN matched_card_settlements m ON m.credit_id=t.id LEFT JOIN transactions d ON d.id=m.debit_id
 WHERE a.account_type='credit_card' AND a.is_active=1
 GROUP BY ir.id,a.id,t.currency ORDER BY MAX(t.booking_date) DESC,ir.id DESC").map_err(db_error)?;
    let statements = q
        .query_map([], |r| {
            Ok(ReconciliationRow {
                source: r.get(0)?,
                account: r.get(1)?,
                currency: r.get(2)?,
                first_date: r.get(3)?,
                last_date: r.get(4)?,
                debits: r.get(5)?,
                credits: r.get(6)?,
                payments: r.get(7)?,
                unmatched_credits: r.get(8)?,
                closing_balance: r.get(9)?,
                payment_dates: r.get(10)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut q=db.prepare("SELECT t.booking_date,a.name,t.currency,-t.amount_minor FROM transactions t JOIN accounts a ON a.id=t.account_id
 WHERE a.is_active=1 AND a.account_type<>'credit_card' AND t.amount_minor<0
 AND (lower(t.description) LIKE '%kreditkart%' OR lower(t.description) LIKE '%credit card%' OR lower(t.description) LIKE '%vis1w%' OR lower(t.description) LIKE '%card center%')
 AND t.id NOT IN (SELECT debit_id FROM matched_card_settlements) ORDER BY t.booking_date DESC").map_err(db_error)?;
    let unmatched_payments = q
        .query_map([], |r| {
            Ok(UnmatchedPayment {
                date: r.get(0)?,
                account: r.get(1)?,
                currency: r.get(2)?,
                amount: r.get(3)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(Reconciliation {
        statements,
        unmatched_payments,
    })
}
