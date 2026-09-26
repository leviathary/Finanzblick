//! Erstellt, prüft und restauriert verschlüsselte Datensicherungen.

use super::connection::open;
use super::handle::Storage;
#[cfg(test)]
use super::session::Session;
use super::settings::read_settings;
use super::LOCKED;
#[cfg(test)]
use rusqlite::Connection;
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::sync::RwLock;
use zeroize::Zeroizing;

const MAX_BACKUP_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

impl Storage {
    pub(crate) fn create_backup_file(&self, destination: &Path) -> Result<(), String> {
        if destination.extension().and_then(|value| value.to_str()) != Some("saldonaut-backup") {
            return Err("Bitte eine Datei mit der Endung .saldonaut-backup wählen.".into());
        }
        let session = self.session.write().map_err(|_| LOCKED)?;
        let password = session
            .password
            .as_ref()
            .filter(|_| !session.expired())
            .ok_or(LOCKED)?;
        let parent = destination.parent().ok_or("Speicherort fehlt.")?;
        let stage = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Backup konnte nicht erstellt werden.")?;
        {
            let source = open(&self.database_path(&session), password, false)
                .map_err(|_| "Finanzprofil konnte nicht gelesen werden.")?;
            source
                .execute(
                    "ATTACH DATABASE ?1 AS backup_database KEY ?2",
                    rusqlite::params![stage.path().to_string_lossy(), password.as_str()],
                )
                .map_err(|_| "Backup konnte nicht erstellt werden.")?;
            source
                .query_row("SELECT sqlcipher_export('backup_database')", [], |_| Ok(()))
                .map_err(|_| "Backup konnte nicht erstellt werden.")?;
            source
                .execute_batch("DETACH DATABASE backup_database;")
                .map_err(|_| "Backup konnte nicht erstellt werden.")?;
        }
        validate_backup(stage.path(), password)?;
        {
            let backup = open(stage.path(), password, false)
                .map_err(|_| "Backup konnte nicht erstellt werden.")?;
            crate::storage::assistant::credentials::clear_config(&backup)
                .map_err(|_| "Backup konnte nicht erstellt werden.")?;
        }
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Backup konnte nicht gespeichert werden.")?;
        stage.persist_noclobber(destination).map_err(|_| {
            "Backup konnte nicht gespeichert werden. Bitte einen neuen Dateinamen wählen."
        })?;
        Ok(())
    }

    pub(crate) fn restore_backup_file(
        &self,
        source: &Path,
        name: &str,
        password: String,
    ) -> Result<(), String> {
        let password = Zeroizing::new(password);
        let _session = self.session.write().map_err(|_| LOCKED)?;
        let destination = self.available_destination(name)?;
        let parent = destination.parent().ok_or("Speicherort fehlt.")?;
        let mut stage = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Backup konnte nicht importiert werden.")?;
        let mut input =
            fs::File::open(source).map_err(|_| "Backup konnte nicht gelesen werden.")?;
        let metadata = input
            .metadata()
            .map_err(|_| "Backup konnte nicht gelesen werden.")?;
        if !metadata.is_file() {
            return Err("Bitte eine Backup-Datei wählen.".into());
        }
        if metadata.len() > MAX_BACKUP_FILE_SIZE {
            return Err("Das Backup ist grösser als 2 GB und kann nicht importiert werden.".into());
        }
        let copied = std::io::copy(
            &mut std::io::Read::take(&mut input, MAX_BACKUP_FILE_SIZE + 1),
            stage.as_file_mut(),
        )
        .map_err(|_| "Backup konnte nicht gelesen werden.")?;
        if copied > MAX_BACKUP_FILE_SIZE {
            return Err("Das Backup ist grösser als 2 GB und kann nicht importiert werden.".into());
        }
        validate_backup(stage.path(), &password)?;
        {
            let restored = open(stage.path(), &password, false)
                .map_err(|_| "Backup konnte nicht importiert werden.")?;
            crate::storage::assistant::credentials::clear_config(&restored)
                .map_err(|_| "Backup konnte nicht importiert werden.")?;
        }
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Backup konnte nicht gespeichert werden.")?;
        stage
            .persist_noclobber(destination)
            .map_err(|_| "Ein Finanzprofil mit diesem Namen existiert bereits.")?;
        Ok(())
    }
}

