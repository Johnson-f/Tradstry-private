//! News data fetching functions using finance-query-core
//!
//! This module provides functions to fetch news data from Yahoo Finance:
//! - News articles for a specific stock symbol
//! - News search by query
//! - Trending news and market news

use super::client_helper::get_yahoo_client;
use finance_query_core::YahooError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// News Types

/// A news article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub uuid: Option<String>,
    pub title: String,
    pub publisher: Option<String>,
    pub link: String,
    pub publish_time: Option<i64>,
    pub publish_time_formatted: Option<String>,
    pub thumbnail_url: Option<String>,
    pub related_tickers: Vec<String>,
}

impl NewsArticle {
    /// Get the publish date as a formatted string
    #[allow(dead_code)]
    pub fn formatted_date(&self) -> String {
        if let Some(formatted) = &self.publish_time_formatted {
            return formatted.clone();
        }

        if let Some(ts) = self.publish_time {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        }
    }

    /// Get relative time (e.g., "2 hours ago")
    #[allow(dead_code)]
    pub fn relative_time(&self) -> String {
        if let Some(ts) = self.publish_time {
            let now = chrono::Utc::now().timestamp();
            let diff = now - ts;

            if diff < 60 {
                format!("{}s ago", diff)
            } else if diff < 3600 {
                format!("{}m ago", diff / 60)
            } else if diff < 86400 {
                format!("{}h ago", diff / 3600)
            } else {
                format!("{}d ago", diff / 86400)
            }
        } else {
            "Unknown".to_string()
        }
    }
}

/// News response containing multiple articles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResponse {
    pub symbol: Option<String>,
    pub query: Option<String>,
    pub articles: Vec<NewsArticle>,
    pub count: usize,
}

impl NewsResponse {
    /// Get articles from a specific publisher
    #[allow(dead_code, clippy::wrong_self_convention)]
    pub fn from_publisher(&self, publisher: &str) -> Vec<&NewsArticle> {
        self.articles
            .iter()
            .filter(|a| {
                a.publisher
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(&publisher.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Get articles mentioning a specific ticker
    #[allow(dead_code)]
    pub fn mentioning_ticker(&self, ticker: &str) -> Vec<&NewsArticle> {
        self.articles
            .iter()
            .filter(|a| {
                a.related_tickers
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(ticker))
            })
            .collect()
    }

    /// Check if there are any articles
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.articles.is_empty()
    }
}

// Helper Functions

/// Parse news articles from Yahoo Finance search response
fn parse_news_from_search(json: &Value) -> Vec<NewsArticle> {
    let mut articles = Vec::new();

    if let Some(news_array) = json.get("news").and_then(|n| n.as_array()) {
        for item in news_array {
            if let Some(title) = item.get("title").and_then(|t| t.as_str()) {
                let article = NewsArticle {
                    uuid: item.get("uuid").and_then(|u| u.as_str()).map(String::from),
                    title: title.to_string(),
                    publisher: item
                        .get("publisher")
                        .and_then(|p| p.as_str())
                        .map(String::from),
                    link: item
                        .get("link")
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .to_string(),
                    publish_time: item.get("providerPublishTime").and_then(|t| t.as_i64()),
                    publish_time_formatted: None,
                    thumbnail_url: item
                        .get("thumbnail")
                        .and_then(|t| t.get("resolutions"))
                        .and_then(|r| r.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|first| first.get("url"))
                        .and_then(|u| u.as_str())
                        .map(String::from),
                    related_tickers: item
                        .get("relatedTickers")
                        .and_then(|t| t.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                articles.push(article);
            }
        }
    }

    articles
}

/// Parse news from quote summary response (reserved for future use)
#[allow(dead_code)]
fn parse_news_from_quote_summary(_json: &Value, _symbol: &str) -> Vec<NewsArticle> {
    // The quote summary doesn't typically include news
    // We rely on the search endpoint instead
    Vec::new()
}

// Public API Functions

/// Get raw news data JSON for a symbol using search
///
/// # Arguments
/// * `symbol` - The stock symbol (e.g., "AAPL")
/// * `count` - Number of news articles to fetch
///
/// # Returns
/// Raw JSON value containing news data
pub async fn get_news_raw(symbol: &str, count: usize) -> Result<Value, YahooError> {
    let client = get_yahoo_client().await?;
    client.search(symbol, count).await
}

/// Get news articles for a stock symbol
///
/// Fetches recent news articles related to a specific stock
///
/// # Arguments
/// * `symbol` - The stock symbol (e.g., "AAPL")
/// * `count` - Maximum number of articles to return
///
/// # Returns
/// NewsResponse with articles related to the symbol
///
/// # Example
/// ```rust,ignore
/// let news = get_news_for_symbol("AAPL", 10).await?;
/// for article in &news.articles {
///     println!("{} - {} ({})", article.title, article.publisher.as_deref().unwrap_or("Unknown"), article.relative_time());
/// }
/// ```
pub async fn get_news_for_symbol(symbol: &str, count: usize) -> Result<NewsResponse, YahooError> {
    let json = get_news_raw(symbol, count).await?;
    let articles = parse_news_from_search(&json);

    Ok(NewsResponse {
        symbol: Some(symbol.to_string()),
        query: None,
        count: articles.len(),
        articles,
    })
}

// Wrapper function for backward compatibility
/// Get news
pub async fn get_news(
    symbol: Option<&str>,
    limit: Option<u32>,
) -> Result<NewsResponse, YahooError> {
    let symbol = symbol.unwrap_or("market");
    let count = limit.unwrap_or(10) as usize;
    get_news_for_symbol(symbol, count).await
}

/// Search for news by query
///
/// Searches for news articles matching a query string
///
/// # Arguments
/// * `query` - Search query (e.g., "tech earnings", "oil prices")
/// * `count` - Maximum number of articles to return
///
/// # Returns
/// NewsResponse with articles matching the query
///
/// # Example
/// ```rust,ignore
/// let news = search_news("artificial intelligence stocks", 10).await?;
/// println!("Found {} articles", news.count);
/// ```
#[allow(dead_code)]
pub async fn search_news(query: &str, count: usize) -> Result<NewsResponse, YahooError> {
    let client = get_yahoo_client().await?;
    let json = client.search(query, count).await?;
    let articles = parse_news_from_search(&json);

    Ok(NewsResponse {
        symbol: None,
        query: Some(query.to_string()),
        count: articles.len(),
        articles,
    })
}
