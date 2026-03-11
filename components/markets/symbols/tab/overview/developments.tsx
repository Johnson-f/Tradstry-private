"use client";

import React, { useMemo } from "react";
import { useNews } from "@/lib/hooks/use-market-data-service";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";

interface DevelopmentsProps {
  symbol: string;
  className?: string;
}

export function Developments({ symbol, className }: DevelopmentsProps) {
  const { news, isLoading } = useNews({ symbol, limit: 3 }, !!symbol);

  const items = useMemo(() => (Array.isArray(news) ? news : []).slice(0, 3), [news]);

  if (isLoading) return <Loading className={className} />;

  if (!items.length) {
    return (
      <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
        <div className="text-sm text-muted-foreground">No recent developments.</div>
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <div className="p-5 sm:p-6">
        <h3 className="text-lg font-semibold mb-4">Recent Developments</h3>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {items.map((n) => {
            const timeStr = n.publish_time_formatted || (n.publish_time ? new Date(n.publish_time * 1000).toISOString() : null);
            return (
            <a
                key={`${n.uuid || n.link}-${n.link}`}
              href={n.link}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-xl border bg-card/60 hover:bg-muted/40 transition-colors p-4 flex flex-col gap-3"
            >
              {n.thumbnail_url && (
                <div className="w-full h-40 rounded-lg overflow-hidden bg-muted relative -mx-4 -mt-4 mb-2">
                  <img 
                    src={n.thumbnail_url} 
                    alt={n.title || "News thumbnail"} 
                    className="w-full h-full object-cover"
                    onError={(e) => {
                      // Hide image container on error
                      const container = e.currentTarget.parentElement;
                      if (container) {
                        container.style.display = 'none';
                      }
                    }}
                    loading="lazy"
                  />
                </div>
              )}
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>{timeAgo(timeStr)}</span>
                {n.publisher && (
                  <span className="text-muted-foreground/70">•</span>
                )}
                {n.publisher && (
                  <span>{n.publisher}</span>
                )}
              </div>
              <div className="text-base font-semibold leading-snug line-clamp-2">{n.title}</div>
            </a>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function Loading({ className }: { className?: string }) {
  return (
    <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="rounded-xl border bg-card/60 p-4 space-y-3">
            <div className="flex items-center gap-2">
              <Skeleton className="h-5 w-5 rounded" />
              <Skeleton className="h-3 w-24" />
            </div>
            <Skeleton className="h-4 w-5/6" />
            <Skeleton className="h-4 w-3/5" />
          </div>
        ))}
      </div>
    </div>
  );
}

function timeAgo(isoOrDate: string | Date | null | undefined) {
  try {
    if (!isoOrDate) return "recently";
    
    const date = typeof isoOrDate === "string" ? new Date(isoOrDate) : isoOrDate;
    
    // Check if date is valid
    if (!(date instanceof Date) || isNaN(date.getTime())) {
      return "recently";
    }
    
    const now = new Date();
    const diff = Math.max(0, now.getTime() - date.getTime());
    
    if (isNaN(diff)) {
      return "recently";
    }
    
    const minutes = Math.floor(diff / (1000 * 60));
    if (minutes < 1) return "just now";
    if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
    const days = Math.floor(hours / 24);
    return `${days} day${days === 1 ? "" : "s"} ago`;
  } catch {
    return "recently";
  }
}

export default Developments;
