use super::*;
use regex::Regex;

const MONEY: &str = r"[+-]?(?:\d{1,3}(?:['’ ]\d{3})+|\d+)\.\d{2}";

fn amount(value: &str) -> Result<i64, String> {
    parse_money(value).ok_or_else(|| "Betrag im PDF ist unlesbar.".into())
}

fn statement(
    provider: &str,
    name: String,
    rows: Vec<ParsedTransaction>,
    balances: Vec<CurrencyBalance>,
    account_type: &str,
    warnings: Vec<String>,
) -> ParsedStatement {
    ParsedStatement {
        provider: provider.into(),
        format: "PDF".into(),
        account_name: name,
        opening_balance_minor: if balances.len() == 1 {
            Some(balances[0].opening_balance_minor)
        } else {
            None
        },
        closing_balance_minor: if balances.len() == 1 {
            Some(balances[0].closing_balance_minor)
        } else {
            None
        },
        transactions: rows,
        currency_balances: balances,
        account_type: Some(account_type.into()),
        warnings,
    }
}

pub(super) fn swissquote(lines: &[String]) -> Result<ParsedStatement, String> {
    let sections: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| line.starts_with("Kontoauszug in ").then_some(i))
        .collect();
    if sections.is_empty() {
        return Err("Keine Swissquote-Währungskonten gefunden.".into());
    }
    let start = Regex::new(r"^(\d{2}\.\d{2}\.\d{4}) (.*)$").unwrap();
    let tail = Regex::new(&format!(
        r"^(.*?)\s+({MONEY}) (\d{{2}}\.\d{{2}}\.\d{{4}}) ({MONEY})$"
    ))
    .unwrap();
    let saldo = Regex::new(&format!(
        r"^Saldo per (\d{{2}}\.\d{{2}}\.\d{{4}}) ({MONEY}) ([A-Z]{{3}})$"
    ))
    .unwrap();
    let mut rows = Vec::new();
    let mut balances = Vec::new();
    for (section, &begin) in sections.iter().enumerate() {
        let end = sections.get(section + 1).copied().unwrap_or(lines.len());
        let currency = lines[begin]
            .strip_prefix("Kontoauszug in ")
            .unwrap()
            .to_string();
        if balances
            .iter()
            .any(|b: &CurrencyBalance| b.currency == currency)
        {
            return Err("Doppelter Swissquote-Währungsabschnitt.".into());
        }
        let data = &lines[begin + 1..end];
        let saldos: Vec<_> = data
            .iter()
            .filter_map(|line| saldo.captures(line))
            .collect();
        if saldos.len() != 2 || saldos.iter().any(|s| s[3] != currency) {
            return Err("Swissquote: Anfangs-/Endsaldo fehlt.".into());
        }
        let opening = amount(&saldos[0][2])?;
        let closing = amount(&saldos[1][2])?;
        let total = |label: &str| -> Result<i64, String> {
            let line = data
                .iter()
                .find_map(|line| line.strip_prefix(label))
                .ok_or("Swissquote: Umsatzsumme fehlt.")?;
            amount(
                line.strip_suffix(&format!(" {currency}"))
                    .ok_or("Swissquote: falsche Summenwährung.")?,
            )
        };
        let debit = total("Total Belastung ")?;
        let credit = total("Total Gutschrift ")?;
        let mut previous = opening;
        let mut active = false;
        let mut finished = false;
        let mut pending: Option<(String, String, usize)> = None;
        let first_row = rows.len();
        for (offset, line) in data.iter().enumerate() {
            if line.starts_with("DATUM INFORMATION") {
                active = true;
                continue;
            }
            if line.starts_with("Dieser Transaktionsbeleg")
                || line.starts_with("Dokument erstellt am")
            {
                active = false;
                continue;
            }
            if !active {
                continue;
            }
            if let Some(c) = start.captures(line) {
                if pending.is_some() {
                    return Err("Swissquote: unvollständige Buchung.".into());
                }
                if c[2].starts_with("Anfangsbestand ") {
                    if amount(c[2].strip_prefix("Anfangsbestand ").unwrap())? != opening {
                        return Err("Swissquote: Anfangssaldo widersprüchlich.".into());
                    }
                    continue;
                }
                if c[2].starts_with("Schlussbilanz ") {
                    if amount(c[2].strip_prefix("Schlussbilanz ").unwrap())? != closing {
                        return Err("Swissquote: Endsaldo widersprüchlich.".into());
                    }
                    finished = true;
                    break;
                }
                pending = Some((
                    normalize_date(&c[1]).ok_or("Swissquote: ungültiges Datum.")?,
                    c[2].to_string(),
                    begin + offset + 2,
                ));
            } else if let Some((_, description, _)) = pending.as_mut() {
                description.push(' ');
                description.push_str(line);
            } else if line != "0.00" {
                return Err(format!(
                    "Swissquote: unerwartete Zeile {} in der Buchungstabelle: {line}",
                    begin + offset + 2
                ));
            }
            if let Some((date, text, source_row)) = pending.as_ref() {
                if let Some(c) = tail.captures(text) {
                    let balance = amount(&c[4])?;
                    let change = balance - previous;
                    if change.abs() != amount(&c[2])?.abs() {
                        return Err(format!("Swissquote {currency}: Betrag und laufender Saldo stimmen nicht überein (Zeile {source_row})."));
                    }
                    rows.push(ParsedTransaction {
                        booking_date: date.clone(),
                        value_date: Some(
                            normalize_date(&c[3]).ok_or("Swissquote: ungültige Valuta.")?,
                        ),
                        description: c[1].to_string(),
                        industry: None,
                        amount_minor: change,
                        balance_minor: Some(balance),
                        currency: currency.clone(),
                        confidence: 0.99,
                        source_row: *source_row,
                    });
                    previous = balance;
                    pending = None;
                }
            }
        }
        let credits: i64 = rows[first_row..]
            .iter()
            .map(|r| r.amount_minor.max(0))
            .sum();
        let debits: i64 = rows[first_row..]
            .iter()
            .map(|r| (-r.amount_minor).max(0))
            .sum();
        if !finished
            || pending.is_some()
            || previous != closing
            || credits != credit
            || debits != debit
        {
            return Err(format!(
                "Swissquote {currency}: Buchungen oder Summen sind unvollständig."
            ));
        }
        balances.push(CurrencyBalance {
            currency,
            opening_date: normalize_date(&saldos[0][1]),
            opening_balance_minor: opening,
            closing_balance_minor: closing,
            closing_date: normalize_date(&saldos[1][1])
                .ok_or("Swissquote: ungültiges Abschlussdatum.")?,
        });
    }
    let iban = lines
        .iter()
        .find(|line| line.starts_with("IBAN "))
        .ok_or("Swissquote: IBAN fehlt.")?;
    let overview_balance = Regex::new(&format!(r"^({MONEY}) ([A-Z]{{3}})$")).unwrap();
    for line in &lines[..sections[0]] {
        if let Some(c) = overview_balance.captures(line) {
            let expected = amount(&c[1])?;
            if !balances.iter().any(|balance| {
                balance.currency == c[2] && balance.closing_balance_minor == expected
            }) {
                return Err(format!(
                    "Swissquote: Währungsabschnitt {} fehlt oder widerspricht der Übersicht.",
                    &c[2]
                ));
            }
        }
    }
    Ok(statement("swissquote", format!("Swissquote Trading · {iban}"), rows, balances, "cash", vec!["Währungskonten werden getrennt gespeichert. Der umgerechnete CHF-Gesamtsaldo und Wertpapierbestände sind keine zusätzlichen Kontoguthaben.".into()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn swissquote_validates_currencies_and_split_rows() {
        let text = "IBAN CH00 TEST
Kontoauszug in CHF
Saldo per 01.01.2024 0.00 CHF
Total Belastung 0.00 CHF
Total Gutschrift 129.35 CHF
Saldo per 31.12.2024 129.35 CHF
DATUM INFORMATION REFERENZ BELASTUNG GUTSCHRIFT VALUTA-DATUM SALDO (CHF)
01.01.2024 Anfangsbestand 0.00
02.01.2024 Verkauf
TEST ASSET
550181750 129.35 03.01.2024 129.35
31.12.2024 Schlussbilanz 129.35
Kontoauszug in EUR
Saldo per 01.01.2024 1.81 EUR
Total Belastung 0.00 EUR
Total Gutschrift 0.00 EUR
Saldo per 31.12.2024 1.81 EUR
DATUM INFORMATION REFERENZ BELASTUNG GUTSCHRIFT VALUTA-DATUM SALDO (EUR)
01.01.2024 Anfangsbestand 1.81
0.00
31.12.2024 Schlussbilanz 1.81";
        let parsed = swissquote(&lines(text)).unwrap();
        assert_eq!(parsed.transactions.len(), 1);
        assert_eq!(parsed.transactions[0].amount_minor, 12935);
        assert!(parsed.transactions[0].description.contains("550181750"));
        assert_eq!(parsed.currency_balances.len(), 2);
        assert_eq!(parsed.closing_balance_minor, None);
        assert!(swissquote(&lines(
            &text.replace("Total Gutschrift 129.35", "Total Gutschrift 129.34")
        ))
        .is_err());
        assert!(swissquote(&lines(&text.replace("31.12.2024 Schlussbilanz 1.81", ""))).is_err());
        assert!(swissquote(&lines(
            &text.replace("550181750 129.35", "550181750 129.34")
        ))
        .is_err());
    }
}
