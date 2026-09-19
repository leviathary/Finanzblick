//! Speichert und lädt wiederverwendbare Zuordnungen für Tabellenimporte.
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::imports::models::{ImportMappingProfile, SaveImportMappingProfile};
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
