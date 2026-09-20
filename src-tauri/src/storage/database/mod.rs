//! Kapselt Datenbankverbindungen, Sitzungen, Profile, Schema und Backups.
mod anonymized_copy;
pub(crate) mod connection;
mod handle;
pub use handle::Storage;
pub mod backups;
pub mod demo;
pub(crate) mod errors;
pub mod profiles;
pub(crate) mod queries;
pub(crate) mod schema;
mod session;
mod settings;
pub(crate) use connection::VaultStatus;
pub(crate) use settings::{read_settings, AppSettings};
const LOCKED: &str = "Finanzblick ist gesperrt. Bitte erneut entsperren.";
