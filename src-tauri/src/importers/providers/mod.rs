//! Registriert Bankanbieter und deren optionale Parser und Spaltenzuordnungen.

use super::{BundledImportProfileSummary, ParsedStatement, ProviderExcelMapping};
use crate::importers::formats::pdf_profile::BundledPdfProfileSummary;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct IntegratedImportProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub formats: &'static [&'static str],
    pub document_type: &'static str,
}

mod generali;
mod migros_bank;
mod postfinance;
mod raiffeisen;
mod swissquote;
mod swissquote_positions;
pub(super) mod ubs;
mod zkb;

/// Bank-specific extension point. Format readers remain shared, while every
/// provider owns its aliases, column mapping and special document layouts.
pub(super) trait ProviderImporter: Sync {
    fn id(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str];
    fn parse_positions(
        &self,
        _path: &Path,
    ) -> Result<Option<crate::domain::securities::position_snapshots::PositionSnapshot>, String>
    {
        Ok(None)
    }
    fn position_reference(&self, value: &str) -> String {
        crate::domain::securities::position_snapshots::normalize_reference(value)
    }
    fn supports_provisional_card_csv(&self) -> bool {
        false
    }
    fn card_credit_kind(&self, _description: &str) -> Option<&'static str> {
        None
    }

    fn excel_mapping(&self) -> Option<&'static ProviderExcelMapping> {
        None
    }

    fn bundled_pdf_profiles(&self) -> &'static [&'static str] {
        &[]
    }

    fn bundled_card_pdf_profiles(&self) -> &'static [&'static str] {
        &[]
    }

    fn integrated_import_profiles(&self) -> &'static [IntegratedImportProfile] {
        &[]
    }

    fn parse_pdf_hook(&self, _path: &Path, _text: &str) -> Option<Result<ParsedStatement, String>> {
        None
    }
}

static PROVIDERS: [&dyn ProviderImporter; 7] = [
    &zkb::IMPORTER,
    &ubs::IMPORTER,
    &postfinance::IMPORTER,
    &migros_bank::IMPORTER,
    &raiffeisen::IMPORTER,
    &swissquote::IMPORTER,
    &generali::IMPORTER,
];

pub(super) fn position_providers() -> &'static [&'static dyn ProviderImporter] {
    &PROVIDERS
}

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

pub(super) fn parse_pdf(
    provider: &dyn ProviderImporter,
    path: &Path,
    text: &str,
) -> Option<Result<ParsedStatement, String>> {
    if let Some(result) = provider.bundled_pdf_profiles().iter().find_map(|profile| {
        crate::importers::formats::pdf_profile::parse_bundled(profile, provider.id(), text)
    }) {
        return Some(result);
    }
    if let Some(result) = provider
        .bundled_card_pdf_profiles()
        .iter()
        .find_map(|profile| {
            crate::importers::formats::pdf_card_profile::parse_bundled(profile, provider.id(), text)
        })
    {
        return Some(result);
    }
    provider.parse_pdf_hook(path, text)
}

fn bundled_pdf_profile_summaries() -> Result<Vec<BundledPdfProfileSummary>, String> {
    let account_profiles = PROVIDERS.iter().flat_map(|provider| {
        provider.bundled_pdf_profiles().iter().map(move |profile| {
            crate::importers::formats::pdf_profile::bundled_profile_summary(profile, provider.id())
        })
    });
    let card_profiles = PROVIDERS.iter().flat_map(|provider| {
        provider
            .bundled_card_pdf_profiles()
            .iter()
            .map(move |profile| {
                crate::importers::formats::pdf_card_profile::bundled_profile_summary(
                    profile,
                    provider.id(),
                )
            })
    });
    account_profiles.chain(card_profiles).collect()
}

pub(super) fn bundled_import_profile_summaries() -> Result<Vec<BundledImportProfileSummary>, String>
{
    let mut profiles = bundled_pdf_profile_summaries()?
        .into_iter()
        .map(|profile| BundledImportProfileSummary {
            id: profile.id,
            provider: profile.provider,
            display_name: profile.display_name,
            formats: vec!["PDF".into()],
            document_type: profile.document_type,
            schema_version: Some(profile.schema_version),
        })
        .collect::<Vec<_>>();
    profiles.extend(PROVIDERS.iter().flat_map(|provider| {
        provider
            .integrated_import_profiles()
            .iter()
            .map(move |profile| BundledImportProfileSummary {
                id: profile.id.into(),
                provider: provider.id().into(),
                display_name: profile.display_name.into(),
                formats: profile
                    .formats
                    .iter()
                    .map(|format| (*format).into())
                    .collect(),
                document_type: profile.document_type.into(),
                schema_version: None,
            })
    }));
    profiles.sort_by(|left, right| {
        left.provider
            .cmp(&right.provider)
            .then(left.display_name.cmp(&right.display_name))
    });
    Ok(profiles)
}

