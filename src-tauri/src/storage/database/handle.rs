//! Gemeinsamer Datenbank-Handle mit geschützter Sitzung und Prozesssperre.
use super::session::Session;
use std::{fs, path::PathBuf, sync::RwLock};
pub struct Storage {
    pub(in crate::storage) path: PathBuf,
    pub(in crate::storage) session: RwLock<Session>,
    pub(in crate::storage) _lock: Option<fs::File>,
}
