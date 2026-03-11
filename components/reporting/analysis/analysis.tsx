"use client";

import { useEffect, useMemo } from "react";
import { AlertCircle, BarChart3, Bot, Waves } from "lucide-react";
import { useAiAnalysisStream, useAiAnalysisHistory } from "@/lib/hooks/use-ai-analysis";
import type { FullAnalysis } from "@/lib/types/ai-analysis";
import { MarkdownFormatter } from "./markdown-formatter";
import { cn } from "@/lib/utils";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import aiAnalysisService from "@/lib/services/ai-analysis-service";

type SectionKey = "weaknesses" | "strengths" | "patterns" | "comprehensive";

interface SectionConfig {
  key: SectionKey;
  title: string;
  description: string;
}

type SectionData =
  | FullAnalysis["weaknesses"]
  | FullAnalysis["strengths"]
  | FullAnalysis["patterns"]
  | FullAnalysis["comprehensive"]
  | undefined;

const SECTIONS: SectionConfig[] = [
  {
    key: "weaknesses",
    title: "Weaknesses",
    description: "Areas that negatively impact performance",
  },
  {
    key: "strengths",
    title: "Strengths",
    description: "Behaviors and setups driving positive results",
  },
  {
    key: "patterns",
    title: "Patterns",
    description: "Recurring themes across recent trades",
  },
  {
    key: "comprehensive",
    title: "Comprehensive",
    description: "Overall summary and comparison stats",
  },
];

const hasTradesAnalyzed = (
  section: SectionData
): section is Extract<SectionData, { trades_analyzed?: number }> =>
  Boolean(section && "trades_analyzed" in section);

const isComprehensive = (
  section: SectionData
): section is Extract<SectionData, { current_trades_count?: number }> =>
  Boolean(section && "current_trades_count" in section);

interface AnalysisProps {
  isGeneratingRequest?: boolean;
}

const parseStoredAnalysis = (content?: string): FullAnalysis | null => {
  if (!content) return null;
  try {
    const parsed = JSON.parse(content);
    // Basic shape guard: ensure sections exist
    if (
      parsed?.weaknesses?.content &&
      parsed?.strengths?.content &&
      parsed?.patterns?.content &&
      parsed?.comprehensive?.content
    ) {
      return parsed as FullAnalysis;
    }
  } catch (error) {
    console.error("Failed to parse stored analysis", error);
  }
  return null;
};

