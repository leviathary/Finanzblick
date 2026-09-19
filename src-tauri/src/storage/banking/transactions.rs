//! Liest Buchungen und Auswertungen und persistiert explizite Einordnungen.
use crate::storage::banking::models::{AnalyzedTransaction, TransferRow};
use crate::storage::database::errors::db_error;
use crate::storage::database::Storage;
use crate::storage::reporting::consumption;
use crate::storage::reporting::models::{
    CategorySpend, DashboardProvider, MonthlySpend, TransactionAnalysis, TransactionHistoryPoint,
};
use crate::storage::rules::merchant_rules;
use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashMap};

pub(crate) fn transaction_analysis(
    storage: &Storage,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
    provider_keys: Vec<String>,
    account_ids: Vec<i64>,
) -> Result<TransactionAnalysis, String> {
    let connection = storage.connect().map_err(db_error)?;
    analyze_transactions_filtered(&connection, from, to, provider_key, account_id, provider_keys, account_ids)
}

pub(crate) fn analyze_transactions(
    connection: &Connection,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
) -> Result<TransactionAnalysis, String> {
    analyze_transactions_filtered(connection, from, to, provider_key, account_id, vec![], vec![])
}

pub(crate) fn analyze_transactions_filtered(
    connection: &Connection,
    from: Option<String>,
    to: Option<String>,
    provider_key: Option<String>,
    account_id: Option<i64>,
    provider_keys: Vec<String>,
    account_ids: Vec<i64>,
) -> Result<TransactionAnalysis, String> {
    consumption::prepare(connection).map_err(db_error)?;
    // Multi-select analysis has its own filters; bank balances use their dedicated command.
    let history = if provider_keys.is_empty() && account_ids.is_empty() {
        transaction_history(connection, &provider_key, account_id).map_err(db_error)?
    } else { vec![] };
    let providers_json = serde_json::to_string(&provider_keys).map_err(|e| e.to_string())?;
    let accounts_json = serde_json::to_string(&account_ids).map_err(|e| e.to_string())?;
    let mut query = connection
        .prepare(
            "SELECT t.id,t.booking_date,t.description,t.industry,t.amount_minor,t.currency,
         COALESCE(c.category_key,'other'),COALESCE(c.label,'Sonstiges'),COALESCE(c.color,'#888888'),
         CASE WHEN t.category_manual=1 THEN 'manual' ELSE t.category_source END,
         i.name,i.provider_key,a.name,t.exclude_from_cashflow,t.is_settlement,t.is_card,
         t.is_manually_overridden,t.expense_minor,t.income_minor
         FROM reporting_transactions t JOIN accounts a ON a.id=t.account_id
         JOIN institutions i ON i.id=a.institution_id LEFT JOIN categories c ON c.id=t.category_id
         WHERE a.is_active=1 AND t.currency='CHF' AND t.amount_minor<>0
         AND (?1 IS NULL OR t.booking_date>=?1) AND (?2 IS NULL OR t.booking_date<=?2)
         AND (?3 IS NULL OR i.provider_key=?3) AND (?4 IS NULL OR a.id=?4)
         AND (json_array_length(?5)=0 OR i.provider_key IN (SELECT value FROM json_each(?5)))
         AND (json_array_length(?6)=0 OR a.id IN (SELECT value FROM json_each(?6)))
         ORDER BY t.booking_date DESC,t.id DESC",
        )
        .map_err(db_error)?;
    let rows = query
        .query_map(params![from, to, provider_key, account_id, providers_json, accounts_json], |r| {
            Ok(AnalyzedTransaction {
                id: r.get(0)?,
                booking_date: r.get(1)?,
                description: r.get(2)?,
                industry: r.get(3)?,
                amount_minor: r.get(4)?,
                currency: r.get(5)?,
                category_key: r.get(6)?,
                category_label: r.get(7)?,
                category_color: r.get(8)?,
                category_source: r.get(9)?,
                provider: r.get(10)?,
                provider_key: r.get(11)?,
                account_name: r.get(12)?,
                excluded_from_totals: r.get(13)?,
                is_card_settlement: r.get(14)?,
                is_card: r.get(15)?,
                is_manually_overridden: r.get(16)?,
                expense_minor: r.get(17)?,
                income_minor: r.get(18)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let first_date = rows.iter().map(|r| r.booking_date.clone()).min();
    let last_date = rows.iter().map(|r| r.booking_date.clone()).max();
    let total_income_minor = rows.iter().map(|r| r.income_minor).sum();
    let income_count = rows.iter().filter(|r| r.income_minor != 0).count() as i64;
    let total_spend_minor = rows.iter().map(|r| r.expense_minor).sum();
    let transaction_count = rows.iter().filter(|r| r.expense_minor != 0).count() as i64;
    let mut groups = BTreeMap::<String, CategorySpend>::new();
    let mut monthly = BTreeMap::<String, i64>::new();
    for r in &rows {
        if r.expense_minor == 0 {
            continue;
        }
        let group = groups
            .entry(r.category_key.clone())
            .or_insert_with(|| CategorySpend {
                key: r.category_key.clone(),
                label: r.category_label.clone(),
                color: r.category_color.clone(),
                amount_minor: 0,
                transaction_count: 0,
            });
        group.amount_minor += r.expense_minor;
        group.transaction_count += 1;
        *monthly.entry(r.booking_date[..7].to_string()).or_default() += r.expense_minor;
    }
    let mut categories: Vec<_> = groups.into_values().collect();
    categories.sort_by(|a, b| b.amount_minor.cmp(&a.amount_minor).then(a.key.cmp(&b.key)));
    let months = monthly
        .into_iter()
        .map(|(month, amount_minor)| MonthlySpend {
            month,
            amount_minor,
        })
        .collect();
    // Retain excluded rows for manual reversal. Positive card credits belong to
    // spending as refunds unless explicitly marked as settlement.
    let (income_transactions, transactions) = rows
        .into_iter()
        .partition(|r| r.amount_minor > 0 && !r.is_card);
    let providers = {
        let mut query = connection.prepare("SELECT i.name,i.provider_key,COUNT(a.id) FROM institutions i JOIN accounts a ON a.institution_id=i.id WHERE a.is_active=1 GROUP BY i.id ORDER BY i.name").map_err(db_error)?;
        let providers = query
            .query_map([], |row| {
                Ok(DashboardProvider {
                    provider: row.get(0)?,
                    provider_key: row.get(1)?,
                    account_count: row.get(2)?,
                    balance_minor: 0,
                    logo_data_url: None,
                })
            })
            .map_err(db_error)?;
        let result = providers.collect::<Result<Vec<_>, _>>().map_err(db_error)?;
        result
    };

    Ok(TransactionAnalysis {
        total_income_minor,
        income_count,
        income_transactions,
        total_spend_minor,
        transaction_count,
        first_date,
        last_date,
        categories,
        months,
        history,
        transactions,
        providers,
    })
}

pub(crate) fn bank_balance_history(
    storage: &Storage,
    provider: Option<String>,
    account: Option<i64>,
) -> Result<Vec<TransactionHistoryPoint>, String> {
    let db = storage.connect().map_err(db_error)?;
    transaction_history(&db, &provider, account).map_err(db_error)
}

pub(crate) fn transaction_history(
    db: &Connection,
    provider: &Option<String>,
    account: Option<i64>,
) -> rusqlite::Result<Vec<TransactionHistoryPoint>> {
    let mut query = db.prepare(
        "SELECT 'account:' || bs.account_id,bs.balance_date,bs.amount_minor
         FROM balance_snapshots bs
         JOIN accounts a ON a.id=bs.account_id
         JOIN institutions i ON i.id=a.institution_id
         WHERE bs.currency='CHF' AND a.is_active=1
           AND date(bs.balance_date)<=date('now','localtime')
           AND (?1 IS NULL OR i.provider_key=?1)
           AND (?2 IS NULL OR a.id=?2)
           AND a.account_type IN ('cash','savings')
           AND bs.id=(SELECT latest.id FROM balance_snapshots latest
             WHERE latest.account_id=bs.account_id AND latest.balance_date=bs.balance_date
             ORDER BY latest.id DESC LIMIT 1)
         ORDER BY bs.balance_date,bs.account_id",
    )?;
    let snapshots = query
        .query_map(params![provider, account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut dates = BTreeMap::<String, Vec<(String, i64)>>::new();
    for (source, date, amount) in snapshots {
        dates.entry(date).or_default().push((source, amount));
    }
    let (Some(first), Some(last)) = (dates.keys().next(), dates.keys().next_back()) else {
        return Ok(Vec::new());
    };
    let mut day = NaiveDate::parse_from_str(first, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let last = NaiveDate::parse_from_str(last, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let mut latest = HashMap::<String, i64>::new();
    let mut history = Vec::new();
    while day <= last {
        let date = day.format("%Y-%m-%d").to_string();
        if let Some(values) = dates.get(&date) {
            for (source, amount) in values {
                latest.insert(source.clone(), *amount);
            }
        }
        history.push(TransactionHistoryPoint {
            date,
            total_minor: latest.values().sum(),
        });
        day += Duration::days(1);
    }
    Ok(history)
}

pub(crate) fn set_transaction_transfers(
    storage: &Storage,
    transaction_ids: Vec<i64>,
    transfer_type: crate::domain::banking::transfers::TransferType,
) -> Result<(), String> {
    let mut db = storage.connect().map_err(db_error)?;
    crate::storage::banking::reporting_flags::set_transfers(
        &mut db,
        &transaction_ids,
        transfer_type,
    )
}

pub(crate) fn list_transaction_transfers(
    storage: &Storage,
    account_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    search: String,
) -> Result<Vec<TransferRow>, String> {
    let db = storage.connect().map_err(db_error)?;
    query_transaction_transfers(&db, account_id, from, to, search)
}

pub(crate) fn query_transaction_transfers(
    db: &Connection,
    account_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    search: String,
) -> Result<Vec<TransferRow>, String> {
    consumption::prepare(&db).map_err(db_error)?;
    let mut query = db.prepare(
        "SELECT t.id,t.booking_date,a.name,t.description,t.amount_minor,t.currency,t.is_settlement,
         m.counterparty_name,m.remittance_information,i.name
         FROM reporting_transactions t JOIN accounts a ON a.id=t.account_id
         JOIN institutions i ON i.id=a.institution_id
         LEFT JOIN transaction_metadata m ON m.transaction_id=t.id
         WHERE t.exclude_from_cashflow=1
         AND (?1 IS NULL OR a.id=?1) AND (?2 IS NULL OR t.booking_date>=?2) AND (?3 IS NULL OR t.booking_date<=?3)
         ORDER BY t.booking_date DESC,t.id DESC"
    ).map_err(db_error)?;
    let rows = query
        .query_map(params![account_id, from, to], |r| {
            Ok(TransferRow {
                id: r.get(0)?,
                booking_date: r.get(1)?,
                account_name: r.get(2)?,
                description: r.get(3)?,
                amount_minor: r.get(4)?,
                currency: r.get(5)?,
                transfer_type: if r.get::<_, bool>(6)? {
                    crate::domain::banking::transfers::TransferType::CreditCardSettlement
                } else {
                    crate::domain::banking::transfers::TransferType::InternalTransfer
                },
                counterparty_name: r.get(7)?,
                remittance_information: r.get(8)?,
                provider: r.get(9)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let search = search.trim().to_lowercase();
    Ok(rows
        .into_iter()
        .filter(|r| {
            search.is_empty()
                || format!(
                    "{} {} {} {} {}",
                    r.description,
                    r.account_name,
                    r.counterparty_name.as_deref().unwrap_or(""),
                    r.remittance_information.as_deref().unwrap_or(""),
                    r.provider
                )
                .to_lowercase()
                .contains(&search)
        })
        .collect())
}

pub(crate) fn set_transaction_settlement(
    storage: &Storage,
    transaction_id: i64,
    is_settlement: bool,
) -> Result<(), String> {
    let connection = storage.connect().map_err(db_error)?;
    crate::storage::banking::reporting_flags::set_settlement(
        &connection,
        transaction_id,
        is_settlement,
    )
}

pub(crate) fn set_transaction_category(
    storage: &Storage,
    transaction_id: i64,
    category_key: String,
) -> Result<usize, String> {
    let mut connection = storage.connect().map_err(db_error)?;
    merchant_rules::learn(&mut connection, transaction_id, &category_key)
}
