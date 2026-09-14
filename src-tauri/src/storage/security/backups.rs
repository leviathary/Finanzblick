use super::*;

impl Storage {
    fn create_backup_file(&self, destination: &Path) -> Result<(), String> {
        if destination.extension().and_then(|value| value.to_str()) != Some("finanzblick-backup") {
            return Err("Bitte eine Datei mit der Endung .finanzblick-backup wählen.".into());
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
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Backup konnte nicht gespeichert werden.")?;
        stage.persist_noclobber(destination).map_err(|_| {
            "Backup konnte nicht gespeichert werden. Bitte einen neuen Dateinamen wählen."
        })?;
        Ok(())
    }

    fn restore_backup_file(
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
        if !input
            .metadata()
            .map_err(|_| "Backup konnte nicht gelesen werden.")?
            .is_file()
        {
            return Err("Bitte eine Backup-Datei wählen.".into());
        }
        std::io::copy(&mut input, stage.as_file_mut())
            .map_err(|_| "Backup konnte nicht gelesen werden.")?;
        validate_backup(stage.path(), &password)?;
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
            .map_err(|_| "Ungültiges Finanzblick-Backup.")?;
        if !exists {
            return Err("Ungültiges Finanzblick-Backup.".into());
        }
    }
    read_settings(&db)?;
    Ok(())
}

#[tauri::command]
pub async fn create_backup(app: tauri::AppHandle, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>().create_backup_file(Path::new(&path))
    })
    .await
    .map_err(|_| "Backup konnte nicht erstellt werden.".to_string())?
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    path: String,
    name: String,
    password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>()
            .restore_backup_file(Path::new(&path), &name, password)
    })
    .await
    .map_err(|_| "Backup konnte nicht importiert werden.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let backup = directory.path().join("wal.finanzblick-backup");
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
        let backup = directory.path().join("test.finanzblick-backup");
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
        let corrupt = directory.path().join("broken.finanzblick-backup");
        fs::write(&corrupt, b"not a database").unwrap();
        assert!(storage
            .restore_backup_file(&corrupt, "broken", "test-password".into())
            .is_err());
        assert!(!directory.path().join("broken.vault.sqlite3").exists());
    }
}
