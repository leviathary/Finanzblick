//! Extrahiert PDF-Belegtexte und wendet Anbieterparser oder allgemeine Erkennungsregeln an.

use crate::importers::formats::tabular::{
    parse_mapped_date, parse_mapped_money, DateFormat, NumberFormat,
};
use crate::importers::*;
use lopdf::Document;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};

const PDF_PREVIEW_ROWS: usize = 40;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfMapping {
    pub date_index: usize,
    pub value_date_index: Option<usize>,
    pub description_source: PdfTextSource,
    #[serde(default)]
    pub additional_description_sources: Vec<PdfTextSource>,
    pub amount_index: usize,
    pub balance_index: Option<usize>,
    #[serde(default = "default_pdf_currency")]
    pub fixed_currency: String,
    #[serde(default)]
    pub amount_sign: PdfAmountSign,
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default)]
    pub number_format: NumberFormat,
    #[serde(default)]
    pub ignored_descriptions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdfTextSource {
    Before,
    #[default]
    Inline,
    After,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdfAmountSign {
    #[default]
    Signed,
    Debit,
    Credit,
    InferFromBalance,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInspection {
    pub row_count: usize,
    pub max_date_count: usize,
    pub max_money_count: usize,
    pub layout_fingerprint: String,
    pub preview: Vec<PdfInspectionRow>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInspectionRow {
    pub source_row: usize,
    pub text_before: String,
    pub text_inline: String,
    pub text_after: String,
    pub dates: Vec<String>,
    pub amounts: Vec<String>,
}

fn default_pdf_currency() -> String {
    "CHF".into()
}

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
        providers::by_id(&provider).and_then(|importer| providers::parse_pdf(importer, path, &text))
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

pub(in crate::importers) fn inspect(path: &Path) -> Result<PdfInspection, String> {
    ensure_pdf(path)?;
    let text = extract_pdf_text(path)?;
    let rows = normalize_mapping_rows(extract_mapping_rows(&text)?);
    if rows.is_empty() {
        return Err("Im PDF wurden keine Zeilen mit Datum und Betrag erkannt.".into());
    }
    let max_date_count = rows.iter().map(|row| row.dates.len()).max().unwrap_or(0);
    let max_money_count = rows.iter().map(|row| row.amounts.len()).max().unwrap_or(0);
    let extracted_row_count = rows.len();
    let booking_rows = rows
        .into_iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (!is_edge_balance_row(index, extracted_row_count, &row, max_money_count)).then_some(row)
        })
        .collect::<Vec<_>>();
    if booking_rows.is_empty() {
        return Err("Im PDF wurden keine Buchungszeilen erkannt.".into());
    }
    let row_shapes = booking_rows
        .iter()
        .take(16)
        .map(|row| {
            format!(
                "{}{}{}{}{}",
                row.dates.len(),
                row.amounts.len(),
                (!row.text_before.is_empty()) as u8,
                (!row.text_inline.is_empty()) as u8,
                (!row.text_after.is_empty()) as u8,
            )
        })
        .collect::<Vec<_>>()
        .join("-");
    let layout_fingerprint = format!("pdf-v1:d{max_date_count}:m{max_money_count}:{row_shapes}");
    Ok(PdfInspection {
        row_count: booking_rows.len(),
        max_date_count,
        max_money_count,
        layout_fingerprint,
        preview: booking_rows.into_iter().take(PDF_PREVIEW_ROWS).collect(),
    })
}

fn is_edge_balance_row(
    index: usize,
    row_count: usize,
    row: &PdfInspectionRow,
    max_money_count: usize,
) -> bool {
    (index == 0 || index + 1 == row_count)
        && row.dates.len() == 1
        && row.amounts.len() == 1
        && max_money_count > row.amounts.len()
}

pub(in crate::importers) fn parse_mapped(
    path: &Path,
    selected_provider: Option<&str>,
    mapping: &PdfMapping,
) -> Result<ParsedStatement, String> {
    ensure_pdf(path)?;
    validate_pdf_mapping(mapping)?;
    let text = extract_pdf_text(path)?;
    let rows = normalize_mapping_rows(extract_mapping_rows(&text)?);
    let currency = normalize_pdf_currency(&mapping.fixed_currency)?;
    let inferred_rows = if matches!(mapping.amount_sign, PdfAmountSign::InferFromBalance) {
        resolve_inferred_pdf_rows(&rows, mapping)?
    } else {
        HashMap::new()
    };
    let mut transactions = Vec::new();
    let mut previous_balance = None;
    let mut opening_balance = None;
    let mut opening_date = None;
    let mut closing_balance = None;
    let mut closing_date = None;

    for row in rows {
        let date = row
            .dates
            .get(mapping.date_index)
            .and_then(|value| parse_mapped_date(value, &mapping.date_format));
        let value_date = mapping
            .value_date_index
            .and_then(|index| row.dates.get(index))
            .and_then(|value| parse_mapped_date(value, &mapping.date_format));
        let extracted_balance = mapping
            .balance_index
            .and_then(|index| row.amounts.get(index))
            .and_then(|value| parse_mapped_money(value, &mapping.number_format));
        let mapped_balance = inferred_rows
            .get(&row.source_row)
            .map(|(_, balance)| *balance)
            .or(extracted_balance);
        let amount = row
            .amounts
            .get(mapping.amount_index)
            .and_then(|value| parse_mapped_money(value, &mapping.number_format));

        if date.is_none()
            || amount.is_none()
            || (mapping.value_date_index.is_some() && value_date.is_none())
        {
            let leading_balance = mapped_balance.or_else(|| {
                (transactions.is_empty()
                    && previous_balance.is_none()
                    && matches!(mapping.amount_sign, PdfAmountSign::InferFromBalance)
                    && mapping.balance_index.is_some()
                    && mapping.value_date_index.is_some()
                    && value_date.is_none()
                    && row.amounts.len() == 1)
                    .then_some(amount)
                    .flatten()
            });
            if let Some(balance) = leading_balance {
                previous_balance = Some(balance);
                opening_balance.get_or_insert(balance);
                if opening_date.is_none() {
                    opening_date = date;
                }
            }
            continue;
        }
        let booking_date = date.expect("date checked");
        let description = mapped_pdf_description(&row, mapping);
        if description.is_empty()
            || ignored_pdf_description(&description, &mapping.ignored_descriptions)
        {
            if let Some(balance) = mapped_balance {
                previous_balance = Some(balance);
            }
            continue;
        }
        let unsigned = amount.expect("amount checked").abs();
        let amount_minor = match mapping.amount_sign {
            PdfAmountSign::Signed => amount.expect("amount checked"),
            PdfAmountSign::Debit => -unsigned,
            PdfAmountSign::Credit => unsigned,
            PdfAmountSign::InferFromBalance => inferred_rows
                .get(&row.source_row)
                .map(|(amount, _)| *amount)
                .ok_or_else(|| {
                    format!(
                        "PDF-Zeile {}: Betrag und laufender Saldo konnten nicht eindeutig abgestimmt werden.",
                        row.source_row
                    )
                })?,
        };
        if let Some(balance) = mapped_balance {
            if let Some(before) = previous_balance {
                if before + amount_minor != balance {
                    return Err(format!(
                        "PDF-Zeile {}: Der laufende Saldo passt nicht zum Betrag.",
                        row.source_row
                    ));
                }
            } else {
                opening_balance = Some(balance - amount_minor);
                opening_date = Some(booking_date.clone());
            }
            previous_balance = Some(balance);
            closing_balance = Some(balance);
            closing_date = Some(booking_date.clone());
        }
        transactions.push(ParsedTransaction {
            booking_date,
            value_date,
            description,
            amount_minor,
            balance_minor: mapped_balance,
            currency: currency.clone(),
            confidence: if mapping.balance_index.is_some() {
                1.0
            } else {
                0.82
            },
            source_row: row.source_row,
            ..ParsedTransaction::default()
        });
    }
    if transactions.is_empty() {
        return Err("Mit dieser PDF-Zuordnung wurden keine Buchungen erkannt.".into());
    }
    let currency_balances = match (
        opening_balance,
        opening_date.clone(),
        closing_balance,
        closing_date.clone(),
    ) {
        (Some(opening), Some(opening_date), Some(closing), Some(closing_date)) => {
            vec![CurrencyBalance {
                currency: currency.clone(),
                opening_date: Some(opening_date),
                opening_balance_minor: opening,
                closing_balance_minor: closing,
                closing_date,
            }]
        }
        _ => Vec::new(),
    };
    let account_reference = find_iban(&text);
    let provider = normalize_provider(selected_provider.unwrap_or("unknown"));
    let mut warnings = vec![
        "Generisches PDF-Profil verwendet. Bitte die Buchungsvorschau mit dem Originaldokument vergleichen."
            .into(),
    ];
    if mapping.balance_index.is_none() {
        warnings.push(
            "Ohne zugeordneten Saldo konnte die rechnerische Abstimmung nicht geprüft werden."
                .into(),
        );
    }
    Ok(ParsedStatement {
        currency_balances,
        account_type: Some("cash".into()),
        provider,
        format: "PDF".into(),
        account_name: account_reference
            .clone()
            .map(|reference| format!("PDF · {reference}"))
            .unwrap_or_else(|| "PDF-Konto".into()),
        transactions,
        opening_balance_minor: opening_balance,
        closing_balance_minor: closing_balance,
        warnings,
        account_reference,
        ..ParsedStatement::default()
    })
}

fn resolve_inferred_pdf_rows(
    rows: &[PdfInspectionRow],
    mapping: &PdfMapping,
) -> Result<HashMap<usize, (i64, i64)>, String> {
    let mut resolved = HashMap::new();
    let mut anchor = None;
    let mut pending = Vec::new();

    for row in rows {
        let date = row
            .dates
            .get(mapping.date_index)
            .and_then(|value| parse_mapped_date(value, &mapping.date_format));
        let value_date = mapping
            .value_date_index
            .and_then(|index| row.dates.get(index))
            .and_then(|value| parse_mapped_date(value, &mapping.date_format));
        let balance = mapping
            .balance_index
            .and_then(|index| row.amounts.get(index))
            .and_then(|value| parse_mapped_money(value, &mapping.number_format));
        let amount = row
            .amounts
            .get(mapping.amount_index)
            .and_then(|value| parse_mapped_money(value, &mapping.number_format));
        let is_transaction = date.is_some()
            && amount.is_some()
            && (mapping.value_date_index.is_none() || value_date.is_some());

        if is_transaction {
            pending.push((row.source_row, amount.expect("amount checked").abs()));
        }
        let Some(target_balance) = balance else {
            continue;
        };
        if pending.is_empty() {
            anchor = Some(target_balance);
            continue;
        }
        let start_balance = anchor.ok_or_else(|| {
            format!(
                "PDF-Zeile {}: Das Vorzeichen kann ohne vorherigen Saldo nicht abgeleitet werden.",
                row.source_row
            )
        })?;
        let signed_amounts = unique_signed_amounts(start_balance, target_balance, &pending)
            .ok_or_else(|| {
                format!(
                    "PDF-Zeile {}: Betrag und laufender Saldo konnten nicht eindeutig abgestimmt werden.",
                    row.source_row
                )
            })?;
        let mut running_balance = start_balance;
        for ((source_row, _), signed_amount) in pending.drain(..).zip(signed_amounts) {
            running_balance += signed_amount;
            resolved.insert(source_row, (signed_amount, running_balance));
        }
        anchor = Some(target_balance);
    }
    Ok(resolved)
}

fn unique_signed_amounts(
    start_balance: i64,
    target_balance: i64,
    pending: &[(usize, i64)],
) -> Option<Vec<i64>> {
    if pending.len() > 16 {
        return None;
    }
    let mut solution = None;
    let combinations = 1_u64 << pending.len();
    for mask in 0..combinations {
        let signed = pending
            .iter()
            .enumerate()
            .map(|(index, (_, amount))| {
                if amount == &0 || mask & (1 << index) != 0 {
                    *amount
                } else {
                    -*amount
                }
            })
            .collect::<Vec<_>>();
        if start_balance + signed.iter().sum::<i64>() != target_balance {
            continue;
        }
        if solution
            .as_ref()
            .is_some_and(|existing| existing != &signed)
        {
            return None;
        }
        solution = Some(signed);
    }
    solution
}

fn mapped_pdf_description(row: &PdfInspectionRow, mapping: &PdfMapping) -> String {
    std::iter::once(mapping.description_source)
        .chain(mapping.additional_description_sources.iter().copied())
        .filter_map(|source| {
            let value = match source {
                PdfTextSource::Before => row.text_before.trim(),
                PdfTextSource::Inline => row.text_inline.trim(),
                PdfTextSource::After => row.text_after.trim(),
            };
            (!value.is_empty()).then_some(value)
        })
        .fold(Vec::new(), |mut parts, value| {
            if !parts.contains(&value) {
                parts.push(value);
            }
            parts
        })
        .join("\n")
}

fn normalize_mapping_rows(rows: Vec<PdfInspectionRow>) -> Vec<PdfInspectionRow> {
    let max_date_count = rows.iter().map(|row| row.dates.len()).max().unwrap_or(0);
    let max_money_count = rows.iter().map(|row| row.amounts.len()).max().unwrap_or(0);
    let dominant_date_shape = rows
        .iter()
        .filter(|row| row.dates.len() == max_date_count && row.amounts.len() == max_money_count)
        .filter_map(|row| row.dates.first())
        .map(|date| {
            date.split(|character: char| !character.is_ascii_digit())
                .filter(|part| !part.is_empty())
                .map(str::len)
                .collect::<Vec<_>>()
        })
        .next();
    let row_count = rows.len();
    rows.into_iter()
        .enumerate()
        .filter_map(|(index, mut row)| {
            let edge = index == 0 || index + 1 == row_count;
            let date_shape = row.dates.first().map(|date| {
                date.split(|character: char| !character.is_ascii_digit())
                    .filter(|part| !part.is_empty())
                    .map(str::len)
                    .collect::<Vec<_>>()
            });
            if !edge
                && row.dates.len() < max_date_count
                && row.amounts.len() < max_money_count
                && date_shape != dominant_date_shape
            {
                return None;
            }
            if !edge
                && max_money_count == 2
                && row.dates.len() == max_date_count
                && row.amounts.len() == 1
            {
                row.amounts.insert(0, String::new());
            }
            Some(row)
        })
        .collect()
}

fn extract_mapping_rows(text: &str) -> Result<Vec<PdfInspectionRow>, String> {
    let date_pattern =
        Regex::new(r"(?x)\b(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[./-]\d{1,2}[./-]\d{2,4})\b")
            .map_err(|_| "PDF-Datumserkennung konnte nicht initialisiert werden.")?;
    let money_pattern =
        Regex::new(r"(?x)[-+]?(?:\d{1,3}(?:[\s'\u{2019}]\d{3})+|\d+)(?:[.,]\d{2})-?")
            .map_err(|_| "PDF-Betragserkennung konnte nicht initialisiert werden.")?;
    let mut rows: Vec<PdfInspectionRow> = Vec::new();
    let mut pending = Vec::new();
    for (line_index, source) in text.lines().enumerate() {
        let line = source.trim();
        if line.is_empty() {
            continue;
        }
        let date_matches = date_pattern.find_iter(line).collect::<Vec<_>>();
        let money_matches = money_pattern
            .find_iter(line)
            .filter(|amount| {
                !date_matches
                    .iter()
                    .any(|date| amount.start() < date.end() && date.start() < amount.end())
            })
            .collect::<Vec<_>>();
        if date_matches.is_empty() || money_matches.is_empty() {
            pending.push(line.to_string());
            continue;
        }
        // Text before the first structured row is the document preamble, not a
        // transaction description. Later pending lines belong to neighboring rows.
        let text_before = if rows.is_empty() {
            String::new()
        } else {
            pending.join("\n")
        };
        if let Some(previous) = rows.last_mut() {
            previous.text_after = text_before.clone();
        }
        pending.clear();
        let mut ranges = date_matches
            .iter()
            .map(|value| (value.start(), value.end()))
            .chain(
                money_matches
                    .iter()
                    .map(|value| (value.start(), value.end())),
            )
            .collect::<Vec<_>>();
        ranges.sort_unstable();
        let mut inline = String::with_capacity(line.len());
        let mut cursor = 0;
        for (start, end) in ranges {
            inline.push_str(&line[cursor..start]);
            inline.push(' ');
            cursor = end;
        }
        inline.push_str(&line[cursor..]);
        rows.push(PdfInspectionRow {
            source_row: line_index + 1,
            text_before,
            text_inline: inline.split_whitespace().collect::<Vec<_>>().join(" "),
            text_after: String::new(),
            dates: date_matches
                .iter()
                .map(|value| value.as_str().to_string())
                .collect(),
            amounts: money_matches
                .iter()
                .map(|value| value.as_str().to_string())
                .collect(),
        });
    }
    if let Some(last) = rows.last_mut() {
        last.text_after = pending.join("\n");
    }
    Ok(rows)
}

pub(crate) fn validate_pdf_mapping(mapping: &PdfMapping) -> Result<(), String> {
    if mapping.date_index >= 6
        || mapping.value_date_index.is_some_and(|index| index >= 6)
        || mapping.amount_index >= 6
        || mapping.balance_index.is_some_and(|index| index >= 6)
    {
        return Err("Die PDF-Zuordnung enthält einen ungültigen Feldindex.".into());
    }
    if mapping.balance_index == Some(mapping.amount_index) {
        return Err("Betrag und Saldo müssen verschiedenen PDF-Werten zugeordnet sein.".into());
    }
    let mut description_sources = vec![mapping.description_source];
    description_sources.extend(mapping.additional_description_sources.iter().copied());
    description_sources.sort_by_key(|source| match source {
        PdfTextSource::Before => 0,
        PdfTextSource::Inline => 1,
        PdfTextSource::After => 2,
    });
    description_sources.dedup();
    if mapping.additional_description_sources.len() > 2
        || description_sources.len() != mapping.additional_description_sources.len() + 1
    {
        return Err("Die PDF-Zuordnung der Beschreibung ist ungültig.".into());
    }
    if matches!(mapping.amount_sign, PdfAmountSign::InferFromBalance)
        && mapping.balance_index.is_none()
    {
        return Err("Für die Vorzeichenableitung muss ein Saldo zugeordnet sein.".into());
    }
    if mapping.ignored_descriptions.len() > 20
        || mapping
            .ignored_descriptions
            .iter()
            .any(|value| value.chars().count() > 80)
    {
        return Err("Es sind höchstens 20 Ausschlussbegriffe mit je 80 Zeichen erlaubt.".into());
    }
    normalize_pdf_currency(&mapping.fixed_currency)?;
    Ok(())
}

fn ignored_pdf_description(description: &str, ignored: &[String]) -> bool {
    let description = normalized(description);
    ignored.iter().any(|value| {
        let value = normalized(value);
        !value.is_empty() && description.contains(&value)
    })
}

fn normalize_pdf_currency(value: &str) -> Result<String, String> {
    let currency = value.trim().to_ascii_uppercase();
    if currency.len() == 3
        && currency
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        Ok(currency)
    } else {
        Err("Bitte eine dreistellige Währung wie CHF eingeben.".into())
    }
}

