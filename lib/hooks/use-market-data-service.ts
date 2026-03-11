'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useCallback, useMemo } from 'react';
import { marketDataService } from '@/lib/services/market-data-service';
import { useWs, useWebSocketControl } from '@/lib/websocket/provider';
import type {
  Quote,
  SimpleQuote,
  MoverItem,
  MoversResponse,
  IndexItem,
  GetQuotesRequest,
  GetHistoricalRequest,
  GetNewsRequest,
  GetIndicatorRequest,
  GetFinancialsRequest,
  GetEarningsTranscriptRequest,
  GetHoldersRequest,
  GetEarningsCalendarRequest,
  EarningsCalendarResponse,
  SubscribeRequest,
  UnsubscribeRequest,
  QuoteUpdate,
  IndexUpdate,
  MoversUpdate,
  LogoUrl,
} from '@/lib/types/market-data';

// =====================================================
// MARKET HEALTH & HOURS HOOKS
// =====================================================

/**
 * Hook to fetch market health status
 */
export function useMarketHealth(enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'health'],
    queryFn: () => marketDataService.getHealth(),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    refetchInterval: 60 * 1000, // Refetch every minute
  });

  return {
    health: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch market hours information
 */
export function useMarketHours(enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'hours'],
    queryFn: () => marketDataService.getHours(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    enabled,
    refetchInterval: 5 * 60 * 1000, // Refetch every 5 minutes
  });

  return {
    hours: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// QUOTES HOOKS
// =====================================================

/**
 * Hook to fetch quotes for symbols
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useQuotes(params?: GetQuotesRequest, enabled: boolean = true) {
  const symbols = params?.symbols ?? [];

  // Subscribe to WebSocket updates for these symbols
  useMarketSubscription(symbols, enabled && symbols.length > 0);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'quotes', params?.symbols],
    queryFn: () => marketDataService.getQuotes(params),
    staleTime: 10 * 1000, // 10 seconds
    gcTime: 1 * 60 * 1000, // 1 minute
    enabled: enabled && symbols.length > 0,
    // No refetchInterval - WebSocket handles real-time updates
  });

  return {
    quotes: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch simple quotes for symbols (summary data)
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useSimpleQuotes(params?: GetQuotesRequest, enabled: boolean = true) {
  const symbols = params?.symbols ?? [];

  // Subscribe to WebSocket updates for these symbols
  useMarketSubscription(symbols, enabled && symbols.length > 0);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'simple-quotes', params?.symbols],
    queryFn: () => marketDataService.getSimpleQuotes(params),
    staleTime: 10 * 1000, // 10 seconds
    gcTime: 1 * 60 * 1000, // 1 minute
    enabled: enabled && symbols.length > 0,
    // No refetchInterval - WebSocket handles real-time updates
  });

  return {
    quotes: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch similar quotes to a symbol
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useSimilar(symbol: string | null, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'similar', symbol],
    queryFn: () => marketDataService.getSimilar(symbol!),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes
    enabled: enabled && !!symbol,
    // No refetchInterval - WebSocket handles real-time updates
  });

  const similarSymbols = (Array.isArray(data) ? data.map((q) => q.symbol) : []);
  // Subscribe to WebSocket updates for similar symbols
  useMarketSubscription(similarSymbols, enabled && !!symbol && similarSymbols.length > 0);

  return {
    similar: Array.isArray(data) ? data : [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch a single quote
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useQuote(symbol: string | null, enabled: boolean = true) {
  // Subscribe to WebSocket updates for this symbol
  useMarketSubscription(symbol ? [symbol] : [], enabled && !!symbol);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'quote', symbol],
    queryFn: () => marketDataService.getQuotes({ symbols: symbol ? [symbol] : [] }),
    staleTime: 10 * 1000, // 10 seconds
    gcTime: 1 * 60 * 1000, // 1 minute
    enabled: enabled && !!symbol,
    // No refetchInterval - WebSocket handles real-time updates
  });

  return {
    quote: data?.[0] ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch detailed quote with company info for a single symbol
 */
export function useDetailedQuote(symbol: string | null, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'detailed-quote', symbol],
    queryFn: () => marketDataService.getDetailedQuote(symbol!),
    staleTime: 5 * 60 * 1000, // 5 minutes (company info doesn't change often)
    gcTime: 30 * 60 * 1000, // 30 minutes
    enabled: enabled && !!symbol,
  });

  return {
    quote: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch only logo URL for a symbol
 */
export function useLogo(symbol: string | null, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'logo', symbol],
    queryFn: () => marketDataService.getLogo(symbol!),
    staleTime: 60 * 60 * 1000, // 1 hour
    gcTime: 24 * 60 * 60 * 1000, // 24 hours
    enabled: enabled && !!symbol,
  });

  return {
    logo: (data ?? null) as LogoUrl,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// HISTORICAL DATA HOOKS
// =====================================================

/**
 * Hook to fetch historical data for a symbol
 */
export function useHistorical(
  params: GetHistoricalRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'historical', params.symbol, params.range, params.interval],
    queryFn: () => marketDataService.getHistorical(params),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes
    // Poll the backend every 1 minute for active views
    refetchInterval: enabled && !!params.symbol ? 60 * 1000 : false,
    refetchIntervalInBackground: true,
    enabled: enabled && !!params.symbol,
  });

  return {
    historical: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// MARKET MOVERS HOOKS
// =====================================================

/**
 * Hook to fetch market movers (gainers, losers, most active)
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useMovers(enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'movers'],
    queryFn: () => marketDataService.getMovers(),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    // No refetchInterval - WebSocket handles real-time updates
  });

  // Extract all symbols from movers data
  const moverSymbols = data
    ? [...new Set([
        ...(Array.isArray(data.gainers) ? data.gainers.map((m) => m.symbol) : []),
        ...(Array.isArray(data.losers) ? data.losers.map((m) => m.symbol) : []),
        ...(Array.isArray(data.mostActive) ? data.mostActive.map((m) => m.symbol) : []),
      ])]
    : [];

  // Subscribe to WebSocket updates for all mover symbols
  useMarketSubscription(moverSymbols, enabled && moverSymbols.length > 0);

  return {
    movers: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch top gainers
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useGainers(count?: number, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'gainers', count],
    queryFn: () => marketDataService.getGainers(count),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    // No refetchInterval - WebSocket handles real-time updates
  });

  const gainerSymbols = data?.map((m) => m.symbol) ?? [];
  // Subscribe to WebSocket updates for gainer symbols
  useMarketSubscription(gainerSymbols, enabled && gainerSymbols.length > 0);

  return {
    gainers: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch top losers
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useLosers(count?: number, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'losers', count],
    queryFn: () => marketDataService.getLosers(count),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    // No refetchInterval - WebSocket handles real-time updates
  });

  const loserSymbols = data?.map((m) => m.symbol) ?? [];
  // Subscribe to WebSocket updates for loser symbols
  useMarketSubscription(loserSymbols, enabled && loserSymbols.length > 0);

  return {
    losers: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch most active stocks
 * Automatically subscribes to WebSocket updates for real-time price changes
 */
export function useActives(count?: number, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'actives', count],
    queryFn: () => marketDataService.getActives(count),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    // No refetchInterval - WebSocket handles real-time updates
  });

  const activeSymbols = data?.map((m) => m.symbol) ?? [];
  // Subscribe to WebSocket updates for active symbols
  useMarketSubscription(activeSymbols, enabled && activeSymbols.length > 0);

  return {
    actives: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// NEWS HOOKS
// =====================================================

/**
 * Hook to fetch market news
 */
export function useNews(params?: GetNewsRequest, enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'news', params?.symbol, params?.limit],
    queryFn: () => marketDataService.getNews(params),
    staleTime: 2 * 60 * 1000, // 2 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    enabled,
  });

  return {
    news: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// INDICES HOOKS
// =====================================================

/**
 * Hook to fetch market indices
 */
export function useIndices(enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'indices'],
    queryFn: () => marketDataService.getIndices(),
    staleTime: 30 * 1000, // 30 seconds
    gcTime: 2 * 60 * 1000, // 2 minutes
    enabled,
    refetchInterval: 60 * 1000, // Refetch every minute
  });

  return {
    indices: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// SECTORS HOOKS
// =====================================================

/**
 * Hook to fetch sector performance
 */
export function useSectors(enabled: boolean = true) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'sectors'],
    queryFn: () => marketDataService.getSectors(),
    staleTime: 60 * 1000, // 1 minute
    gcTime: 5 * 60 * 1000, // 5 minutes
    enabled,
    refetchInterval: 5 * 60 * 1000, // Refetch every 5 minutes
  });

  return {
    sectors: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// SEARCH HOOKS
// =====================================================

/**
 * Hook to search for symbols
 */
export function useSymbolSearch(
  query: string,
  params?: { hits?: number; yahoo?: boolean },
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'search', query, params?.hits, params?.yahoo],
    queryFn: () => marketDataService.search(query, params),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes
    enabled: enabled && query.length >= 2, // Only search if query is at least 2 characters
  });

  return {
    results: data ?? [],
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// INDICATORS HOOKS
// =====================================================

/**
 * Hook to fetch indicator data for a symbol
 */
export function useIndicator(
  params: GetIndicatorRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'indicator', params.symbol, params.indicator, params.interval],
    queryFn: () => marketDataService.getIndicator(params),
    staleTime: 2 * 60 * 1000, // 2 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
    enabled: enabled && !!params.symbol && !!params.indicator,
  });

  return {
    indicator: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// WEBSOCKET SUBSCRIPTION HOOKS
// =====================================================

/**
 * Hook to subscribe to market quote updates via WebSocket
 * This hook manages WebSocket subscriptions and provides real-time quote updates
 * Uses the shared WebSocket connection from WebSocketProvider
 */
export function useMarketSubscription(
  symbols: string[],
  enabled: boolean = true,
  onQuoteUpdate?: (update: QuoteUpdate) => void
) {
  // Use shared WebSocket connection instead of creating a new one
  const { state, send, subscribe: wsSubscribe } = useWs();
  const { enable: enableWebSocket } = useWebSocketControl();
  const queryClient = useQueryClient();
  const subscribedSymbolsRef = useRef<Set<string>>(new Set());
  const unsubscribeRef = useRef<(() => void) | null>(null);

  // Normalize symbols to uppercase for consistent comparison
  const normalizedSymbols = useMemo(() =>
    symbols.map(s => s.toUpperCase().trim()).filter(Boolean),
    [symbols]
  );

  // Enable WebSocket when subscriptions are needed
  useEffect(() => {
    if (enabled && normalizedSymbols.length > 0) {
      enableWebSocket();
    }
  }, [enabled, normalizedSymbols.length, enableWebSocket]);

  // Helper function to update all price-related cache keys
  const updateAllPriceCaches = useCallback((update: QuoteUpdate) => {
    // Normalize update symbol to uppercase for consistent matching
    const updateSymbol = update.symbol.toUpperCase().trim();

    const updateData = {
      symbol: updateSymbol,
      name: update.name,
      price: update.price,
      afterHoursPrice: update.afterHoursPrice ?? null,
      change: String(update.change),
      percentChange: String(update.percentChange),
      logo: update.logo ?? null,
    };

    // Update detailed quotes cache
    queryClient.setQueryData<Quote[]>(
      ['market', 'quotes', [updateSymbol]],
      (oldData) => {
        if (!oldData || !Array.isArray(oldData)) return [{ ...updateData, symbol: updateSymbol } as Quote];
        return oldData.map((quote) =>
          quote.symbol.toUpperCase().trim() === updateSymbol
            ? { ...quote, ...updateData }
            : quote
        );
      }
    );

    // Update simple quotes cache - check all possible symbol arrays
    queryClient.getQueryCache().getAll().forEach((query) => {
      const key = query.queryKey;
      if (key[0] === 'market' && key[1] === 'simple-quotes' && Array.isArray(key[2])) {
        const symbols = key[2] as string[];
        // Normalize symbols for case-insensitive comparison
        const normalizedCacheSymbols = symbols.map(s => s.toUpperCase().trim());
        if (normalizedCacheSymbols.includes(updateSymbol)) {
          queryClient.setQueryData<SimpleQuote[]>(key, (oldData) => {
            if (!oldData || !Array.isArray(oldData)) return [updateData as SimpleQuote];
            return oldData.map((quote) =>
              quote.symbol.toUpperCase().trim() === updateSymbol
                ? { ...quote, ...updateData }
                : quote
            );
          });
        }
      }
    });

    // Update similar quotes cache
    queryClient.setQueryData<SimpleQuote[]>(
      ['market', 'similar', updateSymbol],
      (oldData) => {
        if (!oldData || !Array.isArray(oldData)) return [];
        return oldData.map((quote) =>
          quote.symbol.toUpperCase().trim() === updateSymbol
            ? { ...quote, ...updateData }
            : quote
        );
      }
    );

    // Update single quote cache
    queryClient.setQueryData<Quote[]>(
      ['market', 'quote', updateSymbol],
      (oldData) => {
        if (!oldData || !Array.isArray(oldData)) return [{ ...updateData, symbol: updateSymbol } as Quote];
        return oldData.map((quote) =>
          quote.symbol.toUpperCase().trim() === updateSymbol
            ? { ...quote, ...updateData }
            : quote
        );
      }
    );

    // Update gainers cache
    queryClient.getQueryCache().getAll().forEach((query) => {
      const key = query.queryKey;
      if (key[0] === 'market' && key[1] === 'gainers') {
        queryClient.setQueryData<MoverItem[]>(key, (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return [];
          return oldData.map((item) =>
            item.symbol.toUpperCase().trim() === updateSymbol
              ? {
                  ...item,
                  name: update.name,
                  price: update.price,
                  change: String(update.change),
                  percentChange: String(update.percentChange),
                }
              : item
          );
        });
      }
    });

    // Update losers cache
    queryClient.getQueryCache().getAll().forEach((query) => {
      const key = query.queryKey;
      if (key[0] === 'market' && key[1] === 'losers') {
        queryClient.setQueryData<MoverItem[]>(key, (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return [];
          return oldData.map((item) =>
            item.symbol.toUpperCase().trim() === updateSymbol
              ? {
                  ...item,
                  name: update.name,
                  price: update.price,
                  change: String(update.change),
                  percentChange: String(update.percentChange),
                }
              : item
          );
        });
      }
    });

    // Update actives cache
    queryClient.getQueryCache().getAll().forEach((query) => {
      const key = query.queryKey;
      if (key[0] === 'market' && key[1] === 'actives') {
        queryClient.setQueryData<MoverItem[]>(key, (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return [];
          return oldData.map((item) =>
            item.symbol.toUpperCase().trim() === updateSymbol
              ? {
                  ...item,
                  name: update.name,
                  price: update.price,
                  change: String(update.change),
                  percentChange: String(update.percentChange),
                }
              : item
          );
        });
      }
    });

    // Update movers cache (composite)
    queryClient.setQueryData<MoversResponse>(
      ['market', 'movers'],
      (oldData) => {
        if (!oldData || typeof oldData !== 'object') return { gainers: [], losers: [], mostActive: [] };
        const updateItem = (items: MoverItem[]) =>
          Array.isArray(items) ? items.map((item) =>
            item.symbol.toUpperCase().trim() === updateSymbol
              ? {
                  ...item,
                  name: update.name,
                  price: update.price,
                  change: String(update.change),
                  percentChange: String(update.percentChange),
                }
              : item
          ) : [];
        return {
          gainers: updateItem(oldData.gainers || []),
          losers: updateItem(oldData.losers || []),
          mostActive: updateItem(oldData.mostActive || []),
        };
      }
    );
  }, [queryClient]);

  // Subscribe to websocket events
  useEffect(() => {
    if (!enabled || normalizedSymbols.length === 0 || state !== 'connected') {
      return;
    }

    console.log('📡 Setting up WebSocket handlers for symbols:', normalizedSymbols);

    // Subscribe to market:quote and market:update events
    const unsubscribeQuote = wsSubscribe('market:quote', (data: unknown) => {
      const update = data as QuoteUpdate;
      console.log('📈 Received market:quote update:', update);
      updateAllPriceCaches(update);
      onQuoteUpdate?.(update);
    });

    const unsubscribeUpdate = wsSubscribe('market:update', (data: unknown) => {
      // market:update can contain indices or movers updates
      const update = data as { type?: string; data?: unknown };
      console.log('📊 Received market:update:', update);
      
      if (update.type === 'indices' && Array.isArray(update.data)) {
        // Handle indices update
        const indices = update.data as IndexUpdate[];
        queryClient.setQueryData<IndexItem[]>(['market', 'indices'], (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return indices.map(idx => ({
            name: idx.name,
            price: idx.value.toString(),
            change: idx.change,
            percentChange: idx.percentChange,
          }));
          return oldData.map(item => {
            const update = indices.find(idx => idx.name === item.name);
            return update ? {
              ...item,
              price: update.value.toString(),
              change: update.change,
              percentChange: update.percentChange,
            } : item;
          });
        });
      } else if (update.type === 'movers' && update.data) {
        // Handle movers update
        const movers = update.data as MoversUpdate;
        queryClient.setQueryData<MoversResponse>(['market', 'movers'], {
          gainers: movers.gainers,
          losers: movers.losers,
          mostActive: movers.actives,
        });
      } else {
        // Fallback: treat as quote update for backward compatibility
        const quoteUpdate = data as QuoteUpdate;
        if (quoteUpdate.symbol) {
          updateAllPriceCaches(quoteUpdate);
          onQuoteUpdate?.(quoteUpdate);
        }
      }
    });

    unsubscribeRef.current = () => {
      unsubscribeQuote();
      unsubscribeUpdate();
    };

    return () => {
      unsubscribeQuote();
      unsubscribeUpdate();
    };
  }, [enabled, normalizedSymbols, state, wsSubscribe, updateAllPriceCaches, onQuoteUpdate, queryClient]);

  // Subscribe/unsubscribe via WebSocket when symbols change
  useEffect(() => {
    if (state !== 'connected' || !enabled || normalizedSymbols.length === 0) {
      // If no symbols, unsubscribe from all
      if (normalizedSymbols.length === 0 && subscribedSymbolsRef.current.size > 0) {
        const toUnsubscribe = Array.from(subscribedSymbolsRef.current);
        send({
          type: 'unsubscribe',
          symbols: toUnsubscribe,
          stream: 'quotes',
        });
        subscribedSymbolsRef.current.clear();
      }
      return;
    }

    const currentSymbols = new Set(normalizedSymbols);
    const toSubscribe: string[] = [];
    const toUnsubscribe: string[] = [];

    // Find symbols to subscribe to
    currentSymbols.forEach((symbol) => {
      if (!subscribedSymbolsRef.current.has(symbol)) {
        toSubscribe.push(symbol);
      }
    });

    // Find symbols to unsubscribe from
    subscribedSymbolsRef.current.forEach((symbol) => {
      if (!currentSymbols.has(symbol)) {
        toUnsubscribe.push(symbol);
      }
    });

    // Send unsubscribe message FIRST (important for clean state)
    if (toUnsubscribe.length > 0) {
      console.log('🔕 Unsubscribing from symbols:', toUnsubscribe);
      const unsubMsg = {
        type: 'unsubscribe',
        symbols: toUnsubscribe,
        stream: 'quotes',
      };
      console.log('📤 Sending unsubscribe message:', JSON.stringify(unsubMsg));
      send(unsubMsg);
      toUnsubscribe.forEach((symbol) => subscribedSymbolsRef.current.delete(symbol));
    }

    // Send subscribe message AFTER unsubscribe
    if (toSubscribe.length > 0) {
      console.log('🔔 Subscribing to symbols:', toSubscribe);
      const subMsg = {
        type: 'subscribe',
        symbols: toSubscribe,
        stream: 'quotes',
      };
      console.log('📤 Sending subscribe message:', JSON.stringify(subMsg));
      send(subMsg);
      toSubscribe.forEach((symbol) => subscribedSymbolsRef.current.add(symbol));
    }
  }, [normalizedSymbols, state, enabled, send]);

  // Cleanup on unmount
  useEffect(() => {
    const subscribedSymbols = subscribedSymbolsRef.current;
    return () => {
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
      }
      // Unsubscribe from all symbols on unmount
      if (state === 'connected' && subscribedSymbols.size > 0) {
        send({
          type: 'unsubscribe',
          symbols: Array.from(subscribedSymbols),
          stream: 'quotes',
        });
        subscribedSymbols.clear();
      }
    };
  }, [state, send]);

  return {
    isConnected: state === 'connected',
    isConnecting: state === 'connecting',
    subscribedSymbols: Array.from(subscribedSymbolsRef.current),
  };
}

