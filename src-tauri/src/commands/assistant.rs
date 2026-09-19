//! Tauri-Eingänge für den Finanzassistenten; keine Datenbankabfragen.
use crate::{
    application::assistant::{account, context},
    domain::assistant::PrepareRequest,
};
use tauri::State;
#[tauri::command]
pub async fn prepare_finance_chat(
    app: tauri::AppHandle,
    request: PrepareRequest,
) -> Result<context::Preview, String> {
    context::prepare_finance_chat(app, request).await
}
#[tauri::command]
pub async fn chatgpt_status(app: tauri::AppHandle) -> Result<account::AccountStatus, String> {
    account::chatgpt_status(app).await
}
#[tauri::command]
pub async fn chatgpt_login_start(app: tauri::AppHandle) -> Result<(), String> {
    account::chatgpt_login_start(app).await
}
#[tauri::command]
pub async fn chatgpt_disconnect(app: tauri::AppHandle) -> Result<(), String> {
    account::chatgpt_disconnect(app).await
}
#[tauri::command]
pub fn cancel_finance_chat(state: State<'_, account::ChatState>) {
    account::cancel_finance_chat(state)
}
#[tauri::command]
pub async fn send_finance_chat(app: tauri::AppHandle, preview_id: u64) -> Result<String, String> {
    account::send_finance_chat(app, preview_id).await
}
