//! Erstellt eine verschlüsselte, anonymisierte Profilkopie im temporären Staging; Quelle und aktive Auswahl bleiben unverändert.
use super::{connection::open, session::Session, Storage, LOCKED};
use std::{sync::RwLock, time::SystemTime};
use zeroize::Zeroizing;

impl Storage {
    pub(crate) fn create_anonymized_copy(
        &self,
        name: &str,
        password: String,
        factor: Option<f64>,
        anonymize_descriptions: bool,
    ) -> Result<(), String> {
        let password = Zeroizing::new(password);
        if password.chars().count() < 7 {
            return Err("Bitte mindestens 7 Zeichen verwenden.".into());
        }
        if factor.is_some_and(|f| {
            !f.is_finite() || !(0.01..=100.0).contains(&f) || (f - 1.0).abs() < 1e-9
        }) {
            return Err(
                "Bitte einen Faktor zwischen 0,01 und 100 eingeben, der nicht 1 ist.".into(),
            );
        }
        let destination = self.available_destination(name)?;
        // Keep the source profile locked against switching for the complete operation.
        let session = self.session.write().map_err(|_| LOCKED)?;
        let source_password = session
            .password
            .as_ref()
            .filter(|_| !session.expired())
            .ok_or(LOCKED)?;
        let result = (|| -> Result<(), String> {
            let stage =
                tempfile::NamedTempFile::new_in(self.path.parent().ok_or("Speicherort fehlt.")?)
                    .map_err(|e| e.to_string())?;
            {
                let source = open(&self.database_path(&session), source_password, false)
                    .map_err(|e| e.to_string())?;
                source
                    .execute(
                        "ATTACH DATABASE ?1 AS anonymous_copy KEY ?2",
                        rusqlite::params![stage.path().to_string_lossy(), password.as_str()],
                    )
                    .map_err(|e| e.to_string())?;
                source
                    .query_row("SELECT sqlcipher_export('anonymous_copy')", [], |_| Ok(()))
                    .map_err(|e| e.to_string())?;
                source
                    .execute_batch("DETACH DATABASE anonymous_copy;")
                    .map_err(|e| e.to_string())?;
            }
            {
                let copy = Storage {
                    path: stage.path().to_path_buf(),
                    session: RwLock::new(Session {
                        password: Some(password),
                        activity: Some(SystemTime::now()),
                        ..Session::default()
                    }),
                    _lock: None,
                };
                match factor {
                    Some(factor) => {
                        copy.anonymize_database_with_factor(factor, anonymize_descriptions)?
                    }
                    None => copy.anonymize_database(anonymize_descriptions)?,
                }
                let db = copy.connect().map_err(|e| e.to_string())?;
                crate::storage::assistant::credentials::clear_config(&db)
                    .map_err(|e| e.to_string())?;
                let integrity: String = db
                    .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                    .map_err(|e| e.to_string())?;
                if integrity != "ok"
                    || db
                        .prepare("PRAGMA foreign_key_check")
                        .and_then(|mut q| q.exists([]))
                        .unwrap_or(true)
                {
                    return Err("Profilkopie konnte nicht geprüft werden.".into());
                }
            }
            stage.as_file().sync_all().map_err(|e| e.to_string())?;
            stage
                .persist_noclobber(destination)
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        result.map_err(|_| "Anonymisierte Kopie konnte nicht erstellt werden.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copy_keeps_source_and_selection_and_uses_new_password() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: dir.path().join("source.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        };
        storage.unlock("source-password".into(), true).unwrap();
        storage.connect().unwrap().execute("INSERT INTO annual_tax_snapshots(tax_year,valuation_date,gross_assets_minor,liabilities_minor,taxable_wealth_minor,currency,source_name,source_hash,parser_version,extraction_confidence,imported_at) VALUES(2025,'2025-12-31',100000,20000,80000,'CHF','private.pdf','test','test',1.0,'2026-01-01')", []).unwrap();
        let original = std::fs::read(&storage.path).unwrap();
        for (name, factor) in [("scaled", Some(0.5)), ("random", None)] {
            storage
                .create_anonymized_copy(name, "copy-password".into(), factor, true)
                .unwrap();
            let path = dir.path().join(format!("{name}.vault.sqlite3"));
            assert!(open(&path, "copy-password", false).is_ok());
            assert!(open(&path, "source-password", false).is_err());
            let db = open(&path, "copy-password", false).unwrap();
            let (amount, description): (i64, String) = db
                .query_row(
                    "SELECT gross_assets_minor,source_name FROM annual_tax_snapshots",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            if factor.is_some() {
                assert_eq!(amount, 50000);
            } else {
                assert_ne!(amount, 100000);
            }
            assert_ne!(description, "private.pdf");
            assert_eq!(std::fs::read(&storage.path).unwrap(), original);
            assert_eq!(
                super::super::profiles::session_id(&storage.session.read().unwrap()),
                "original"
            );
        }
        let copy = std::fs::read(dir.path().join("scaled.vault.sqlite3")).unwrap();
        assert!(storage
            .create_anonymized_copy("scaled", "different-password".into(), None, true)
            .is_err());
        assert_eq!(
            std::fs::read(dir.path().join("scaled.vault.sqlite3")).unwrap(),
            copy
        );
        assert!(storage
            .create_anonymized_copy("invalid", "copy-password".into(), Some(1.0), true)
            .is_err());
        assert!(storage
            .create_anonymized_copy("invalid", "short".into(), None, true)
            .is_err());
        assert!(!dir.path().join("invalid.vault.sqlite3").exists());
        storage
            .create_anonymized_copy("keep-text", "copy-password".into(), Some(0.5), false)
            .unwrap();
        let retained: String = open(
            &dir.path().join("keep-text.vault.sqlite3"),
            "copy-password",
            false,
        )
        .unwrap()
        .query_row("SELECT source_name FROM annual_tax_snapshots", [], |r| {
            r.get(0)
        })
        .unwrap();
        assert_eq!(retained, "private.pdf");
        // Failure after export must not publish an unmodified or partially processed copy.
        storage.connect().unwrap().execute_batch("CREATE TRIGGER fail_anonymization BEFORE UPDATE ON annual_tax_snapshots BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
        let before_failure = std::fs::read(&storage.path).unwrap();
        assert!(storage
            .create_anonymized_copy("failed", "copy-password".into(), Some(0.5), true)
            .is_err());
        assert!(!dir.path().join("failed.vault.sqlite3").exists());
        assert_eq!(std::fs::read(&storage.path).unwrap(), before_failure);
        assert!(!dir.path().join("database-choice").exists());
        storage.lock_session().unwrap();
        assert!(storage
            .create_anonymized_copy("locked", "copy-password".into(), None, true)
            .is_err());
        assert!(!dir.path().join("locked.vault.sqlite3").exists());
    }
}
