use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use finance_query_core::client::scraper::{
    scrape_earnings_calls_list, scrape_earnings_transcript_from_url,
};
use finance_query_core::{FetchClient, YahooError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub symbol: String,
    pub quarter: String,
    pub year: i32,
    pub date: String,
    pub transcript: String,
    pub participants: Vec<String>,
    pub metadata: TranscriptMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    pub source: String,
    #[serde(rename = "retrieved_at")]
    pub retrieved_at: String,
    #[serde(rename = "transcripts_id")]
    pub transcripts_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarningsTranscriptsResponse {
    pub symbol: String,
    pub transcripts: Vec<Transcript>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    #[serde(rename = "total_transcripts")]
    pub total_transcripts: u32,
    #[serde(rename = "filters_applied")]
    pub filters_applied: FiltersApplied,
    #[serde(rename = "retrieved_at")]
    pub retrieved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiltersApplied {
    pub quarter: String,
    pub year: i32,
}

pub async fn get_earnings_transcript(
    symbol: &str,
    quarter: Option<&str>,
    year: Option<i32>,
) -> Result<EarningsTranscriptsResponse, YahooError> {
    // Get fetch_client for scraper functions
    let fetch_client = Arc::new(FetchClient::new(None)?);

    // Get earnings calls list
    let calls_list = scrape_earnings_calls_list(&fetch_client, symbol).await?;

    // Filter by quarter/year if provided
    let filtered_calls: Vec<&Value> = calls_list
        .iter()
        .filter(|call| {
            let call_quarter = call.get("quarter").and_then(|v| v.as_str());
            let call_year = call.get("year").and_then(|v| v.as_i64()).map(|y| y as i32);

            let quarter_match = quarter.is_none_or(|q| {
                call_quarter.is_some_and(|cq| cq.to_uppercase() == q.to_uppercase())
            });

            let year_match = year.is_none_or(|y| call_year == Some(y));

            quarter_match && year_match
        })
        .collect();

    // Fetch transcripts for filtered calls
    let mut transcripts = Vec::new();
    for call in filtered_calls {
        if let Some(url) = call.get("url").and_then(|v| v.as_str())
            && let Ok(transcript_data) =
                scrape_earnings_transcript_from_url(&fetch_client, url).await
            && let Some(transcript) =
                parse_transcript_from_value(&transcript_data, symbol, call)
        {
            transcripts.push(transcript);
        }
    }

    // Build response
    let filters_applied = FiltersApplied {
        quarter: quarter.unwrap_or("all").to_string(),
        year: year.unwrap_or(0),
    };

    let total_transcripts = transcripts.len() as u32;
    Ok(EarningsTranscriptsResponse {
        symbol: symbol.to_string(),
        transcripts,
        metadata: ResponseMetadata {
            total_transcripts,
            filters_applied,
            retrieved_at: chrono::Utc::now().to_rfc3339(),
        },
    })
}

fn parse_transcript_from_value(data: &Value, symbol: &str, call: &Value) -> Option<Transcript> {
    let quarter = call.get("quarter")?.as_str()?.to_string();
    let year = call.get("year")?.as_i64()? as i32;

    // Extract transcript text from the data structure
    // The structure may vary, so we try multiple paths
    let transcript_text = data
        .get("transcriptContent")
        .or_else(|| data.get("transcript"))
        .or_else(|| data.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract participants/speakers
    let participants = data
        .get("participants")
        .or_else(|| data.get("speakers"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    p.as_str()
                        .or_else(|| p.get("name").and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract date
    let date = data
        .get("date")
        .or_else(|| data.get("eventDate"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(Transcript {
        symbol: symbol.to_string(),
        quarter,
        year,
        date,
        transcript: transcript_text,
        participants,
        metadata: TranscriptMetadata {
            source: "yahoo_finance".to_string(),
            retrieved_at: chrono::Utc::now().to_rfc3339(),
            transcripts_id: call.get("eventId")?.as_str()?.parse().ok()?,
        },
    })
}
