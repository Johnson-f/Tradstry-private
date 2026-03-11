'use client';

import { useEffect, useCallback, useRef, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useWs } from '@/lib/websocket/provider';
import { tradeNoteService } from '@/lib/services/trade-note-service';
import type {
  TradeNote,
  CreateTradeNoteForTradeRequest,
} from '@/lib/types/trade-note';

/**
 * Hook to manage a trade note linked to a specific trade
 * @param tradeType - 'stock' or 'option'
 * @param tradeId - The trade ID
 * @param enabled - Whether to fetch the note (default: true)
 * @param debounceMs - Debounce delay for auto-save (default: 1000ms)
 */
export function useTradeNote(
  tradeType: 'stock' | 'option',
  tradeId: number,
  enabled: boolean = true,
  debounceMs: number = 1000
) {
  const queryClient = useQueryClient();
  const { subscribe } = useWs();
  const debounceTimerRef = useRef<NodeJS.Timeout | null>(null);

  const queryKey = useMemo(
    () => ['trade-note', tradeType, tradeId] as const,
    [tradeType, tradeId]
  );

  // Fetch trade note
  const query = useQuery({
    queryKey,
    queryFn: async () => {
      const response = await tradeNoteService.getTradeNote(tradeType, tradeId);
      return response.data;
    },
    enabled: enabled && !!tradeId,
    staleTime: 2 * 60 * 1000, // 2 minutes
    retry: (failureCount, error: unknown) => {
      // Don't retry on 404 (note doesn't exist yet)
      const httpError = error as { status?: number };
      if (httpError?.status === 404) {
        return false;
      }
      return failureCount < 2;
    },
  });

  // Upsert mutation
  const upsertMutation = useMutation({
    mutationFn: (payload: CreateTradeNoteForTradeRequest) =>
      tradeNoteService.upsertTradeNote(tradeType, tradeId, payload),
    onSuccess: (data) => {
      // Update cache with new/updated note
      if (data.data) {
        queryClient.setQueryData<TradeNote>(queryKey, data.data);
      }
      // Invalidate list queries
      queryClient.invalidateQueries({ queryKey: ['notes'] });
    },
    onError: (error) => {
      console.error('Failed to save trade note:', error);
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: () => tradeNoteService.deleteTradeNote(tradeType, tradeId),
    onSuccess: () => {
      // Remove from cache
      queryClient.setQueryData<TradeNote>(queryKey, undefined);
      // Invalidate list queries
      queryClient.invalidateQueries({ queryKey: ['notes'] });
    },
    onError: (error) => {
      console.error('Failed to delete trade note:', error);
    },
  });

  // Debounced upsert function
  const debouncedUpsert = useCallback(
    (payload: CreateTradeNoteForTradeRequest) => {
      // Clear existing timer
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }

      // Set new timer
      debounceTimerRef.current = setTimeout(() => {
        upsertMutation.mutate(payload);
      }, debounceMs);
    },
    [upsertMutation, debounceMs]
  );

  // Immediate upsert (for manual saves)
  const upsertImmediate = useCallback(
    (payload: CreateTradeNoteForTradeRequest) => {
      // Clear any pending debounced calls
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
        debounceTimerRef.current = null;
      }
      upsertMutation.mutate(payload);
    },
    [upsertMutation]
  );

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  // Subscribe to WebSocket events for this specific note
  useEffect(() => {
    const unsubCreate = subscribe('note:created', (data: unknown) => {
      const note = data as TradeNote;
      // Only update if this note matches our trade
      if (
        (tradeType === 'stock' && note.stockTradeId === tradeId) ||
        (tradeType === 'option' && note.optionTradeId === tradeId)
      ) {
        queryClient.setQueryData<TradeNote>(queryKey, note);
      }
    });

    const unsubUpdate = subscribe('note:updated', (data: unknown) => {
      const note = data as TradeNote;
      // Only update if this note matches our trade
      if (
        (tradeType === 'stock' && note.stockTradeId === tradeId) ||
        (tradeType === 'option' && note.optionTradeId === tradeId)
      ) {
        queryClient.setQueryData<TradeNote>(queryKey, note);
      }
    });

    const unsubDelete = subscribe('note:deleted', (data: unknown) => {
      const payload = data as { trade_type?: string; id?: number };
      // Check if deleted note matches our trade
      if (
        payload.trade_type === tradeType &&
        payload.id === tradeId
      ) {
        queryClient.setQueryData<TradeNote>(queryKey, undefined);
      }
    });

    return () => {
      unsubCreate();
      unsubUpdate();
      unsubDelete();
    };
  }, [subscribe, queryClient, tradeType, tradeId, queryKey]);

  return {
    note: query.data,
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    error: query.error,
    refetch: query.refetch,
    // Debounced upsert (auto-save on content change)
    upsert: debouncedUpsert,
    // Immediate upsert (manual save)
    upsertImmediate,
    upsertAsync: upsertMutation.mutateAsync,
    isUpserting: upsertMutation.isPending,
    delete: deleteMutation.mutate,
    deleteAsync: deleteMutation.mutateAsync,
    isDeleting: deleteMutation.isPending,
    // Expose mutation for advanced use cases
    upsertMutation,
    deleteMutation,
  };
}

