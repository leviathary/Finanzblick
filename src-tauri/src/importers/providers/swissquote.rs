use super::ProviderImporter;

pub(super) static IMPORTER: SwissquoteImporter = SwissquoteImporter;

pub(super) struct SwissquoteImporter;

impl ProviderImporter for SwissquoteImporter {
    fn id(&self) -> &'static str {
        "swissquote"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["swissquote bank", "swissquote"]
    }
}
