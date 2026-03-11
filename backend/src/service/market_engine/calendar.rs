//! Calendar data fetching via Supabase Edge Function (earnings)

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::quotes::get_simple_quotes;

const DEFAULT_CALENDAR_URL: &str =
    "https://bnavjgbowcekeppwgnxc.supabase.co/functions/v1/earnings-calendar";
const DEFAULT_CALENDAR_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImJuYXZqZ2Jvd2Nla2VwcHdnbnhjIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTkwODk1OTAsImV4cCI6MjA3NDY2NTU5MH0.AK7v9ofCWWgjsj4fUfr4nsRRcVwFQMeaNt1zNs6bjN0";

/// Minimum market cap threshold in USD ($100 million)
const MIN_MARKET_CAP: i64 = 100_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsStock {
    pub importance: i64,
    pub symbol: String,
    pub date: String,
    pub time: String,
    pub title: String,
    pub emoji: Option<String>,
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsDay {
    pub stocks: Vec<EarningsStock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsCalendarResponse {
    pub success: bool,
    pub date_from: String,
    pub date_to: String,
    pub total_dates: Option<usize>,
    pub total_earnings: Option<usize>,
    pub earnings: HashMap<String, EarningsDay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsCalendarRequest<'a> {
    #[serde(rename = "fromDate", skip_serializing_if = "Option::is_none")]
    pub from_date: Option<&'a str>,
    #[serde(rename = "toDate", skip_serializing_if = "Option::is_none")]
    pub to_date: Option<&'a str>,
    #[serde(rename = "includeLogos", skip_serializing_if = "Option::is_none")]
    pub include_logos: Option<bool>,
}

fn calendar_url() -> String {
    std::env::var("EARNINGS_CALENDAR_URL").unwrap_or_else(|_| DEFAULT_CALENDAR_URL.to_string())
}

fn calendar_token() -> String {
    std::env::var("EARNINGS_CALENDAR_TOKEN").unwrap_or_else(|_| DEFAULT_CALENDAR_TOKEN.to_string())
}

/// Fetch earnings calendar from the Supabase Edge Function (unfiltered).
/// Date format: YYYY-MM-DD. If not provided, the edge function defaults internally.
pub async fn fetch_earnings_calendar_raw(
    from_date: Option<&str>,
    to_date: Option<&str>,
    include_logos: bool,
) -> Result<EarningsCalendarResponse> {
    let client = Client::new();
    let url = calendar_url();
    let token = calendar_token();

    let payload = EarningsCalendarRequest {
        from_date,
        to_date,
        include_logos: Some(include_logos),
    };

    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .context("failed to call earnings calendar edge function")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("edge function error: status {status} body: {body}"));
    }

    let parsed: EarningsCalendarResponse = resp
        .json()
        .await
        .context("failed to parse earnings calendar response")?;

    if !parsed.success {
        return Err(anyhow!("edge function returned success=false"));
    }

    Ok(parsed)
}

/// Fetch market caps for a batch of symbols.
/// Returns a HashMap of symbol -> market_cap (None if unavailable).
async fn fetch_market_caps(symbols: &[&str]) -> HashMap<String, Option<i64>> {
    let mut result: HashMap<String, Option<i64>> = HashMap::new();

    if symbols.is_empty() {
        return result;
    }

    // Batch in chunks of 50 to avoid API limits
    for chunk in symbols.chunks(50) {
        match get_simple_quotes(chunk).await {
            Ok(value) => {
                if let Some(quotes) = value
                    .get("quoteResponse")
                    .and_then(|qr| qr.get("result"))
                    .and_then(|r| r.as_array())
                {
                    for quote in quotes {
                        if let Some(symbol) = quote.get("symbol").and_then(|s| s.as_str()) {
                            let market_cap = quote.get("marketCap").and_then(|mc| mc.as_i64());
                            result.insert(symbol.to_uppercase(), market_cap);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch market caps for batch: {}", e);
                // Mark all symbols in this batch as None
                for sym in chunk {
                    result.insert(sym.to_uppercase(), None);
                }
            }
        }
    }

    result
}

/// Filter earnings stocks by market cap (>= $100M).
/// Stocks with unknown market cap are excluded.
fn filter_by_market_cap(
    earnings: HashMap<String, EarningsDay>,
    market_caps: &HashMap<String, Option<i64>>,
    min_cap: i64,
) -> HashMap<String, EarningsDay> {
    earnings
        .into_iter()
        .map(|(date, day)| {
            let filtered_stocks: Vec<EarningsStock> = day
                .stocks
                .into_iter()
                .filter(|stock| {
                    let symbol = stock.symbol.to_uppercase();
                    match market_caps.get(&symbol) {
                        Some(Some(cap)) => *cap >= min_cap,
                        _ => false, // Exclude if market cap unknown
                    }
                })
                .collect();

            (date, EarningsDay { stocks: filtered_stocks })
        })
        .filter(|(_, day)| !day.stocks.is_empty())
        .collect()
}

/// Fetch earnings calendar with market cap filtering (>= $100M).
/// Date format: YYYY-MM-DD. If not provided, the edge function defaults internally.
pub async fn fetch_earnings_calendar(
    from_date: Option<&str>,
    to_date: Option<&str>,
    include_logos: bool,
) -> Result<EarningsCalendarResponse> {
    // Fetch raw earnings data
    let mut response = fetch_earnings_calendar_raw(from_date, to_date, include_logos).await?;

    // Collect all unique symbols
    let symbols: Vec<String> = response
        .earnings
        .values()
        .flat_map(|day| day.stocks.iter().map(|s| s.symbol.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();

    // Fetch market caps
    let market_caps = fetch_market_caps(&symbol_refs).await;

    // Filter by market cap
    response.earnings = filter_by_market_cap(response.earnings, &market_caps, MIN_MARKET_CAP);

    // Update totals
    let total_earnings: usize = response.earnings.values().map(|d| d.stocks.len()).sum();
    response.total_dates = Some(response.earnings.len());
    response.total_earnings = Some(total_earnings);

    Ok(response)
}

/// Fetch earnings calendar with custom market cap filter.
/// `min_market_cap` is in USD (e.g., 100_000_000 for $100M).
pub async fn fetch_earnings_calendar_filtered(
    from_date: Option<&str>,
    to_date: Option<&str>,
    include_logos: bool,
    min_market_cap: i64,
) -> Result<EarningsCalendarResponse> {
    let mut response = fetch_earnings_calendar_raw(from_date, to_date, include_logos).await?;

    let symbols: Vec<String> = response
        .earnings
        .values()
        .flat_map(|day| day.stocks.iter().map(|s| s.symbol.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
    let market_caps = fetch_market_caps(&symbol_refs).await;

    response.earnings = filter_by_market_cap(response.earnings, &market_caps, min_market_cap);

    let total_earnings: usize = response.earnings.values().map(|d| d.stocks.len()).sum();
    response.total_dates = Some(response.earnings.len());
    response.total_earnings = Some(total_earnings);

    Ok(response)
}
