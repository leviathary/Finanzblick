use super::{ProviderExcelMapping, ProviderImporter};
use crate::importers::{
    normalize_date, normalized, parse_money, CurrencyBalance, ParsedPdfRows, ParsedStatement,
    ParsedTransaction,
};
use regex::Regex;
use std::path::Path;

const MONEY: &str = r"[+-]?(?:\d{1,3}(?:['’ ]\d{3})+|\d+)\.\d{2}";

pub(super) static IMPORTER: UbsImporter = UbsImporter;

static EXCEL_MAPPING: ProviderExcelMapping = ProviderExcelMapping {
    date: &["buchungsdatum", "buchungstag", "datum"],
    value_date: &["valutadatum", "valuta"],
    description: &[
        "beschreibung",
        "informationen",
        "text",
        "buchungstext",
        "vorgang",
    ],
    debit: &["belastung", "soll chf", "soll"],
    credit: &["gutschrift", "haben chf", "haben"],
    amount: &["betrag", "betrag chf"],
    balance: &["saldo", "saldo chf", "kontostand"],
    currency: &["währung", "waehrung"],
    industry: &["branche", "industry"],
};

pub(super) struct UbsImporter;

impl ProviderImporter for UbsImporter {
    fn id(&self) -> &'static str {
        "ubs"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ubs", "ubs switzerland"]
    }

    fn excel_mapping(&self) -> Option<&'static ProviderExcelMapping> {
        Some(&EXCEL_MAPPING)
    }

    fn parse_pdf(&self, _path: &Path, text: &str) -> Option<Result<ParsedStatement, String>> {
        Some(parse_pdf_text(text))
    }
}

pub(in crate::importers) fn parse_pdf_text(text: &str) -> Result<ParsedStatement, String> {
    let clean_lines: Vec<String> = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect();
    if text.contains("Kreditkartenabrechnung in CHF") {
        return parse_mastercard(&clean_lines);
    }

    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let account_name = lines
        .iter()
        .find(|line| line.contains("IBAN"))
        .and_then(|line| line.split('|').next())
        .unwrap_or("Neues UBS-Konto")
        .trim()
        .to_string();
    let validated_from_balances = lines
        .iter()
        .any(|line| line.starts_with("Ihr Konto auf einen Blick"));
    let (transactions, opening_balance_minor, closing_balance_minor) = if validated_from_balances {
        parse_account_rows(&lines)?
    } else {
        parse_legacy_account_rows(&lines)?
    };
    Ok(ParsedStatement {
        currency_balances: Vec::new(),
        account_type: None,
        provider: "ubs".into(),
        format: "PDF".into(),
        account_name,
        transactions,
        opening_balance_minor,
        closing_balance_minor,
        warnings: if validated_from_balances {
            vec!["UBS-PDF anhand von Buchungen, Summen und Salden geprüft.".into()]
        } else {
            vec!["Älteres UBS-PDF-Layout heuristisch erkannt. Bitte Datum, Betrag und Saldo kontrollieren.".into()]
        },
    })
}

/// Older UBS account exports use one compact row with booking date, optional
/// amount, value date and balance. Keep this layout local to UBS as well.
fn parse_legacy_account_rows(lines: &[String]) -> Result<ParsedPdfRows, String> {
    let footer_index = lines
        .iter()
        .position(|line| line.starts_with("Dieses Dokument wurde"))
        .unwrap_or(lines.len());
    let table_start = lines
        .iter()
        .position(|line| {
            let value = normalized(line);
            value.contains("datum") && (value.contains("saldo") || value.contains("kontostand"))
        })
        .ok_or("Keine UBS-Buchungstabelle im PDF gefunden.")?;
    let money = r"[+-]?\d[\d'’]*(?:[.,]\d{2})";
    let row = Regex::new(&format!(
        r"^(\d{{2}}\.\d{{2}}\.\d{{2}}) (.*?) (?:({money}) )?(\d{{2}}\.\d{{2}}\.\d{{2}}) ({money})$"
    ))
    .map_err(|_| "UBS-PDF-Erkennung konnte nicht initialisiert werden.")?;
    let mut transactions = Vec::new();
    let mut opening = None;
    let mut closing = None;
    for (offset, line) in lines[table_start + 1..footer_index].iter().enumerate() {
        let Some(captures) = row.captures(line) else {
            continue;
        };
        let Some(booking_date) = normalize_date(&captures[1]) else {
            continue;
        };
        let description = captures[2].trim().to_string();
        let Some(balance) = parse_money(&captures[5]) else {
            continue;
        };
        let description_key = normalized(&description);
        if description_key.contains("anfangssaldo") || description_key.contains("saldovortrag") {
            opening = Some(balance);
            continue;
        }
        if description_key.contains("schlusssaldo") {
            closing = Some(balance);
            continue;
        }
        let amount = captures
            .get(3)
            .and_then(|value| parse_money(value.as_str()))
            .map(|value| signed_amount(&description_key, value))
            .unwrap_or(0);
        transactions.push(ParsedTransaction {
            booking_date,
            value_date: normalize_date(&captures[4]),
            description,
            industry: None,
            amount_minor: amount,
            balance_minor: Some(balance),
            currency: "CHF".into(),
            confidence: 0.82,
            source_row: table_start + offset + 2,
        });
    }
    if transactions.is_empty() {
        return Err("Im UBS-PDF konnten keine Buchungszeilen erkannt werden.".into());
    }
    let closing = closing.or_else(|| transactions.last().and_then(|row| row.balance_minor));
    Ok((transactions, opening, closing))
}

