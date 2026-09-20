//! Liefert aggregierte Vermögens- und Übersichtsprojektionen.
use crate::domain::banking::accounts::{account_type_label, asset_type_label};
use crate::storage::banking::accounts::accounts_on;
use crate::storage::banking::models::ManagedAccount;
use crate::storage::database::Storage;
use crate::storage::database::{errors::db_error, queries::count};
use crate::storage::reporting::models::{
    DashboardAccount, DashboardData, DashboardProvider, DatabaseStatus, RecentImport,
    WealthBreakdown, WealthData, WealthHistoryPoint,
};
use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) fn database_status(storage: &Storage) -> Result<DatabaseStatus, String> {
    let connection = storage.connect().map_err(db_error)?;
    Ok(DatabaseStatus {
        path: connection.path().unwrap_or_default().to_string(),
        accounts: count(&connection, "accounts")?,
        imports: count(&connection, "import_runs")?,
        transactions: count(&connection, "transactions")?,
    })
}

pub(crate) fn dashboard_data(storage: &Storage) -> Result<DashboardData, String> {
    dashboard_from(&storage)
}

pub(crate) fn wealth_data(
    storage: &Storage,
    account_ids: Option<Vec<i64>>,
) -> Result<WealthData, String> {
    wealth_from(&storage, account_ids.as_deref())
}

pub(crate) fn wealth_from(
    storage: &Storage,
    account_ids: Option<&[i64]>,
) -> Result<WealthData, String> {
    let connection = storage.connect().map_err(db_error)?;
    wealth_on(&connection, account_ids)
}

