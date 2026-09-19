//! Zustandslose Klassifizierung von Kartenbuchungen und explizite Gutschriftentscheidungen.
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditDecision {
    pub transaction_id: i64,
    pub kind: String,
    pub category_key: Option<String>,
}

pub fn classify(
    card: bool,
    amount: i64,
    neutral: bool,
    settlement: bool,
    expense: i64,
) -> &'static str {
    if settlement {
        "SETTLEMENT"
    } else if neutral {
        "TRANSFER"
    } else if card && amount < 0 {
        "PURCHASE"
    } else if card && amount > 0 && expense < 0 {
        "REFUND"
    } else if card && amount > 0 {
        "UNKNOWN"
    } else {
        "REGULAR"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_preserves_priority_and_unknown_credits() {
        assert_eq!(classify(true, 100, true, true, 0), "SETTLEMENT");
        assert_eq!(classify(true, -100, true, false, 0), "TRANSFER");
        assert_eq!(classify(true, -100, false, false, 100), "PURCHASE");
        assert_eq!(classify(true, 100, false, false, -100), "REFUND");
        assert_eq!(classify(true, 100, false, false, 0), "UNKNOWN");
        assert_eq!(classify(false, 100, false, false, 0), "REGULAR");
        assert_eq!(classify(false, -100, false, false, 100), "REGULAR");
        assert_eq!(classify(false, -100, true, true, 0), "SETTLEMENT");
        assert_eq!(classify(false, -100, true, false, 0), "TRANSFER");
    }
}
