//! Normalisierte Kursdaten und Regeln für historische Bewertungslücken.
use chrono::{Duration, NaiveDate};
#[derive(Debug)]
pub(crate) struct PositionToRefresh {
    pub listing_id: i64,
    pub label: String,
    pub identifier_type: String,
    pub identifier: String,
    pub market_symbol: Option<String>,
    pub market_currency: Option<String>,
    pub valuation_currency: String,
    pub history_start_date: String,
    pub earliest_price_date: Option<String>,
    pub latest_price_date: Option<String>,
    pub preferred_price_source: Option<String>,
    pub asset_type: String,
}

#[derive(Clone, Debug)]
pub(crate) struct DailyPrice {
    pub date: String,
    pub close: f64,
}

pub(crate) const LEADING_PRICE_GAP_TOLERANCE_DAYS: i64 = 7;
pub(crate) fn needs_historical_backfill(position: &PositionToRefresh) -> bool {
    let Ok(history_start) = NaiveDate::parse_from_str(&position.history_start_date, "%Y-%m-%d")
    else {
        return false;
    };
    position
        .earliest_price_date
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .is_none_or(|earliest| {
            earliest > history_start + Duration::days(LEADING_PRICE_GAP_TOLERANCE_DAYS)
        })
}
