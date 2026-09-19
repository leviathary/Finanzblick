//! Lädt freigegebene Marktindizes als reine Vergleichsdaten, ohne Konten oder Bewertungen zu ändern.
use crate::{infrastructure::market_data::fetch_yahoo_chart, storage::Storage};
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Serialize)]
pub struct BenchmarkPoint {
    pub date: String,
    pub close: f64,
}
#[derive(Serialize)]
pub struct BenchmarkData {
    pub name: String,
    pub currency: String,
    pub points: Vec<BenchmarkPoint>,
}

pub(crate) async fn load(
    storage: &Storage,
    benchmark: &str,
    from: &str,
) -> Result<BenchmarkData, String> {
    drop(storage.require_unlocked()?);
    let (symbol, name) = match benchmark {
        "smi" => ("^SSMI", "SMI"),
        "sp500" => ("^GSPC", "S&P 500"),
        _ => return Err("Unbekannter Vergleichsindex.".into()),
    };
    let start = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|_| "Ungültiger Vergleichszeitraum.")?;
    if start.to_string() != from
        || start < NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
        || start > chrono::Utc::now().date_naive()
    {
        return Err("Ungültiger Vergleichszeitraum.".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Finanzblick/0.5")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "Die Kursverbindung konnte nicht vorbereitet werden.")?;
    // Only the public index symbol and date are sent. No account names, balances or transactions.
    let (currency, prices) = fetch_yahoo_chart(&client, symbol, Some(from)).await?;
    drop(storage.require_unlocked()?);
    let points = prices
        .into_iter()
        .filter(|p| p.close.is_finite() && p.close > 0.0)
        .map(|p| BenchmarkPoint {
            date: p.date,
            close: p.close,
        })
        .collect();
    Ok(BenchmarkData {
        name: name.into(),
        currency,
        points,
    })
}
