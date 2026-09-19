//! Tauri-Schnittstellen für Transaktionen und Auswertungsmarkierungen, ohne eigene SQL-Abfragen.
use crate::storage;
use crate::storage::banking::models::TransferRow;
use crate::storage::database::Storage;
use crate::storage::reporting::models::TransactionAnalysis;
use tauri::State;

#[tauri::command]
pub fn bank_balance_history(
    storage: State<'_, Storage>,
    provider_key: Option<String>,
    account_id: Option<i64>,
) -> Result<Vec<storage::reporting::models::TransactionHistoryPoint>, String> {
    storage::banking::transactions::bank_balance_history(&storage, provider_key, account_id)
}

#[tauri::command]
pub fn transaction_analysis(
    storage: State<'_, Storage>,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
    provider_keys: Option<Vec<String>>,
    account_ids: Option<Vec<i64>>,
) -> Result<TransactionAnalysis, String> {
    storage::banking::transactions::transaction_analysis(
        &storage,
        from,
        to,
        provider_key,
        account_id,
        provider_keys.unwrap_or_default(),
        account_ids.unwrap_or_default(),
    )
}

#[tauri::command]
pub fn set_transaction_transfers(
    storage: State<'_, Storage>,
    transaction_ids: Vec<i64>,
    transfer_type: crate::domain::banking::transfers::TransferType,
) -> Result<(), String> {
    storage::banking::transactions::set_transaction_transfers(
        &storage,
        transaction_ids,
        transfer_type,
    )
}

#[tauri::command]
pub fn list_transaction_transfers(
    storage: State<'_, Storage>,
    account_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    search: String,
) -> Result<Vec<TransferRow>, String> {
    storage::banking::transactions::list_transaction_transfers(
        &storage, account_id, from, to, search,
    )
}

#[tauri::command]
pub fn set_transaction_settlement(
    storage: State<'_, Storage>,
    transaction_id: i64,
    is_settlement: bool,
) -> Result<(), String> {
    storage::banking::transactions::set_transaction_settlement(
        &storage,
        transaction_id,
        is_settlement,
    )
}

#[tauri::command]
pub fn set_transaction_category(
    storage: State<'_, Storage>,
    transaction_id: i64,
    category_key: String,
) -> Result<usize, String> {
    storage::banking::transactions::set_transaction_category(&storage, transaction_id, category_key)
}
