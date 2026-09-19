//! Speichert jährliche Steuerwerte und deren Vermögensaufteilung atomar.
use crate::domain::taxes::{
    SaveTaxStatementResult, TaxSnapshot, TaxSnapshotInput, UpdateTaxSnapshotRequest,
};
use crate::storage::db_error;
use chrono::Utc;
use rusqlite::params;
pub(crate) fn persist_tax_snapshot(
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

pub fn update_tax_snapshot(
    connection: &mut rusqlite::Connection,
    request: UpdateTaxSnapshotRequest,
) -> Result<TaxSnapshot, String> {
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
    snapshot_for_id(connection, request.id)
}

pub fn delete_tax_snapshot(connection: &rusqlite::Connection, id: i64) -> Result<(), String> {
    if connection
        .execute("DELETE FROM annual_tax_snapshots WHERE id=?1", [id])
        .map_err(db_error)?
        != 1
    {
        return Err("Der ausgewählte Steuerwert existiert nicht mehr.".into());
    }
    Ok(())
}

pub(crate) fn snapshots_from(
    connection: &rusqlite::Connection,
) -> Result<Vec<TaxSnapshot>, String> {
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

pub(crate) fn year_exists(connection: &rusqlite::Connection, year: i32) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM annual_tax_snapshots WHERE tax_year=?1)",
            [year],
            |row| row.get(0),
        )
        .map_err(db_error)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::taxes::MANUAL_ENTRY_VERSION;
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
}
