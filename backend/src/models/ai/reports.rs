#![allow(dead_code)]

use crate::models::stock::stocks::TimeRange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Report type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    Comprehensive,
    Performance,
    Risk,
    Trading,
    Behavioral,
    Market,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportType::Comprehensive => write!(f, "comprehensive"),
            ReportType::Performance => write!(f, "performance"),
            ReportType::Risk => write!(f, "risk"),
            ReportType::Trading => write!(f, "trading"),
            ReportType::Behavioral => write!(f, "behavioral"),
            ReportType::Market => write!(f, "market"),
        }
    }
}

/// Report section enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportSection {
    Summary,
    Analytics,
    Insights,
    Trades,
    Patterns,
    Recommendations,
    RiskAnalysis,
    PerformanceMetrics,
    BehavioralAnalysis,
    MarketAnalysis,
}

impl std::fmt::Display for ReportSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportSection::Summary => write!(f, "summary"),
            ReportSection::Analytics => write!(f, "analytics"),
            ReportSection::Insights => write!(f, "insights"),
            ReportSection::Trades => write!(f, "trades"),
            ReportSection::Patterns => write!(f, "patterns"),
            ReportSection::Recommendations => write!(f, "recommendations"),
            ReportSection::RiskAnalysis => write!(f, "risk_analysis"),
            ReportSection::PerformanceMetrics => write!(f, "performance_metrics"),
            ReportSection::BehavioralAnalysis => write!(f, "behavioral_analysis"),
            ReportSection::MarketAnalysis => write!(f, "market_analysis"),
        }
    }
}

/// Report request structure
#[derive(Debug, Clone, Deserialize)]
pub struct ReportRequest {
    pub time_range: TimeRange,
    pub include_sections: Option<Vec<ReportSection>>,
    pub include_charts: Option<bool>,
    pub include_predictions: Option<bool>,
    pub format: Option<ReportFormat>,
}

/// Payload used when the frontend persists a streamed report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveReportRequest {
    pub time_range: TimeRange,
    pub title: String,
    pub summary: Option<String>,
    pub analytics: AnalyticsData,
    pub trades: Vec<TradeData>,
    pub recommendations: Option<Vec<String>>,
    pub risk_metrics: Option<RiskMetrics>,
    pub performance_metrics: Option<PerformanceMetrics>,
    pub metadata: ReportMetadata,
}

/// Report format enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportFormat {
    Json,
    Html,
    Pdf,
    Markdown,
}

/// Analytics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub total_pnl: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_gain: f64,
    pub avg_loss: f64,
    pub biggest_winner: f64,
    pub biggest_loser: f64,
    pub avg_hold_time_winners: f64,
    pub avg_hold_time_losers: f64,
    pub risk_reward_ratio: f64,
    pub trade_expectancy: f64,
    pub avg_position_size: f64,
    pub net_pnl: f64,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub break_even_trades: u32,
}

/// Trade data for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub id: String,
    pub symbol: String,
    pub trade_type: String,
    pub quantity: i32,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub pnl: Option<f64>,
    pub entry_date: DateTime<Utc>,
    pub exit_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// Trading report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingReport {
    pub id: String,
    pub user_id: String,
    pub time_range: TimeRange,
    pub report_type: ReportType,
    pub title: String,
    pub summary: String,
    pub analytics: AnalyticsData,
    pub trades: Vec<TradeData>,
    pub recommendations: Vec<String>,
    pub risk_metrics: RiskMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub generated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: ReportMetadata,
}

/// Trading pattern structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub frequency: u32,
    pub success_rate: f64,
    pub avg_return: f64,
    pub confidence_score: f32,
    pub examples: Vec<String>,
}

/// Risk metrics structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
    pub volatility: f64,
    pub var_95: f64, // Value at Risk 95%
    pub var_99: f64, // Value at Risk 99%
    pub risk_score: f32,
    pub concentration_risk: f32,
    pub leverage_risk: f32,
}

/// Performance metrics structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub monthly_returns: Vec<f64>,
    pub quarterly_returns: Vec<f64>,
    pub yearly_returns: Vec<f64>,
    pub best_month: f64,
    pub worst_month: f64,
    pub consistency_score: f32,
    pub trend_direction: String,
    pub momentum_score: f32,
}

/// Behavioral insight structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralInsight {
    pub category: String,
    pub description: String,
    pub impact_score: f32,
    pub frequency: u32,
    pub recommendations: Vec<String>,
}