/**
 * Hook to manage market subscriptions with mutations
 * Provides explicit subscribe/unsubscribe functions
 * Uses the shared WebSocket connection from WebSocketProvider
 */
export function useMarketSubscriptionMutation() {
  const queryClient = useQueryClient();
  const { state, send, subscribe: wsSubscribe } = useWs();
  const { enable: enableWebSocket } = useWebSocketControl();
  const unsubscribeHandlersRef = useRef<Map<string, () => void>>(new Map());

  // Enable WebSocket when this hook is used
  useEffect(() => {
    enableWebSocket();
  }, [enableWebSocket]);

  // Subscribe to websocket events
  useEffect(() => {
    if (state !== 'connected') {
      return;
    }

    const unsubscribeQuote = wsSubscribe('market:quote', (data: unknown) => {
      const update = data as QuoteUpdate;
      queryClient.setQueryData<Quote[]>(
        ['market', 'quotes', [update.symbol]],
        (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return [{ symbol: update.symbol, price: update.price, change: String(update.change), percentChange: String(update.percentChange) }];
          return oldData.map((quote) =>
            quote.symbol === update.symbol
              ? {
                  ...quote,
                  price: update.price,
                  change: String(update.change),
                  percentChange: String(update.percentChange),
                }
              : quote
          );
        }
      );
    });

    const unsubscribeUpdate = wsSubscribe('market:update', (data: unknown) => {
      // market:update can contain indices or movers updates
      const update = data as { type?: string; data?: unknown };
      
      if (update.type === 'indices' && Array.isArray(update.data)) {
        // Handle indices update
        const indices = update.data as IndexUpdate[];
        queryClient.setQueryData<IndexItem[]>(['market', 'indices'], (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return indices.map(idx => ({
            name: idx.name,
            price: idx.value.toString(),
            change: idx.change,
            percentChange: idx.percentChange,
          }));
          return oldData.map(item => {
            const update = indices.find(idx => idx.name === item.name);
            return update ? {
              ...item,
              price: update.value.toString(),
              change: update.change,
              percentChange: update.percentChange,
            } : item;
          });
        });
      } else if (update.type === 'movers' && update.data) {
        // Handle movers update
        const movers = update.data as MoversUpdate;
        queryClient.setQueryData<MoversResponse>(['market', 'movers'], {
          gainers: movers.gainers,
          losers: movers.losers,
          mostActive: movers.actives,
        });
      } else {
        // Fallback: treat as quote update for backward compatibility
        const quoteUpdate = data as QuoteUpdate;
        if (quoteUpdate.symbol) {
      queryClient.setQueryData<Quote[]>(
            ['market', 'quotes', [quoteUpdate.symbol]],
        (oldData) => {
              if (!oldData || !Array.isArray(oldData)) return [{ symbol: quoteUpdate.symbol, price: quoteUpdate.price, change: String(quoteUpdate.change), percentChange: String(quoteUpdate.percentChange) }];
          return oldData.map((quote) =>
                quote.symbol === quoteUpdate.symbol
              ? {
                  ...quote,
                      price: quoteUpdate.price,
                      change: String(quoteUpdate.change),
                      percentChange: String(quoteUpdate.percentChange),
                }
              : quote
          );
        }
      );
        }
      }
    });

    unsubscribeHandlersRef.current.set('market:quote', unsubscribeQuote);
    unsubscribeHandlersRef.current.set('market:update', unsubscribeUpdate);

    return () => {
      unsubscribeQuote();
      unsubscribeUpdate();
    };
  }, [state, wsSubscribe, queryClient]);

  const subscribe = useMutation({
    mutationFn: async (params: SubscribeRequest & { stream?: string }) => {
      if (state !== 'connected') {
        throw new Error('WebSocket not connected');
      }
      send({
        type: 'subscribe',
        symbols: params.symbols,
        stream: params.stream || 'quotes',
      });
      return params;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['market', 'quotes'] });
    },
  });

  const unsubscribe = useMutation({
    mutationFn: async (params: UnsubscribeRequest & { stream?: string }) => {
      if (state !== 'connected') {
        throw new Error('WebSocket not connected');
      }
      send({
        type: 'unsubscribe',
        symbols: params.symbols,
        stream: params.stream || 'quotes',
      });
      return params;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['market', 'quotes'] });
    },
  });

  return {
    subscribe: subscribe.mutateAsync,
    unsubscribe: unsubscribe.mutateAsync,
    isSubscribing: subscribe.isPending,
    isUnsubscribing: unsubscribe.isPending,
    isConnected: state === 'connected',
    isConnecting: state === 'connecting',
    subscribeError: subscribe.error,
    unsubscribeError: unsubscribe.error,
  };
}

