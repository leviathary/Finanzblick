//! Bindet das versionierte PostFinance-Kontoauszugsprofil an die gemeinsame PDF-Profilengine.

use super::ProviderImporter;

const ACCOUNT_STATEMENT_PROFILE: &str =
    include_str!("profiles/postfinance-account-statement-v1.json");
const PDF_PROFILES: &[&str] = &[ACCOUNT_STATEMENT_PROFILE];

pub(super) static IMPORTER: PostFinanceImporter = PostFinanceImporter;

pub(super) struct PostFinanceImporter;

impl ProviderImporter for PostFinanceImporter {
    fn id(&self) -> &'static str {
        "postfinance"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["postfinance", "post finance", "pofichbexxx"]
    }

    fn bundled_pdf_profiles(&self) -> &'static [&'static str] {
        PDF_PROFILES
    }
}

#[cfg(test)]
mod tests {
    use super::PostFinanceImporter;
    use crate::importers::providers::parse_pdf;
    use std::path::Path;

    const STATEMENT: &str = r#"
PostFinance AG
Geschäftskonto
Kontoauszug 01.08.2026 - 31.08.2026
IBAN  CH00 0900 0000 0000 0000 0 CHF
Kontonummer  00-000000-0
BIC  POFICHBEXXX
Datum  01.09.2026
Datum  Saldo Valuta Lastschrift Gutschrift Text
31.07.26  10 000.00 Kontostand
LASTSCHRIFT
MUSTERAMT
03.08.26  03.08.26  9 874.60 125.40
GUTSCHRIFT
BEISPIEL AG
SENDER REFERENZ:
REF-123
10.08.26  10.08.26  10 194.70 320.10
LASTSCHRIFT
TELEKOM AG
17.08.26  17.08.26 42.25
Seite 1 / 2
Datum  Saldo Valuta Lastschrift Gutschrift Text
Datum
IBAN
Kontonummer  CH00 0900 0000 0000 0000 0
00-000000-0
01.09.2026 00000 DE 000001.00
LASTSCHRIFT
HANDWERK AG
17.08.26  17.08.26  9 977.30 175.15
PREIS FÜR DIE KONTOFÜHRUNG 31.08.26  31.08.26  9 972.80 4.50
Total  347.30 320.10
31.08.26  9 972.80 Kontostand
Bitte überprüfen Sie den Kontoauszug.
"#;

    fn parse(text: &str) -> Result<crate::importers::ParsedStatement, String> {
        parse_pdf(&PostFinanceImporter, Path::new("statement.pdf"), text)
            .expect("PostFinance profile should recognize statement")
    }

    #[test]
    fn bundled_profile_parses_and_reconciles_multi_page_statement() {
        let parsed = parse(STATEMENT).unwrap();
        assert_eq!(parsed.provider, "postfinance");
        assert_eq!(
            parsed.account_reference.as_deref(),
            Some("CH0009000000000000000")
        );
        assert_eq!(parsed.account_name, "Geschäftskonto");
        assert_eq!(parsed.transactions.len(), 5);
        assert_eq!(parsed.opening_balance_minor, Some(1_000_000));
        assert_eq!(parsed.closing_balance_minor, Some(997_280));
        assert!(parsed.warnings.is_empty());
        assert_eq!(
            parsed
                .transactions
                .iter()
                .map(|row| row.amount_minor)
                .collect::<Vec<_>>(),
            vec![-12_540, 32_010, -4_225, -17_515, -450]
        );
        assert_eq!(parsed.transactions[2].balance_minor, Some(1_015_245));
        assert!(!parsed.transactions[3].description.contains("Kontonummer"));
        assert_eq!(
            parsed.transactions[1].external_reference.as_deref(),
            Some("REF123")
        );
    }

    #[test]
    fn bundled_profile_rejects_unbalanced_control_totals() {
        let error = parse(&STATEMENT.replace("347.30", "347.29")).unwrap_err();
        assert!(error.contains("Kontrollsummen"));
    }

    #[test]
    fn bundled_profile_ignores_other_providers() {
        assert!(parse_pdf(
            &PostFinanceImporter,
            Path::new("statement.pdf"),
            "UBS Kontoauszug Datum Saldo Valuta Lastschrift Gutschrift Text"
        )
        .is_none());
    }
}
