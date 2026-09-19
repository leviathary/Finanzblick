//! Lokale SQLite-Persistenz und Initialisierung des aktuellen Datenmodells.
pub(crate) mod banking;
#[cfg(test)]
pub(crate) use banking::transactions::analyze_transactions;
pub(crate) mod database;
pub(crate) mod imports;
pub(crate) mod reporting;
#[cfg(test)]
use database::schema;
pub(crate) use database::Storage;
pub(crate) mod securities;
#[cfg(test)]
use crate::domain::banking::accounts::default_include_in_net_worth;
#[cfg(test)]
use banking::accounts::accounts_from;

#[cfg(test)]
use banking::transactions::{query_transaction_transfers, transaction_history};
pub(crate) use imports::models::{SaveImportRequest, SaveImportResult};
#[cfg(test)]
pub(crate) use imports::*;
pub(crate) use reporting::models::WealthHistoryPoint;

#[cfg(test)]
use reporting::{dashboard_from, wealth_from};
#[cfg(test)]
use rules::categorization::apply_categories;
#[cfg(test)]
pub(crate) use schema::initialize_schema;
pub(crate) use securities::positions::decimal_value;
#[cfg(test)]
use securities::positions::rebuild_daily_valuations;
pub(crate) mod assistant;
pub mod categories;
#[cfg(test)]
mod tests;
use reporting::consumption;
pub(crate) mod rules;
pub(crate) use rules::settlement_rules;

#[cfg(test)]
use rusqlite::{params, Connection};

#[cfg(test)]
use std::{collections::BTreeMap, collections::HashSet, fs};

pub(crate) use database::errors::db_error;
#[cfg(test)]
use database::queries::count;
pub(crate) mod taxes;
