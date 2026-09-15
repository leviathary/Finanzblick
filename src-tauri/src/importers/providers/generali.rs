use super::ProviderImporter;

pub(super) static IMPORTER: GeneraliImporter = GeneraliImporter;

pub(super) struct GeneraliImporter;

impl ProviderImporter for GeneraliImporter {
    fn id(&self) -> &'static str {
        "generali"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["generali"]
    }
}
