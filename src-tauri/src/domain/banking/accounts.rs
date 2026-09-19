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

pub(crate) fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}
