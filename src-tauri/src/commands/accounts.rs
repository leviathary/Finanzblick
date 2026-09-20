//! Tauri-Schnittstellen für Kontenverwaltung, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::banking::models::{
    CreateAccountRequest, CreateInstitutionRequest, ManagedAccount, ManagedInstitution,
    UpdateAccountRequest, UpdateInstitutionRequest,
};
use crate::storage::database::Storage;
use tauri::State;

#[tauri::command]
pub fn list_accounts(storage: State<'_, Storage>) -> Result<Vec<ManagedAccount>, String> {
    storage::banking::accounts::list_accounts(&storage)
}

#[tauri::command]
pub fn list_institutions(storage: State<'_, Storage>) -> Result<Vec<ManagedInstitution>, String> {
    storage::banking::accounts::list_institutions(&storage)
}

#[tauri::command]
pub fn create_institution(
    storage: State<'_, Storage>,
    request: CreateInstitutionRequest,
) -> Result<i64, String> {
    storage::banking::accounts::create_institution(&storage, request)
}

#[tauri::command]
pub fn create_account(
    storage: State<'_, Storage>,
    request: CreateAccountRequest,
) -> Result<i64, String> {
    storage::banking::accounts::create_account(&storage, request)
}

#[tauri::command]
pub fn update_account(
    storage: State<'_, Storage>,
    request: UpdateAccountRequest,
) -> Result<(), String> {
    storage::banking::accounts::update_account(&storage, request)
}

#[tauri::command]
pub fn update_institution(
    storage: State<'_, Storage>,
    request: UpdateInstitutionRequest,
) -> Result<(), String> {
    storage::banking::accounts::update_institution(&storage, request)
}

#[tauri::command]
pub fn delete_account(storage: State<'_, Storage>, account_id: i64) -> Result<(), String> {
    storage::banking::accounts::delete_account(&storage, account_id)
}

#[tauri::command]
pub fn set_institution_logo(
    storage: State<'_, Storage>,
    institution_id: i64,
    data_url: Option<String>,
) -> Result<(), String> {
    storage::banking::accounts::set_institution_logo(&storage, institution_id, data_url)
}
