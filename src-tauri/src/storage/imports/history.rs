//! Listet und entfernt Importläufe mit ihren zugehörigen Buchungen.
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::ImportRun;

pub(crate) fn list_imports(storage: &Storage) -> Result<Vec<ImportRun>, String> {
    imports_from(storage)
}

pub(crate) fn imports_from(storage: &Storage) -> Result<Vec<ImportRun>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection.prepare("SELECT ir.id, ir.source_name, ir.source_format, i.name,
        (SELECT group_concat(a.name || ' (' || a.currency || ')', ', ') FROM accounts a
          WHERE a.id = ir.account_id OR EXISTS(SELECT 1 FROM transactions t WHERE t.import_id = ir.id AND t.account_id = a.id)
          OR EXISTS(SELECT 1 FROM balance_snapshots b WHERE b.import_id = ir.id AND b.account_id = a.id)),
        ir.imported_at, (SELECT COUNT(*) FROM transactions t WHERE t.import_id = ir.id),
        (SELECT COUNT(*) FROM ignored_duplicate_transactions ignored JOIN transactions t ON t.id=ignored.transaction_id WHERE t.import_id=ir.id),
        (SELECT MIN(booking_date) FROM transactions t WHERE t.import_id = ir.id),
        (SELECT MAX(booking_date) FROM transactions t WHERE t.import_id = ir.id)
        FROM import_runs ir JOIN accounts main ON main.id = ir.account_id
        JOIN institutions i ON i.id = main.institution_id ORDER BY ir.imported_at DESC, ir.id DESC").map_err(db_error)?;
    let rows = query
        .query_map([], |row| {
            Ok(ImportRun {
                id: row.get(0)?,
                source_name: row.get(1)?,
                source_format: row.get(2)?,
                provider: row.get(3)?,
                accounts: row.get(4)?,
                imported_at: row.get(5)?,
                transaction_count: row.get(6)?,
                ignored_duplicate_count: row.get(7)?,
                first_date: row.get(8)?,
                last_date: row.get(9)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub(crate) fn delete_imports(storage: &Storage, ids: Vec<i64>) -> Result<usize, String> {
    delete_imports_from(storage, ids)
}

pub(crate) fn delete_imports_from(storage: &Storage, ids: Vec<i64>) -> Result<usize, String> {
    let ids: std::collections::BTreeSet<i64> = ids.into_iter().collect();
    if ids.is_empty() {
        return Err("Bitte mindestens einen Import auswählen.".into());
    }
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    for id in &ids {
        // Foreign-key cascades remove every associated transaction and snapshot,
        // including all currency accounts. Account records and source files stay.
        if transaction
            .execute("DELETE FROM import_runs WHERE id = ?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("Ein ausgewählter Import existiert nicht mehr. Bitte die Liste aktualisieren; es wurde nichts gelöscht.".into());
        }
    }
    transaction.commit().map_err(db_error)?;
    Ok(ids.len())
}

pub(crate) fn restore_import_duplicates(
    storage: &Storage,
    import_id: i64,
) -> Result<usize, String> {
    let connection = storage.connect().map_err(db_error)?;
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM import_runs WHERE id=?1)",
            [import_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)?;
    if !exists {
        return Err("Import nicht gefunden. Bitte die Liste aktualisieren.".into());
    }
    connection
        .execute(
            "DELETE FROM ignored_duplicate_transactions
             WHERE transaction_id IN (SELECT id FROM transactions WHERE import_id=?1)",
            [import_id],
        )
        .map_err(db_error)
}
