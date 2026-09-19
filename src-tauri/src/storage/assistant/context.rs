//! Liefert begrenzte, datensparsame Finanzprojektionen für den Assistenten.
use crate::domain::assistant::{
    credit_card_scope, safe_category, validate_request, Month, PrepareRequest,
};
use crate::storage::{banking::transactions::analyze_transactions, db_error, reporting::wealth_on};
use chrono::{Datelike, NaiveDate};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::BTreeMap;
const MAX_DETAIL_TRANSACTIONS: usize = 2_000;
pub(crate) fn aggregate(db: &Connection, r: &PrepareRequest) -> Result<Value, String> {
    let (from, to) = validate_request(r)?;
    crate::storage::reporting::consumption::prepare(db).map_err(db_error)?;
    if credit_card_scope(r) {
        return credit_card_aggregate(db, r, from, to);
    }
    let analysis = analyze_transactions(db, Some(r.from.clone()), Some(r.to.clone()), None, None)?;
    let wealth = wealth_on(db, None)?;
    let mut months = BTreeMap::<String, Month>::new();
    let mut year = from.year();
    let mut month = from.month();
    while (year, month) <= (to.year(), to.month()) {
        months.insert(format!("{year:04}-{month:02}"), Month::default());
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    for row in analysis
        .transactions
        .iter()
        .chain(&analysis.income_transactions)
    {
        let Some(month) = row.booking_date.get(..7).and_then(|m| months.get_mut(m)) else {
            continue;
        };
        if !row.is_card && !row.excluded_from_totals {
            if row.amount_minor > 0 {
                month.cash_in_minor += row.amount_minor;
            } else {
                month.cash_out_minor -= row.amount_minor;
            }
            month.net_cash_flow_minor += row.amount_minor;
        }
        if row.expense_minor != 0 {
            month.spending_minor += row.expense_minor;
            *month
                .spending_by_category_minor
                .entry(safe_category(&row.category_key))
                .or_default() += row.expense_minor;
        }
    }
    let mut categories = BTreeMap::<&str, i64>::new();
    for category in &analysis.categories {
        *categories.entry(safe_category(&category.key)).or_default() += category.amount_minor;
    }
    let selected: Vec<_> = wealth
        .history
        .iter()
        .filter(|p| p.date >= r.from && p.date <= r.to)
        .collect();
    let mut monthly_wealth = BTreeMap::new();
    for point in &selected {
        monthly_wealth.insert(&point.date[..7], *point);
    }
    let first = selected.first().copied();
    let last = selected.last().copied();
    let mut data = json!({
        "accountScope": "all", "period": {"from": r.from, "to": r.to}, "currency": "CHF", "unit": "hundredths of CHF",
        "cashFlow": {"source":"Geldfluss", "inMinor":months.values().map(|m|m.cash_in_minor).sum::<i64>(),"outMinor":months.values().map(|m|m.cash_out_minor).sum::<i64>(),
            "netMinor":months.values().map(|m|m.net_cash_flow_minor).sum::<i64>(),"firstBookingDate":analysis.first_date,"lastBookingDate":analysis.last_date},
        "spending": {"source":"Ausgaben", "totalMinor":categories.values().sum::<i64>(),"byCategoryMinor":categories},
        "hasNeutralizedSettlements":analysis.transactions.iter().chain(&analysis.income_transactions).any(|r|r.is_card_settlement),
        "unresolvedCardCredits":analysis.transactions.iter().filter(|r|r.is_card && r.amount_minor>0 && !r.excluded_from_totals && r.expense_minor==0).count(),
        "months":months,
        "wealth": {"source":"Vermögen", "first":first,"last":last,"changeMinor":first.zip(last).map(|(a,b)|b.total_minor-a.total_minor),
            "monthlyLastKnownValues":monthly_wealth.values().collect::<Vec<_>>(),
            "currentlyExcludedAccounts":wealth.excluded_account_count,
            "currentlyUnvaluedOrNonChfAccounts":wealth.accounts.iter().filter(|a|a.balance_minor.is_none() || a.balance_currency != "CHF").count()}
    });
    data["mode"] = json!(if r.include_details {
        "details"
    } else {
        "summary"
    });
    if r.include_details {
        let rows = detail_transactions(db, r)?;
        data["detailCoverage"] = json!({"count": rows.len(), "complete": true, "scope": "nonzero transactions of active accounts; all currencies; selected period only"});
        data["detailTransactions"] = json!(rows);
    }
    Ok(data)
}

// A separate query and snapshot prevent unrelated balances or income leaking through aggregates.
pub(crate) fn credit_card_aggregate(
    db: &Connection,
    r: &PrepareRequest,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Value, String> {
    let mut query = db
        .prepare(
            "SELECT t.id,t.booking_date,t.amount_minor,t.currency,
        COALESCE(c.label,'Ohne Kategorie'),COALESCE(c.category_key,'uncategorized'),t.expense_minor,t.is_settlement,t.exclude_from_cashflow
        FROM reporting_transactions t JOIN accounts a ON a.id=t.account_id
        LEFT JOIN categories c ON c.id=t.category_id
        WHERE a.is_active=1 AND a.account_type='credit_card' AND t.amount_minor<>0
        AND t.booking_date>=?1 AND t.booking_date<=?2 ORDER BY t.booking_date,t.id",
        )
        .map_err(db_error)?;
    let mut rows = query
        .query(rusqlite::params![r.from, r.to])
        .map_err(db_error)?;
    let mut details = Vec::new();
    let mut count = 0;
    let mut months = BTreeMap::<String, Value>::new();
    let (mut year, mut month) = (from.year(), from.month());
    while (year, month) <= (to.year(), to.month()) {
        months.insert(
            format!("{year:04}-{month:02}"),
            json!({"debitsMinor":0,"creditsMinor":0,"byCategoryMinor":{}}),
        );
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    let mut debits = 0i64;
    let mut credits = 0i64;
    let mut unresolved_credits = 0usize;
    let mut categories = BTreeMap::<&str, i64>::new();
    while let Some(row) = rows.next().map_err(db_error)? {
        count += 1;
        let date: String = row.get(1).map_err(db_error)?;
        let amount: i64 = row.get(2).map_err(db_error)?;
        let currency: String = row.get(3).map_err(db_error)?;
        let expense: i64 = row.get(6).map_err(db_error)?;
        if amount > 0 && expense == 0 && !row.get::<_, bool>(8).map_err(db_error)? {
            unresolved_credits += 1;
        }
        if currency == "CHF" {
            let key: String = row.get(5).map_err(db_error)?;
            let category = safe_category(&key);
            if amount < 0 {
                debits -= amount;
            } else {
                credits += amount;
            }
            *categories.entry(category).or_default() += expense;
            if let Some(m) = date.get(..7).and_then(|key| months.get_mut(key)) {
                let field = if amount < 0 {
                    "debitsMinor"
                } else {
                    "creditsMinor"
                };
                m[field] = json!(m[field].as_i64().unwrap_or(0) + amount.abs());
                m["byCategoryMinor"][category] =
                    json!(m["byCategoryMinor"][category].as_i64().unwrap_or(0) + expense);
            }
        }
        if r.include_details {
            if count > MAX_DETAIL_TRANSACTIONS {
                return Err("Zu viele Detailtransaktionen. Bitte einen kürzeren Zeitraum wählen (maximal 2000 Buchungen).".into());
            }
            details.push(json!({"id":row.get::<_,i64>(0).map_err(db_error)?,"bookingDate":date,
                "amountMinor":amount,"currency":currency,"categoryLabel":row.get::<_,String>(4).map_err(db_error)?,
                "isCardSettlement":row.get::<_,bool>(7).map_err(db_error)?,
                "excludedFromChfCashFlow":true,"excludedFromChfSpending":currency!="CHF"||expense==0}));
        }
    }
    let mut data = json!({"accountScope":"credit_cards","period":{"from":r.from,"to":r.to},
        "currency":"CHF","unit":"hundredths of CHF","mode":if r.include_details {"details"} else {"summary"},
        "spending":{"source":"Ausgaben","totalMinor":categories.values().sum::<i64>(),"byCategoryMinor":categories},
        "creditCardActivity":{"debitsMinor":debits,"creditsMinor":credits},"unresolvedCardCredits":unresolved_credits,"months":months});
    if r.include_details {
        data["detailCoverage"] = json!({"count":count,"complete":true,"scope":"nonzero transactions of active credit-card accounts only; all currencies; selected period only"});
        data["detailTransactions"] = json!(details);
    }
    Ok(data)
}

pub(crate) fn detail_transactions(
    db: &Connection,
    r: &PrepareRequest,
) -> Result<Vec<Value>, String> {
    // The reporting view is prepared by aggregate above. Bind all
    // date parameters; never accept SQL or a filter expression from the model.
    let mut query = db
        .prepare(
            "SELECT t.id,t.booking_date,t.amount_minor,t.currency,
                COALESCE(c.label,'Ohne Kategorie'),a.account_type,
                t.is_settlement,t.expense_minor,t.exclude_from_cashflow
         FROM reporting_transactions t JOIN accounts a ON a.id=t.account_id
         LEFT JOIN categories c ON c.id=t.category_id
         WHERE a.is_active=1 AND t.amount_minor<>0 AND t.booking_date>=?1 AND t.booking_date<=?2
         ORDER BY t.booking_date,t.id LIMIT ?3",
        )
        .map_err(db_error)?;
    // Explicit allowlist: sensitive source fields are not even selected for serialization.
    let rows = query
        .query_map(
            rusqlite::params![r.from, r.to, MAX_DETAIL_TRANSACTIONS + 1],
            |row| {
                let amount: i64 = row.get(2)?;
                let currency: String = row.get(3)?;
                let account_type: String = row.get(5)?;
                let settlement: bool = row.get(6)?;
                Ok(json!({
                    "id":row.get::<_,i64>(0)?, "bookingDate":row.get::<_,String>(1)?,
                    "amountMinor":amount, "currency":currency,
                    "categoryLabel":row.get::<_,String>(4)?,
                    "isCardSettlement":settlement,
                    "excludedFromChfCashFlow": currency != "CHF" || account_type == "credit_card" || row.get::<_,bool>(8)?,
                    "excludedFromChfSpending": currency != "CHF" || row.get::<_,i64>(7)? == 0
                }))
            },
        )
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    if rows.len() > MAX_DETAIL_TRANSACTIONS {
        return Err("Zu viele Detailtransaktionen. Bitte einen kürzeren Zeitraum wählen (maximal 2000 Buchungen).".into());
    }
    Ok(rows)
}
