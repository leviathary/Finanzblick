use super::ProviderImporter;

pub(super) static IMPORTER: MigrosBankImporter = MigrosBankImporter;

pub(super) struct MigrosBankImporter;

impl ProviderImporter for MigrosBankImporter {
    fn id(&self) -> &'static str {
        "migros"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["migros bank", "migros"]
    }
}
