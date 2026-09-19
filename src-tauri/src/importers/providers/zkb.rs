//! Liest ZKB-Belegmetadaten aus PDFs und übersetzt sie in normalisierte Importdaten.

use super::{ParsedStatement, ProviderImporter};
use crate::importers::{
    normalize_date, normalized, parse_money, ParsedSecurityDetails, ParsedTransaction,
};
use lopdf::Document;
use std::{collections::HashMap, path::Path};

pub(super) static IMPORTER: ZkbImporter = ZkbImporter;

pub(super) struct ZkbImporter;

impl ProviderImporter for ZkbImporter {
    fn id(&self) -> &'static str {
        "zkb"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["zürcher kantonalbank", "zuercher kantonalbank", "zkb"]
    }

    fn parse_pdf(&self, path: &Path, _text: &str) -> Option<Result<ParsedStatement, String>> {
        let keywords = pdf_keywords(path).ok().flatten()?;
        let metadata = parse_metadata(&keywords);
        if !is_transaction_notice(&metadata) {
            return None;
        }
        Some(statement_from_metadata(&metadata))
    }
}

fn pdf_keywords(path: &Path) -> Result<Option<String>, String> {
    Document::load_metadata(path)
        .or_else(|_| Document::load_metadata_with_password(path, ""))
        .map(|metadata| metadata.keywords)
        .map_err(|error| format!("ZKB-PDF-Metadaten konnten nicht gelesen werden: {error}"))
}

fn parse_metadata(keywords: &str) -> HashMap<String, String> {
    keywords
        .split(';')
        .filter_map(|pair| {
            let (label, value) = pair.split_once(':')?;
            let value = value.trim();
            (!value.is_empty()).then(|| (normalized(label), value.to_string()))
        })
        .collect()
}

fn is_transaction_notice(metadata: &HashMap<String, String>) -> bool {
    metadata
        .get("belegkategorie")
        .is_some_and(|value| value.trim() == "30")
        || metadata.get("belegtitel").is_some_and(|value| {
            let title = normalized(value);
            title.contains("gutschrift") || title.contains("belastung")
        })
}

fn required<'a>(metadata: &'a HashMap<String, String>, key: &str) -> Result<&'a str, String> {
    metadata
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("ZKB-PDF-Metadatum fehlt: {key}."))
}

