//! HTTP-Adapter für Kursanbieter und Symbolauflösung; kein Datenbankzugriff.
use crate::domain::securities::quotes::{
    DailyPrice, PositionToRefresh, LEADING_PRICE_GAP_TOLERANCE_DAYS,
};
use chrono::{Duration, Local, NaiveDate, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
const API_ROOT: &str = "https://www.alphavantage.co/query";
const MARKETSTACK_API_ROOT: &str = "https://api.marketstack.com/v2/eod";
const YAHOO_CHART_ROOT: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
pub(crate) async fn refresh_position(
    client: &reqwest::Client,
    marketstack_api_key: &str,
    alpha_vantage_api_key: &str,
    position: &PositionToRefresh,
) -> Result<(String, String, String, Vec<DailyPrice>), String> {
    let mut errors = Vec::new();
    for source in market_source_order(
        position.preferred_price_source.as_deref(),
        position.asset_type == "crypto",
    ) {
        let loaded = match source {
            "marketstack" if !marketstack_api_key.is_empty() => {
                fetch_marketstack_prices(client, marketstack_api_key, position).await
            }
            "alpha_vantage" if !alpha_vantage_api_key.is_empty() => {
                fetch_alpha_vantage_prices(client, alpha_vantage_api_key, position).await
            }
            "yahoo" => fetch_yahoo_prices(client, position).await,
            _ => continue,
        };
        match loaded {
            Ok((symbol, currency, values)) => {
                return Ok((symbol, currency, source.to_string(), values));
            }
            Err(error) => errors.push(format!("{}: {error}", market_source_name(source))),
        }
    }
    Err(errors.join(" · "))
}

fn market_source_order(preferred: Option<&str>, prefer_yahoo: bool) -> Vec<&'static str> {
    let mut sources = Vec::new();
    if let Some(preferred) = match preferred {
        Some("marketstack") => Some("marketstack"),
        Some("alpha_vantage") => Some("alpha_vantage"),
        Some("yahoo") => Some("yahoo"),
        _ => None,
    } {
        sources.push(preferred);
    }
    if prefer_yahoo && !sources.contains(&"yahoo") {
        sources.push("yahoo");
    }
    for source in ["marketstack", "alpha_vantage", "yahoo"] {
        if !sources.contains(&source) {
            sources.push(source);
        }
    }
    sources
}

fn market_source_name(source: &str) -> &str {
    match source {
        "marketstack" => "Marketstack",
        "alpha_vantage" => "Alpha Vantage",
        "yahoo" => "Yahoo",
        _ => source,
    }
}

async fn fetch_alpha_vantage_prices(
    client: &reqwest::Client,
    api_key: &str,
    position: &PositionToRefresh,
) -> Result<(String, String, Vec<DailyPrice>), String> {
    let (symbol, currency) = if let (Some(symbol), Some(currency)) =
        (&position.market_symbol, &position.market_currency)
    {
        let alpha_symbol = if position.identifier_type.eq_ignore_ascii_case("ticker") {
            alpha_symbol_hint(&position.identifier)
        } else {
            symbol.clone()
        };
        (alpha_symbol, currency.clone())
    } else {
        resolve_symbol(
            client,
            api_key,
            &position.identifier_type,
            &normalize_market_identifier(&position.identifier),
        )
        .await?
    };
    let values = fetch_daily_prices(client, api_key, &symbol).await?;
    Ok((symbol, currency, values))
}

async fn fetch_yahoo_prices(
    client: &reqwest::Client,
    position: &PositionToRefresh,
) -> Result<(String, String, Vec<DailyPrice>), String> {
    let identifier = if position.identifier_type.eq_ignore_ascii_case("isin") {
        resolve_isin(client, &position.identifier).await?
    } else {
        normalize_market_identifier(&position.identifier)
    };
    let symbol = yahoo_symbol_hint(&identifier);
    let start_date = market_sync_start_date(
        position.earliest_price_date.as_deref(),
        position.latest_price_date.as_deref(),
        &position.history_start_date,
        Local::now().date_naive(),
    );
    let (currency, prices) = fetch_yahoo_chart(client, &symbol, Some(&start_date)).await?;
    Ok((symbol, currency, prices))
}

pub(crate) fn market_sync_start_date(
    earliest_price_date: Option<&str>,
    latest_price_date: Option<&str>,
    history_start_date: &str,
    today: NaiveDate,
) -> String {
    let history_start = NaiveDate::parse_from_str(history_start_date, "%Y-%m-%d").unwrap_or(today);
    let earliest_price =
        earliest_price_date.and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok());
    if earliest_price.is_none_or(|earliest| {
        earliest > history_start + Duration::days(LEADING_PRICE_GAP_TOLERANCE_DAYS)
    }) {
        return history_start.to_string();
    }
    latest_price_date
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        // Reload one boundary day so an intraday value can be replaced by its
        // final close and weekends without a new quote remain successful.
        .map(|date| date.min(today))
        .unwrap_or(history_start)
        .to_string()
}

