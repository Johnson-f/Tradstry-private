/**
 * AI Analysis Types
 * Matches backend models/ai/analysis.rs
 */

export type AnalysisPeriod =
  | 'last_7_days'
  | 'last_30_days'
  | 'last_90_days'
  | 'this_month'
  | 'last_month'
  | 'this_quarter'
  | 'last_quarter'
  | 'this_year'
  | 'last_year';

export type AnalysisErrorType =
  | 'insufficient_trades'
  | 'no_trades'
  | 'qdrant_unavailable'
  | 'ai_generation'
  | 'invalid_period';

export interface AnalysisRequest {
  period: AnalysisPeriod;
}

export interface AnalysisResult {
  period: AnalysisPeriod;
  content: string;
  trades_analyzed: number;
  generated_at: string;
  focus: 'weaknesses' | 'strengths' | 'patterns';
}

export interface ComprehensiveAnalysis {
  period: AnalysisPeriod;
  content: string;
  current_trades_count: number;
  previous_trades_count: number;
  has_comparison: boolean;
  generated_at: string;
}

export interface FullAnalysis {
  period: AnalysisPeriod;
  weaknesses: AnalysisResult;
  strengths: AnalysisResult;
  patterns: AnalysisResult;
  comprehensive: ComprehensiveAnalysis;
}

// WebSocket streaming payloads
export type AiAnalysisStage =
  | 'start'
  | 'weaknesses'
  | 'strengths'
  | 'patterns'
  | 'comprehensive'
  | 'complete';

export interface AiAnalysisProgressEvent {
  stage: AiAnalysisStage;
  status?: 'generating';
  period: string;
}

export interface AiAnalysisSectionEvent {
  section: 'weaknesses' | 'strengths' | 'patterns' | 'comprehensive';
  content: string;
  trades_analyzed?: number;
  current_trades_count?: number;
  previous_trades_count?: number;
  has_comparison?: boolean;
  generated_at: string;
  period: AnalysisPeriod;
}

export interface AiAnalysisCompleteEvent {
  stage: 'complete';
  period: string;
}

export type AiAnalysisWsEvent =
  | AiAnalysisProgressEvent
  | AiAnalysisSectionEvent
  | AiAnalysisCompleteEvent;

export interface AnalysisErrorResponse {
  success: false;
  error: string;
  error_type: AnalysisErrorType;
}

export interface AnalysisSuccessResponse {
  success: true;
  data: FullAnalysis;
}

export type AnalysisResponse = AnalysisSuccessResponse | AnalysisErrorResponse;

// Stored analysis record (persisted in ai_analysis table)
export interface AnalysisRecord {
  id: string;
  time_range: string;
  title: string;
  content: string;
  created_at: string;
}

export interface AnalysisHistoryItemResponse {
  success: true;
  data: AnalysisRecord;
}

/**
 * Helper function to check if response is an error
 */
export function isAnalysisError(
  response: AnalysisResponse
): response is AnalysisErrorResponse {
  return !response.success;
}
