use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Analysis period options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisPeriod {
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[serde(rename = "last_90_days")]
    Last90Days,
    #[serde(rename = "this_month")]
    ThisMonth,
    #[serde(rename = "last_month")]
    LastMonth,
    #[serde(rename = "this_quarter")]
    ThisQuarter,
    #[serde(rename = "last_quarter")]
    LastQuarter,
    #[serde(rename = "this_year")]
    ThisYear,
    #[serde(rename = "last_year")]
    LastYear,
}

impl AnalysisPeriod {
    pub fn to_date_range(&self) -> DateRange {
        let now = Utc::now();

        match self {
            Self::Last7Days => DateRange {
                start: now - Duration::days(7),
                end: now,
            },
            Self::Last30Days => DateRange {
                start: now - Duration::days(30),
                end: now,
            },
            Self::Last90Days => DateRange {
                start: now - Duration::days(90),
                end: now,
            },
            Self::ThisMonth => {
                let start = now
                    .date_naive()
                    .with_day(1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                DateRange { start, end: now }
            }
            Self::LastMonth => {
                let this_month_start = now.date_naive().with_day(1).unwrap();
                let last_month_end = this_month_start - Duration::days(1);
                let last_month_start = last_month_end.with_day(1).unwrap();
                DateRange {
                    start: last_month_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    end: last_month_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                }
            }
            Self::ThisQuarter => {
                let month = now.month();
                let quarter_start_month = ((month - 1) / 3) * 3 + 1;
                let start = now
                    .date_naive()
                    .with_month(quarter_start_month)
                    .and_then(|d| d.with_day(1))
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                DateRange { start, end: now }
            }
            Self::LastQuarter => {
                let month = now.month();
                let current_quarter_start_month = ((month - 1) / 3) * 3 + 1;
                let last_quarter_end_month = if current_quarter_start_month == 1 {
                    12
                } else {
                    current_quarter_start_month - 1
                };
                let last_quarter_start_month = ((last_quarter_end_month - 1) / 3) * 3 + 1;

                let last_quarter_end = now
                    .date_naive()
                    .with_month(last_quarter_end_month)
                    .map(|d| {
                        chrono::NaiveDate::from_ymd_opt(d.year(), last_quarter_end_month, 1)
                            .unwrap()
                            .with_day(1)
                            .unwrap()
                            .with_month(last_quarter_end_month + 1)
                            .map(|d| d - Duration::days(1))
                            .unwrap_or_else(|| d.with_day(28).unwrap())
                    })
                    .unwrap()
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc();

                let last_quarter_start = last_quarter_end
                    .date_naive()
                    .with_month(last_quarter_start_month)
                    .and_then(|d| d.with_day(1))
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();

                DateRange {
                    start: last_quarter_start,
                    end: last_quarter_end,
                }
            }
            Self::ThisYear => {
                let start = now
                    .date_naive()
                    .with_month(1)
                    .and_then(|d| d.with_day(1))
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                DateRange { start, end: now }
            }
            Self::LastYear => {
                let last_year = now.year() - 1;
                let start = chrono::NaiveDate::from_ymd_opt(last_year, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc();
                let end = chrono::NaiveDate::from_ymd_opt(last_year, 12, 31)
                    .unwrap()
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc();
                DateRange { start, end }
            }
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Self::Last7Days => Self::Last7Days, // Previous 7 days (same logic)
            Self::Last30Days => Self::Last30Days, // Previous 30 days (same logic)
            Self::ThisMonth => Self::LastMonth,
            Self::ThisQuarter => Self::LastQuarter,
            Self::ThisYear => Self::LastYear,
            _ => self.clone(), // For "last" periods, return same (already historical)
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Last7Days => "Last 7 Days",
            Self::Last30Days => "Last 30 Days",
            Self::Last90Days => "Last 90 Days",
            Self::ThisMonth => "This Month",
            Self::LastMonth => "Last Month",
            Self::ThisQuarter => "This Quarter",
            Self::LastQuarter => "Last Quarter",
            Self::ThisYear => "This Year",
            Self::LastYear => "Last Year",
        }
    }
}

/// Date range for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Analysis focus area
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisFocus {
    #[serde(rename = "weaknesses")]
    Weaknesses,
    #[serde(rename = "strengths")]
    Strengths,
    #[serde(rename = "patterns")]
    Patterns,
    #[serde(rename = "overall")]
    Overall,
}

/// Analysis request
#[derive(Debug, Deserialize)]
pub struct AnalysisRequest {
    pub period: AnalysisPeriod,
}

/// Save analysis payload from client sync
#[derive(Debug, Deserialize)]
pub struct SaveAnalysisRequest {
    pub time_range: String,
    pub title: String,
    pub content: String,
}

/// Stored analysis record pulled from user database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRecord {
    pub id: String,
    pub time_range: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

/// Save analysis response
#[derive(Debug, Serialize)]
pub struct SaveAnalysisResponse {
    pub id: String,
}

/// Analysis result
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub focus: AnalysisFocus,
    pub period: AnalysisPeriod,
    pub content: String,
    pub trades_analyzed: usize,
    pub generated_at: String,
}

/// Comprehensive analysis with comparison
#[derive(Debug, Serialize)]
pub struct ComprehensiveAnalysis {
    pub period: AnalysisPeriod,
    pub content: String,
    pub current_trades_count: usize,
    pub previous_trades_count: usize,
    pub has_comparison: bool,
    pub generated_at: String,
}

/// Full analysis bundle (auto mode)
#[derive(Debug, Serialize)]
pub struct FullAnalysis {
    pub period: AnalysisPeriod,
    pub weaknesses: AnalysisResult,
    pub strengths: AnalysisResult,
    pub patterns: AnalysisResult,
    pub comprehensive: ComprehensiveAnalysis,
}

/// Custom error types for analysis
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error(
        "Insufficient trades: need at least {required} trades for analysis, but only have {actual} trades in this period"
    )]
    InsufficientTrades { required: usize, actual: usize },

    #[error("No trades found in {}. Please add trades to generate analysis.", period.description())]
    NoTrades { period: AnalysisPeriod },

    #[error(
        "Vector database is temporarily unavailable: {message}. Please try again in a few moments."
    )]
    QdrantUnavailable { message: String },

    #[error("AI analysis generation failed: {message}. Please try again.")]
    AIGeneration { message: String },

    #[error("Invalid period: {message}")]
    #[allow(dead_code)]
    InvalidPeriod { message: String },
}

impl ResponseError for AnalysisError {
    fn error_response(&self) -> HttpResponse {
        let status = match self {
            AnalysisError::InsufficientTrades { .. } => StatusCode::BAD_REQUEST,
            AnalysisError::NoTrades { .. } => StatusCode::NOT_FOUND,
            AnalysisError::QdrantUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AnalysisError::AIGeneration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AnalysisError::InvalidPeriod { .. } => StatusCode::BAD_REQUEST,
        };

        HttpResponse::build(status).json(serde_json::json!({
            "success": false,
            "error": self.to_string(),
            "error_type": match self {
                AnalysisError::InsufficientTrades { .. } => "insufficient_trades",
                AnalysisError::NoTrades { .. } => "no_trades",
                AnalysisError::QdrantUnavailable { .. } => "qdrant_unavailable",
                AnalysisError::AIGeneration { .. } => "ai_generation",
                AnalysisError::InvalidPeriod { .. } => "invalid_period",
            }
        }))
    }
}