pub(super) fn detect(value: &str) -> Option<&'static dyn ProviderImporter> {
    let normalized = super::normalized(value);
    PROVIDERS
        .iter()
        .copied()
        .filter_map(|provider| {
            provider
                .aliases()
                .iter()
                .filter_map(|alias| normalized.find(&super::normalized(alias)))
                .min()
                .map(|position| (position, provider))
        })
        .min_by_key(|(position, _)| *position)
        .map(|(_, provider)| provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ubs_credit_labels_are_narrow_and_provider_owned() {
        let ubs = by_id("ubs").unwrap();
        assert!(ubs.supports_provisional_card_csv());
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
        assert!(!by_id("swissquote").unwrap().supports_provisional_card_csv());
    }

    #[test]
    fn lists_every_tested_provider_import_profile() {
        let profiles = bundled_import_profile_summaries().unwrap();

        assert_eq!(profiles.len(), 9);
        assert!(profiles.iter().any(|profile| {
            profile.id == "migros-pillar3a-statement-v1"
                && profile.provider == "migros"
                && profile.schema_version == Some(1)
                && profile.document_type == "statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "migros-savings-statement-v1"
                && profile.provider == "migros"
                && profile.display_name == "Migros-Bank-Kontoauszug"
                && profile.schema_version == Some(1)
                && profile.document_type == "statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "postfinance-account-statement-v1"
                && profile.provider == "postfinance"
                && profile.schema_version == Some(1)
                && profile.formats == ["PDF"]
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "ubs-monthly-account-statement-v1"
                && profile.provider == "ubs"
                && profile.schema_version == Some(1)
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "ubs-credit-card-statement-v1"
                && profile.provider == "ubs"
                && profile.document_type == "credit_card_statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "ubs-credit-card-csv"
                && profile.formats == ["CSV"]
                && profile.document_type == "credit_card_transactions"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "swissquote-position-statement"
                && profile.formats == ["PDF", "XLSX", "XLS"]
                && profile.document_type == "position_statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "swissquote-account-statement"
                && profile.formats == ["PDF"]
                && profile.document_type == "statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "zkb-transaction-notice"
                && profile.formats == ["PDF"]
                && profile.document_type == "transaction_notice"
        }));
    }

    #[test]
    fn complete_overview_includes_bank_independent_statement_standards() {
        let profiles = crate::importers::bundled_import_profile_summaries().unwrap();

        assert_eq!(profiles.len(), 12);
        assert!(profiles.iter().any(|profile| {
            profile.id == "mt940-account-statement"
                && profile.provider == "standard"
                && profile.formats == ["MT940", "STA"]
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "camt053-account-statement"
                && profile.formats == ["XML"]
                && profile.document_type == "statement"
        }));
        assert!(profiles.iter().any(|profile| {
            profile.id == "camt054-account-notification"
                && profile.formats == ["XML"]
                && profile.document_type == "transaction_notification"
        }));
    }

    #[test]
    fn resolves_every_registered_provider() {
        let path = Path::new("positions.xlsx");
        assert!(by_id("ubs")
            .unwrap()
            .parse_positions(path)
            .unwrap()
            .is_none());
        for provider in PROVIDERS {
            assert_eq!(by_id(provider.id()).unwrap().id(), provider.id());
        }
        assert_eq!(detect("UBS Switzerland AG").unwrap().id(), "ubs");
        assert_eq!(
            detect("UBS Privatkonto CHF\nBuchungen\nZKB ZUERICH ZOO")
                .unwrap()
                .id(),
            "ubs"
        );
        assert_eq!(detect("Migros Bank AG").unwrap().id(), "migros");
        assert_eq!(detect("PostFinance AG").unwrap().id(), "postfinance");
        assert_eq!(detect("Zürcher Kantonalbank").unwrap().id(), "zkb");
        assert_ne!(detect("UBS Kontoauszug").unwrap().id(), "postfinance");
    }
}
