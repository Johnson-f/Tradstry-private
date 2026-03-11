import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Loader2, RefreshCw } from "lucide-react";
import { useAiAnalysisHistory } from "@/lib/hooks/use-ai-analysis";

interface AnalysisHistoryProps {
  onSelect?: (id: string) => void;
  onClose?: () => void;
}

export function AnalysisHistory({ onSelect, onClose }: AnalysisHistoryProps) {
  const { records, loading, isFetching, refresh } = useAiAnalysisHistory(20, 0);

  const sorted = useMemo(
    () =>
      [...records].sort(
        (a, b) =>
          new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      ),
    [records]
  );

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between py-3">
        <div className="space-y-1">
          <p className="text-sm font-semibold ml-4">Analysis History</p>
          <p className="text-xs text-muted-foreground ml-4">
            Most recent first
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => refresh()}
            disabled={isFetching}
            aria-label="Refresh history"
          >
            {isFetching ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
          </Button>
          {onClose && (
            <Button variant="ghost" size="sm" onClick={onClose}>
              Close
            </Button>
          )}
        </div>
      </div>
      <Separator />
      <div className="flex-1 overflow-y-auto py-2">
        {loading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground ml-4 px-2 py-3">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading history...
          </div>
        ) : sorted.length === 0 ? (
          <div className="px-2 py-3 text-sm text-muted-foreground ml-4">
            No analyses saved yet.
          </div>
        ) : (
          <div className="space-y-1">
            {sorted.map((item) => (
              <button
                key={item.id}
                className="w-full rounded-md border border-transparent px-2 py-3 text-left transition hover:border-primary/40 hover:bg-muted"
                onClick={() => onSelect?.(item.id)}
              >
                <div className="text-sm font-semibold line-clamp-1">
                  {item.title || `Analysis (${item.time_range})`}
                </div>
                <div className="text-xs text-muted-foreground flex items-center justify-between">
                  <span>{item.time_range}</span>
                  <span>{new Date(item.created_at).toLocaleString()}</span>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
