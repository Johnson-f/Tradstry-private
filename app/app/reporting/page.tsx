"use client";

import { useState } from "react";
import { FileText, History, Loader2, Sparkles } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import { AppPageHeader } from "@/components/app-page-header";
import { Analysis } from "@/components/reporting/analysis/analysis";
import aiAnalysisService from "@/lib/services/ai-analysis-service";
import type { AnalysisPeriod } from "@/lib/types/ai-analysis";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { AnalysisHistory } from "@/components/reporting/analysis/history";
import { ReportHistory } from "@/components/reporting/report/history";
import { Reports } from "@/components/reporting/report/reports";
import { aiReportsService, normalizeReportForSave } from "@/lib/services/ai-reports-service";
import type { ReportRequest, TimeRange } from "@/lib/types/ai-reports";
import { useRouter } from "next/navigation";

type GenerationMode = "analysis" | "reports";

export default function ReportingPage() {
  const router = useRouter();
  const [mode, setMode] = useState<GenerationMode>("analysis");
  const [isGenerating, setIsGenerating] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedPeriod, setSelectedPeriod] = useState<AnalysisPeriod>("last_30_days");
  const [historyOpen, setHistoryOpen] = useState(false);

  const toReportRange = (period: AnalysisPeriod): TimeRange => {
    switch (period) {
      case "last_7_days":
        return "7d";
      case "last_90_days":
      case "this_quarter":
      case "last_quarter":
        return "90d";
      case "this_year":
      case "last_year":
        return "1y";
      case "this_month":
      case "last_month":
      case "last_30_days":
      default:
        return "30d";
    }
  };

  const handleConfirmGenerate = async () => {
    if (isGenerating) return;
    if (mode === "analysis") {
      try {
        setIsGenerating(true);
        await aiAnalysisService.generateAnalysis({ period: selectedPeriod });
        toast.success("Analysis generation started");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Failed to generate analysis";
        toast.error(message);
        console.error("[analysis-generate]", error);
      } finally {
        setIsGenerating(false);
        setDialogOpen(false);
      }
    } else {
      try {
        setIsGenerating(true);
        const request: ReportRequest = {
          time_range: toReportRange(selectedPeriod),
        };
        const reports = await aiReportsService.generateReport(request);
        if (reports?.length) {
          const payload = {
            savedAt: new Date().toISOString(),
            reports,
          };
          localStorage.setItem("ai-reports:last", JSON.stringify(payload));
          window.dispatchEvent(new Event("ai-reports:update"));

          // Schedule backend persistence similar to streaming flow
          setTimeout(async () => {
            try {
              await Promise.all(
                reports.map((report) =>
                  aiReportsService.saveReport(normalizeReportForSave(report))
                )
              );
              localStorage.setItem("ai-reports:last-synced", new Date().toISOString());
              window.dispatchEvent(new Event("ai-reports:update"));
            } catch (error) {
              console.error("Failed to sync reports to backend", error);
            }
          }, 60 * 1000);
        }
        toast.success("Report generation started");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Failed to generate report";
        toast.error(message);
        console.error("[report-generate]", error);
      } finally {
        setIsGenerating(false);
        setDialogOpen(false);
      }
    }
  };

  const ModeIcon = mode === "analysis" ? Sparkles : FileText;

  const headerActions = (
    <div className="flex items-center gap-3">
      <Button
        onClick={() => setDialogOpen(true)}
        disabled={isGenerating}
      >
        {isGenerating ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <ModeIcon className="mr-2 h-4 w-4" />
        )}
        {mode === "analysis"
          ? isGenerating
            ? "Generating..."
            : "Generate Analysis"
          : "Generate Reports"}
      </Button>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="outline"
            size="icon"
            onClick={() => setHistoryOpen(true)}
            aria-label="View history"
          >
            <History className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          <p>Show your saved analyses</p>
        </TooltipContent>
      </Tooltip>
    </div>
  );

  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="Reporting" actions={headerActions} />
      <TooltipProvider>
        <div className="flex-1 p-6">
          <Tabs value={mode} onValueChange={(value) => setMode(value as GenerationMode)} className="space-y-6">
            <div className="flex items-center justify-between">
              <TabsList className="grid w-[260px] grid-cols-2">
                <TabsTrigger value="analysis" className="w-full">
                  Analysis
                </TabsTrigger>
                <TabsTrigger value="reports" className="w-full">
                  Reports
                </TabsTrigger>
              </TabsList>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    className="flex items-center gap-3 rounded-md p-2 text-muted-foreground transition hover:bg-muted"
                    onClick={() => setHistoryOpen(true)}
                  >
                    <History className="h-6 w-6" />
                  </button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>Click to show your full history</p>
                </TooltipContent>
              </Tooltip>
            </div>

            <TabsContent value="analysis" className="space-y-4">
              <Analysis isGeneratingRequest={isGenerating} />
            </TabsContent>
            <TabsContent value="reports" className="space-y-4">
              <Reports isGeneratingRequest={isGenerating} />
            </TabsContent>
          </Tabs>
        </div>
      </TooltipProvider>

      <Sheet open={historyOpen} onOpenChange={setHistoryOpen}>
        <SheetContent side="right" className="w-full sm:max-w-lg">
          <SheetHeader>
            <SheetTitle>{mode === "analysis" ? "Analysis History" : "Report History"}</SheetTitle>
          </SheetHeader>
          <div className="pt-4">
            {mode === "analysis" ? (
              <AnalysisHistory
                onSelect={(id) => {
                  setHistoryOpen(false);
                  router.push(`/app/reporting/history/${id}`);
                }}
                onClose={() => setHistoryOpen(false)}
              />
            ) : (
              <ReportHistory
                onSelect={(id) => {
                  setHistoryOpen(false);
                  router.push(`/app/reporting/history/${id}`);
                }}
                onClose={() => setHistoryOpen(false)}
              />
            )}
          </div>
        </SheetContent>
      </Sheet>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{mode === "analysis" ? "Generate Analysis" : "Generate Reports"}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">Select date range</p>
            <div className="flex items-center gap-3">
              <Tabs
                value={selectedPeriod}
                onValueChange={(value) => setSelectedPeriod(value as AnalysisPeriod)}
                className="w-full"
              >
                <TabsList className="grid w-full grid-cols-3">
                  <TabsTrigger value="last_7_days">Last 7 days</TabsTrigger>
                  <TabsTrigger value="last_30_days">Last 30 days</TabsTrigger>
                  <TabsTrigger value="last_90_days">Last 90 days</TabsTrigger>
                </TabsList>
                <TabsList className="grid w-full grid-cols-3 mt-2">
                  <TabsTrigger value="this_month">This month</TabsTrigger>
                  <TabsTrigger value="last_month">Last month</TabsTrigger>
                  <TabsTrigger value="this_quarter">This quarter</TabsTrigger>
                </TabsList>
                <TabsList className="grid w-full grid-cols-3 mt-2">
                  <TabsTrigger value="last_quarter">Last quarter</TabsTrigger>
                  <TabsTrigger value="this_year">This year</TabsTrigger>
                  <TabsTrigger value="last_year">Last year</TabsTrigger>
                </TabsList>
              </Tabs>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={isGenerating}
            >
              Cancel
            </Button>
            <Button onClick={handleConfirmGenerate} disabled={isGenerating}>
              {isGenerating ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <ModeIcon className="mr-2 h-4 w-4" />
              )}
              Done
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
