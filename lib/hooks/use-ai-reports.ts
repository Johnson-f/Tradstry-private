'use client';

import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { aiReportsService } from '@/lib/services/ai-reports-service';
import { useWs } from '@/lib/websocket/provider';
import type {
  AiReportChunkEvent,
  AiReportCompleteEvent,
  AiReportProgressEvent,
  ReportHistoryItem,
  ReportHistoryResponse,
  SaveReportPayload,
  TradingReport,
} from '@/lib/types/ai-reports';

interface ReportStreamState {
  progress: AiReportProgressEvent | null;
  reports: TradingReport[];
  complete: boolean;
}

export function useAiReportStream() {
  const { subscribe } = useWs();
  const [state, setState] = useState<ReportStreamState>({
    progress: null,
    reports: [],
    complete: false,
  });

  useEffect(() => {
    const unsubProgress = subscribe('ai:report:progress', (data) => {
      const event = data as AiReportProgressEvent;
      setState((prev) => ({
        ...prev,
        progress: event,
        complete: false,
      }));
    });

    const unsubChunk = subscribe('ai:report:chunk', (data) => {
      const event = data as AiReportChunkEvent;
      const incoming = event.report;
      setState((prev) => {
        const nextReports = [...prev.reports];
        const idx = nextReports.findIndex((r) => r.id === incoming.id);
        if (idx >= 0) {
          nextReports[idx] = incoming;
        } else {
          nextReports.push(incoming);
        }

        return {
          ...prev,
          reports: nextReports,
          progress: {
            ...(prev.progress ?? {}),
            report_type: event.report_type ?? (incoming as unknown as { report_type?: string }).report_type,
            index: event.index,
            total: event.total,
          },
        };
      });
    });

    const unsubComplete = subscribe('ai:report:complete', (data) => {
      const event = data as AiReportCompleteEvent;
      setState((prev) => ({
        ...prev,
        complete: true,
        reports: Array.isArray(event.reports)
          ? (event.reports as TradingReport[])
          : prev.reports,
        progress: {
          ...(prev.progress ?? {}),
          stage: event.stage ?? 'complete',
        },
      }));
    });

    return () => {
      unsubProgress();
      unsubChunk();
      unsubComplete();
    };
  }, [subscribe]);

  const reset = () =>
    setState({
      progress: null,
      reports: [],
      complete: false,
    });

  return {
    progress: state.progress,
    reports: state.reports,
    complete: state.complete,
    reset,
  };
}

// History list
export function useAiReportHistory(limit = 50, offset = 0) {
  const query = useQuery<ReportHistoryResponse>({
    queryKey: ['ai-reports', 'history', limit, offset],
    queryFn: () => aiReportsService.getHistory(limit, offset),
    staleTime: 60_000,
  });

  return {
    history: query.data ?? [],
    loading: query.isLoading,
    error: query.error ?? null,
    refresh: query.refetch,
    isFetching: query.isFetching,
  };
}

// History item
export function useAiReportHistoryItem(id?: string) {
  const enabled = Boolean(id);
  const query = useQuery<ReportHistoryItem | TradingReport | unknown>({
    queryKey: ['ai-reports', 'history-item', id],
    queryFn: () => aiReportsService.getHistoryById(id as string),
    enabled,
    staleTime: 60_000,
  });

  return {
    report: query.data ?? null,
    loading: query.isLoading,
    error: query.error ?? null,
    refresh: query.refetch,
    isFetching: query.isFetching,
  };
}

// Save report payload
export function useSaveAiReport() {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationKey: ['ai-reports', 'save'],
    mutationFn: (payload: SaveReportPayload) => aiReportsService.saveReport(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-reports', 'history'] });
    },
  });

  return {
    save: mutation.mutateAsync,
    saving: mutation.isPending,
    error: mutation.error ?? null,
  };
}
