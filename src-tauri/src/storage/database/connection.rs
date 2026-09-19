//! Öffnet SQLCipher-Verbindungen mit Sitzungsschutz und verwaltet den Datenbankzugriff.
use super::handle::Storage;
use super::profiles as databases;
use super::schema::initialize_schema;
use super::session::Session;
use super::settings::read_settings;
use super::settings::validate_settings;
use super::settings::AppSettings;
use super::LOCKED;
use fs2::FileExt;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use serde::Serialize;
use std::fs;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::time::Duration;
use std::time::SystemTime;
use tauri::Manager;
use zeroize::Zeroizing;
// The read lease lives as long as the connection: locking waits for running
// operations to finish before erasing the credential and reporting success.
pub(crate) struct VaultConnection<'a> {
    connection: Connection,
    _lease: RwLockReadGuard<'a, Session>,
}
impl VaultConnection<'_> {
    pub(crate) fn chat_session(&self) -> (String, u64) {
        (self._lease.database_id.clone(), self._lease.generation)
    }
}
impl Deref for VaultConnection<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.connection
    }
}
impl DerefMut for VaultConnection<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    initialized: bool,
    unlocked: bool,
    data_path: String,
    settings: AppSettings,
    pub(super) database_id: String,
}

pub(super) fn open(path: &Path, password: &str, create: bool) -> rusqlite::Result<Connection> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let db = Connection::open_with_flags(path, flags)?;
    // Querying this pragma must succeed: ordinary SQLite must never silently
    // ignore the encryption settings and create a plaintext database.
    let _: String = db.query_row("PRAGMA cipher_version", [], |r| r.get(0))?;
    db.pragma_update(None, "key", password)?;
    // SQLCipher wipes its key allocations on close; keep its default allocator.
    // Enabling locking for every SQLite allocation exhausts Windows working-set quotas.
    db.execute_batch("PRAGMA temp_store = MEMORY; PRAGMA foreign_keys = ON;")?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    })?;
    Ok(db)
}

impl Storage {
    pub fn initialize(app: &tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = app.path().app_local_data_dir()?;
        fs::create_dir_all(&directory)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("vault.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|_| "Finanzblick läuft bereits. Bitte die andere Instanz schliessen.")?;
        let mut session = Session::default();
        // Only the display language is available before unlocking. No financial
        // information or password is written to this small local preference.
        if let Ok(language) = fs::read_to_string(directory.join("language")) {
            if ["de", "en", "fr", "it"].contains(&language.as_str()) {
                session.settings.language = language;
            }
        }
        let choice = directory.join("database-choice");
        if choice.exists() {
            let id = fs::read_to_string(choice)?;
            if !databases::valid_id(&id) {
                return Err("Ungültige Profilauswahl.".into());
            }
            if id != "original" && !directory.join(format!("{id}.vault.sqlite3")).exists() {
                return Err("Das gewählte Finanzprofil fehlt.".into());
            }
            session.database_id = id;
        }
        Ok(Self {
            path: directory.join("finanzblick.vault.sqlite3"),
            session: RwLock::new(session),
            _lock: Some(lock),
        })
    }

    pub(crate) fn connect(&self) -> rusqlite::Result<VaultConnection<'_>> {
        let lease = self
            .session
            .read()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let password = lease
            .password
            .as_ref()
            .filter(|_| !lease.expired())
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let connection = open(&self.database_path(&lease), password, false)?;
        Ok(VaultConnection {
            connection,
            _lease: lease,
        })
    }

