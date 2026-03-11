'use client';

import { useEffect, useMemo, useState } from 'react';
import { useWs } from '@/lib/websocket/provider';
import type {
  AiAnalysisCompleteEvent,
  AiAnalysisProgressEvent,
  AiAnalysisSectionEvent,
  FullAnalysis,
} from '@/lib/types/ai-analysis';
import type { AnalysisRecord } from '@/lib/types/ai-analysis';
import aiAnalysisService from '@/lib/services/ai-analysis-service';
import { useQuery } from '@tanstack/react-query';

type SectionKey = 'weaknesses' | 'strengths' | 'patterns' | 'comprehensive';

interface StreamState {
  stage: AiAnalysisProgressEvent['stage'] | null;
  period: string | null;
  sections: Partial<Record<SectionKey, AiAnalysisSectionEvent>>;
  complete: boolean;
}

export function useAiAnalysisStream() {
  const { subscribe } = useWs();
  const [state, setState] = useState<StreamState>({
    stage: null,
    period: null,
    sections: {},
    complete: false,
  });

  useEffect(() => {
    const unsubProgress = subscribe('ai:analysis:progress', (data) => {
      const event = data as AiAnalysisProgressEvent;
      setState((prev) => ({
        ...prev,
        stage: event.stage,
        period: event.period ?? prev.period,
      }));
    });

    const unsubSection = subscribe('ai:analysis:section', (data) => {
      const event = data as AiAnalysisSectionEvent;
      setState((prev) => ({
        ...prev,
        sections: {
          ...prev.sections,
          [event.section]: event,
        },
      }));
    });

    const unsubComplete = subscribe('ai:analysis:complete', (data) => {
      const event = data as AiAnalysisCompleteEvent;
      setState((prev) => ({
        ...prev,
        stage: event.stage,
        period: event.period ?? prev.period,
        complete: true,
      }));
    });

    return () => {
      unsubProgress();
      unsubSection();
      unsubComplete();
    };
  }, [subscribe]);

  const assembled = useMemo<FullAnalysis | null>(() => {
    const { sections } = state;
    if (
      sections.weaknesses &&
      sections.strengths &&
      sections.patterns &&
      sections.comprehensive
    ) {
      return {
        period: sections.weaknesses.period,
        weaknesses: {
          focus: 'weaknesses',
          period: sections.weaknesses.period,
          content: sections.weaknesses.content,
          trades_analyzed: sections.weaknesses.trades_analyzed ?? 0,
          generated_at: sections.weaknesses.generated_at,
        },
        strengths: {
          focus: 'strengths',
          period: sections.strengths.period,
          content: sections.strengths.content,
          trades_analyzed: sections.strengths.trades_analyzed ?? 0,
          generated_at: sections.strengths.generated_at,
        },
        patterns: {
          focus: 'patterns',
          period: sections.patterns.period,
          content: sections.patterns.content,
          trades_analyzed: sections.patterns.trades_analyzed ?? 0,
          generated_at: sections.patterns.generated_at,
        },
        comprehensive: {
          period: sections.comprehensive.period,
          content: sections.comprehensive.content,
          current_trades_count:
            sections.comprehensive.current_trades_count ?? 0,
          previous_trades_count:
            sections.comprehensive.previous_trades_count ?? 0,
          has_comparison: sections.comprehensive.has_comparison ?? false,
          generated_at: sections.comprehensive.generated_at,
        },
      };
    }
    return null;
  }, [state]);

  const reset = () => {
    setState({
      stage: null,
      period: null,
      sections: {},
      complete: false,
    });
  };

  return {
    stage: state.stage,
    period: state.period,
    sections: state.sections,
    complete: state.complete,
    assembled,
    reset,
  };
}

export function useAiAnalysisHistory(limit = 50, offset = 0) {
  const query = useQuery<AnalysisRecord[]>({
    queryKey: ['ai-analysis-history', limit, offset],
    queryFn: () => aiAnalysisService.fetchHistory({ limit, offset }),
    staleTime: 60_000, // 1 minute
  });

  return {
    records: query.data ?? [],
    loading: query.isLoading,
    error: query.error ?? null,
    refresh: query.refetch,
    isFetching: query.isFetching,
  };
}

export function useAiAnalysisHistoryItem(id?: string) {
  const enabled = Boolean(id);
  const query = useQuery<AnalysisRecord>({
    queryKey: ['ai-analysis-history-item', id],
    queryFn: () => aiAnalysisService.fetchHistoryItem(id as string),
    enabled,
    staleTime: 60_000,
  });

  return {
    record: query.data ?? null,
    loading: query.isLoading,
    error: query.error ?? null,
    refresh: query.refetch,
    isFetching: query.isFetching,
  };
}
