//! Liest vollständige Swissquote-Positionssnapshots aus den dafür vorgesehenen Excel- und PDF-Exporten.
//! Der Parser übernimmt ausschliesslich Symbole und exakte Mengen; Kurs- und Bewertungsfelder bleiben unberücksichtigt.

use crate::domain::securities::position_snapshots::{
    parse_quantity, quantity_string, PositionSnapshot, PositionSnapshotRow, SnapshotScope,
};
use calamine::{open_workbook_auto, Data, Reader};
use chrono::NaiveDate;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;

use super::super::formats::pdf::extract_pdf_text;

pub fn parse(path: &Path) -> Result<Option<PositionSnapshot>, String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "xlsx" | "xls" => parse_excel(path),
        "pdf" => parse_pdf(path),
        _ => Ok(None),
    }
}

fn parse_excel(path: &Path) -> Result<Option<PositionSnapshot>, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Swissquote-Excel konnte nicht geöffnet werden: {error}"))?;
    for sheet_name in workbook.sheet_names().to_vec() {
        let range = workbook.worksheet_range(&sheet_name).map_err(|error| {
            format!("Swissquote-Tabellenblatt konnte nicht gelesen werden: {error}")
        })?;
        let rows: Vec<&[Data]> = range.rows().collect();
        if let Some(snapshot) = parse_excel_rows(path, &rows)? {
            return Ok(Some(snapshot));
        }
    }
    Ok(None)
}

fn parse_excel_rows(path: &Path, rows: &[&[Data]]) -> Result<Option<PositionSnapshot>, String> {
    let Some((header_index, symbol_column, quantity_column, currency_column)) = excel_header(rows)
    else {
        return Ok(None);
    };
    let mut category = String::new();
    let mut positions = Vec::new();
    let mut complete = false;
    let mut empty_total_is_zero = false;
    for (index, row) in rows.iter().enumerate().skip(header_index + 1) {
        let first = cell_text(row.first());
        let symbol = cell_text(row.get(symbol_column));
        if symbol.eq_ignore_ascii_case("Gesamt") {
            complete = true;
            let total_column = rows[header_index]
                .iter()
                .position(|cell| normalize_label(&cell.to_string()) == "totalwert chf");
            empty_total_is_zero = total_column
                .and_then(|column| parse_quantity(&cell_text(row.get(column))).ok())
                .is_some_and(|value| value.amount == 0);
            continue;
        }
        if symbol.is_empty() {
            if !cell_text(row.get(quantity_column)).is_empty() {
                return Err(
                    "Eine Positionszeile ohne Symbol kann nicht sicher importiert werden.".into(),
                );
            }
            if !first.is_empty() {
                category = first;
            }
            continue;
        }
        let quantity = cell_text(row.get(quantity_column));
        let scaled = parse_quantity(&quantity)
            .map_err(|error| format!("Swissquote-Excel, Zeile {}: {error}", index + 1))?;
        if scaled.amount <= 0 {
            return Err(format!(
                "Swissquote-Excel, Zeile {}: Die Positionsmenge muss grösser als null sein.",
                index + 1
            ));
        }
        let quote_currency = cell_text(row.get(currency_column)).to_ascii_uppercase();
        positions.push(position_row(
            &symbol,
            &quantity_string(scaled),
            &category,
            &quote_currency,
            index + 1,
        )?);
    }
    if !complete {
        return Err("Der Swissquote-Excel-Export enthält keine Gesamtzeile und wird deshalb nicht als vollständiger Positionsbestand importiert.".into());
    }
    if positions.is_empty() && !empty_total_is_zero {
        return Err("Ein leeres Depot benötigt eine eindeutige Gesamtzeile mit Wert null.".into());
    }
    validate_unique_symbols(&positions)?;
    let (account_reference, snapshot_date, warning) = filename_metadata(path);
    Ok(Some(PositionSnapshot {
        provider: "swissquote".into(),
        reference_is_shared: true,
        scope: SnapshotScope::FullPortfolio,
        format: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("xlsx")
            .to_ascii_uppercase(),
        snapshot_date,
        account_reference,
        positions,
        warnings: warning.into_iter().collect(),
    }))
}

