//! Liest eine Datei über die Format-Registry und normalisiert Provider-Hinweise ohne Datenbankzugriff.
use super::{card_credit_hint, registry, ParsedStatement, TabularMapping, MAX_FILE_SIZE};
use std::{fs, path::Path};

#[cfg(test)]
pub fn parse_statement(
    path: String,
    selected_provider: Option<String>,
    mapping: Option<TabularMapping>,
) -> Result<ParsedStatement, String> {
    parse_statement_with_pdf_mapping(path, selected_provider, mapping, None)
}

pub fn parse_statement_with_pdf_mapping(
    path: String,
    selected_provider: Option<String>,
    mapping: Option<TabularMapping>,
    pdf_mapping: Option<super::PdfMapping>,
) -> Result<ParsedStatement, String> {
    let path = Path::new(&path);
    let metadata = fs::metadata(path)
        .map_err(|_| "Die ausgewählte Datei ist nicht mehr verfügbar.".to_string())?;
    if !metadata.is_file() {
        return Err("Bitte eine Datei und keinen Ordner auswählen.".to_string());
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err("Die Datei ist größer als 25 MB.".to_string());
    }
    let mut statement = if let Some(pdf_mapping) = pdf_mapping.as_ref() {
        super::pdf::parse_mapped(path, selected_provider.as_deref(), pdf_mapping)?
    } else {
        registry::parse(path, selected_provider.as_deref(), mapping.as_ref())?
    };
    if statement.account_type.as_deref() == Some("credit_card") {
        for row in &mut statement.transactions {
            if row.amount_minor > 0 && row.transaction_kind == "cash_transaction" {
                if let Some(kind) = card_credit_hint(&statement.provider, &row.description) {
                    row.transaction_kind = kind.into();
                }
            }
        }
    }
    Ok(statement)
}