fn find_iban(text: &str) -> Option<String> {
    for line in text.lines() {
        let Some(position) = line.to_ascii_lowercase().find("iban") else {
            continue;
        };
        let tail = line.get(position + 4..).unwrap_or_default();
        let canonical = tail
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .map(|character| character.to_ascii_uppercase())
            .collect::<String>();
        for length in 15..=canonical.len().min(34) {
            let candidate = &canonical[..length];
            if valid_iban(candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn valid_iban(value: &str) -> bool {
    if value.len() < 15
        || value.len() > 34
        || !value[..2]
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        || !value[2..4]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return false;
    }
    value[4..]
        .chars()
        .chain(value[..4].chars())
        .try_fold(0_u32, |remainder, character| {
            let digits = if character.is_ascii_digit() {
                character.to_string()
            } else if character.is_ascii_alphabetic() {
                ((character.to_ascii_uppercase() as u8 - b'A') + 10).to_string()
            } else {
                return None;
            };
            Some(digits.bytes().fold(remainder, |value, digit| {
                (value * 10 + u32::from(digit - b'0')) % 97
            }))
        })
        == Some(1)
}

fn ensure_pdf(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|_| "Die ausgewählte Datei ist nicht mehr verfügbar.".to_string())?;
    if !metadata.is_file() {
        return Err("Bitte eine Datei und keinen Ordner auswählen.".into());
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err("Die Datei ist größer als 25 MB.".into());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err("Das PDF-Mapping unterstützt nur PDF-Dateien.".into());
    }
    Ok(())
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
    use super::{
        extract_mapping_rows, guarded_pdf_extract, is_edge_balance_row, mapped_pdf_description,
        normalize_mapping_rows, resolve_inferred_pdf_rows, strip_inline_images, valid_iban,
        validate_pdf_mapping, PdfAmountSign, PdfMapping, PdfTextSource,
    };
    use crate::importers::formats::tabular::{DateFormat, NumberFormat};

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

    #[test]
    fn extracts_language_neutral_pdf_mapping_fields_and_neighboring_text() {
        let rows = extract_mapping_rows(
            "Opening balance\n31.07.2026 1'000.00\nCoffee shop\n03.08.2026 03.08.2026 5.50 994.50\nReference 42",
        )
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].text_before.is_empty());
        assert_eq!(rows[0].text_after, "Coffee shop");
        assert_eq!(rows[1].dates, ["03.08.2026", "03.08.2026"]);
        assert_eq!(rows[1].amounts, ["5.50", "994.50"]);
        assert_eq!(rows[1].text_before, "Coffee shop");
        assert_eq!(rows[1].text_after, "Reference 42");
    }

    #[test]
    fn ignores_pdf_preamble_and_keeps_spaced_thousands_out_of_descriptions() {
        let rows = extract_mapping_rows(
            "Bank address\nIBAN CH36 0000 0000 0000 0000 0\nDatum Informationen Belastungen Valuta Kontostand\n01.01.23 Anfangssaldo 14 097.75\n02.01.23 ZAHLUNG UBS TWINT 270.95 31.12.22 13 826.80\nCOOP THUN",
        )
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].text_before.is_empty());
        assert_eq!(rows[0].text_inline, "Anfangssaldo");
        assert_eq!(rows[0].amounts, ["14 097.75"]);
        assert_eq!(rows[1].text_inline, "ZAHLUNG UBS TWINT");
        assert_eq!(rows[1].amounts, ["270.95", "13 826.80"]);
        assert_eq!(rows[1].text_after, "COOP THUN");
        assert!(is_edge_balance_row(0, rows.len(), &rows[0], 2));
        assert!(!is_edge_balance_row(1, rows.len(), &rows[1], 2));

        let mapping = PdfMapping {
            date_index: 0,
            value_date_index: Some(1),
            description_source: PdfTextSource::Inline,
            additional_description_sources: vec![PdfTextSource::After],
            amount_index: 0,
            balance_index: Some(1),
            fixed_currency: "CHF".into(),
            amount_sign: PdfAmountSign::InferFromBalance,
            date_format: DateFormat::Auto,
            number_format: NumberFormat::Auto,
            ignored_descriptions: Vec::new(),
        };
        assert_eq!(
            mapped_pdf_description(&rows[1], &mapping),
            "ZAHLUNG UBS TWINT\nCOOP THUN"
        );
    }

    #[test]
    fn aligns_single_transaction_amounts_and_removes_incomplete_page_metadata() {
        let rows = normalize_mapping_rows(
            extract_mapping_rows(
                "28.02.26 6 674.39 Kontostand\nLASTSCHRIFT SWISSCOM\n30.03.26 30.03.26 45.00\nSeite 1 / 2\n01.04.2026 00656 DE 000036.00\nLASTSCHRIFT RAIFFEISEN\n30.03.26 30.03.26 4 721.76 989.75\n31.03.26 4 716.76 Kontostand",
            )
            .unwrap(),
        );

        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1].dates, ["30.03.26", "30.03.26"]);
        assert_eq!(rows[1].amounts, ["", "45.00"]);
        assert_eq!(rows[2].amounts, ["4 721.76", "989.75"]);
        assert!(rows.iter().all(|row| !row.text_inline.contains("00656")));
    }

    #[test]
    fn infers_a_missing_intermediate_balance_from_the_next_known_balance() {
        let rows = normalize_mapping_rows(
            extract_mapping_rows(
                "28.02.26 6 240.39 Kontostand\n27.03.26 27.03.26 5 756.51 483.88\nLASTSCHRIFT SWISSCOM\n30.03.26 30.03.26 45.00\nLASTSCHRIFT RAIFFEISEN\n30.03.26 30.03.26 4 721.76 989.75",
            )
            .unwrap(),
        );
        let mapping = PdfMapping {
            date_index: 0,
            value_date_index: Some(1),
            description_source: PdfTextSource::Before,
            additional_description_sources: vec![PdfTextSource::Inline],
            amount_index: 1,
            balance_index: Some(0),
            fixed_currency: "CHF".into(),
            amount_sign: PdfAmountSign::InferFromBalance,
            date_format: DateFormat::Auto,
            number_format: NumberFormat::Auto,
            ignored_descriptions: Vec::new(),
        };

        let inferred = resolve_inferred_pdf_rows(&rows, &mapping).unwrap();

        assert_eq!(inferred.get(&4), Some(&(-4_500, 571_151)));
        assert_eq!(inferred.get(&6), Some(&(-98_975, 472_176)));
    }

    #[test]
    fn requires_a_balance_for_inferred_pdf_signs() {
        let mapping = PdfMapping {
            date_index: 0,
            value_date_index: None,
            description_source: PdfTextSource::Before,
            additional_description_sources: Vec::new(),
            amount_index: 0,
            balance_index: None,
            fixed_currency: "CHF".into(),
            amount_sign: PdfAmountSign::InferFromBalance,
            date_format: DateFormat::Auto,
            number_format: NumberFormat::Auto,
            ignored_descriptions: Vec::new(),
        };

        assert!(validate_pdf_mapping(&mapping)
            .unwrap_err()
            .contains("Saldo zugeordnet"));
    }

    #[test]
    fn validates_iban_without_storing_document_text() {
        assert!(valid_iban("GB82WEST12345698765432"));
        assert!(!valid_iban("GB82WEST12345698765431"));
    }
}
