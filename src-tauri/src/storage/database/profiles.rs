//! Verwaltet lokale Datenbankprofile einschließlich Auswahl, Kopien und Anonymisierung.
use super::demo;

use super::connection::open;
use super::handle::Storage;
use super::schema::initialize_schema;
use super::session::Session;
use super::settings::AppSettings;
use super::LOCKED;
use serde::Serialize;
use std::fs;
use std::time::SystemTime;
use std::{io::Write, path::PathBuf};
use zeroize::Zeroizing;

const PRIMARY_DATABASE_ID: &str = "original";
const DATABASE_SUFFIX: &str = ".vault.sqlite3";

pub(super) fn valid_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && id.len() <= 80
        && id != "."
        && id != ".."
        && !id.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
}

pub(super) fn session_id(session: &Session) -> &str {
    if session.database_id.is_empty() {
        PRIMARY_DATABASE_ID
    } else {
        &session.database_id
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseChoice {
    id: String,
    name: String,
    active: bool,
    demo: bool,
}

impl Storage {
    fn path_for_database(&self, id: &str) -> PathBuf {
        if id == PRIMARY_DATABASE_ID {
            self.path.clone()
        } else {
            self.path.with_file_name(format!("{id}{DATABASE_SUFFIX}"))
        }
    }

    pub(crate) fn select_database(&self, id: &str) -> Result<(), String> {
        if !valid_id(id) {
            return Err("Ungültige Profilauswahl.".into());
        }
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        let path = self.path_for_database(id);
        if !path.is_file() {
            return Err("Das gewählte Finanzprofil fehlt.".into());
        }
        self.persist_database_choice(id)?;
        session.clear();
        session.database_id = id.to_string();
        Ok(())
    }

    pub(super) fn database_path(&self, session: &Session) -> PathBuf {
        self.path_for_database(session_id(session))
    }

    pub(super) fn persist_database_choice(&self, id: &str) -> Result<(), String> {
        let mut stage =
            tempfile::NamedTempFile::new_in(self.path.parent().ok_or("Speicherort fehlt.")?)
                .map_err(|_| "Profilauswahl konnte nicht gespeichert werden.")?;
        stage
            .write_all(id.as_bytes())
            .and_then(|_| stage.as_file().sync_all())
            .map_err(|_| "Profilauswahl konnte nicht gespeichert werden.")?;
        stage
            .persist(self.path.with_file_name("database-choice"))
            .map_err(|_| "Profilauswahl konnte nicht gespeichert werden.")?;
        Ok(())
    }

    pub(super) fn available_destination(&self, name: &str) -> Result<PathBuf, String> {
        let name = name.trim();
        if !valid_id(name) || name == PRIMARY_DATABASE_ID {
            return Err("Bitte einen gültigen, eindeutigen Profilnamen eingeben.".into());
        }
        let destination = self.path_for_database(name);
        if destination.exists() {
            return Err("Ein Finanzprofil mit diesem Namen existiert bereits.".into());
        }
        Ok(destination)
    }

    pub(crate) fn create_database(&self, name: &str, password: String) -> Result<(), String> {
        let name = name.trim();
        if password.chars().count() < 7 {
            return Err("Bitte mindestens 7 Zeichen verwenden.".into());
        }
        let destination = self.available_destination(name)?;

        let directory = self.path.parent().ok_or("Speicherort fehlt.")?;
        let stage = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| "Finanzprofil konnte nicht erstellt werden.")?;
        {
            let db = open(stage.path(), &password, true)
                .map_err(|_| "Finanzprofil konnte nicht erstellt werden.")?;
            initialize_schema(&db).map_err(|_| "Finanzprofil konnte nicht eingerichtet werden.")?;
            let integrity: String = db
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .map_err(|_| "Finanzprofil konnte nicht geprüft werden.")?;
            if integrity != "ok" {
                return Err("Finanzprofil konnte nicht geprüft werden.".into());
            }
        }
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Finanzprofil konnte nicht gespeichert werden.")?;
        stage
            .persist_noclobber(&destination)
            .map_err(|_| "Finanzprofil konnte nicht gespeichert werden.")?;

        self.persist_database_choice(name)?;
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        session.clear();
        session.database_id = name.to_string();
        session.password = Some(Zeroizing::new(password));
        session.activity = Some(SystemTime::now());
        session.settings = AppSettings::default();
        Ok(())
    }

    pub(crate) fn copy_database(&self, name: &str) -> Result<(), String> {
        let name = name.trim();
        let destination = self.available_destination(name)?;
        let mut session = self.session.write().map_err(|_| LOCKED)?;
        let password = session
            .password
            .as_ref()
            .filter(|_| !session.expired())
            .ok_or(LOCKED)?;
        let directory = self.path.parent().ok_or("Speicherort fehlt.")?;
        let stage = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
        {
            let source = open(&self.database_path(&session), password, false)
                .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
            source
                .execute(
                    "ATTACH DATABASE ?1 AS copied_database KEY ?2",
                    rusqlite::params![stage.path().to_string_lossy(), password.as_str()],
                )
                .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
            source
                .query_row("SELECT sqlcipher_export('copied_database')", [], |_| Ok(()))
                .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
            source
                .execute_batch("DETACH DATABASE copied_database;")
                .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
        }
        {
            let copy = open(stage.path(), password, false)
                .map_err(|_| "Profilkopie konnte nicht geprüft werden.")?;
            crate::storage::assistant::credentials::clear_config(&copy)
                .map_err(|_| "Profilkopie konnte nicht geprüft werden.")?;
            let integrity: String = copy
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .map_err(|_| "Profilkopie konnte nicht geprüft werden.")?;
            if integrity != "ok"
                || copy
                    .prepare("PRAGMA foreign_key_check")
                    .and_then(|mut query| query.exists([]))
                    .unwrap_or(true)
            {
                return Err("Profilkopie konnte nicht geprüft werden.".into());
            }
        }
        stage
            .as_file()
            .sync_all()
            .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
        stage
            .persist_noclobber(destination)
            .map_err(|_| "Finanzprofil konnte nicht kopiert werden.")?;
        self.persist_database_choice(name)?;
        session.database_id = name.to_string();
        session.activity = Some(SystemTime::now());
        Ok(())
    }

    pub(super) fn anonymize_database(&self, anonymize_descriptions: bool) -> Result<(), String> {
        let db = self.connect().map_err(|_| LOCKED)?;
        let anonymize_descriptions = if anonymize_descriptions { 1 } else { 0 };
        db.execute_batch(&format!(
            "PRAGMA secure_delete=ON;
             BEGIN IMMEDIATE;
             DELETE FROM finance_chat_settings;
             CREATE TEMP TABLE anonymized_transaction_amounts(
               transaction_id INTEGER PRIMARY KEY,
               original INTEGER NOT NULL,
               changed INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO anonymized_transaction_amounts(transaction_id, original)
               SELECT id, abs(amount_minor) FROM transactions;
             UPDATE anonymized_transaction_amounts
               SET changed=CAST(round(original * (100000 + (random() & 2147483647) % 2900001) / 1000000.0) AS INTEGER);
             UPDATE anonymized_transaction_amounts SET changed=1 WHERE original>0 AND changed=0;
             UPDATE anonymized_transaction_amounts SET changed=original+1 WHERE original>=2 AND changed=original;
             CREATE TEMP TABLE anonymized_balances(
               currency TEXT NOT NULL,
               original INTEGER NOT NULL,
               changed INTEGER NOT NULL DEFAULT 0,
               PRIMARY KEY(currency, original)
             );
             INSERT INTO anonymized_balances(currency, original)
               SELECT currency, abs(balance_minor) FROM transactions WHERE balance_minor IS NOT NULL
               UNION SELECT currency, abs(amount_minor) FROM balance_snapshots;
             UPDATE anonymized_balances
               SET changed=CAST(round(original * (100000 + (random() & 2147483647) % 2900001) / 1000000.0) AS INTEGER);
             UPDATE anonymized_balances SET changed=1 WHERE original>0 AND changed=0;
             UPDATE anonymized_balances SET changed=original+1 WHERE original>=2 AND changed=original;
             UPDATE transactions SET
               description=CASE WHEN {anonymize_descriptions}=1
                 THEN 'Anonymisierte Buchung ' || lower(hex(randomblob(8))) ELSE description END,
               amount_minor=(CASE WHEN amount_minor<0 THEN -1 ELSE 1 END) *
                 (SELECT changed FROM anonymized_transaction_amounts m WHERE m.transaction_id=transactions.id),
               balance_minor=CASE WHEN balance_minor IS NULL THEN NULL ELSE
                 (CASE WHEN balance_minor<0 THEN -1 ELSE 1 END) *
                 (SELECT changed FROM anonymized_balances m WHERE m.currency=transactions.currency AND m.original=abs(transactions.balance_minor)) END;
             UPDATE transaction_metadata SET
               counterparty_name=CASE WHEN {anonymize_descriptions}=1 THEN NULL ELSE counterparty_name END,
               remittance_information=CASE WHEN {anonymize_descriptions}=1 THEN NULL ELSE remittance_information END,
               reference_namespace=CASE WHEN {anonymize_descriptions}=1 THEN NULL ELSE reference_namespace END,
               external_reference=CASE WHEN {anonymize_descriptions}=1 THEN NULL ELSE external_reference END,
               fallback_fingerprint=CASE WHEN {anonymize_descriptions}=1
                 THEN lower(hex(randomblob(32))) ELSE fallback_fingerprint END;
             UPDATE import_document_metadata SET
               account_reference=CASE WHEN {anonymize_descriptions}=1 THEN NULL ELSE account_reference END;
             UPDATE security_transactions SET
               quantity=NULL, price=NULL, exchange_rate=NULL,
               gross_amount_minor=CASE WHEN gross_amount_minor IS NULL THEN NULL ELSE CAST(round(gross_amount_minor *
                 COALESCE((SELECT changed * 1.0 / NULLIF(original,0) FROM anonymized_transaction_amounts m WHERE m.transaction_id=security_transactions.transaction_id),1)) AS INTEGER) END,
               fees_minor=CASE WHEN fees_minor IS NULL THEN NULL ELSE CAST(round(fees_minor *
                 COALESCE((SELECT changed * 1.0 / NULLIF(original,0) FROM anonymized_transaction_amounts m WHERE m.transaction_id=security_transactions.transaction_id),1)) AS INTEGER) END,
               taxes_minor=CASE WHEN taxes_minor IS NULL THEN NULL ELSE CAST(round(taxes_minor *
                 COALESCE((SELECT changed * 1.0 / NULLIF(original,0) FROM anonymized_transaction_amounts m WHERE m.transaction_id=security_transactions.transaction_id),1)) AS INTEGER) END,
               withholding_tax_minor=CASE WHEN withholding_tax_minor IS NULL THEN NULL ELSE CAST(round(withholding_tax_minor *
                 COALESCE((SELECT changed * 1.0 / NULLIF(original,0) FROM anonymized_transaction_amounts m WHERE m.transaction_id=security_transactions.transaction_id),1)) AS INTEGER) END,
               accrued_interest_minor=CASE WHEN accrued_interest_minor IS NULL THEN NULL ELSE CAST(round(accrued_interest_minor *
                 COALESCE((SELECT changed * 1.0 / NULLIF(original,0) FROM anonymized_transaction_amounts m WHERE m.transaction_id=security_transactions.transaction_id),1)) AS INTEGER) END;
             UPDATE balance_snapshots SET
               amount_minor=(CASE WHEN amount_minor<0 THEN -1 ELSE 1 END) *
                 (SELECT changed FROM anonymized_balances m WHERE m.currency=balance_snapshots.currency AND m.original=abs(balance_snapshots.amount_minor));
             CREATE TEMP TABLE anonymized_tax_factors(
               snapshot_id INTEGER PRIMARY KEY,
               factor_ppm INTEGER NOT NULL
             );
             INSERT INTO anonymized_tax_factors(snapshot_id, factor_ppm)
               SELECT id, (random() & 2147483647) % 2900000
               FROM annual_tax_snapshots;
             UPDATE anonymized_tax_factors SET factor_ppm=CASE
               WHEN factor_ppm<900000 THEN factor_ppm+100000
               ELSE factor_ppm+100001 END;
             UPDATE annual_tax_snapshots SET
               gross_assets_minor=CAST(round(gross_assets_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER),
               liabilities_minor=CAST(round(liabilities_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER),
               taxable_wealth_minor=CAST(round(gross_assets_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER)
                 - CAST(round(liabilities_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER),
               canton_taxable_wealth_minor=CASE WHEN canton_taxable_wealth_minor IS NULL THEN NULL ELSE
                 min(CAST(round(canton_taxable_wealth_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER),
                 CAST(round(gross_assets_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER)
                 - CAST(round(liabilities_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshots.id)) AS INTEGER)) END,
               source_name=CASE WHEN {anonymize_descriptions}=1
                 THEN 'Anonymisierte Steuererklärung ' || tax_year || '.pdf' ELSE source_name END;
             UPDATE annual_tax_snapshot_breakdowns SET
               securities_and_cash_minor=CAST(round(securities_and_cash_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshot_breakdowns.snapshot_id)) AS INTEGER),
               real_estate_minor=CAST(round(real_estate_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshot_breakdowns.snapshot_id)) AS INTEGER),
               other_assets_minor=(SELECT gross_assets_minor FROM annual_tax_snapshots s
                 WHERE s.id=annual_tax_snapshot_breakdowns.snapshot_id)
                 - CAST(round(securities_and_cash_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshot_breakdowns.snapshot_id)) AS INTEGER)
                 - CAST(round(real_estate_minor *
                 (SELECT factor_ppm / 1000000.0 FROM anonymized_tax_factors f WHERE f.snapshot_id=annual_tax_snapshot_breakdowns.snapshot_id)) AS INTEGER);
             DELETE FROM merchant_category_rules;
             DELETE FROM settlement_rules;
             DROP TABLE anonymized_transaction_amounts;
             DROP TABLE anonymized_balances;
             DROP TABLE anonymized_tax_factors;
             COMMIT;
             VACUUM;"
        ))
        .map_err(|_| "Finanzprofil konnte nicht anonymisiert werden.")?;
        Ok(())
    }

    pub(super) fn anonymize_database_with_factor(
        &self,
        factor: f64,
        anonymize_descriptions: bool,
    ) -> Result<(), String> {
        if !factor.is_finite() || !(0.01..=100.0).contains(&factor) || (factor - 1.0).abs() < 1e-9 {
            return Err(
                "Bitte einen Faktor zwischen 0,01 und 100 eingeben, der nicht 1 ist.".into(),
            );
        }
        let db = self.connect().map_err(|_| LOCKED)?;
        db.execute_batch("PRAGMA secure_delete=ON; BEGIN IMMEDIATE;")
            .map_err(|_| "Finanzprofil konnte nicht anonymisiert werden.")?;
        let result = (|| -> rusqlite::Result<()> {
            crate::storage::assistant::credentials::clear_config(&db)?;
            db.execute(
                "UPDATE transactions SET
                   description=CASE WHEN ?2
                     THEN 'Anonymisierte Buchung ' || lower(hex(randomblob(8))) ELSE description END,
                   amount_minor=CASE WHEN amount_minor=0 THEN 0 ELSE
                     (CASE WHEN amount_minor<0 THEN -1 ELSE 1 END) *
                     max(1,CAST(round(abs(amount_minor) * ?1) AS INTEGER)) END,
                   balance_minor=CASE WHEN balance_minor IS NULL THEN NULL WHEN balance_minor=0 THEN 0 ELSE
                     (CASE WHEN balance_minor<0 THEN -1 ELSE 1 END) *
                     max(1,CAST(round(abs(balance_minor) * ?1) AS INTEGER)) END",
                rusqlite::params![factor, anonymize_descriptions],
            )?;
            db.execute(
                "UPDATE transaction_metadata SET
                   counterparty_name=CASE WHEN ?1 THEN NULL ELSE counterparty_name END,
                   remittance_information=CASE WHEN ?1 THEN NULL ELSE remittance_information END,
                   reference_namespace=CASE WHEN ?1 THEN NULL ELSE reference_namespace END,
                   external_reference=CASE WHEN ?1 THEN NULL ELSE external_reference END,
                   fallback_fingerprint=CASE WHEN ?1 THEN lower(hex(randomblob(32))) ELSE fallback_fingerprint END",
                rusqlite::params![anonymize_descriptions],
            )?;
            db.execute(
                "UPDATE import_document_metadata SET
                   account_reference=CASE WHEN ?1 THEN NULL ELSE account_reference END",
                rusqlite::params![anonymize_descriptions],
            )?;
            db.execute(
                "UPDATE security_transactions SET
                   quantity=NULL, price=NULL, exchange_rate=NULL,
                   gross_amount_minor=CASE WHEN gross_amount_minor IS NULL THEN NULL ELSE CAST(round(gross_amount_minor * ?1) AS INTEGER) END,
                   fees_minor=CASE WHEN fees_minor IS NULL THEN NULL ELSE CAST(round(fees_minor * ?1) AS INTEGER) END,
                   taxes_minor=CASE WHEN taxes_minor IS NULL THEN NULL ELSE CAST(round(taxes_minor * ?1) AS INTEGER) END,
                   withholding_tax_minor=CASE WHEN withholding_tax_minor IS NULL THEN NULL ELSE CAST(round(withholding_tax_minor * ?1) AS INTEGER) END,
                   accrued_interest_minor=CASE WHEN accrued_interest_minor IS NULL THEN NULL ELSE CAST(round(accrued_interest_minor * ?1) AS INTEGER) END",
                rusqlite::params![factor],
            )?;
            db.execute(
                "UPDATE balance_snapshots SET
                   amount_minor=CASE WHEN amount_minor=0 THEN 0 ELSE
                     (CASE WHEN amount_minor<0 THEN -1 ELSE 1 END) *
                     max(1,CAST(round(abs(amount_minor) * ?1) AS INTEGER)) END",
                rusqlite::params![factor],
            )?;
            db.execute(
                "UPDATE annual_tax_snapshots SET
                   gross_assets_minor=CAST(round(gross_assets_minor * ?1) AS INTEGER),
                   liabilities_minor=CAST(round(liabilities_minor * ?1) AS INTEGER),
                   taxable_wealth_minor=CAST(round(gross_assets_minor * ?1) AS INTEGER)
                     - CAST(round(liabilities_minor * ?1) AS INTEGER),
                   canton_taxable_wealth_minor=CASE WHEN canton_taxable_wealth_minor IS NULL THEN NULL ELSE
                     min(CAST(round(canton_taxable_wealth_minor * ?1) AS INTEGER),
                     CAST(round(gross_assets_minor * ?1) AS INTEGER)
                       - CAST(round(liabilities_minor * ?1) AS INTEGER)) END,
                   source_name=CASE WHEN ?2
                     THEN 'Anonymisierte Steuererklärung ' || tax_year || '.pdf' ELSE source_name END",
                rusqlite::params![factor, anonymize_descriptions],
            )?;
            db.execute(
                "UPDATE annual_tax_snapshot_breakdowns SET
                   securities_and_cash_minor=CAST(round(securities_and_cash_minor * ?1) AS INTEGER),
                   real_estate_minor=CAST(round(real_estate_minor * ?1) AS INTEGER),
                   other_assets_minor=(SELECT gross_assets_minor FROM annual_tax_snapshots s
                     WHERE s.id=annual_tax_snapshot_breakdowns.snapshot_id)
                     - CAST(round(securities_and_cash_minor * ?1) AS INTEGER)
                     - CAST(round(real_estate_minor * ?1) AS INTEGER)",
                rusqlite::params![factor],
            )?;
            db.execute("DELETE FROM merchant_category_rules", [])?;
            db.execute("DELETE FROM settlement_rules", [])?;
            db.execute_batch("COMMIT; VACUUM;")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = db.execute_batch("ROLLBACK;");
            return Err("Finanzprofil konnte nicht anonymisiert werden.".into());
        }
        Ok(())
    }

    pub(crate) fn delete_database(&self) -> Result<(), String> {
        drop(self.require_unlocked()?);
        let id = {
            let session = self.session.read().map_err(|_| LOCKED)?;
            session_id(&session).to_string()
        };
        if id == PRIMARY_DATABASE_ID {
            return Err("Das Hauptprofil kann nicht gelöscht werden.".into());
        }
        let path = self.path_for_database(&id);
        self.persist_database_choice(PRIMARY_DATABASE_ID)?;
        if fs::remove_file(path).is_err() {
            let _ = self.persist_database_choice(&id);
            return Err("Finanzprofil konnte nicht gelöscht werden.".into());
        }
        {
            let mut session = self.session.write().map_err(|_| LOCKED)?;
            session.clear();
            session.database_id = PRIMARY_DATABASE_ID.into();
            session.settings = AppSettings::default();
        }
        Ok(())
    }
}

pub(crate) fn list_profiles(storage: &Storage) -> Result<Vec<DatabaseChoice>, String> {
    let session = storage.session.read().map_err(|_| LOCKED)?;
    let mut ids = if storage.path.is_file() {
        vec![PRIMARY_DATABASE_ID.to_string()]
    } else {
        Vec::new()
    };
    for entry in fs::read_dir(storage.path.parent().ok_or("Speicherort fehlt.")?)
        .map_err(|_| "Finanzprofile konnten nicht gelesen werden.")?
    {
        let entry = entry.map_err(|_| "Finanzprofile konnten nicht gelesen werden.")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path() == storage.path {
            continue;
        }
        if let Some(id) = name.strip_suffix(DATABASE_SUFFIX) {
            if id != PRIMARY_DATABASE_ID
                && valid_id(id)
                && entry
                    .file_type()
                    .map_err(|_| "Finanzprofil nicht lesbar.")?
                    .is_file()
            {
                ids.push(id.into());
            }
        }
    }
    ids.sort_by_key(|id| id.to_lowercase());
    ids.dedup();
    Ok(ids
        .into_iter()
        .map(|id| DatabaseChoice {
            demo: demo::is_reusable_demo(&storage.path_for_database(&id), &id),
            name: if id == PRIMARY_DATABASE_ID {
                "Meine Daten".into()
            } else {
                id.clone()
            },
            active: id == session_id(&session),
            id,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    type TaxValues = (i64, i64, i64, i64, i64, i64, i64);

    fn storage_with_tax_snapshot() -> (tempfile::TempDir, Storage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage {
            path: directory.path().join("anonymization-test.sqlite3"),
            session: std::sync::RwLock::new(Session::default()),
            _lock: None,
        };
        storage.unlock("test-password".into(), true).unwrap();
        let connection = storage.connect().unwrap();
        connection
            .execute(
                "INSERT INTO annual_tax_snapshots(
               tax_year,valuation_date,gross_assets_minor,liabilities_minor,
               taxable_wealth_minor,canton_taxable_wealth_minor,currency,
               source_name,source_hash,parser_version,extraction_confidence,imported_at
             ) VALUES(2025,'2025-12-31',100000000,20000000,80000000,80000000,
               'CHF','Steuererklärung 2025.pdf','test-hash','test',1.0,'2026-01-01')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO annual_tax_snapshot_breakdowns(
               snapshot_id,securities_and_cash_minor,real_estate_minor,other_assets_minor
             ) VALUES(1,60000000,30000000,10000000)",
                [],
            )
            .unwrap();
        drop(connection);
        (directory, storage)
    }

    fn tax_values(storage: &Storage) -> TaxValues {
        let connection = storage.connect().unwrap();
        connection
            .query_row(
                "SELECT s.gross_assets_minor,s.liabilities_minor,s.taxable_wealth_minor,
                    s.canton_taxable_wealth_minor,b.securities_and_cash_minor,
                    b.real_estate_minor,b.other_assets_minor
             FROM annual_tax_snapshots s
             JOIN annual_tax_snapshot_breakdowns b ON b.snapshot_id=s.id",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap()
    }

    fn assert_consistent(values: TaxValues) {
        assert_eq!(values.0 - values.1, values.2);
        assert!(values.3 <= values.2);
        assert_eq!(values.4 + values.5 + values.6, values.0);
    }

    #[test]
    fn random_anonymization_changes_every_tax_amount_consistently() {
        let (_directory, storage) = storage_with_tax_snapshot();
        let original = tax_values(&storage);

        storage.anonymize_database(false).unwrap();

        let anonymized = tax_values(&storage);
        assert_ne!(anonymized.0, original.0);
        assert_ne!(anonymized.1, original.1);
        assert_ne!(anonymized.2, original.2);
        assert_ne!(anonymized.3, original.3);
        assert_ne!(anonymized.4, original.4);
        assert_ne!(anonymized.5, original.5);
        assert_ne!(anonymized.6, original.6);
        assert_consistent(anonymized);
    }

    #[test]
    fn factor_anonymization_scales_every_tax_amount_consistently() {
        let (_directory, storage) = storage_with_tax_snapshot();

        storage.anonymize_database_with_factor(0.5, true).unwrap();

        let anonymized = tax_values(&storage);
        assert_eq!(
            anonymized,
            (50000000, 10000000, 40000000, 40000000, 30000000, 15000000, 5000000)
        );
        assert_consistent(anonymized);
        let source_name: String = storage
            .connect()
            .unwrap()
            .query_row("SELECT source_name FROM annual_tax_snapshots", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(source_name, "Anonymisierte Steuererklärung 2025.pdf");
    }
}