/// Market analysis structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub market_conditions: String,
    pub sector_performance: Vec<SectorPerformance>,
    pub correlation_analysis: Vec<CorrelationData>,
    pub market_timing_score: f32,
    pub recommendations: Vec<String>,
}

/// Sector performance structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorPerformance {
    pub sector: String,
    pub performance: f64,
    pub user_exposure: f64,
    pub recommendation: String,
}

/// Correlation data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationData {
    pub symbol: String,
    pub correlation_with_portfolio: f64,
    pub correlation_strength: String,
}

/// Report metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub trade_count: u32,
    pub analysis_period_days: u32,
    pub model_version: String,
    pub processing_time_ms: u64,
    pub data_quality_score: f32,
    pub sections_included: Vec<ReportSection>,
    pub charts_generated: u32,
}

impl TradingReport {
    pub fn new(
        user_id: String,
        time_range: TimeRange,
        report_type: ReportType,
        title: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            time_range,
            report_type,
            title,
            summary: String::new(),
            analytics: AnalyticsData {
                total_pnl: 0.0,
                win_rate: 0.0,
                profit_factor: 0.0,
                avg_gain: 0.0,
                avg_loss: 0.0,
                biggest_winner: 0.0,
                biggest_loser: 0.0,
                avg_hold_time_winners: 0.0,
                avg_hold_time_losers: 0.0,
                risk_reward_ratio: 0.0,
                trade_expectancy: 0.0,
                avg_position_size: 0.0,
                net_pnl: 0.0,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                break_even_trades: 0,
            },
            trades: Vec::new(),
            recommendations: Vec::new(),
            risk_metrics: RiskMetrics {
                max_drawdown: 0.0,
                sharpe_ratio: 0.0,
                volatility: 0.0,
                var_95: 0.0,
                var_99: 0.0,
                risk_score: 0.0,
                concentration_risk: 0.0,
                leverage_risk: 0.0,
            },
            performance_metrics: PerformanceMetrics {
                monthly_returns: Vec::new(),
                quarterly_returns: Vec::new(),
                yearly_returns: Vec::new(),
                best_month: 0.0,
                worst_month: 0.0,
                consistency_score: 0.0,
                trend_direction: String::new(),
                momentum_score: 0.0,
            },
            generated_at: Utc::now(),
            expires_at: None,
            metadata: ReportMetadata {
                trade_count: 0,
                analysis_period_days: 0,
                model_version: "1.0".to_string(),
                processing_time_ms: 0,
                data_quality_score: 0.0,
                sections_included: Vec::new(),
                charts_generated: 0,
            },
        }
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    pub fn with_analytics(mut self, analytics: AnalyticsData) -> Self {
        self.analytics = analytics;
        self
    }

    pub fn with_trades(mut self, trades: Vec<TradeData>) -> Self {
        self.trades = trades;
        self
    }

    pub fn with_recommendations(mut self, recommendations: Vec<String>) -> Self {
        self.recommendations = recommendations;
        self
    }

    pub fn with_metadata(mut self, metadata: ReportMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn set_expiration(&mut self, hours_from_now: u32) {
        self.expires_at = Some(Utc::now() + chrono::Duration::hours(hours_from_now as i64));
    }

    pub fn get_total_recommendations(&self) -> usize {
        self.recommendations.len()
    }
}

/// Report list response
#[derive(Debug, Serialize)]
pub struct ReportListResponse {
    pub reports: Vec<ReportSummary>,
    pub total_count: u32,
    pub has_more: bool,
}

/// Report summary for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: String,
    pub title: String,
    pub report_type: ReportType,
    pub time_range: TimeRange,
    pub generated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub trade_count: u32,
    pub recommendations_count: u32,
}

impl From<TradingReport> for ReportSummary {
    fn from(report: TradingReport) -> Self {
        let recommendations_count = report.get_total_recommendations() as u32;
        Self {
            id: report.id,
            title: report.title,
            report_type: report.report_type.clone(),
            time_range: report.time_range.clone(),
            generated_at: report.generated_at,
            expires_at: report.expires_at,
            total_pnl: report.analytics.total_pnl,
            win_rate: report.analytics.win_rate,
            trade_count: report.analytics.total_trades,
            recommendations_count,
        }
    }
}

/// Report generation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportGenerationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
}