fn excel_header(rows: &[&[Data]]) -> Option<(usize, usize, usize, usize)> {
    for (row_index, row) in rows.iter().enumerate().take(20) {
        let headers = row
            .iter()
            .map(|cell| normalize_label(&cell.to_string()))
            .collect::<Vec<_>>();
        let Some(symbol) = headers.iter().position(|value| value == "symbol") else {
            continue;
        };
        let Some(quantity) = headers.iter().position(|value| value == "anzahl") else {
            continue;
        };
        if !headers.iter().any(|value| value == "totalwert chf") {
            continue;
        }
        let Some(currency) = headers.iter().position(|value| value == "wahrung") else {
            continue;
        };
        return Some((row_index, symbol, quantity, currency));
    }
    None
}

fn parse_pdf(path: &Path) -> Result<Option<PositionSnapshot>, String> {
    let text = extract_pdf_text(path)?;
    if !text.to_lowercase().contains("kontoübersicht")
        || !text.contains("Produkt Anzahl Einstandskurs")
    {
        return Ok(None);
    }
    let section_start = text
        .find("Produkt Anzahl Einstandskurs")
        .ok_or("Swissquote-Positionstabelle fehlt im PDF.")?;
    let section = &text[section_start..];
    if !section.contains("Zwischensumme") || !section.contains("Gesamt CHF") {
        return Err("Die Swissquote-PDF-Positionstabelle ist nicht vollständig und wird nicht als Bestandsabgleich importiert.".into());
    }
    let row_pattern = Regex::new(r"^([A-Z][A-Z0-9.\-]{1,14})\s+([0-9][0-9'\u{2019}.,]*)\s+")
        .map_err(|error| error.to_string())?;
    let currency_pattern = Regex::new(r"\b(CHF|USD|EUR|GBP|CAD|AUD|JPY|CNY|HKD|SGD|SEK|NOK|DKK)\b")
        .map_err(|error| error.to_string())?;
    let lines = section.lines().map(str::trim).collect::<Vec<_>>();
    let mut category = String::new();
    let mut positions = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if is_category_label(line) {
            category = (*line).to_string();
            continue;
        }
        let Some(captures) = row_pattern.captures(line) else {
            continue;
        };
        let symbol = captures[1].to_string();
        if matches!(symbol.as_str(), "CHF" | "USD") {
            continue;
        }
        let scaled = parse_quantity(&captures[2])
            .map_err(|error| format!("Swissquote-PDF, Positionszeile {}: {error}", index + 1))?;
        if scaled.amount <= 0 {
            continue;
        }
        let quote_currency = quote_currency_after_quantity(
            &lines,
            index,
            captures.get(0).map(|value| value.end()).unwrap_or_default(),
            &currency_pattern,
        )
        .unwrap_or_else(|| "USD".into());
        positions.push(position_row(
            &symbol,
            &quantity_string(scaled),
            &category,
            &quote_currency,
            index + 1,
        )?);
    }
    if positions.is_empty() {
        return Err("Ein leeres PDF-Depot kann nicht sicher erkannt werden.".into());
    }
    validate_unique_symbols(&positions)?;
    let date_pattern =
        Regex::new(r"(?m)^(\d{2})/(\d{2})/(\d{4})\s+\d{2}:\d{2}:\d{2}\s+IBAN:\s*([A-Z0-9 ]+)")
            .map_err(|error| error.to_string())?;
    let (snapshot_date, account_reference) = if let Some(values) = date_pattern.captures(&text) {
        let date = NaiveDate::from_ymd_opt(
            values[3].parse().unwrap_or_default(),
            values[2].parse().unwrap_or_default(),
            values[1].parse().unwrap_or_default(),
        )
        .ok_or("Das Datum der Swissquote-Kontoübersicht ist ungültig.")?;
        (
            Some(date.to_string()),
            canonical_account_reference(&values[4]),
        )
    } else {
        let (_, fallback_date, _) = filename_metadata(path);
        (fallback_date, None)
    };
    Ok(Some(PositionSnapshot {
        provider: "swissquote".into(),
        reference_is_shared: true,
        scope: SnapshotScope::FullPortfolio,
        format: "PDF".into(),
        snapshot_date,
        account_reference,
        positions,
        warnings: vec!["PDF-Mengen wurden aus der dargestellten Swissquote-Positionstabelle gelesen. Bitte die Vorschau besonders sorgfältig prüfen.".into()],
    }))
}