fn yahoo_symbol_hint(identifier: &str) -> String {
    let upper = identifier.trim().to_uppercase();
    let Some((mic, symbol)) = upper.split_once(':') else {
        return upper;
    };
    match mic {
        "XASX" => format!("{symbol}.AX"),
        "XLON" => format!("{symbol}.L"),
        "XSWX" => format!("{symbol}.SW"),
        "XETR" => format!("{symbol}.DE"),
        "XTSE" => format!("{symbol}.TO"),
        _ => symbol.to_string(),
    }
}

async fn fetch_yahoo_chart(
    client: &reqwest::Client,
    symbol: &str,
    start_date: Option<&str>,
) -> Result<(String, Vec<DailyPrice>), String> {
    let url = format!("{YAHOO_CHART_ROOT}/{}", urlencoding::encode(symbol));
    let period_start = start_date
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| date.and_utc().timestamp())
        .unwrap_or(0)
        .to_string();
    let period_end = (Utc::now().timestamp() + 86_400).to_string();
    let response = client
        .get(url)
        .query(&[
            ("period1", period_start.as_str()),
            ("period2", period_end.as_str()),
            ("interval", "1d"),
            ("events", "history"),
        ])
        .send()
        .await
        .map_err(|_| "Die Kursquelle ist momentan nicht erreichbar.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Die Kursquelle antwortete mit Status {}.",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Die Antwort der Kursquelle war nicht lesbar.".to_string())?;
    if !value["chart"]["error"].is_null() {
        return Err(value["chart"]["error"]["description"]
            .as_str()
            .unwrap_or("Für diese Kennung wurden keine Kurse gefunden.")
            .to_string());
    }
    let chart = value["chart"]["result"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| "Für diese Kennung wurden keine Kurse gefunden.".to_string())?;
    let currency = chart["meta"]["currency"]
        .as_str()
        .ok_or_else(|| "Die Kursquelle lieferte keine Handelswährung.".to_string())?
        .to_uppercase();
    let exchange_offset = chart["meta"]["gmtoffset"].as_i64().unwrap_or(0);
    let timestamps = chart["timestamp"]
        .as_array()
        .ok_or_else(|| "Die Kursquelle lieferte keine Tageskurse.".to_string())?;
    let closes = chart["indicators"]["quote"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|quote| quote["close"].as_array())
        .ok_or_else(|| "Die Kursquelle lieferte keine Schlusskurse.".to_string())?;
    let mut prices = timestamps
        .iter()
        .zip(closes)
        .filter_map(|(timestamp, close)| {
            Some(DailyPrice {
                date: chrono::DateTime::from_timestamp(timestamp.as_i64()? + exchange_offset, 0)?
                    .date_naive()
                    .to_string(),
                close: close.as_f64()?,
            })
        })
        .collect::<Vec<_>>();
    prices.sort_by(|a, b| a.date.cmp(&b.date));
    prices.dedup_by(|a, b| a.date == b.date);
    if prices.is_empty() {
        return Err("Die Kursquelle lieferte keine verwendbaren Tageskurse.".into());
    }
    Ok((currency, prices))
}

async fn fetch_marketstack_prices(
    client: &reqwest::Client,
    api_key: &str,
    position: &PositionToRefresh,
) -> Result<(String, String, Vec<DailyPrice>), String> {
    let identifier = normalize_market_identifier(&position.identifier);
    let (mic, symbol) = identifier
        .split_once(':')
        .map_or((None, identifier.as_str()), |(mic, symbol)| {
            (Some(mic), symbol)
        });
    let mut request = client.get(MARKETSTACK_API_ROOT).query(&[
        ("access_key", api_key),
        ("symbols", symbol),
        ("limit", "1000"),
    ]);
    if let Some(exchange) = mic {
        request = request.query(&[("exchange", exchange)]);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Marketstack ist momentan nicht erreichbar.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Marketstack antwortete mit Status {}.",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Die Marketstack-Antwort war nicht lesbar.".to_string())?;
    if let Some(message) = value["error"]["message"].as_str() {
        return Err(message.to_string());
    }
    let rows = value["data"]
        .as_array()
        .ok_or_else(|| "Marketstack lieferte keine Tageskurse.".to_string())?;
    let mut prices: Vec<DailyPrice> = rows
        .iter()
        .filter(|row| {
            mic.is_none()
                || row["exchange"]
                    .as_str()
                    .is_some_and(|exchange| Some(exchange) == mic)
        })
        .filter_map(|row| {
            Some(DailyPrice {
                date: row["date"].as_str()?.get(..10)?.to_string(),
                close: row["close"].as_f64()?,
            })
        })
        .collect();
    prices.sort_by(|a, b| a.date.cmp(&b.date));
    if prices.is_empty() {
        return Err("Marketstack lieferte keine passenden Tageskurse.".into());
    }
    let currency = position
        .market_currency
        .clone()
        .or_else(|| match mic {
            Some("XASX") => Some("AUD".into()),
            Some("XSWX") => Some("CHF".into()),
            Some("XLON") => Some("GBP".into()),
            Some("XETR") => Some("EUR".into()),
            Some("XNAS" | "XNYS" | "XASE") => Some("USD".into()),
            _ => None,
        })
        .unwrap_or_else(|| position.valuation_currency.clone());
    Ok((symbol.to_string(), currency, prices))
}