/**
 * Hook to automatically subscribe to market indices, movers, and watchlist quotes streams
 * This should be called once when the app initializes to receive real-time market overview data
 */
export function useMarketStreamsSubscription(enabled: boolean = true) {
  const { state, send, subscribe: wsSubscribe } = useWs();
  const { enable: enableWebSocket } = useWebSocketControl();
  const queryClient = useQueryClient();
  const subscribedRef = useRef(false);
  const watchlistSubscribedRef = useRef(false);

  // Enable WebSocket when this hook is used
  useEffect(() => {
    if (enabled) {
      enableWebSocket();
    }
  }, [enabled, enableWebSocket]);

  // Fetch watchlist and subscribe to quotes when websocket connects
  useEffect(() => {
    if (!enabled || state !== 'connected' || watchlistSubscribedRef.current) {
      return;
    }

    // Fetch watchlist and subscribe to quotes
    const fetchAndSubscribeWatchlist = async () => {
      try {
        const { createClient } = await import('@/lib/supabase/client');
        const supabase = createClient();
        const { data: { session } } = await supabase.auth.getSession();
        const token = session?.access_token;

        if (!token) {
          console.warn('No auth token available for watchlist subscription');
          return;
        }

        const response = await fetch('/api/watchlist', {
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
          },
        });

        if (response.ok) {
          const result = await response.json();
          if (result.success && result.data && Array.isArray(result.data)) {
            const watchlistEntries = result.data as Array<{ ticker_symbol: string }>;
            const symbols = watchlistEntries
              .map(entry => entry.ticker_symbol?.toUpperCase().trim())
              .filter(Boolean) as string[];

            if (symbols.length > 0) {
              console.log('📡 Auto-subscribing to watchlist quotes:', symbols);
              send({
                type: 'subscribe',
                symbols,
                stream: 'quotes',
              });
              watchlistSubscribedRef.current = true;
            }
          }
        }
      } catch (error) {
        console.error('Failed to fetch watchlist for auto-subscription:', error);
        // Don't block on watchlist fetch failure - continue with indices/movers
      }
    };

    fetchAndSubscribeWatchlist();
  }, [enabled, state, send]);

  // Subscribe to indices and movers when websocket connects
  useEffect(() => {
    if (!enabled || state !== 'connected' || subscribedRef.current) {
      return;
    }

    console.log('📡 Auto-subscribing to market indices and movers streams');

    // Subscribe to indices stream
    send({
      type: 'subscribe',
      symbols: [],
      stream: 'indices',
    });

    // Subscribe to movers stream
    send({
      type: 'subscribe',
      symbols: [],
      stream: 'movers',
    });

    subscribedRef.current = true;

    // Set up handlers for indices and movers updates
    const unsubscribeIndices = wsSubscribe('market:update', (data: unknown) => {
      const update = data as { type?: string; data?: unknown };
      if (update.type === 'indices' && Array.isArray(update.data)) {
        const indices = update.data as IndexUpdate[];
        queryClient.setQueryData<IndexItem[]>(['market', 'indices'], (oldData) => {
          if (!oldData || !Array.isArray(oldData)) return indices.map(idx => ({
            name: idx.name,
            price: idx.value.toString(),
            change: idx.change,
            percentChange: idx.percentChange,
          }));
          return oldData.map(item => {
            const update = indices.find(idx => idx.name === item.name);
            return update ? {
              ...item,
              price: update.value.toString(),
              change: update.change,
              percentChange: update.percentChange,
            } : item;
          });
        });
      }
    });

    const unsubscribeMovers = wsSubscribe('market:update', (data: unknown) => {
      const update = data as { type?: string; data?: unknown };
      if (update.type === 'movers' && update.data) {
        const movers = update.data as MoversUpdate;
        queryClient.setQueryData<MoversResponse>(['market', 'movers'], {
          gainers: movers.gainers,
          losers: movers.losers,
          mostActive: movers.actives,
        });
      }
    });

    return () => {
      unsubscribeIndices();
      unsubscribeMovers();
      if (state === 'connected') {
        send({ type: 'unsubscribe', symbols: [], stream: 'indices' });
        send({ type: 'unsubscribe', symbols: [], stream: 'movers' });
      }
      subscribedRef.current = false;
      watchlistSubscribedRef.current = false;
    };
  }, [enabled, state, send, wsSubscribe, queryClient]);

  return {
    isConnected: state === 'connected',
    isSubscribed: subscribedRef.current,
  };
}

