use super::*;
use calamine::{open_workbook_auto, Reader};

pub(super) fn parse(
    path: &Path,
    selected_provider: Option<&str>,
) -> Result<ParsedStatement, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Excel-Datei konnte nicht geöffnet werden: {error}"))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("Die Arbeitsmappe enthält kein Tabellenblatt.")?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|error| format!("Tabellenblatt konnte nicht gelesen werden: {error}"))?;
    let rows: Vec<&[Data]> = range.rows().collect();
    let provider =
        normalize_provider(selected_provider.unwrap_or_else(|| detect_provider_in_cells(&rows)));
    if provider == "unknown" {
        return Err(
            "Der Anbieter konnte nicht erkannt werden. Bitte im vorherigen Schritt auswählen."
                .to_string(),
        );
    }
    let header_row = rows
        .iter()
        .position(|row| row.iter().any(|cell| is_date_header(&cell.to_string())))
        .ok_or("Keine unterstützte Buchungstabelle gefunden.")?;
    let headers: Vec<String> = rows[header_row]
        .iter()
        .map(|cell| normalized(&cell.to_string()))
        .collect();
    let mapping = providers::by_id(&provider)
        .and_then(|importer| importer.excel_mapping())
        .unwrap_or(&DEFAULT_EXCEL_MAPPING);
    let date_index = find_header(&headers, mapping.date)?;
    let value_date_index = find_header_optional(&headers, mapping.value_date);
    let description_index = find_header(&headers, mapping.description)?;
    let debit_index = find_header_optional(&headers, mapping.debit);
    let credit_index = find_header_optional(&headers, mapping.credit);
    let amount_index = find_header_optional(&headers, mapping.amount);
    let balance_index = find_header_optional(&headers, mapping.balance);
    let currency_index = find_header_optional(&headers, mapping.currency);
    let industry_index = find_header_optional(&headers, mapping.industry);
    let account_name = rows
        .get(1)
        .and_then(|row| row.first())
        .map(|cell| {
            cell.to_string()
                .split('|')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Neues Konto".to_string());
    let mut transactions = Vec::new();
    let mut opening_balance = None;
    let mut closing_balance = None;
    let mut warnings = Vec::new();
    for (row_number, row) in rows.iter().enumerate().skip(header_row + 1) {
        let description = cell_string(row, description_index);
        if description.is_empty() {
            continue;
        }
        let balance = balance_index.and_then(|index| cell_money(row, index));
        let description_key = normalized(&description);
        if is_opening_label(&description_key) {
            opening_balance = balance;
            continue;
        }
        if is_closing_label(&description_key) {
            closing_balance = balance.or(closing_balance);
            continue;
        }
        let Some(booking_date) = cell_date(row, date_index) else {
            warnings.push(format!(
                "Zeile {}: Datum konnte nicht gelesen werden.",
                row_number + 1
            ));
            continue;
        };
        let amount = if let Some(index) = amount_index {
            cell_money(row, index).unwrap_or(0)
        } else {
            let debit = debit_index
                .and_then(|index| cell_money(row, index))
                .unwrap_or(0);
            let credit = credit_index
                .and_then(|index| cell_money(row, index))
                .unwrap_or(0);
            credit - debit
        };
        let currency = currency_index
            .map(|index| cell_string(row, index))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "CHF".to_string());
        transactions.push(ParsedTransaction {
            booking_date,
            value_date: value_date_index.and_then(|index| cell_date(row, index)),
            description,
            industry: industry_index
                .map(|index| cell_string(row, index))
                .filter(|value| !value.is_empty()),
            amount_minor: amount,
            balance_minor: balance,
            currency,
            confidence: 1.0,
            source_row: row_number + 1,
        });
    }
    closing_balance =
        closing_balance.or_else(|| transactions.last().and_then(|row| row.balance_minor));
    if transactions.is_empty() {
        return Err("Es wurden keine Buchungen gefunden.".to_string());
    }
    Ok(ParsedStatement {
        currency_balances: Vec::new(),
        account_type: None,
        provider,
        format: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("xlsx")
            .to_ascii_uppercase(),
        account_name,
        transactions,
        opening_balance_minor: opening_balance,
        closing_balance_minor: closing_balance,
        warnings,
    })
}
