//! Ermittelt historische Mengen, Preise und Bewertungen einzelner Vermögenspositionen.

use crate::storage::{db_error, decimal_value, Storage, WealthHistoryPoint};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionChart {
    id: i64,
    label: String,
    holding_end_date: Option<String>,
    history: Vec<WealthHistoryPoint>,
    prices: Vec<PricePoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricePoint {
    date: String,
    total_minor: f64,
    currency: String,
}

pub fn position_chart_data(
    storage: &Storage,
    account_id: i64,
) -> Result<Vec<PositionChart>, String> {
    let connection = storage.connect().map_err(db_error)?;
    read_positions(&connection, account_id).map_err(db_error)
}

fn read_positions(
    connection: &Connection,
    account_id: i64,
) -> rusqlite::Result<Vec<PositionChart>> {
    let mut statement = connection.prepare("SELECT p.id,p.label,p.holding_end_date FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id WHERE a.id=?1 AND a.account_type IN ('portfolio','manual_asset','pillar3a') ORDER BY p.label,p.id")?;
    let mut positions = statement
        .query_map([account_id], |row| {
            Ok(PositionChart {
                id: row.get(0)?,
                label: row.get(1)?,
                holding_end_date: row.get(2)?,
                history: Vec::new(),
                prices: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for position in &mut positions {
        let mut values = connection.prepare("SELECT d.valuation_date,d.value_minor FROM daily_valuations d JOIN portfolio_positions p ON p.id=d.position_id WHERE p.id=?1 AND d.currency='CHF' AND d.valuation_date>=p.holding_start_date AND (p.holding_end_date IS NULL OR d.valuation_date<=p.holding_end_date) AND d.valuation_date<=date('now','localtime') UNION ALL SELECT date(holding_end_date,'+1 day'),0 FROM portfolio_positions WHERE id=?1 AND date(holding_end_date,'+1 day')<=date('now','localtime') ORDER BY 1")?;
        position.history = values
            .query_map([position.id], |row| {
                Ok(WealthHistoryPoint {
                    date: row.get(0)?,
                    total_minor: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut prices = connection.prepare("SELECT d.valuation_date,ip.price_amount,ip.price_scale,ip.currency FROM daily_valuations d JOIN instrument_prices ip ON ip.id=d.instrument_price_id JOIN portfolio_positions p ON p.id=d.position_id WHERE p.id=?1 AND d.valuation_date>=p.holding_start_date AND (p.holding_end_date IS NULL OR d.valuation_date<=p.holding_end_date) AND d.valuation_date<=date('now','localtime') ORDER BY d.valuation_date")?;
        position.prices = prices
            .query_map([position.id], |row| {
                Ok(PricePoint {
                    date: row.get(0)?,
                    total_minor: decimal_value(row.get(1)?, row.get(2)?) * 100.0,
                    currency: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histories_keep_sold_positions_and_original_unit_prices() {
        let connection = Connection::open_in_memory().unwrap();
        crate::storage::initialize_schema(&connection).unwrap();
        connection.execute_batch(
            "INSERT INTO institutions(id,provider_key,name,institution_type,created_at) VALUES(1,'broker','Broker','broker','2020-01-01');
             INSERT INTO accounts(id,institution_id,name,account_type,currency,created_at) VALUES(1,1,'Depot','manual_asset','CHF','2020-01-01'),(2,1,'Other','manual_asset','CHF','2020-01-01');
             INSERT INTO portfolio_positions(id,account_id,label,asset_type,holding_start_date,holding_end_date,created_at,updated_at) VALUES(1,1,'Sold','stock','2020-01-01','2020-01-02','2020-01-01','2020-01-01'),(2,2,'Other','stock','2020-01-01',NULL,'2020-01-01','2020-01-01');
             INSERT INTO daily_valuations(position_id,valuation_date,value_minor,currency,calculated_at) VALUES(1,'2020-01-01',10000,'CHF','2020-01-01'),(1,'2020-01-02',20000,'CHF','2020-01-02'),(2,'2020-01-01',99999,'CHF','2020-01-01');"
        ).unwrap();
        let positions = read_positions(&connection, 1).unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].holding_end_date.as_deref(), Some("2020-01-02"));
        assert_eq!(
            positions[0]
                .history
                .iter()
                .map(|point| point.total_minor)
                .collect::<Vec<_>>(),
            vec![10000, 20000, 0]
        );
        assert_eq!(positions[0].history[2].date, "2020-01-03");
        assert!(positions[0].prices.is_empty());
        assert!(read_positions(&connection, 999).unwrap().is_empty());
        connection.execute_batch(
            "INSERT INTO instruments(id,name,asset_type) VALUES(1,'Stock','stock');
             INSERT INTO instrument_listings(id,instrument_id,quote_currency) VALUES(1,1,'USD');
             INSERT INTO instrument_prices(id,listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at) VALUES(1,1,'2020-01-01','eod_close',123456,4,'USD','manual','2020-01-01');
             UPDATE portfolio_positions SET listing_id=1 WHERE id=1;
             UPDATE daily_valuations SET instrument_price_id=1 WHERE position_id=1;"
        ).unwrap();
        let positions = read_positions(&connection, 1).unwrap();
        assert_eq!(positions[0].prices.len(), 2);
        for price in &positions[0].prices {
            assert_eq!(price.currency, "USD");
            assert!((price.total_minor - 1234.56).abs() < 0.00001);
        }
    }
}
