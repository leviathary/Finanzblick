//! Registriert Bankanbieter und deren optionale Parser und Spaltenzuordnungen.

use super::{ParsedStatement, ProviderExcelMapping};
use std::path::Path;

mod generali;
mod migros_bank;
mod raiffeisen;
mod swissquote;
pub(super) mod ubs;
mod zkb;

/// Bank-specific extension point. Format readers remain shared, while every
/// provider owns its aliases, column mapping and special document layouts.
pub(super) trait ProviderImporter: Sync {
    fn id(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str];
    fn card_credit_kind(&self, _description: &str) -> Option<&'static str> {
        None
    }

    fn excel_mapping(&self) -> Option<&'static ProviderExcelMapping> {
        None
    }

    fn parse_pdf(&self, _path: &Path, _text: &str) -> Option<Result<ParsedStatement, String>> {
        None
    }
}

static PROVIDERS: [&dyn ProviderImporter; 6] = [
    &zkb::IMPORTER,
    &ubs::IMPORTER,
    &migros_bank::IMPORTER,
    &raiffeisen::IMPORTER,
    &swissquote::IMPORTER,
    &generali::IMPORTER,
];

pub(super) fn by_id(id: &str) -> Option<&'static dyn ProviderImporter> {
    let normalized = super::normalized(id);
    PROVIDERS.iter().copied().find(|provider| {
        provider.id() == normalized
            || provider
                .aliases()
                .iter()
                .any(|alias| normalized.contains(&super::normalized(alias)))
    })
}

pub(super) fn detect(value: &str) -> Option<&'static dyn ProviderImporter> {
    let normalized = super::normalized(value);
    PROVIDERS.iter().copied().find(|provider| {
        provider
            .aliases()
            .iter()
            .any(|alias| normalized.contains(&super::normalized(alias)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ubs_credit_labels_are_narrow_and_provider_owned() {
        let ubs = by_id("ubs").unwrap();
        assert_eq!(
            ubs.card_credit_kind("2002 LSV-ZAHLUNG · Einkauf: 01.01.2026"),
            Some("card_settlement")
        );
        assert_eq!(ubs.card_credit_kind("LSV-Zahlung"), Some("card_settlement"));
        for text in [
            "Refund",
            "LSV insurance",
            "2002 LSV-ZAHLUNG SHOP",
            "Shop 2002 LSV-ZAHLUNG",
        ] {
            assert_eq!(ubs.card_credit_kind(text), None);
        }
        assert_eq!(
            by_id("swissquote")
                .unwrap()
                .card_credit_kind("2002 LSV-ZAHLUNG"),
            None
        );
    }

    #[test]
    fn resolves_every_registered_provider() {
        for provider in PROVIDERS {
            assert_eq!(by_id(provider.id()).unwrap().id(), provider.id());
        }
        assert_eq!(detect("UBS Switzerland AG").unwrap().id(), "ubs");
        assert_eq!(detect("Migros Bank AG").unwrap().id(), "migros");
        assert_eq!(detect("Zürcher Kantonalbank").unwrap().id(), "zkb");
    }
}
