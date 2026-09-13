use calamine::{open_workbook_auto, Data, DataType, Reader};
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
mod csv_import;
mod extended;
mod mt940;
use csv_import::parse_csv;

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
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "xlsx" | "xls" => parse_excel(path, selected_provider.as_deref()),
        "csv" => parse_csv(path, selected_provider.as_deref()),
        "mt940" | "sta" => mt940::parse(path, selected_provider.as_deref()),
        "pdf" => parse_pdf(path, selected_provider.as_deref()),
        _ => Err("Unterstützt werden XLSX, XLS, CSV, PDF und MT940.".to_string()),
    }
}

fn parse_excel(path: &Path, selected_provider: Option<&str>) -> Result<ParsedStatement, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Excel-Datei konnte nicht geöffnet werden: {error}"))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("Die Arbeitsmappe enthält kein Tabellenblatt.")?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|error| format!("Tabellenblatt konnte nicht gelesen werden: {error}"))?;
    let rows: Vec<&[Data]> = range.rows().collect();
    let provider =
        normalize_provider(selected_provider.unwrap_or_else(|| detect_provider_in_cells(&rows)));
    if provider == "unknown" {
        return Err(
            "Der Anbieter konnte nicht erkannt werden. Bitte im vorherigen Schritt auswählen."
                .to_string(),
        );
    }
    let header_row = rows
        .iter()
        .position(|row| row.iter().any(|cell| is_date_header(&cell.to_string())))
        .ok_or("Keine unterstützte Buchungstabelle gefunden.")?;
    let headers: Vec<String> = rows[header_row]
        .iter()
        .map(|cell| normalized(&cell.to_string()))
        .collect();
    let date_index = find_header(&headers, &["buchungsdatum", "buchungstag", "datum"])?;
    let value_date_index = find_header_optional(&headers, &["valutadatum", "valuta"]);
    let description_index = find_header(
        &headers,
        &[
            "beschreibung",
            "informationen",
            "text",
            "buchungstext",
            "vorgang",
        ],
    )?;
    let debit_index = find_header_optional(&headers, &["belastung", "soll chf", "soll"]);
    let credit_index = find_header_optional(&headers, &["gutschrift", "haben chf", "haben"]);
    let amount_index = find_header_optional(&headers, &["betrag", "betrag chf"]);
    let balance_index = find_header_optional(
        &headers,
        &["saldo", "saldo chf", "kontostand", "vertragswert"],
    );
    let currency_index = find_header_optional(&headers, &["währung", "waehrung"]);
    let industry_index = find_header_optional(&headers, &["branche", "industry"]);
    let account_name = rows
        .get(1)
        .and_then(|row| row.first())
        .map(|cell| {
            cell.to_string()
                .split('|')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Neues Konto".to_string());
    let mut transactions = Vec::new();
    let mut opening_balance = None;
    let mut closing_balance = None;
    let mut warnings = Vec::new();
    for (row_number, row) in rows.iter().enumerate().skip(header_row + 1) {
        let description = cell_string(row, description_index);
        if description.is_empty() {
            continue;
        }
        let balance = balance_index.and_then(|index| cell_money(row, index));
        let description_key = normalized(&description);
        if is_opening_label(&description_key) {
            opening_balance = balance;
            continue;
        }
        if is_closing_label(&description_key) {
            closing_balance = balance.or(closing_balance);
            continue;
        }
        let Some(booking_date) = cell_date(row, date_index) else {
            warnings.push(format!(
                "Zeile {}: Datum konnte nicht gelesen werden.",
                row_number + 1
            ));
            continue;
        };
        let amount = if let Some(index) = amount_index {
            cell_money(row, index).unwrap_or(0)
        } else {
            let debit = debit_index
                .and_then(|index| cell_money(row, index))
                .unwrap_or(0);
            let credit = credit_index
                .and_then(|index| cell_money(row, index))
                .unwrap_or(0);
            credit - debit
        };
        let currency = currency_index
            .map(|index| cell_string(row, index))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "CHF".to_string());
        transactions.push(ParsedTransaction {
            booking_date,
            value_date: value_date_index.and_then(|index| cell_date(row, index)),
            description,
            industry: industry_index
                .map(|index| cell_string(row, index))
                .filter(|value| !value.is_empty()),
            amount_minor: amount,
            balance_minor: balance,
            currency,
            confidence: 1.0,
            source_row: row_number + 1,
        });
    }
    closing_balance =
        closing_balance.or_else(|| transactions.last().and_then(|row| row.balance_minor));
    if transactions.is_empty() {
        return Err("Es wurden keine Buchungen gefunden.".to_string());
    }
    Ok(ParsedStatement {
        currency_balances: Vec::new(),
        account_type: None,
        provider,
        format: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("xlsx")
            .to_ascii_uppercase(),
        account_name,
        transactions,
        opening_balance_minor: opening_balance,
        closing_balance_minor: closing_balance,
        warnings,
    })
}

