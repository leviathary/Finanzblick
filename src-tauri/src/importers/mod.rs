use calamine::{Data, DataType};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
mod csv_import;
mod excel;
mod extended;
mod mt940;
mod pdf;
mod providers;
mod registry;
mod tabular;
use csv_import::parse_csv;
pub use tabular::{inspect_tabular_file, TabularInspection, TabularMapping};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyBalance {
    pub currency: String,
    #[serde(default)]
    pub opening_date: Option<String>,
    pub opening_balance_minor: i64,
    pub closing_balance_minor: i64,
    pub closing_date: String,
}

const MAX_FILE_SIZE: u64 = 25 * 1024 * 1024;
type ParsedPdfRows = (Vec<ParsedTransaction>, Option<i64>, Option<i64>);

/// Column aliases are owned by a provider importer; the Excel reader only
/// applies the selected provider's mapping to the workbook.
pub(super) struct ProviderExcelMapping {
    pub date: &'static [&'static str],
    pub value_date: &'static [&'static str],
    pub description: &'static [&'static str],
    pub debit: &'static [&'static str],
    pub credit: &'static [&'static str],
    pub amount: &'static [&'static str],
    pub balance: &'static [&'static str],
    pub currency: &'static [&'static str],
    pub industry: &'static [&'static str],
}

