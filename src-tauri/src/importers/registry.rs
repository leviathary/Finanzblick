//! Erkennt unterstützte Dateiformate und leitet sie an den passenden Importer weiter.

use super::*;

/// Common extension point for statement importers. New bank- or format-specific
/// implementations can be registered before the generic fallbacks without
/// changing the storage and review pipeline.
pub(super) trait StatementImporter: Sync {
    fn id(&self) -> &'static str;
    fn supports(
        &self,
        path: &Path,
        selected_provider: Option<&str>,
        mapping: Option<&TabularMapping>,
    ) -> bool;
    fn parse(
        &self,
        path: &Path,
        selected_provider: Option<&str>,
        mapping: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String>;
}

struct MappedTabularImporter;
struct ExcelImporter;
struct CsvImporter;
struct Mt940Importer;
struct PdfImporter;
struct Camt053Importer;

impl StatementImporter for Camt053Importer {
    fn id(&self) -> &'static str {
        "camt053"
    }
    fn supports(&self, path: &Path, _: Option<&str>, _: Option<&TabularMapping>) -> bool {
        extension(path) == "xml"
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        _: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        camt053::parse(path, provider)
    }
}

impl StatementImporter for MappedTabularImporter {
    fn id(&self) -> &'static str {
        "mapped-tabular"
    }
    fn supports(&self, path: &Path, _: Option<&str>, mapping: Option<&TabularMapping>) -> bool {
        matches!(extension(path).as_str(), "xlsx" | "xls") && mapping.is_some()
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        mapping: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        tabular::parse_mapped_workbook(path, provider, mapping.expect("mapping checked"))
    }
}

impl StatementImporter for ExcelImporter {
    fn id(&self) -> &'static str {
        "excel"
    }
    fn supports(&self, path: &Path, _: Option<&str>, mapping: Option<&TabularMapping>) -> bool {
        matches!(extension(path).as_str(), "xlsx" | "xls") && mapping.is_none()
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        _: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        excel::parse(path, provider)
    }
}

impl StatementImporter for CsvImporter {
    fn id(&self) -> &'static str {
        "csv"
    }
    fn supports(&self, path: &Path, _: Option<&str>, _: Option<&TabularMapping>) -> bool {
        extension(path) == "csv"
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        _: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        parse_csv(path, provider)
    }
}

impl StatementImporter for Mt940Importer {
    fn id(&self) -> &'static str {
        "mt940"
    }
    fn supports(&self, path: &Path, _: Option<&str>, _: Option<&TabularMapping>) -> bool {
        matches!(extension(path).as_str(), "mt940" | "sta")
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        _: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        mt940::parse(path, provider)
    }
}

impl StatementImporter for PdfImporter {
    fn id(&self) -> &'static str {
        "pdf"
    }
    fn supports(&self, path: &Path, _: Option<&str>, _: Option<&TabularMapping>) -> bool {
        extension(path) == "pdf"
    }
    fn parse(
        &self,
        path: &Path,
        provider: Option<&str>,
        _: Option<&TabularMapping>,
    ) -> Result<ParsedStatement, String> {
        pdf::parse(path, provider)
    }
}

static IMPORTERS: [&dyn StatementImporter; 6] = [
    &Camt053Importer,
    &MappedTabularImporter,
    &ExcelImporter,
    &CsvImporter,
    &Mt940Importer,
    &PdfImporter,
];

pub(super) fn parse(
    path: &Path,
    provider: Option<&str>,
    mapping: Option<&TabularMapping>,
) -> Result<ParsedStatement, String> {
    let importer = IMPORTERS
        .iter()
        .find(|importer| importer.supports(path, provider, mapping))
        .ok_or_else(|| {
            "Unterstützt werden XLSX, XLS, CSV, PDF, MT940 und camt.053 (XML).".to_string()
        })?;
    debug_assert!(!importer.id().is_empty());
    importer.parse(path, provider, mapping)
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_importer_precedes_excel_fallback() {
        let mapping = TabularMapping::default();
        let path = Path::new("statement.xlsx");
        assert_eq!(
            IMPORTERS
                .iter()
                .find(|item| item.supports(path, None, Some(&mapping)))
                .unwrap()
                .id(),
            "mapped-tabular"
        );
        assert_eq!(
            IMPORTERS
                .iter()
                .find(|item| item.supports(path, None, None))
                .unwrap()
                .id(),
            "excel"
        );
    }
}
