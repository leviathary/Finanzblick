//! Typen und Plausibilitätsregeln für jährliche Steuerwerte.
use serde::{Deserialize, Serialize};
pub(crate) const MANUAL_ENTRY_VERSION: &str = "manual-v1";
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxStatementPreview {
    pub source_name: String,
    pub tax_year: i32,
    pub valuation_date: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
    pub currency: String,
    pub confidence: f64,
    pub warnings: Vec<String>,
    pub existing_year: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxSnapshot {
    pub id: i64,
    pub tax_year: i32,
    pub valuation_date: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
    pub currency: String,
    pub source_name: String,
    pub confidence: f64,
    pub imported_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTaxStatementRequest {
    pub source_path: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveManualTaxSnapshotRequest {
    pub tax_year: i32,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTaxStatementResult {
    pub snapshot: TaxSnapshot,
    pub replaced_existing_year: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaxSnapshotRequest {
    pub id: i64,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Debug)]
pub(crate) struct ParsedTaxStatement {
    pub(crate) source_name: String,
    pub(crate) source_hash: String,
    pub(crate) tax_year: i32,
    pub(crate) gross_assets_minor: i64,
    pub(crate) liabilities_minor: i64,
    pub(crate) taxable_wealth_minor: i64,
    pub(crate) securities_and_cash_minor: i64,
    pub(crate) real_estate_minor: i64,
    pub(crate) other_assets_minor: i64,
    pub(crate) confidence: f64,
    pub(crate) warnings: Vec<String>,
}

pub(crate) struct TaxSnapshotInput {
    pub(crate) tax_year: i32,
    pub(crate) gross_assets_minor: i64,
    pub(crate) liabilities_minor: i64,
    pub(crate) taxable_wealth_minor: i64,
    pub(crate) securities_and_cash_minor: i64,
    pub(crate) real_estate_minor: i64,
    pub(crate) other_assets_minor: i64,
    pub(crate) canton_taxable_wealth_minor: Option<i64>,
    pub(crate) source_name: String,
    pub(crate) source_hash: String,
    pub(crate) parser_version: &'static str,
    pub(crate) confidence: f64,
}

pub(crate) fn validate_amounts(
    gross_assets_minor: i64,
    liabilities_minor: i64,
    taxable_wealth_minor: i64,
    canton_taxable_wealth_minor: Option<i64>,
) -> Result<(), String> {
    if gross_assets_minor < 0 || liabilities_minor < 0 || taxable_wealth_minor < 0 {
        return Err("Vermögen und Schulden dürfen nicht negativ sein.".into());
    }
    if gross_assets_minor
        .checked_sub(liabilities_minor)
        .ok_or_else(|| "Die Beträge sind ausserhalb des unterstützten Bereichs.".to_string())?
        != taxable_wealth_minor
    {
        return Err(
            "Die Plausibilitätsprüfung ist fehlgeschlagen: Vermögenswerte minus Schulden müssen dem steuerbaren Vermögen entsprechen."
                .into(),
        );
    }
    if canton_taxable_wealth_minor.is_some_and(|value| value < 0 || value > taxable_wealth_minor) {
        return Err("Das kantonale steuerbare Vermögen ist nicht plausibel.".into());
    }
    Ok(())
}

pub(crate) fn validate_breakdown(
    securities_and_cash_minor: i64,
    real_estate_minor: i64,
) -> Result<(), String> {
    if securities_and_cash_minor < 0 || real_estate_minor < 0 {
        return Err("Wertschriften, Guthaben und Liegenschaften dürfen nicht negativ sein.".into());
    }
    Ok(())
}
