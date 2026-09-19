//! Tauri-Eingänge für bestätigte Ausgleichsregeln.
use crate::storage::rules::settlement_rules::{self as repository, Preview, Rule};
use crate::storage::Storage;
use tauri::State;
#[tauri::command]
pub fn preview_settlement_rule(
    storage: State<'_, Storage>,
    transaction_id: i64,
    prefix: Option<String>,
) -> Result<Preview, String> {
    repository::preview_settlement_rule(&storage, transaction_id, prefix)
}
#[tauri::command]
pub fn confirm_settlement_rule(
    storage: State<'_, Storage>,
    transaction_id: i64,
    prefix: String,
    expected_ids: Vec<i64>,
    future: bool,
) -> Result<usize, String> {
    repository::confirm_settlement_rule(&storage, transaction_id, prefix, expected_ids, future)
}
#[tauri::command]
pub fn list_settlement_rules(storage: State<'_, Storage>) -> Result<Vec<Rule>, String> {
    repository::list_settlement_rules(&storage)
}
#[tauri::command]
pub fn delete_settlement_rule(storage: State<'_, Storage>, rule_id: i64) -> Result<(), String> {
    repository::delete_settlement_rule(&storage, rule_id)
}
