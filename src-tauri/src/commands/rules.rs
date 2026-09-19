//! Tauri-Eingänge für Umbuchungsregeln und den bestehenden Kartenausgleichs-Workflow.
use crate::domain::banking::transfers::TransferRuleInput;
use crate::storage::rules::settlement_rules::{self as repository, Preview, Rule};
use crate::storage::rules::transfer_rules;
use crate::storage::Storage;
use tauri::State;

#[tauri::command]
pub fn list_transfer_rules(
    storage: State<'_, Storage>,
) -> Result<Vec<transfer_rules::Rule>, String> {
    transfer_rules::list_transfer_rules(&storage)
}
#[tauri::command]
pub fn preview_transfer_rule(
    storage: State<'_, Storage>,
    input: TransferRuleInput,
) -> Result<transfer_rules::Preview, String> {
    transfer_rules::preview_transfer_rule(&storage, input)
}
#[tauri::command]
pub fn save_transfer_rule(
    storage: State<'_, Storage>,
    input: TransferRuleInput,
    expected_ids: Vec<i64>,
    past: bool,
    future: bool,
) -> Result<usize, String> {
    transfer_rules::save_transfer_rule(&storage, input, expected_ids, past, future)
}
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
