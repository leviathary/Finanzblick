//! Extrahiert PDF-Belegtexte und wendet Anbieterparser oder allgemeine Erkennungsregeln an.

use crate::importers::*;
use regex::Regex;

pub(in crate::importers) fn parse(
    path: &Path,
    selected_provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let text = pdf_extract::extract_text(path)
        .map_err(|error| format!("PDF-Text konnte nicht gelesen werden: {error}"))?;
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
    if let Some(result) =
        providers::by_id(&provider).and_then(|importer| importer.parse_pdf(path, &text))
    {
        return result;
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
    Ok(ParsedStatement { currency_balances: Vec::new(), account_type: None, provider, format: "PDF".to_string(), account_name, transactions, opening_balance_minor: opening_balance, closing_balance_minor: closing_balance, warnings: vec!["PDF-Erkennung ist heuristisch. Bitte Datum, Betrag und Saldo vor dem Import kontrollieren.".to_string()], ..ParsedStatement::default() })
}

fn parse_pdf_rows(lines: &[String], provider: &str) -> Result<ParsedPdfRows, String> {
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
        let (booking_raw, value_date, description, displayed_amount, balance) =
            if provider == "raiffeisen" {
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
            ..ParsedTransaction::default()
        });
    }
    if parsed.is_empty() {
        return Err("Im PDF konnten keine Buchungszeilen erkannt werden.".to_string());
    }
    let final_balance = closing.or_else(|| parsed.last().and_then(|row| row.balance_minor));
    Ok((parsed, opening, final_balance))
}

fn signed_pdf_amount(provider: &str, description: &str, amount: i64) -> i64 {
    if amount < 0 {
        return amount;
    }
    let positive = match provider {
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
