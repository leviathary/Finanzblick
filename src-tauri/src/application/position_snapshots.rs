//! Orchestriert den providerneutralen Positionsimport mit explizitem Stichtag und normalisierten Kontoreferenzen.
use crate::domain::securities::position_snapshots::{validate_date, PositionSnapshot};
use crate::importers::position_snapshots as importers;
use crate::storage::database::Storage;
use crate::storage::securities::position_snapshots::{
    self as repository, PositionSnapshotPreview, SavePositionSnapshotResult,
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub(crate) fn preview(
    storage: &Storage,
    path: String,
    account_id: Option<i64>,
    snapshot_date: Option<String>,
) -> Result<Option<PositionSnapshotPreview>, String> {
    let (source, hash) = validated_source(&path)?;
    let Some(mut snapshot) = importers::parse(&source)? else {
        return Ok(None);
    };
    apply_date(&mut snapshot, snapshot_date)?;
    let provider = snapshot.provider.clone();
    repository::preview(storage, account_id, snapshot, &hash, &|value| {
        importers::account_reference(&provider, value)
    })
    .map(Some)
}

pub(crate) fn save(
    storage: &Storage,
    path: String,
    account_id: i64,
    snapshot_date: Option<String>,
) -> Result<SavePositionSnapshotResult, String> {
    let (source, hash) = validated_source(&path)?;
    let mut snapshot =
        importers::parse(&source)?.ok_or("Kein unterstützter Positionsbestand erkannt.")?;
    apply_date(&mut snapshot, snapshot_date)?;
    snapshot.date()?;
    let provider = snapshot.provider.clone();
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Positionsbestand");
    repository::save(storage, account_id, snapshot, name, &hash, &|value| {
        importers::account_reference(&provider, value)
    })
}

fn apply_date(snapshot: &mut PositionSnapshot, supplied: Option<String>) -> Result<(), String> {
    if let Some(date) = supplied {
        validate_date(&date)?;
        if snapshot
            .snapshot_date
            .as_deref()
            .is_some_and(|detected| detected != date)
        {
            return Err("Der Stichtag stimmt nicht mit dem Dokumentdatum überein.".into());
        }
        snapshot.snapshot_date = Some(date);
    }
    snapshot.validate()
}

fn validated_source(path: &str) -> Result<(PathBuf, String), String> {
    let path = PathBuf::from(path);
    let metadata =
        fs::metadata(&path).map_err(|_| "Die Quelldatei ist nicht mehr verfügbar.".to_string())?;
    if !metadata.is_file() || metadata.len() > 25 * 1024 * 1024 {
        return Err("Die Quelldatei ist ungültig oder grösser als 25 MB.".into());
    }
    let bytes =
        fs::read(&path).map_err(|_| "Die Quelldatei konnte nicht gelesen werden.".to_string())?;
    Ok((path, format!("{:x}", Sha256::digest(bytes))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::securities::position_snapshots::SnapshotScope;

    #[test]
    fn explicit_date_is_required_only_when_document_has_no_date() {
        let mut snapshot = PositionSnapshot {
            provider: "example".into(),
            format: "TEST".into(),
            snapshot_date: None,
            scope: SnapshotScope::FullPortfolio,
            account_reference: None,
            reference_is_shared: false,
            positions: vec![],
            warnings: vec![],
        };
        apply_date(&mut snapshot, None).unwrap();
        assert!(snapshot.date().is_err());
        apply_date(&mut snapshot, Some("2026-01-02".into())).unwrap();
        assert_eq!(snapshot.date().unwrap(), "2026-01-02");
        assert!(apply_date(&mut snapshot, Some("2026-01-03".into())).is_err());
        assert_eq!(snapshot.date().unwrap(), "2026-01-02");
    }
}
