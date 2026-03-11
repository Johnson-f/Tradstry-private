pub mod core;
pub mod options;
pub mod performance;
pub mod risk;
pub mod time_series;

pub use core::CoreMetrics;
pub use options::AnalyticsOptions;
pub use performance::PerformanceMetrics;
pub use risk::RiskMetrics;
pub use time_series::TimeSeriesData;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Comprehensive analytics result containing all metric categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveAnalytics {
    pub core_metrics: CoreMetrics,
    pub risk_metrics: RiskMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub time_series: TimeSeriesData,
    pub grouped_analytics: HashMap<String, GroupedMetrics>,
}

/// Grouped analytics for specific symbols or strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedMetrics {
    pub group_name: String,
    pub group_type: GroupType,
    pub core_metrics: CoreMetrics,
    pub risk_metrics: RiskMetrics,
    pub performance_metrics: PerformanceMetrics,
}

/// Type of grouping applied to analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupType {
    Symbol,
    Strategy,
    TradeDirection,
    TimePeriod,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub date: String,
    pub value: f64,
    pub cumulative_value: f64,
    pub trade_count: u32, // Number of trades in this time period
}

/// Time series interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesInterval {
    Daily,
    Weekly,
    Monthly,
}