export function Analysis({ isGeneratingRequest = false }: AnalysisProps) {
  const { assembled, sections, stage, period, complete } = useAiAnalysisStream();
  const {
    records,
    // loading: historyLoading,
    refresh: refreshHistory,
  } = useAiAnalysisHistory(1, 0);

  const latestSaved = useMemo(
    () => parseStoredAnalysis(records[0]?.content),
    [records]
  );

  // Prefer live/streaming analysis when in progress, else fall back to latest saved
  const sourceAnalysis = useMemo<FullAnalysis | null>(() => {
    if (assembled) return assembled;
    if (isGeneratingRequest || stage) return null;
    return latestSaved ?? null;
  }, [assembled, isGeneratingRequest, stage, latestSaved]);

  const activePeriod = sourceAnalysis?.period ?? period ?? "latest period";

  const hasContent = useMemo(
    () =>
      SECTIONS.some((section) => {
        const currentSection = sourceAnalysis?.[section.key] ?? sections[section.key];
        return Boolean(currentSection?.content);
      }),
    [sourceAnalysis, sections]
  );

  const showStreaming = Boolean(stage && hasContent);

  const status = useMemo(() => {
    if (complete) {
      return { icon: <BarChart3 className="h-4 w-4 text-emerald-500" />, label: "Completed" };
    }
    if (showStreaming) {
      return { icon: <Waves className="h-4 w-4 text-blue-500" />, label: "Streaming..." };
    }
    if (isGeneratingRequest) {
      return { icon: <Bot className="h-4 w-4 text-muted-foreground" />, label: "Thinking..." };
    }
    return null;
  }, [complete, showStreaming, isGeneratingRequest]);

  const showEmpty = !hasContent && !stage && !complete && !isGeneratingRequest;
  const showStatusPill = Boolean(status);

  // Persist completed analysis to localStorage when streaming finishes
  useEffect(() => {
    if (complete && assembled) {
      try {
        const payload = {
          savedAt: new Date().toISOString(),
          analysis: assembled,
        };
        localStorage.setItem("ai-analysis:last", JSON.stringify(payload));
      } catch (error) {
        console.error("Failed to persist analysis to localStorage", error);
      }
    }
  }, [complete, assembled]);

  // Schedule a sync to backend after completion (2 minutes)
  useEffect(() => {
    if (!(complete && assembled)) return;

    const timer = setTimeout(async () => {
      try {
        await aiAnalysisService.syncAnalysis({
          timeRange: assembled.period,
          title: `AI Analysis - ${assembled.period}`,
          content: JSON.stringify(assembled),
        });
        localStorage.setItem("ai-analysis:last-synced", new Date().toISOString());
        await refreshHistory();
      } catch (error) {
        console.error("Failed to sync analysis to backend", error);
      }
    }, 1 * 60 * 1000);

    return () => clearTimeout(timer);
  }, [complete, assembled, refreshHistory]);

  // When the streaming completes and history is updated, prefer newest history if no live stream
  // This ensures after navigation/reload we display the last synced analysis
  useEffect(() => {
    if (!assembled && !stage && records.length === 0) {
      // Nothing in memory and no history yet; try refresh (no-op if already fetched)
      refreshHistory();
    }
  }, [assembled, stage, records.length, refreshHistory]);

  return (
    <div className="space-y-6">
      {showStatusPill && (
        <div className="flex">
          <div className="ml-4 inline-flex items-center gap-2 rounded-lg border bg-card px-4 py-2 text-xs text-muted-foreground shadow-sm">
            {status?.icon}
            <span>{status?.label}</span>
          </div>
        </div>
      )}

      {showEmpty ? (
        <Empty className="border border-dashed">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <AlertCircle className="h-5 w-5" />
            </EmptyMedia>
            <EmptyTitle>AI Analysis</EmptyTitle>
            <EmptyDescription className="space-y-1">
              <div>Period: {activePeriod}</div>
              <div>Awaiting generation. Click Generate Analysis to start.</div>
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent />
        </Empty>
      ) : (
        <>
          <div className="flex">
            <header className="ml-4 flex w-full items-center justify-between rounded-lg border bg-card px-4 py-3 shadow-sm">
              <div>
                <p className="text-xs uppercase tracking-wide text-muted-foreground">AI Analysis</p>
                <p className="text-sm text-muted-foreground">
                  Period: <span className="font-medium text-foreground">{activePeriod}</span>
                </p>
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                {complete ? (
                  <>
                    <BarChart3 className="h-4 w-4 text-emerald-500" />
                    Completed
                  </>
                ) : showStreaming ? (
                  <>
                    <Waves className="h-4 w-4 text-blue-500" />
                    Streaming...
                  </>
                ) : isGeneratingRequest ? (
                  <>
                    <Bot className="h-4 w-4 text-muted-foreground" />
                    Thinking...
                  </>
                ) : null}
              </div>
            </header>
          </div>

          {hasContent ? (
            <div className="grid grid-cols-1 gap-4 px-4">
              {SECTIONS.map((section) => {
                const currentSection = (sourceAnalysis?.[section.key] ??
                  sections[section.key]) as SectionData;
                if (!currentSection?.content) return null;

                const tradesAnalyzed = hasTradesAnalyzed(currentSection)
                  ? currentSection.trades_analyzed
                  : undefined;
                const generatedAt =
                  "generated_at" in currentSection ? currentSection.generated_at : undefined;
                const comparison = isComprehensive(currentSection)
                  ? {
                      current: currentSection.current_trades_count ?? 0,
                      previous: currentSection.previous_trades_count ?? 0,
                      hasComparison: currentSection.has_comparison ?? false,
                    }
                  : null;

                return (
                  <article
                    key={section.key}
                    className={cn(
                      "ml-4 rounded-lg border bg-card p-4 shadow-sm",
                      "hover:border-primary/60 transition"
                    )}
                  >
                    <div className="flex flex-col gap-1 mb-3">
                      <div className="flex items-center justify-between">
                        <div>
                          <h3 className="text-base font-semibold text-foreground">
                            {section.title}
                          </h3>
                          <p className="text-xs text-muted-foreground">{section.description}</p>
                        </div>
                        <div className="text-right text-xs text-muted-foreground space-y-0.5">
                          {tradesAnalyzed !== undefined && (
                            <p>{tradesAnalyzed} trades analyzed</p>
                          )}
                          {comparison?.hasComparison && (
                            <p>
                              {comparison.current} vs {comparison.previous} trades
                            </p>
                          )}
                          {generatedAt && (
                            <p className="font-medium text-foreground">
                              {new Date(generatedAt).toLocaleString()}
                            </p>
                          )}
                        </div>
                      </div>
                    </div>
                    <MarkdownFormatter content={currentSection.content} />
                  </article>
                );
              })}
            </div>
          ) : (
            <div className="rounded-lg border border-dashed p-6 text-sm text-muted-foreground">
              Waiting for analysis content...
            </div>
          )}
        </>
      )}
    </div>
  );
}

