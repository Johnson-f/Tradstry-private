// AI Reports Types
// Based on backend models from src/models/ai/reports.rs

export type TimeRange = 
  | '7d' 
  | '30d' 
  | '90d' 
  | '1y' 
  | 'ytd' 
  | 'custom' 
  | 'all_time';

export type ReportType = 
  | 'comprehensive'
  | 'performance'
  | 'risk'
  | 'trading'
  | 'behavioral'
  | 'market';

export type ReportSection = 
  | 'summary'
  | 'analytics'
  | 'insights'
  | 'trades'
  | 'patterns'
  | 'recommendations'
  | 'risk_analysis'
  | 'performance_metrics'
  | 'behavioral_analysis'
  | 'market_analysis';

export type ReportFormat = 
  | 'json'
  | 'html'
  | 'pdf'
  | 'markdown';

// Request types
export interface ReportRequest {
  time_range: TimeRange;
  include_sections?: ReportSection[];
  include_charts?: boolean;
  include_predictions?: boolean;
  format?: ReportFormat;
}

export interface CustomTimeRange {
  start_date?: string; // ISO 8601 format
  end_date?: string; // ISO 8601 format
}

// Analytics and metrics types - matching backend AnalyticsData
export interface AnalyticsData {
  total_pnl: number;
  win_rate: number;
  profit_factor: number;
  avg_gain: number;
  avg_loss: number;
  biggest_winner: number;
  biggest_loser: number;
  avg_hold_time_winners: number;
  avg_hold_time_losers: number;
  risk_reward_ratio: number;
  trade_expectancy: number;
  avg_position_size: number;
  net_pnl: number;
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  break_even_trades: number;
}

export interface RiskMetrics {
  max_drawdown: number;
  sharpe_ratio: number;
  volatility: number;
  var_95: number; // Value at Risk 95%
  var_99: number; // Value at Risk 99%
  risk_score: number;
  concentration_risk: number;
  leverage_risk: number;
}

export interface PerformanceMetrics {
  monthly_returns: number[];
  quarterly_returns: number[];
  yearly_returns: number[];
  best_month: number;
  worst_month: number;
  consistency_score: number;
  trend_direction: string;
  momentum_score: number;
}

export interface TradingPattern {
  id: string;
  name: string;
  description: string;
  frequency: number;
  success_rate: number;
  avg_return: number;
  confidence_score: number;
  examples: string[];
}

export interface TradeData {
  id: string;
  symbol: string;
  trade_type: string;
  quantity: number;
  entry_price: number;
  exit_price?: number;
  pnl?: number;
  entry_date: string; // ISO 8601 format (DateTime<Utc> serialized)
  exit_date?: string; // ISO 8601 format (DateTime<Utc> serialized)
  notes?: string;
}

export interface BehavioralInsight {
  category: string;
  description: string;
  impact_score: number;
  frequency: number;
  recommendations: string[];
}

export interface SectorPerformance {
  sector: string;
  performance: number;
  user_exposure: number;
  recommendation: string;
}

export interface CorrelationData {
  symbol: string;
  correlation_with_portfolio: number;
  correlation_strength: string;
}

export interface MarketAnalysis {
  market_conditions: string;
  sector_performance: SectorPerformance[];
  correlation_analysis: CorrelationData[];
  market_timing_score: number;
  recommendations: string[];
}

export interface ReportMetadata {
  trade_count: number;
  analysis_period_days: number;
  model_version: string;
  processing_time_ms: number;
  data_quality_score: number;
  sections_included: ReportSection[];
  charts_generated: number;
}

// Main report structure - matching backend TradingReport
export interface TradingReport {
  id: string;
  user_id: string;
  time_range: TimeRange;
  report_type: ReportType;
  title: string;
  summary: string;
  analytics: AnalyticsData;
  trades: TradeData[];
  recommendations: string[];
  risk_metrics: RiskMetrics;
  performance_metrics: PerformanceMetrics;
  generated_at: string; // ISO 8601 format (DateTime<Utc> serialized)
  expires_at?: string; // ISO 8601 format (DateTime<Utc> serialized)
  metadata: ReportMetadata;
}

// Report summary for list view - matching backend ReportSummary
export interface ReportSummary {
  id: string;
  title: string;
  report_type: ReportType;
  time_range: TimeRange;
  generated_at: string; // ISO 8601 format (DateTime<Utc> serialized)
  expires_at?: string; // ISO 8601 format (DateTime<Utc> serialized)
  total_pnl: number;
  win_rate: number;
  trade_count: number;
  recommendations_count: number;
}

