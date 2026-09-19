//! Erzeugt kanonische Referenzen und Fingerabdrücke normalisierter Buchungen.
use crate::importers::ParsedStatement;
use crate::storage::imports::models::TransactionIdentity;
use sha2::Digest;
use sha2::Sha256;

pub(crate) fn canonical_identity_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn transaction_identity(
    statement: &ParsedStatement,
    row: &crate::importers::ParsedTransaction,
    account_id: i64,
    fallback_occurrence: usize,
) -> TransactionIdentity {
    let external_reference = row
        .external_reference
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let namespace = external_reference.as_ref().map(|_| {
        row.reference_namespace
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(statement.provider.as_str())
            .to_lowercase()
    });
    let fingerprint_source = format!(
        "{account_id}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        row.booking_date,
        row.amount_minor,
        row.balance_minor
            .map(|value| value.to_string())
            .unwrap_or_default(),
        row.currency.to_uppercase(),
        canonical_identity_text(row.counterparty_name.as_deref().unwrap_or("")),
        canonical_identity_text(row.remittance_information.as_deref().unwrap_or("")),
        canonical_identity_text(&row.description),
        fallback_occurrence,
    );
    TransactionIdentity {
        namespace,
        external_reference,
        fallback_fingerprint: format!("{:x}", Sha256::digest(fingerprint_source.as_bytes())),
    }
}
