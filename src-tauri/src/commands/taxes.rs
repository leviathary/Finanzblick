//! Stabile Tauri-Eingänge für Steuerimporte und manuelle Jahreswerte.
use crate::{
    application::taxes,
    domain::taxes::{
        SaveManualTaxSnapshotRequest, SaveTaxStatementRequest, SaveTaxStatementResult, TaxSnapshot,
        TaxStatementPreview, UpdateTaxSnapshotRequest,
    },
    storage::Storage,
};
use tauri::State;
#[tauri::command]
pub async fn preview_tax_statement(
    app: tauri::AppHandle,
    path: String,
) -> Result<TaxStatementPreview, String> {
    taxes::preview_tax_statement(app, path).await
}
#[tauri::command]
pub async fn save_tax_statement(
    app: tauri::AppHandle,
    request: SaveTaxStatementRequest,
) -> Result<SaveTaxStatementResult, String> {
    taxes::save_tax_statement(app, request).await
}
#[tauri::command]
pub fn save_manual_tax_snapshot(
    storage: State<'_, Storage>,
    request: SaveManualTaxSnapshotRequest,
) -> Result<SaveTaxStatementResult, String> {
    taxes::save_manual_tax_snapshot(&storage, request)
}
#[tauri::command]
pub fn update_tax_snapshot(
    storage: State<'_, Storage>,
    request: UpdateTaxSnapshotRequest,
) -> Result<TaxSnapshot, String> {
    taxes::update_tax_snapshot(&storage, request)
}
#[tauri::command]
pub fn list_tax_snapshots(storage: State<'_, Storage>) -> Result<Vec<TaxSnapshot>, String> {
    taxes::list_tax_snapshots(&storage)
}
#[tauri::command]
pub fn delete_tax_snapshot(storage: State<'_, Storage>, id: i64) -> Result<(), String> {
    taxes::delete_tax_snapshot(&storage, id)
}
