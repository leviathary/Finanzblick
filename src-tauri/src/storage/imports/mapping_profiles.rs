//! Speichert und lädt wiederverwendbare Zuordnungen für Tabellenimporte.
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::{
    ImportMappingProfile, PdfImportMappingProfile, SaveImportMappingProfile,
    SavePdfImportMappingProfile,
};
use chrono::Utc;
use rusqlite::params;

pub(crate) fn list_import_mapping_profiles(
    storage: &Storage,
) -> Result<Vec<ImportMappingProfile>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection.prepare("SELECT id,name,header_fingerprint,mapping_json FROM import_mapping_profiles ORDER BY name COLLATE NOCASE")
        .map_err(db_error)?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(db_error)?;
    let mut profiles = Vec::new();
    for row in rows {
        let (id, name, header_fingerprint, json) = row.map_err(db_error)?;
        let mapping = serde_json::from_str(&json)
            .map_err(|_| "Ein gespeichertes Importprofil ist beschädigt.".to_string())?;
        profiles.push(ImportMappingProfile {
            id,
            name,
            header_fingerprint,
            mapping,
        });
    }
    Ok(profiles)
}

pub(crate) fn list_pdf_import_mapping_profiles(
    storage: &Storage,
) -> Result<Vec<PdfImportMappingProfile>, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection
        .prepare(
            "SELECT id,name,layout_fingerprint,mapping_json FROM pdf_import_mapping_profiles ORDER BY name COLLATE NOCASE",
        )
        .map_err(db_error)?;
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(db_error)?;
    let mut profiles = Vec::new();
    for row in rows {
        let (id, name, layout_fingerprint, json) = row.map_err(db_error)?;
        let mapping = serde_json::from_str(&json)
            .map_err(|_| "Ein gespeichertes PDF-Importprofil ist beschädigt.".to_string())?;
        profiles.push(PdfImportMappingProfile {
            id,
            name,
            layout_fingerprint,
            mapping,
        });
    }
    Ok(profiles)
}

pub(crate) fn save_import_mapping_profile(
    storage: &Storage,
    profile: SaveImportMappingProfile,
) -> Result<i64, String> {
    let name = profile.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Bitte einen Profilnamen mit höchstens 80 Zeichen eingeben.".into());
    }
    if profile.header_fingerprint.trim().is_empty() || profile.header_fingerprint.len() > 4096 {
        return Err("Die Spaltenstruktur des Profils ist ungültig.".into());
    }
    let mapping = serde_json::to_string(&profile.mapping)
        .map_err(|_| "Importprofil konnte nicht gespeichert werden.".to_string())?;
    let connection = storage.connect().map_err(db_error)?;
    connection.execute(
        "INSERT INTO import_mapping_profiles(name,header_fingerprint,mapping_json,updated_at) VALUES(?1,?2,?3,?4)
         ON CONFLICT(name) DO UPDATE SET header_fingerprint=excluded.header_fingerprint,mapping_json=excluded.mapping_json,updated_at=excluded.updated_at",
        params![name, profile.header_fingerprint, mapping, Utc::now().to_rfc3339()],
    ).map_err(db_error)?;
    connection
        .query_row(
            "SELECT id FROM import_mapping_profiles WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .map_err(db_error)
}

pub(crate) fn save_pdf_import_mapping_profile(
    storage: &Storage,
    profile: SavePdfImportMappingProfile,
) -> Result<i64, String> {
    let name = profile.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Bitte einen Profilnamen mit höchstens 80 Zeichen eingeben.".into());
    }
    if profile.layout_fingerprint.trim().is_empty() || profile.layout_fingerprint.len() > 256 {
        return Err("Die PDF-Struktur des Profils ist ungültig.".into());
    }
    crate::importers::validate_pdf_mapping(&profile.mapping)?;
    let mapping = serde_json::to_string(&profile.mapping)
        .map_err(|_| "PDF-Importprofil konnte nicht gespeichert werden.".to_string())?;
    let connection = storage.connect().map_err(db_error)?;
    connection
        .execute(
            "INSERT INTO pdf_import_mapping_profiles(name,layout_fingerprint,mapping_json,updated_at) VALUES(?1,?2,?3,?4)
             ON CONFLICT(name) DO UPDATE SET layout_fingerprint=excluded.layout_fingerprint,mapping_json=excluded.mapping_json,updated_at=excluded.updated_at",
            params![name, profile.layout_fingerprint, mapping, Utc::now().to_rfc3339()],
        )
        .map_err(db_error)?;
    connection
        .query_row(
            "SELECT id FROM pdf_import_mapping_profiles WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .map_err(db_error)
}

#[cfg(test)]
mod tests {
    use super::{list_pdf_import_mapping_profiles, save_pdf_import_mapping_profile};
    use crate::importers::PdfMapping;
    use crate::storage::database::{schema::initialize_schema, Storage};
    use crate::storage::imports::models::SavePdfImportMappingProfile;
    use serde_json::json;

    #[test]
    fn pdf_mapping_profiles_round_trip_without_source_content() {
        let directory = std::env::temp_dir().join(format!(
            "saldonaut-pdf-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let storage = Storage::test_storage(directory.join("test.sqlite3"));
        initialize_schema(&storage.connect().unwrap()).unwrap();
        let mapping: PdfMapping = serde_json::from_value(json!({
            "dateIndex": 0,
            "valueDateIndex": 1,
            "descriptionSource": "before",
            "amountIndex": 0,
            "balanceIndex": 1,
            "fixedCurrency": "CHF",
            "amountSign": "infer-from-balance",
            "dateFormat": "dmy",
            "numberFormat": "decimal-point",
            "ignoredDescriptions": ["opening balance"]
        }))
        .unwrap();

        save_pdf_import_mapping_profile(
            &storage,
            SavePdfImportMappingProfile {
                name: "Example layout".into(),
                layout_fingerprint: "pdf-v1:d2:m2:22100".into(),
                mapping,
            },
        )
        .unwrap();
        let profiles = list_pdf_import_mapping_profiles(&storage).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Example layout");
        assert_eq!(profiles[0].layout_fingerprint, "pdf-v1:d2:m2:22100");
        assert!(!serde_json::to_string(&profiles[0].mapping)
            .unwrap()
            .contains("Example layout"));

        drop(storage);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
