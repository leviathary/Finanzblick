//! Übersetzt SWIFT-MT940-Kontoauszüge in Buchungen und datierte Kontosalden.

use crate::importers::*;
use chrono::Datelike;
use regex::Regex;

fn date(value: &str) -> Result<NaiveDate, String> {
    if value.len() != 6 || !value.bytes().all(|c| c.is_ascii_digit()) {
        return Err("MT940: Ungültiges Datum.".into());
    }
    let year: i32 = value[..2].parse().map_err(|_| "MT940: Ungültiges Jahr.")?;
    NaiveDate::parse_from_str(
        &format!("{}{}", if year >= 70 { "19" } else { "20" }, value),
        "%Y%m%d",
    )
    .map_err(|_| "MT940: Ungültiges Datum.".into())
}

fn amount(value: &str) -> Result<i64, String> {
    let (whole, fraction) = value.split_once(',').ok_or("MT940: Dezimalkomma fehlt.")?;
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return Err("MT940: Ungültiger Betrag oder mehr als zwei Nachkommastellen.".into());
    }
    let whole: i64 = whole.parse().map_err(|_| "MT940: Betrag zu gross.")?;
    let cents = format!("{fraction:0<2}")
        .parse::<i64>()
        .map_err(|_| "MT940: Ungültige Nachkommastellen.")?;
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(cents))
        .ok_or("MT940: Betrag zu gross.".into())
}

fn balance(value: &str) -> Result<(NaiveDate, String, i64), String> {
    let pattern = Regex::new(r"^([CD])(\d{6})([A-Z]{3})(\d+,\d{0,2})$").unwrap();
    let fields = pattern
        .captures(value.trim())
        .ok_or("MT940: Ungültiges Saldofeld.")?;
    Ok((
        date(&fields[2])?,
        fields[3].into(),
        amount(&fields[4])? * if &fields[1] == "D" { -1 } else { 1 },
    ))
}

pub(in crate::importers) fn parse(
    path: &Path,
    provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let bytes = fs::read(path).map_err(|_| "MT940-Datei konnte nicht gelesen werden.")?;
    parse_bytes(&bytes, provider)
}