fn statement_from_metadata(metadata: &HashMap<String, String>) -> Result<ParsedStatement, String> {
    let title = required(metadata, "belegtitel")?.trim();
    let title_key = normalized(title);
    let document_date =
        normalize_date(required(metadata, "belegdatum")?).ok_or("ZKB-Belegdatum ist ungültig.")?;
    // ZKB exposes the creation date of the PDF, but no separate booking date in
    // this metadata profile. Keep both concepts distinct and use the documented
    // fallback only for the transaction date.
    let booking_date = document_date.clone();
    let value_date = metadata
        .get("valuta")
        .map(|value| normalize_date(value).ok_or("ZKB-Valutadatum ist ungültig."))
        .transpose()?;
    let currency = required(metadata, "waehrung")?.trim().to_uppercase();
    if currency.len() != 3 {
        return Err("ZKB-Währung ist ungültig.".into());
    }
    let unsigned_amount = parse_money(required(metadata, "betrag")?)
        .ok_or("ZKB-Betrag ist ungültig.")?
        .abs();
    let amount_minor = if title_key.contains("gutschrift") {
        unsigned_amount
    } else if title_key.contains("belastung") {
        -unsigned_amount
    } else {
        return Err("ZKB-Belegtitel enthält weder Gutschrift noch Belastung.".into());
    };
    let closing_balance_minor = metadata
        .get("saldo")
        .map(|value| parse_money(value).ok_or_else(|| "ZKB-Saldo ist ungültig.".to_string()))
        .transpose()?;
    let opening_balance_minor = closing_balance_minor.map(|balance| balance - amount_minor);
    let account_reference = metadata
        .get("geschaeftsnriban")
        .or_else(|| metadata.get("geschaeftsnr"))
        .map(String::as_str)
        .unwrap_or("Konto");
    let mut warnings = vec![
        "Der ZKB-Beleg enthält kein separates Buchungsdatum; das Belegdatum wird dafür verwendet."
            .into(),
    ];
    if closing_balance_minor.is_none() {
        warnings.push("Der ZKB-Beleg enthält keinen Saldo. Der Kontostand muss beim Import kontrolliert werden.".into());
    }
    let external_reference = metadata.get("zkbreferenz").cloned();
    let isin = metadata.get("isin").cloned();
    let valor_number = metadata.get("valornr").cloned();
    let security_details =
        (isin.is_some() || valor_number.is_some()).then_some(ParsedSecurityDetails {
            isin,
            valor_number,
            quantity: None,
            price: None,
            price_currency: None,
            exchange_rate: None,
            gross_amount_minor: None,
            fees_minor: None,
            taxes_minor: None,
            withholding_tax_minor: None,
            accrued_interest_minor: None,
        });
    let transaction_kind = if title_key.contains("dividende") {
        "dividend"
    } else if title_key.contains("kauf") || title_key.contains("verkauf") {
        "security_trade"
    } else {
        "cash_transaction"
    };

    Ok(ParsedStatement {
        currency_balances: Vec::new(),
        account_type: Some("cash".into()),
        provider: "zkb".into(),
        format: "PDF".into(),
        account_name: format!("ZKB · {account_reference}"),
        transactions: vec![ParsedTransaction {
            booking_date,
            value_date,
            description: title.to_string(),
            industry: None,
            amount_minor,
            balance_minor: closing_balance_minor,
            currency,
            confidence: 1.0,
            source_row: 1,
            transaction_kind: transaction_kind.into(),
            reference_namespace: external_reference.as_ref().map(|_| "zkb_pdf".into()),
            external_reference,
            security_details,
            ..ParsedTransaction::default()
        }],
        opening_balance_minor,
        closing_balance_minor,
        warnings,
        document_type: Some("advice".into()),
        document_date: Some(document_date),
        record_definition_id: metadata.get("recorddefinitionid").cloned(),
        account_reference: Some(account_reference.into()),
        ..ParsedStatement::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zkb_credit_notice_metadata() {
        let metadata = parse_metadata("Belegkategorie:30;RecordDefinitionID:12345;BelegTitel:Gutschrift;PartnerNr:1.234.567-0;GeschaeftsNrIBAN:CH8300700113100012345;ZKBReferenz:Z123456789012;BelegDatum:2021-01-25;Valuta:2021-01-26;Waehrung:CHF;Betrag:125.40;Saldo:99999.00;ISIN:CH0000000001;ValorNr:100001;");
        let statement = statement_from_metadata(&metadata).unwrap();

        assert_eq!(statement.provider, "zkb");
        assert_eq!(statement.account_name, "ZKB · CH8300700113100012345");
        assert_eq!(statement.opening_balance_minor, Some(9_987_360));
        assert_eq!(statement.closing_balance_minor, Some(9_999_900));
        assert_eq!(statement.transactions[0].booking_date, "2021-01-25");
        assert_eq!(
            statement.transactions[0].value_date.as_deref(),
            Some("2021-01-26")
        );
        assert_eq!(statement.transactions[0].amount_minor, 12_540);
        assert_eq!(statement.document_date.as_deref(), Some("2021-01-25"));
        assert_eq!(statement.document_type.as_deref(), Some("advice"));
        assert_eq!(statement.record_definition_id.as_deref(), Some("12345"));
        assert_eq!(
            statement.account_reference.as_deref(),
            Some("CH8300700113100012345")
        );
        assert_eq!(
            statement.transactions[0].reference_namespace.as_deref(),
            Some("zkb_pdf")
        );
        assert_eq!(
            statement.transactions[0].external_reference.as_deref(),
            Some("Z123456789012")
        );
        assert_eq!(
            statement.transactions[0]
                .security_details
                .as_ref()
                .and_then(|details| details.isin.as_deref()),
            Some("CH0000000001")
        );
    }

    #[test]
    fn parses_zkb_debit_and_rejects_incomplete_metadata() {
        let metadata = parse_metadata("Belegkategorie:30;BelegTitel:Belastung;GeschaeftsNr:0-1234-00123456;BelegDatum:2021-01-25;Waehrung:CHF;Betrag:25.00;Saldo:100.00;");
        let statement = statement_from_metadata(&metadata).unwrap();
        assert_eq!(statement.transactions[0].amount_minor, -2_500);
        assert_eq!(statement.opening_balance_minor, Some(12_500));

        let incomplete = parse_metadata("Belegkategorie:30;BelegTitel:Belastung;");
        assert!(statement_from_metadata(&incomplete)
            .unwrap_err()
            .contains("belegdatum"));
    }

    #[test]
    fn leaves_non_transaction_documents_to_the_generic_pdf_importer() {
        let metadata =
            parse_metadata("Belegkategorie:10;BelegTitel:Kontoauszug;BelegDatum:2021-01-25;");
        assert!(!is_transaction_notice(&metadata));
    }
}