pub(crate) fn normalize_market_identifier(identifier: &str) -> String {
    let upper = identifier.trim().to_uppercase();
    let Some((left, right)) = upper.split_once(':') else {
        return upper;
    };
    match (left, right) {
        ("ASX" | "XASX", symbol) => format!("XASX:{symbol}"),
        (symbol, "ASX" | "XASX") => format!("XASX:{symbol}"),
        _ => upper,
    }
}

async fn resolve_symbol(
    client: &reqwest::Client,
    api_key: &str,
    identifier_type: &str,
    identifier: &str,
) -> Result<(String, String), String> {
    let lookup_identifier = if identifier_type == "isin" {
        resolve_isin(client, identifier).await?
    } else {
        identifier.to_string()
    };
    let query = lookup_identifier
        .split(':')
        .next_back()
        .unwrap_or(&lookup_identifier);
    let value = fetch_json(
        client,
        &[
            ("function", "SYMBOL_SEARCH"),
            ("keywords", query),
            ("apikey", api_key),
        ],
    )
    .await?;
    let matches = value["bestMatches"]
        .as_array()
        .ok_or_else(|| "Für diese Kennung wurde kein Wertpapier gefunden.".to_string())?;
    let wanted = alpha_symbol_hint(&lookup_identifier);
    let base_symbol = lookup_identifier
        .split(':')
        .next_back()
        .unwrap_or(&lookup_identifier);
    let expected_region = lookup_identifier
        .split_once(':')
        .and_then(|(mic, _)| mic_region(mic));
    let selected = matches
        .iter()
        .find(|item| item["1. symbol"].as_str() == Some(wanted.as_str()))
        .or_else(|| {
            expected_region.and_then(|region| {
                matches.iter().find(|item| {
                    item["1. symbol"]
                        .as_str()
                        .is_some_and(|symbol| symbol.split('.').next() == Some(base_symbol))
                        && item["4. region"]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(region))
                })
            })
        })
        .or_else(|| matches.first())
        .ok_or_else(|| "Für diese Kennung wurde kein Wertpapier gefunden.".to_string())?;
    let symbol = selected["1. symbol"]
        .as_str()
        .ok_or_else(|| "Die Kursquelle lieferte kein Tickersymbol.".to_string())?;
    let currency = selected["8. currency"]
        .as_str()
        .ok_or_else(|| "Die Kursquelle lieferte keine Handelswährung.".to_string())?;
    Ok((symbol.to_string(), currency.to_uppercase()))
}

fn mic_region(mic: &str) -> Option<&'static str> {
    match mic {
        "XNAS" | "XNYS" | "XASE" => Some("United States"),
        "XASX" => Some("Australia"),
        "XLON" => Some("United Kingdom"),
        "XSWX" => Some("Switzerland"),
        "XETR" => Some("Germany"),
        "XTSE" => Some("Canada"),
        _ => None,
    }
}

