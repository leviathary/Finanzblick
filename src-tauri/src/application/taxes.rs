//! Koordiniert Steuerimport, Vorschau und validierte manuelle Änderungen.
use crate::domain::taxes::{
    validate_amounts, validate_breakdown, SaveManualTaxSnapshotRequest, SaveTaxStatementRequest,
    SaveTaxStatementResult, TaxSnapshot, TaxSnapshotInput, TaxStatementPreview,
    UpdateTaxSnapshotRequest, MANUAL_ENTRY_VERSION,
};
use crate::importers::taxes::zurich::{parse_tax_statement_file, PARSER_VERSION};
use crate::storage::{db_error, taxes::snapshots as repository, Storage};
use repository::persist_tax_snapshot;
use std::path::Path;
use tauri::Manager;
pub async fn preview_tax_statement(
    app: tauri::AppHandle,
    path: String,
) -> Result<TaxStatementPreview, String> {
    let parsed =
        tauri::async_runtime::spawn_blocking(move || parse_tax_statement_file(Path::new(&path)))
            .await
            .map_err(|_| "Die Steuererklärung konnte nicht verarbeitet werden.".to_string())??;
    let storage = app.state::<Storage>();
    let connection = storage.connect().map_err(db_error)?;
    let existing_year = repository::year_exists(&connection, parsed.tax_year)?;
    Ok(TaxStatementPreview {
        source_name: parsed.source_name,
        tax_year: parsed.tax_year,
        valuation_date: format!("{}-12-31", parsed.tax_year),
        gross_assets_minor: parsed.gross_assets_minor,
        liabilities_minor: parsed.liabilities_minor,
        taxable_wealth_minor: parsed.taxable_wealth_minor,
        securities_and_cash_minor: parsed.securities_and_cash_minor,
        real_estate_minor: parsed.real_estate_minor,
        other_assets_minor: parsed.other_assets_minor,
        canton_taxable_wealth_minor: None,
        currency: "CHF".into(),
        confidence: parsed.confidence,
        warnings: parsed.warnings,
        existing_year,
    })
}

pub async fn save_tax_statement(
    app: tauri::AppHandle,
    request: SaveTaxStatementRequest,
) -> Result<SaveTaxStatementResult, String> {
    let source_path = request.source_path.clone();
    let parsed = tauri::async_runtime::spawn_blocking(move || {
        parse_tax_statement_file(Path::new(&source_path))
    })
    .await
    .map_err(|_| "Die Steuererklärung konnte nicht verarbeitet werden.".to_string())??;
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    let storage = app.state::<Storage>();
    let mut connection = storage.connect().map_err(db_error)?;
    persist_tax_snapshot(
        &mut connection,
        TaxSnapshotInput {
            tax_year: parsed.tax_year,
            gross_assets_minor: request.gross_assets_minor,
            liabilities_minor: request.liabilities_minor,
            taxable_wealth_minor: request.taxable_wealth_minor,
            securities_and_cash_minor: request.securities_and_cash_minor,
            real_estate_minor: request.real_estate_minor,
            other_assets_minor: request.other_assets_minor,
            canton_taxable_wealth_minor: request.canton_taxable_wealth_minor,
            source_name: format!("Steuererklärung {}.pdf", parsed.tax_year),
            source_hash: parsed.source_hash,
            parser_version: PARSER_VERSION,
            confidence: parsed.confidence,
        },
    )
}

pub fn save_manual_tax_snapshot(
    storage: &Storage,
    request: SaveManualTaxSnapshotRequest,
) -> Result<SaveTaxStatementResult, String> {
    if !(1990..=2100).contains(&request.tax_year) {
        return Err("Bitte ein Steuerjahr zwischen 1990 und 2100 eingeben.".into());
    }
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    validate_breakdown(request.securities_and_cash_minor, request.real_estate_minor)?;
    let mut connection = storage.connect().map_err(db_error)?;
    persist_tax_snapshot(
        &mut connection,
        TaxSnapshotInput {
            tax_year: request.tax_year,
            gross_assets_minor: request.gross_assets_minor,
            liabilities_minor: request.liabilities_minor,
            taxable_wealth_minor: request.taxable_wealth_minor,
            securities_and_cash_minor: request.securities_and_cash_minor,
            real_estate_minor: request.real_estate_minor,
            other_assets_minor: request.other_assets_minor,
            canton_taxable_wealth_minor: request.canton_taxable_wealth_minor,
            source_name: format!("Manuelle Eingabe {}", request.tax_year),
            source_hash: format!("manual:{}", request.tax_year),
            parser_version: MANUAL_ENTRY_VERSION,
            confidence: 1.0,
        },
    )
}

pub(crate) fn update_tax_snapshot(
    storage: &Storage,
    request: UpdateTaxSnapshotRequest,
) -> Result<TaxSnapshot, String> {
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    let mut connection = storage.connect().map_err(db_error)?;
    repository::update_tax_snapshot(&mut connection, request)
}
pub(crate) fn list_tax_snapshots(storage: &Storage) -> Result<Vec<TaxSnapshot>, String> {
    let connection = storage.connect().map_err(db_error)?;
    repository::snapshots_from(&connection)
}
pub(crate) fn delete_tax_snapshot(storage: &Storage, id: i64) -> Result<(), String> {
    let connection = storage.connect().map_err(db_error)?;
    repository::delete_tax_snapshot(&connection, id)
}
