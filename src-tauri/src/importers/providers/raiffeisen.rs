use super::ProviderImporter;

pub(super) static IMPORTER: RaiffeisenImporter = RaiffeisenImporter;

pub(super) struct RaiffeisenImporter;

impl ProviderImporter for RaiffeisenImporter {
    fn id(&self) -> &'static str {
        "raiffeisen"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["raiffeisen"]
    }
}