/// Report generation task
#[derive(Debug, Clone)]
pub struct ReportGenerationTask {
    pub task_id: String,
    pub user_id: String,
    pub report_request: ReportRequest,
    pub status: ReportGenerationStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub result_report_id: Option<String>,
    pub progress_percentage: u8,
}

impl ReportGenerationTask {
    pub fn new(user_id: String, report_request: ReportRequest) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            user_id,
            report_request,
            status: ReportGenerationStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            result_report_id: None,
            progress_percentage: 0,
        }
    }

    pub fn start(&mut self) {
        self.status = ReportGenerationStatus::Processing;
        self.started_at = Some(Utc::now());
        self.progress_percentage = 10;
    }

    pub fn update_progress(&mut self, percentage: u8) {
        self.progress_percentage = percentage.min(90); // Don't go to 100% until completed
    }

    pub fn complete(&mut self, report_id: String) {
        self.status = ReportGenerationStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.result_report_id = Some(report_id);
        self.progress_percentage = 100;
    }

    pub fn fail(&mut self, error_message: String) {
        self.status = ReportGenerationStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error_message);
        self.progress_percentage = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_report_creation() {
        let report = TradingReport::new(
            "user123".to_string(),
            TimeRange::ThirtyDays,
            ReportType::Comprehensive,
            "Monthly Report".to_string(),
        );

        assert_eq!(report.user_id, "user123");
        assert_eq!(report.report_type, ReportType::Comprehensive);
        assert_eq!(report.title, "Monthly Report");
        assert_eq!(report.analytics.total_trades, 0);
    }

    #[test]
    fn test_trading_report_with_data() {
        let analytics = AnalyticsData {
            total_pnl: 1000.0,
            win_rate: 65.0,
            profit_factor: 1.5,
            avg_gain: 200.0,
            avg_loss: -100.0,
            biggest_winner: 500.0,
            biggest_loser: -300.0,
            avg_hold_time_winners: 5.0,
            avg_hold_time_losers: 3.0,
            risk_reward_ratio: 2.0,
            trade_expectancy: 50.0,
            avg_position_size: 1000.0,
            net_pnl: 1000.0,
            total_trades: 10,
            winning_trades: 7,
            losing_trades: 3,
            break_even_trades: 0,
        };

        let report = TradingReport::new(
            "user123".to_string(),
            TimeRange::ThirtyDays,
            ReportType::Performance,
            "Performance Report".to_string(),
        )
        .with_analytics(analytics);

        assert_eq!(report.analytics.total_pnl, 1000.0);
        assert_eq!(report.analytics.win_rate, 65.0);
        assert_eq!(report.analytics.total_trades, 10);
    }

    #[test]
    fn test_report_expiration() {
        let mut report = TradingReport::new(
            "user123".to_string(),
            TimeRange::SevenDays,
            ReportType::Risk,
            "Risk Report".to_string(),
        );

        assert!(!report.is_expired());

        report.set_expiration(24); // 24 hours from now
        assert!(!report.is_expired());

        // Set expiration in the past
        report.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(report.is_expired());
    }

    #[test]
    fn test_report_generation_task() {
        let request = ReportRequest {
            time_range: TimeRange::ThirtyDays,
            include_sections: Some(vec![ReportSection::Summary, ReportSection::Analytics]),
            include_charts: Some(true),
            include_predictions: Some(false),
            format: Some(ReportFormat::Json),
        };

        let mut task = ReportGenerationTask::new("user123".to_string(), request);
        assert_eq!(task.status, ReportGenerationStatus::Pending);
        assert_eq!(task.progress_percentage, 0);

        task.start();
        assert_eq!(task.status, ReportGenerationStatus::Processing);
        assert_eq!(task.progress_percentage, 10);

        task.update_progress(50);
        assert_eq!(task.progress_percentage, 50);

        task.complete("report123".to_string());
        assert_eq!(task.status, ReportGenerationStatus::Completed);
        assert_eq!(task.progress_percentage, 100);
        assert_eq!(task.result_report_id, Some("report123".to_string()));
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Comprehensive.to_string(), "comprehensive");
        assert_eq!(ReportType::Performance.to_string(), "performance");
        assert_eq!(ReportType::Risk.to_string(), "risk");
    }

    #[test]
    fn test_report_section_display() {
        assert_eq!(ReportSection::Summary.to_string(), "summary");
        assert_eq!(ReportSection::Analytics.to_string(), "analytics");
        assert_eq!(ReportSection::Insights.to_string(), "insights");
    }
}
