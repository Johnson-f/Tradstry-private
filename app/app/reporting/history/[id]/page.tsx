"use client";

import { notFound, useParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { MarkdownFormatter } from "@/components/reporting/analysis/markdown-formatter";
import { useAiAnalysisHistoryItem } from "@/lib/hooks/use-ai-analysis";
import { AppPageHeader } from "@/components/app-page-header";

export default function AnalysisHistoryDetailPage() {
  const params = useParams<{ id: string }>();
  const id = params?.id;
  const { record, loading, error } = useAiAnalysisHistoryItem(id);

  if (!id) return notFound();
  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center text-muted-foreground">
        <Loader2 className="h-5 w-5 animate-spin mr-2" />
        Loading analysis...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center text-destructive">
        Failed to load analysis: {error.message}
      </div>
    );
  }

  if (!record) return notFound();

  let content = record.content;
  try {
    const parsed = JSON.parse(record.content);
    content = parsed?.comprehensive?.content || record.content;
  } catch {
    // fall back to raw content
  }

  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader
        title={record.title || "Analysis"}
        subtitle={`${record.time_range} · ${new Date(record.created_at).toLocaleString()}`}
      />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="rounded-lg border bg-card p-4 shadow-sm">
          <MarkdownFormatter content={content} />
        </div>
      </div>
    </div>
  );
}

