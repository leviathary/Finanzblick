use super::{db_error, Storage};
use chrono::Utc;
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use tauri::{Manager, State};

const PARSER_VERSION: &str = "zurich-tax-v2";
const MANUAL_ENTRY_VERSION: &str = "manual-v1";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxStatementPreview {
    pub source_name: String,
    pub tax_year: i32,
    pub valuation_date: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
    pub currency: String,
    pub confidence: f64,
    pub warnings: Vec<String>,
    pub existing_year: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxSnapshot {
    pub id: i64,
    pub tax_year: i32,
    pub valuation_date: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
    pub currency: String,
    pub source_name: String,
    pub confidence: f64,
    pub imported_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTaxStatementRequest {
    pub source_path: String,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveManualTaxSnapshotRequest {
    pub tax_year: i32,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTaxStatementResult {
    pub snapshot: TaxSnapshot,
    pub replaced_existing_year: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaxSnapshotRequest {
    pub id: i64,
    pub gross_assets_minor: i64,
    pub liabilities_minor: i64,
    pub taxable_wealth_minor: i64,
    pub securities_and_cash_minor: i64,
    pub real_estate_minor: i64,
    pub other_assets_minor: i64,
    pub canton_taxable_wealth_minor: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct AmountOccurrence {
    value: i64,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct ParsedTaxStatement {
    source_name: String,
    source_hash: String,
    tax_year: i32,
    gross_assets_minor: i64,
    liabilities_minor: i64,
    taxable_wealth_minor: i64,
    securities_and_cash_minor: i64,
    real_estate_minor: i64,
    other_assets_minor: i64,
    confidence: f64,
    warnings: Vec<String>,
}

struct TaxSnapshotInput {
    tax_year: i32,
    gross_assets_minor: i64,
    liabilities_minor: i64,
    taxable_wealth_minor: i64,
    securities_and_cash_minor: i64,
    real_estate_minor: i64,
    other_assets_minor: i64,
    canton_taxable_wealth_minor: Option<i64>,
    source_name: String,
    source_hash: String,
    parser_version: &'static str,
    confidence: f64,
}

#[tauri::command]
pub async fn preview_tax_statement(
    app: tauri::AppHandle,
    path: String,
) -> Result<TaxStatementPreview, String> {
    let parsed =
        tauri::async_runtime::spawn_blocking(move || parse_tax_statement_file(Path::new(&path)))
            .await
            .map_err(|_| "Die Steuererklärung konnte nicht verarbeitet werden.".to_string())??;
    let storage = app.state::<Storage>();
    let connection = storage.connect().map_err(db_error)?;
    let existing_year = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM annual_tax_snapshots WHERE tax_year=?1)",
            [parsed.tax_year],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    Ok(TaxStatementPreview {
        source_name: parsed.source_name,
        tax_year: parsed.tax_year,
        valuation_date: format!("{}-12-31", parsed.tax_year),
        gross_assets_minor: parsed.gross_assets_minor,
        liabilities_minor: parsed.liabilities_minor,
        taxable_wealth_minor: parsed.taxable_wealth_minor,
        securities_and_cash_minor: parsed.securities_and_cash_minor,
        real_estate_minor: parsed.real_estate_minor,
        other_assets_minor: parsed.other_assets_minor,
        canton_taxable_wealth_minor: None,
        currency: "CHF".into(),
        confidence: parsed.confidence,
        warnings: parsed.warnings,
        existing_year,
    })
}

#[tauri::command]
pub async fn save_tax_statement(
    app: tauri::AppHandle,
    request: SaveTaxStatementRequest,
) -> Result<SaveTaxStatementResult, String> {
    let source_path = request.source_path.clone();
    let parsed = tauri::async_runtime::spawn_blocking(move || {
        parse_tax_statement_file(Path::new(&source_path))
    })
    .await
    .map_err(|_| "Die Steuererklärung konnte nicht verarbeitet werden.".to_string())??;
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    let storage = app.state::<Storage>();
    let mut connection = storage.connect().map_err(db_error)?;
    persist_tax_snapshot(
        &mut connection,
        TaxSnapshotInput {
            tax_year: parsed.tax_year,
            gross_assets_minor: request.gross_assets_minor,
            liabilities_minor: request.liabilities_minor,
            taxable_wealth_minor: request.taxable_wealth_minor,
            securities_and_cash_minor: request.securities_and_cash_minor,
            real_estate_minor: request.real_estate_minor,
            other_assets_minor: request.other_assets_minor,
            canton_taxable_wealth_minor: request.canton_taxable_wealth_minor,
            source_name: format!("Steuererklärung {}.pdf", parsed.tax_year),
            source_hash: parsed.source_hash,
            parser_version: PARSER_VERSION,
            confidence: parsed.confidence,
        },
    )
}

#[tauri::command]
pub fn save_manual_tax_snapshot(
    storage: State<'_, Storage>,
    request: SaveManualTaxSnapshotRequest,
) -> Result<SaveTaxStatementResult, String> {
    if !(1990..=2100).contains(&request.tax_year) {
        return Err("Bitte ein Steuerjahr zwischen 1990 und 2100 eingeben.".into());
    }
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    validate_breakdown(request.securities_and_cash_minor, request.real_estate_minor)?;
    let mut connection = storage.connect().map_err(db_error)?;
    persist_tax_snapshot(
        &mut connection,
        TaxSnapshotInput {
            tax_year: request.tax_year,
            gross_assets_minor: request.gross_assets_minor,
            liabilities_minor: request.liabilities_minor,
            taxable_wealth_minor: request.taxable_wealth_minor,
            securities_and_cash_minor: request.securities_and_cash_minor,
            real_estate_minor: request.real_estate_minor,
            other_assets_minor: request.other_assets_minor,
            canton_taxable_wealth_minor: request.canton_taxable_wealth_minor,
            source_name: format!("Manuelle Eingabe {}", request.tax_year),
            source_hash: format!("manual:{}", request.tax_year),
            parser_version: MANUAL_ENTRY_VERSION,
            confidence: 1.0,
        },
    )
}

fn persist_tax_snapshot(
    connection: &mut rusqlite::Connection,
    input: TaxSnapshotInput,
) -> Result<SaveTaxStatementResult, String> {
    let transaction = connection.transaction().map_err(db_error)?;
    let replaced_existing_year = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM annual_tax_snapshots WHERE tax_year=?1)",
            [input.tax_year],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let imported_at = Utc::now().to_rfc3339();
    transaction
        .execute(
            "INSERT INTO annual_tax_snapshots(
               tax_year,valuation_date,gross_assets_minor,liabilities_minor,
               taxable_wealth_minor,canton_taxable_wealth_minor,currency,
               source_name,source_hash,parser_version,extraction_confidence,imported_at
             ) VALUES(?1,?2,?3,?4,?5,?6,'CHF',?7,?8,?9,?10,?11)
             ON CONFLICT(tax_year) DO UPDATE SET
               valuation_date=excluded.valuation_date,
               gross_assets_minor=excluded.gross_assets_minor,
               liabilities_minor=excluded.liabilities_minor,
               taxable_wealth_minor=excluded.taxable_wealth_minor,
               canton_taxable_wealth_minor=excluded.canton_taxable_wealth_minor,
               currency=excluded.currency,
               source_name=excluded.source_name,
               source_hash=excluded.source_hash,
               parser_version=excluded.parser_version,
               extraction_confidence=excluded.extraction_confidence,
               imported_at=excluded.imported_at",
            params![
                input.tax_year,
                format!("{}-12-31", input.tax_year),
                input.gross_assets_minor,
                input.liabilities_minor,
                input.taxable_wealth_minor,
                input.canton_taxable_wealth_minor,
                input.source_name,
                input.source_hash,
                input.parser_version,
                input.confidence,
                imported_at,
            ],
        )
        .map_err(db_error)?;
    let snapshot_id: i64 = transaction
        .query_row(
            "SELECT id FROM annual_tax_snapshots WHERE tax_year=?1",
            [input.tax_year],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    transaction
        .execute(
            "INSERT INTO annual_tax_snapshot_breakdowns(
               snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor
             ) VALUES(?1,?2,?3,?4)
             ON CONFLICT(snapshot_id) DO UPDATE SET
               securities_and_cash_minor=excluded.securities_and_cash_minor,
               real_estate_minor=excluded.real_estate_minor,
               other_assets_minor=excluded.other_assets_minor",
            params![
                snapshot_id,
                input.securities_and_cash_minor,
                input.real_estate_minor,
                input.other_assets_minor,
            ],
        )
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)?;
    let snapshot = snapshot_for_year(connection, input.tax_year)?;
    Ok(SaveTaxStatementResult {
        snapshot,
        replaced_existing_year,
    })
}

#[tauri::command]
pub fn list_tax_snapshots(storage: State<'_, Storage>) -> Result<Vec<TaxSnapshot>, String> {
    let connection = storage.connect().map_err(db_error)?;
    snapshots_from(&connection)
}

#[tauri::command]
pub fn update_tax_snapshot(
    storage: State<'_, Storage>,
    request: UpdateTaxSnapshotRequest,
) -> Result<TaxSnapshot, String> {
    validate_amounts(
        request.gross_assets_minor,
        request.liabilities_minor,
        request.taxable_wealth_minor,
        request.canton_taxable_wealth_minor,
    )?;
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    if transaction
        .execute(
            "UPDATE annual_tax_snapshots SET
               gross_assets_minor=?1, liabilities_minor=?2, taxable_wealth_minor=?3,
               canton_taxable_wealth_minor=?4
             WHERE id=?5",
            params![
                request.gross_assets_minor,
                request.liabilities_minor,
                request.taxable_wealth_minor,
                request.canton_taxable_wealth_minor,
                request.id,
            ],
        )
        .map_err(db_error)?
        != 1
    {
        return Err("Der ausgewählte Steuerwert existiert nicht mehr.".into());
    }
    transaction
        .execute(
            "INSERT INTO annual_tax_snapshot_breakdowns(
               snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor
             ) VALUES(?1,?2,?3,?4)
             ON CONFLICT(snapshot_id) DO UPDATE SET
               securities_and_cash_minor=excluded.securities_and_cash_minor,
               real_estate_minor=excluded.real_estate_minor,
               other_assets_minor=excluded.other_assets_minor",
            params![
                request.id,
                request.securities_and_cash_minor,
                request.real_estate_minor,
                request.other_assets_minor,
            ],
        )
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)?;
    snapshot_for_id(&connection, request.id)
}

#[tauri::command]
pub fn delete_tax_snapshot(storage: State<'_, Storage>, id: i64) -> Result<(), String> {
    let connection = storage.connect().map_err(db_error)?;
    if connection
        .execute("DELETE FROM annual_tax_snapshots WHERE id=?1", [id])
        .map_err(db_error)?
        != 1
    {
        return Err("Der ausgewählte Steuerwert existiert nicht mehr.".into());
    }
    Ok(())
}

fn snapshots_from(connection: &rusqlite::Connection) -> Result<Vec<TaxSnapshot>, String> {
    let mut query = connection
        .prepare(
            "SELECT s.id,s.tax_year,s.valuation_date,s.gross_assets_minor,s.liabilities_minor,
                    s.taxable_wealth_minor,
                    COALESCE(b.securities_and_cash_minor,0),
                    COALESCE(b.real_estate_minor,0),
                    COALESCE(b.other_assets_minor,s.gross_assets_minor),
                    s.canton_taxable_wealth_minor,s.currency,
                    s.source_name,s.extraction_confidence,s.imported_at
             FROM annual_tax_snapshots s
             LEFT JOIN annual_tax_snapshot_breakdowns b ON b.snapshot_id=s.id
             ORDER BY s.tax_year",
        )
        .map_err(db_error)?;
    let snapshots = query
        .query_map([], snapshot_from_row)
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(snapshots)
}

fn snapshot_for_year(
    connection: &rusqlite::Connection,
    tax_year: i32,
) -> Result<TaxSnapshot, String> {
    connection
        .query_row(
            "SELECT s.id,s.tax_year,s.valuation_date,s.gross_assets_minor,s.liabilities_minor,
                    s.taxable_wealth_minor,
                    COALESCE(b.securities_and_cash_minor,0),
                    COALESCE(b.real_estate_minor,0),
                    COALESCE(b.other_assets_minor,s.gross_assets_minor),
                    s.canton_taxable_wealth_minor,s.currency,
                    s.source_name,s.extraction_confidence,s.imported_at
             FROM annual_tax_snapshots s
             LEFT JOIN annual_tax_snapshot_breakdowns b ON b.snapshot_id=s.id
             WHERE s.tax_year=?1",
            [tax_year],
            snapshot_from_row,
        )
        .map_err(db_error)
}

fn snapshot_for_id(connection: &rusqlite::Connection, id: i64) -> Result<TaxSnapshot, String> {
    connection
        .query_row(
            "SELECT s.id,s.tax_year,s.valuation_date,s.gross_assets_minor,s.liabilities_minor,
                    s.taxable_wealth_minor,
                    COALESCE(b.securities_and_cash_minor,0),
                    COALESCE(b.real_estate_minor,0),
                    COALESCE(b.other_assets_minor,s.gross_assets_minor),
                    s.canton_taxable_wealth_minor,s.currency,
                    s.source_name,s.extraction_confidence,s.imported_at
             FROM annual_tax_snapshots s
             LEFT JOIN annual_tax_snapshot_breakdowns b ON b.snapshot_id=s.id
             WHERE s.id=?1",
            [id],
            snapshot_from_row,
        )
        .map_err(db_error)
}

fn snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaxSnapshot> {
    Ok(TaxSnapshot {
        id: row.get(0)?,
        tax_year: row.get(1)?,
        valuation_date: row.get(2)?,
        gross_assets_minor: row.get(3)?,
        liabilities_minor: row.get(4)?,
        taxable_wealth_minor: row.get(5)?,
        securities_and_cash_minor: row.get(6)?,
        real_estate_minor: row.get(7)?,
        other_assets_minor: row.get(8)?,
        canton_taxable_wealth_minor: row.get(9)?,
        currency: row.get(10)?,
        source_name: row.get(11)?,
        confidence: row.get(12)?,
        imported_at: row.get(13)?,
    })
}

fn validate_amounts(
    gross_assets_minor: i64,
    liabilities_minor: i64,
    taxable_wealth_minor: i64,
    canton_taxable_wealth_minor: Option<i64>,
) -> Result<(), String> {
    if gross_assets_minor < 0 || liabilities_minor < 0 || taxable_wealth_minor < 0 {
        return Err("Vermögen und Schulden dürfen nicht negativ sein.".into());
    }
    if gross_assets_minor
        .checked_sub(liabilities_minor)
        .ok_or_else(|| "Die Beträge sind ausserhalb des unterstützten Bereichs.".to_string())?
        != taxable_wealth_minor
    {
        return Err(
            "Die Plausibilitätsprüfung ist fehlgeschlagen: Vermögenswerte minus Schulden müssen dem steuerbaren Vermögen entsprechen."
                .into(),
        );
    }
    if canton_taxable_wealth_minor.is_some_and(|value| value < 0 || value > taxable_wealth_minor) {
        return Err("Das kantonale steuerbare Vermögen ist nicht plausibel.".into());
    }
    Ok(())
}

fn validate_breakdown(
    securities_and_cash_minor: i64,
    real_estate_minor: i64,
) -> Result<(), String> {
    if securities_and_cash_minor < 0 || real_estate_minor < 0 {
        return Err("Wertschriften, Guthaben und Liegenschaften dürfen nicht negativ sein.".into());
    }
    Ok(())
}

fn parse_tax_statement_file(path: &Path) -> Result<ParsedTaxStatement, String> {
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
    fn stores_and_replaces_a_manual_tax_year_without_a_pdf() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::initialize_schema(&connection).unwrap();
        let input = |gross_assets_minor, source_hash: &str| TaxSnapshotInput {
            tax_year: 2025,
            gross_assets_minor,
            liabilities_minor: 20_000,
            taxable_wealth_minor: gross_assets_minor - 20_000,
            securities_and_cash_minor: 60_000,
            real_estate_minor: 30_000,
            other_assets_minor: gross_assets_minor - 90_000,
            canton_taxable_wealth_minor: None,
            source_name: "Manuelle Eingabe 2025".into(),
            source_hash: source_hash.into(),
            parser_version: MANUAL_ENTRY_VERSION,
            confidence: 1.0,
        };

        let created = persist_tax_snapshot(&mut connection, input(100_000, "manual:2025")).unwrap();
        assert!(!created.replaced_existing_year);
        assert_eq!(created.snapshot.gross_assets_minor, 100_000);

        let replaced =
            persist_tax_snapshot(&mut connection, input(120_000, "manual:2025")).unwrap();
        assert!(replaced.replaced_existing_year);
        assert_eq!(replaced.snapshot.gross_assets_minor, 120_000);
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM annual_tax_snapshots", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            1
        );
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