    pub fn require_unlocked(&self) -> Result<impl Drop + '_, String> {
        let lease = self.session.read().map_err(|_| LOCKED.to_string())?;
        if lease.password.is_none() || lease.expired() {
            return Err(LOCKED.into());
        }
        Ok(lease)
    }

    pub(crate) fn chat_session(&self) -> Result<(String, u64), String> {
        let session = self.session.read().map_err(|_| LOCKED)?;
        if session.password.is_none() || session.expired() {
            return Err(LOCKED.into());
        }
        Ok((session.database_id.clone(), session.generation))
    }

    pub(crate) fn require_chat_session(
        &self,
        expected: &(String, u64),
    ) -> Result<impl Drop + '_, String> {
        let session = self.session.read().map_err(|_| LOCKED)?;
        if session.password.is_none()
            || session.expired()
            || (&session.database_id, session.generation) != (&expected.0, expected.1)
        {
            return Err(LOCKED.into());
        }
        Ok(session)
    }

    pub fn lock_session(&self) -> Result<bool, String> {
        let mut session = self.session.write().map_err(|_| LOCKED.to_string())?;
        let was_unlocked = session.password.is_some();
        session.clear();
        Ok(was_unlocked)
    }

    pub fn expire_session(&self) {
        if let Ok(mut session) = self.session.try_write() {
            if session.expired() {
                session.clear();
            }
        }
    }

    pub(crate) fn status(&self) -> Result<VaultStatus, String> {
        let session = self.session.read().map_err(|_| LOCKED.to_string())?;
        Ok(VaultStatus {
            initialized: self.database_path(&session).exists(),
            unlocked: session.password.is_some() && !session.expired(),
            data_path: self.database_path(&session).to_string_lossy().into_owned(),
            database_id: databases::session_id(&session).into(),
            settings: session.settings.clone(),
        })
    }

    pub(crate) fn unlock(&self, password: String, setup: bool) -> Result<(), String> {
        let password = Zeroizing::new(password);
        if password.len() > 1024 || password.is_empty() {
            return Err("Bitte ein Passwort mit höchstens 1024 Bytes eingeben.".into());
        }
        let mut session = self.session.write().map_err(|_| LOCKED.to_string())?;
        if session.retry_after.is_some_and(|t| t > SystemTime::now()) {
            return Err("Bitte einige Sekunden warten und erneut versuchen.".into());
        }
        if setup {
            if self.database_path(&session).exists()
                || databases::session_id(&session) != "original"
            {
                return Err("Der Schutz ist bereits eingerichtet.".into());
            }
            if password.chars().count() < 7 {
                return Err("Bitte mindestens 7 Zeichen verwenden.".into());
            }
            self.create_vault(&password).map_err(|_| "Das verschlüsselte Finanzprofil konnte nicht eingerichtet werden. Bestehende Quelldaten bleiben erhalten.".to_string())?;
        } else {
            let db = match open(&self.database_path(&session), &password, false) {
                Ok(db) => db,
                Err(_) => {
                    session.retry_after = Some(SystemTime::now() + Duration::from_secs(3));
                    return Err("Passwort falsch oder Finanzprofil nicht lesbar.".into());
                }
            };
            initialize_schema(&db)
                .map_err(|_| "Das Finanzprofil konnte nicht aktualisiert werden.".to_string())?;
        }
        let db = open(&self.database_path(&session), &password, false)
            .map_err(|_| "Finanzprofil nicht lesbar.")?;
        session.settings = read_settings(&db)?;
        session.generation = session.generation.wrapping_add(1);
        session.password = Some(password);
        session.activity = Some(SystemTime::now());
        session.retry_after = None;
        drop(db);
        Ok(())
    }

    pub(crate) fn save_settings(&self, settings: AppSettings) -> Result<(), String> {
        validate_settings(&settings)?;
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        let password = session
            .password
            .as_ref()
            .filter(|_| !session.expired())
            .ok_or(LOCKED)?;
        let db = open(&self.database_path(&session), password, false)
            .map_err(|_| "Finanzprofil nicht lesbar.")?;
        let json = serde_json::to_string(&settings)
            .map_err(|_| "Einstellungen konnten nicht gespeichert werden.")?;
        let transaction = db
            .unchecked_transaction()
            .map_err(|_| "Einstellungen konnten nicht gespeichert werden.")?;
        transaction.execute("INSERT INTO app_settings(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value", [json]).map_err(|_| "Einstellungen konnten nicht gespeichert werden.")?;
        let directory = self.path.parent().ok_or("Speicherort nicht verfügbar.")?;
        let mut hint = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| "Sprache konnte nicht gespeichert werden.")?;
        use std::io::Write;
        hint.write_all(settings.language.as_bytes())
            .map_err(|_| "Sprache konnte nicht gespeichert werden.")?;
        hint.as_file()
            .sync_all()
            .map_err(|_| "Sprache konnte nicht gespeichert werden.")?;
        hint.persist(directory.join("language"))
            .map_err(|_| "Sprache konnte nicht gespeichert werden.")?;
        transaction
            .commit()
            .map_err(|_| "Einstellungen konnten nicht gespeichert werden.")?;
        session.settings = settings;
        session.activity = Some(SystemTime::now());
        Ok(())
    }

    pub(crate) fn change_password(
        &self,
        current_password: String,
        new_password: String,
    ) -> Result<(), String> {
        let current = Zeroizing::new(current_password);
        let new = Zeroizing::new(new_password);
        if new.chars().count() < 7 || new.len() > 1024 {
            return Err("Bitte mindestens 7 Zeichen verwenden.".into());
        }
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        if session.password.is_none() || session.expired() {
            session.clear();
            return Err(LOCKED.into());
        }
        if session.retry_after.is_some_and(|t| t > SystemTime::now()) {
            return Err("Bitte einige Sekunden warten und erneut versuchen.".into());
        }
        let db = match open(&self.database_path(&session), &current, false) {
            Ok(db) => db,
            Err(_) => {
                session.retry_after = Some(SystemTime::now() + Duration::from_secs(3));
                return Err("Das bisherige Passwort ist nicht korrekt.".into());
            }
        };
        // No live connections remain under the write lock. SQLCipher rekeys
        // transactionally, using an encrypted rollback journal.
        db.pragma_update(None, "rekey", new.as_str())
            .map_err(|_| "Das Passwort konnte nicht geändert werden.".to_string())?;
        drop(db);
        session.password = Some(new);
        session.activity = Some(SystemTime::now());
        session.retry_after = None;
        if open(
            &self.database_path(&session),
            session.password.as_ref().unwrap(),
            false,
        )
        .is_err()
        {
            session.clear();
            return Err("Bitte mit dem neuen Passwort erneut entsperren.".into());
        }
        Ok(())
    }

    pub(super) fn create_vault(&self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        // A failed/interrupted setup never publishes a partially initialized vault.
        // The staging file is encrypted from the moment it is created.
        let stage =
            tempfile::NamedTempFile::new_in(self.path.parent().ok_or("Missing directory")?)?;
        {
            let db = open(stage.path(), password, true)?;
            initialize_schema(&db)?;
            let integrity: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            if integrity != "ok" {
                return Err("Integrity check failed".into());
            }
        }
        stage.as_file().sync_all()?;
        stage.persist_noclobber(&self.path)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn test_storage(path: std::path::PathBuf) -> Self {
        let password = "synthetic-test-password".to_string();
        drop(open(&path, &password, true).unwrap());
        Self {
            path,
            session: RwLock::new(Session {
                generation: 0,
                database_id: String::new(),
                password: Some(Zeroizing::new(password)),
                activity: Some(SystemTime::now()),
                retry_after: None,
                settings: AppSettings::default(),
            }),
            _lock: None,
        }
    }
}

