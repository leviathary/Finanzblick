//! Anbieterneutrale Depotbestände zum Stichtag und exakte Mengen; kennt weder Dateiformate noch Datenbank.
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSnapshotRow {
    pub symbol: String,
    pub market_symbol: String,
    pub isin: Option<String>,
    pub valor: Option<String>,
    pub quantity: String,
    pub category: String,
    pub asset_type: String,
    pub quote_currency: String,
    pub price_source: Option<String>,
    pub source_row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotScope {
    FullPortfolio,
    #[allow(dead_code)] // Extension point for providers exporting only selected holdings.
    Partial,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSnapshot {
    pub provider: String,
    pub format: String,
    pub snapshot_date: Option<String>,
    pub scope: SnapshotScope,
    pub account_reference: Option<String>,
    pub reference_is_shared: bool,
    pub positions: Vec<PositionSnapshotRow>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScaledQuantity {
    pub amount: i64,
    pub scale: u32,
}

impl PositionSnapshot {
    pub fn date(&self) -> Result<&str, String> {
        let date = self
            .snapshot_date
            .as_deref()
            .ok_or("Bitte den Stichtag des Positionsbestands angeben.")?;
        validate_date(date)?;
        Ok(date)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.snapshot_date.is_some() {
            self.date()?;
        }
        if self.provider.trim().is_empty() {
            return Err("Der Anbieter des Positionsbestands fehlt.".into());
        }
        let mut symbols = std::collections::BTreeSet::new();
        let mut identifiers = std::collections::BTreeSet::new();
        for row in &self.positions {
            if row.market_symbol.trim().is_empty() || !symbols.insert(row.market_symbol.clone()) {
                return Err(
                    "Die Instrumentzuordnung des Positionsbestands ist leer oder mehrdeutig."
                        .into(),
                );
            }
            parse_quantity(&row.quantity)?;
            for (kind, value) in [
                ("isin", row.isin.as_deref()),
                ("valor", row.valor.as_deref()),
            ] {
                if let Some(value) = value {
                    if value.trim().is_empty() || !identifiers.insert((kind, value)) {
                        return Err("Die Instrumentzuordnung des Positionsbestands ist leer oder mehrdeutig.".into());
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn validate_date(value: &str) -> Result<(), String> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Bitte einen gültigen Stichtag angeben.".to_string())?;
    if date.to_string() != value {
        return Err("Bitte einen gültigen Stichtag angeben.".into());
    }
    Ok(())
}

pub fn normalize_reference(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn parse_quantity(value: &str) -> Result<ScaledQuantity, String> {
    let mut normalized = value.trim().replace(['\'', '’', ' '], "").replace(',', ".");
    if normalized.starts_with('+') {
        normalized.remove(0);
    }
    if normalized.starts_with('-') {
        return Err("Die Positionsmenge darf nicht negativ sein.".into());
    }
    let parts = normalized.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts
            .first()
            .is_none_or(|part| part.is_empty() || !part.chars().all(|value| value.is_ascii_digit()))
        || parts
            .get(1)
            .is_some_and(|part| !part.chars().all(|value| value.is_ascii_digit()))
    {
        return Err(format!("Positionsmenge ist unlesbar: {value}"));
    }
    let mut fraction = parts.get(1).copied().unwrap_or_default().to_string();
    while fraction.ends_with('0') {
        fraction.pop();
    }
    if fraction.len() > 9 {
        return Err(
            "Positionsmengen mit mehr als neun Nachkommastellen werden nicht unterstützt.".into(),
        );
    }
    let scale = fraction.len() as u32;
    let digits = format!("{}{}", parts[0], fraction);
    let amount = digits
        .parse::<i64>()
        .map_err(|_| "Die Positionsmenge ist zu gross.".to_string())?;
    Ok(ScaledQuantity { amount, scale })
}

pub fn quantity_string(value: ScaledQuantity) -> String {
    if value.scale == 0 {
        return value.amount.to_string();
    }
    let digits = value.amount.to_string();
    let scale = value.scale as usize;
    if digits.len() <= scale {
        format!("0.{}{}", "0".repeat(scale - digits.len()), digits)
    } else {
        format!(
            "{}.{}",
            &digits[..digits.len() - scale],
            &digits[digits.len() - scale..]
        )
    }
}
