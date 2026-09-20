//! Tauri-Schnittstellen für Importfreigabe und Importverwaltung, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::database::Storage;
use crate::storage::imports::models::{
    DuplicateCheck, ImportMappingProfile, ImportRun, SaveImportMappingProfile, SaveImportRequest,
    SaveImportResult,
};
use tauri::State;

#[tauri::command]
pub fn list_imports(storage: State<'_, Storage>) -> Result<Vec<ImportRun>, String> {
    storage::imports::list_imports(&storage)
}

#[tauri::command]
pub fn delete_imports(storage: State<'_, Storage>, ids: Vec<i64>) -> Result<usize, String> {
    storage::imports::delete_imports(&storage, ids)
}

#[tauri::command]
pub fn restore_import_duplicates(
    storage: State<'_, Storage>,
    import_id: i64,
) -> Result<usize, String> {
    storage::imports::restore_import_duplicates(&storage, import_id)
}

#[tauri::command]
pub fn list_import_mapping_profiles(
    storage: State<'_, Storage>,
) -> Result<Vec<ImportMappingProfile>, String> {
    storage::imports::list_import_mapping_profiles(&storage)
}

#[tauri::command]
pub fn save_import_mapping_profile(
    storage: State<'_, Storage>,
    profile: SaveImportMappingProfile,
) -> Result<i64, String> {
    storage::imports::save_import_mapping_profile(&storage, profile)
}

#[tauri::command]
pub fn check_import_duplicates(
    storage: State<'_, Storage>,
    request: SaveImportRequest,
) -> Result<DuplicateCheck, String> {
    storage::imports::check_import_duplicates(&storage, request)
}

#[tauri::command]
pub fn is_file_imported(storage: State<'_, Storage>, path: String) -> Result<bool, String> {
    storage::imports::is_file_imported(&storage, path)
}

#[tauri::command]
pub fn save_import(
    storage: State<'_, Storage>,
    request: SaveImportRequest,
) -> Result<SaveImportResult, String> {
    crate::application::imports::save(&storage, request)
}
