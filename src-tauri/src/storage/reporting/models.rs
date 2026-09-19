//! Fachbezogene Repository-Anfragen und Ergebnisprojektionen.
use crate::storage::banking::models::{AnalyzedTransaction, ManagedAccount};
use serde::Serialize;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub path: String,
    pub accounts: i64,
    pub imports: i64,
    pub transactions: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardAccount {
    pub id: i64,
    pub provider: String,
    pub provider_key: String,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub balance_currency: String,
    pub balance_minor: Option<i64>,
    pub balance_date: Option<String>,
    pub include_in_net_worth: bool,
    pub logo_data_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardProvider {
    pub provider: String,
    pub provider_key: String,
    pub balance_minor: i64,
    pub account_count: i64,
    pub logo_data_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentImport {
    pub source_name: String,
    pub provider: String,
    pub account_name: String,
    pub imported_at: String,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total_balance_minor: i64,
    pub currency: String,
    pub accounts: Vec<DashboardAccount>,
    pub providers: Vec<DashboardProvider>,
    pub recent_import: Option<RecentImport>,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthHistoryPoint {
    pub date: String,
    pub total_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthBreakdown {
    pub key: String,
    pub label: String,
    pub amount_minor: i64,
    pub account_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthData {
    pub currency: String,
    pub current_total_minor: i64,
    pub first_total_minor: Option<i64>,
    pub change_minor: Option<i64>,
    pub history: Vec<WealthHistoryPoint>,
    pub by_type: Vec<WealthBreakdown>,
    pub by_provider: Vec<WealthBreakdown>,
    pub accounts: Vec<ManagedAccount>,
    pub excluded_account_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySpend {
    pub key: String,
    pub label: String,
    pub color: String,
    pub amount_minor: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySpend {
    pub month: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryPoint {
    pub date: String,
    pub total_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionAnalysis {
    pub total_income_minor: i64,
    pub income_count: i64,
    pub income_transactions: Vec<AnalyzedTransaction>,
    pub total_spend_minor: i64,
    pub transaction_count: i64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub categories: Vec<CategorySpend>,
    pub months: Vec<MonthlySpend>,
    pub history: Vec<TransactionHistoryPoint>,
    pub transactions: Vec<AnalyzedTransaction>,
    pub providers: Vec<DashboardProvider>,
}
