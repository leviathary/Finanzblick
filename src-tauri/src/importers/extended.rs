use super::*;

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

pub(super) fn mastercard(lines: &[String]) -> Result<ParsedStatement, String> {
    let dated_amount =
        Regex::new(&format!(r"^(\d{{2}}\.\d{{2}}\.\d{{4}}) (.*?) ({MONEY})$")).unwrap();
    let dated = Regex::new(r"^(\d{2}\.\d{2}\.\d{4}) (.+)$").unwrap();
    let payment_tail = Regex::new(&format!(
        r"^(?:(?:[A-Z]{{3}} {MONEY}|\d{{2}}\.\d{{2}}\.\d{{4}}) )?({MONEY})$"
    ))
    .unwrap();
    let purchase_date = Regex::new(r"\d{2}\.\d{2}\.\d{4}").unwrap();
    let mut rows: Vec<ParsedTransaction> = Vec::new();
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
        if line.starts_with("Buchungsdatum Detail Betrag CHF") {
            active = true;
            detail_mode = true;
            continue;
        }
        if !detail_mode {
            if let Some(c) = dated_amount.captures(line) {
                let value = amount(&c[3])?;
                match &c[2] {
                    "Betrag letzte Rechnung" => {
                        opening = Some(-value);
                        opening_date = normalize_date(&c[1]);
                        continue;
                    }
                    "Rechnungsbetrag" => {
                        closing = Some(-value);
                        closing_date = normalize_date(&c[1]);
                        continue;
                    }
                    label if label.starts_with("Kartentotal ") => {
                        expected_card_totals.push(value);
                        continue;
                    }
                    _ => {}
                }
                rows.push(ParsedTransaction {
                    booking_date: normalize_date(&c[1]).ok_or("Ungültiges Rechnungsdatum.")?,
                    value_date: None,
                    description: c[2].into(),
                    industry: None,
                    amount_minor: -value,
                    balance_minor: None,
                    currency: "CHF".into(),
                    confidence: 0.99,
                    source_row: index + 1,
                });
            }
            continue;
        }
        if line.starts_with("Übertrag auf Seite") || line.starts_with("Kartentotal ") {
            if pending.is_some() {
                return Err("Mastercard: unvollständige Einzelbuchung.".into());
            }
            let value = line
                .rsplit_once(' ')
                .and_then(|(_, value)| parse_money(value))
                .ok_or("Mastercard: unlesbares Kartentotal.")?;
            if value != card_total {
                return Err(format!("Mastercard: Karten-/Seitenbetrag in Zeile {} stimmt nicht überein: erkannt {:.2} CHF, ausgewiesen {:.2} CHF.", index + 1, card_total as f64 / 100.0, value as f64 / 100.0));
            }
            if line.starts_with("Kartentotal ") {
                checked_card_totals.push(value);
                card_total = 0;
                card.clear();
            }
            active = false;
            continue;
        }
        // A second card can start immediately after a card total on the same
        // page, without another table header.
        if line.contains(", UBS Mastercard") {
            if pending.is_some() || card_total != 0 {
                return Err("Mastercard: Kartenwechsel vor Abschluss der vorherigen Karte.".into());
            }
            card = line.clone();
            active = true;
            continue;
        }
        if !active {
            continue;
        }
        if line.starts_with("Übertrag von Seite") {
            continue;
        }
        if line.starts_with("XXXX ") {
            card.push(' ');
            card.push_str(line);
            continue;
        }
        // A surcharge's transaction date and amount can share a line. Finish
        // pending rows before considering a new booking-date prefix.
        if let Some(c) = payment_tail.captures(line) {
            let (date, mut description, source_row) =
                pending.take().ok_or("Mastercard: Betrag ohne Buchung.")?;
            let value = amount(&c[1])?;
            if line.starts_with(|c: char| c.is_ascii_alphabetic()) || purchase_date.is_match(line) {
                description.push('\n');
                description.push_str(line);
            }
            if !card.is_empty() {
                description.push('\n');
                description.push_str(&card);
            }
            rows.push(ParsedTransaction {
                booking_date: date,
                value_date: None,
                description,
                industry: None,
                amount_minor: -value,
                balance_minor: None,
                currency: "CHF".into(),
                confidence: 0.99,
                source_row,
            });
            card_total += value;
        } else if let Some(c) = dated.captures(line) {
            if pending.is_some() {
                return Err("Mastercard: Betrag einer Buchung fehlt.".into());
            }
            pending = Some((
                normalize_date(&c[1]).ok_or("Mastercard: ungültiges Buchungsdatum.")?,
                c[2].into(),
                index + 1,
            ));
        } else if let Some((_, description, _)) = pending.as_mut() {
            description.push('\n');
            description.push_str(line);
        } else {
            return Err("Mastercard: unerwartete Zeile in der Buchungstabelle.".into());
        }
    }
    if pending.is_some()
        || expected_card_totals.is_empty()
        || expected_card_totals != checked_card_totals
    {
        return Err("Mastercard: Kartendetails fehlen oder stimmen nicht überein.".into());
    }
    let opening = opening.ok_or("Mastercard: Vorrechnung fehlt.")?;
    let closing = closing.ok_or("Mastercard: Rechnungsbetrag fehlt.")?;
    let date = closing_date.ok_or("Mastercard: Rechnungsdatum fehlt.")?;
    let difference = closing - opening - rows.iter().map(|r| r.amount_minor).sum::<i64>();
    let mut warnings = vec!["Kreditkartenschulden werden negativ geführt. Käufe und Gebühren sind Belastungen, Rückerstattungen und LSV-Zahlungen Gutschriften. Transaktionsdaten und Fremdwährungsdetails stehen im Buchungstext.".into()];
    if difference != 0 {
        if difference.abs() > 5 {
            return Err(
                "Mastercard: Rechnungsbetrag stimmt nicht mit den Buchungen überein.".into(),
            );
        }
        warnings.push(format!("Die Rechnung weist eine Rundungsdifferenz von {difference} Rappen auf; sie wird als eigene Ausgleichsbuchung erfasst."));
        rows.push(ParsedTransaction {
            booking_date: date.clone(),
            value_date: None,
            description: "Rundungsausgleich zum ausgewiesenen Rechnungsbetrag".into(),
            industry: None,
            amount_minor: difference,
            balance_minor: None,
            currency: "CHF".into(),
            confidence: 0.9,
            source_row: lines.len() + 1,
        });
    }
    rows.sort_by(|a, b| {
        a.booking_date
            .cmp(&b.booking_date)
            .then(a.source_row.cmp(&b.source_row))
    });
    let account = lines
        .iter()
        .find(|line| line.starts_with("Kartenkonto "))
        .ok_or("Mastercard: Kartenkonto fehlt.")?;
    Ok(statement(
        "ubs",
        format!("UBS Mastercard · {account}"),
        rows,
        vec![CurrencyBalance {
            currency: "CHF".into(),
            opening_date,
            opening_balance_minor: opening,
            closing_balance_minor: closing,
            closing_date: date,
        }],
        "credit_card",
        warnings,
    ))
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

    #[test]
    fn mastercard_checks_detail_totals_and_preserves_foreign_amounts() {
        let text = "Kreditkartenabrechnung in CHF
01.04.2021 Betrag letzte Rechnung 10.00
02.04.2021 LSV-Zahlung -10.00
30.04.2021 Kartentotal TEST, UBS Mastercard Gold 95.01
30.04.2021 Rechnungsbetrag 95.00
Kartenkonto TEST
Buchungsdatum Detail Betrag CHF
TEST, UBS Mastercard Gold, XXXX 1234
03.04.2021 TEST SHOP
02.04.2021
USD 110.00 100.00
04.04.2021 REFUND
03.04.2021
-4.99
Kartentotal 95.01";
        let parsed = mastercard(&lines(text)).unwrap();
        let same_page = text
            .replace("30.04.2021 Rechnungsbetrag 95.00", "30.04.2021 Kartentotal SECOND, UBS Mastercard Gold 10.00\n30.04.2021 Rechnungsbetrag 105.00")
            + "\nSECOND, UBS Mastercard Gold, XXXX 5678\n05.04.2021 SECOND SHOP\n04.04.2021\n10.00\nÜbertrag auf Seite 3 10.00\nBuchungsdatum Detail Betrag CHF\nÜbertrag von Seite 2 10.00\nKartentotal 10.00";
        let second_card = mastercard(&lines(&same_page)).unwrap();
        assert_eq!(second_card.closing_balance_minor, Some(-10500));
        assert!(second_card
            .transactions
            .iter()
            .any(|row| row.description.contains("SECOND SHOP")
                && row.description.contains("XXXX 5678")
                && row.amount_minor == -1000));
        assert_eq!(parsed.transactions.len(), 4);
        assert_eq!(parsed.account_type.as_deref(), Some("credit_card"));
        assert_eq!(parsed.closing_balance_minor, Some(-9500));
        assert!(parsed
            .transactions
            .iter()
            .any(|r| r.amount_minor == -10000 && r.description.contains("USD 110.00")));
        assert!(parsed.transactions.iter().any(|r| r.amount_minor == 499));
        assert!(parsed.transactions.iter().any(|r| r.amount_minor == 1));
        assert!(mastercard(&lines(
            &text.replace("Kartentotal 95.01", "Kartentotal 95.02")
        ))
        .is_err());
        assert!(mastercard(&lines(&text.replace("USD 110.00 100.00", ""))).is_err());
        assert!(mastercard(&lines(
            &text.replace("Rechnungsbetrag 95.00", "Rechnungsbetrag 96.00")
        ))
        .is_err());
    }
}
