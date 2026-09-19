//! Speichert Kartenentscheidungen und liest Kartenbuchungen ohne UI- oder Provider-Abhängigkeiten.
use crate::domain::banking::cards::{classify, CreditDecision};
use crate::storage::{consumption, db_error};
use rusqlite::{params, Connection};
pub struct CardRecord {
    pub id: i64,
    pub booking_date: String,
    pub description: String,
    pub amount_minor: i64,
    pub currency: String,
    pub kind: String,
    pub category_key: String,
    pub provider: String,
    pub is_card: bool,
    pub manually_overridden: bool,
}

pub(crate) fn apply_import(db: &Connection, import_id: i64) -> Result<(), String> {
    db.execute("INSERT INTO transaction_reporting_flags(transaction_id,exclude_from_cashflow,is_settlement,is_manually_overridden)
      SELECT t.id,1,1,0 FROM transactions t JOIN accounts a ON a.id=t.account_id
      JOIN transaction_metadata m ON m.transaction_id=t.id
      WHERE t.import_id=?1 AND a.account_type='credit_card' AND t.amount_minor>0 AND m.transaction_kind='card_settlement'
      ON CONFLICT(transaction_id) DO UPDATE SET exclude_from_cashflow=1,is_settlement=1
      WHERE transaction_reporting_flags.is_manually_overridden=0",[import_id]).map_err(db_error)?;
    Ok(())
}

pub(crate) fn list_rows(db: &Connection, account_id: i64) -> Result<Vec<CardRecord>, String> {
    consumption::prepare(db).map_err(db_error)?;
    let mut q=db.prepare("SELECT t.id,t.booking_date,t.description,t.amount_minor,t.currency,t.is_card,
      t.exclude_from_cashflow,t.is_settlement,t.expense_minor,COALESCE(c.category_key,'other'),i.provider_key,t.is_manually_overridden
      FROM reporting_transactions t JOIN accounts a ON a.id=t.account_id JOIN institutions i ON i.id=a.institution_id
      LEFT JOIN categories c ON c.id=t.category_id WHERE a.id=?1 AND a.is_active=1 ORDER BY t.booking_date DESC,t.id DESC").map_err(db_error)?;
    let rows = q
        .query_map([account_id], |r| {
            let amount: i64 = r.get(3)?;
            let text: String = r.get(2)?;
            let provider: String = r.get(10)?;
            let card: bool = r.get(5)?;
            let neutral: bool = r.get(6)?;
            let settlement: bool = r.get(7)?;
            let expense: i64 = r.get(8)?;
            let kind = classify(card, amount, neutral, settlement, expense);
            Ok(CardRecord {
                id: r.get(0)?,
                booking_date: r.get(1)?,
                provider,
                is_card: card,
                description: text,
                amount_minor: amount,
                currency: r.get(4)?,
                kind: kind.into(),
                category_key: r.get(9)?,
                manually_overridden: r.get(11)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub(crate) fn decide(
    db: &Connection,
    decision: &CreditDecision,
    card_id: Option<i64>,
) -> Result<(), String> {
    let account: i64 = db
        .query_row(
            "SELECT t.account_id FROM transactions t JOIN accounts a ON a.id=t.account_id
      WHERE t.id=?1 AND a.account_type='credit_card' AND a.is_active=1 AND t.amount_minor>0",
            [decision.transaction_id],
            |r| r.get(0),
        )
        .map_err(db_error)?;
    if card_id.is_some_and(|id| id != account)
        || !matches!(decision.kind.as_str(), "REFUND" | "UNKNOWN")
    {
        return Err("Ungültige Einordnung der Kartengutschrift.".into());
    }
    if decision.kind == "REFUND" {
        let key = decision
            .category_key
            .as_deref()
            .ok_or("Bitte eine Ausgabenkategorie wählen.")?;
        let category:i64=db.query_row("SELECT id FROM categories WHERE category_key=?1 AND category_key NOT IN ('income','credit_card')",[key],|r|r.get(0)).map_err(db_error)?;
        db.execute("UPDATE transactions SET category_id=?2,category_manual=1,category_source='manual' WHERE id=?1",
          params![decision.transaction_id,category]).map_err(db_error)?;
    }
    crate::storage::banking::reporting_flags::set_settlement(db, decision.transaction_id, false)?;
    db.execute(
        "INSERT INTO card_credit_decisions(transaction_id,kind) VALUES(?1,?2)
      ON CONFLICT(transaction_id) DO UPDATE SET kind=excluded.kind",
        params![decision.transaction_id, decision.kind],
    )
    .map_err(db_error)?;
    Ok(())
}

pub(crate) fn is_active_card(db: &Connection, card_id: i64) -> Result<bool, String> {
    db.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND account_type='credit_card' AND is_active=1)", [card_id], |r| r.get(0)).map_err(db_error)
}
pub(crate) fn valid_rule_seed(
    tx: &Connection,
    transaction_id: i64,
    card_side: bool,
    card_id: i64,
) -> Result<bool, String> {
    tx.query_row("SELECT EXISTS(SELECT 1 FROM transactions t JOIN accounts a ON a.id=t.account_id WHERE t.id=?1 AND a.is_active=1
              AND ((?2=1 AND a.id=?3 AND a.account_type='credit_card' AND t.amount_minor>0)
                OR (?2=0 AND a.id<>?3 AND a.account_type IN ('cash','checking','savings') AND t.amount_minor<0)))",
              params![transaction_id,card_side,card_id],|row|row.get(0)).map_err(db_error)
}
