//! Tauri-Schnittstelle zur Kurssynchronisation.
use crate::{application::market_data, storage::Storage};
use tauri::State;
#[tauri::command]
pub async fn refresh_market_data(
    storage: State<'_, Storage>,
    force: Option<bool>,
) -> Result<market_data::MarketRefreshResult, String> {
    market_data::refresh_market_data(&storage, force).await
}
