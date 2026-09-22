//! Extrahiert PDF-Belegtexte und wendet Anbieterparser oder allgemeine Erkennungsregeln an.

use crate::importers::*;
use lopdf::Document;
use regex::Regex;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub(in crate::importers) fn parse(
    path: &Path,
    selected_provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let text = extract_pdf_text(path)?;
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

pub(in crate::importers) fn extract_pdf_text(path: &Path) -> Result<String, String> {
    match guarded_pdf_extract(|| pdf_extract::extract_text(path)) {
        Ok(text) => Ok(text),
        Err(primary_error) => extract_text_without_inline_images(path).map_err(|fallback_error| {
            format!("{primary_error} Alternativer PDF-Leser: {fallback_error}")
        }),
    }
}

/// Some older bank PDFs contain valid binary inline images that the text
/// parser rejects as an invalid content stream. Text extraction does not need
/// those images, so retry with only their BI..ID..EI blocks removed.
fn extract_text_without_inline_images(path: &Path) -> Result<String, String> {
    let mut document = Document::load(path)
        .or_else(|_| Document::load_with_password(path, ""))
        .map_err(|error| format!("PDF konnte nicht geöffnet werden: {error}"))?;
    let pages = document.get_pages();
    for page_id in pages.values() {
        let content = document
            .get_page_content(*page_id)
            .map_err(|error| format!("PDF-Seiteninhalt ist unlesbar: {error}"))?;
        let sanitized = strip_inline_images(&content)?;
        document
            .change_page_content(*page_id, sanitized)
            .map_err(|error| {
                format!("PDF-Seiteninhalt konnte nicht vorbereitet werden: {error}")
            })?;
    }
    document
        .extract_text(&pages.keys().copied().collect::<Vec<_>>())
        .map_err(|error| format!("PDF-Text konnte nicht gelesen werden: {error}"))
}

fn strip_inline_images(content: &[u8]) -> Result<Vec<u8>, String> {
    let mut sanitized = Vec::with_capacity(content.len());
    let mut cursor = 0;
    while let Some(start) = find_token(content, cursor, b"BI") {
        sanitized.extend_from_slice(&content[cursor..start]);
        let Some(data_marker) = find_token(content, start + 2, b"ID").filter(|marker| {
            *marker <= start + 1024
                && content[start + 2..*marker]
                    .iter()
                    .all(|byte| byte.is_ascii_graphic() || byte.is_ascii_whitespace())
                && content[start + 2..*marker]
                    .windows(2)
                    .any(|value| value == b"/W" || value == b"/H")
        }) else {
            sanitized.extend_from_slice(b"BI");
            cursor = start + 2;
            continue;
        };
        let end_marker = find_inline_image_end(content, data_marker + 2)
            .ok_or("Inline-Bild im PDF ist nicht vollständig.")?;
        sanitized.extend_from_slice(b"\n");
        cursor = end_marker + 2;
    }
    sanitized.extend_from_slice(&content[cursor..]);
    Ok(sanitized)
}

fn find_inline_image_end(content: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while let Some(candidate) = find_token(content, cursor, b"EI") {
        let next = content[candidate + 2..]
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .map(|offset| candidate + 2 + offset);
        if next.is_none_or(|index| {
            content[index] == b'Q'
                && (index + 1 == content.len() || content[index + 1].is_ascii_whitespace())
        }) {
            return Some(candidate);
        }
        cursor = candidate + 2;
    }
    None
}

fn find_token(content: &[u8], start: usize, token: &[u8]) -> Option<usize> {
    content
        .windows(token.len())
        .enumerate()
        .skip(start)
        .find_map(|(index, candidate)| {
            let before = index == 0 || is_pdf_delimiter(content[index - 1]);
            let after = index + token.len() == content.len()
                || is_pdf_delimiter(content[index + token.len()]);
            (candidate == token && before && after).then_some(index)
        })
}

fn is_pdf_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}

fn guarded_pdf_extract<E>(extract: impl FnOnce() -> Result<String, E>) -> Result<String, String>
where
    E: std::fmt::Display,
{
    match catch_unwind(AssertUnwindSafe(extract)) {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(error)) => Err(format!("PDF-Text konnte nicht gelesen werden: {error}")),
        Err(_) => Err("Der PDF-Text ist beschädigt oder verwendet ein nicht unterstütztes älteres Format. Die übrigen Dateien können weiterverarbeitet werden.".into()),
    }
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

#[cfg(test)]
mod tests {
    use super::{guarded_pdf_extract, strip_inline_images};

    #[test]
    fn pdf_extractor_panics_are_reported_as_file_errors() {
        let error = guarded_pdf_extract(|| -> Result<String, &'static str> {
            panic!("invalid content stream")
        })
        .unwrap_err();
        assert!(error.contains("nicht unterstütztes älteres Format"));
    }

    #[test]
    fn removes_binary_inline_images_without_touching_text_operations() {
        let content = b"BT (before) Tj ET\nq\nBI\n/W 48 /H 2 /BPC 1 /IM true\nID \0\0\x01\x02\0\0 EI\nQ\nBT (after) Tj ET";
        let sanitized = strip_inline_images(content).unwrap();
        let text = String::from_utf8(sanitized).unwrap();
        assert_eq!(text, "BT (before) Tj ET\nq\n\n\nQ\nBT (after) Tj ET");
    }
}
