//! Koordiniert Kartenprüflisten und atomare Einrichtung; enthält keine SQL-Abfragen.
use crate::domain::banking::cards::CreditDecision;
use crate::storage::banking::cards as repository;
use crate::storage::{db_error, settlement_rules, Storage};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRow {
    id: i64,
    booking_date: String,
    description: String,
    amount_minor: i64,
    currency: String,
    kind: String,
    category_key: String,
    suggested_settlement: bool,
    manually_overridden: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupRule {
    transaction_id: i64,
    prefix: String,
    expected_ids: Vec<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupRequest {
    card_id: i64,
    card_rule: Option<SetupRule>,
    bank_rule: Option<SetupRule>,
    decisions: Vec<CreditDecision>,
    past: bool,
    future: bool,
}

pub(crate) fn list_rows(db: &Connection, account_id: i64) -> Result<Vec<CardRow>, String> {
    repository::list_rows(db, account_id).map(|rows| {
        rows.into_iter()
            .map(|r| CardRow {
                suggested_settlement: r.is_card
                    && r.amount_minor > 0
                    && crate::importers::card_credit_hint(&r.provider, &r.description)
                        == Some("card_settlement"),
                id: r.id,
                booking_date: r.booking_date,
                description: r.description,
                amount_minor: r.amount_minor,
                currency: r.currency,
                kind: r.kind,
                category_key: r.category_key,
                manually_overridden: r.manually_overridden,
            })
            .collect()
    })
}
pub(crate) fn list(storage: &Storage, account_id: i64) -> Result<Vec<CardRow>, String> {
    let db = storage.connect().map_err(db_error)?;
    list_rows(&db, account_id)
}
pub(crate) fn set_decision(storage: &Storage, decision: CreditDecision) -> Result<(), String> {
    let mut db = storage.connect().map_err(db_error)?;
    let tx = db.transaction().map_err(db_error)?;
    repository::decide(&tx, &decision, None)?;
    tx.commit().map_err(db_error)
}
pub(crate) fn confirm(storage: &Storage, request: SetupRequest) -> Result<(), String> {
    let mut db = storage.connect().map_err(db_error)?;
    apply_setup(&mut db, request)
}
pub(crate) fn apply_setup(db: &mut Connection, r: SetupRequest) -> Result<(), String> {
    let tx = db.transaction().map_err(db_error)?;
    let valid = repository::is_active_card(&tx, r.card_id)?;
    if !valid || r.decisions.len() > 5000 {
        return Err("Ungültige Karteneinrichtung.".into());
    }
    for (rule, card_side) in [(&r.card_rule, true), (&r.bank_rule, false)] {
        if let Some(rule) = rule {
            let valid =
                repository::valid_rule_seed(&tx, rule.transaction_id, card_side, r.card_id)?;
            if !valid || (!r.past && !r.future) {
                return Err("Ungültige Karteneinrichtung.".into());
            }
            if r.decisions
                .iter()
                .any(|d| rule.expected_ids.contains(&d.transaction_id))
            {
                return Err("Die Regel enthält eine separat eingeordnete Gutschrift. Bitte den Textanfang präzisieren.".into());
            }
            settlement_rules::apply_confirmed(
                &tx,
                rule.transaction_id,
                rule.prefix.clone(),
                rule.expected_ids.clone(),
                r.past,
                r.future,
            )?;
        }
    }
    for d in &r.decisions {
        repository::decide(&tx, d, Some(r.card_id))?;
    }
    tx.commit().map_err(db_error)
}

#[cfg(test)]
mod tests;
