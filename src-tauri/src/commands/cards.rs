//! Übersetzt Karten-Commands in Aufrufe der Anwendungsschicht.
use crate::{application::cards, domain::banking::cards::CreditDecision, storage::Storage};
use tauri::State;
#[tauri::command]
pub fn list_card_setup_transactions(
    storage: State<'_, Storage>,
    account_id: i64,
) -> Result<Vec<cards::CardRow>, String> {
    cards::list(&storage, account_id)
}
#[tauri::command]
pub fn set_card_credit_decision(
    storage: State<'_, Storage>,
    decision: CreditDecision,
) -> Result<(), String> {
    cards::set_decision(&storage, decision)
}
#[tauri::command]
pub fn confirm_card_setup(
    storage: State<'_, Storage>,
    request: cards::SetupRequest,
) -> Result<(), String> {
    cards::confirm(&storage, request)
}