fn parse_pdf(path: &Path, selected_provider: Option<&str>) -> Result<ParsedStatement, String> {
    let text = pdf_extract::extract_text(path)
        .map_err(|error| format!("PDF-Text konnte nicht gelesen werden: {error}"))?;
    let clean_lines: Vec<String> = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect();
    if text.contains("Swissquote Bank") {
        return extended::swissquote(&clean_lines);
    }
    if text.contains("Kreditkartenabrechnung in CHF") && text.contains("UBS") {
        return extended::mastercard(&clean_lines);
    }
    let provider = normalize_provider(
        selected_provider
            .filter(|value| *value != "unknown")
            .unwrap_or_else(|| detect_provider(&text)),
    );
    if provider == "unknown" {
        return Err(
            "Der Anbieter konnte im PDF nicht erkannt werden. Bitte manuell auswählen.".to_string(),
        );
    }
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let account_name = lines
        .iter()
        .find(|line| line.contains("IBAN") || line.contains("Police") || line.contains("Konto MB-"))
        .and_then(|line| line.split('|').next())
        .unwrap_or("Neues Konto")
        .trim()
        .to_string();
    let (transactions, opening_balance, closing_balance) = parse_pdf_rows(&lines, &provider)?;
    Ok(ParsedStatement { currency_balances: Vec::new(), account_type: None, provider, format: "PDF".to_string(), account_name, transactions, opening_balance_minor: opening_balance, closing_balance_minor: closing_balance, warnings: vec!["PDF-Erkennung ist heuristisch. Bitte Datum, Betrag und Saldo vor dem Import kontrollieren.".to_string()] })
}

