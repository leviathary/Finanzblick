//! Sprachneutrale fachliche Typisierung neutraler Geldbewegungen.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum TransferType {
    None,
    CreditCardSettlement,
    InternalTransfer,
}

/// Editable conditions for one side of a neutral money movement.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransferRuleInput {
    pub id: Option<i64>,
    pub name: String,
    pub account_id: i64,
    pub currency: String,
    pub direction: i64,
    pub prefix: String,
    pub transfer_type: TransferType,
}

pub(crate) fn normalized_rule_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl TransferRuleInput {
    pub fn validate(&mut self) -> Result<(), String> {
        self.name = self.name.trim().into();
        self.prefix = normalized_rule_text(&self.prefix);
        self.currency = self.currency.trim().to_uppercase();
        if self.name.is_empty() || self.name.chars().count() > 120 {
            return Err("Bitte einen Regelnamen mit 1 bis 120 Zeichen eingeben.".into());
        }
        if self.prefix.chars().count() < 8 || self.prefix.len() > 2000 {
            return Err("Bitte einen passenden Textanfang mit mindestens 8 Zeichen wählen.".into());
        }
        if ![-1, 1].contains(&self.direction)
            || self.currency.len() != 3
            || !self.currency.bytes().all(|c| c.is_ascii_uppercase())
            || matches!(self.transfer_type, TransferType::None)
        {
            return Err("Bitte Konto, Währung, Zahlungsrichtung und Regeltyp prüfen.".into());
        }
        Ok(())
    }
    pub fn type_key(&self) -> &'static str {
        match self.transfer_type {
            TransferType::InternalTransfer => "INTERNAL_TRANSFER",
            _ => "CREDIT_CARD_SETTLEMENT",
        }
    }
}
