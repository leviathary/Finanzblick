//! Liest Synchronisationszustand und speichert Marktpreise und Wechselkurse atomar.
use crate::domain::securities::quotes::{needs_historical_backfill, DailyPrice, PositionToRefresh};
use crate::storage::{database, db_error, Storage};
use chrono::Local;
use rusqlite::params;
use std::collections::BTreeMap;

pub(crate) struct FetchedPrices {
    pub(crate) symbol: String,
    pub(crate) currency: String,
    pub(crate) source: String,
    pub(crate) prices: Vec<DailyPrice>,
    pub(crate) fx_source: String,
    pub(crate) rates: BTreeMap<String, f64>,
}

pub(crate) fn demo_position_count(storage: &Storage) -> Result<Option<usize>, String> {
    let db = storage.connect().map_err(db_error)?;
    if database::demo::is_demo(&db) {
        db.query_row("SELECT COUNT(*) FROM portfolio_positions", [], |r| r.get(0))
            .map(Some)
            .map_err(db_error)
    } else {
        Ok(None)
    }
}
pub(crate) fn last_success(storage: &Storage) -> Result<Option<String>, String> {
    let db = storage.connect().map_err(db_error)?;
    Ok(db
        .query_row(
            "SELECT last_success_at FROM market_sync_state WHERE id=1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(None))
}
pub(crate) fn record_attempt(storage: &Storage, now: &str) -> Result<(), String> {
    storage.connect().map_err(db_error)?.execute("INSERT INTO market_sync_state(id,last_attempt_at) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET last_attempt_at=excluded.last_attempt_at", [now]).map_err(db_error)?;
    Ok(())
}
pub(crate) fn record_success(storage: &Storage, now: &str) -> Result<(), String> {
    storage
        .connect()
        .map_err(db_error)?
        .execute(
            "UPDATE market_sync_state SET last_success_at=?1 WHERE id=1",
            [now],
        )
        .map_err(db_error)?;
    Ok(())
}
pub(crate) fn skipped_positions(storage: &Storage) -> Result<usize, String> {
    storage
        .connect()
        .map_err(db_error)?
        .query_row(
            "SELECT COUNT(*) FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id
 WHERE a.is_active=1 AND (p.listing_id IS NULL OR NOT EXISTS(
 SELECT 1 FROM position_quantities q WHERE q.position_id=p.id AND q.quantity_amount>0))",
            [],
            |r| r.get(0),
        )
        .map_err(db_error)
}
pub(crate) fn refresh_inputs(
    storage: &Storage,
) -> Result<(String, String, Vec<PositionToRefresh>), String> {
    let connection = storage.connect().map_err(db_error)?;
    let settings = database::read_settings(&connection)?;
    let mut statement = connection
            .prepare(
                "SELECT DISTINCT p.listing_id,p.label,ii.identifier_type,ii.identifier,l.market_symbol,l.quote_currency,'CHF',
                   (SELECT MIN(history.holding_start_date) FROM portfolio_positions history WHERE history.listing_id=p.listing_id),
                   (SELECT MIN(price.price_date) FROM instrument_prices price
                    WHERE price.listing_id=p.listing_id AND price.price_type IN ('official_close','eod_close','nav','last')
                      AND (l.preferred_price_source IS NULL OR price.source=l.preferred_price_source)),
                   (SELECT MAX(price.price_date) FROM instrument_prices price
                    WHERE price.listing_id=p.listing_id AND price.price_type IN ('official_close','eod_close','nav','last')
                      AND (l.preferred_price_source IS NULL OR price.source=l.preferred_price_source)),
                   l.preferred_price_source,p.asset_type
                 FROM portfolio_positions p JOIN accounts a ON a.id=p.account_id
                 JOIN instrument_listings l ON l.id=p.listing_id
                 JOIN instrument_identifiers ii ON ii.id=(SELECT identifier.id FROM instrument_identifiers identifier
                   WHERE identifier.instrument_id=l.instrument_id
                   ORDER BY CASE identifier.identifier_type WHEN 'ticker' THEN 1 WHEN 'isin' THEN 2 ELSE 3 END,identifier.id LIMIT 1)
                 WHERE a.is_active=1 AND ii.identifier<>'' AND COALESCE((SELECT q.quantity_amount
                   FROM position_quantities q WHERE q.position_id=p.id
                   ORDER BY date(q.valid_from) DESC,q.id DESC LIMIT 1),0)>0",
            )
            .map_err(db_error)?;
    let positions = statement
        .query_map([], |row| {
            Ok(PositionToRefresh {
                listing_id: row.get(0)?,
                label: row.get(1)?,
                identifier_type: row.get(2)?,
                identifier: row.get(3)?,
                market_symbol: row.get(4)?,
                market_currency: row.get(5)?,
                valuation_currency: row.get(6)?,
                history_start_date: row.get(7)?,
                earliest_price_date: row.get(8)?,
                latest_price_date: row.get(9)?,
                preferred_price_source: row.get(10)?,
                asset_type: row.get(11)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok((
        settings.marketstack_api_key,
        settings.alpha_vantage_api_key,
        positions,
    ))
}
pub(crate) fn store_prices(
    storage: &Storage,
    position: &PositionToRefresh,
    fetched: FetchedPrices,
) -> Result<usize, String> {
    let FetchedPrices {
        symbol,
        currency,
        source,
        prices,
        fx_source,
        rates,
    } = fetched;
    let mut connection = storage.connect().map_err(db_error)?;
    let transaction = connection.transaction().map_err(db_error)?;
    transaction
        .execute(
            "UPDATE instrument_listings SET market_symbol=?1,quote_currency=?2,preferred_price_source=?3 WHERE id=?4",
            params![symbol, currency, source, position.listing_id],
        )
        .map_err(db_error)?;
    let fetched_at = Local::now().to_rfc3339();
    for price in &prices {
        transaction
            .prepare_cached(
                "INSERT INTO instrument_prices(listing_id,price_date,price_type,price_amount,price_scale,currency,source,fetched_at)
                 VALUES(?1,?2,'eod_close',?3,4,?4,?5,?6)
                 ON CONFLICT(listing_id,price_date,price_type,source) DO UPDATE SET
                   price_amount=excluded.price_amount,price_scale=excluded.price_scale,currency=excluded.currency,fetched_at=excluded.fetched_at",
            )
            .map_err(db_error)?
            .execute(
                params![
                    position.listing_id,
                    price.date,
                    (price.close * 10_000.0).round() as i64,
                    currency,
                    source,
                    &fetched_at
                ],
            )
            .map_err(db_error)?;
    }
    if currency != position.valuation_currency {
        for (date, rate) in &rates {
            transaction
                .prepare_cached(
                    "INSERT INTO fx_rates(base_currency,quote_currency,rate_date,rate_amount,rate_scale,source,fetched_at)
                     VALUES(?1,?2,?3,?4,9,?5,?6)
                     ON CONFLICT(base_currency,quote_currency,rate_date,source) DO UPDATE SET
                       rate_amount=excluded.rate_amount,rate_scale=excluded.rate_scale,fetched_at=excluded.fetched_at",
                )
                .map_err(db_error)?
                .execute(
                    params![currency, position.valuation_currency, date, (rate * 1_000_000_000.0).round() as i64, fx_source, &fetched_at],
                )
                .map_err(db_error)?;
        }
    }
    if needs_historical_backfill(position) {
        transaction
            .execute(
                "DELETE FROM daily_valuations
                 WHERE position_id IN (SELECT id FROM portfolio_positions WHERE listing_id=?1)",
                [position.listing_id],
            )
            .map_err(db_error)?;
    }
    let stored = prices
        .iter()
        .filter(|price| {
            let date = price.date.as_str();
            position
                .earliest_price_date
                .as_deref()
                .is_none_or(|earliest| date < earliest)
                || position
                    .latest_price_date
                    .as_deref()
                    .is_none_or(|latest| date > latest)
        })
        .count();
    transaction.commit().map_err(db_error)?;
    Ok(stored)
}
