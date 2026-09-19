//! Gemeinsame Zählabfragen für Datenbankprojektionen.
use super::errors::db_error;
use rusqlite::Connection;

pub(in crate::storage) fn count(connection: &Connection, table: &str) -> Result<i64, String> {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(db_error)
}
