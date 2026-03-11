'use client';

import { useQuery } from '@tanstack/react-query';
import {
  getCoreAnalytics,
  getRiskAnalytics,
  getPerformanceAnalytics,
  getTimeSeriesAnalytics,
  getGroupedAnalytics,
  getComprehensiveAnalytics,
  getIndividualTradeAnalytics,
  getSymbolAnalytics,
  getStocksAnalytics,
  getOptionsAnalytics,
} from '@/lib/services/analytics-service';
import type {
  AnalyticsRequest,
  UseAnalyticsCoreReturn,
  UseAnalyticsRiskReturn,
  UseAnalyticsPerformanceReturn,
  UseAnalyticsTimeSeriesReturn,
  UseAnalyticsGroupedReturn,
  UseAnalyticsComprehensiveReturn,
  UseIndividualTradeAnalyticsReturn,
  UseSymbolAnalyticsReturn,
} from '@/lib/types/analytics';

/**
 * Hook to fetch core analytics
 */
export function useAnalyticsCore(request?: AnalyticsRequest): UseAnalyticsCoreReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'core', request],
    queryFn: async () => {
      const response = await getCoreAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch core analytics';
      console.error('[useAnalyticsCore] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch risk analytics
 */
export function useAnalyticsRisk(request?: AnalyticsRequest): UseAnalyticsRiskReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'risk', request],
    queryFn: async () => {
      const response = await getRiskAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch risk analytics';
      console.error('[useAnalyticsRisk] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch performance analytics
 */
export function useAnalyticsPerformance(
  request?: AnalyticsRequest
): UseAnalyticsPerformanceReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'performance', request],
    queryFn: async () => {
      const response = await getPerformanceAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch performance analytics';
      console.error('[useAnalyticsPerformance] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch time series analytics
 */
export function useAnalyticsTimeSeries(
  request?: AnalyticsRequest
): UseAnalyticsTimeSeriesReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'time-series', request],
    queryFn: async () => {
      const response = await getTimeSeriesAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch time series analytics';
      console.error('[useAnalyticsTimeSeries] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch grouped analytics
 */
export function useAnalyticsGrouped(request?: AnalyticsRequest): UseAnalyticsGroupedReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'grouped', request],
    queryFn: async () => {
      const response = await getGroupedAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch grouped analytics';
      console.error('[useAnalyticsGrouped] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch comprehensive analytics (all metrics)
 */
export function useAnalyticsComprehensive(
  request?: AnalyticsRequest
): UseAnalyticsComprehensiveReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'comprehensive', request],
    queryFn: async () => {
      const response = await getComprehensiveAnalytics(request);
      if (response.success && response.data) {
        return response.data;
      }
      // Backend returns 'error' field, not 'message'
      const errorMsg = (response as { error?: string; message?: string }).error || response.message || 'Failed to fetch comprehensive analytics';
      console.error('[useAnalyticsComprehensive] Analytics API error:', errorMsg, response);
      throw new Error(errorMsg);
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Master hook that combines all analytics endpoints
 * Returns all analytics data in a single object
 */
export function useAnalytics(request?: AnalyticsRequest) {
  const core = useAnalyticsCore(request);
  const risk = useAnalyticsRisk(request);
  const performance = useAnalyticsPerformance(request);
  const timeSeries = useAnalyticsTimeSeries(request);
  const grouped = useAnalyticsGrouped(request);

  return {
    core_metrics: core.data,
    risk_metrics: risk.data,
    performance_metrics: performance.data,
    time_series: timeSeries.data,
    grouped_analytics: grouped.data,
    isLoading:
      core.isLoading || risk.isLoading || performance.isLoading || timeSeries.isLoading || grouped.isLoading,
    error: core.error || risk.error || performance.error || timeSeries.error || grouped.error,
    refetch: () => {
      core.refetch();
      risk.refetch();
      performance.refetch();
      timeSeries.refetch();
      grouped.refetch();
    },
  };
}

/**
 * Hook to fetch individual trade analytics
 * Gets analytics for a single trade including risk-to-reward ratios
 * Uses new columns: profit_target, initial_target, stop_loss
 */
export function useIndividualTradeAnalytics(
  tradeId: number,
  tradeType: 'stock' | 'option',
  enabled: boolean = true
): UseIndividualTradeAnalyticsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'trade', tradeId, tradeType],
    queryFn: async () => {
      const response = await getIndividualTradeAnalytics(tradeId, tradeType);
      if (response.success) {
        return response.data;
      }
      throw new Error(response.error || 'Failed to fetch individual trade analytics');
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch symbol-level analytics
 * Gets aggregated analytics for all trades on a specific symbol
 * Useful for analyzing repeated trades on the same symbol (e.g., AAPL)
 */
/**
 * Hook to fetch symbol-level analytics
 */
export function useSymbolAnalytics(
  symbol: string,
  timeRange?: string,
  enabled: boolean = true
): UseSymbolAnalyticsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'symbol', symbol, timeRange],
    queryFn: async () => {
      console.log('[useSymbolAnalytics] Fetching:', { symbol, timeRange });
      const response = await getSymbolAnalytics(symbol, timeRange);
      console.log('[useSymbolAnalytics] Response:', response);
      
      if (response.success) {
        return response.data;
      }
      throw new Error(response.error || 'Failed to fetch symbol analytics');
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch stocks analytics
 * Returns analytics for all stock trades including P&L, profit factor, win rate, etc.
 */
export function useStocksAnalytics(
  symbol: string,
  timeRange?: string,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'stocks', symbol, timeRange],
    queryFn: async () => {
      const response = await getStocksAnalytics(symbol, timeRange);
      if (response.success) {
        return response.data;
      }
      throw new Error(response.message || 'Failed to fetch symbol analytics');
    },
    enabled: enabled && !!symbol,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch options analytics
 * Returns analytics for all option trades including P&L, profit factor, win rate, etc.
 */
export function useOptionsAnalytics(
  timeRange?: string,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['analytics', 'options', timeRange],
    queryFn: async () => {
      const response = await getOptionsAnalytics(timeRange);
      if (response.success) {
        return response.data;
      }
      throw new Error(response.message || 'Failed to fetch options analytics');
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    retry: 2,
  });

  return {
    data: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

