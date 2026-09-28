//! Definiert UBS-Erkennungsregeln sowie Zuordnungen für Excel-, Konto- und Kreditkartenbelege.

use super::{IntegratedImportProfile, ProviderExcelMapping, ProviderImporter};
use crate::importers::{
    normalize_date, normalized, parse_money, CurrencyBalance, ParsedPdfRows, ParsedStatement,
    ParsedTransaction,
};
use regex::Regex;
use std::path::Path;

const UBS_MONTHLY_ACCOUNT_PROFILE: &str =
    include_str!("profiles/ubs-monthly-account-statement-v1.json");
const PDF_PROFILES: &[&str] = &[UBS_MONTHLY_ACCOUNT_PROFILE];
const UBS_CREDIT_CARD_PROFILE: &str = include_str!("profiles/ubs-credit-card-statement-v1.json");
const CARD_PDF_PROFILES: &[&str] = &[UBS_CREDIT_CARD_PROFILE];
const INTEGRATED_IMPORT_PROFILES: &[IntegratedImportProfile] = &[IntegratedImportProfile {
    id: "ubs-credit-card-csv",
    display_name: "UBS-Kreditkartenumsätze",
    formats: &["CSV"],
    document_type: "credit_card_transactions",
}];

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
    fn supports_provisional_card_csv(&self) -> bool {
        true
    }

    fn card_credit_kind(&self, description: &str) -> Option<&'static str> {
        // Exact UBS payment labels only; a positive amount or generic LSV text is insufficient.
        let label = description
            .split(['·', '\n'])
            .next()
            .unwrap_or("")
            .trim()
            .to_uppercase();
        matches!(label.as_str(), "2002 LSV-ZAHLUNG" | "LSV-ZAHLUNG").then_some("card_settlement")
    }
    fn id(&self) -> &'static str {
        "ubs"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ubs", "ubs switzerland"]
    }

    fn excel_mapping(&self) -> Option<&'static ProviderExcelMapping> {
        Some(&EXCEL_MAPPING)
    }

    fn bundled_pdf_profiles(&self) -> &'static [&'static str] {
        PDF_PROFILES
    }

    fn bundled_card_pdf_profiles(&self) -> &'static [&'static str] {
        CARD_PDF_PROFILES
    }

    fn integrated_import_profiles(&self) -> &'static [IntegratedImportProfile] {
        INTEGRATED_IMPORT_PROFILES
    }

    fn parse_pdf_hook(&self, _path: &Path, text: &str) -> Option<Result<ParsedStatement, String>> {
        Some(parse_pdf_text(text))
    }
}

