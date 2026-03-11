"use client";

import { useEffect, useMemo, useState } from "react";
import { AlertCircle, BarChart3, Bot, FileText, Waves } from "lucide-react";
import { MarkdownFormatter } from "./markdown-formatter";
import { useAiReportHistory, useAiReportStream } from "@/lib/hooks/use-ai-reports";
import type { ReportHistoryItem, TradingReport } from "@/lib/types/ai-reports";
import { cn } from "@/lib/utils";
import { aiReportsService, normalizeReportForSave } from "@/lib/services/ai-reports-service";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";

type ReportRenderable = TradingReport | ReportHistoryItem;

const isTradingReport = (report: ReportRenderable): report is TradingReport =>
  "user_id" in report;

const isHistoryItem = (report: ReportRenderable): report is ReportHistoryItem =>
  "created_at" in report && !isTradingReport(report);

interface ReportsProps {
  isGeneratingRequest?: boolean;
}

export function Reports({ isGeneratingRequest = false }: ReportsProps) {
  const { reports: streamedReports, progress, complete } = useAiReportStream();
  const { history, isFetching, refresh } = useAiReportHistory(10, 0);
  const [storedReports, setStoredReports] = useState<TradingReport[]>([]);

  const activeReports = useMemo<ReportRenderable[]>(() => {
    if (streamedReports.length > 0) return streamedReports;
    if (history.length > 0) return history;
    if (storedReports.length > 0) return storedReports;
    return [];
  }, [streamedReports, history, storedReports]);

  const hasContent = activeReports.length > 0;
  const showStreaming = Boolean(progress?.report_type || progress?.stage) && streamedReports.length > 0;

  const status = useMemo(() => {
    if (complete && streamedReports.length > 0) {
      return { icon: <BarChart3 className="h-4 w-4 text-emerald-500" />, label: "Completed" };
    }
    if (showStreaming) {
      const currentIdx = progress?.index !== undefined ? progress.index + 1 : undefined;
      const total = progress?.total;
      const suffix =
        currentIdx !== undefined && total !== undefined
          ? `(${currentIdx}/${total})`
          : progress?.report_type
            ? `(${progress.report_type})`
            : "";
      return { icon: <Waves className="h-4 w-4 text-blue-500" />, label: `Streaming... ${suffix}`.trim() };
    }
    if (isGeneratingRequest) {
      return { icon: <Bot className="h-4 w-4 text-muted-foreground" />, label: "Thinking..." };
    }
    return null;
  }, [complete, streamedReports.length, showStreaming, progress, isGeneratingRequest]);

  const showEmpty = !hasContent && !showStreaming && !isGeneratingRequest;
  const showStatusPill = Boolean(status);

  const loadStoredReports = () => {
    try {
      const raw = localStorage.getItem("ai-reports:last");
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed?.reports)) {
        setStoredReports(parsed.reports as TradingReport[]);
      }
    } catch (error) {
      console.error("Failed to load stored AI reports", error);
    }
  };

  // Load last stored reports for quick fallback and listen for updates
  useEffect(() => {
    loadStoredReports();

    const handleStorage = (event: StorageEvent) => {
      if (event.key === "ai-reports:last") {
        loadStoredReports();
      }
    };

    const handleCustom = () => loadStoredReports();

    window.addEventListener("storage", handleStorage);
    window.addEventListener("ai-reports:update", handleCustom as EventListener);

    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("ai-reports:update", handleCustom as EventListener);
    };
  }, []);

  // Persist latest streamed reports when complete
  useEffect(() => {
    if (!(complete && streamedReports.length > 0)) return;
    try {
      const payload = {
        savedAt: new Date().toISOString(),
        reports: streamedReports,
      };
      localStorage.setItem("ai-reports:last", JSON.stringify(payload));
      setStoredReports(streamedReports);
    } catch (error) {
      console.error("Failed to persist AI reports to localStorage", error);
    }
  }, [complete, streamedReports]);

  // Schedule backend sync after completion
  useEffect(() => {
    if (!(complete && streamedReports.length > 0)) return;

    const timer = setTimeout(async () => {
      try {
        await Promise.all(
          streamedReports.map((report) => aiReportsService.saveReport(normalizeReportForSave(report)))
        );
        localStorage.setItem("ai-reports:last-synced", new Date().toISOString());
        await refresh();
      } catch (error) {
        console.error("Failed to sync AI reports to backend", error);
      }
    }, 60 * 1000);

    return () => clearTimeout(timer);
  }, [complete, streamedReports, refresh]);

  return (
    <div className="space-y-6">
      {showStatusPill && (
        <div className="flex">
          <div className="ml-4 inline-flex items-center gap-2 rounded-lg border bg-card px-4 py-2 text-xs text-muted-foreground shadow-sm">
            {status?.icon}
            <span>{status?.label}</span>
            {isFetching && <span className="text-[10px] text-muted-foreground/80">(refreshing)</span>}
            <button
              type="button"
              className="text-xs underline decoration-dotted decoration-muted-foreground/70 underline-offset-2"
              onClick={() => refresh()}
              disabled={isFetching}
            >
              refresh
            </button>
          </div>
        </div>
      )}

      {showEmpty ? (
        <Empty className="border border-dashed">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <AlertCircle className="h-5 w-5" />
            </EmptyMedia>
            <EmptyTitle>AI Reports</EmptyTitle>
            <EmptyDescription className="space-y-1">
              <div>No reports yet.</div>
              <div>Generate a report to see insights and metrics.</div>
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent />
        </Empty>
      ) : (
        <div className="grid grid-cols-1 gap-4 px-4">
          {activeReports.map((report) => {
            const generatedAt = isTradingReport(report)
              ? report.generated_at
              : isHistoryItem(report)
                ? report.created_at
                : undefined;
            const title = report.title || "AI Report";
            const timeRange =
              (report as TradingReport).time_range ||
              (isHistoryItem(report) ? (report.time_range as string | undefined) : undefined) ||
              "latest period";
            const analytics = isTradingReport(report)
              ? report.analytics
              : isHistoryItem(report)
                ? (report.analytics as TradingReport["analytics"] | undefined)
                : undefined;
            const metadata = isTradingReport(report)
              ? report.metadata
              : isHistoryItem(report)
                ? (report.metadata as TradingReport["metadata"] | undefined)
                : undefined;
            const tradeCount =
              analytics?.total_trades ??
              metadata?.trade_count ??
              (isHistoryItem(report) && typeof report.trades === "object" && Array.isArray(report.trades)
                ? report.trades.length
                : undefined);
            const recommendationsCount = isTradingReport(report)
              ? report.recommendations?.length ?? 0
              : isHistoryItem(report) && Array.isArray(report.recommendations)
                ? report.recommendations.length
                : 0;

            return (
              <article
                key={report.id}
                className={cn(
                  "ml-4 rounded-lg border bg-card p-4 shadow-sm",
                  "hover:border-primary/60 transition"
                )}
              >
                <div className="flex flex-col gap-1 mb-3">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-base font-semibold text-foreground flex items-center gap-2">
                        <FileText className="h-4 w-4 text-primary" />
                        {title}
                      </h3>
                      <p className="text-xs text-muted-foreground">
                        {isTradingReport(report) ? report.report_type : "report"} • {timeRange}
                      </p>
                    </div>
                    <div className="text-right text-xs text-muted-foreground space-y-0.5">
                      {tradeCount !== undefined && <p>{tradeCount} trades analyzed</p>}
                      {metadata?.processing_time_ms !== undefined && (
                        <p>Processed in {metadata.processing_time_ms} ms</p>
                      )}
                      {generatedAt && (
                        <p className="font-medium text-foreground">
                          {new Date(generatedAt).toLocaleString()}
                        </p>
                      )}
                    </div>
                  </div>
                  {analytics && (
                    <div className="flex flex-wrap gap-3 text-xs text-muted-foreground">
                      <span className="rounded-md bg-muted px-2 py-1">
                        PnL: {analytics.total_pnl.toLocaleString()}
                      </span>
                      <span className="rounded-md bg-muted px-2 py-1">
                        Win rate: {analytics.win_rate.toFixed(2)}%
                      </span>
                      <span className="rounded-md bg-muted px-2 py-1">
                        Trades: {analytics.total_trades}
                      </span>
                      <span className="rounded-md bg-muted px-2 py-1">
                        Recommendations: {recommendationsCount}
                      </span>
                    </div>
                  )}
                </div>

                <MarkdownFormatter content={report.summary || "No summary available yet."} />
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}


