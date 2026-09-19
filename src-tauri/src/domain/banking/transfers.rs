//! Sprachneutrale fachliche Typisierung neutraler Geldbewegungen.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum TransferType {
    None,
    CreditCardSettlement,
    InternalTransfer,
}
