//! Koordiniert die Freigabe normalisierter Importe; Parser und Datenbankabgleich bleiben getrennt.
use crate::storage::{imports, SaveImportRequest, SaveImportResult, Storage};

pub(crate) fn save(
    storage: &Storage,
    request: SaveImportRequest,
) -> Result<SaveImportResult, String> {
    if request.account_ids.is_empty() {
        return Err(
            "Bitte ein bestehendes Konto auswählen. Neue Konten unter Banken & Konten anlegen."
                .into(),
        );
    }
    imports::save_import_to(storage, request)
}