fn validate_backup(path: &Path, password: &str) -> Result<(), String> {
    let db = open(path, password, false).map_err(|_| "Backup oder Passwort ungültig.")?;
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| "Backup ist beschädigt.")?;
    if integrity != "ok" {
        return Err("Backup ist beschädigt.".into());
    }
    for table in ["accounts", "transactions", "import_runs", "app_settings"] {
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(|_| "Ungültiges Saldonaut-Backup.")?;
        if !exists {
            return Err("Ungültiges Saldonaut-Backup.".into());
        }
    }
    read_settings(&db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_credentials_and_consent_do_not_follow_copies_backups_or_anonymization() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("vault.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        storage.unlock("test-password".into(), true).unwrap();
        let seed = |db: &Connection| {
            db.execute(
                "INSERT OR REPLACE INTO finance_chat_settings VALUES(1,?1)",
                [r#"{"enabled":true,"provider":"openai","model":"test","apiKey":"secret"}"#],
            )
            .unwrap();
        };
        let count = |db: &Connection| {
            db.query_row("SELECT COUNT(*) FROM finance_chat_settings", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        };
        seed(&storage.connect().unwrap());
        let backup = directory.path().join("chat.saldonaut-backup");
        storage.create_backup_file(&backup).unwrap();
        assert_eq!(count(&storage.connect().unwrap()), 1);
        {
            let exported = open(&backup, "test-password", false).unwrap();
            assert_eq!(count(&exported), 0);
            // Simulate an older/external backup that still includes a credential.
            seed(&exported);
        }
        storage
            .restore_backup_file(&backup, "restored-chat", "test-password".into())
            .unwrap();
        assert_eq!(
            count(
                &open(
                    &directory.path().join("restored-chat.vault.sqlite3"),
                    "test-password",
                    false
                )
                .unwrap()
            ),
            0
        );
        storage.copy_database("copied-chat").unwrap();
        assert_eq!(count(&storage.connect().unwrap()), 0);
        seed(&storage.connect().unwrap());
        storage.anonymize_database_with_factor(2.0, true).unwrap();
        assert_eq!(count(&storage.connect().unwrap()), 0);
        seed(&storage.connect().unwrap());
        storage.anonymize_database(true).unwrap();
        assert_eq!(count(&storage.connect().unwrap()), 0);
    }

    #[test]
    fn backup_includes_committed_wal_data_from_selected_database() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("vault.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        storage.unlock("original-password".into(), true).unwrap();
        storage
            .create_database("selected", "selected-password".into())
            .unwrap();
        let selected = directory.path().join("selected.vault.sqlite3");
        let writer = open(&selected, "selected-password", false).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE wal_fixture(value TEXT); INSERT INTO wal_fixture VALUES('committed');").unwrap();
        let backup = directory.path().join("wal.saldonaut-backup");
        storage.create_backup_file(&backup).unwrap();
        let restored = open(&backup, "selected-password", false).unwrap();
        assert_eq!(
            restored
                .query_row("SELECT value FROM wal_fixture", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "committed"
        );
        assert!(open(&backup, "original-password", false).is_err());
        assert_eq!(storage.status().unwrap().database_id, "selected");
    }

    #[test]
    fn backup_roundtrip_preserves_encrypted_data_and_existing_database() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("vault.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        let backup = directory.path().join("test.saldonaut-backup");
        assert!(storage.create_backup_file(&backup).is_err());
        storage.unlock("test-password".into(), true).unwrap();
        storage.connect().unwrap().execute_batch("CREATE TABLE backup_fixture(value TEXT); INSERT INTO backup_fixture VALUES('private-data');").unwrap();
        storage.create_backup_file(&backup).unwrap();
        let bytes = fs::read(&backup).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(!bytes.windows(12).any(|value| value == b"private-data"));
        assert!(storage.create_backup_file(&backup).is_err());
        assert_eq!(bytes, fs::read(&backup).unwrap());
        assert!(storage
            .restore_backup_file(&backup, "wrong", "wrong-password".into())
            .is_err());
        assert!(!directory.path().join("wrong.vault.sqlite3").exists());
        assert!(storage
            .restore_backup_file(&backup, "../escape", "test-password".into())
            .is_err());
        storage
            .restore_backup_file(&backup, "restored", "test-password".into())
            .unwrap();
        assert!(storage
            .restore_backup_file(&backup, "restored", "test-password".into())
            .is_err());
        let restored = open(
            &directory.path().join("restored.vault.sqlite3"),
            "test-password",
            false,
        )
        .unwrap();
        assert_eq!(
            restored
                .query_row("SELECT value FROM backup_fixture", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "private-data"
        );
        assert_eq!(storage.status().unwrap().database_id, "original");
        storage
            .change_password("test-password".into(), "new-password".into())
            .unwrap();
        assert!(open(&backup, "test-password", false).is_ok());
        assert!(open(&backup, "new-password", false).is_err());
        let corrupt = directory.path().join("broken.saldonaut-backup");
        fs::write(&corrupt, b"not a database").unwrap();
        assert!(storage
            .restore_backup_file(&corrupt, "broken", "test-password".into())
            .is_err());
        assert!(!directory.path().join("broken.vault.sqlite3").exists());
    }

    #[test]
    fn restore_rejects_oversized_backups_before_copying_them() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("vault.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        let oversized = directory.path().join("oversized.saldonaut-backup");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_BACKUP_FILE_SIZE + 1)
            .unwrap();

        let error = storage
            .restore_backup_file(&oversized, "oversized", "test-password".into())
            .unwrap_err();

        assert!(error.contains("grösser als 2 GB"));
        assert!(!directory.path().join("oversized.vault.sqlite3").exists());
    }
}
