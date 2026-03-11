use crate::models::analytics::TimeSeriesInterval;
use crate::models::stock::stocks::TimeRange;
use serde::{Deserialize, Serialize};

/// Configuration options for analytics calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsOptions {
    pub time_range: TimeRange,
    pub include_time_series: bool,
    pub time_series_interval: TimeSeriesInterval,
    pub include_grouped_analytics: bool,
    pub grouping_types: Vec<GroupingType>,
    pub risk_free_rate: f64,
    pub confidence_levels: Vec<f64>,
}

/// Types of grouping for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupingType {
    Symbol,
    Strategy,
    TradeDirection,
    TimePeriod,
}

impl Default for AnalyticsOptions {
    fn default() -> Self {
        Self {
            time_range: TimeRange::AllTime,
            include_time_series: true,
            time_series_interval: TimeSeriesInterval::Daily,
            include_grouped_analytics: false,
            grouping_types: vec![GroupingType::Symbol],
            risk_free_rate: 0.02, // 2% annual risk-free rate
            confidence_levels: vec![0.95, 0.99],
        }
    }
}
