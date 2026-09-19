//! Entfernt alte lokale Chat-Zugangsdaten bei sicherheitsrelevanten Datenbankoperationen.
use rusqlite::Connection;
// Remove credentials left by the retired API integration, including older backups.
pub(crate) fn clear_config(db: &Connection) -> rusqlite::Result<()> {
    // Older backups may predate this optional table.
    let exists: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='finance_chat_settings' AND type='table')", [], |r| r.get(0))?;
    if exists {
        db.execute_batch("PRAGMA secure_delete=ON;")?;
        db.execute("DELETE FROM finance_chat_settings", [])?;
    }
    Ok(())
}
