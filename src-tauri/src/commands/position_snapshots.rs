//! Stellt die gemeinsamen Tauri-Kommandos für Depotbestände zum Stichtag bereit.

use crate::application;
use crate::storage::database::Storage;
use crate::storage::securities::position_snapshots::{
    PositionSnapshotPreview, SavePositionSnapshotResult,
};
use tauri::State;

#[tauri::command]
pub fn preview_position_snapshot(
    storage: State<'_, Storage>,
    path: String,
    account_id: Option<i64>,
    snapshot_date: Option<String>,
) -> Result<Option<PositionSnapshotPreview>, String> {
    application::position_snapshots::preview(&storage, path, account_id, snapshot_date)
}

#[tauri::command]
pub fn save_position_snapshot(
    storage: State<'_, Storage>,
    path: String,
    account_id: i64,
    snapshot_date: Option<String>,
) -> Result<SavePositionSnapshotResult, String> {
    application::position_snapshots::save(&storage, path, account_id, snapshot_date)
}