fn quote_currency_after_quantity(
    lines: &[&str],
    row_index: usize,
    row_match_end: usize,
    currency_pattern: &Regex,
) -> Option<String> {
    lines
        .iter()
        .skip(row_index)
        .take(3)
        .enumerate()
        .find_map(|(offset, candidate)| {
            let candidate = if offset == 0 {
                candidate.get(row_match_end..)?
            } else {
                candidate
            };
            currency_pattern
                .captures(candidate)
                .map(|value| value[1].to_string())
        })
}

fn position_row(
    symbol: &str,
    quantity: &str,
    category: &str,
    quote_currency: &str,
    source_row: usize,
) -> Result<PositionSnapshotRow, String> {
    let symbol = symbol.trim().to_ascii_uppercase();
    if !Regex::new(r"^[A-Z0-9.\-]{2,15}$")
        .map_err(|error| error.to_string())?
        .is_match(&symbol)
    {
        return Err(format!("Swissquote-Symbol ist ungültig: {symbol}"));
    }
    let asset_type = category_asset_type(category).to_string();
    let quote_currency = if quote_currency.len() == 3 {
        quote_currency.to_ascii_uppercase()
    } else if asset_type == "crypto" {
        "USD".into()
    } else {
        String::new()
    };
    let market_symbol = if asset_type == "crypto" && !symbol.contains('-') {
        format!(
            "{symbol}-{}",
            if quote_currency.is_empty() {
                "USD"
            } else {
                &quote_currency
            }
        )
    } else {
        symbol.clone()
    };
    Ok(PositionSnapshotRow {
        isin: None,
        valor: None,
        symbol,
        market_symbol,
        quantity: quantity.into(),
        category: if category.trim().is_empty() {
            "Positionen".into()
        } else {
            category.trim().into()
        },
        asset_type,
        quote_currency,
        price_source: Some("yahoo".into()),
        source_row,
    })
}

fn category_asset_type(category: &str) -> &'static str {
    let value = normalize_label(category);
    if value.contains("krypto") {
        "crypto"
    } else if value.contains("aktie") {
        "stock"
    } else if value.contains("etf") || value.contains("fonds") {
        "fund"
    } else if value.contains("obligation") || value.contains("anleihe") {
        "bond"
    } else {
        "other"
    }
}

fn is_category_label(value: &str) -> bool {
    let value = normalize_label(value);
    value.contains("kryptowahrung")
        || value == "aktien"
        || value.contains("fonds")
        || value.contains("etf")
        || value.contains("obligation")
        || value.contains("anleihe")
}

fn filename_metadata(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let pattern =
        Regex::new(r"(?i)^Positions_(\d+)_(\d{4})_(\d{2})_(\d{2})(?:_\d{2}_\d{2})?").unwrap();
    if let Some(values) = pattern.captures(name) {
        if let Some(date) = NaiveDate::from_ymd_opt(
            values[2].parse().unwrap_or_default(),
            values[3].parse().unwrap_or_default(),
            values[4].parse().unwrap_or_default(),
        ) {
            return (Some(values[1].into()), Some(date.to_string()), None);
        }
    }
    (
        None,
        None,
        Some("Bitte den Stichtag des Positionsbestands angeben.".into()),
    )
}

pub fn canonical_account_reference(value: &str) -> Option<String> {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.starts_with("CH") && normalized.len() >= 10 {
        let digits = normalized
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>();
        if digits.len() >= 8 && digits.ends_with("00") {
            return Some(digits[digits.len() - 8..digits.len() - 2].into());
        }
    }
    Some(normalized)
}

fn validate_unique_symbols(positions: &[PositionSnapshotRow]) -> Result<(), String> {
    let mut symbols = BTreeSet::new();
    for position in positions {
        if !symbols.insert(position.market_symbol.clone()) {
            return Err(format!(
                "Das Swissquote-Dokument enthält das Symbol {} mehrfach.",
                position.symbol
            ));
        }
    }
    Ok(())
}

