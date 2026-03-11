// Time Range Type
// Based on backend TimeRange enum from src/models/stock/stocks.rs
export interface CustomTimeRange {
  start_date?: string; // ISO 8601 format (Date to RFC3339)
  end_date?: string; // ISO 8601 format (Date to RFC3339)
}

export type TimeRange =
  | '7d'
  | '30d'
  | '90d'
  | '1y'
  | 'ytd'
  | { custom: CustomTimeRange }
  | 'all_time';

// Core Trading Metrics
export interface CoreMetrics {
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  break_even_trades: number;
  win_rate: number;
  loss_rate: number;
  total_pnl: number;
  net_profit_loss: number;
  gross_profit: number;
  gross_loss: number;
  average_win: number;
  average_loss: number;
  average_position_size: number;
  biggest_winner: number;
  biggest_loser: number;
  profit_factor: number;
  win_loss_ratio: number;
  max_consecutive_wins: number;
  max_consecutive_losses: number;
  total_commissions: number;
  average_commission_per_trade: number;
}

// Risk Metrics
export interface RiskMetrics {
  sharpe_ratio: number;
  sortino_ratio: number;
  calmar_ratio: number;
  maximum_drawdown: number;
  maximum_drawdown_percentage: number;
  maximum_drawdown_duration_days: number;
  current_drawdown: number;
  var_95: number;
  var_99: number;
  expected_shortfall_95: number;
  expected_shortfall_99: number;
  average_risk_per_trade: number;
  risk_reward_ratio: number;
  volatility: number;
  downside_deviation: number;
  ulcer_index: number;
  recovery_factor: number;
  sterling_ratio: number;
}

// Performance Metrics
export interface PerformanceMetrics {
  trade_expectancy: number;
  edge: number;
  average_hold_time_days: number;
  average_hold_time_winners_days: number;
  average_hold_time_losers_days: number;
  average_position_size: number;
  position_size_standard_deviation: number;
  position_size_variability: number;
  kelly_criterion: number;
  system_quality_number: number;
  payoff_ratio: number;
  average_r_multiple: number;
  r_multiple_standard_deviation: number;
  positive_r_multiple_count: number;
  negative_r_multiple_count: number;
  consistency_ratio: number;
  monthly_win_rate: number;
  quarterly_win_rate: number;
  average_slippage: number;
  commission_impact_percentage: number;
}

// Time Series Point
export interface TimeSeriesPoint {
  date: string;
  value: number;
  cumulative_value: number;
  trade_count: number;
}

// Time Series Data
export interface TimeSeriesData {
  daily_pnl: TimeSeriesPoint[];
  weekly_pnl: TimeSeriesPoint[];
  monthly_pnl: TimeSeriesPoint[];
  rolling_win_rate_20: TimeSeriesPoint[];
  rolling_win_rate_50: TimeSeriesPoint[];
  rolling_win_rate_100: TimeSeriesPoint[];
  rolling_sharpe_ratio_20: TimeSeriesPoint[];
  rolling_sharpe_ratio_50: TimeSeriesPoint[];
  profit_by_day_of_week: Record<string, number>;
  profit_by_month: Record<string, number>;
  drawdown_curve: TimeSeriesPoint[];
  cumulative_return: number;
  annualized_return: number;
  total_return_percentage: number;
  total_trades: number;
  
  // Legacy/computed fields for backward compatibility
  rolling_win_rate?: TimeSeriesPoint[];
  rolling_sharpe_ratio?: TimeSeriesPoint[];
}

// Group Type
export enum GroupType {
  Symbol = 'Symbol',
  Strategy = 'Strategy',
  TradeDirection = 'TradeDirection',
  TimePeriod = 'TimePeriod',
}

// Grouped Metrics
export interface GroupedMetrics {
  group_name: string;
  group_type: GroupType;
  core_metrics: CoreMetrics;
  risk_metrics: RiskMetrics;
  performance_metrics: PerformanceMetrics;
}

// Grouping Type for options
export enum GroupingType {
  Symbol = 'Symbol',
  Strategy = 'Strategy',
  TradeDirection = 'TradeDirection',
  TimePeriod = 'TimePeriod',
}

// Analytics Options
export interface AnalyticsOptions {
  include_time_series?: boolean;
  include_grouped_analytics?: boolean;
  grouping_types?: GroupingType[];
  confidence_levels?: number[];
  window_size?: number;
  risk_free_rate?: number;
}