pub(crate) fn touch_activity(storage: &Storage) -> Result<(), String> {
    let mut session = storage.session.write().map_err(|_| LOCKED.to_string())?;
    if session.password.is_none() || session.expired() {
        session.clear();
        return Err(LOCKED.into());
    }
    session.activity = Some(SystemTime::now());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn locked(directory: &Path) -> Storage {
        Storage {
            path: directory.join("test.vault.sqlite3"),
            session: RwLock::new(Session::default()),
            _lock: None,
        }
    }
    const PASSWORD: &str = "Test ' Wörter 123456789";

    #[test]
    pub(super) fn financial_chat_authorization_does_not_survive_lock_and_reunlock() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        vault.unlock(PASSWORD.into(), true).unwrap();
        let original = vault.chat_session().unwrap();
        assert!(vault.require_chat_session(&original).is_ok());
        vault.lock_session().unwrap();
        assert!(vault.chat_session().is_err());
        vault.unlock(PASSWORD.into(), false).unwrap();
        assert_ne!(vault.chat_session().unwrap(), original);
        assert!(vault.require_chat_session(&original).is_err());
    }

    #[test]
    pub(super) fn multiple_databases_have_independent_data_and_passwords() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        vault.unlock(PASSWORD.into(), true).unwrap();
        vault.connect().unwrap().execute_batch(
            "CREATE TABLE private_data(value TEXT); INSERT INTO private_data VALUES('primary');
             INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'fixture','Fixture','bank','2026-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Fixture','checking','CHF','2026-01-01');
             INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(1,1,'sample','CSV','hash','2026-01-01',1,'[]');
             INSERT INTO transactions(account_id,import_id,booking_date,description,amount_minor,balance_minor,currency,confidence,source_row)
               VALUES(1,1,'2026-01-01','Sensitive booking text',100000,500000,'CHF',1,1),
                     (1,1,'2026-01-02','Small sensitive amount',100,NULL,'CHF',1,2),
                     (1,1,'2026-02-01','Recurring sensitive payment',100000,NULL,'CHF',1,3),
                     (1,1,'2026-03-01','Recurring sensitive payment',100000,NULL,'CHF',1,4);
             INSERT INTO balance_snapshots(account_id,import_id,balance_date,amount_minor,currency)
               VALUES(1,1,'2026-01-01',500000,'CHF');
             INSERT INTO annual_tax_snapshots(
               tax_year,valuation_date,gross_assets_minor,liabilities_minor,taxable_wealth_minor,
               currency,source_name,source_hash,parser_version,extraction_confidence,imported_at)
               VALUES(2025,'2025-12-31',100000000,20000000,80000000,'CHF',
                 'Persönliche Steuererklärung.pdf','tax-hash','test',1,'2026-01-01');
             INSERT INTO annual_tax_snapshot_breakdowns(
               snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor)
               SELECT id,30000000,60000000,10000000 FROM annual_tax_snapshots WHERE tax_year=2025;",
        ).unwrap();
        vault.anonymize_database(true).unwrap();
        {
            let db = vault.connect().unwrap();
            let (description, amount, balance): (String, i64, i64) = db
                .query_row(
                    "SELECT description,amount_minor,balance_minor FROM transactions WHERE source_row=1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            assert!(description.starts_with("Anonymisierte Buchung "));
            assert_ne!(description, "Sensitive booking text");
            assert!((10000..=300000).contains(&amount));
            assert_ne!(amount, 100000);
            assert!((50000..=1500000).contains(&balance));
            assert_ne!(balance, 500000);
            let snapshot: i64 = db
                .query_row("SELECT amount_minor FROM balance_snapshots", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(snapshot, balance);
            let small_amount: i64 = db
                .query_row(
                    "SELECT amount_minor FROM transactions WHERE source_row=2",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!((10..=300).contains(&small_amount));
            assert_ne!(small_amount, 100);
            let distinct_recurring_amounts: i64 = db
                .query_row(
                    "SELECT count(DISTINCT amount_minor) FROM transactions WHERE source_row IN (1,3,4)",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(distinct_recurring_amounts > 1);
            let tax_values: (i64, i64, i64, String) = db
                .query_row(
                    "SELECT gross_assets_minor,liabilities_minor,taxable_wealth_minor,source_name
                     FROM annual_tax_snapshots",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
            assert_eq!(tax_values.0 - tax_values.1, tax_values.2);
            assert_ne!(tax_values.0, 100000000);
            assert_eq!(tax_values.3, "Anonymisierte Steuererklärung 2025.pdf");
            let breakdown: (i64, i64, i64) = db
                .query_row(
                    "SELECT securities_and_cash_minor,real_estate_minor,other_assets_minor
                     FROM annual_tax_snapshot_breakdowns",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            assert_eq!(breakdown.0 + breakdown.1 + breakdown.2, tax_values.0);
        }
        let original = fs::read(&vault.path).unwrap();
        vault
            .create_database("Person A", "other-password".into())
            .unwrap();
        assert_eq!(fs::read(&vault.path).unwrap(), original);
        let status = vault.status().unwrap();
        assert_eq!(status.database_id, "Person A");
        assert_eq!(
            fs::read_to_string(dir.path().join("database-choice")).unwrap(),
            status.database_id
        );
        assert!(!fs::read(&status.data_path)
            .unwrap()
            .starts_with(b"SQLite format 3"));
        assert!(open(&vault.path, PASSWORD, false).is_ok());
        assert!(open(Path::new(&status.data_path), PASSWORD, false).is_err());
        assert!(open(Path::new(&status.data_path), "other-password", false).is_ok());
        assert!(status.unlocked);
        assert!(vault
            .connect()
            .unwrap()
            .prepare("SELECT * FROM private_data")
            .is_err());
        vault.select_database("original").unwrap();
        assert!(!vault.status().unwrap().unlocked);
        vault.unlock(PASSWORD.into(), false).unwrap();
        assert_eq!(vault.status().unwrap().database_id, "original");
        assert_eq!(
            vault
                .connect()
                .unwrap()
                .query_row("SELECT value FROM private_data", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "primary"
        );
        assert!(vault.select_database("../other").is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("database-choice")).unwrap(),
            "original"
        );
        assert!(vault
            .create_database("Person A", "another-password".into())
            .is_err());
        vault.copy_database("Primary copy").unwrap();
        let copied_path = vault.status().unwrap().data_path;
        assert!(open(Path::new(&copied_path), PASSWORD, false).is_ok());
        assert_eq!(
            vault
                .connect()
                .unwrap()
                .query_row("SELECT value FROM private_data", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "primary"
        );
        vault.delete_database().unwrap();
        assert!(!Path::new(&copied_path).exists());
        vault.select_database("Person A").unwrap();
        vault.unlock("other-password".into(), false).unwrap();
        let removed_path = vault.status().unwrap().data_path;
        vault.delete_database().unwrap();
        assert!(!Path::new(&removed_path).exists());
        assert_eq!(vault.status().unwrap().database_id, "original");
        assert!(!vault.status().unwrap().unlocked);
    }

    #[test]
    pub(super) fn fixed_factor_anonymization_preserves_amount_ratios_and_balances() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        vault.unlock(PASSWORD.into(), true).unwrap();
        vault.connect().unwrap().execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'fixture','Fixture','bank','2026-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Fixture','checking','CHF','2026-01-01');
             INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(1,1,'sample','CSV','factor-hash','2026-01-01',2,'[]');
             INSERT INTO transactions(account_id,import_id,booking_date,description,amount_minor,balance_minor,currency,confidence,source_row)
               VALUES(1,1,'2026-01-01','Private income',1000000,2000000,'CHF',1,1),
                     (1,1,'2026-01-02','Private expense',-250000,NULL,'CHF',1,2);
             INSERT INTO balance_snapshots(account_id,import_id,balance_date,amount_minor,currency)
               VALUES(1,1,'2026-01-01',2000000,'CHF');
             INSERT INTO annual_tax_snapshots(
               tax_year,valuation_date,gross_assets_minor,liabilities_minor,taxable_wealth_minor,
               currency,source_name,source_hash,parser_version,extraction_confidence,imported_at)
               VALUES(2025,'2025-12-31',100000000,20000000,80000000,'CHF',
                 'Private tax.pdf','factor-tax-hash','test',1,'2026-01-01');
             INSERT INTO annual_tax_snapshot_breakdowns(
               snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor)
               SELECT id,30000000,60000000,10000000 FROM annual_tax_snapshots WHERE tax_year=2025;",
        ).unwrap();

        assert!(vault.anonymize_database_with_factor(1.0, false).is_err());
        assert!(vault.anonymize_database_with_factor(0.0, false).is_err());
        vault.anonymize_database_with_factor(0.5, false).unwrap();

        let db = vault.connect().unwrap();
        let values: (i64, i64, i64, String) = db
            .query_row(
                "SELECT
                   (SELECT amount_minor FROM transactions WHERE source_row=1),
                   (SELECT amount_minor FROM transactions WHERE source_row=2),
                   (SELECT balance_minor FROM transactions WHERE source_row=1),
                   (SELECT description FROM transactions WHERE source_row=1)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(values.0, 500000);
        assert_eq!(values.1, -125000);
        assert_eq!(values.2, 1000000);
        assert_eq!(values.3, "Private income");
        assert_eq!(
            db.query_row("SELECT amount_minor FROM balance_snapshots", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            values.2
        );
        let tax_values: (i64, i64, i64, String) = db
            .query_row(
                "SELECT gross_assets_minor,liabilities_minor,taxable_wealth_minor,source_name
                 FROM annual_tax_snapshots",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(tax_values.0, 50000000);
        assert_eq!(tax_values.1, 10000000);
        assert_eq!(tax_values.2, 40000000);
        assert_eq!(tax_values.3, "Private tax.pdf");
        let breakdown: (i64, i64, i64) = db
            .query_row(
                "SELECT securities_and_cash_minor,real_estate_minor,other_assets_minor
                 FROM annual_tax_snapshot_breakdowns",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(breakdown, (15000000, 30000000, 5000000));
    }

    #[test]
    pub(super) fn settings_persist_and_control_backend_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        let settings = AppSettings {
            auto_lock_minutes: 1,
            language: "fr".into(),
            region: "FR".into(),
            default_currency: "EUR".into(),
            marketstack_api_key: "marketstack-key".into(),
            alpha_vantage_api_key: "demo-key".into(),
        };
        assert!(vault.save_settings(settings.clone()).is_err());
        vault.unlock(PASSWORD.into(), true).unwrap();
        let mut invalid = settings.clone();
        invalid.auto_lock_minutes = 0;
        assert!(vault.save_settings(invalid).is_err());
        vault.save_settings(settings.clone()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("language")).unwrap(),
            "fr"
        );
        vault.session.write().unwrap().clear();
        vault.unlock(PASSWORD.into(), false).unwrap();
        assert_eq!(vault.session.read().unwrap().settings, settings);
        vault.session.write().unwrap().activity = Some(SystemTime::now() - Duration::from_secs(61));
        assert!(vault.connect().is_err());
        assert!(vault.save_settings(AppSettings::default()).is_err());
        vault.expire_session();
        assert!(vault.session.read().unwrap().password.is_none());
    }

    #[test]
    pub(super) fn password_change_requires_current_password_and_preserves_data() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        assert!(vault
            .change_password(PASSWORD.into(), "1234567".into())
            .is_err());
        vault.unlock(PASSWORD.into(), true).unwrap();
        vault
            .connect()
            .unwrap()
            .execute_batch(
                "CREATE TABLE secret(value TEXT); INSERT INTO secret VALUES('rekey fixture');",
            )
            .unwrap();
        assert!(vault
            .change_password(PASSWORD.into(), "123456".into())
            .is_err());
        assert!(vault
            .change_password("wrong password".into(), "1234567".into())
            .is_err());
        assert!(vault
            .change_password(PASSWORD.into(), "1234567".into())
            .is_err()); // throttle
        vault.session.write().unwrap().retry_after = None;
        vault
            .change_password(PASSWORD.into(), "1234567".into())
            .unwrap();
        assert!(open(&vault.path, PASSWORD, false).is_err());
        assert_eq!(
            vault
                .connect()
                .unwrap()
                .query_row("SELECT value FROM secret", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "rekey fixture"
        );
        vault.session.write().unwrap().clear();
        vault.unlock("1234567".into(), false).unwrap();
        assert!(!fs::read(&vault.path)
            .unwrap()
            .starts_with(b"SQLite format 3"));
    }

    #[test]
    pub(super) fn encrypted_at_rest_restart_wrong_password_and_manual_lock() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        assert!(vault.connect().is_err());
        assert!(vault.require_unlocked().is_err());
        vault.unlock(PASSWORD.into(), true).unwrap();
        vault.connect().unwrap().execute_batch("CREATE TABLE secret(value TEXT); INSERT INTO secret VALUES ('PRIVATE_TRANSACTION_123');").unwrap();
        let bytes = fs::read(&vault.path).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(!bytes
            .windows(b"PRIVATE_TRANSACTION_123".len())
            .any(|w| w == b"PRIVATE_TRANSACTION_123"));
        let plaintext = Connection::open(&vault.path).unwrap();
        assert!(plaintext
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| r
                .get::<_, i64>(0))
            .is_err());
        drop(plaintext);
        drop(vault);
        let vault = locked(dir.path());
        assert!(!vault.status().unwrap().unlocked);
        assert!(vault.unlock("wrong-password".into(), false).is_err());
        assert!(vault.connect().is_err());
        assert!(vault
            .unlock(PASSWORD.into(), false)
            .unwrap_err()
            .contains("warten"));
        vault.session.write().unwrap().retry_after = None;
        vault.unlock(PASSWORD.into(), false).unwrap();
        assert_eq!(
            vault
                .connect()
                .unwrap()
                .query_row("SELECT value FROM secret", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "PRIVATE_TRANSACTION_123"
        );
        assert!(vault.lock_session().unwrap());
        assert!(vault.connect().is_err());
        assert!(vault.require_unlocked().is_err());
        assert!(!vault.status().unwrap().unlocked);
        assert!(vault.session.read().unwrap().password.is_none());
        assert!(!vault.lock_session().unwrap());
    }

    #[test]
    pub(super) fn inactivity_expires_in_backend_without_frontend_and_cannot_be_revived() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        vault.unlock(PASSWORD.into(), true).unwrap();
        vault.session.write().unwrap().activity =
            Some(SystemTime::now() - Duration::from_secs(15 * 60 + 1));
        assert!(vault.connect().is_err());
        assert!(vault.require_unlocked().is_err());
        assert!(!vault.status().unwrap().unlocked);
        vault.expire_session();
        assert!(vault.session.read().unwrap().password.is_none());
    }

    #[test]
    pub(super) fn setup_accepts_seven_characters_and_never_overwrites_existing_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = locked(dir.path());
        assert!(vault.unlock("123456".into(), true).is_err());
        assert!(!vault.path.exists());
        vault.unlock("1234567".into(), true).unwrap();
        let before = fs::read(&vault.path).unwrap();
        assert!(vault.unlock("another long password".into(), true).is_err());
        assert_eq!(fs::read(&vault.path).unwrap(), before);
    }
}