pub(crate) fn wealth_on(
    connection: &Connection,
    account_ids: Option<&[i64]>,
) -> Result<WealthData, String> {
    let all_accounts = accounts_on(connection)?;
    let excluded_account_count = all_accounts
        .iter()
        .filter(|account| !account.is_active || !account.include_in_net_worth)
        .count();
    let available_accounts: Vec<ManagedAccount> = all_accounts
        .into_iter()
        .filter(|account| account.is_active && account.include_in_net_worth)
        .collect();
    let accounts: Vec<ManagedAccount> = available_accounts
        .iter()
        .filter(|account| account_ids.is_none_or(|ids| ids.contains(&account.id)))
        .cloned()
        .collect();
    let current_total_minor = accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
        .filter_map(|account| account.balance_minor)
        .sum();
    let regular_accounts = accounts
        .iter()
        .filter(|account| !["manual_asset", "pillar3a"].contains(&account.account_type.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let mut by_type = breakdown(&regular_accounts, |account| {
        (
            account.account_type.clone(),
            account_type_label(&account.account_type).to_string(),
        )
    });
    let selected_account_ids = accounts
        .iter()
        .map(|account| account.id)
        .collect::<HashSet<_>>();
    let mut manual_type_accounts = HashMap::<String, HashSet<i64>>::new();
    let mut manual_values = HashMap::<(String, String), i64>::new();
    {
        let mut statement = connection.prepare(
            "SELECT p.account_id,a.account_type,p.asset_type,d.value_minor
             FROM portfolio_positions p
             JOIN accounts a ON a.id=p.account_id
             JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest
               WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime')
               ORDER BY latest.valuation_date DESC LIMIT 1)
             WHERE d.currency='CHF' AND date(p.holding_start_date)<=date('now','localtime')
               AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))"
        ).map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(db_error)?;
        for row in rows {
            let (account_id, account_type, asset_type, value_minor) = row.map_err(db_error)?;
            if !selected_account_ids.contains(&account_id) {
                continue;
            }
            let (key, label) = if account_type == "pillar3a" {
                (
                    "pillar3a".to_string(),
                    account_type_label("pillar3a").to_string(),
                )
            } else {
                (
                    format!("manual_asset_{asset_type}"),
                    asset_type_label(&asset_type).to_string(),
                )
            };
            *manual_values.entry((key.clone(), label)).or_default() += value_minor;
            manual_type_accounts
                .entry(key)
                .or_default()
                .insert(account_id);
        }
    }
    for ((key, label), amount_minor) in manual_values {
        by_type.push(WealthBreakdown {
            key: key.clone(),
            label,
            amount_minor,
            account_count: manual_type_accounts
                .get(&key)
                .map_or(0, |ids| ids.len() as i64),
        });
    }
    by_type.sort_by_key(|item| std::cmp::Reverse(item.amount_minor));
    let by_provider = breakdown(&accounts, |account| {
        (account.provider_key.clone(), account.provider.clone())
    });

    let account_filter = account_ids.map_or(String::new(), |ids| {
        if ids.is_empty() {
            " AND 0".to_string()
        } else {
            format!(
                " AND a.id IN ({})",
                ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
            )
        }
    });
    let history_query = format!(
        "SELECT 'account:' || bs.account_id AS source_key,bs.balance_date,bs.amount_minor
         FROM balance_snapshots bs JOIN accounts a ON a.id=bs.account_id
         WHERE a.account_type NOT IN ('manual_asset','pillar3a') AND a.is_active=1
           AND a.include_in_net_worth=1 AND a.currency='CHF'
           AND date(bs.balance_date)<=date('now','localtime'){account_filter}
         UNION ALL
         SELECT 'position:' || d.position_id,d.valuation_date,d.value_minor
         FROM daily_valuations d JOIN portfolio_positions p ON p.id=d.position_id
         JOIN accounts a ON a.id=p.account_id
         WHERE a.is_active=1 AND a.include_in_net_worth=1
           AND d.currency='CHF' AND date(d.valuation_date)<=date('now','localtime'){account_filter}
         UNION ALL
         SELECT 'position:' || p.id,date(p.holding_end_date,'+1 day'),0
         FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id
         WHERE p.holding_end_date IS NOT NULL AND a.is_active=1
           AND a.include_in_net_worth=1
           AND EXISTS(SELECT 1 FROM daily_valuations ended
             WHERE ended.position_id=p.id AND ended.currency='CHF')
           AND date(p.holding_end_date,'+1 day')<=date('now','localtime'){account_filter}
         ORDER BY 2,1"
    );
    let mut statement = connection.prepare(&history_query).map_err(db_error)?;
    let snapshots = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut dates: BTreeMap<String, Vec<(String, i64)>> = BTreeMap::new();
    for (source_key, date, amount) in snapshots {
        let day = date.get(..10).unwrap_or(&date).to_string();
        dates.entry(day).or_default().push((source_key, amount));
    }
    let mut latest = HashMap::<String, i64>::new();
    let mut history = Vec::new();
    if let (Some(first), Some(last)) = (dates.keys().next(), dates.keys().next_back()) {
        let mut day = NaiveDate::parse_from_str(first, "%Y-%m-%d")
            .map_err(|error| format!("Ungültiges Bewertungsdatum: {error}"))?;
        let last = NaiveDate::parse_from_str(last, "%Y-%m-%d")
            .map_err(|error| format!("Ungültiges Bewertungsdatum: {error}"))?;
        while day <= last {
            let date = day.format("%Y-%m-%d").to_string();
            if let Some(values) = dates.get(&date) {
                for (source_key, amount) in values {
                    latest.insert(source_key.clone(), *amount);
                }
            }
            history.push(WealthHistoryPoint {
                date,
                total_minor: latest.values().sum(),
            });
            day += Duration::days(1);
        }
    }
    let first_total_minor = history.first().map(|point| point.total_minor);
    let change_minor = first_total_minor.map(|first| current_total_minor - first);
    Ok(WealthData {
        currency: "CHF".to_string(),
        current_total_minor,
        first_total_minor,
        change_minor,
        history,
        by_type,
        by_provider,
        accounts: available_accounts,
        excluded_account_count,
    })
}

pub(crate) fn breakdown<F>(accounts: &[ManagedAccount], key_and_label: F) -> Vec<WealthBreakdown>
where
    F: Fn(&ManagedAccount) -> (String, String),
{
    let mut values = HashMap::<String, WealthBreakdown>::new();
    for account in accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
    {
        let (key, label) = key_and_label(account);
        let entry = values.entry(key.clone()).or_insert(WealthBreakdown {
            key,
            label,
            amount_minor: 0,
            account_count: 0,
        });
        entry.amount_minor += account.balance_minor.unwrap_or(0);
        entry.account_count += 1;
    }
    let mut result: Vec<_> = values.into_values().collect();
    result.sort_by_key(|item| std::cmp::Reverse(item.amount_minor));
    result
}

