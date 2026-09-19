//! Fachbezogene Repository-Anfragen und Ergebnisprojektionen.
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualValuationRequest {
    pub id: Option<i64>,
    pub account_id: i64,
    pub label: String,
    pub valuation_date: String,
    pub amount_minor: i64,
    pub quantity: Option<f64>,
    pub unit_price_minor: Option<i64>,
    pub quote_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub asset_type: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier: Option<String>,
    pub holding_start_date: Option<String>,
    pub holding_end_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualPosition {
    pub id: i64,
    pub account_id: i64,
    pub label: String,
    pub valuation_date: String,
    pub amount_minor: i64,
    pub value_currency: String,
    pub quantity: Option<f64>,
    pub unit_price_minor: Option<i64>,
    pub quote_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub asset_type: Option<String>,
    pub identifier_type: Option<String>,
    pub identifier: Option<String>,
    pub price_source: Option<String>,
    pub holding_start_date: Option<String>,
    pub holding_end_date: Option<String>,
}
