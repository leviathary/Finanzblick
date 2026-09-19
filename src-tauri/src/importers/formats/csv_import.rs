//! Liest CSV-Kontoauszüge und vereinheitlicht Zeichencodierung, Trennzeichen und Spalten.

use crate::importers::*;
use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};
use std::collections::BTreeMap;

pub(in crate::importers) fn decode(bytes: &[u8]) -> Result<(String, Vec<String>), String> {
    let (encoding, data) = if bytes.starts_with(&[0xff, 0xfe]) {
        (Some(UTF_16LE), &bytes[2..])
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (Some(UTF_16BE), &bytes[2..])
    } else {
        (
            None,
            bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
        )
    };
    if let Some(encoding) = encoding {
        let (text, errors) = encoding.decode_without_bom_handling(data);
        if errors {
            return Err("Die UTF-16-Datei enthält beschädigte Zeichen.".into());
        }
        return Ok((text.into_owned(), vec![]));
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if text.contains('\0') {
            return Err(
                "CSV enthält Nullzeichen. Bitte als UTF-8 oder UTF-16 mit BOM exportieren.".into(),
            );
        }
        return Ok((text.to_owned(), vec![]));
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err("Die als UTF-8 markierte Datei enthält beschädigte Zeichen.".into());
    }
    let (text, errors) = WINDOWS_1252.decode_without_bom_handling(data);
    if errors || text.contains('\0') {
        return Err("Die CSV-Zeichenkodierung konnte nicht gelesen werden.".into());
    }
    Ok((text.into_owned(), vec!["Ältere Zeichenkodierung als Windows-1252/Latin-1 eingelesen. Bitte Umlaute und Sonderzeichen in der Vorschau prüfen.".into()]))
}

pub(in crate::importers) fn parse_csv(
    path: &Path,
    selected_provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let bytes =
        fs::read(path).map_err(|_| "Die CSV-Datei konnte nicht gelesen werden.".to_string())?;
    parse_bytes(&bytes, selected_provider)
}

