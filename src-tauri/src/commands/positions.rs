//! Tauri-Schnittstellen für manuelle Positionen, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::database::Storage;
use crate::storage::securities::models::{
    DeleteManualValuationRequest, ManualPosition, ManualValuation, ManualValuationRequest,
    PositionQuantityChangeRequest, UpdateManualValuationRequest,
};
use tauri::{Manager, State};

#[tauri::command]
pub async fn save_manual_valuation(
    app: tauri::AppHandle,
    request: ManualValuationRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        storage::securities::positions::save_manual_valuation(&app.state::<Storage>(), request)
    })
    .await
    .map_err(|_| "Die Bewertung konnte nicht gespeichert werden.".to_string())?
}

#[tauri::command]
pub fn list_manual_positions(
    storage: State<'_, Storage>,
    account_id: i64,
) -> Result<Vec<ManualPosition>, String> {
    storage::securities::positions::list_manual_positions(&storage, account_id)
}

#[tauri::command]
pub fn list_manual_valuations(
    storage: State<'_, Storage>,
    position_id: i64,
) -> Result<Vec<ManualValuation>, String> {
    storage::securities::positions::list_manual_valuations(&storage, position_id)
}

#[tauri::command]
pub async fn update_manual_valuation(
    app: tauri::AppHandle,
    request: UpdateManualValuationRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        storage::securities::positions::update_manual_valuation(&app.state::<Storage>(), request)
    })
    .await
    .map_err(|_| "Die Bewertung konnte nicht korrigiert werden.".to_string())?
}

#[tauri::command]
pub async fn delete_manual_valuation(
    app: tauri::AppHandle,
    request: DeleteManualValuationRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        storage::securities::positions::delete_manual_valuation(&app.state::<Storage>(), request)
    })
    .await
    .map_err(|_| "Die Bewertung konnte nicht gelöscht werden.".to_string())?
}

#[tauri::command]
pub fn delete_manual_position(storage: State<'_, Storage>, position_id: i64) -> Result<(), String> {
    storage::securities::positions::delete_manual_position(&storage, position_id)
}

#[tauri::command]
pub async fn save_position_quantity_change(
    app: tauri::AppHandle,
    request: PositionQuantityChangeRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        storage::securities::positions::save_position_quantity_change(
            &app.state::<Storage>(),
            request,
        )
    })
    .await
    .map_err(|_| "Die Bestandsänderung konnte nicht gespeichert werden.".to_string())?
}
