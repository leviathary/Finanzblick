//! Untersucht Tabellen und übersetzt frei zugeordnete Spalten in normalisierte Importdaten.

use crate::importers::*;
use calamine::{open_workbook_auto, Reader};

const PREVIEW_ROWS: usize = 30;
const PREVIEW_COLUMNS: usize = 100;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabularMapping {
    pub sheet_name: String,
    /// One-based row number as shown in Excel.
    pub header_row: usize,
    /// One-based first data row.
    pub data_start_row: usize,
    pub date_column: usize,
    #[serde(default)]
    pub description_columns: Vec<usize>,
    pub amount_column: Option<usize>,
    pub debit_column: Option<usize>,
    pub credit_column: Option<usize>,
    pub value_date_column: Option<usize>,
    pub balance_column: Option<usize>,
    pub currency_column: Option<usize>,
    pub industry_column: Option<usize>,
    #[serde(default = "default_currency")]
    pub fixed_currency: String,
    #[serde(default)]
    pub invert_amount: bool,
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default)]
    pub number_format: NumberFormat,
}

fn default_currency() -> String {
    "CHF".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateFormat {
    #[default]
    Auto,
    Dmy,
    Mdy,
    Ymd,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumberFormat {
    #[default]
    Auto,
    DecimalComma,
    DecimalPoint,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabularInspection {
    pub sheets: Vec<SheetInspection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetInspection {
    pub name: String,
    pub row_count: usize,
    pub column_count: usize,
    pub suggested_header_row: usize,
    pub header_detected: bool,
    pub header_score: usize,
    pub preview: Vec<Vec<String>>,
}

pub fn inspect_tabular_file(path: String) -> Result<TabularInspection, String> {
    let path = Path::new(&path);
    ensure_workbook(path)?;
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Excel-Datei konnte nicht geöffnet werden: {error}"))?;
    let names = workbook.sheet_names().to_vec();
    let mut sheets = Vec::new();
    for name in names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|error| format!("Tabellenblatt konnte nicht gelesen werden: {error}"))?;
        let rows: Vec<&[Data]> = range.rows().collect();
        let column_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
        let detected_header = detect_header_row(&rows);
        let suggested_header_row = detected_header.map(|(index, _)| index + 1).unwrap_or(1);
        let preview = rows
            .iter()
            .take(PREVIEW_ROWS)
            .map(|row| {
                (0..column_count.min(PREVIEW_COLUMNS))
                    .map(|index| cell_string(row, index))
                    .collect()
            })
            .collect();
        sheets.push(SheetInspection {
            name,
            row_count: rows.len(),
            column_count,
            suggested_header_row,
            header_detected: detected_header.is_some(),
            header_score: detected_header.map(|(_, score)| score).unwrap_or(0),
            preview,
        });
    }
    if sheets.is_empty() {
        return Err("Die Arbeitsmappe enthält kein Tabellenblatt.".into());
    }
    Ok(TabularInspection { sheets })
}

fn detect_header_row(rows: &[&[Data]]) -> Option<(usize, usize)> {
    let mut best = None;
    for (index, row) in rows.iter().take(PREVIEW_ROWS).enumerate() {
        let score = header_score(row);
        if score >= 3
            && best
                .map(|(_, best_score)| score > best_score)
                .unwrap_or(true)
        {
            best = Some((index, score));
        }
    }
    best
}

fn header_score(row: &[Data]) -> usize {
    let headings: Vec<String> = row
        .iter()
        .map(|cell| normalized(&cell.to_string()))
        .collect();
    let text_cells = row
        .iter()
        .filter(|cell| matches!(cell, Data::String(value) if !value.trim().is_empty()))
        .count();
    if text_cells < 2 {
        return 0;
    }
    let has_date = find_header_optional(
        &headings,
        &[
            "datum",
            "buchungsdatum",
            "buchungstag",
            "kaufdatum",
            "transaktionsdatum",
        ],
    )
    .is_some();
    let has_description = find_header_optional(
        &headings,
        &[
            "beschreibung",
            "informationen",
            "text",
            "buchungstext",
            "vorgang",
            "notiz",
            "gegenpartei",
        ],
    )
    .is_some();
    let has_amount = find_header_optional(
        &headings,
        &[
            "betrag",
            "belastung",
            "gutschrift",
            "abfluss",
            "zufluss",
            "soll",
            "haben",
        ],
    )
    .is_some();
    text_cells
        + usize::from(has_date) * 6
        + usize::from(has_description) * 4
        + usize::from(has_amount) * 3
}

pub(in crate::importers) fn parse_mapped_workbook(
    path: &Path,
    selected_provider: Option<&str>,
    mapping: &TabularMapping,
) -> Result<ParsedStatement, String> {
    ensure_workbook(path)?;
    validate_mapping(mapping)?;
    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Excel-Datei konnte nicht geöffnet werden: {error}"))?;
    let range = workbook.worksheet_range(&mapping.sheet_name).map_err(|_| {
        format!(
            "Das Tabellenblatt „{}“ ist nicht mehr vorhanden.",
            mapping.sheet_name
        )
    })?;
    let rows: Vec<&[Data]> = range.rows().collect();
    if mapping.header_row > rows.len() || mapping.data_start_row > rows.len() + 1 {
        return Err("Kopf- oder Datenzeile liegt außerhalb des Tabellenblatts.".into());
    }
    let provider = normalize_provider(selected_provider.unwrap_or("unknown"));
    let fixed_currency = normalize_currency(&mapping.fixed_currency)?;
    let mut transactions = Vec::new();
    let mut errors = Vec::new();
    for (index, row) in rows
        .iter()
        .enumerate()
        .skip(mapping.data_start_row.saturating_sub(1))
    {
        if row.iter().all(|cell| cell.to_string().trim().is_empty()) {
            continue;
        }
        let source_row = index + 1;
        let result = parse_row(row, source_row, mapping, &fixed_currency);
        match result {
            Ok(Some(transaction)) => transactions.push(transaction),
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }
    if !errors.is_empty() {
        let shown = errors.iter().take(8).cloned().collect::<Vec<_>>().join(" ");
        let rest = errors.len().saturating_sub(8);
        return Err(if rest > 0 {
            format!("{shown} Weitere fehlerhafte Zeilen: {rest}.")
        } else {
            shown
        });
    }
    if transactions.is_empty() {
        return Err("Es wurden keine gültigen Buchungen gefunden.".into());
    }
    let closing_balance_minor = transactions.last().and_then(|row| row.balance_minor);
    Ok(ParsedStatement {
        currency_balances: Vec::new(),
        account_type: None,
        provider,
        format: format!("{} (Mapping)", path.extension().and_then(|value| value.to_str()).unwrap_or("xlsx").to_ascii_uppercase()),
        account_name: "Zugeordnetes Konto".into(),
        transactions,
        opening_balance_minor: None,
        closing_balance_minor,
        warnings: vec!["Spalten wurden mit einem benutzerdefinierten Excel-Mapping eingelesen. Bitte Summen und Vorzeichen in der Vorschau kontrollieren.".into()],
        ..ParsedStatement::default()
    })
}

fn parse_row(
    row: &[Data],
    source_row: usize,
    mapping: &TabularMapping,
    fixed_currency: &str,
) -> Result<Option<ParsedTransaction>, String> {
    let description = mapping
        .description_columns
        .iter()
        .map(|index| cell_string(row, *index))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    if description.is_empty() {
        return Err(format!("Zeile {source_row}: Beschreibung fehlt."));
    }
    let description_key = normalized(&description);
    if is_opening_label(&description_key)
        || is_closing_label(&description_key)
        || description_key == "total"
        || description_key.starts_with("total ")
        || description_key == "summe"
        || description_key.starts_with("summe ")
        || description_key.starts_with("gesamtsumme")
    {
        return Ok(None);
    }
    let booking_date = mapped_date(row, mapping.date_column, &mapping.date_format)
        .ok_or_else(|| format!("Zeile {source_row}: Buchungsdatum fehlt oder ist ungültig."))?;
    let amount = if let Some(column) = mapping.amount_column {
        let value = mapped_money(row, column, &mapping.number_format)
            .ok_or_else(|| format!("Zeile {source_row}: Betrag fehlt oder ist ungültig."))?;
        if mapping.invert_amount {
            -value
        } else {
            value
        }
    } else {
        let debit_text = mapping
            .debit_column
            .map(|column| cell_string(row, column))
            .unwrap_or_default();
        let credit_text = mapping
            .credit_column
            .map(|column| cell_string(row, column))
            .unwrap_or_default();
        if debit_text.is_empty() && credit_text.is_empty() {
            return Err(format!(
                "Zeile {source_row}: Belastung oder Gutschrift fehlt."
            ));
        }
        let debit = mapped_optional_money(
            row,
            mapping.debit_column,
            &mapping.number_format,
            source_row,
            "Belastung",
        )?
        .unwrap_or(0)
        .abs();
        let credit = mapped_optional_money(
            row,
            mapping.credit_column,
            &mapping.number_format,
            source_row,
            "Gutschrift",
        )?
        .unwrap_or(0)
        .abs();
        credit - debit
    };
    let currency = if let Some(column) = mapping.currency_column {
        let value = cell_string(row, column);
        if value.is_empty() {
            fixed_currency.to_string()
        } else {
            normalize_currency(&value)?
        }
    } else {
        fixed_currency.to_string()
    };
    Ok(Some(ParsedTransaction {
        booking_date,
        value_date: mapped_optional_date(
            row,
            mapping.value_date_column,
            &mapping.date_format,
            source_row,
        )?,
        description,
        industry: mapping
            .industry_column
            .map(|column| cell_string(row, column))
            .filter(|value| !value.is_empty()),
        amount_minor: amount,
        balance_minor: mapped_optional_money(
            row,
            mapping.balance_column,
            &mapping.number_format,
            source_row,
            "Saldo",
        )?,
        currency,
        confidence: 1.0,
        source_row,
        ..ParsedTransaction::default()
    }))
}

fn validate_mapping(mapping: &TabularMapping) -> Result<(), String> {
    if mapping.sheet_name.trim().is_empty()
        || mapping.header_row == 0
        || mapping.data_start_row <= mapping.header_row
    {
        return Err(
            "Bitte Tabellenblatt, Kopfzeile und eine spätere erste Datenzeile wählen.".into(),
        );
    }
    if mapping.description_columns.is_empty() {
        return Err("Bitte mindestens eine Beschreibungsspalte wählen.".into());
    }
    if mapping.amount_column.is_none()
        && mapping.debit_column.is_none()
        && mapping.credit_column.is_none()
    {
        return Err("Bitte eine Betragsspalte oder Belastung/Gutschrift zuordnen.".into());
    }
    if mapping.amount_column.is_some()
        && (mapping.debit_column.is_some() || mapping.credit_column.is_some())
    {
        return Err("Bitte entweder Betrag oder Belastung/Gutschrift verwenden.".into());
    }
    Ok(())
}

fn mapped_date(row: &[Data], column: usize, format: &DateFormat) -> Option<String> {
    row.get(column)
        .and_then(|cell| cell.as_date())
        .map(|date| date.format("%Y-%m-%d").to_string())
        .or_else(|| {
            let value = row.get(column)?.to_string();
            let formats: &[&str] = match format {
                DateFormat::Auto => &["%Y-%m-%d", "%d.%m.%Y", "%d.%m.%y", "%d/%m/%Y", "%m/%d/%Y"],
                DateFormat::Dmy => &["%d.%m.%Y", "%d.%m.%y", "%d/%m/%Y", "%d-%m-%Y"],
                DateFormat::Mdy => &["%m/%d/%Y", "%m-%d-%Y"],
                DateFormat::Ymd => &["%Y-%m-%d", "%Y/%m/%d"],
            };
            formats
                .iter()
                .find_map(|pattern| NaiveDate::parse_from_str(value.trim(), pattern).ok())
                .map(|date| date.format("%Y-%m-%d").to_string())
        })
}

fn mapped_optional_date(
    row: &[Data],
    column: Option<usize>,
    format: &DateFormat,
    source_row: usize,
) -> Result<Option<String>, String> {
    let Some(column) = column else {
        return Ok(None);
    };
    if cell_string(row, column).is_empty() {
        return Ok(None);
    }
    mapped_date(row, column, format)
        .ok_or_else(|| format!("Zeile {source_row}: Valutadatum ist ungültig."))
        .map(Some)
}

fn mapped_optional_money(
    row: &[Data],
    column: Option<usize>,
    format: &NumberFormat,
    source_row: usize,
    label: &str,
) -> Result<Option<i64>, String> {
    let Some(column) = column else {
        return Ok(None);
    };
    let text = cell_string(row, column);
    if text.is_empty() {
        return Ok(None);
    }
    mapped_money(row, column, format)
        .ok_or_else(|| format!("Zeile {source_row}: {label} ist ungültig."))
        .map(Some)
}

fn mapped_money(row: &[Data], column: usize, format: &NumberFormat) -> Option<i64> {
    if let Some(number) = row.get(column).and_then(|cell| cell.as_f64()) {
        return Some((number * 100.0).round() as i64);
    }
    parse_mapped_money(&cell_string(row, column), format)
}

fn parse_mapped_money(value: &str, format: &NumberFormat) -> Option<i64> {
    let original = value.trim();
    if original.is_empty() {
        return None;
    }
    let negative = original.starts_with('-')
        || (original.starts_with('(') && original.ends_with(')'))
        || original.ends_with('-');
    let mut cleaned: String = original
        .chars()
        .filter(|character| character.is_ascii_digit() || matches!(character, '.' | ',' | '-'))
        .collect();
    cleaned = cleaned.trim_matches('-').to_string();
    match format {
        NumberFormat::DecimalComma => cleaned = cleaned.replace('.', "").replace(',', "."),
        NumberFormat::DecimalPoint => cleaned = cleaned.replace(',', ""),
        NumberFormat::Auto => {
            let comma = cleaned.rfind(',');
            let dot = cleaned.rfind('.');
            let decimal = match (comma, dot) {
                (Some(c), Some(d)) => Some(c.max(d)),
                (Some(c), None) if cleaned.len() - c - 1 <= 2 => Some(c),
                (None, Some(d)) if cleaned.len() - d - 1 <= 2 => Some(d),
                _ => None,
            };
            cleaned = cleaned
                .chars()
                .enumerate()
                .filter_map(|(index, character)| {
                    if Some(index) == decimal {
                        Some('.')
                    } else if character == ',' || character == '.' {
                        None
                    } else {
                        Some(character)
                    }
                })
                .collect();
        }
    }
    cleaned.parse::<f64>().ok().map(|number| {
        let minor = (number * 100.0).round() as i64;
        if negative {
            -minor
        } else {
            minor
        }
    })
}

fn normalize_currency(value: &str) -> Result<String, String> {
    let currency = value.trim().to_ascii_uppercase();
    if currency.len() == 3
        && currency
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        Ok(currency)
    } else {
        Err(format!(
            "Ungültige Währung: {value}. Bitte einen dreistelligen Code wie CHF verwenden."
        ))
    }
}

fn ensure_workbook(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|_| "Die ausgewählte Datei ist nicht mehr verfügbar.".to_string())?;
    if !metadata.is_file() {
        return Err("Bitte eine Datei und keinen Ordner auswählen.".into());
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err("Die Datei ist größer als 25 MB.".into());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "xlsx" | "xls") {
        return Err("Das Spalten-Mapping unterstützt XLSX- und XLS-Dateien.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_number_formats_without_losing_signs() {
        assert_eq!(
            parse_mapped_money("CHF 1'234.50", &NumberFormat::Auto),
            Some(123450)
        );
        assert_eq!(
            parse_mapped_money("1.234,50-", &NumberFormat::Auto),
            Some(-123450)
        );
        assert_eq!(
            parse_mapped_money("(1,234.50)", &NumberFormat::Auto),
            Some(-123450)
        );
        assert_eq!(
            parse_mapped_money("1.234", &NumberFormat::DecimalComma),
            Some(123400)
        );
    }

    #[test]
    fn detects_custom_excel_header_aliases() {
        let row = vec![
            Data::String("Kaufdatum".into()),
            Data::String("Notiz".into()),
            Data::String("Gegenpartei".into()),
            Data::String("Abfluss".into()),
            Data::String("Zufluss".into()),
        ];
        assert!(header_score(&row) > 10);
    }

    #[test]
    fn mapped_excel_uses_selected_sheet_columns_and_skips_balance_markers() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures")
            .join("bank-statements")
            .join("xlsx")
            .join("ubs_kontoauszug_2026-08.xlsx");
        let mapping = TabularMapping {
            sheet_name: "Kontobewegungen".into(),
            header_row: 4,
            data_start_row: 5,
            date_column: 0,
            description_columns: vec![2],
            amount_column: None,
            debit_column: Some(3),
            credit_column: Some(4),
            value_date_column: Some(1),
            balance_column: Some(5),
            currency_column: Some(6),
            industry_column: None,
            fixed_currency: "CHF".into(),
            invert_amount: false,
            date_format: DateFormat::Auto,
            number_format: NumberFormat::Auto,
        };
        let parsed = parse_mapped_workbook(&path, None, &mapping).unwrap();
        assert_eq!(parsed.provider, "unknown");
        assert_eq!(parsed.transactions.len(), 8);
        assert_eq!(parsed.transactions[0].amount_minor, -185_000);
        assert_eq!(parsed.transactions[2].amount_minor, 685_000);
        assert_eq!(parsed.closing_balance_minor, Some(1_078_450));
        let inspection = inspect_tabular_file(path.to_string_lossy().into_owned()).unwrap();
        assert_eq!(inspection.sheets[0].suggested_header_row, 4);
    }
}
