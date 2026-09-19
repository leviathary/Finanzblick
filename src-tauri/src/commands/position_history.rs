//! Tauri-Eingänge für Positionsverläufe.
use crate::storage::securities::position_history::{self as repository, PositionChart};
use crate::storage::Storage;
use tauri::State;
#[tauri::command]
pub fn position_chart_data(
    storage: State<'_, Storage>,
    account_id: i64,
) -> Result<Vec<PositionChart>, String> {
    repository::position_chart_data(&storage, account_id)
}