fn parse_pdf_rows(lines: &[String], provider: &str) -> Result<ParsedPdfRows, String> {
    if provider == "ubs"
        && lines
            .iter()
            .any(|line| line.starts_with("Ihr Konto auf einen Blick"))
    {
        return parse_ubs_statement(lines);
    }
    let footer_index = lines
        .iter()
        .position(|line| line.starts_with("Dieses Dokument wurde"))
        .unwrap_or(lines.len());
    let table_start = lines
        .iter()
        .position(|line| {
            let value = normalized(line);
            (value.contains("datum") || value.contains("buchungstag"))
                && (value.contains("saldo")
                    || value.contains("kontostand")
                    || value.contains("vertragswert"))
        })
        .ok_or("Keine Buchungstabelle im PDF gefunden.")?;
    let data = &lines[table_start + 1..footer_index];
    let money = r"[+-]?\d[\d'’]*(?:[.,]\d{2})";
    let row_pattern = match provider {
        "ubs" => format!(
            r"^(\d{{2}}\.\d{{2}}\.\d{{2}}) (.*?) (?:({money}) )?(\d{{2}}\.\d{{2}}\.\d{{2}}) ({money})$"
        ),
        "raiffeisen" => format!(
            r"^(\d{{2}}\.\d{{2}}\.\d{{4}}) (\d{{2}}\.\d{{2}}\.\d{{4}}) (.*?) ({money}) ({money})$"
        ),
        _ => format!(r"^(\d{{2}}\.\d{{2}}\.\d{{4}}) (.*?) (?:({money}) )?({money})$"),
    };
    let row_regex = Regex::new(&row_pattern)
        .map_err(|_| "PDF-Erkennung konnte nicht initialisiert werden.".to_string())?;
    let mut parsed = Vec::new();
    let mut opening = None;
    let mut closing = None;
    for (offset, line) in data.iter().enumerate() {
        let Some(captures) = row_regex.captures(line) else {
            continue;
        };
        let (booking_raw, value_date, description, displayed_amount, balance) = if provider == "ubs"
        {
            (
                captures.get(1),
                captures
                    .get(4)
                    .and_then(|value| normalize_date(value.as_str())),
                captures.get(2),
                captures.get(3),
                captures.get(5),
            )
        } else if provider == "raiffeisen" {
            (
                captures.get(1),
                captures
                    .get(2)
                    .and_then(|value| normalize_date(value.as_str())),
                captures.get(3),
                captures.get(4),
                captures.get(5),
            )
        } else {
            (
                captures.get(1),
                None,
                captures.get(2),
                captures.get(3),
                captures.get(4),
            )
        };
        let Some(booking_date) = booking_raw.and_then(|value| normalize_date(value.as_str()))
        else {
            continue;
        };
        let description = description
            .map(|value| value.as_str().trim().to_string())
            .unwrap_or_default();
        let Some(balance_minor) = balance.and_then(|value| parse_money(value.as_str())) else {
            continue;
        };
        let description_key = normalized(&description);
        if is_opening_label(&description_key) {
            opening = Some(balance_minor);
            continue;
        }
        if is_closing_label(&description_key) {
            closing = Some(balance_minor);
            continue;
        }
        let signed_amount = displayed_amount
            .and_then(|value| parse_money(value.as_str()))
            .map(|amount| signed_pdf_amount(provider, &description_key, amount))
            .unwrap_or(0);
        parsed.push(ParsedTransaction {
            booking_date,
            value_date,
            description,
            industry: None,
            amount_minor: signed_amount,
            balance_minor: Some(balance_minor),
            currency: "CHF".to_string(),
            confidence: 0.82,
            source_row: table_start + offset + 2,
        });
    }
    if parsed.is_empty() {
        return Err("Im PDF konnten keine Buchungszeilen erkannt werden.".to_string());
    }
    let final_balance = closing.or_else(|| parsed.last().and_then(|row| row.balance_minor));
    Ok((parsed, opening, final_balance))
}

