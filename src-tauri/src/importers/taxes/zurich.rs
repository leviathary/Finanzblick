//! Liest elektronische Zürcher Steuererklärungen ohne Datenbankzugriff.
use crate::domain::taxes::{validate_amounts, ParsedTaxStatement};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
pub(crate) const PARSER_VERSION: &str = "zurich-tax-v2";
#[derive(Clone, Copy, Debug)]
struct AmountOccurrence {
    value: i64,
    start: usize,
    end: usize,
}

pub(crate) fn parse_tax_statement_file(path: &Path) -> Result<ParsedTaxStatement, String> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_lowercase)
        .as_deref()
        != Some("pdf")
    {
        return Err("Bitte eine Steuererklärung im PDF-Format auswählen.".into());
    }
    let bytes =
        fs::read(path).map_err(|_| "Die PDF-Datei ist nicht mehr verfügbar.".to_string())?;
    let source_hash = format!("{:x}", Sha256::digest(&bytes));
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|_| "Der Text dieser PDF-Datei konnte nicht gelesen werden.".to_string())?;
    let source_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Steuererklärung.pdf")
        .to_string();
    let tax_year = extract_tax_year(&text, path)?;
    let legacy_reversed_digits =
        text.contains("Total5der5Vermögenswerte5") || text.contains("Vermögen5im5In45und5Ausland");
    let amounts = extract_amounts(&text, legacy_reversed_digits);
    let (gross, liabilities, taxable) = (if legacy_reversed_digits {
        find_legacy_summary_triplet(&text)
    } else {
        find_summary_triplet(&amounts)
    }).ok_or_else(|| {
        "Die Vermögenssummen wurden nicht sicher erkannt. Diese erste Version unterstützt elektronisch erzeugte Zürcher Steuererklärungen; gescannte PDFs folgen später."
            .to_string()
    })?;
    let gross_assets_minor = gross
        .checked_mul(100)
        .ok_or_else(|| "Der erkannte Vermögenswert ist zu gross.".to_string())?;
    let liabilities_minor = liabilities
        .checked_mul(100)
        .ok_or_else(|| "Der erkannte Schuldenwert ist zu gross.".to_string())?;
    let taxable_wealth_minor = taxable
        .checked_mul(100)
        .ok_or_else(|| "Der erkannte Steuerwert ist zu gross.".to_string())?;
    let (securities_and_cash, real_estate) = if legacy_reversed_digits {
        find_legacy_breakdown(&text, gross).unwrap_or((0, 0))
    } else {
        (
            extract_breakdown_value(&text, "Wertschriften und Guthaben", false).unwrap_or(0),
            extract_breakdown_value(&text, "Liegenschaften (Repartitionswerte)", false)
                .unwrap_or(0),
        )
    };
    let securities_and_cash_minor = securities_and_cash
        .checked_mul(100)
        .ok_or_else(|| "Der erkannte Wertschriftenwert ist zu gross.".to_string())?;
    let real_estate_minor = real_estate
        .checked_mul(100)
        .ok_or_else(|| "Der erkannte Liegenschaftswert ist zu gross.".to_string())?;
    let other_assets_minor = gross_assets_minor
        .checked_sub(securities_and_cash_minor)
        .and_then(|value| value.checked_sub(real_estate_minor))
        .ok_or_else(|| "Die erkannte Vermögensaufteilung ist zu gross.".to_string())?;
    validate_amounts(
        gross_assets_minor,
        liabilities_minor,
        taxable_wealth_minor,
        None,
    )?;
    let mut warnings = Vec::new();
    if securities_and_cash_minor == 0 || real_estate_minor == 0 {
        warnings.push(
            "Die Vermögensaufteilung konnte nicht vollständig erkannt werden. Bitte Wertschriften, Guthaben und Liegenschaften in der Vorschau prüfen."
                .into(),
        );
    }
    let confidence = if legacy_reversed_digits {
        warnings.push(
            "Älteres PDF-Format erkannt. Bitte die Werte in der Vorschau besonders sorgfältig prüfen."
                .into(),
        );
        0.86
    } else {
        0.98
    };
    Ok(ParsedTaxStatement {
        source_name,
        source_hash,
        tax_year,
        gross_assets_minor,
        liabilities_minor,
        taxable_wealth_minor,
        securities_and_cash_minor,
        real_estate_minor,
        other_assets_minor,
        confidence,
        warnings,
    })
}

