//! Datenbankgestützte Auswertungsprojektionen.
pub(crate) mod consumption;
pub(crate) mod account_details;
mod overview;
pub(crate) use overview::{dashboard_data, database_status, wealth_data, wealth_on};
#[cfg(test)]
pub(crate) use overview::{dashboard_from, wealth_from};
pub(crate) mod models;
