import { apiConfig, getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type {
  AnalyticsRequest,
  CoreAnalyticsResponse,
  RiskAnalyticsResponse,
  PerformanceAnalyticsResponse,
  TimeSeriesAnalyticsResponse,
  GroupedAnalyticsResponse,
  ComprehensiveAnalyticsResponse,
  IndividualTradeAnalyticsResponse,
  SymbolAnalyticsResponse,
} from '@/lib/types/analytics';

/**
 * Analytics Service
 * Handles all analytics API calls to the Rust backend
 */
export class AnalyticsService {
  private baseURL: string;
  private supabase;

  constructor() {
    this.baseURL = apiConfig.baseURL + apiConfig.apiPrefix;
    this.supabase = createClient();
  }

  /**
   * Get core analytics (basic trading metrics)
   */
  async getCoreAnalytics(request?: AnalyticsRequest): Promise<CoreAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.core
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch core analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get risk metrics analytics
   */
  async getRiskAnalytics(request?: AnalyticsRequest): Promise<RiskAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.risk
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch risk analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get performance metrics analytics
   */
  async getPerformanceAnalytics(
    request?: AnalyticsRequest
  ): Promise<PerformanceAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.performance
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch performance analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get time series analytics data
   */
  async getTimeSeriesAnalytics(
    request?: AnalyticsRequest
  ): Promise<TimeSeriesAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.timeSeries
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch time series analytics: ${response.statusText}`);
    }

    const result = await response.json();
    
    // Transform backend response to include legacy fields for backward compatibility
    if (result.success && result.data) {
      result.data = {
        ...result.data,
        // Use rolling_win_rate_50 as default rolling_win_rate for backward compatibility
        rolling_win_rate: result.data.rolling_win_rate_50 || [],
        // Use rolling_sharpe_ratio_50 as default rolling_sharpe_ratio for backward compatibility
        rolling_sharpe_ratio: result.data.rolling_sharpe_ratio_50 || [],
      };
    }
    
    return result;
  }

  /**
   * Get grouped analytics (by symbol, strategy, direction, etc.)
   */
  async getGroupedAnalytics(
    request?: AnalyticsRequest
  ): Promise<GroupedAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.grouped
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch grouped analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get comprehensive analytics (all metrics combined)
   */
  async getComprehensiveAnalytics(
    request?: AnalyticsRequest
  ): Promise<ComprehensiveAnalyticsResponse> {
    const url = getFullUrl(
      apiConfig.endpoints.analytics.comprehensive
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch comprehensive analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get analytics for an individual trade
   * @param tradeId - The ID of the trade
   * @param tradeType - 'stock' or 'option'
   * @returns Analytics data including net P&L, risk-to-reward ratios, and commission impact
   */
  async getIndividualTradeAnalytics(
    tradeId: number,
    tradeType: 'stock' | 'option'
  ): Promise<IndividualTradeAnalyticsResponse> {
    const params = new URLSearchParams({
      trade_id: tradeId.toString(),
      trade_type: tradeType,
    });

    const url = `${getFullUrl(
      apiConfig.endpoints.analytics.trade
    )}?${params}`;
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch individual trade analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get aggregated analytics for all trades on a specific symbol
   * Useful for analyzing repeated trades on the same symbol (e.g., AAPL)
   * @param symbol - The stock/option symbol (e.g., 'AAPL')
   * @param timeRange - Optional time range filter (e.g., '7d', '30d', '90d', '1y', 'ytd', 'all_time')
   * @returns Aggregated analytics including profit factor, expectancy, win rate, etc.
   */
  async getSymbolAnalytics(
    symbol: string,
    timeRange?: string
  ): Promise<SymbolAnalyticsResponse> {
    const params = new URLSearchParams({
      symbol: symbol.toUpperCase(),
    });

    if (timeRange) {
      params.append('time_range', timeRange);
    }

    const url = `${getFullUrl(
      apiConfig.endpoints.analytics.symbol
    )}?${params}`;
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch symbol analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get stock analytics for a specific symbol (correct endpoint)
   * @param symbol - The stock symbol (e.g., 'AAPL')
   * @param timeRange - Optional time range filter
   */
  async getStocksAnalytics(symbol: string, timeRange?: string) {
    const params = new URLSearchParams({ symbol: symbol.toUpperCase() });
    if (timeRange) params.append('time_range', timeRange);
    const url = `${getFullUrl(
      apiConfig.endpoints.analytics.symbol
    )}?${params}`;
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch symbol analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get options analytics - returns comprehensive analytics for all option trades
   * @param timeRange - Optional time range filter
   * @returns Options analytics including P&L, profit factor, win rate, etc.
   */
  async getOptionsAnalytics(timeRange?: string) {
    const params = timeRange ? `?time_range=${timeRange}` : '';
    const url = getFullUrl(
      apiConfig.endpoints.options.analytics.summary + params
    );
    const token = await this.getAuthToken();
    
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch options analytics: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get authentication token from Supabase session
   */
  private async getAuthToken(): Promise<string | null> {
    try {
      const { data: { session }, error } = await this.supabase.auth.getSession();
      
      if (error) {
        console.error('Error getting session:', error);
        return null;
      }
      
      if (!session?.access_token) {
        console.log('No authentication token found - user not logged in');
        return null;
      }
      
      // Check if token expires soon (within 5 minutes)
      const tokenExpiry = session.expires_at ? new Date(session.expires_at * 1000) : null;
      const now = new Date();
      const fiveMinutesFromNow = new Date(now.getTime() + 5 * 60 * 1000);
      
      if (tokenExpiry && tokenExpiry < fiveMinutesFromNow) {
        console.log('Token expires soon, refreshing...');
        const { data: { session: refreshedSession }, error: refreshError } = await this.supabase.auth.refreshSession();
        
        if (refreshError) {
          console.error('Error refreshing session:', refreshError);
          return null;
        }
        
        if (refreshedSession?.access_token) {
          console.log('Token refreshed successfully');
          return refreshedSession.access_token;
        }
      }
      
      return session.access_token;
    } catch (error) {
      console.error('Error getting auth token:', error);
      return null;
    }
  }
}

// Export singleton instance
export const analyticsService = new AnalyticsService();

// Export individual functions for convenience
export const getCoreAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getCoreAnalytics(request);

export const getRiskAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getRiskAnalytics(request);

export const getPerformanceAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getPerformanceAnalytics(request);

export const getTimeSeriesAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getTimeSeriesAnalytics(request);

export const getGroupedAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getGroupedAnalytics(request);

export const getComprehensiveAnalytics = (request?: AnalyticsRequest) =>
  analyticsService.getComprehensiveAnalytics(request);

export const getIndividualTradeAnalytics = (
  tradeId: number,
  tradeType: 'stock' | 'option'
) => analyticsService.getIndividualTradeAnalytics(tradeId, tradeType);

export const getSymbolAnalytics = (symbol: string, timeRange?: string) =>
  analyticsService.getSymbolAnalytics(symbol, timeRange);

export const getStocksAnalytics = (symbol: string, timeRange?: string) =>
  analyticsService.getStocksAnalytics(symbol, timeRange);

export const getOptionsAnalytics = (timeRange?: string) =>
  analyticsService.getOptionsAnalytics(timeRange);