// UBS monthly statements have a balance on every booking. Its change determines
// the sign without guessing from transaction labels.
pub(in crate::importers) fn parse_account_rows(lines: &[String]) -> Result<ParsedPdfRows, String> {
    if !lines
        .iter()
        .any(|line| line.starts_with("Ihr Konto auf einen Blick"))
    {
        return Err("Das UBS-PDF-Layout wird noch nicht unterstützt.".into());
    }
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
                return Err(format!(
                    "UBS-Auszug: Betrag und Saldo passen in Zeile {} nicht zusammen. Import abgebrochen.",
                    index + 1
                ));
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

fn amount(value: &str) -> Result<i64, String> {
    parse_money(value).ok_or_else(|| "Betrag im UBS-PDF ist unlesbar.".into())
}

fn parse_mastercard(lines: &[String]) -> Result<ParsedStatement, String> {
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
    Ok(ParsedStatement {
        provider: "ubs".into(),
        format: "PDF".into(),
        account_name: format!("UBS Mastercard · {account}"),
        opening_balance_minor: Some(opening),
        closing_balance_minor: Some(closing),
        transactions: rows,
        currency_balances: vec![CurrencyBalance {
            currency: "CHF".into(),
            opening_date,
            opening_balance_minor: opening,
            closing_balance_minor: closing,
            closing_date: date,
        }],
        account_type: Some("credit_card".into()),
        warnings,
    })
}

pub(in crate::importers) fn signed_amount(description: &str, amount: i64) -> i64 {
    if amount < 0
        || !(normalized(description).contains("lohn")
            || normalized(description).contains("gutschrift"))
    {
        -amount.abs()
    } else {
        amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn validates_monthly_account_totals() {
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
31.08.23 Schlusssaldo 1 177.00";
        let (rows, opening, closing) = parse_account_rows(&lines(text)).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(opening, Some(100_000));
        assert_eq!(closing, Some(117_700));
        assert_eq!(rows[0].amount_minor, 20_000);
        assert!(parse_account_rows(&lines(&text.replace("TWINT 200.00", "TWINT 199.00"))).is_err());
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
        let parsed = parse_pdf_text(text).unwrap();
        let same_page = text
            .replace("30.04.2021 Rechnungsbetrag 95.00", "30.04.2021 Kartentotal SECOND, UBS Mastercard Gold 10.00\n30.04.2021 Rechnungsbetrag 105.00")
            + "\nSECOND, UBS Mastercard Gold, XXXX 5678\n05.04.2021 SECOND SHOP\n04.04.2021\n10.00\nÜbertrag auf Seite 3 10.00\nBuchungsdatum Detail Betrag CHF\nÜbertrag von Seite 2 10.00\nKartentotal 10.00";
        let second_card = parse_pdf_text(&same_page).unwrap();
        assert_eq!(second_card.closing_balance_minor, Some(-10500));
        assert!(second_card.transactions.iter().any(|row| {
            row.description.contains("SECOND SHOP")
                && row.description.contains("XXXX 5678")
                && row.amount_minor == -1000
        }));
        assert_eq!(parsed.transactions.len(), 4);
        assert_eq!(parsed.account_type.as_deref(), Some("credit_card"));
        assert_eq!(parsed.closing_balance_minor, Some(-9500));
        assert!(parsed
            .transactions
            .iter()
            .any(|row| row.amount_minor == -10000 && row.description.contains("USD 110.00")));
        assert!(parsed
            .transactions
            .iter()
            .any(|row| row.amount_minor == 499));
        assert!(parsed.transactions.iter().any(|row| row.amount_minor == 1));
        assert!(parse_pdf_text(&text.replace("Kartentotal 95.01", "Kartentotal 95.02")).is_err());
        assert!(parse_pdf_text(&text.replace("USD 110.00 100.00", "")).is_err());
        assert!(
            parse_pdf_text(&text.replace("Rechnungsbetrag 95.00", "Rechnungsbetrag 96.00"))
                .is_err()
        );
    }
}
