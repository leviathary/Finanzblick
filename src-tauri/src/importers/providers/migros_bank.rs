//! Bindet Migros-Bank-Erkennung und das Vorsorgekonto-PDF-Profil an die gemeinsame Importengine.

use super::ProviderImporter;

const PILLAR3A_STATEMENT_PROFILE: &str = include_str!("profiles/migros-pillar3a-statement-v1.json");
const SAVINGS_STATEMENT_PROFILE: &str = include_str!("profiles/migros-savings-statement-v1.json");
const PDF_PROFILES: &[&str] = &[PILLAR3A_STATEMENT_PROFILE, SAVINGS_STATEMENT_PROFILE];

pub(super) static IMPORTER: MigrosBankImporter = MigrosBankImporter;

pub(super) struct MigrosBankImporter;

impl ProviderImporter for MigrosBankImporter {
    fn id(&self) -> &'static str {
        "migros"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["migros bank", "migros"]
    }

    fn bundled_pdf_profiles(&self) -> &'static [&'static str] {
        PDF_PROFILES
    }
}

#[cfg(test)]
mod tests {
    use super::MigrosBankImporter;
    use crate::importers::providers::parse_pdf;
    use std::path::Path;

    const STATEMENT: &str = r#"
MIGROS BANK
Migros Bank AG
Portfolio Vorsorge-Portfolio
Vorsorgekonto 000.000.00 / CHF
IBAN CH00 0840 1000 0000 0000 0
Anfangssaldo CHF 10'000.00
Belastungen CHF 0.00
Gutschriften CHF 120.00
Schlusssaldo CHF 10'120.00
Kontoabschluss per 31.12.2025 Abschluss 01.01.2025 - 31.12.2025
Kontoauszug 01.01.2025 - 31.12.2025
Datum Text Valuta Belastungen Gutschriften Saldo CHF
Saldovortrag 10'000.00
31.12.25 Habenzins 31.12.25 120.00 10'120.00
Saldo per 31.12.2025 0.00 120.00 10'120.00
Schlusssaldo 10'120.00
Konditionen Gültig ab Limiten Zins
"#;

    const SAVINGS_STATEMENT: &str = r#"
Migros Bank AG
Datum/Zeit 03.01.2026 10:15:00
Vertrag EB00000000
Konto 000.000.00 / CHF
IBAN CH00 0840 1000 0000 0000 0
Bezeichnung Sparkonto
Kontoauszug
Anfangssaldo per 01.01.2026 CHF 100.00
Belastungen CHF 30.00
Gutschriften CHF 50.00
Schlusssaldo per 02.01.2026 CHF 120.00
Datum Buchungstext Valuta Belastung Gutschrift Saldo CHF
01.01.2026 Belastung 01.01.2026 30.00 70.00
Begünstigter
Beispiel AG
Mitteilung / Referenz
Rechnung 100
Angaben ohne Gewähr Seite 1 / 2 0000000 | 00000000000000 | 0000
Migros Bank AG
Postfach
8000 Zürich
CHE-000.000.000 MWST
BIC/Swift: TESTCHZZXXX
migrosbank.ch/kontakt
Kontoauszug
Datum Buchungstext Valuta Belastung Gutschrift Saldo CHF
02.01.2026 Zahlungseingang 02.01.2026 50.00 120.00
Auftraggeber
Muster GmbH
Mitteilung / Referenz
Gutschrift 200
Angaben ohne Gewähr Seite 2 / 2 0000000 | 00000000000000 | 0000
"#;

    fn parse(text: &str) -> Result<crate::importers::ParsedStatement, String> {
        parse_pdf(&MigrosBankImporter, Path::new("statement.pdf"), text)
            .expect("Migros pillar 3a profile should recognize statement")
    }

    #[test]
    fn parses_and_reconciles_pillar3a_statement() {
        let parsed = parse(STATEMENT).unwrap();

        assert_eq!(parsed.provider, "migros");
        assert_eq!(parsed.account_type.as_deref(), Some("pillar3a"));
        assert_eq!(parsed.account_name, "Vorsorgekonto");
        assert_eq!(
            parsed.account_reference.as_deref(),
            Some("CH0008401000000000000")
        );
        assert_eq!(parsed.document_date.as_deref(), Some("2025-12-31"));
        assert_eq!(parsed.opening_balance_minor, Some(1_000_000));
        assert_eq!(parsed.closing_balance_minor, Some(1_012_000));
        assert_eq!(
            parsed.currency_balances[0].opening_date.as_deref(),
            Some("2025-01-01")
        );
        assert_eq!(parsed.currency_balances[0].closing_date, "2025-12-31");
        assert_eq!(parsed.transactions.len(), 1);
        assert_eq!(parsed.transactions[0].description, "Habenzins");
        assert_eq!(parsed.transactions[0].amount_minor, 12_000);
        assert_eq!(parsed.transactions[0].balance_minor, Some(1_012_000));
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn parses_multi_page_savings_statement_with_details() {
        let parsed = parse(SAVINGS_STATEMENT).unwrap();

        assert_eq!(parsed.provider, "migros");
        assert_eq!(parsed.account_type.as_deref(), Some("savings"));
        assert_eq!(parsed.account_name, "Sparkonto");
        assert_eq!(parsed.opening_balance_minor, Some(10_000));
        assert_eq!(parsed.closing_balance_minor, Some(12_000));
        assert_eq!(parsed.transactions.len(), 2);
        assert_eq!(parsed.transactions[0].amount_minor, -3_000);
        assert_eq!(parsed.transactions[0].balance_minor, Some(7_000));
        assert!(parsed.transactions[0].description.contains("Rechnung 100"));
        assert!(!parsed.transactions[0]
            .description
            .contains("Migros Bank AG"));
        assert_eq!(parsed.transactions[1].amount_minor, 5_000);
        assert_eq!(parsed.transactions[1].balance_minor, Some(12_000));
        assert!(parsed.transactions[1]
            .description
            .contains("Gutschrift 200"));
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn rejects_inconsistent_totals_and_other_providers() {
        assert!(parse(&STATEMENT.replace(
            "Saldo per 31.12.2025 0.00 120.00 10'120.00",
            "Saldo per 31.12.2025 0.00 120.01 10'120.00",
        ))
        .is_err());
        assert!(parse(
            &SAVINGS_STATEMENT.replace("Belastungen CHF 30.00", "Belastungen CHF 30.01",)
        )
        .is_err());
        assert!(parse_pdf(
            &MigrosBankImporter,
            Path::new("statement.pdf"),
            "PostFinance Kontoauszug Datum Saldo Valuta Lastschrift Gutschrift Text",
        )
        .is_none());
    }
}