fn extract_breakdown_value(text: &str, label: &str, legacy_reversed_digits: bool) -> Option<i64> {
    text.lines()
        .filter(|line| line.contains(label))
        .filter(|line| !line.contains("Ertrag aus"))
        .filter_map(|line| {
            let remainder = line.split_once(label)?.1;
            parse_first_column_amount(remainder, legacy_reversed_digits)
        })
        .filter(|value| *value >= 1_000)
        .max()
}

fn find_legacy_summary_triplet(text: &str) -> Option<(i64, i64, i64)> {
    let regex = Regex::new(r"(?s)(\d{5,8})\s+0\s+(\d{4,8})\s+\*\s+(\d{5,8})\s+0")
        .expect("valid legacy summary regex");
    let result = regex.captures_iter(text).find_map(|capture| {
        let decode = |index| {
            capture
                .get(index)?
                .as_str()
                .chars()
                .rev()
                .collect::<String>()
                .parse::<i64>()
                .ok()
        };
        let gross = decode(1)?;
        let liabilities = decode(2)?;
        let taxable = decode(3)?;
        (gross >= 10_000 && gross.checked_sub(liabilities) == Some(taxable)).then_some((
            gross,
            liabilities,
            taxable,
        ))
    });
    result
}

fn find_legacy_breakdown(text: &str, gross: i64) -> Option<(i64, i64)> {
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        if *line != "BE" {
            continue;
        }
        let pairs = lines
            .iter()
            .skip(index + 1)
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let values = line.split_whitespace().collect::<Vec<_>>();
                (values.len() == 2
                    && values[0] == values[1]
                    && values[0].chars().all(|value| value.is_ascii_digit()))
                .then_some(values[0])
            })
            .take(3)
            .collect::<Vec<_>>();
        if pairs.len() != 3 {
            continue;
        }
        let decode = |value: &str| value.chars().rev().collect::<String>().parse::<i64>().ok();
        let property_without_adjustment = decode(pairs[0])?;
        let securities = decode(pairs[1])?;
        let other_assets = decode(pairs[2])?;
        let remaining_assets = gross
            .checked_sub(securities)?
            .checked_sub(property_without_adjustment)?;
        if property_without_adjustment > 0
            && securities > 0
            && other_assets > 0
            && remaining_assets >= other_assets
        {
            return Some((securities, property_without_adjustment));
        }
    }
    None
}

fn parse_first_column_amount(text: &str, legacy_reversed_digits: bool) -> Option<i64> {
    let regex = Regex::new(r"\d+").expect("valid digit regex");
    let groups = regex
        .find_iter(text)
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let first = *groups.first()?;
    if legacy_reversed_digits && (5..=8).contains(&first.len()) {
        return first.chars().rev().collect::<String>().parse().ok();
    }
    let mut amount = first.parse::<i64>().ok()?;
    for group in groups.iter().skip(1) {
        if group.len() != 3 {
            break;
        }
        let candidate = amount
            .checked_mul(1_000)?
            .checked_add(group.parse().ok()?)?;
        if candidate > 100_000_000 {
            break;
        }
        amount = candidate;
    }
    Some(amount)
}

fn extract_tax_year(text: &str, path: &Path) -> Result<i32, String> {
    let patterns = [
        r"(?i)Steuerwert\s+am\s+31\.\s*Dezember\s+(\d{4})",
        r"(?i)Vermögen.{0,24}31\.12\.(\d{4})",
        r"(?i)Steuererklärung\s+(\d{4})",
    ];
    for pattern in patterns {
        let regex = Regex::new(pattern).expect("valid tax year regex");
        if let Some(year) = regex
            .captures(text)
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<i32>().ok())
            .filter(|year| (1990..=2100).contains(year))
        {
            return Ok(year);
        }
    }
    let path_year = Regex::new(r"(?:^|[^0-9])(20\d{2})(?:[^0-9]|$)")
        .expect("valid path year regex")
        .captures_iter(&path.to_string_lossy())
        .filter_map(|capture| capture.get(1)?.as_str().parse::<i32>().ok())
        .find(|year| (2000..=2100).contains(year));
    path_year.ok_or_else(|| "Das Steuerjahr konnte nicht erkannt werden.".into())
}