fn parse_bytes(bytes: &[u8], selected_provider: Option<&str>) -> Result<ParsedStatement, String> {
    let (content, mut warnings) = decode(bytes)?;
    let content = content.trim_start_matches(['\r', '\n']);
    let first = content.lines().next().ok_or("Die CSV-Datei ist leer.")?;
    let (delimiter, content, offset) = if first.to_ascii_lowercase().starts_with("sep=") {
        let separator = first
            .trim_end()
            .as_bytes()
            .get(4)
            .copied()
            .ok_or("Ungültige sep=-Zeile.")?;
        if ![b';', b',', b'\t'].contains(&separator) {
            return Err("Nicht unterstütztes CSV-Trennzeichen.".into());
        }
        (
            separator,
            content.split_once('\n').map(|(_, body)| body).unwrap_or(""),
            1,
        )
    } else {
        let separator = [b';', b',', b'\t']
            .into_iter()
            .max_by_key(|delimiter| first.bytes().filter(|byte| byte == delimiter).count())
            .unwrap();
        (separator, content, 0)
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|error| format!("CSV-Kopfzeile: {error}"))?
        .iter()
        .map(normalized)
        .collect();
    let column = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| find_header_optional(&headers, &[*name]))
    };
    let date_index = column(&[
        "buchungsdatum",
        "buchungstag",
        "buchung",
        "datum",
        "einkaufsdatum",
    ])
    .ok_or("Benötigte Datumsspalte fehlt.")?;
    let description_index = find_header(
        &headers,
        &[
            "beschreibung",
            "informationen",
            "text",
            "buchungstext",
            "vorgang",
        ],
    )?;
    let amount_index = column(&["betrag", "betrag chf"]);
    let debit_index = column(&["belastung", "soll chf", "soll"]);
    let credit_index = column(&["gutschrift", "haben chf", "haben"]);
    let balance_index = column(&["saldo", "saldo chf", "kontostand", "vertragswert"]);
    let currency_index = column(&["währung", "waehrung", "currency"]);
    let original_currency_index = column(&["originalwährung", "originalwaehrung"]);
    let purchase_index = column(&["einkaufsdatum"]);
    let value_index = column(&["valutadatum", "valuta"]);
    let industry_index = column(&["branche", "industry"]);
    let card = column(&["kartennummer"]).is_some()
        && purchase_index.is_some()
        && column(&["buchung"]).is_some();
    if debit_index.is_none() && credit_index.is_none() && amount_index.is_none() {
        return Err("Benötigte Betragsspalte fehlt.".into());
    }
    // Merchant names must never be used to guess the issuing bank.
    let provider = normalize_provider(selected_provider.unwrap_or("unknown"));
    if provider == "unknown" {
        warnings.push(
            "Anbieter nicht im CSV ausgewiesen. Er wird aus deiner Kontozuordnung übernommen."
                .into(),
        );
    }
    let mut transactions = Vec::new();
    let mut openings = BTreeMap::<String, i64>::new();
    let mut totals = BTreeMap::<String, i64>::new();
    for record in reader.records() {
        let record = record
            .map_err(|error| format!("CSV konnte nicht vollständig gelesen werden: {error}"))?;
        let row = record
            .position()
            .map(|position| position.line() as usize + offset)
            .unwrap_or(0);
        let get = |index: Option<usize>| index.and_then(|index| record.get(index)).unwrap_or("");
        let mut description = get(Some(description_index)).to_owned();
        // Some bank exports end with a visually empty row containing non-ASCII
        // whitespace (for example a non-breaking space). The csv crate's trim
        // option only removes ASCII whitespace, so normalize it here as well.
        if record.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        if description.is_empty() {
            return Err(format!("Zeile {row}: Buchungstext fehlt."));
        }
        let normalized_description = normalized(&description);
        if is_closing_label(&normalized_description)
            || (normalized_description.starts_with("total ")
                && (get(Some(date_index)).is_empty() || (card && get(purchase_index).is_empty())))
        {
            continue;
        }
        let number = |index: Option<usize>| -> Result<Option<i64>, String> {
            let text = get(index);
            if text.is_empty() {
                return Ok(None);
            }
            parse_money(text)
                .map(Some)
                .ok_or_else(|| format!("Zeile {row}: Ungültiger Geldbetrag."))
        };
        let amount = if debit_index.is_some() || credit_index.is_some() {
            let debit = number(debit_index)?;
            let credit = number(credit_index)?;
            if debit.is_none() && credit.is_none() {
                None
            } else {
                Some(credit.unwrap_or(0) - debit.unwrap_or(0))
            }
        } else {
            number(amount_index)?
        };
        let currency = if currency_index.is_some() {
            get(currency_index).to_uppercase()
        } else {
            "CHF".into()
        };
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(format!("Zeile {row}: Währung fehlt oder ist ungültig."));
        }
        if is_opening_label(&normalized_description) {
            if let Some(balance) = if card {
                amount
            } else {
                number(balance_index)?.or(amount)
            } {
                if openings.insert(currency, balance).is_some() {
                    return Err(format!(
                        "Zeile {row}: Mehrere Anfangssalden für dieselbe Währung."
                    ));
                }
            }
            continue;
        }
        let amount = amount.ok_or_else(|| format!("Zeile {row}: Abgerechneter Betrag fehlt."))?;
        let booking_date = normalize_date(get(Some(date_index)))
            .ok_or_else(|| format!("Zeile {row}: Ungültiges oder fehlendes Buchungsdatum."))?;
        let value_date = if get(value_index).is_empty() {
            None
        } else {
            Some(
                normalize_date(get(value_index))
                    .ok_or_else(|| format!("Zeile {row}: Ungültiges Valutadatum."))?,
            )
        };
        if card
            && !get(original_currency_index).is_empty()
            && get(original_currency_index) != currency
        {
            description.push_str(&format!(
                " · Originalbetrag: {} {}",
                get(amount_index),
                get(original_currency_index)
            ));
        }
        if card && !get(purchase_index).is_empty() {
            description.push_str(&format!(" · Einkauf: {}", get(purchase_index)));
        }
        *totals.entry(currency.clone()).or_default() += amount;
        let industry = get(industry_index).trim();
        transactions.push(ParsedTransaction {
            booking_date,
            value_date,
            description,
            industry: (!industry.is_empty()).then(|| industry.to_string()),
            amount_minor: amount,
            balance_minor: number(balance_index)?,
            currency,
            confidence: 0.95,
            source_row: row,
            ..ParsedTransaction::default()
        });
    }
    if transactions.is_empty() {
        return Err("Es wurden keine Buchungen gefunden.".into());
    }
    transactions.sort_by(|a, b| {
        a.booking_date
            .cmp(&b.booking_date)
            .then(a.source_row.cmp(&b.source_row))
    });
    let mut currency_balances = Vec::new();
    if card {
        for (currency, opening) in &openings {
            if let Some(date) = transactions
                .iter()
                .filter(|row| &row.currency == currency)
                .map(|row| &row.booking_date)
                .max()
            {
                currency_balances.push(CurrencyBalance {
                    currency: currency.clone(),
                    opening_date: None,
                    opening_balance_minor: *opening,
                    closing_balance_minor: opening + totals.get(currency).copied().unwrap_or(0),
                    closing_date: date.clone(),
                });
            }
        }
        warnings.push("Kreditkarten-CSV: Belastungen werden negativ und Gutschriften positiv übernommen. Der Saldo wird, sofern ein Saldovortrag vorliegt, daraus und aus den Buchungen berechnet; bitte mit der Rechnung vergleichen.".into());
    }
    let opening_balance_minor = if openings.len() == 1 {
        openings.values().next().copied()
    } else {
        None
    };
    let closing_balance_minor = if currency_balances.len() == 1 {
        Some(currency_balances[0].closing_balance_minor)
    } else {
        transactions.last().and_then(|row| row.balance_minor)
    };
    Ok(ParsedStatement {
        currency_balances,
        account_type: card.then(|| "credit_card".into()),
        provider,
        format: "CSV".into(),
        account_name: "Bestehendes Konto auswählen".into(),
        opening_balance_minor,
        closing_balance_minor,
        transactions,
        warnings,
        ..ParsedStatement::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_encodings_separators_quotes_and_credit_card_amounts() {
        let csv = "sep=;\r\nKontonummer;Kartennummer;Einkaufsdatum;Buchungstext;Betrag;Originalwährung;Währung;Belastung;Gutschrift;Buchung\r\n1;;;Saldovortrag;;;CHF;100;;;\r\n";
        // Create rows with exactly the header's field count.
        let csv = csv.replace("CHF;100;;;", "CHF;100;;") + "1;123;01.08.2026;\"Café; Zürich\";20;EUR;CHF;19.50;;02.08.2026\r\n1;123;03.08.2026;Rückerstattung;5;CHF;CHF;;5;03.08.2026\r\n";
        let (cp, _, errors) = WINDOWS_1252.encode(&csv);
        assert!(!errors);
        let mut variants = vec![
            csv.as_bytes().to_vec(),
            [b"\xef\xbb\xbf".as_slice(), csv.as_bytes()].concat(),
            cp.into_owned(),
        ];
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in csv.encode_utf16() {
                bytes.extend(if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            variants.push(bytes);
        }
        for bytes in variants {
            let parsed = parse_bytes(&bytes, None).unwrap();
            assert_eq!(parsed.transactions.len(), 2);
            assert!(parsed.transactions[0].description.contains("Café; Zürich"));
            assert_eq!(parsed.transactions[0].amount_minor, -1950);
            assert_eq!(parsed.transactions[1].amount_minor, 500);
            assert_eq!(parsed.closing_balance_minor, Some(-11450));
            assert_eq!(parsed.account_type.as_deref(), Some("credit_card"));
            assert_eq!(parsed.provider, "unknown");
        }
        for delimiter in [",", "\t"] {
            let csv = format!("Datum{delimiter}Text{delimiter}Betrag\n01.08.2026{delimiter}\"Zeile eins\nZeile zwei\"{delimiter}-1.25\n");
            let parsed = parse_bytes(csv.as_bytes(), Some("ubs")).unwrap();
            assert_eq!(parsed.transactions[0].amount_minor, -125);
            assert!(parsed.transactions[0].description.contains('\n'));
        }
    }

    #[test]
    fn rejects_corrupt_dates_amounts_and_unicode() {
        for row in [
            "kein Datum;Text;12",
            "01.08.2026;Text;falsch",
            "01.08.2026;Text;",
        ] {
            assert!(parse_bytes(
                format!("Datum;Text;Betrag\n{row}\n").as_bytes(),
                Some("ubs")
            )
            .is_err());
        }
        assert!(decode(&[0xff, 0xfe, 0x41]).is_err());
        assert!(decode(&[0xef, 0xbb, 0xbf, 0xff]).is_err());
    }

    #[test]
    fn ignores_visually_empty_rows_with_unicode_whitespace() {
        let csv = "Buchung;Buchungstext;Betrag;Währung\n01.08.2026;Einkauf;-12.50;CHF\n\u{a0};\u{a0};\u{a0};\u{a0}\n;Total pro Währung;;\n;Total Kartenbuchungen CHF;-12.50;\n";
        let parsed = parse_bytes(csv.as_bytes(), Some("ubs")).unwrap();
        assert_eq!(parsed.transactions.len(), 1);
        assert_eq!(parsed.transactions[0].currency, "CHF");
    }

    #[test]
    fn preserves_credit_card_industry_as_structured_data() {
        let csv = "Buchung;Buchungstext;Betrag;Währung;Branche\n01.08.2026;Händler;-12.50;CHF;Lebensmittelgeschäfte\n";
        let parsed = parse_bytes(csv.as_bytes(), Some("ubs")).unwrap();
        assert_eq!(
            parsed.transactions[0].industry.as_deref(),
            Some("Lebensmittelgeschäfte")
        );
    }
}
