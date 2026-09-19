//! Persistiert ausschließlich das lokale Farbschema, ohne Datenbank- oder Profilzugriff.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}

pub fn load(path: &Path) -> Appearance {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, appearance: Appearance) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("Missing configuration directory")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(
        path,
        serde_json::to_vec(&appearance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_round_trip_and_invalid_fallback() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config").join("appearance.json");
        assert_eq!(load(&path), Appearance::System);
        for mode in [Appearance::Light, Appearance::Dark, Appearance::System] {
            save(&path, mode).unwrap();
            assert_eq!(load(&path), mode);
        }
        std::fs::write(&path, b"invalid").unwrap();
        assert_eq!(load(&path), Appearance::System);
        assert!(save(directory.path(), Appearance::Dark).is_err());
    }
}