// =====================================================
// FINANCIALS HOOKS
// =====================================================

/**
 * Hook to fetch financial statements for a symbol
 */
export function useFinancials(
  params: GetFinancialsRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'financials', params.symbol, params.statement, params.frequency],
    queryFn: () => marketDataService.getFinancials(params),
    staleTime: 60 * 60 * 1000, // 1 hour
    gcTime: 24 * 60 * 60 * 1000, // 24 hours
    enabled: enabled && !!params.symbol,
  });

  return {
    financials: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// EARNINGS TRANSCRIPT HOOKS
// =====================================================

/**
 * Hook to fetch earnings transcript for a symbol
 */
export function useEarningsTranscript(
  params: GetEarningsTranscriptRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'earnings-transcript', params.symbol, params.quarter, params.year],
    queryFn: () => marketDataService.getEarningsTranscript(params),
    staleTime: 60 * 60 * 1000, // 1 hour
    gcTime: 24 * 60 * 60 * 1000, // 24 hours
    enabled: enabled && !!params.symbol,
  });

  return {
    transcript: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// HOLDERS HOOKS
// =====================================================

/**
 * Hook to fetch holders data for a symbol
 */
export function useHolders(
  params: GetHoldersRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'holders', params.symbol, params.holder_type],
    queryFn: () => marketDataService.getHolders(params),
    staleTime: 30 * 60 * 1000, // 30 minutes
    gcTime: 4 * 60 * 60 * 1000, // 4 hours
    enabled: enabled && !!params.symbol,
  });

  return {
    holders: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// =====================================================
// EARNINGS CALENDAR HOOKS
// =====================================================

/**
 * Hook to fetch earnings calendar for a date range
 * Returns companies reporting earnings filtered by market cap >= $100M
 */
export function useEarningsCalendar(
  params?: GetEarningsCalendarRequest,
  enabled: boolean = true
) {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['market', 'earnings-calendar', params?.fromDate, params?.toDate],
    queryFn: () => marketDataService.getEarningsCalendar(params),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes
    enabled,
  });

  return {
    calendar: data ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to fetch earnings for a specific date
 * Convenience wrapper around useEarningsCalendar for single-day queries
 */
export function useEarningsForDate(
  date: Date | null,
  enabled: boolean = true
) {
  const dateStr = date ? formatDateForApi(date) : undefined;
  
  const { calendar, isLoading, error, refetch } = useEarningsCalendar(
    dateStr ? { fromDate: dateStr, toDate: dateStr } : undefined,
    enabled && !!date
  );

  // Extract earnings for the specific date
  const earnings = calendar?.earnings?.[dateStr ?? '']?.stocks ?? [];

  return {
    earnings,
    totalEarnings: earnings.length,
    isLoading,
    error,
    refetch,
  };
}

/**
 * Format date as YYYY-MM-DD for API
 */
function formatDateForApi(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}