// WebSocket streaming event payloads for AI reports
export interface AiReportProgressEvent {
  stage?: string;
  time_range?: unknown;
  total_report_types?: number;
  report_type?: string;
  index?: number;
  total?: number;
}

export interface AiReportChunkEvent {
  report_type?: string;
  index?: number;
  total?: number;
  report: TradingReport;
}

export interface AiReportCompleteEvent {
  stage?: string;
  total_reports?: number;
  reports?: TradingReport[];
}

// Saved report shape returned by /history and /history/{id}
export interface ReportHistoryItem {
  id: string;
  time_range: TimeRange | unknown;
  title: string;
  analytics: AnalyticsData | unknown;
  trades: TradeData[] | unknown;
  recommendations: string[] | unknown;
  metrics: unknown;
  metadata: ReportMetadata | unknown;
  summary: string;
  created_at: string; // ISO 8601
}

export type ReportHistoryResponse = ReportHistoryItem[];

// API Response wrapper
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

// Payload shape expected by POST /api/reports/save
export interface SaveReportPayload {
  time_range: TimeRange;
  title: string;
  summary: string;
  analytics: AnalyticsData;
  trades: Array<
    TradeData & {
      entry_date: string;
      exit_date?: string;
    }
  >;
  recommendations?: string[];
  risk_metrics?: RiskMetrics;
  performance_metrics?: PerformanceMetrics;
  metadata: ReportMetadata;
}

// Service method types
export interface AIReportsServiceInterface {
  generateReport(request: ReportRequest): Promise<TradingReport[]>;
  saveReport(payload: SaveReportPayload): Promise<string>;
  getHistory(limit?: number, offset?: number): Promise<ReportHistoryResponse>;
  getHistoryById(id: string): Promise<ReportHistoryItem | TradingReport | unknown>;
}

export interface ReportFilters {
  time_range?: TimeRange;
  report_type?: ReportType;
  limit?: number;
  offset?: number;
}

// Hook return types
export interface UseAIReportsReturn {
  // State
  reports: ReportHistoryItem[];
  currentReport: ReportHistoryItem | TradingReport | null;
  loading: boolean;
  generating: boolean;
  error: Error | null;
  
  // Actions
  generateReport: (request: ReportRequest) => Promise<void>;
  saveReport: (payload: unknown) => Promise<string>;
  getHistory: (filters?: ReportFilters) => Promise<void>;
  getHistoryItem: (reportId: string) => Promise<void>;
  refreshReports: () => Promise<void>;
  
  // Utilities
  clearError: () => void;
  clearCurrentReport: () => void;
}

// Error types
export class AIReportsError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'AIReportsError';
  }
}

export class ValidationError extends AIReportsError {
  constructor(message: string, field?: string) {
    super(message, 'VALIDATION_ERROR', { field });
  }
}

export class GenerationError extends AIReportsError {
  constructor(message: string, taskId?: string) {
    super(message, 'GENERATION_ERROR', { taskId });
  }
}

// Constants
export const REPORT_TYPES: Record<ReportType, string> = {
  comprehensive: 'Comprehensive Report',
  performance: 'Performance Report',
  risk: 'Risk Assessment Report',
  trading: 'Trading Analysis Report',
  behavioral: 'Behavioral Analysis Report',
  market: 'Market Analysis Report',
};

export const REPORT_SECTIONS: Record<ReportSection, string> = {
  summary: 'Executive Summary',
  analytics: 'Analytics Overview',
  insights: 'Key Insights',
  trades: 'Trade Analysis',
  patterns: 'Trading Patterns',
  recommendations: 'Recommendations',
  risk_analysis: 'Risk Analysis',
  performance_metrics: 'Performance Metrics',
  behavioral_analysis: 'Behavioral Analysis',
  market_analysis: 'Market Analysis',
};

export const REPORT_FORMATS: Record<ReportFormat, string> = {
  json: 'JSON',
  html: 'HTML',
  pdf: 'PDF',
  markdown: 'Markdown',
};

export const TIME_RANGES: Record<TimeRange, string> = {
  '7d': 'Last 7 Days',
  '30d': 'Last 30 Days',
  '90d': 'Last 90 Days',
  '1y': 'Last Year',
  'ytd': 'Year to Date',
  'custom': 'Custom Range',
  'all_time': 'All Time',
};

export const DEFAULT_REPORT_FILTERS: ReportFilters = {
  limit: 10,
  offset: 0,
};
