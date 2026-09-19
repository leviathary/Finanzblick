//! Koordiniert Kurssynchronisation ohne SQL oder Provider-Protokolle.
use crate::domain::securities::quotes::DailyPrice;
use crate::infrastructure::market_data::{
    fetch_fx_with_fallback, market_sync_start_date, normalize_market_identifier, refresh_position,
};
use crate::storage::{
    securities::market_data as repository,
    securities::positions::rebuild_daily_valuations_incremental, Storage,
};
use chrono::{Local, Timelike};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRefreshResult {
    pub updated_positions: usize,
    pub stored_days: usize,
    pub skipped_positions: usize,
    pub errors: Vec<String>,
    pub refreshed_at: String,
}

pub(crate) async fn refresh_market_data(
    storage: &Storage,
    force: Option<bool>,
) -> Result<MarketRefreshResult, String> {
    let now = Local::now();
    if let Some(count) = repository::demo_position_count(storage)? {
        return Ok(MarketRefreshResult {
            updated_positions: 0,
            stored_days: 0,
            skipped_positions: count,
            errors: vec!["Demo: Simulierte Kurse werden nicht durch Live-Kurse ersetzt.".into()],
            refreshed_at: now.to_rfc3339(),
        });
    }
    if !force.unwrap_or(false) {
        let last_success = repository::last_success(storage)?;
        if let Some(last_success) = last_success {
            let today = now.format("%Y-%m-%d").to_string();
            let already_checked_after_close = last_success.starts_with(&today)
                && last_success
                    .get(11..13)
                    .and_then(|hour| hour.parse::<u32>().ok())
                    .is_some_and(|hour| hour >= 22);
            if last_success.starts_with(&today) && (now.hour() < 22 || already_checked_after_close)
            {
                return Ok(MarketRefreshResult {
                    updated_positions: 0,
                    stored_days: 0,
                    skipped_positions: 0,
                    errors: Vec::new(),
                    refreshed_at: last_success,
                });
            }
        }
    }
    repository::record_attempt(storage, &now.to_rfc3339())?;
    let (marketstack_api_key, alpha_vantage_api_key, positions) =
        repository::refresh_inputs(storage)?;
    let client = reqwest::Client::builder()
        .user_agent("Finanzblick/0.1")
        .build()
        .map_err(|_| "Die Kursverbindung konnte nicht vorbereitet werden.".to_string())?;
    let mut result = MarketRefreshResult {
        updated_positions: 0,
        stored_days: 0,
        skipped_positions: 0,
        errors: Vec::new(),
        refreshed_at: Local::now().to_rfc3339(),
    };

    let mut price_cache =
        HashMap::<String, Result<(String, String, String, Vec<DailyPrice>), String>>::new();
    let mut fx_cache = HashMap::<String, Result<(String, BTreeMap<String, f64>), String>>::new();
    for position in positions {
        let cache_key = format!(
            "{}:{}:{}",
            position.identifier_type,
            normalize_market_identifier(&position.identifier),
            market_sync_start_date(
                position.earliest_price_date.as_deref(),
                position.latest_price_date.as_deref(),
                &position.history_start_date,
                Local::now().date_naive(),
            ),
        );
        let quote = if let Some(cached) = price_cache.get(&cache_key) {
            cached.clone()
        } else {
            let loaded = refresh_position(
                &client,
                &marketstack_api_key,
                &alpha_vantage_api_key,
                &position,
            )
            .await;
            price_cache.insert(cache_key, loaded.clone());
            loaded
        };
        match quote {
            Ok((symbol, currency, source, values)) => {
                let first_price_date = values.first().map(|price| price.date.as_str());
                let (fx_source, rates) = if currency == position.valuation_currency {
                    (String::new(), BTreeMap::new())
                } else {
                    let fx_key = format!(
                        "{currency}:{}:{}",
                        position.valuation_currency,
                        first_price_date.unwrap_or_default(),
                    );
                    let loaded_rates = if let Some(cached) = fx_cache.get(&fx_key) {
                        cached.clone()
                    } else {
                        let loaded = fetch_fx_with_fallback(
                            &client,
                            &alpha_vantage_api_key,
                            &currency,
                            &position.valuation_currency,
                            first_price_date,
                            Some(source.as_str()),
                        )
                        .await;
                        fx_cache.insert(fx_key, loaded.clone());
                        loaded
                    };
                    match loaded_rates {
                        Ok(rates) => rates,
                        Err(error) => {
                            result.errors.push(format!("{}: {error}", position.label));
                            (String::new(), BTreeMap::new())
                        }
                    }
                };
                match repository::store_prices(
                    &storage, &position, &symbol, &currency, &source, values, &fx_source, rates,
                ) {
                    Ok(stored) => {
                        result.updated_positions += 1;
                        result.stored_days += stored;
                    }
                    Err(error) => result.errors.push(format!("{}: {error}", position.label)),
                }
            }
            Err(error) => result.errors.push(format!("{}: {error}", position.label)),
        }
    }
    result.skipped_positions = repository::skipped_positions(storage)?;
    if result.errors.is_empty() {
        repository::record_success(storage, &result.refreshed_at)?;
    }
    rebuild_daily_valuations_incremental(&storage)?;
    Ok(result)
}
