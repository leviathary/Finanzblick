//! Fachbezogene Repository-Anfragen und Ergebnisprojektionen.
use crate::importers::{ParsedStatement, TabularMapping};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRun {
    pub(crate) id: i64,
    pub(crate) source_name: String,
    pub(crate) source_format: String,
    pub(crate) provider: String,
    pub(crate) accounts: String,
    pub(crate) imported_at: String,
    pub(crate) transaction_count: i64,
    pub(crate) ignored_duplicate_count: i64,
    pub(crate) first_date: Option<String>,
    pub(crate) last_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportRequest {
    #[serde(default)]
    pub account_ids: BTreeMap<String, i64>,
    pub source_path: String,
    pub account_name: String,
    pub statement: ParsedStatement,
    #[serde(default)]
    pub duplicate_resolutions: Vec<DuplicateResolution>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResolution {
    pub transaction_index: usize,
    pub action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMappingProfile {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) header_fingerprint: String,
    pub(crate) mapping: TabularMapping,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportMappingProfile {
    pub(crate) name: String,
    pub(crate) header_fingerprint: String,
    pub(crate) mapping: TabularMapping,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportResult {
    pub import_id: i64,
    pub account_id: i64,
    pub inserted_transactions: usize,
    pub updated_transactions: usize,
    pub duplicate: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCheck {
    pub(crate) exact_file: bool,
    pub(crate) matching_transactions: usize,
    pub(crate) updatable_transactions: usize,
    pub(crate) total_transactions: usize,
    pub(crate) suspected_transactions: Vec<SuspectedDuplicate>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspectedDuplicate {
    pub(crate) transaction_index: usize,
    pub(crate) match_source: String,
    pub(crate) existing_transaction_id: Option<i64>,
    pub(crate) compared_transaction_index: Option<usize>,
    pub(crate) booking_date: String,
    pub(crate) description: String,
    pub(crate) amount_minor: i64,
    pub(crate) currency: String,
    pub(crate) match_kind: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct TransactionIdentity {
    pub(crate) namespace: Option<String>,
    pub(crate) external_reference: Option<String>,
    pub(crate) fallback_fingerprint: String,
}