// Comprehensive Analytics
export interface ComprehensiveAnalytics {
  core_metrics: CoreMetrics;
  risk_metrics: RiskMetrics;
  performance_metrics: PerformanceMetrics;
  time_series: TimeSeriesData;
  grouped_analytics: Record<string, GroupedMetrics>;
}

// API Request/Response Types
export interface AnalyticsRequest {
  time_range?: TimeRange;
  options?: AnalyticsOptions;
}

export interface CoreAnalyticsResponse {
  success: boolean;
  data: CoreMetrics | null;
  error?: string;
  message?: string; // Legacy support
}

export interface RiskAnalyticsResponse {
  success: boolean;
  data: RiskMetrics | null;
  error?: string;
  message?: string; // Legacy support
}

export interface DurationPerformanceMetrics {
  duration_bucket: string;
  trade_count: number;
  win_rate: number;
  total_pnl: number;
  avg_pnl: number;
  avg_hold_time_days: number;
  best_trade: number;
  worst_trade: number;
  profit_factor: number;
  winning_trades: number;
  losing_trades: number;
}

export interface DurationPerformanceResponse {
  duration_buckets: DurationPerformanceMetrics[];
  overall_metrics: CoreMetrics;
}

export interface PerformanceAnalyticsResponse {
  success: boolean;
  data: {
    performance_metrics: PerformanceMetrics;
    duration_performance: DurationPerformanceResponse;
  } | null;
  error?: string;
  message?: string; // Legacy support
}

export interface TimeSeriesAnalyticsResponse {
  success: boolean;
  data: TimeSeriesData | null;
  error?: string;
  message?: string; // Legacy support
}

export interface GroupedAnalyticsResponse {
  success: boolean;
  data: Record<string, GroupedMetrics> | null;
  error?: string;
  message?: string; // Legacy support
}

export interface ComprehensiveAnalyticsResponse {
  success: boolean;
  data: ComprehensiveAnalytics | null;
  error?: string;
  message?: string; // Legacy support
}

// Hook Return Types
export interface UseAnalyticsReturn {
  core_metrics: CoreMetrics | null;
  risk_metrics: RiskMetrics | null;
  performance_metrics: PerformanceMetrics | null;
  time_series: TimeSeriesData | null;
  grouped_analytics: Record<string, GroupedMetrics> | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsCoreReturn {
  data: CoreMetrics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsRiskReturn {
  data: RiskMetrics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsPerformanceReturn {
  data: {
    performance_metrics: PerformanceMetrics;
    duration_performance: DurationPerformanceResponse;
  } | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsTimeSeriesReturn {
  data: TimeSeriesData | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsGroupedReturn {
  data: Record<string, GroupedMetrics> | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAnalyticsComprehensiveReturn {
  data: ComprehensiveAnalytics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

// Individual Trade Analytics (uses new columns: profit_target, stop_loss, etc.)
export interface IndividualTradeAnalytics {
  trade_id: number;
  trade_type: string; // "stock" or "option"
  symbol: string;
  net_pnl: number;
  planned_risk_to_reward: number | null;
  realized_risk_to_reward: number | null;
  commission_impact: number;
  hold_time_days: number | null;
}

// Symbol-level Aggregate Analytics
export interface SymbolAnalytics {
  symbol: string;
  total_trades: number;
  stock_trades: number;
  option_trades: number;
  winning_trades: number;
  losing_trades: number;
  net_pnl: number;
  gross_profit: number;
  gross_loss: number;
  average_gain: number; // Average of winning trades
  average_loss: number; // Average of losing trades (absolute value)
  profit_factor: number;
  win_rate: number;
  expectancy: number;
  biggest_winner: number;
  biggest_loser: number;
}

// Response types
export interface IndividualTradeAnalyticsResponse {
  success: boolean;
  data: IndividualTradeAnalytics;
  error?: string;
}

export interface SymbolAnalyticsResponse {
  success: boolean;
  data: SymbolAnalytics;
  error?: string;
}

// Hook return types
export interface UseIndividualTradeAnalyticsReturn {
  data: IndividualTradeAnalytics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseSymbolAnalyticsReturn {
  data: SymbolAnalytics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

