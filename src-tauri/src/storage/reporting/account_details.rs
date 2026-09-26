//! Rein lesende Konto- und Depotdetails, unabhängig vom Einbezug ins Gesamtvermögen.
use crate::domain::banking::accounts::supports_positions;
use crate::storage::banking::{accounts::accounts_on, models::ManagedAccount};
use crate::storage::database::{errors::db_error, Storage};
use crate::storage::reporting::models::WealthHistoryPoint;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetails {
    account: ManagedAccount,
    positions: Vec<PositionDetail>,
    history: Vec<WealthHistoryPoint>,
    history_currency: String,
    transactions: Vec<AccountBooking>,
    has_more: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionDetail {
    id: i64,
    label: String,
    symbol: Option<String>,
    quantity: Option<f64>,
    price_minor: Option<f64>,
    price_currency: Option<String>,
    value_minor: Option<i64>,
    value_currency: Option<String>,
    valuation_date: Option<String>,
    start: String,
    end: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountBooking {
    id: i64,
    date: String,
    description: String,
    amount_minor: i64,
    currency: String,
}

pub(crate) fn account_details(
    storage: &Storage,
    account_id: i64,
    limit: usize,
) -> Result<AccountDetails, String> {
    let connection = storage.connect().map_err(db_error)?;
    read_details(&connection, account_id, limit).map_err(db_error)
}

fn read_details(db: &Connection, id: i64, limit: usize) -> rusqlite::Result<AccountDetails> {
    let account = accounts_on(db)
        .map_err(rusqlite::Error::InvalidParameterName)?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let is_depot = supports_positions(&account.account_type);
    let mut positions = Vec::new();
    if is_depot {
        let mut query = db.prepare(
            "SELECT p.id,p.label,COALESCE(l.market_symbol,(SELECT identifier FROM instrument_identifiers WHERE instrument_id=l.instrument_id ORDER BY id LIMIT 1)),
             q.quantity_amount,q.quantity_scale,ip.price_amount,ip.price_scale,ip.currency,
             d.value_minor,d.currency,d.valuation_date,p.holding_start_date,p.holding_end_date
             FROM portfolio_positions p
             LEFT JOIN instrument_listings l ON l.id=p.listing_id
             LEFT JOIN position_quantities q ON q.id=(SELECT id FROM position_quantities WHERE position_id=p.id AND valid_from<=date('now','localtime') ORDER BY valid_from DESC LIMIT 1)
             LEFT JOIN daily_valuations d ON d.id=(SELECT id FROM daily_valuations WHERE position_id=p.id AND valuation_date>=p.holding_start_date AND (p.holding_end_date IS NULL OR valuation_date<=p.holding_end_date) AND valuation_date<=date('now','localtime') ORDER BY valuation_date DESC LIMIT 1)
             LEFT JOIN instrument_prices ip ON ip.id=d.instrument_price_id
             WHERE p.account_id=?1 ORDER BY p.label,p.id")?;
        positions = query
            .query_map([id], |r| {
                let decimal = |amount: usize, scale: usize| -> rusqlite::Result<Option<f64>> {
                    Ok(r.get::<_, Option<i64>>(amount)?
                        .zip(r.get::<_, Option<i32>>(scale)?)
                        .map(|(a, s)| a as f64 / 10_f64.powi(s)))
                };
                Ok(PositionDetail {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    symbol: r.get(2)?,
                    quantity: decimal(3, 4)?,
                    price_minor: decimal(5, 6)?.map(|p| p * 100.0),
                    price_currency: r.get(7)?,
                    value_minor: r.get(8)?,
                    value_currency: r.get(9)?,
                    valuation_date: r.get(10)?,
                    start: r.get(11)?,
                    end: r.get(12)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    let history_currency = if is_depot {
        "CHF".to_string()
    } else {
        account.currency.clone()
    };
    // Each date uses the latest balance evidence plus subsequent non-duplicate bookings.
    // Depot history includes an explicit zero after an end date, never before inception.
    let sql = if is_depot {
        "WITH changes AS (
          SELECT d.position_id,d.valuation_date day,d.value_minor value FROM daily_valuations d JOIN portfolio_positions p ON p.id=d.position_id
          WHERE p.account_id=?1 AND d.currency=?2 AND d.valuation_date>=p.holding_start_date AND (p.holding_end_date IS NULL OR d.valuation_date<=p.holding_end_date)
          UNION ALL SELECT p.id,date(p.holding_end_date,'+1 day'),0 FROM portfolio_positions p WHERE p.account_id=?1 AND p.holding_end_date IS NOT NULL AND EXISTS(SELECT 1 FROM daily_valuations d WHERE d.position_id=p.id AND d.currency=?2)
        ), deltas AS (
          SELECT day,value-COALESCE(LAG(value) OVER(PARTITION BY position_id ORDER BY day),0) delta FROM changes WHERE day<=date('now','localtime')
        ), daily AS (SELECT day,SUM(delta) delta FROM deltas GROUP BY day)
        SELECT day,SUM(delta) OVER(ORDER BY day) FROM daily ORDER BY day"
    } else {
        "WITH days AS (SELECT balance_date day FROM balance_snapshots WHERE account_id=?1 AND currency=?2
          UNION SELECT booking_date FROM transactions WHERE account_id=?1 AND currency=?2 AND NOT EXISTS(SELECT 1 FROM ignored_duplicate_transactions i WHERE i.transaction_id=transactions.id)),
        evidence AS (SELECT day,(SELECT id FROM balance_snapshots WHERE account_id=?1 AND currency=?2 AND balance_date<=days.day ORDER BY balance_date DESC,id DESC LIMIT 1) snapshot FROM days WHERE day<=date('now','localtime'))
        SELECT e.day,b.amount_minor+COALESCE((SELECT SUM(t.amount_minor) FROM transactions t WHERE t.account_id=?1 AND t.currency=?2 AND t.booking_date>b.balance_date AND t.booking_date<=e.day AND NOT EXISTS(SELECT 1 FROM ignored_duplicate_transactions i WHERE i.transaction_id=t.id)),0)
        FROM evidence e JOIN balance_snapshots b ON b.id=e.snapshot ORDER BY e.day"
    };
    let mut query = db.prepare(sql)?;
    let history = query
        .query_map(params![id, history_currency], |r| {
            Ok(WealthHistoryPoint {
                date: r.get(0)?,
                total_minor: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let limit = limit.clamp(1, 5000);
    let mut query = db.prepare("SELECT id,booking_date,description,amount_minor,currency FROM transactions t WHERE account_id=?1 AND NOT EXISTS(SELECT 1 FROM ignored_duplicate_transactions i WHERE i.transaction_id=t.id) ORDER BY booking_date DESC,id DESC LIMIT ?2")?;
    let mut transactions = query
        .query_map(params![id, limit + 1], |r| {
            Ok(AccountBooking {
                id: r.get(0)?,
                date: r.get(1)?,
                description: r.get(2)?,
                amount_minor: r.get(3)?,
                currency: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let has_more = transactions.len() > limit;
    transactions.truncate(limit);
    Ok(AccountDetails {
        account,
        positions,
        history,
        history_currency,
        transactions,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        crate::storage::initialize_schema(&db).unwrap();
        db.execute_batch("INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'test','Test','bank','now');
          INSERT INTO accounts(id,institution_id,name,account_type,currency,is_active,include_in_net_worth,created_at) VALUES(1,1,'Depot','portfolio','CHF',0,0,'now'),(2,1,'EUR account','cash','EUR',1,1,'now');
          INSERT INTO portfolio_positions(id,account_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,'Closed','stock','2026-01-02','2026-01-03','now','now'),(2,1,'Unpriced','stock','2026-01-02',NULL,'now','now'),(3,1,'Future','stock','2099-01-01',NULL,'now','now');
          INSERT INTO position_quantities(position_id,valid_from,quantity_amount,quantity_scale,source,recorded_at) VALUES(1,'2026-01-02',2,0,'test','now'),(1,'2099-01-01',10,0,'test','now');
          INSERT INTO daily_valuations(position_id,valuation_date,value_minor,currency,calculated_at) VALUES(1,'2026-01-02',2000,'CHF','now'),(1,'2026-01-03',3000,'CHF','now'),(3,'2099-01-01',5000,'CHF','now');
          INSERT INTO import_runs(id,account_id,source_name,source_format,source_hash,imported_at,transaction_count,warnings_json) VALUES(1,2,'test','test','test','now',3,'[]');
          INSERT INTO balance_snapshots(account_id,import_id,balance_date,amount_minor,currency) VALUES(2,1,'2026-01-01',10000,'EUR');
          INSERT INTO transactions(id,account_id,import_id,booking_date,description,amount_minor,currency,confidence,source_row) VALUES(1,2,1,'2026-01-02','Expense',-1000,'EUR',1,1),(2,2,1,'2026-01-03','Duplicate',-1000,'EUR',1,2),(3,2,1,'2026-01-04','Income',2000,'EUR',1,3);
          INSERT INTO ignored_duplicate_transactions(transaction_id) VALUES(2);").unwrap();
        db
    }
    #[test]
    fn depot_details_are_read_only_and_include_excluded_archived_accounts() {
        let db = database();
        let changes = db.total_changes();
        let detail = read_details(&db, 1, 50).unwrap();
        assert!(!detail.account.is_active);
        assert!(!detail.account.include_in_net_worth);
        assert_eq!(detail.positions.len(), 3);
        assert_eq!(
            detail
                .positions
                .iter()
                .find(|p| p.id == 1)
                .unwrap()
                .quantity,
            Some(2.0)
        );
        assert_eq!(
            detail
                .positions
                .iter()
                .find(|p| p.id == 2)
                .unwrap()
                .value_minor,
            None
        );
        assert_eq!(
            detail
                .history
                .iter()
                .map(|p| (p.date.as_str(), p.total_minor))
                .collect::<Vec<_>>(),
            vec![
                ("2026-01-02", 2000),
                ("2026-01-03", 3000),
                ("2026-01-04", 0)
            ]
        );
        assert!(detail.transactions.is_empty());
        assert_eq!(db.total_changes(), changes);
        assert!(read_details(&db, 999, 50).is_err());
    }
    #[test]
    fn account_details_keep_currency_exclude_duplicates_and_page_bookings() {
        let db = database();
        let detail = read_details(&db, 2, 1).unwrap();
        assert_eq!(detail.history_currency, "EUR");
        assert_eq!(
            detail
                .history
                .iter()
                .map(|p| p.total_minor)
                .collect::<Vec<_>>(),
            vec![10000, 9000, 11000]
        );
        assert_eq!(detail.transactions.len(), 1);
        assert_eq!(detail.transactions[0].id, 3);
        assert!(detail.has_more);
        let detail = read_details(&db, 2, 50).unwrap();
        assert_eq!(detail.transactions.len(), 2);
        assert!(!detail.has_more);
    }
}
