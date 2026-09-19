//! Tauri-Schnittstellen für manuelle Positionen, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::database::Storage;
use crate::storage::securities::models::{ManualPosition, ManualValuationRequest};
use tauri::State;

#[tauri::command]
pub fn save_manual_valuation(
    storage: State<'_, Storage>,
    request: ManualValuationRequest,
) -> Result<(), String> {
    storage::securities::positions::save_manual_valuation(&storage, request)
}

#[tauri::command]
pub fn list_manual_positions(
    storage: State<'_, Storage>,
    account_id: i64,
) -> Result<Vec<ManualPosition>, String> {
    storage::securities::positions::list_manual_positions(&storage, account_id)
}

#[tauri::command]
pub fn delete_manual_position(storage: State<'_, Storage>, position_id: i64) -> Result<(), String> {
    storage::securities::positions::delete_manual_position(&storage, position_id)
}
