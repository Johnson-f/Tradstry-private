pub mod core_metrics;
pub mod grouping;
pub mod performance_metrics;
pub mod playbook_analytics;
pub mod risk_metrics;
pub mod time_series;

use crate::models::analytics::{
    AnalyticsOptions, ComprehensiveAnalytics, CoreMetrics, PerformanceMetrics, RiskMetrics,
    TimeSeriesData,
};
use crate::models::stock::stocks::TimeRange;
use anyhow::Result;
use libsql::Connection;

/// Main analytics engine that orchestrates all calculations
pub struct AnalyticsEngine;

impl AnalyticsEngine {
    /// Create a new analytics engine instance
    pub fn new() -> Self {
        Self
    }

    /// Calculate comprehensive analytics for all metrics
    pub async fn calculate_comprehensive_analytics(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
        options: AnalyticsOptions,
    ) -> Result<ComprehensiveAnalytics> {
        // Calculate core metrics
        let core_metrics = self.calculate_core_metrics(conn, time_range).await?;

        // Calculate risk metrics
        let risk_metrics = self
            .calculate_risk_metrics(conn, time_range, &options)
            .await?;

        // Calculate performance metrics
        let performance_metrics = self.calculate_performance_metrics(conn, time_range).await?;

        // Calculate time series data if requested
        let time_series = if options.include_time_series {
            self.calculate_time_series_data(conn, time_range, &options)
                .await?
        } else {
            TimeSeriesData::default()
        };

        // Calculate grouped analytics if requested
        let grouped_analytics = if options.include_grouped_analytics {
            self.calculate_grouped_analytics(conn, time_range, &options)
                .await?
        } else {
            std::collections::HashMap::new()
        };

        Ok(ComprehensiveAnalytics {
            core_metrics,
            risk_metrics,
            performance_metrics,
            time_series,
            grouped_analytics,
        })
    }

    /// Calculate core trading metrics
    pub async fn calculate_core_metrics(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
    ) -> Result<CoreMetrics> {
        core_metrics::calculate_core_metrics(conn, time_range).await
    }

    /// Calculate risk-adjusted metrics
    pub async fn calculate_risk_metrics(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
        options: &AnalyticsOptions,
    ) -> Result<RiskMetrics> {
        risk_metrics::calculate_risk_metrics(conn, time_range, options).await
    }

    /// Calculate performance metrics
    pub async fn calculate_performance_metrics(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
    ) -> Result<PerformanceMetrics> {
        performance_metrics::calculate_performance_metrics(conn, time_range).await
    }

    /// Calculate time series data
    pub async fn calculate_time_series_data(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
        options: &AnalyticsOptions,
    ) -> Result<TimeSeriesData> {
        time_series::calculate_time_series_data(conn, time_range, options).await
    }

    /// Calculate grouped analytics
    pub async fn calculate_grouped_analytics(
        &self,
        conn: &Connection,
        time_range: &TimeRange,
        options: &AnalyticsOptions,
    ) -> Result<std::collections::HashMap<String, crate::models::analytics::GroupedMetrics>> {
        grouping::calculate_grouped_analytics(conn, time_range, options).await
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}