async fn resolve_isin(client: &reqwest::Client, isin: &str) -> Result<String, String> {
    let response = client
        .post("https://api.openfigi.com/v3/mapping")
        .json(&serde_json::json!([{"idType": "ID_ISIN", "idValue": isin}]))
        .send()
        .await
        .map_err(|_| "Die ISIN-Auflösung ist momentan nicht erreichbar.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Die ISIN-Auflösung antwortete mit Status {}.",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Die Antwort der ISIN-Auflösung war nicht lesbar.".to_string())?;
    let match_data = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item["data"].as_array())
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["marketSector"].as_str() == Some("Equity"))
                .or_else(|| items.first())
        })
        .ok_or_else(|| "Für diese ISIN wurde kein Wertpapier gefunden.".to_string())?;
    let ticker = match_data["ticker"]
        .as_str()
        .ok_or_else(|| "Für diese ISIN wurde kein Ticker gefunden.".to_string())?;
    let prefix = match match_data["exchCode"].as_str().unwrap_or_default() {
        "AU" => "XASX:",
        "LN" => "XLON:",
        "SW" => "XSWX:",
        "GR" | "GY" => "XETR:",
        "CN" | "CT" => "XTSE:",
        _ => "",
    };
    Ok(format!("{prefix}{ticker}"))
}

fn alpha_symbol_hint(identifier: &str) -> String {
    let upper = identifier.trim().to_uppercase();
    let Some((mic, symbol)) = upper.split_once(':') else {
        return upper;
    };
    match mic {
        "XASX" => format!("{symbol}.AX"),
        "XLON" => format!("{symbol}.LON"),
        "XSWX" => format!("{symbol}.SW"),
        "XETR" => format!("{symbol}.DEX"),
        "XTSE" => format!("{symbol}.TRT"),
        _ => symbol.to_string(),
    }
}

async fn fetch_daily_prices(
    client: &reqwest::Client,
    api_key: &str,
    symbol: &str,
) -> Result<Vec<DailyPrice>, String> {
    let full = fetch_json(
        client,
        &[
            ("function", "TIME_SERIES_DAILY"),
            ("symbol", symbol),
            ("outputsize", "full"),
            ("apikey", api_key),
        ],
    )
    .await;
    let value = match full {
        Ok(value) => value,
        Err(_) => {
            fetch_json(
                client,
                &[
                    ("function", "TIME_SERIES_DAILY"),
                    ("symbol", symbol),
                    ("outputsize", "compact"),
                    ("apikey", api_key),
                ],
            )
            .await?
        }
    };
    let series = value["Time Series (Daily)"]
        .as_object()
        .ok_or_else(|| "Die Kursquelle lieferte keine Tageskurse.".to_string())?;
    let mut prices = Vec::new();
    for (date, row) in series {
        if let Some(close) = row["4. close"].as_str().and_then(|v| v.parse().ok()) {
            prices.push(DailyPrice {
                date: date.clone(),
                close,
            });
        }
    }
    prices.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(prices)
}

async fn fetch_fx(
    client: &reqwest::Client,
    api_key: &str,
    from: &str,
    to: &str,
) -> Result<BTreeMap<String, f64>, String> {
    let full = fetch_json(
        client,
        &[
            ("function", "FX_DAILY"),
            ("from_symbol", from),
            ("to_symbol", to),
            ("outputsize", "full"),
            ("apikey", api_key),
        ],
    )
    .await;
    let value = match full {
        Ok(value) => value,
        Err(_) => {
            fetch_json(
                client,
                &[
                    ("function", "FX_DAILY"),
                    ("from_symbol", from),
                    ("to_symbol", to),
                    ("outputsize", "compact"),
                    ("apikey", api_key),
                ],
            )
            .await?
        }
    };
    let series = value["Time Series FX (Daily)"]
        .as_object()
        .ok_or_else(|| "Die Kursquelle lieferte keine täglichen Wechselkurse.".to_string())?;
    Ok(series
        .iter()
        .filter_map(|(date, row)| {
            row["4. close"]
                .as_str()
                .and_then(|value| value.parse().ok())
                .map(|rate| (date.clone(), rate))
        })
        .collect())
}

pub(crate) async fn fetch_fx_with_fallback(
    client: &reqwest::Client,
    alpha_vantage_api_key: &str,
    from: &str,
    to: &str,
    first_price_date: Option<&str>,
    preferred_source: Option<&str>,
) -> Result<(String, BTreeMap<String, f64>), String> {
    let mut errors = Vec::new();
    for source in market_source_order(preferred_source, false) {
        let loaded = match source {
            "alpha_vantage" if !alpha_vantage_api_key.is_empty() => {
                fetch_fx(client, alpha_vantage_api_key, from, to).await
            }
            "yahoo" => fetch_yahoo_fx(client, from, to, first_price_date).await,
            _ => continue,
        };
        match loaded {
            Ok(rates) if !rates.is_empty() => return Ok((source.into(), rates)),
            Ok(_) => errors.push(format!(
                "{}: keine Wechselkurse geliefert.",
                market_source_name(source)
            )),
            Err(error) => errors.push(format!("{}: {error}", market_source_name(source))),
        }
    }
    Err(errors.join(" · "))
}

