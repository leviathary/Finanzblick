//! Tauri-Eingänge für Anmeldung, Datenbankprofile und Backups; keine SQL- oder Passwortverwaltung.
use crate::storage::database::profiles::DatabaseChoice;
use crate::storage::database::{AppSettings, Storage, VaultStatus};
use chrono::Local;
use std::path::Path;
use tauri::{Manager, State};

#[tauri::command]
pub fn vault_status(storage: State<'_, Storage>) -> Result<VaultStatus, String> {
    storage.status()
}

#[tauri::command]
pub async fn unlock_vault(
    app: tauri::AppHandle,
    password: String,
    setup: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().unlock(password, setup))
        .await
        .map_err(|_| "Entsperren fehlgeschlagen.".to_string())?
}

#[tauri::command]
pub fn lock_vault(storage: State<'_, Storage>) -> Result<(), String> {
    storage.lock_session().map(|_| ())
}

#[tauri::command]
pub fn vault_activity(storage: State<'_, Storage>) -> Result<(), String> {
    crate::storage::database::connection::touch_activity(&storage)
}

#[tauri::command]
pub async fn save_app_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().save_settings(settings))
        .await
        .map_err(|_| "Einstellungen konnten nicht gespeichert werden.".to_string())?
}

#[tauri::command]
pub async fn change_vault_password(
    app: tauri::AppHandle,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .change_password(current_password, new_password)
    })
    .await
    .map_err(|_| "Das Passwort konnte nicht geändert werden.".to_string())?
}

#[tauri::command]
pub fn list_databases(storage: State<'_, Storage>) -> Result<Vec<DatabaseChoice>, String> {
    crate::storage::database::profiles::list_profiles(&storage)
}

#[tauri::command]
pub async fn create_database(
    app: tauri::AppHandle,
    name: String,
    password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>().create_database(&name, password)
    })
    .await
    .map_err(|_| "Finanzprofil konnte nicht erstellt werden.".to_string())?
}

#[tauri::command]
pub async fn copy_database(app: tauri::AppHandle, name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().copy_database(&name))
        .await
        .map_err(|_| "Finanzprofil konnte nicht kopiert werden.".to_string())?
}

#[tauri::command]
pub async fn switch_database(app: tauri::AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().select_database(&id))
        .await
        .map_err(|_| "Finanzprofil konnte nicht gewechselt werden.".to_string())?
}

#[tauri::command]
pub async fn anonymize_database(
    app: tauri::AppHandle,
    anonymize_descriptions: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .anonymize_database(anonymize_descriptions)
    })
    .await
    .map_err(|_| "Finanzprofil konnte nicht anonymisiert werden.".to_string())?
}

#[tauri::command]
pub async fn anonymize_database_with_factor(
    app: tauri::AppHandle,
    factor: f64,
    anonymize_descriptions: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .anonymize_database_with_factor(factor, anonymize_descriptions)
    })
    .await
    .map_err(|_| "Finanzprofil konnte nicht anonymisiert werden.".to_string())?
}

#[tauri::command]
pub async fn delete_database(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().delete_database())
        .await
        .map_err(|_| "Finanzprofil konnte nicht gelöscht werden.".to_string())?
}

#[tauri::command]
pub async fn create_backup(app: tauri::AppHandle, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>().create_backup_file(Path::new(&path))
    })
    .await
    .map_err(|_| "Backup konnte nicht erstellt werden.".to_string())?
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    path: String,
    name: String,
    password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .restore_backup_file(Path::new(&path), &name, password)
    })
    .await
    .map_err(|_| "Backup konnte nicht importiert werden.".to_string())?
}

#[tauri::command]
pub fn demo_status(storage: State<'_, Storage>) -> Result<bool, String> {
    crate::storage::database::demo::read_demo_status(&storage)
}

#[tauri::command]
pub async fn create_demo_database(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .create_demo(Local::now().date_naive())
    })
    .await
    .map_err(|_| "Demo konnte nicht erstellt werden.".to_string())?
}
