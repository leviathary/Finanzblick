//! Tauri-Schnittstellen zur Kurssynchronisation und zum lesenden Benchmark-Abruf.
use crate::{application::market_data, storage::Storage};
use tauri::State;
#[tauri::command]
pub async fn benchmark_data(
    storage: State<'_, Storage>,
    benchmark: String,
    from: String,
) -> Result<crate::application::benchmarks::BenchmarkData, String> {
    crate::application::benchmarks::load(&storage, &benchmark, &from).await
}
#[tauri::command]
pub async fn refresh_market_data(
    storage: State<'_, Storage>,
    force: Option<bool>,
) -> Result<market_data::MarketRefreshResult, String> {
    market_data::refresh_market_data(&storage, force).await
}