// UBS monthly statements have a balance on every booking. Its change determines
// the sign without guessing from transaction labels.
fn parse_ubs_statement(lines: &[String]) -> Result<ParsedPdfRows, String> {
    let money = r"[+-]?\d+(?:[ '’]\d{3})*[.,]\d{2}";
    let row = Regex::new(&format!(
        r"^(\d{{2}}\.\d{{2}}\.\d{{2}}) (.*?) ({money}) (\d{{2}}\.\d{{2}}\.\d{{2}}) ({money})$"
    ))
    .unwrap();
    let balance_row = Regex::new(&format!(
        r"^\d{{2}}\.\d{{2}}\.\d{{2}} (Anfangssaldo|Schlusssaldo) ({money})$"
    ))
    .unwrap();
    let dated = Regex::new(r"^\d{2}\.\d{2}\.\d{2}(?: |$)").unwrap();
    let summary = |label: &str| -> Result<i64, String> {
        lines
            .iter()
            .find_map(|line| line.strip_prefix(label).and_then(parse_money))
            .ok_or_else(|| format!("UBS-Auszug: {label} fehlt oder ist unlesbar."))
    };
    let opening = summary("Anfangssaldo ")?;
    let closing = summary("Schlusssaldo ")?;
    let credits = summary("Total Gutschriften ")?;
    let debits = summary("Total Belastungen ")?;
    let mut previous = opening;
    let mut transactions: Vec<ParsedTransaction> = Vec::new();
    let mut in_table = false;
    let mut complete = false;
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("Datum Informationen") && line.contains("Kontostand") {
            in_table = true;
            continue;
        }
        // pdf-extract may emit page headers after the last booking on a page.
        if line.contains("GNZKOA") || line == "aUBS" || line.contains("Formular ohne Unterschrift")
        {
            in_table = false;
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(capture) = balance_row.captures(line) {
            let balance = parse_money(&capture[2]).ok_or("UBS-Saldo unlesbar.")?;
            if &capture[1] == "Schlusssaldo" {
                if balance != closing {
                    return Err("UBS-Schlusssalden stimmen nicht überein.".into());
                }
                complete = true;
                break;
            }
            if balance != opening {
                return Err("UBS-Anfangssalden stimmen nicht überein.".into());
            }
            continue;
        }
        if line.starts_with("Umsatztotal") {
            continue;
        }
        if let Some(capture) = row.captures(line) {
            let balance = parse_money(&capture[5]).ok_or("UBS-Kontostand unlesbar.")?;
            let amount = parse_money(&capture[3]).ok_or("UBS-Betrag unlesbar.")?;
            let change = balance - previous;
            if change.abs() != amount.abs() {
                return Err(format!("UBS-Auszug: Betrag und Saldo passen in Zeile {} nicht zusammen. Import abgebrochen.", index + 1));
            }
            transactions.push(ParsedTransaction {
                booking_date: normalize_date(&capture[1]).ok_or("Ungültiges UBS-Buchungsdatum.")?,
                value_date: Some(normalize_date(&capture[4]).ok_or("Ungültige UBS-Valuta.")?),
                description: capture[2].to_string(),
                industry: None,
                amount_minor: change,
                balance_minor: Some(balance),
                currency: "CHF".into(),
                confidence: 0.99,
                source_row: index + 1,
            });
            previous = balance;
        } else if dated.is_match(line) {
            return Err(format!(
                "UBS-Buchungszeile {} konnte nicht vollständig gelesen werden.",
                index + 1
            ));
        } else if let Some(transaction) = transactions.last_mut() {
            transaction.description.push('\n');
            transaction.description.push_str(line);
        }
    }
    let actual_credits: i64 = transactions.iter().map(|row| row.amount_minor.max(0)).sum();
    let actual_debits: i64 = transactions
        .iter()
        .map(|row| (-row.amount_minor).max(0))
        .sum();
    if !complete
        || transactions.is_empty()
        || previous != closing
        || actual_credits != credits
        || actual_debits != debits
    {
        return Err(
            "UBS-Auszug unvollständig: Buchungen, Summen oder Schlusssaldo stimmen nicht überein."
                .into(),
        );
    }
    Ok((transactions, Some(opening), Some(closing)))
}

fn signed_pdf_amount(provider: &str, description: &str, amount: i64) -> i64 {
    if amount < 0 {
        return amount;
    }
    let positive = match provider {
        "ubs" => description.contains("lohn") || description.contains("gutschrift"),
        "migros" => {
            description.contains("eingang")
                || description.contains("gutschrift")
                || description.contains("zins")
        }
        "generali" => {
            description.contains("pramienzahlung") || description.contains("wertentwicklung")
        }
        _ => true,
    };
    if positive {
        amount
    } else {
        -amount
    }
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
    let value = normalized(value);
    if value.contains("ubs") {
        "ubs"
    } else if value.contains("migros") {
        "migros"
    } else if value.contains("raiffeisen") {
        "raiffeisen"
    } else if value.contains("generali") {
        "generali"
    } else {
        "unknown"
    }
}
fn normalize_provider(value: &str) -> String {
    match normalized(value).as_str() {
        value if value.contains("ubs") => "ubs",
        value if value.contains("migros") => "migros",
        value if value.contains("raiffeisen") => "raiffeisen",
        value if value.contains("generali") => "generali",
        _ => "unknown",
    }
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
            let parsed = parse_statement(fixture(path), None)
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
        let parsed = parse_statement(path.to_string_lossy().into_owned(), None)
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
            let parsed = parse_statement(fixture(path), None)
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
            parse_pdf_rows(&text.lines().map(str::to_string).collect::<Vec<_>>(), "ubs")
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
