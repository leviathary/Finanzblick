//! Führt vertrauenswürdige, deklarative PDF-Providerprofile ohne anbieterspezifischen Parsercode aus.
//! Die Engine extrahiert Metadaten und Buchungen und prüft Salden sowie Kontrollsummen.

use crate::importers::{
    normalize_date, normalized, parse_money, CurrencyBalance, ParsedStatement, ParsedTransaction,
};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundledPdfProfile {
    schema_version: u8,
    id: String,
    provider: String,
    display_name: String,
    recognition_all: Vec<String>,
    account_type: String,
    document_type: String,
    account_reference: Option<CaptureRule>,
    #[serde(default = "default_true")]
    require_account_reference: bool,
    currency: Option<CaptureRule>,
    fixed_currency: Option<String>,
    document_date: Option<CaptureRule>,
    account_name: LineBeforeRule,
    table: TableRules,
    rows: RowRules,
    transaction_reference: Option<TransactionReferenceRule>,
    confidence: f32,
    warning: Option<String>,
    #[serde(default)]
    sign_strategy: SignStrategy,
    #[serde(default)]
    allow_empty_transactions: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledPdfProfileSummary {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub schema_version: u8,
    pub document_type: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SignStrategy {
    #[default]
    ReconciledTotals,
    RunningBalance,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureRule {
    regex: String,
    group: usize,
    #[serde(default)]
    canonical_alphanumeric: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LineBeforeRule {
    marker_contains: String,
    fallback: String,
    regex: Option<String>,
    group: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TableRules {
    header_all: Vec<String>,
    stop_starts_with: Vec<String>,
    ignore_exact: Vec<String>,
    ignore_regex: Vec<String>,
    #[serde(default)]
    ignore_account_reference_lines: bool,
    #[serde(default)]
    ignore_document_date_prefix: bool,
    content_after_header_marker: Option<String>,
    #[serde(default)]
    stop_after_closing_balance: bool,
    #[serde(default)]
    continuation_to_previous: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RowRules {
    money_pattern: String,
    balance_regex: String,
    balance_date_group: usize,
    balance_amount_group: usize,
    transaction_regex: String,
    description_group: usize,
    booking_date_group: usize,
    value_date_group: Option<usize>,
    balance_group: Option<usize>,
    amount_group: usize,
    trailing_description_group: Option<usize>,
    totals_regex: Option<String>,
    debit_total_group: Option<usize>,
    credit_total_group: Option<usize>,
    debit_total_regex: Option<String>,
    debit_total_value_group: Option<usize>,
    credit_total_regex: Option<String>,
    credit_total_value_group: Option<usize>,
    #[serde(default)]
    transaction_alternatives: Vec<TransactionRowRule>,
    candidate_regex: Option<String>,
    #[serde(default)]
    trailing_minus: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TransactionRowRule {
    regex: String,
    description_group: usize,
    booking_date_group: usize,
    value_date_group: Option<usize>,
    balance_group: Option<usize>,
    amount_group: usize,
    trailing_description_group: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TransactionReferenceRule {
    label_starts_with: String,
    namespace: String,
}

#[derive(Debug)]
struct CandidateTransaction {
    booking_date: String,
    value_date: Option<String>,
    description: String,
    amount_minor: i64,
    balance_minor: Option<i64>,
    source_row: usize,
}

#[derive(Clone, Copy)]
struct TransactionGroups {
    description: usize,
    booking_date: usize,
    value_date: Option<usize>,
    balance: Option<usize>,
    amount: usize,
    trailing_description: Option<usize>,
}

struct CompiledProfile {
    profile: BundledPdfProfile,
    account_reference: Option<Regex>,
    currency: Option<Regex>,
    document_date: Option<Regex>,
    account_name: Option<Regex>,
    balance: Regex,
    transaction: Regex,
    totals: Option<Regex>,
    debit_total: Option<Regex>,
    credit_total: Option<Regex>,
    transaction_alternatives: Vec<(Regex, TransactionRowRule)>,
    candidate: Option<Regex>,
    ignored: Vec<Regex>,
}

fn default_true() -> bool {
    true
}

pub(in crate::importers) fn parse_bundled(
    profile_json: &str,
    expected_provider: &str,
    text: &str,
) -> Option<Result<ParsedStatement, String>> {
    let compiled = match compile_profile(profile_json) {
        Ok(profile) => profile,
        Err(error) => return Some(Err(error)),
    };
    if compiled.profile.provider != normalized(expected_provider) {
        return Some(Err(
            "Mitgeliefertes PDF-Profil ist dem falschen Anbieter zugeordnet.".into(),
        ));
    }
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
    let compiled = compile_profile(profile_json)?;
    if compiled.profile.provider != normalized(expected_provider) {
        return Err("Mitgeliefertes PDF-Profil ist dem falschen Anbieter zugeordnet.".into());
    }
    Ok(BundledPdfProfileSummary {
        id: compiled.profile.id,
        provider: compiled.profile.provider,
        display_name: compiled.profile.display_name,
        schema_version: compiled.profile.schema_version,
        document_type: compiled.profile.document_type,
    })
}

fn compile_profile(profile_json: &str) -> Result<CompiledProfile, String> {
    let profile: BundledPdfProfile = serde_json::from_str(profile_json)
        .map_err(|error| format!("Mitgeliefertes PDF-Profil ist ungültig: {error}"))?;
    if profile.schema_version != 1
        || profile.id.trim().is_empty()
        || profile.provider.trim().is_empty()
        || profile.display_name.trim().is_empty()
        || profile.recognition_all.is_empty()
        || !(0.0..=1.0).contains(&profile.confidence)
        || profile
            .warning
            .as_ref()
            .is_some_and(|warning| warning.trim().is_empty())
    {
        return Err("Mitgeliefertes PDF-Profil enthält ungültige Metadaten.".into());
    }
    let compile = |value: &str| {
        Regex::new(&value.replace("{money}", &profile.rows.money_pattern)).map_err(|error| {
            format!(
                "PDF-Profil {} enthält einen ungültigen Ausdruck: {error}",
                profile.id
            )
        })
    };
    let account_reference = profile
        .account_reference
        .as_ref()
        .map(|rule| compile(&rule.regex))
        .transpose()?;
    let currency = profile
        .currency
        .as_ref()
        .map(|rule| compile(&rule.regex))
        .transpose()?;
    let document_date = profile
        .document_date
        .as_ref()
        .map(|rule| compile(&rule.regex))
        .transpose()?;
    let account_name = profile
        .account_name
        .regex
        .as_deref()
        .map(compile)
        .transpose()?;
    let balance = compile(&profile.rows.balance_regex)?;
    let transaction = compile(&profile.rows.transaction_regex)?;
    let totals = profile
        .rows
        .totals_regex
        .as_deref()
        .map(compile)
        .transpose()?;
    let debit_total = profile
        .rows
        .debit_total_regex
        .as_deref()
        .map(compile)
        .transpose()?;
    let credit_total = profile
        .rows
        .credit_total_regex
        .as_deref()
        .map(compile)
        .transpose()?;
    let transaction_alternatives = profile
        .rows
        .transaction_alternatives
        .iter()
        .map(|rule| compile(&rule.regex).map(|regex| (regex, rule.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    let candidate = profile
        .rows
        .candidate_regex
        .as_deref()
        .map(compile)
        .transpose()?;
    let ignored = profile
        .table
        .ignore_regex
        .iter()
        .map(|value| compile(value))
        .collect::<Result<_, _>>()?;
    if let (Some(rule), Some(regex)) = (&profile.account_reference, &account_reference) {
        validate_group(regex, rule.group, "Kontoreferenz")?;
    }
    if let (Some(rule), Some(regex)) = (&profile.currency, &currency) {
        validate_group(regex, rule.group, "Währung")?;
    }
    if let (Some(rule), Some(regex)) = (&profile.document_date, &document_date) {
        validate_group(regex, rule.group, "Dokumentdatum")?;
    }
    match (&account_name, profile.account_name.group) {
        (Some(regex), Some(group)) => validate_group(regex, group, "Kontoname")?,
        (None, None) => {}
        _ => return Err("PDF-Profil enthält eine unvollständige Kontonamensregel.".into()),
    }
    validate_group(&balance, profile.rows.balance_date_group, "Saldodatum")?;
    validate_group(&balance, profile.rows.balance_amount_group, "Saldo")?;
    validate_group(&transaction, profile.rows.description_group, "Buchungstext")?;
    validate_group(
        &transaction,
        profile.rows.booking_date_group,
        "Buchungsdatum",
    )?;
    if let Some(group) = profile.rows.value_date_group {
        validate_group(&transaction, group, "Valutadatum")?;
    }
    if let Some(group) = profile.rows.balance_group {
        validate_group(&transaction, group, "Zwischensaldo")?;
    }
    validate_group(&transaction, profile.rows.amount_group, "Buchungsbetrag")?;
    if let Some(group) = profile.rows.trailing_description_group {
        validate_group(&transaction, group, "nachgestellter Buchungstext")?;
    }
    match (
        &totals,
        profile.rows.debit_total_group,
        profile.rows.credit_total_group,
    ) {
        (Some(regex), Some(debit_group), Some(credit_group)) => {
            validate_group(regex, debit_group, "Belastungstotal")?;
            validate_group(regex, credit_group, "Gutschriftentotal")?;
        }
        (None, None, None) => {}
        _ => return Err("PDF-Profil enthält unvollständige gemeinsame Kontrollsummen.".into()),
    }
    match (&debit_total, profile.rows.debit_total_value_group) {
        (Some(regex), Some(group)) => validate_group(regex, group, "Belastungstotal")?,
        (None, None) => {}
        _ => return Err("PDF-Profil enthält ein unvollständiges Belastungstotal.".into()),
    }
    match (&credit_total, profile.rows.credit_total_value_group) {
        (Some(regex), Some(group)) => validate_group(regex, group, "Gutschriftentotal")?,
        (None, None) => {}
        _ => return Err("PDF-Profil enthält ein unvollständiges Gutschriftentotal.".into()),
    }
    if totals.is_none() && (debit_total.is_none() || credit_total.is_none()) {
        return Err("PDF-Profil enthält keine vollständigen Kontrollsummen.".into());
    }
    for (regex, rule) in &transaction_alternatives {
        validate_transaction_rule(regex, rule)?;
    }
    Ok(CompiledProfile {
        profile,
        account_reference,
        currency,
        document_date,
        account_name,
        balance,
        transaction,
        totals,
        debit_total,
        credit_total,
        transaction_alternatives,
        candidate,
        ignored,
    })
}

fn validate_transaction_rule(regex: &Regex, rule: &TransactionRowRule) -> Result<(), String> {
    validate_group(regex, rule.description_group, "alternativen Buchungstext")?;
    validate_group(regex, rule.booking_date_group, "alternatives Buchungsdatum")?;
    validate_group(regex, rule.amount_group, "alternativer Buchungsbetrag")?;
    for (group, field) in [
        (rule.value_date_group, "alternatives Valutadatum"),
        (rule.balance_group, "alternativer Zwischensaldo"),
        (
            rule.trailing_description_group,
            "alternativer nachgestellter Buchungstext",
        ),
    ] {
        if let Some(group) = group {
            validate_group(regex, group, field)?;
        }
    }
    Ok(())
}

fn validate_group(regex: &Regex, group: usize, field: &str) -> Result<(), String> {
    if group == 0 || group >= regex.captures_len() {
        Err(format!(
            "Mitgeliefertes PDF-Profil enthält eine ungültige Gruppe für {field}."
        ))
    } else {
        Ok(())
    }
}

fn parse_with_profile(compiled: &CompiledProfile, text: &str) -> Result<ParsedStatement, String> {
    let profile = &compiled.profile;
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let account_reference = match (&profile.account_reference, &compiled.account_reference) {
        (Some(rule), Some(regex)) => capture_value(regex, text, rule),
        _ => None,
    };
    if profile.require_account_reference && account_reference.is_none() {
        return Err(format!(
            "Im {} fehlt die Kontoreferenz.",
            profile.display_name
        ));
    }
    let currency = match (&profile.currency, &compiled.currency) {
        (Some(rule), Some(regex)) => capture_value(regex, text, rule),
        _ => None,
    }
    .or_else(|| profile.fixed_currency.clone())
    .ok_or_else(|| format!("Im {} fehlt die Währung.", profile.display_name))?;
    if currency.len() != 3 || !currency.chars().all(|value| value.is_ascii_alphabetic()) {
        return Err(format!(
            "Im {} ist die Währung ungültig.",
            profile.display_name
        ));
    }
    let document_date = match (&profile.document_date, &compiled.document_date) {
        (Some(rule), Some(regex)) => {
            capture_value(regex, text, rule).and_then(|value| normalize_date(&value))
        }
        _ => None,
    };
    let account_name_marker = normalized(&profile.account_name.marker_contains);
    let account_name = compiled
        .account_name
        .as_ref()
        .zip(profile.account_name.group)
        .and_then(|(regex, group)| regex.captures(text)?.get(group).map(|value| value.as_str()))
        .map(str::trim)
        .map(str::to_string)
        .or_else(|| {
            lines
                .windows(2)
                .find(|pair| normalized(pair[1]).contains(&account_name_marker))
                .map(|pair| pair[0].to_string())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| profile.account_name.fallback.clone());

    let mut opening = None;
    let mut opening_date = None;
    let mut closing = None;
    let mut closing_date = None;
    let (total_debits, total_credits) = extract_totals(compiled, &lines)?;
    let mut pending_description = Vec::new();
    let mut candidates = Vec::new();
    let mut table_started = false;

    for (index, line) in lines.iter().enumerate() {
        let key = normalized(line);
        let mut row_line = *line;
        if profile
            .table
            .header_all
            .iter()
            .all(|marker| key.contains(&normalized(marker)))
        {
            table_started = true;
            let Some(marker) = profile.table.content_after_header_marker.as_deref() else {
                continue;
            };
            let Some((_, remainder)) = line.split_once(marker) else {
                continue;
            };
            row_line = remainder.trim();
            if row_line.is_empty() {
                continue;
            }
        }
        if !table_started {
            continue;
        }
        if profile
            .table
            .stop_starts_with
            .iter()
            .any(|marker| key.starts_with(&normalized(marker)))
        {
            break;
        }
        if totals_line(compiled, row_line) {
            pending_description.clear();
            continue;
        }
        if let Some(captures) = compiled.balance.captures(row_line) {
            let date = parse_capture_date(&captures, profile.rows.balance_date_group)
                .ok_or_else(|| format!("{}-Saldodatum ist ungültig.", profile.display_name))?;
            let balance = parse_capture_profile_money(
                &captures,
                profile.rows.balance_amount_group,
                profile.rows.trailing_minus,
            )
            .ok_or_else(|| format!("{}-Saldo ist ungültig.", profile.display_name))?;
            if opening.is_none() && candidates.is_empty() {
                opening = Some(balance);
                opening_date = Some(date);
            } else {
                closing = Some(balance);
                closing_date = Some(date);
            }
            pending_description.clear();
            if closing.is_some() && profile.table.stop_after_closing_balance {
                break;
            }
            continue;
        }
        if let Some((captures, groups)) = transaction_captures(compiled, row_line) {
            if let Some(prefix) = captures
                .get(groups.description)
                .map(|value| value.as_str().trim())
                .filter(|value| !value.is_empty())
            {
                pending_description.push(prefix);
            }
            if let Some(details) = groups
                .trailing_description
                .and_then(|group| captures.get(group))
                .map(|value| value.as_str().trim())
                .filter(|value| !value.is_empty())
            {
                pending_description.push(details);
            }
            let booking_date = parse_capture_date(&captures, groups.booking_date)
                .ok_or_else(|| format!("{}-Buchungsdatum ist ungültig.", profile.display_name))?;
            let value_date = groups
                .value_date
                .and_then(|group| parse_capture_date(&captures, group));
            if groups.value_date.is_some() && value_date.is_none() {
                return Err(format!(
                    "{}-Valutadatum ist ungültig.",
                    profile.display_name
                ));
            }
            let amount_minor =
                parse_capture_profile_money(&captures, groups.amount, profile.rows.trailing_minus)
                    .ok_or_else(|| {
                        format!("{}-Buchungsbetrag ist ungültig.", profile.display_name)
                    })?;
            let balance_minor = groups.balance.and_then(|group| {
                parse_capture_profile_money(&captures, group, profile.rows.trailing_minus)
            });
            let description = pending_description.join("\n");
            if description.is_empty() {
                return Err(format!("{}-Buchungstext fehlt.", profile.display_name));
            }
            candidates.push(CandidateTransaction {
                booking_date,
                value_date,
                description,
                amount_minor,
                balance_minor,
                source_row: index + 1,
            });
            pending_description.clear();
            continue;
        }
        if compiled
            .candidate
            .as_ref()
            .is_some_and(|candidate| candidate.is_match(row_line))
        {
            return Err(format!(
                "{}-Buchungszeile {} konnte nicht vollständig gelesen werden.",
                profile.display_name,
                index + 1
            ));
        }
        if ignored_line(
            compiled,
            row_line,
            &key,
            account_reference.as_deref(),
            document_date.as_deref(),
        ) {
            continue;
        }
        if profile.table.continuation_to_previous {
            if let Some(candidate) = candidates.last_mut() {
                candidate.description.push('\n');
                candidate.description.push_str(row_line);
            }
        } else {
            pending_description.push(row_line);
        }
    }

    let opening =
        opening.ok_or_else(|| format!("Im {} fehlt der Anfangssaldo.", profile.display_name))?;
    let closing =
        closing.ok_or_else(|| format!("Im {} fehlt der Schlusssaldo.", profile.display_name))?;
    if candidates.is_empty() && !profile.allow_empty_transactions {
        return Err(format!(
            "Im {} wurden keine Buchungen erkannt.",
            profile.display_name
        ));
    }
    let signed_amounts = match profile.sign_strategy {
        SignStrategy::ReconciledTotals => reconcile_signs(
            &candidates,
            opening,
            closing,
            total_debits,
            total_credits,
            &profile.display_name,
        )?
        .into_iter()
        .zip(candidates.iter())
        .map(|(sign, candidate)| sign * candidate.amount_minor.abs())
        .collect(),
        SignStrategy::RunningBalance => running_balance_amounts(
            &candidates,
            opening,
            closing,
            total_debits,
            total_credits,
            &profile.display_name,
        )?,
    };
    let mut running_balance = opening;
    let transactions = candidates
        .into_iter()
        .zip(signed_amounts)
        .map(|(candidate, amount_minor)| {
            running_balance += amount_minor;
            if candidate
                .balance_minor
                .is_some_and(|balance| balance != running_balance)
            {
                return Err(format!(
                    "Ein {}-Zwischensaldo passt nicht zu den Buchungen.",
                    profile.display_name
                ));
            }
            let external_reference = profile.transaction_reference.as_ref().and_then(|rule| {
                reference_after_label(&candidate.description, &rule.label_starts_with)
            });
            Ok(ParsedTransaction {
                booking_date: candidate.booking_date,
                value_date: candidate.value_date,
                description: candidate.description,
                amount_minor,
                balance_minor: Some(running_balance),
                currency: currency.clone(),
                confidence: profile.confidence,
                source_row: candidate.source_row,
                reference_namespace: external_reference.as_ref().and_then(|_| {
                    profile
                        .transaction_reference
                        .as_ref()
                        .map(|rule| rule.namespace.clone())
                }),
                external_reference,
                ..ParsedTransaction::default()
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if running_balance != closing {
        return Err(format!(
            "Der {}-Schlusssaldo passt nicht zu den Buchungen.",
            profile.display_name
        ));
    }
    let opening_date =
        opening_date.ok_or_else(|| format!("{}-Anfangsdatum fehlt.", profile.display_name))?;
    let closing_date =
        closing_date.ok_or_else(|| format!("{}-Schlussdatum fehlt.", profile.display_name))?;
    Ok(ParsedStatement {
        currency_balances: vec![CurrencyBalance {
            currency: currency.clone(),
            opening_date: Some(opening_date),
            opening_balance_minor: opening,
            closing_balance_minor: closing,
            closing_date,
        }],
        account_type: Some(profile.account_type.clone()),
        provider: profile.provider.clone(),
        format: "PDF".into(),
        account_name,
        transactions,
        opening_balance_minor: Some(opening),
        closing_balance_minor: Some(closing),
        warnings: profile.warning.clone().into_iter().collect(),
        document_type: Some(profile.document_type.clone()),
        document_date,
        account_reference,
        ..ParsedStatement::default()
    })
}

fn ignored_line(
    compiled: &CompiledProfile,
    line: &str,
    key: &str,
    account_reference: Option<&str>,
    document_date: Option<&str>,
) -> bool {
    let table = &compiled.profile.table;
    table
        .ignore_exact
        .iter()
        .any(|value| normalized(value) == key)
        || compiled
            .ignored
            .iter()
            .any(|pattern| pattern.is_match(line))
        || (table.ignore_account_reference_lines
            && account_reference
                .is_some_and(|reference| canonical_alphanumeric(line).contains(reference)))
        || (table.ignore_document_date_prefix
            && document_date.is_some_and(|date| {
                normalize_date(line.split_whitespace().next().unwrap_or_default()).as_deref()
                    == Some(date)
                    && line.split_whitespace().count() > 1
            }))
}

fn capture_value(regex: &Regex, text: &str, rule: &CaptureRule) -> Option<String> {
    let value = regex.captures(text)?.get(rule.group)?.as_str().trim();
    Some(if rule.canonical_alphanumeric {
        canonical_alphanumeric(value)
    } else {
        value.to_ascii_uppercase()
    })
}

fn extract_totals(compiled: &CompiledProfile, lines: &[&str]) -> Result<(i64, i64), String> {
    let rows = &compiled.profile.rows;
    if let (Some(regex), Some(debit_group), Some(credit_group)) = (
        &compiled.totals,
        rows.debit_total_group,
        rows.credit_total_group,
    ) {
        return lines
            .iter()
            .find_map(|line| {
                let captures = regex.captures(line)?;
                Some((
                    parse_capture_profile_money(&captures, debit_group, rows.trailing_minus)?,
                    parse_capture_profile_money(&captures, credit_group, rows.trailing_minus)?,
                ))
            })
            .ok_or_else(|| {
                format!(
                    "Im {} fehlen die Kontrollsummen.",
                    compiled.profile.display_name
                )
            });
    }
    let debit = compiled
        .debit_total
        .as_ref()
        .zip(rows.debit_total_value_group)
        .and_then(|(regex, group)| {
            lines.iter().find_map(|line| {
                regex.captures(line).and_then(|captures| {
                    parse_capture_profile_money(&captures, group, rows.trailing_minus)
                })
            })
        })
        .ok_or_else(|| {
            format!(
                "Im {} fehlt das Belastungstotal.",
                compiled.profile.display_name
            )
        })?;
    let credit = compiled
        .credit_total
        .as_ref()
        .zip(rows.credit_total_value_group)
        .and_then(|(regex, group)| {
            lines.iter().find_map(|line| {
                regex.captures(line).and_then(|captures| {
                    parse_capture_profile_money(&captures, group, rows.trailing_minus)
                })
            })
        })
        .ok_or_else(|| {
            format!(
                "Im {} fehlt das Gutschriftentotal.",
                compiled.profile.display_name
            )
        })?;
    Ok((debit, credit))
}

fn totals_line(compiled: &CompiledProfile, line: &str) -> bool {
    compiled
        .totals
        .as_ref()
        .is_some_and(|regex| regex.is_match(line))
        || compiled
            .debit_total
            .as_ref()
            .is_some_and(|regex| regex.is_match(line))
        || compiled
            .credit_total
            .as_ref()
            .is_some_and(|regex| regex.is_match(line))
}

fn transaction_captures<'a>(
    compiled: &'a CompiledProfile,
    line: &'a str,
) -> Option<(Captures<'a>, TransactionGroups)> {
    let rows = &compiled.profile.rows;
    if let Some(captures) = compiled.transaction.captures(line) {
        return Some((
            captures,
            TransactionGroups {
                description: rows.description_group,
                booking_date: rows.booking_date_group,
                value_date: rows.value_date_group,
                balance: rows.balance_group,
                amount: rows.amount_group,
                trailing_description: rows.trailing_description_group,
            },
        ));
    }
    compiled
        .transaction_alternatives
        .iter()
        .find_map(|(regex, rule)| {
            regex.captures(line).map(|captures| {
                (
                    captures,
                    TransactionGroups {
                        description: rule.description_group,
                        booking_date: rule.booking_date_group,
                        value_date: rule.value_date_group,
                        balance: rule.balance_group,
                        amount: rule.amount_group,
                        trailing_description: rule.trailing_description_group,
                    },
                )
            })
        })
}

fn parse_capture_profile_money(
    captures: &Captures<'_>,
    group: usize,
    trailing_minus: bool,
) -> Option<i64> {
    let value = captures.get(group)?.as_str().trim();
    if trailing_minus {
        if let Some(unsigned) = value.strip_suffix('-') {
            return parse_money(unsigned.trim()).map(|amount| -amount.abs());
        }
    }
    parse_money(value)
}

fn parse_capture_date(captures: &Captures<'_>, group: usize) -> Option<String> {
    captures
        .get(group)
        .and_then(|value| normalize_date(value.as_str()))
}

fn canonical_alphanumeric(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

fn reference_after_label(description: &str, label: &str) -> Option<String> {
    let label = normalized(label);
    let mut lines = description.lines();
    while let Some(line) = lines.next() {
        if normalized(line).starts_with(&label) {
            return lines
                .next()
                .map(canonical_alphanumeric)
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn reconcile_signs(
    candidates: &[CandidateTransaction],
    opening: i64,
    closing: i64,
    total_debits: i64,
    total_credits: i64,
    display_name: &str,
) -> Result<Vec<i64>, String> {
    if candidates
        .iter()
        .map(|row| row.amount_minor.abs())
        .sum::<i64>()
        != total_debits + total_credits
        || opening + total_credits - total_debits != closing
    {
        return Err(format!(
            "Die {display_name}-Kontrollsummen passen nicht zu Beträgen und Schlusssaldo."
        ));
    }
    let credit_indices = credit_indices_for_statement(candidates, opening, total_credits)
        .ok_or_else(|| {
            format!(
                "Gutschriften und Belastungen im {display_name} sind nicht eindeutig abstimmbar."
            )
        })?;
    let mut signs = vec![-1; candidates.len()];
    for index in credit_indices {
        signs[index] = 1;
    }
    Ok(signs)
}

fn running_balance_amounts(
    candidates: &[CandidateTransaction],
    opening: i64,
    closing: i64,
    total_debits: i64,
    total_credits: i64,
    display_name: &str,
) -> Result<Vec<i64>, String> {
    let mut previous = opening;
    let mut debits = 0;
    let mut credits = 0;
    let mut amounts = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let balance = candidate
            .balance_minor
            .ok_or_else(|| format!("Im {display_name} fehlt ein erforderlicher Zwischensaldo."))?;
        let change = balance - previous;
        if change.abs() != candidate.amount_minor.abs() {
            return Err(format!(
                "Im {display_name} passen Betrag und Zwischensaldo nicht zusammen."
            ));
        }
        if candidate.amount_minor < 0 {
            if change > 0 {
                debits += candidate.amount_minor;
            } else {
                credits += candidate.amount_minor;
            }
        } else if change > 0 {
            credits += candidate.amount_minor;
        } else {
            debits += candidate.amount_minor;
        }
        amounts.push(change);
        previous = balance;
    }
    if previous != closing || debits != total_debits || credits != total_credits {
        return Err(format!(
            "{display_name} ist unvollständig: Buchungen, Summen oder Schlusssaldo stimmen nicht überein."
        ));
    }
    Ok(amounts)
}

fn credit_indices_for_statement(
    candidates: &[CandidateTransaction],
    opening: i64,
    target: i64,
) -> Option<Vec<usize>> {
    if target == 0 {
        return candidates
            .iter()
            .scan(opening, |balance, candidate| {
                *balance -= candidate.amount_minor.abs();
                Some(
                    candidate
                        .balance_minor
                        .is_none_or(|value| value == *balance),
                )
            })
            .all(|matches| matches)
            .then(Vec::new);
    }
    let mut sums = BTreeMap::from([(0_i64, Vec::<usize>::new())]);
    let mut prefix_total = 0;
    for (index, candidate) in candidates.iter().enumerate() {
        prefix_total += candidate.amount_minor.abs();
        let additions = sums
            .iter()
            .filter_map(|(sum, selected)| {
                let next = sum + candidate.amount_minor.abs();
                (next <= target).then(|| {
                    let mut selected = selected.clone();
                    selected.push(index);
                    (next, selected)
                })
            })
            .collect::<Vec<_>>();
        for (sum, selected) in additions {
            sums.entry(sum).or_insert(selected);
        }
        if let Some(expected_balance) = candidate.balance_minor {
            sums.retain(|credit_sum, _| {
                opening - prefix_total + (2 * credit_sum) == expected_balance
            });
        }
        if sums.len() > 100_000 {
            return None;
        }
    }
    sums.remove(&target)
}

#[cfg(test)]
mod tests {
    use super::parse_bundled;

    #[test]
    fn rejects_unknown_profile_properties() {
        let profile = r#"{"schemaVersion":1,"unknown":true}"#;
        assert!(parse_bundled(profile, "test", "anything").unwrap().is_err());
    }
}
