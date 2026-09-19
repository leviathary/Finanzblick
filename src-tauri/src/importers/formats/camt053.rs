//! Bank-independent camt.053 reader. One Ntry remains one ledger posting:
//! TxDtls enrich that posting, never add a second cash movement.
use crate::importers::{CurrencyBalance, ParsedStatement, ParsedTransaction};
use chrono::NaiveDate;
use roxmltree::{Document, Node, ParsingOptions};
use std::{fs, path::Path};

type XmlNode<'a> = Node<'a, 'a>;

fn child<'a>(node: XmlNode<'a>, name: &str) -> Option<XmlNode<'a>> {
    node.children().find(|item| {
        item.is_element()
            && item.tag_name().name() == name
            && item.tag_name().namespace() == node.tag_name().namespace()
    })
}

fn at<'a>(mut node: XmlNode<'a>, path: &str) -> Option<XmlNode<'a>> {
    for name in path.split('/') {
        node = child(node, name)?;
    }
    Some(node)
}

fn text(node: XmlNode<'_>, path: &str) -> Option<String> {
    at(node, path)
        .and_then(|value| value.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn required(node: XmlNode<'_>, path: &str) -> Result<String, String> {
    text(node, path).ok_or_else(|| format!("camt.053: Pflichtfeld {path} fehlt."))
}

fn date(value: &str) -> Result<String, String> {
    if value.len() == 10 {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(|day| day.to_string())
            .map_err(|_| "camt.053: Ungültiges Datum.".into())
    } else {
        // Preserve the bank's local calendar date, not its UTC conversion.
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|day| day.date_naive().to_string())
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|day| day.date().to_string())
            })
            .map_err(|_| "camt.053: Ungültiges Datum oder Zeitformat.".into())
    }
}

fn choice_date(node: XmlNode<'_>, path: &str) -> Result<String, String> {
    let parent = at(node, path).ok_or_else(|| format!("camt.053: {path} fehlt."))?;
    date(
        &text(parent, "Dt")
            .or_else(|| text(parent, "DtTm"))
            .ok_or_else(|| format!("camt.053: Datum in {path} fehlt."))?,
    )
}

// The application's money model uses hundredths in all currencies. Never round
// a more precise source silently and never parse bank amounts as floating point.
fn minor(value: &str) -> Result<i64, String> {
    let error = || {
        "camt.053: Betrag ist ungültig oder nicht exakt in Hundertsteln darstellbar.".to_string()
    };
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
        || (fraction.len() > 2 && fraction.as_bytes()[2..].iter().any(|c| *c != b'0'))
    {
        return Err(error());
    }
    let whole = whole.parse::<i64>().map_err(|_| error())?;
    let cents = fraction
        .bytes()
        .take(2)
        .enumerate()
        .map(|(index, digit)| i64::from(digit - b'0') * if index == 0 { 10 } else { 1 })
        .sum::<i64>();
    whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(cents))
        .ok_or_else(error)
}

fn amount(node: XmlNode<'_>, currency: &str) -> Result<i64, String> {
    let amt = child(node, "Amt").ok_or("camt.053: Amt fehlt.")?;
    if amt.attribute("Ccy") != Some(currency) {
        return Err("camt.053: Buchungs- oder Saldowährung weicht von der Kontowährung ab.".into());
    }
    let value = minor(amt.text().unwrap_or("").trim())?;
    match required(node, "CdtDbtInd")?.as_str() {
        "CRDT" => Ok(value),
        "DBIT" => Ok(-value),
        _ => Err("camt.053: Ungültiger Soll-/Haben-Indikator.".into()),
    }
}

fn reference(node: XmlNode<'_>, path: &str) -> Option<String> {
    text(node, path).filter(|v| {
        !matches!(
            v.to_ascii_uppercase().as_str(),
            "NOTPROVIDED" | "NONREF" | "NOTAVAILABLE"
        )
    })
}

