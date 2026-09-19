//! Liest und validiert lokale Anwendungseinstellungen.
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub auto_lock_minutes: u64,
    pub language: String,
    pub region: String,
    pub default_currency: String,
    pub marketstack_api_key: String,
    pub alpha_vantage_api_key: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_lock_minutes: 15,
            language: "de".into(),
            region: "CH".into(),
            default_currency: "CHF".into(),
            marketstack_api_key: String::new(),
            alpha_vantage_api_key: String::new(),
        }
    }
}

pub(crate) fn read_settings(db: &Connection) -> Result<AppSettings, String> {
    let value: Option<String> = db
        .query_row("SELECT value FROM app_settings WHERE id = 1", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| "Einstellungen konnten nicht gelesen werden.")?;
    let settings = value
        .map(|s| serde_json::from_str::<AppSettings>(&s))
        .transpose()
        .map_err(|_| "Einstellungen sind nicht lesbar.")?
        .unwrap_or_default();
    validate_settings(&settings)?;
    Ok(settings)
}

pub(super) fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    if ![1, 5, 10, 15, 30, 60].contains(&settings.auto_lock_minutes)
        || !["de", "en", "fr", "it"].contains(&settings.language.as_str())
        || !["CH", "DE", "AT", "FR", "IT", "GB", "US"].contains(&settings.region.as_str())
        || !["CHF", "EUR", "USD", "GBP"].contains(&settings.default_currency.as_str())
        || settings.alpha_vantage_api_key.len() > 256
        || settings.marketstack_api_key.len() > 256
    {
        return Err("Bitte gültige Einstellungen auswählen.".into());
    }
    Ok(())
}