static DEFAULT_EXCEL_MAPPING: ProviderExcelMapping = ProviderExcelMapping {
    date: &["buchungsdatum", "buchungstag", "datum"],
    value_date: &["valutadatum", "valuta"],
    description: &[
        "beschreibung",
        "informationen",
        "text",
        "buchungstext",
        "vorgang",
    ],
    debit: &["belastung", "soll chf", "soll"],
    credit: &["gutschrift", "haben chf", "haben"],
    amount: &["betrag", "betrag chf"],
    balance: &["saldo", "saldo chf", "kontostand", "vertragswert"],
    currency: &["währung", "waehrung"],
    industry: &["branche", "industry"],
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTransaction {
    pub booking_date: String,
    pub value_date: Option<String>,
    pub description: String,
    #[serde(default)]
    pub industry: Option<String>,
    pub amount_minor: i64,
    pub balance_minor: Option<i64>,
    pub currency: String,
    pub confidence: f32,
    pub source_row: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedStatement {
    #[serde(default)]
    pub currency_balances: Vec<CurrencyBalance>,
    #[serde(default)]
    pub account_type: Option<String>,
    pub provider: String,
    pub format: String,
    pub account_name: String,
    pub transactions: Vec<ParsedTransaction>,
    pub opening_balance_minor: Option<i64>,
    pub closing_balance_minor: Option<i64>,
    pub warnings: Vec<String>,
}

pub fn parse_statement(
    path: String,
    selected_provider: Option<String>,
    mapping: Option<TabularMapping>,
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
    registry::parse(path, selected_provider.as_deref(), mapping.as_ref())
}

fn detect_provider_in_cells(rows: &[&[Data]]) -> &'static str {
    for row in rows.iter().take(5) {
        for cell in row.iter().take(3) {
            let provider = detect_provider(&cell.to_string());
            if provider != "unknown" {
                return provider;
            }
        }
    }
    "unknown"
}
fn detect_provider(value: &str) -> &'static str {
    providers::detect(value)
        .map(|provider| provider.id())
        .unwrap_or("unknown")
}
fn normalize_provider(value: &str) -> String {
    providers::by_id(value)
        .map(|provider| provider.id())
        .unwrap_or("unknown")
        .to_string()
}
fn normalized(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['ä', 'á', 'à'], "a")
        .replace(['ö', 'ó', 'ò'], "o")
        .replace(['ü', 'ú', 'ù'], "u")
}
fn is_date_header(value: &str) -> bool {
    matches!(
        normalized(value).as_str(),
        "datum" | "buchungsdatum" | "buchungstag"
    )
}
fn find_header(headers: &[String], names: &[&str]) -> Result<usize, String> {
    find_header_optional(headers, names)
        .ok_or_else(|| format!("Benötigte Spalte fehlt: {}", names.join(" / ")))
}
fn find_header_optional(headers: &[String], names: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|header| names.iter().any(|name| header == &normalized(name)))
}
fn cell_string(row: &[Data], index: usize) -> String {
    row.get(index)
        .map(ToString::to_string)
        .unwrap_or_default()
        .trim()
        .to_string()
}
fn cell_money(row: &[Data], index: usize) -> Option<i64> {
    row.get(index)
        .and_then(|cell| cell.as_f64())
        .map(|value| (value * 100.0).round() as i64)
        .or_else(|| {
            row.get(index)
                .and_then(|cell| parse_money(&cell.to_string()))
        })
}
fn cell_date(row: &[Data], index: usize) -> Option<String> {
    row.get(index)
        .and_then(|cell| cell.as_date())
        .map(|date| date.format("%Y-%m-%d").to_string())
        .or_else(|| {
            row.get(index)
                .and_then(|cell| normalize_date(&cell.to_string()))
        })
}
fn parse_money(value: &str) -> Option<i64> {
    let cleaned = value.trim().replace(['\'', '’', ' '], "").replace(',', ".");
    if cleaned.is_empty() {
        None
    } else {
        cleaned
            .parse::<f64>()
            .ok()
            .map(|number| (number * 100.0).round() as i64)
    }
}
fn normalize_date(value: &str) -> Option<String> {
    let value = value.trim();
    if let Ok(date) = NaiveDate::parse_from_str(value, "%d.%m.%y") {
        return Some(date.format("%Y-%m-%d").to_string());
    }
    ["%Y-%m-%d", "%d.%m.%Y"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(value, format).ok())
        .map(|date| date.format("%Y-%m-%d").to_string())
}
fn is_opening_label(value: &str) -> bool {
    value.contains("anfangssaldo")
        || value.contains("saldovortrag")
        || value.contains("anfangswert")
}
fn is_closing_label(value: &str) -> bool {
    value.contains("schlusssaldo") || value == "vertragswert"
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(relative: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures")
            .join("bank-statements")
            .join(relative)
            .to_string_lossy()
            .into_owned()
    }
    #[test]
    fn parses_all_excel_fixtures() {
        let cases = [
            ("xlsx/ubs_kontoauszug_2026-08.xlsx", "ubs", 8),
            ("xlsx/migros_bank_konto_2026-08.xlsx", "migros", 5),
            (
                "xlsx/raiffeisen_transaktionen_2026-08.xlsx",
                "raiffeisen",
                6,
            ),
            ("xlsx/generali_vorsorge_2026.xlsx", "generali", 7),
        ];
        for (path, provider, count) in cases {
            let parsed = parse_statement(fixture(path), None, None)
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            assert_eq!(parsed.provider, provider);
            assert_eq!(parsed.transactions.len(), count, "{path}");
            assert!(parsed.closing_balance_minor.is_some(), "{path}");
        }
    }
    #[test]
    fn parses_long_term_wealth_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("outputs")
            .join("wealth-history-test")
            .join("ubs_langzeit_konto_2020-2026.xlsx");
        let parsed = parse_statement(path.to_string_lossy().into_owned(), None, None)
            .expect("parse long-term workbook");
        assert_eq!(parsed.provider, "ubs");
        assert_eq!(parsed.transactions.len(), 599);
        assert_eq!(parsed.opening_balance_minor, Some(2_500_000));
        assert_eq!(parsed.closing_balance_minor, Some(19_533_000));
        assert_eq!(
            parsed.transactions.first().unwrap().booking_date,
            "2020-01-02"
        );
        assert_eq!(
            parsed.transactions.last().unwrap().booking_date,
            "2026-08-31"
        );
    }
    #[test]
    fn parses_all_pdf_fixtures() {
        let cases = [
            ("pdf/ubs_kontoauszug_2026-08.pdf", "ubs", 8),
            ("pdf/migros_bank_konto_2026-08.pdf", "migros", 5),
            ("pdf/raiffeisen_transaktionen_2026-08.pdf", "raiffeisen", 6),
            ("pdf/generali_vorsorge_2026.pdf", "generali", 7),
        ];
        for (path, provider, count) in cases {
            let parsed = parse_statement(fixture(path), None, None)
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            assert_eq!(parsed.provider, provider);
            assert_eq!(parsed.transactions.len(), count, "{path}");
            assert!(parsed.closing_balance_minor.is_some(), "{path}");
        }
    }
    #[test]
    fn ubs_monthly_layout_preserves_details_and_rejects_incomplete_imports() {
        let text = "Ihr Konto auf einen Blick
Anfangssaldo 1 000.00
Total Gutschriften 200.00
Total Belastungen 23.00
Schlusssaldo 1 177.00
Datum Informationen Belastungen Gutschriften Valuta Kontostand
01.08.23 Anfangssaldo 1 000.00
02.08.23 STORNO UBS TWINT 200.00 01.08.23 1 200.00
TEST SHOP
Referenz 123
31.08.2023GNZKOA01 / TEST
Kontoinhaber
aUBS
Datum Informationen Belastungen Gutschriften Valuta Kontostand
31.08.23 SALDO DIENSTLEISTUNGSPREISABSCHLUSS 23.00 31.08.23 1 177.00
Umsatztotal 23.00 200.00
31.08.23 Schlusssaldo 1 177.00
Dienstleistungspreisabschluss
31.08.23 KEINE BUCHUNG 23.00 31.08.23 1 154.00";
        let parse = |text: &str| {
            providers::ubs::parse_account_rows(
                &text.lines().map(str::to_string).collect::<Vec<_>>(),
            )
        };
        let (rows, opening, closing) = parse(text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(opening, Some(100_000));
        assert_eq!(closing, Some(117_700));
        assert_eq!(rows[0].amount_minor, 20_000);
        assert_eq!(rows[0].booking_date, "2023-08-02");
        assert_eq!(rows[0].value_date.as_deref(), Some("2023-08-01"));
        assert_eq!(
            rows[0].description,
            "STORNO UBS TWINT\nTEST SHOP\nReferenz 123"
        );
        assert_eq!(rows[1].amount_minor, -2_300);
        assert!(parse(&text.replace("TWINT 200.00", "TWINT 199.00")).is_err());
        assert!(
            parse(&text.replace("Total Gutschriften 200.00", "Total Gutschriften 201.00")).is_err()
        );
        assert!(parse(&text.replace("31.08.23 Schlusssaldo 1 177.00", "")).is_err());
        assert!(parse(&text.replace("02.08.23 STORNO", "32.08.23 STORNO")).is_err());
    }
}