pub(in crate::importers) fn parse_pdf_text(text: &str) -> Result<ParsedStatement, String> {
    if let Some(result) = crate::importers::formats::pdf_card_profile::parse_bundled(
        UBS_CREDIT_CARD_PROFILE,
        "ubs",
        text,
    ) {
        return result;
    }
    if let Some(result) = crate::importers::formats::pdf_profile::parse_bundled(
        UBS_MONTHLY_ACCOUNT_PROFILE,
        "ubs",
        text,
    ) {
        return result;
    }

    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let account_reference = extract_iban(text);
    let account_name = lines
        .iter()
        .find(|line| line.contains("IBAN"))
        .and_then(|line| line.split('|').next())
        .unwrap_or("Neues UBS-Konto")
        .trim()
        .to_string();
    let (transactions, opening_balance_minor, closing_balance_minor) =
        parse_legacy_account_rows(&lines)?;
    let currency_balances = if transactions.is_empty() {
        match (
            extract_statement_period(text),
            opening_balance_minor,
            closing_balance_minor,
        ) {
            (Some((opening_date, closing_date)), Some(opening), Some(closing)) => {
                vec![CurrencyBalance {
                    currency: "CHF".into(),
                    opening_date: Some(opening_date),
                    opening_balance_minor: opening,
                    closing_balance_minor: closing,
                    closing_date,
                }]
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    Ok(ParsedStatement {
        currency_balances,
        account_type: None,
        provider: "ubs".into(),
        format: "PDF".into(),
        account_name,
        transactions,
        opening_balance_minor,
        closing_balance_minor,
        warnings: vec!["Älteres UBS-PDF-Layout heuristisch erkannt. Bitte Datum, Betrag und Saldo kontrollieren.".into()],
        account_reference,
        ..ParsedStatement::default()
    })
}

fn extract_iban(text: &str) -> Option<String> {
    let pattern = Regex::new(r"(?i)\bCH\d{2}(?:\s*[A-Z0-9]){17}\b").ok()?;
    pattern.find(text).map(|value| {
        value
            .as_str()
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
            .to_uppercase()
    })
}

fn extract_statement_period(text: &str) -> Option<(String, String)> {
    let pattern =
        Regex::new(r"(?m)^\s*(\d{2}\.\d{2}\.\d{4})\s*-\s*(\d{2}\.\d{2}\.\d{4})\s*/").ok()?;
    let captures = pattern.captures(text)?;
    Some((normalize_date(&captures[1])?, normalize_date(&captures[2])?))
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
            ..ParsedTransaction::default()
        });
    }
    if transactions.is_empty() {
        return Err("Im UBS-PDF konnten keine Buchungszeilen erkannt werden.".into());
    }
    let closing = closing.or_else(|| transactions.last().and_then(|row| row.balance_minor));
    Ok((transactions, opening, closing))
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

    fn parse_profile_rows(lines: &[String]) -> Result<ParsedPdfRows, String> {
        let parsed = parse_pdf_text(&lines.join("\n"))?;
        Ok((
            parsed.transactions,
            parsed.opening_balance_minor,
            parsed.closing_balance_minor,
        ))
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
        let (rows, opening, closing) = parse_profile_rows(&lines(text)).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(opening, Some(100_000));
        assert_eq!(closing, Some(117_700));
        assert_eq!(rows[0].amount_minor, 20_000);
        assert!(parse_profile_rows(&lines(&text.replace("TWINT 200.00", "TWINT 199.00"))).is_err());
    }

    #[test]
    fn parses_trailing_negative_balances_and_reordered_credit_labels() {
        let text = "Ihr Konto auf einen Blick
Anfangssaldo 280.41
Total Gutschriften 12 078.20
Total Belastungen 4 000.00
Schlusssaldo 8 358.61
Datum Informationen Belastungen Gutschriften Valuta Kontostand
01.12.14 Anfangssaldo 280.41
19.12.14 E-BANKING-AUFTRAG 2 000.00 19.12.14 1 719.59 -
19.12.14 E-BANKING-AUFTRAG 2 000.00 19.12.14 3 719.59-
19.12.14 12 078.20 20.12.14 8 358.61HSGUTSCHRIFT
Umsatztotal 4 000.00 12 078.20
31.12.14 Schlusssaldo 8 358.61";

        let (rows, opening, closing) = parse_profile_rows(&lines(text)).unwrap();

        assert_eq!(opening, Some(28_041));
        assert_eq!(closing, Some(835_861));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].amount_minor, -200_000);
        assert_eq!(rows[0].balance_minor, Some(-171_959));
        assert_eq!(rows[1].amount_minor, -200_000);
        assert_eq!(rows[1].balance_minor, Some(-371_959));
        assert_eq!(rows[2].description, "HSGUTSCHRIFT");
        assert_eq!(rows[2].amount_minor, 1_207_820);
    }

    #[test]
    fn validates_reversals_against_the_original_ubs_total_column() {
        let text = "Ihr Konto auf einen Blick
Anfangssaldo 1 000.00
Total Gutschriften 200.00
Total Belastungen 50.00
Schlusssaldo 1 150.00
Datum Informationen Belastungen Gutschriften Valuta Kontostand
01.09.18 Anfangssaldo 1 000.00
03.09.18 BELASTUNG 100.00 03.09.18 900.00
07.09.18 STORNOBUCHUNG 50.00- 07.09.18 950.00
28.09.18 GUTSCHRIFT 200.00 28.09.18 1 150.00
Umsatztotal 50.00 200.00
30.09.18 Schlusssaldo 1 150.00";

        let (rows, opening, closing) = parse_profile_rows(&lines(text)).unwrap();

        assert_eq!(opening, Some(100_000));
        assert_eq!(closing, Some(115_000));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].amount_minor, 5_000);
        assert!(rows[1].description.contains("STORNOBUCHUNG"));
    }

    #[test]
    fn accepts_a_valid_balance_only_savings_statement() {
        let text = "UBS Sparkonto CHF
IBAN CH28 0029 0000 0000 0000 0
Kontoauszug
01.01.2019 - 31.12.2019 / Jährlich
Ihr Konto auf einen Blick Belastungen Gutschriften Kontostand
Anfangssaldo 42.67
Total Gutschriften 0.00
Total Belastungen 0.00
Schlusssaldo 42.67
Datum Informationen Belastungen Gutschriften Valuta Kontostand
01.01.19 Anfangssaldo 42.67
Umsatztotal 0.00 0.00
31.12.19 Schlusssaldo 42.67";

        let parsed = parse_pdf_text(text).unwrap();

        assert!(parsed.transactions.is_empty());
        assert_eq!(parsed.opening_balance_minor, Some(4_267));
        assert_eq!(parsed.closing_balance_minor, Some(4_267));
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.account_name, "UBS Sparkonto CHF");
        assert_eq!(
            parsed.account_reference.as_deref(),
            Some("CH2800290000000000000")
        );
        assert_eq!(parsed.currency_balances.len(), 1);
        assert_eq!(
            parsed.currency_balances[0].opening_date.as_deref(),
            Some("2019-01-01")
        );
        assert_eq!(parsed.currency_balances[0].closing_date, "2019-12-31");

        let same_line = text.replace(
            "UBS Sparkonto CHF\nIBAN CH28",
            "UBS Sparkonto CHF | IBAN CH28",
        );
        assert_eq!(
            parse_pdf_text(&same_line).unwrap().account_name,
            "UBS Sparkonto CHF"
        );
    }

    #[test]
    fn parses_compact_text_runs_from_older_pdf_streams() {
        let text = "UBS Privatkonto CHF IBAN CH28 0029 Ihr Konto auf einen Blick Belastungen Gutschriften Kontostand Anfangssaldo 1 000.00 Total Gutschriften 200.00 Total Belastungen 23.00 Schlusssaldo 1 177.00
Datum Informationen Belastungen Gutschriften Valuta Kontostand 01.08.23 Anfangssaldo 1 000.00
02.08.23 STORNO UBS TWINT 200.00 01.08.23 1 200.00 TEST SHOP Referenz 123
Datum Informationen Belastungen Gutschriften Valuta Kontostand 31.08.23 SALDO DIENSTLEISTUNGSPREISABSCHLUSS 23.00 31.08.23 1 177.00 Details
Umsatztotal 23.00 200.00
31.08.23 Schlusssaldo 1 177.00 Formular ohne Unterschrift";

        let (rows, opening, closing) = parse_profile_rows(&lines(text)).unwrap();

        assert_eq!(opening, Some(100_000));
        assert_eq!(closing, Some(117_700));
        assert_eq!(rows.len(), 2);
        assert!(rows[0].description.contains("TEST SHOP"));
    }

    #[test]
    fn extracts_canonical_iban_from_account_statement() {
        assert_eq!(
            extract_iban("UBS Privatkonto CHF\nIBAN CH28 0029 0000 0000 0000 0"),
            Some("CH2800290000000000000".into())
        );
        assert_eq!(extract_iban("UBS Mastercard ohne IBAN"), None);
    }

    #[test]
    fn keeps_dated_payment_details_with_their_transaction() {
        let text = "Ihr Konto auf einen Blick
Anfangssaldo 100.00
Total Gutschriften 20.00
Total Belastungen 10.00
Schlusssaldo 110.00
Datum Informationen Belastungen Gutschriften Valuta Kontostand
01.01.19 Anfangssaldo 100.00
02.01.19 E-BANKING-AUFTRAG 10.00 02.01.19 90.00
21.11.18 CHF 10.00
03.01.19 GUTSCHRIFT 20.00 03.01.19 110.00
Umsatztotal 10.00 20.00
31.01.19 Schlusssaldo 110.00";

        let (rows, _, _) = parse_profile_rows(&lines(text)).unwrap();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].description.contains("21.11.18 CHF 10.00"));
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

    #[test]
    fn credit_card_profile_does_not_claim_another_providers_statement() {
        let result = crate::importers::formats::pdf_card_profile::parse_bundled(
            UBS_CREDIT_CARD_PROFILE,
            "ubs",
            "PostFinance Geschäftskonto\nDatum Text Gutschrift Lastschrift Valuta Saldo",
        );

        assert!(result.is_none());
    }
}