fn join(values: impl Iterator<Item = String>) -> Option<String> {
    let mut parts = Vec::new();
    for value in values {
        if !parts.contains(&value) {
            parts.push(value);
        }
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

pub(in crate::importers) fn parse(
    path: &Path,
    provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let bytes = fs::read(path).map_err(|_| "camt.053: Datei konnte nicht gelesen werden.")?;
    let source = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let encoding = if bytes[0] == 0xff {
            encoding_rs::UTF_16LE
        } else {
            encoding_rs::UTF_16BE
        };
        let (decoded, _, errors) = encoding.decode(&bytes);
        if errors {
            return Err("camt.053: Ungültige UTF-16-Datei.".into());
        }
        decoded.into_owned()
    } else {
        String::from_utf8(bytes)
            .map_err(|_| "camt.053: Erwartet wird UTF-8 oder UTF-16 mit BOM.")?
    };
    parse_xml(source.trim_start_matches('\u{feff}'), provider)
}

fn parse_xml(source: &str, provider: Option<&str>) -> Result<ParsedStatement, String> {
    let doc = Document::parse_with_options(
        source,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 500_000,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| "camt.053: Ungültiges XML oder nicht unterstützte DTD.")?;
    let root = doc.root_element();
    let namespace = root.tag_name().namespace().unwrap_or("");
    if root.tag_name().name() != "Document"
        || !matches!(
            namespace,
            "urn:iso:std:iso:20022:tech:xsd:camt.053.001.02"
                | "urn:iso:std:iso:20022:tech:xsd:camt.053.001.04"
                | "urn:iso:std:iso:20022:tech:xsd:camt.053.001.08"
                | "urn:iso:std:iso:20022:tech:xsd:camt.053.001.10"
        )
    {
        return Err("Erwartet wird camt.053 XML in Version 02, 04, 08 oder 10.".into());
    }
    let message = child(root, "BkToCstmrStmt").ok_or("camt.053: BkToCstmrStmt fehlt.")?;
    let statements: Vec<_> = message
        .children()
        .filter(|n| n.has_tag_name((namespace, "Stmt")))
        .collect();
    // The current review maps one account per currency. Reject multiple statements
    // rather than mixing same-currency accounts or dropping any of their data.
    if statements.len() != 1 {
        return Err("camt.053: Bitte genau einen Kontoauszug pro XML-Datei exportieren; mehrere Stmt-Abschnitte werden noch nicht unterstützt.".into());
    }
    let stmt = statements[0];
    let account_reference = text(stmt, "Acct/Id/IBAN")
        .or_else(|| text(stmt, "Acct/Id/Othr/Id"))
        .ok_or("camt.053: IBAN oder Kontoreferenz fehlt.")?;
    let currency = required(stmt, "Acct/Ccy")?;
    if currency.len() != 3 || !currency.bytes().all(|c| c.is_ascii_uppercase()) {
        return Err("camt.053: Ungültige Kontowährung.".into());
    }
    let document_date = text(stmt, "CreDtTm")
        .or_else(|| text(message, "GrpHdr/CreDtTm"))
        .map(|v| date(&v))
        .transpose()?;
    let mut opening = None;
    let mut closing = None;
    for bal in stmt
        .children()
        .filter(|n| n.has_tag_name((namespace, "Bal")))
    {
        let code = text(bal, "Tp/CdOrPrtry/Cd").unwrap_or_default();
        if code != "OPBD" && code != "CLBD" {
            continue;
        }
        let value = (choice_date(bal, "Dt")?, amount(bal, &currency)?);
        let slot = if code == "OPBD" {
            &mut opening
        } else {
            &mut closing
        };
        if slot.replace(value).is_some() {
            return Err("camt.053: Mehrere OPBD-/CLBD-Salden sind nicht eindeutig.".into());
        }
    }
    let (opening_date, opening_amount) =
        opening.ok_or("camt.053: Gebuchter Anfangssaldo OPBD fehlt.")?;
    let (closing_date, closing_amount) =
        closing.ok_or("camt.053: Gebuchter Schlusssaldo CLBD fehlt.")?;
    if closing_date < opening_date {
        return Err("camt.053: Saldenzeitraum ist ungültig.".into());
    }
    let mut warnings = Vec::new();
    let mut transactions = Vec::new();
    let mut entry_references = std::collections::HashSet::new();
    for (index, entry) in stmt
        .children()
        .filter(|n| n.has_tag_name((namespace, "Ntry")))
        .enumerate()
    {
        let status = text(entry, "Sts/Cd")
            .or_else(|| text(entry, "Sts"))
            .unwrap_or_default();
        match status.as_str() {
            "BOOK" => {}
            "PDNG" | "INFO" => {
                warnings.push(format!(
                    "camt.053: Eintrag {} ist nicht gebucht und wird ausgelassen.",
                    index + 1
                ));
                continue;
            }
            _ => {
                return Err(format!(
                    "camt.053: Unbekannter oder fehlender Buchungsstatus in Eintrag {}.",
                    index + 1
                ))
            }
        }
        let amount_minor = amount(entry, &currency)?;
        let booking_date = choice_date(entry, "BookgDt")?;
        if booking_date < opening_date || booking_date > closing_date {
            return Err("camt.053: Buchungsdatum liegt ausserhalb des Saldenzeitraums.".into());
        }
        let value_date = child(entry, "ValDt")
            .map(|_| choice_date(entry, "ValDt"))
            .transpose()?;
        let details: Vec<_> = entry
            .children()
            .filter(|n| n.has_tag_name((namespace, "NtryDtls")))
            .flat_map(|n| n.children())
            .filter(|n| n.has_tag_name((namespace, "TxDtls")))
            .collect();
        let party = if required(entry, "CdtDbtInd")? == "CRDT" {
            "Dbtr"
        } else {
            "Cdtr"
        };
        let counterparty_name = join(details.iter().filter_map(|n| {
            text(*n, &format!("RltdPties/{party}/Pty/Nm"))
                .or_else(|| text(*n, &format!("RltdPties/{party}/Nm")))
        }));
        let remittance_information = join(details.iter().flat_map(|n| {
            let mut values = Vec::new();
            if let Some(rmt) = child(*n, "RmtInf") {
                for item in rmt
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().namespace() == Some(namespace))
                {
                    if item.tag_name().name() == "Ustrd" {
                        if let Some(value) = item.text().map(str::trim).filter(|v| !v.is_empty()) {
                            values.push(value.to_owned());
                        }
                    } else if item.tag_name().name() == "Strd" {
                        if let Some(value) = text(item, "CdtrRefInf/Ref") {
                            values.push(value);
                        }
                        for info in item
                            .children()
                            .filter(|n| n.has_tag_name((namespace, "AddtlRmtInf")))
                        {
                            if let Some(value) = info.text() {
                                values.push(value.trim().to_owned());
                            }
                        }
                    }
                }
            }
            values
        }));
        // Entry-level IDs identify this cash movement. A detail-level reference
        // can substitute only when the entry contains exactly one detail.
        let bank_ref = reference(entry, "AcctSvcrRef").or_else(|| {
            (details.len() == 1)
                .then(|| reference(details[0], "Refs/AcctSvcrRef"))
                .flatten()
        });
        let (external_reference, reference_namespace) = if let Some(value) = bank_ref {
            (Some(value), Some(String::from("camt_acct_svcr_ref")))
        } else if let Some(value) = reference(entry, "NtryRef") {
            (Some(value), Some(String::from("camt_entry_ref")))
        } else {
            (None, None)
        };
        if let (Some(reference), Some(namespace)) = (&external_reference, &reference_namespace) {
            if !entry_references.insert((namespace.clone(), reference.clone())) {
                return Err("camt.053: Eine Bankreferenz wird für mehrere Kontobuchungen verwendet; der Auszug ist nicht eindeutig importierbar.".into());
            }
        }
        let mut description = join(
            counterparty_name
                .clone()
                .into_iter()
                .chain(remittance_information.clone())
                .chain(text(entry, "AddtlNtryInf"))
                .chain(details.iter().filter_map(|n| text(*n, "AddtlTxInf")))
                .chain(
                    details
                        .iter()
                        .filter_map(|n| reference(*n, "Refs/EndToEndId")),
                ),
        )
        .or_else(|| external_reference.clone())
        .unwrap_or_else(|| "camt.053 Buchung".into());
        if details.len() > 1 {
            description = format!("Sammelbuchung ({}): {description}", details.len());
            if !warnings
                .iter()
                .any(|v| v.contains("Sammelbuchungen bleiben"))
            {
                warnings.push("Sammelbuchungen bleiben jeweils eine Kontobuchung mit dem Gesamtbetrag; Gegenparteien und Zahlungsinformationen werden zusammengeführt.".into());
            }
        }
        if text(entry, "RvslInd").is_some_and(|v| v == "true" || v == "1") {
            description = format!("Storno · {description}");
        }
        transactions.push(ParsedTransaction {
            booking_date,
            value_date,
            description,
            amount_minor,
            currency: currency.clone(),
            confidence: 1.0,
            source_row: index + 1,
            external_reference,
            reference_namespace,
            counterparty_name,
            remittance_information,
            ..ParsedTransaction::default()
        });
    }
    let computed = transactions
        .iter()
        .try_fold(opening_amount, |sum, row| sum.checked_add(row.amount_minor))
        .ok_or("camt.053: Betragssumme ist zu gross.")?;
    if computed != closing_amount {
        return Err("camt.053: OPBD plus gebuchte Bewegungen stimmt nicht mit CLBD überein. Der Auszug wird nicht teilweise importiert.".into());
    }
    transactions.sort_by(|a, b| {
        a.booking_date
            .cmp(&b.booking_date)
            .then(a.source_row.cmp(&b.source_row))
    });
    // OPBD is the balance before the reported period's postings, even when its
    // date equals the first booking date. Store it at the previous day in that case.
    let snapshot_opening_date = if transactions
        .first()
        .is_some_and(|row| row.booking_date == opening_date)
    {
        NaiveDate::parse_from_str(&opening_date, "%Y-%m-%d")
            .unwrap()
            .pred_opt()
            .ok_or("camt.053: Anfangsdatum ausserhalb des unterstützten Bereichs.")?
            .to_string()
    } else {
        opening_date
    };
    Ok(ParsedStatement {
        provider: provider
            .filter(|v| *v != "unknown")
            .unwrap_or("unknown")
            .into(),
        format: "CAMT053".into(),
        account_name: account_reference.clone(),
        account_reference: Some(account_reference),
        document_type: Some("statement".into()),
        document_date,
        record_definition_id: Some(namespace.into()),
        currency_balances: vec![CurrencyBalance {
            currency,
            opening_date: Some(snapshot_opening_date),
            opening_balance_minor: opening_amount,
            closing_balance_minor: closing_amount,
            closing_date,
        }],
        opening_balance_minor: Some(opening_amount),
        closing_balance_minor: Some(closing_amount),
        transactions,
        warnings,
        ..ParsedStatement::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../fixtures/camt053.xml");

    #[test]
    fn versions_dates_amounts_parties_references_and_batches() {
        for version in ["02", "04", "08", "10"] {
            let mut xml = FIXTURE.replace("camt.053.001.08", &format!("camt.053.001.{version}"));
            if matches!(version, "02" | "04") {
                xml = xml
                    .replace("<Sts><Cd>BOOK</Cd></Sts>", "<Sts>BOOK</Sts>")
                    .replace("<Sts><Cd>PDNG</Cd></Sts>", "<Sts>PDNG</Sts>")
                    .replace("<Pty>", "")
                    .replace("</Pty>", "");
            }
            let result = parse_xml(&xml, None).unwrap();
            assert_eq!(result.provider, "unknown");
            assert_eq!(result.format, "CAMT053");
            assert_eq!(result.document_date.as_deref(), Some("2026-09-02"));
            assert_eq!(
                result.currency_balances[0].opening_date.as_deref(),
                Some("2026-08-31")
            );
            assert_eq!(result.transactions.len(), 2);
            let credit = &result.transactions[0];
            assert_eq!(credit.amount_minor, 15000);
            assert_eq!(credit.value_date.as_deref(), Some("2026-09-02"));
            assert_eq!(credit.counterparty_name.as_deref(), Some("Example & Co"));
            assert!(credit
                .remittance_information
                .as_ref()
                .unwrap()
                .contains("210000000003139471430009017"));
            assert_eq!(credit.external_reference.as_deref(), Some("BANK-1"));
            assert_eq!(
                credit.reference_namespace.as_deref(),
                Some("camt_acct_svcr_ref")
            );
            let debit = &result.transactions[1];
            assert_eq!(debit.amount_minor, -2000);
            assert_eq!(debit.booking_date, "2026-09-01");
            assert_eq!(debit.counterparty_name.as_deref(), Some("Shop A · Shop B"));
            assert_eq!(debit.external_reference.as_deref(), Some("ENTRY-2"));
            assert_eq!(debit.reference_namespace.as_deref(), Some("camt_entry_ref"));
            assert!(debit.description.contains("Sammelbuchung"));
            assert!(result
                .transactions
                .iter()
                .all(|row| row.balance_minor.is_none()));
        }
    }

    #[test]
    fn rejects_incomplete_ambiguous_or_unsafe_sources() {
        for xml in [
            FIXTURE.replace("1130.00", "1130.01"),
            FIXTURE.replace("150.00", "150.001"),
            FIXTURE.replace("<Amt Ccy=\"CHF\">150.00", "<Amt Ccy=\"EUR\">150.00"),
            FIXTURE.replace("<Sts><Cd>BOOK</Cd></Sts>", "<Sts><Cd>OTHER</Cd></Sts>"),
            FIXTURE.replace("<BookgDt><Dt>2026-09-01</Dt></BookgDt>", ""),
            FIXTURE.replace("<Dt>2026-09-01</Dt>", "<Dt>2026-02-30</Dt>"),
            FIXTURE.replace("</Stmt>", "</Stmt><Stmt/>"),
            FIXTURE.replace(
                "<NtryRef>ENTRY-2</NtryRef>",
                "<AcctSvcrRef>BANK-1</AcctSvcrRef>",
            ),
            FIXTURE.replace("camt.053", "camt.054"),
            FIXTURE.replace(
                "<Document xmlns=",
                "<!DOCTYPE Document [<!ENTITY x SYSTEM 'file:///private'>]><Document xmlns=",
            ),
        ] {
            assert!(parse_xml(&xml, None).is_err(), "accepted invalid document");
        }
        assert_eq!(minor("92233720368547758.07").unwrap(), i64::MAX);
        assert!(minor("92233720368547758.08").is_err());
        assert!(minor("-12.00").is_err());
        assert_eq!(minor("12.3400").unwrap(), 1234);
    }

    #[test]
    fn supports_prefixes_utf16_and_zero_activity() {
        let xml = FIXTURE
            .replace("<Document xmlns=", "<c:Document xmlns:c=")
            .replace("</Document>", "</c:Document>");
        // Prefixes do not matter; all children still need the camt namespace.
        let xml = xml.replace(
            "<c:Document xmlns:c=",
            "<c:Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt.053.001.08\" xmlns:c=",
        );
        assert_eq!(
            parse_xml(&xml, Some("raiffeisen")).unwrap().provider,
            "raiffeisen"
        );
        let empty = format!(
            "{} </Stmt></BkToCstmrStmt></Document>",
            FIXTURE.split("<Ntry>").next().unwrap()
        )
        .replace("1130.00", "1000.00");
        assert!(parse_xml(&empty, None).unwrap().transactions.is_empty());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statement.xml");
        let mut bytes = vec![0xff, 0xfe];
        for unit in FIXTURE.replace("UTF-8", "UTF-16").encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        fs::write(&path, bytes).unwrap();
        assert_eq!(
            crate::importers::parse_statement(path.to_string_lossy().into(), None, None)
                .unwrap()
                .transactions
                .len(),
            2
        );
    }

    #[test]
    fn reversal_keeps_reported_sign_and_placeholder_reference_is_ignored() {
        let xml = FIXTURE.replace(
            "<AcctSvcrRef>BANK-1</AcctSvcrRef>",
            "<AcctSvcrRef>NOTPROVIDED</AcctSvcrRef><RvslInd>true</RvslInd>",
        );
        let result = parse_xml(&xml, None).unwrap();
        assert_eq!(result.transactions[0].amount_minor, 15000);
        assert_eq!(
            result.transactions[0].external_reference.as_deref(),
            Some("ENTRY-1")
        );
        assert!(result.transactions[0].description.starts_with("Storno"));
    }
}
