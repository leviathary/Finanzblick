//! Tauri-Schnittstellen für Übersichten und Vermögensauswertungen, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::database::Storage;
use crate::storage::reporting::models::{DashboardData, DatabaseStatus, WealthData};
use tauri::State;

#[tauri::command]
pub fn database_status(storage: State<'_, Storage>) -> Result<DatabaseStatus, String> {
    storage::reporting::database_status(&storage)
}

#[tauri::command]
pub fn dashboard_data(storage: State<'_, Storage>) -> Result<DashboardData, String> {
    storage::reporting::dashboard_data(&storage)
}

#[tauri::command]
pub fn wealth_data(
    storage: State<'_, Storage>,
    account_ids: Option<Vec<i64>>,
) -> Result<WealthData, String> {
    storage::reporting::wealth_data(&storage, account_ids)
}