fn cell_text(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn normalize_label(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ä', "a")
        .replace('ö', "o")
        .replace('ü', "u")
        .replace('é', "e")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::securities::position_snapshots::ScaledQuantity;

    #[test]
    fn empty_excel_requires_explicit_zero_total_and_never_ignores_unidentified_rows() {
        let header =
            ["Symbol", "Anzahl", "Währung", "Totalwert CHF"].map(|v| Data::String(v.into()));
        let total = [
            Data::String("Gesamt".into()),
            Data::Empty,
            Data::Empty,
            Data::Int(0),
        ];
        let result = parse_excel_rows(
            Path::new("Positions_123456_2026_09_21.xlsx"),
            &[&header, &total],
        )
        .unwrap()
        .unwrap();
        assert!(result.positions.is_empty());
        assert_eq!(result.scope, SnapshotScope::FullPortfolio);
        let nonzero_total = [
            Data::String("Gesamt".into()),
            Data::Empty,
            Data::Empty,
            Data::Int(100),
        ];
        assert!(parse_excel_rows(Path::new("positions.xlsx"), &[&header, &nonzero_total]).is_err());
        let unidentified = [
            Data::Empty,
            Data::Int(2),
            Data::String("CHF".into()),
            Data::Int(0),
        ];
        assert!(parse_excel_rows(
            Path::new("positions.xlsx"),
            &[&header, &unidentified, &total]
        )
        .is_err());
    }

    #[test]
    fn excel_provider_returns_generic_dated_snapshot() {
        let header =
            ["Symbol", "Anzahl", "Währung", "Totalwert CHF"].map(|v| Data::String(v.into()));
        let row = [
            Data::String("AAA".into()),
            Data::String("1.123456789".into()),
            Data::String("CHF".into()),
            Data::Int(100),
        ];
        let total = [
            Data::String("Gesamt".into()),
            Data::Empty,
            Data::Empty,
            Data::Int(100),
        ];
        let result = parse_excel_rows(
            Path::new("Positions_123456_2026_09_21.xlsx"),
            &[&header, &row, &total],
        )
        .unwrap()
        .unwrap();
        result.validate().unwrap();
        assert_eq!(result.provider, "swissquote");
        assert_eq!(result.date().unwrap(), "2026-09-21");
        assert_eq!(result.positions[0].quantity, "1.123456789");
        assert!(parse_excel_rows(Path::new("positions.xlsx"), &[&header, &row]).is_err());
    }

    #[test]
    fn missing_date_never_defaults_to_today() {
        let (_, date, warning) = filename_metadata(Path::new("renamed.xlsx"));
        assert!(date.is_none());
        assert!(warning.is_some());
        let (reference, date, _) = filename_metadata(Path::new("Positions_123456_2026_09_21.xlsx"));
        assert_eq!(reference.as_deref(), Some("123456"));
        assert_eq!(date.as_deref(), Some("2026-09-21"));
    }

    #[test]
    fn unrelated_provider_workbook_is_not_a_position_snapshot() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/bank-statements/xlsx/ubs_kontoauszug_2026-08.xlsx");
        assert!(parse(&path).unwrap().is_none());
    }

    #[test]
    fn preserves_swissquote_quantity_precision() {
        let value = parse_quantity("503892.69507239").unwrap();
        assert_eq!(
            value,
            ScaledQuantity {
                amount: 50_389_269_507_239,
                scale: 8
            }
        );
        assert_eq!(quantity_string(value), "503892.69507239");
    }

    #[test]
    fn derives_account_number_from_swissquote_iban() {
        assert_eq!(
            canonical_account_reference("CH11 0878 1000 0326 2870 0").as_deref(),
            Some("326287")
        );
    }

    #[test]
    fn reads_quote_currency_after_crypto_symbol_and_quantity() {
        let row = "ADA 503892.69507239 0.51 USD 257000.12";
        let row_pattern =
            Regex::new(r"^([A-Z][A-Z0-9.\-]{1,14})\s+([0-9][0-9'\u{2019}.,]*)\s+").unwrap();
        let currency_pattern =
            Regex::new(r"\b(CHF|USD|EUR|GBP|CAD|AUD|JPY|CNY|HKD|SGD|SEK|NOK|DKK)\b").unwrap();
        let captures = row_pattern.captures(row).unwrap();
        assert_eq!(
            quote_currency_after_quantity(
                &[row],
                0,
                captures.get(0).unwrap().end(),
                &currency_pattern,
            )
            .as_deref(),
            Some("USD")
        );
    }
}
