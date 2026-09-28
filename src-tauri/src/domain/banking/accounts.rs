//! DB-unabhängige Kontenvalidierung, Typbezeichnungen und Vermögensvorgaben.
pub(crate) fn account_type_label(value: &str) -> &'static str {
    match value {
        "cash" => "Konten",
        "savings" => "Sparkonten",
        "portfolio" => "Depots",
        "pillar3a" => "Vorsorgekonten",
        "mortgage" => "Hypotheken",
        "credit_card" => "Kreditkarten",
        "manual_asset" => "Manuelle Positionen",
        _ => "Sonstiges",
    }
}

pub(crate) fn asset_type_label(value: &str) -> &'static str {
    match value {
        "stock" => "Aktien",
        "option" => "Optionen",
        "crypto" => "Kryptowährungen",
        "cash" => "Cash & Geldbeträge",
        "fund" => "Fonds & ETF",
        "bond" => "Obligationen",
        _ => "Sonstige Anlagen",
    }
}

pub(crate) fn validate_account(
    name: &str,
    currency: &str,
    account_type: &str,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Bitte einen Kontonamen eingeben.".to_string());
    }
    if currency.trim().len() != 3 {
        return Err("Die Währung muss aus drei Buchstaben bestehen.".to_string());
    }
    if ![
        "cash",
        "savings",
        "portfolio",
        "pillar3a",
        "mortgage",
        "credit_card",
        "manual_asset",
    ]
    .contains(&account_type)
    {
        return Err("Der Kontotyp ist ungültig.".to_string());
    }
    Ok(())
}

pub(crate) fn default_include_in_net_worth(account_type: &str) -> bool {
    account_type != "pillar3a"
}

/// Positionen werden unabhängig von Anbieter und Erfassungsweg gleich verwaltet.
pub(crate) fn supports_positions(account_type: &str) -> bool {
    matches!(account_type, "portfolio" | "manual_asset" | "pillar3a")
}

/// Normalisiert IBANs und andere Kontokennungen für Speicherung und Vergleich.
/// Neben Unicode-Leerraum werden unsichtbare Copy/Paste-Trennzeichen entfernt.
pub(crate) fn normalize_account_reference(value: &str) -> String {
    let trimmed = value.trim();
    let without_label = if trimmed
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("IBAN"))
    {
        trimmed[4..].trim_start_matches([':', ' '])
    } else {
        trimmed
    };
    without_label
        .chars()
        .filter(|character| {
            !character.is_whitespace() && !matches!(character, '\u{200b}' | '\u{2060}' | '\u{feff}')
        })
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_account_reference;

    #[test]
    fn normalizes_formatted_and_pasted_ibans() {
        assert_eq!(
            normalize_account_reference(
                " IBAN: ch36 0000\u{00a0}0000\u{202f}0000\u{200b}0000\u{feff}0 "
            ),
            "CH3600000000000000000"
        );
    }
}