fn parse_bytes(bytes: &[u8], provider: Option<&str>) -> Result<ParsedStatement, String> {
    let (text, mut warnings) = super::csv_import::decode(bytes)?;
    let tag_pattern = Regex::new(r"^:(\d{2}[A-Z]?):(.*)$").unwrap();
    let row_pattern =
        Regex::new(r"^(\d{6})(\d{4})?(RC|RD|C|D)([A-Z])?(\d+,\d{0,2})([A-Z][A-Z0-9]{3})(.*)$")
            .unwrap();
    let structured_prefix = Regex::new(r"^[A-Z]\d{2}\?").unwrap();
    let structured_separator = Regex::new(r"\?\d{2}").unwrap();
    let mut tags: Vec<(String, String, usize)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('{') || line.starts_with("-}") || line == "$" {
            continue;
        }
        if let Some(parts) = tag_pattern.captures(line) {
            tags.push((parts[1].into(), parts[2].into(), index + 1));
        } else if let Some((_, content, _)) = tags.last_mut() {
            content.push('\n');
            content.push_str(line);
        } else {
            return Err("MT940: Kein gültiger Nachrichtenanfang gefunden.".into());
        }
    }
    let mut account = String::new();
    let mut currency = String::new();
    let mut opening = None;
    let mut opening_date = None;
    let mut running = 0i64;
    let mut active = false;
    let mut final_close = false;
    let mut closing_date = None;
    let mut transactions: Vec<ParsedTransaction> = Vec::new();
    let mut last_row = None;
    for (tag, content, line) in tags {
        match tag.as_str() {
            "25" => {
                if !account.is_empty() && account != content {
                    return Err(
                        "MT940 enthält mehrere Konten. Bitte je Konto eine Datei exportieren."
                            .into(),
                    );
                }
                account = content;
            }
            "60F" | "60M" => {
                if active || account.is_empty() {
                    return Err(
                        "MT940: Anfangssaldo oder Kontokennung fehlt bzw. ist doppelt.".into(),
                    );
                }
                let (day, unit, value) = balance(&content)?;
                if !currency.is_empty() && currency != unit {
                    return Err("MT940: Mehrere Währungen bitte getrennt exportieren.".into());
                }
                if opening.is_some()
                    && (running != value || closing_date.is_some_and(|previous| day < previous))
                {
                    return Err("MT940: Aufeinanderfolgende Auszüge haben widersprüchliche Salden oder Daten.".into());
                }
                if opening.is_none() && tag == "60M" {
                    return Err("MT940: Erste Auszugsseite mit Anfangssaldo 60F fehlt.".into());
                }
                if opening.is_none() {
                    opening = Some(value);
                    opening_date = Some(day);
                }
                currency = unit;
                running = value;
                active = true;
                final_close = false;
                last_row = None;
            }
            "61" => {
                if !active {
                    return Err(format!("MT940 Zeile {line}: Buchung ohne Anfangssaldo."));
                }
                let (first, supplement) = content.split_once('\n').unwrap_or((&content, ""));
                let fields = row_pattern.captures(first).ok_or_else(|| {
                    format!("MT940 Zeile {line}: Buchungsformat nicht unterstützt.")
                })?;
                let value_date = date(&fields[1])?;
                let booking_date = if let Some(entry) = fields.get(2) {
                    (value_date.year() - 1..=value_date.year() + 1)
                        .filter_map(|year| {
                            NaiveDate::parse_from_str(
                                &format!("{year}{}", entry.as_str()),
                                "%Y%m%d",
                            )
                            .ok()
                        })
                        .min_by_key(|day| (*day - value_date).num_days().abs())
                        .ok_or("MT940: Ungültiges Buchungsdatum.")?
                } else {
                    value_date
                };
                let signed = amount(&fields[5])?
                    * if matches!(&fields[3], "D" | "RC") {
                        -1
                    } else {
                        1
                    };
                running = running.checked_add(signed).ok_or("MT940: Saldoüberlauf.")?;
                let reference = fields[7].trim();
                let description = format!(
                    "{}{}",
                    if supplement.is_empty() {
                        &fields[6]
                    } else {
                        supplement
                    },
                    if reference.is_empty() || reference == "NONREF" {
                        String::new()
                    } else {
                        format!(" · Referenz: {reference}")
                    }
                );
                transactions.push(ParsedTransaction {
                    booking_date: booking_date.to_string(),
                    value_date: Some(value_date.to_string()),
                    description,
                    industry: None,
                    amount_minor: signed,
                    balance_minor: Some(running),
                    currency: currency.clone(),
                    confidence: 1.0,
                    source_row: line,
                    ..ParsedTransaction::default()
                });
                last_row = Some(transactions.len() - 1);
            }
            "86" => {
                if let Some(index) = last_row {
                    let clean = structured_prefix.replace(&content, "").into_owned();
                    let clean = structured_separator.replace_all(&clean, " ").into_owned();
                    let previous = &transactions[index].description;
                    let reference = previous
                        .find(" · Referenz:")
                        .map(|offset| &previous[offset..])
                        .unwrap_or("");
                    transactions[index].description = format!("{}{reference}", clean.trim());
                } else {
                    warnings.push(format!("MT940: Zusätzlicher Auszugshinweis: {content}"));
                }
            }
            "62F" | "62M" => {
                if !active {
                    return Err("MT940: Schlusssaldo ohne Anfangssaldo.".into());
                }
                let (day, unit, value) = balance(&content)?;
                if unit != currency || value != running {
                    return Err(format!("MT940 Zeile {line}: Buchungen stimmen nicht mit dem Schlusssaldo überein. Import abgebrochen."));
                }
                closing_date = Some(day);
                active = false;
                final_close = tag == "62F";
                last_row = None;
            }
            "20" | "21" | "28" | "28C" | "64" | "65" | "90C" | "90D" => {
                last_row = None;
            }
            _ => return Err(format!("MT940: Feld {tag} wird noch nicht unterstützt.")),
        }
    }
    if active || !final_close || opening.is_none() {
        return Err("MT940: Datei unvollständig; Anfangs- oder Schlusssaldo fehlt.".into());
    }
    let provider = provider
        .filter(|value| *value != "unknown")
        .unwrap_or(if text.starts_with("{1:F01UBSW") {
            "ubs"
        } else {
            "unknown"
        })
        .to_owned();
    if provider == "unknown" {
        warnings.push("Anbieter wird aus dem ausgewählten Konto übernommen.".into());
    }
    Ok(ParsedStatement {
        provider,
        format: "MT940".into(),
        account_name: account.clone(),
        transactions,
        opening_balance_minor: opening,
        closing_balance_minor: Some(running),
        warnings,
        currency_balances: vec![CurrencyBalance {
            currency,
            opening_date: opening_date.map(|day| day.to_string()),
            opening_balance_minor: opening.unwrap(),
            closing_balance_minor: running,
            closing_date: closing_date.unwrap().to_string(),
        }],
        account_type: None,
        account_reference: Some(account.clone()),
        ..ParsedStatement::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = ":20:TEST\n:25:TEST-ACCOUNT\n:28C:1\n:60F:C251231CHF100,00\n:61:2512310101D10,00NTRFNONREF\n:86:Coffee\n:61:260101RD5,NTRFNONREF\n:86:Reversal\n:62F:C260101CHF95,00\n";
    #[test]
    fn signs_dates_descriptions_and_balance_validation() {
        let parsed = parse_bytes(SAMPLE.as_bytes(), None).unwrap();
        assert_eq!(parsed.transactions.len(), 2);
        assert_eq!(
            parsed.currency_balances[0].opening_date.as_deref(),
            Some("2025-12-31")
        );
        assert_eq!(parsed.transactions[0].booking_date, "2026-01-01");
        assert_eq!(parsed.transactions[0].description, "Coffee");
        assert_eq!(parsed.transactions[1].amount_minor, 500);
        assert!(parse_bytes(SAMPLE.replace("CHF95,00", "CHF96,00").as_bytes(), None).is_err());
        assert!(parse_bytes(SAMPLE.replace(":62F:C260101CHF95,00", "").as_bytes(), None).is_err());
        assert!(parse_bytes(
            SAMPLE
                .replace(":61:260101RD", ":61:260101RC")
                .replace("CHF95,00", "CHF85,00")
                .as_bytes(),
            None
        )
        .is_ok());
        let empty = ":20:A\n:25:A\n:60F:D260101EUR1,00\n:62F:D260101EUR1,00\n";
        assert!(parse_bytes(empty.as_bytes(), None)
            .unwrap()
            .transactions
            .is_empty());
        assert!(parse_bytes(
            format!("{SAMPLE}{}", SAMPLE.replace("TEST-ACCOUNT", "OTHER")).as_bytes(),
            None
        )
        .is_err());
    }
}
