//! Führt deklarative PDF-Kreditkartenprofile mit Mehrseiten- und Kartensummenprüfung aus.
//! Anbieterprofile liefern ausschließlich Erkennungs-, Zeilen- und Beschriftungsregeln als JSON.

use crate::importers::formats::pdf_profile::BundledPdfProfileSummary;
use crate::importers::{
    normalize_date, normalized, parse_money, CurrencyBalance, ParsedStatement, ParsedTransaction,
};
use regex::{Captures, Regex};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CardPdfProfile {
    schema_version: u8,
    id: String,
    provider: String,
    display_name: String,
    recognition_all: Vec<String>,
    account_type: String,
    document_type: String,
    fixed_currency: String,
    account_name_prefix: String,
    account_name_label: String,
    summary: SummaryRules,
    details: DetailRules,
    confidence: f32,
    warning: String,
    rounding_tolerance_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SummaryRules {
    dated_amount_regex: String,
    date_group: usize,
    label_group: usize,
    amount_group: usize,
    opening_label: String,
    closing_label: String,
    card_total_prefix: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DetailRules {
    header_prefix: String,
    dated_description_regex: String,
    date_group: usize,
    description_group: usize,
    amount_tail_regex: String,
    amount_group: usize,
    purchase_date_regex: String,
    card_marker: String,
    card_number_prefix: String,
    carry_to_prefix: String,
    carry_from_prefix: String,
    card_total_prefix: String,
}

struct CompiledCardProfile {
    profile: CardPdfProfile,
    dated_amount: Regex,
    dated_description: Regex,
    amount_tail: Regex,
    purchase_date: Regex,
}

pub(in crate::importers) fn parse_bundled(
    profile_json: &str,
    expected_provider: &str,
    text: &str,
) -> Option<Result<ParsedStatement, String>> {
    let compiled = match compile_profile(profile_json, expected_provider) {
        Ok(profile) => profile,
        Err(error) => return Some(Err(error)),
    };
    let key = normalized(text);
    if !compiled
        .profile
        .recognition_all
        .iter()
        .all(|marker| key.contains(&normalized(marker)))
    {
        return None;
    }
    Some(parse_with_profile(&compiled, text))
}

pub(in crate::importers) fn bundled_profile_summary(
    profile_json: &str,
    expected_provider: &str,
) -> Result<BundledPdfProfileSummary, String> {
    let compiled = compile_profile(profile_json, expected_provider)?;
    Ok(BundledPdfProfileSummary {
        id: compiled.profile.id,
        provider: compiled.profile.provider,
        display_name: compiled.profile.display_name,
        schema_version: compiled.profile.schema_version,
        document_type: compiled.profile.document_type,
    })
}

fn compile_profile(
    profile_json: &str,
    expected_provider: &str,
) -> Result<CompiledCardProfile, String> {
    let profile: CardPdfProfile = serde_json::from_str(profile_json)
        .map_err(|error| format!("Mitgeliefertes PDF-Kreditkartenprofil ist ungültig: {error}"))?;
    if profile.schema_version != 1
        || profile.id.trim().is_empty()
        || profile.provider != normalized(expected_provider)
        || profile.display_name.trim().is_empty()
        || profile.recognition_all.is_empty()
        || profile.fixed_currency.len() != 3
        || profile.account_name_prefix.trim().is_empty()
        || profile.account_name_label.trim().is_empty()
        || profile.warning.trim().is_empty()
        || !(0.0..=1.0).contains(&profile.confidence)
        || !(0..=100).contains(&profile.rounding_tolerance_minor)
    {
        return Err("Mitgeliefertes PDF-Kreditkartenprofil enthält ungültige Metadaten.".into());
    }
    let dated_amount = Regex::new(&profile.summary.dated_amount_regex).map_err(|error| {
        format!("PDF-Kreditkartenprofil enthält einen ungültigen Ausdruck: {error}")
    })?;
    let dated_description =
        Regex::new(&profile.details.dated_description_regex).map_err(|error| {
            format!("PDF-Kreditkartenprofil enthält einen ungültigen Ausdruck: {error}")
        })?;
    let amount_tail = Regex::new(&profile.details.amount_tail_regex).map_err(|error| {
        format!("PDF-Kreditkartenprofil enthält einen ungültigen Ausdruck: {error}")
    })?;
    let purchase_date = Regex::new(&profile.details.purchase_date_regex).map_err(|error| {
        format!("PDF-Kreditkartenprofil enthält einen ungültigen Ausdruck: {error}")
    })?;
    for (regex, group, field) in [
        (&dated_amount, profile.summary.date_group, "Rechnungsdatum"),
        (
            &dated_amount,
            profile.summary.label_group,
            "Zusammenfassungsbezeichnung",
        ),
        (
            &dated_amount,
            profile.summary.amount_group,
            "Zusammenfassungsbetrag",
        ),
        (
            &dated_description,
            profile.details.date_group,
            "Buchungsdatum",
        ),
        (
            &dated_description,
            profile.details.description_group,
            "Buchungstext",
        ),
        (&amount_tail, profile.details.amount_group, "Buchungsbetrag"),
    ] {
        if group == 0 || group >= regex.captures_len() {
            return Err(format!(
                "PDF-Kreditkartenprofil enthält eine ungültige Gruppe für {field}."
            ));
        }
    }
    Ok(CompiledCardProfile {
        profile,
        dated_amount,
        dated_description,
        amount_tail,
        purchase_date,
    })
}

fn capture<'a>(captures: &'a Captures<'a>, group: usize) -> Result<&'a str, String> {
    captures
        .get(group)
        .map(|value| value.as_str())
        .ok_or_else(|| {
            "PDF-Kreditkartenprofil konnte eine konfigurierte Gruppe nicht lesen.".into()
        })
}

fn parse_with_profile(
    compiled: &CompiledCardProfile,
    text: &str,
) -> Result<ParsedStatement, String> {
    let profile = &compiled.profile;
    let lines = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut opening = None;
    let mut opening_date = None;
    let mut closing = None;
    let mut closing_date = None;
    let mut expected_card_totals = Vec::new();
    let mut checked_card_totals = Vec::new();
    let mut card_total = 0;
    let mut active = false;
    let mut detail_mode = false;
    let mut card = String::new();
    let mut pending: Option<(String, String, usize)> = None;

    for (index, line) in lines.iter().enumerate() {
        if line.starts_with(&profile.details.header_prefix) {
            active = true;
            detail_mode = true;
            continue;
        }
        if !detail_mode {
            if let Some(captures) = compiled.dated_amount.captures(line) {
                let date = capture(&captures, profile.summary.date_group)?;
                let label = capture(&captures, profile.summary.label_group)?.trim();
                let value = parse_profile_money(capture(&captures, profile.summary.amount_group)?)?;
                if label == profile.summary.opening_label {
                    opening = Some(-value);
                    opening_date = normalize_date(date);
                } else if label == profile.summary.closing_label {
                    closing = Some(-value);
                    closing_date = normalize_date(date);
                } else if label.starts_with(&profile.summary.card_total_prefix) {
                    expected_card_totals.push(value);
                } else {
                    rows.push(transaction(
                        normalize_date(date).ok_or("Kreditkarten-Rechnungsdatum ist ungültig.")?,
                        label.to_string(),
                        -value,
                        profile,
                        index + 1,
                    ));
                }
            }
            continue;
        }
        if line.starts_with(&profile.details.carry_to_prefix)
            || line.starts_with(&profile.details.card_total_prefix)
        {
            if pending.is_some() {
                return Err("Kreditkartenprofil: unvollständige Einzelbuchung.".into());
            }
            let value = line
                .rsplit_once(' ')
                .and_then(|(_, value)| parse_money(value))
                .ok_or("Kreditkartenprofil: unlesbares Kartentotal.")?;
            if value != card_total {
                return Err(format!(
                    "Kreditkartenprofil: Karten-/Seitenbetrag in Zeile {} stimmt nicht überein.",
                    index + 1
                ));
            }
            if line.starts_with(&profile.details.card_total_prefix) {
                checked_card_totals.push(value);
                card_total = 0;
                card.clear();
            }
            active = false;
            continue;
        }
        if line.contains(&profile.details.card_marker) {
            if pending.is_some() || card_total != 0 {
                return Err(
                    "Kreditkartenprofil: Kartenwechsel vor Abschluss der vorherigen Karte.".into(),
                );
            }
            card = line.clone();
            active = true;
            continue;
        }
        if !active {
            continue;
        }
        if line.starts_with(&profile.details.carry_from_prefix) {
            continue;
        }
        if line.starts_with(&profile.details.card_number_prefix) {
            card.push(' ');
            card.push_str(line);
            continue;
        }
        if let Some(captures) = compiled.amount_tail.captures(line) {
            let (date, mut description, source_row) = pending
                .take()
                .ok_or("Kreditkartenprofil: Betrag ohne Buchung.")?;
            let value = parse_profile_money(capture(&captures, profile.details.amount_group)?)?;
            if line.starts_with(|character: char| character.is_ascii_alphabetic())
                || compiled.purchase_date.is_match(line)
            {
                description.push('\n');
                description.push_str(line);
            }
            if !card.is_empty() {
                description.push('\n');
                description.push_str(&card);
            }
            rows.push(transaction(date, description, -value, profile, source_row));
            card_total += value;
        } else if let Some(captures) = compiled.dated_description.captures(line) {
            if pending.is_some() {
                return Err("Kreditkartenprofil: Betrag einer Buchung fehlt.".into());
            }
            pending = Some((
                normalize_date(capture(&captures, profile.details.date_group)?)
                    .ok_or("Kreditkartenprofil: ungültiges Buchungsdatum.")?,
                capture(&captures, profile.details.description_group)?.to_string(),
                index + 1,
            ));
        } else if let Some((_, description, _)) = pending.as_mut() {
            description.push('\n');
            description.push_str(line);
        } else {
            return Err("Kreditkartenprofil: unerwartete Zeile in der Buchungstabelle.".into());
        }
    }
    if pending.is_some()
        || expected_card_totals.is_empty()
        || expected_card_totals != checked_card_totals
    {
        return Err("Kreditkartenprofil: Kartendetails fehlen oder stimmen nicht überein.".into());
    }
    let opening = opening.ok_or("Kreditkartenprofil: Vorrechnung fehlt.")?;
    let closing = closing.ok_or("Kreditkartenprofil: Rechnungsbetrag fehlt.")?;
    let closing_date = closing_date.ok_or("Kreditkartenprofil: Rechnungsdatum fehlt.")?;
    let difference = closing - opening - rows.iter().map(|row| row.amount_minor).sum::<i64>();
    let mut warnings = vec![profile.warning.clone()];
    if difference != 0 {
        if difference.abs() > profile.rounding_tolerance_minor {
            return Err(
                "Kreditkartenprofil: Rechnungsbetrag stimmt nicht mit den Buchungen überein."
                    .into(),
            );
        }
        warnings.push(format!(
            "Die Rechnung weist eine Rundungsdifferenz von {difference} Rappen auf; sie wird als eigene Ausgleichsbuchung erfasst."
        ));
        rows.push(transaction(
            closing_date.clone(),
            "Rundungsausgleich zum ausgewiesenen Rechnungsbetrag".into(),
            difference,
            profile,
            lines.len() + 1,
        ));
    }
    rows.sort_by(|left, right| {
        left.booking_date
            .cmp(&right.booking_date)
            .then(left.source_row.cmp(&right.source_row))
    });
    let account = lines
        .iter()
        .find(|line| line.starts_with(&profile.account_name_prefix))
        .ok_or("Kreditkartenprofil: Kartenkonto fehlt.")?;
    Ok(ParsedStatement {
        provider: profile.provider.clone(),
        format: "PDF".into(),
        account_name: format!("{} · {account}", profile.account_name_label),
        opening_balance_minor: Some(opening),
        closing_balance_minor: Some(closing),
        transactions: rows,
        currency_balances: vec![CurrencyBalance {
            currency: profile.fixed_currency.clone(),
            opening_date,
            opening_balance_minor: opening,
            closing_balance_minor: closing,
            closing_date: closing_date.clone(),
        }],
        account_type: Some(profile.account_type.clone()),
        document_type: Some(profile.document_type.clone()),
        document_date: Some(closing_date),
        warnings,
        ..ParsedStatement::default()
    })
}

fn parse_profile_money(value: &str) -> Result<i64, String> {
    parse_money(value).ok_or_else(|| "Betrag im PDF-Kreditkartenprofil ist unlesbar.".into())
}

fn transaction(
    booking_date: String,
    description: String,
    amount_minor: i64,
    profile: &CardPdfProfile,
    source_row: usize,
) -> ParsedTransaction {
    ParsedTransaction {
        booking_date,
        value_date: None,
        description,
        amount_minor,
        balance_minor: None,
        currency: profile.fixed_currency.clone(),
        confidence: profile.confidence,
        source_row,
        ..ParsedTransaction::default()
    }
}