async fn fetch_yahoo_fx(
    client: &reqwest::Client,
    from: &str,
    to: &str,
    first_price_date: Option<&str>,
) -> Result<BTreeMap<String, f64>, String> {
    let symbol = format!("{from}{to}=X");
    let fx_start_date = first_price_date
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .map(|date| (date - Duration::days(7)).to_string());
    let (_, prices) = fetch_yahoo_chart(client, &symbol, fx_start_date.as_deref()).await?;
    Ok(prices
        .into_iter()
        .map(|price| (price.date, price.close))
        .collect())
}

async fn fetch_json(
    client: &reqwest::Client,
    parameters: &[(&str, &str)],
) -> Result<Value, String> {
    let response = client
        .get(API_ROOT)
        .query(parameters)
        .send()
        .await
        .map_err(|_| "Die Kursquelle ist momentan nicht erreichbar.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Die Kursquelle antwortete mit Status {}.",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Die Antwort der Kursquelle war nicht lesbar.".to_string())?;
    if let Some(message) = value["Error Message"].as_str() {
        return Err(message.to_string());
    }
    if let Some(message) = value["Note"]
        .as_str()
        .or_else(|| value["Information"].as_str())
    {
        return Err(message.to_string());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        alpha_symbol_hint, market_source_order, market_sync_start_date,
        normalize_market_identifier, yahoo_symbol_hint,
    };
    use chrono::NaiveDate;

    #[test]
    fn converts_mic_tickers_for_alpha_vantage() {
        assert_eq!(alpha_symbol_hint("XNAS:AAPL"), "AAPL");
        assert_eq!(alpha_symbol_hint("XASX:XYZ"), "XYZ.AX");
        assert_eq!(alpha_symbol_hint("AAPL"), "AAPL");
    }

    #[test]
    fn normalizes_common_asx_ticker_notations() {
        assert_eq!(normalize_market_identifier("ASX:XYZ"), "XASX:XYZ");
        assert_eq!(normalize_market_identifier("XYZ:ASX"), "XASX:XYZ");
        assert_eq!(normalize_market_identifier("XASX:XYZ"), "XASX:XYZ");
    }

    #[test]
    fn converts_mic_tickers_for_yahoo() {
        assert_eq!(yahoo_symbol_hint("XNAS:AAPL"), "AAPL");
        assert_eq!(yahoo_symbol_hint("XASX:XYZ"), "XYZ.AX");
        assert_eq!(yahoo_symbol_hint("XLON:TSCO"), "TSCO.L");
        assert_eq!(yahoo_symbol_hint("XSWX:NESN"), "NESN.SW");
        assert_eq!(yahoo_symbol_hint("XETR:SAP"), "SAP.DE");
        assert_eq!(yahoo_symbol_hint("XTSE:SHOP"), "SHOP.TO");
        assert_eq!(yahoo_symbol_hint("BTC-USD"), "BTC-USD");
    }

    #[test]
    fn starts_market_sync_at_the_latest_stored_day_as_a_small_overlap() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
        assert_eq!(
            market_sync_start_date(Some("2022-09-01"), Some("2026-06-12"), "2022-09-01", today,),
            "2026-06-12"
        );
        assert_eq!(
            market_sync_start_date(Some("2022-09-01"), Some("2026-09-13"), "2022-09-01", today,),
            "2026-09-13"
        );
        assert_eq!(
            market_sync_start_date(None, None, "2022-09-01", today),
            "2022-09-01"
        );
    }

    #[test]
    fn backfills_when_the_holding_start_moves_before_the_first_stored_price() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
        assert_eq!(
            market_sync_start_date(Some("2025-11-07"), Some("2026-09-13"), "2025-01-01", today,),
            "2025-01-01"
        );
    }

    #[test]
    fn accepts_a_short_leading_gap_for_weekends_and_market_holidays() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
        assert_eq!(
            market_sync_start_date(Some("2025-01-02"), Some("2026-09-12"), "2025-01-01", today,),
            "2026-09-12"
        );
    }

    #[test]
    fn tries_the_last_successful_market_source_first() {
        assert_eq!(
            market_source_order(Some("yahoo"), false),
            vec!["yahoo", "marketstack", "alpha_vantage"]
        );
        assert_eq!(
            market_source_order(Some("alpha_vantage"), false),
            vec!["alpha_vantage", "marketstack", "yahoo"]
        );
        assert_eq!(
            market_source_order(None, false),
            vec!["marketstack", "alpha_vantage", "yahoo"]
        );
        assert_eq!(
            market_source_order(None, true),
            vec!["yahoo", "marketstack", "alpha_vantage"]
        );
    }
}
