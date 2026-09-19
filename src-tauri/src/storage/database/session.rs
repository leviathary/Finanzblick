//! Verwaltet Ablauf und sichere Freigabe des flüchtigen Sitzungspassworts.
use super::settings::AppSettings;
use std::time::Duration;
use std::time::SystemTime;
use zeroize::Zeroizing;
#[derive(Default)]
pub(in crate::storage) struct Session {
    pub(super) database_id: String,
    pub(super) generation: u64,
    pub(super) password: Option<Zeroizing<String>>,
    pub(super) activity: Option<SystemTime>,
    pub(super) retry_after: Option<SystemTime>,
    pub(super) settings: AppSettings,
}

impl Session {
    pub(super) fn expired(&self) -> bool {
        self.activity
            .and_then(|t| t.elapsed().ok())
            .map_or(true, |d| {
                d >= Duration::from_secs(self.settings.auto_lock_minutes * 60)
            })
    }
    pub(super) fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.password = None;
        self.activity = None;
    }
}
