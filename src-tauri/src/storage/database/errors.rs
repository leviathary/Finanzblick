//! Übersetzt Datenbankfehler in die bestehenden Anwendungsfehlermeldungen.
pub(crate) fn db_error(error: rusqlite::Error) -> String {
    if matches!(error, rusqlite::Error::InvalidQuery) {
        return "Finanzblick ist gesperrt. Bitte erneut entsperren.".into();
    }
    format!("Das lokale Finanzprofil konnte nicht aktualisiert werden: {error}")
}
