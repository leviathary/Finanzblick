//! SQL-Projektion für Konsum und Einkommen; verändert keine Buchungen oder Salden.
use rusqlite::Connection;
pub(in crate::storage) fn prepare(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch(
        "CREATE TEMP VIEW IF NOT EXISTS reporting_transactions AS
        SELECT t.*,
          COALESCE(f.exclude_from_cashflow,0) AS exclude_from_cashflow,
          COALESCE(f.is_settlement,0) AS is_settlement,
          COALESCE(f.is_manually_overridden,0) AS is_manually_overridden,
          a.account_type='credit_card' AS is_card,
          CASE WHEN COALESCE(f.exclude_from_cashflow,0)=0
            AND (t.amount_minor<0 OR (a.account_type='credit_card'
              AND COALESCE(d.kind,CASE WHEN COALESCE(f.is_manually_overridden,0)=0 AND m.transaction_kind='card_refund' THEN 'REFUND' ELSE 'UNKNOWN' END)='REFUND'))
            THEN -t.amount_minor ELSE 0 END AS expense_minor,
          CASE WHEN COALESCE(f.exclude_from_cashflow,0)=0
            AND t.amount_minor>0 AND a.account_type<>'credit_card'
            THEN t.amount_minor ELSE 0 END AS income_minor
        FROM transactions t JOIN accounts a ON a.id=t.account_id
        LEFT JOIN transaction_reporting_flags f ON f.transaction_id=t.id
        LEFT JOIN card_credit_decisions d ON d.transaction_id=t.id
        LEFT JOIN transaction_metadata m ON m.transaction_id=t.id;",
    )
}