fn extract_amounts(text: &str, legacy_reversed_digits: bool) -> Vec<AmountOccurrence> {
    let regex =
        Regex::new(r"\d{1,3}(?:[ \t\u{00a0}\u{202f}’'™]\d{3})+|\d+").expect("valid amount regex");
    regex
        .find_iter(text)
        .filter_map(|found| {
            let mut digits: String = found
                .as_str()
                .chars()
                .filter(char::is_ascii_digit)
                .collect();
            if legacy_reversed_digits
                && found
                    .as_str()
                    .chars()
                    .all(|character| character.is_ascii_digit())
                && (5..=8).contains(&digits.len())
            {
                digits = digits.chars().rev().collect();
            }
            let value = digits.parse::<i64>().ok()?;
            if (value >= 1_000 || value == 0) && value <= 100_000_000 {
                Some(AmountOccurrence {
                    value,
                    start: found.start(),
                    end: found.end(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_summary_triplet(amounts: &[AmountOccurrence]) -> Option<(i64, i64, i64)> {
    amounts
        .windows(3)
        .filter_map(|values| {
            let [gross, liabilities, taxable] = values else {
                return None;
            };
            let span = taxable.end.saturating_sub(gross.start);
            (gross.value >= 10_000
                && liabilities.value >= 0
                && span <= 100
                && gross.value.checked_sub(liabilities.value) == Some(taxable.value))
            .then_some((gross.value, liabilities.value, taxable.value))
        })
        .max_by_key(|(gross, _, _)| *gross)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn occurrences(values: &[i64]) -> Vec<AmountOccurrence> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| AmountOccurrence {
                value: *value,
                start: index * 10,
                end: index * 10 + 7,
            })
            .collect()
    }

    #[test]
    fn selects_largest_plausible_summary_triplet() {
        let values = occurrences(&[
            182_955, 23_355, 159_600, 1_730_012, 1_142_691, 587_321, 48_742,
        ]);
        assert_eq!(
            find_summary_triplet(&values),
            Some((1_730_012, 1_142_691, 587_321))
        );
    }

    #[test]
    fn decodes_legacy_reversed_amounts() {
        let text = "531474 68561 945754";
        let values = extract_amounts(text, true);
        assert_eq!(
            find_summary_triplet(&values),
            Some((474_135, 16_586, 457_549))
        );
    }

    #[test]
    fn rejects_inconsistent_manual_values() {
        assert!(validate_amounts(100_000, 20_000, 70_000, None).is_err());
        assert!(validate_amounts(100_000, 20_000, 80_000, None).is_ok());
    }

    #[test]
    fn reads_the_first_amount_column_from_tax_allocation_rows() {
        assert_eq!(
            parse_first_column_amount(" 391 511 391 511", false),
            Some(391_511)
        );
        assert_eq!(
            parse_first_column_amount(" 1 103 508 611 800 491 708", false),
            Some(1_103_508)
        );
        assert_eq!(
            parse_first_column_amount(" 270™397 270™397", false),
            Some(270_397)
        );
    }

    #[test]
    fn reads_legacy_summary_and_breakdown() {
        let summary = "531474 0\n68561 *\n945754 0";
        assert_eq!(
            find_legacy_summary_triplet(summary),
            Some((474_135, 16_586, 457_549))
        );
        let allocation = "BE\n763201 763201\n423013 423013\n\n07005 07005";
        assert_eq!(
            find_legacy_breakdown(allocation, 474_135),
            Some((310_324, 102_367))
        );
    }
}
