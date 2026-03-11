import { apiClient } from './api-client';
import type {
  AnalysisRequest,
  AnalysisResponse,
  AnalysisErrorResponse,
  FullAnalysis,
  AnalysisRecord,
} from '@/lib/types/ai-analysis';

export class AIAnalysisService {
  private readonly endpoint = '/analysis/generate';

  /**
   * Generate trading analysis bundle for a period
   * @param request - Analysis request with period
   * @returns Full analysis bundle (weaknesses, strengths, patterns, comprehensive)
   * @throws ApiError if request fails
   */
  async generateAnalysis(
    request: AnalysisRequest
  ): Promise<FullAnalysis> {
    try {
      const response = await apiClient.post<AnalysisResponse>(
        this.endpoint,
        request
      );

      if (!response.success) {
        const errorResponse = response as AnalysisErrorResponse;
        throw new Error(
          `${errorResponse.error_type}: ${errorResponse.error}`
        );
      }

      return response.data;
    } catch (error) {
      // Re-throw with more context
      if (error instanceof Error) {
        throw error;
      }
      throw new Error(
        `Failed to generate analysis: ${String(error)}`
      );
    }
  }

  /**
   * Convenience generator (same as generateAnalysis)
   */
  async analyzeAll(period: AnalysisRequest['period']): Promise<FullAnalysis> {
    return this.generateAnalysis({ period });
  }

  /**
   * Sync completed analysis to backend storage
   */
  async syncAnalysis(payload: { timeRange: string; title: string; content: string }) {
    return apiClient.post<{ id: string }>("/analysis/save", {
      time_range: payload.timeRange,
      title: payload.title,
      content: payload.content,
    });
  }

  /**
   * Fetch stored analysis history (newest first)
   */
  async fetchHistory(params?: { limit?: number; offset?: number }): Promise<AnalysisRecord[]> {
    const response = await apiClient.get<{ success: true; data: AnalysisRecord[] }>(
      "/analysis/history",
      { params }
    );
    return response.data;
  }

  /**
   * Fetch a single analysis record by id
   */
  async fetchHistoryItem(id: string): Promise<AnalysisRecord> {
    const response = await apiClient.get<{ success: true; data: AnalysisRecord }>(
      `/analysis/history/${id}`
    );
    return response.data;
  }
}

// Export singleton instance
export const aiAnalysisService = new AIAnalysisService();
export default aiAnalysisService;
