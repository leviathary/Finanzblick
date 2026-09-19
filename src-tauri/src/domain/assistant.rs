//! Anfrage- und Datenschutzregeln für den Finanzassistenten ohne Datenbankzugriff.
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub(crate) role: Role,
    pub(crate) content: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareRequest {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) question: String,
    pub(crate) history: Vec<Message>,
    #[serde(default)]
    pub(crate) include_details: bool,
    #[serde(default)]
    pub(crate) account_scope: AccountScope,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccountScope {
    #[default]
    Auto,
    All,
    CreditCards,
}

pub(crate) fn credit_card_question(question: &str) -> bool {
    let q = question.to_lowercase();
    [
        "kreditkart",
        "credit card",
        "creditcard",
        "credit-card",
        "carte de crédit",
        "carte de credit",
        "cartes de crédit",
        "carta di credito",
        "carte di credito",
    ]
    .iter()
    .any(|word| q.contains(word))
}

pub(crate) fn credit_card_scope(r: &PrepareRequest) -> bool {
    match r.account_scope {
        AccountScope::CreditCards => true,
        AccountScope::All => false,
        AccountScope::Auto => {
            credit_card_question(&r.question)
                || r.history
                    .iter()
                    .any(|m| matches!(m.role, Role::User) && credit_card_question(&m.content))
        }
    }
}

pub(crate) fn validate_request(r: &PrepareRequest) -> Result<(NaiveDate, NaiveDate), String> {
    let parse = |s: &str| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .filter(|d| d.format("%Y-%m-%d").to_string() == s)
    };
    let (Some(from), Some(to)) = (parse(&r.from), parse(&r.to)) else {
        return Err("Bitte einen gültigen Zeitraum auswählen.".into());
    };
    if from > to || (to - from).num_days() > 366 * 5 || to > Local::now().date_naive() {
        return Err("Bitte einen Zeitraum von höchstens fünf Jahren bis heute auswählen.".into());
    }
    if r.question.trim().is_empty()
        || r.question.len() > 4000
        || r.history.len() > 12
        || r.history.iter().enumerate().any(|(i, m)| {
            m.content.len() > 20_000
                || !matches!((&m.role, i % 2), (Role::User, 0) | (Role::Assistant, 1))
        })
        || r.history.len() % 2 != 0
    {
        return Err("Die Frage oder der Chat ist zu lang. Bitte einen neuen Chat beginnen.".into());
    }
    Ok((from, to))
}

// Never serialize user-defined category keys or labels: they may contain personal data.
pub(crate) fn safe_category(key: &str) -> &'static str {
    match key {
        "housing" => "housing",
        "furnishing" => "furnishing",
        "electronics" => "electronics",
        "groceries" => "groceries",
        "health" => "health",
        "leisure" => "leisure",
        "restaurants" => "restaurants",
        "telecom" => "telecom",
        "digital_subscriptions" => "digital_subscriptions",
        "transport" => "transport",
        "travel" => "travel",
        "taxes" => "taxes",
        "alimony" => "alimony",
        "saving" => "saving",
        "income" => "income",
        "other" => "other",
        _ => "custom_categories",
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Month {
    pub(crate) cash_in_minor: i64,
    pub(crate) cash_out_minor: i64,
    pub(crate) net_cash_flow_minor: i64,
    pub(crate) spending_minor: i64,
    pub(crate) spending_by_category_minor: BTreeMap<&'static str, i64>,
}
