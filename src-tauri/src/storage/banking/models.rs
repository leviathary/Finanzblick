//! Fachbezogene Repository-Anfragen und Ergebnisprojektionen.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAccount {
    pub id: i64,
    pub institution_id: i64,
    pub provider: String,
    pub provider_key: String,
    pub institution_type: String,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
    pub is_active: bool,
    pub include_in_net_worth: bool,
    pub balance_minor: Option<i64>,
    pub balance_date: Option<String>,
    pub balance_currency: String,
    pub import_count: i64,
    pub manual_valuation_count: i64,
    pub manual_quantity: Option<f64>,
    pub manual_unit_price_minor: Option<i64>,
    pub manual_quote_currency: Option<String>,
    pub manual_exchange_rate: Option<f64>,
    pub logo_data_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub institution_name: String,
    pub institution_type: String,
    pub account_name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub id: i64,
    pub institution_name: Option<String>,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub external_reference: Option<String>,
    pub is_active: bool,
    pub include_in_net_worth: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzedTransaction {
    pub id: i64,
    pub booking_date: String,
    pub description: String,
    pub industry: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub category_key: String,
    pub category_label: String,
    pub category_color: String,
    pub category_source: String,
    pub provider: String,
    pub provider_key: String,
    pub account_name: String,
    pub excluded_from_totals: bool,
    pub is_card_settlement: bool,
    pub is_card: bool,
    pub is_manually_overridden: bool,
    pub expense_minor: i64,
    pub income_minor: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRow {
    pub(crate) id: i64,
    pub(crate) booking_date: String,
    pub(crate) account_name: String,
    pub(crate) description: String,
    pub(crate) amount_minor: i64,
    pub(crate) currency: String,
    pub(crate) transfer_type: crate::domain::banking::transfers::TransferType,
    pub(crate) counterparty_name: Option<String>,
    pub(crate) remittance_information: Option<String>,
    pub(crate) provider: String,
}