pub(crate) fn dashboard_from(storage: &Storage) -> Result<DashboardData, String> {
    let connection = storage.connect().map_err(db_error)?;
    let mut query = connection
        .prepare(
            "SELECT a.id, i.name, i.provider_key, a.name, a.account_type, a.currency,
                    CASE WHEN a.account_type IN ('manual_asset','pillar3a') THEN (SELECT SUM(d.value_minor) FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id AND date(p.holding_start_date)<=date('now','localtime') AND (p.holding_end_date IS NULL OR date(p.holding_end_date)>=date('now','localtime'))) WHEN bs.id IS NULL THEN NULL ELSE bs.amount_minor + COALESCE((
                      SELECT SUM(t.amount_minor) FROM transactions t
                      WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                        AND date(t.booking_date) <= date('now', 'localtime')
                        AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)
                    ), 0) END,
                    CASE WHEN a.account_type IN ('manual_asset','pillar3a') THEN (SELECT MAX(d.valuation_date) FROM portfolio_positions p JOIN daily_valuations d ON d.position_id=p.id WHERE p.account_id=a.id AND date(d.valuation_date)<=date('now','localtime')) WHEN bs.id IS NULL THEN NULL ELSE COALESCE((
                      SELECT MAX(t.booking_date) FROM transactions t
                      WHERE t.account_id = a.id AND date(t.booking_date) > date(bs.balance_date)
                        AND date(t.booking_date) <= date('now', 'localtime')
                        AND NOT EXISTS (SELECT 1 FROM ignored_duplicate_transactions ignored WHERE ignored.transaction_id=t.id)
                    ), bs.balance_date) END,
                    CASE WHEN a.account_type IN ('manual_asset','pillar3a') THEN COALESCE((SELECT d.currency FROM portfolio_positions p JOIN daily_valuations d ON d.id=(SELECT latest.id FROM daily_valuations latest WHERE latest.position_id=p.id AND date(latest.valuation_date)<=date('now','localtime') ORDER BY latest.valuation_date DESC LIMIT 1) WHERE p.account_id=a.id LIMIT 1),a.currency) ELSE a.currency END,
                    a.include_in_net_worth, i.logo_data_url
             FROM accounts a
             JOIN institutions i ON i.id = a.institution_id
             LEFT JOIN balance_snapshots bs ON bs.id = (
               SELECT latest.id FROM balance_snapshots latest
               WHERE latest.account_id = a.id
                 AND date(latest.balance_date) <= date('now', 'localtime')
               ORDER BY latest.balance_date DESC, latest.id DESC LIMIT 1
             )
             WHERE a.is_active = 1
             ORDER BY COALESCE(bs.amount_minor, 0) DESC, i.name, a.name",
        )
        .map_err(db_error)?;
    let accounts = query
        .query_map([], |row| {
            Ok(DashboardAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                provider_key: row.get(2)?,
                name: row.get(3)?,
                account_type: row.get(4)?,
                currency: row.get(5)?,
                balance_minor: row.get(6)?,
                balance_date: row.get(7)?,
                balance_currency: row.get(8)?,
                include_in_net_worth: row.get(9)?,
                logo_data_url: row.get(10)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut providers = Vec::<DashboardProvider>::new();
    for account in &accounts {
        if !account.include_in_net_worth {
            continue;
        }
        let chf_balance = if account.balance_currency == "CHF" {
            account.balance_minor.unwrap_or(0)
        } else {
            0
        };
        if let Some(provider) = providers
            .iter_mut()
            .find(|provider| provider.provider_key == account.provider_key)
        {
            provider.balance_minor += chf_balance;
            provider.account_count += 1;
        } else {
            providers.push(DashboardProvider {
                provider: account.provider.clone(),
                provider_key: account.provider_key.clone(),
                balance_minor: chf_balance,
                account_count: 1,
                logo_data_url: account.logo_data_url.clone(),
            });
        }
    }
    providers.sort_by_key(|provider| std::cmp::Reverse(provider.balance_minor));
    let recent_import = connection
        .query_row(
            "SELECT ir.source_name, i.name, a.name, ir.imported_at, ir.transaction_count
             FROM import_runs ir
             JOIN accounts a ON a.id = ir.account_id
             JOIN institutions i ON i.id = a.institution_id
             ORDER BY ir.imported_at DESC, ir.id DESC LIMIT 1",
            [],
            |row| {
                Ok(RecentImport {
                    source_name: row.get(0)?,
                    provider: row.get(1)?,
                    account_name: row.get(2)?,
                    imported_at: row.get(3)?,
                    transaction_count: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(db_error)?;
    let transaction_count = count(&connection, "transactions")?;
    let total_balance_minor = accounts
        .iter()
        .filter(|account| account.balance_currency == "CHF")
        .filter(|account| account.include_in_net_worth)
        .filter_map(|account| account.balance_minor)
        .sum();
    Ok(DashboardData {
        total_balance_minor,
        currency: "CHF".to_string(),
        accounts,
        providers,
        recent_import,
        transaction_count,
    })
}
