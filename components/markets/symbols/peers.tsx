"use client";

import React, { useMemo } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";
import { useSimilar, useLogo } from "@/lib/hooks/use-market-data-service";
import { Skeleton } from "@/components/ui/skeleton";
import type { SimpleQuote } from "@/lib/types/market-data";

interface PeersProps {
  symbol: string;
  className?: string;
}

export function Peers({ symbol, className }: PeersProps) {
  const router = useRouter();
  const { similar, isLoading } = useSimilar(symbol, !!symbol);

  // Ensure we have an array of SimpleQuote items
  const items = useMemo(() => {
    if (!Array.isArray(similar)) return [];
    return similar.filter((item): item is SimpleQuote => 
      item && typeof item === 'object' && 'symbol' in item
    );
  }, [similar]);

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <div className="p-5 sm:p-6">
        <h3 className="text-lg font-semibold mb-4">Peers</h3>

        {isLoading ? (
          <LoadingList />
        ) : items.length === 0 ? (
          <div className="text-sm text-muted-foreground">No peers found.</div>
        ) : (
          <div className="divide-y">
            {items.map((peer) => (
              <PeerItem key={peer.symbol} peer={peer} onClick={() => router.push(`/app/markets/${peer.symbol}`)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function LoadingList() {
  return (
    <div className="divide-y">
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} className="flex items-center gap-3 sm:gap-4 py-3">
          <Skeleton className="h-10 w-10 rounded-lg" />
          <div className="flex-1 min-w-0">
            <Skeleton className="h-4 w-56" />
            <div className="mt-2">
              <Skeleton className="h-3 w-24" />
            </div>
          </div>
          <div className="flex flex-col items-end gap-2">
            <Skeleton className="h-4 w-12" />
            <Skeleton className="h-3 w-10" />
          </div>
        </div>
      ))}
    </div>
  );
}

export default Peers;



function PeerItem({ peer, onClick }: { peer: SimpleQuote; onClick: () => void }) {
  const { logo } = useLogo(!peer.logo ? peer.symbol : null, !peer.logo);
  const [imageError, setImageError] = React.useState(false);
  React.useEffect(() => { setImageError(false); }, [logo, peer.logo]);
  
  const priceText = peer.price ? `$${peer.price}` : "—";
  const pctNum = peer.percentChange ? parseFloat(String(peer.percentChange)) : NaN;
  const pctColor = isNaN(pctNum)
    ? "text-muted-foreground"
    : pctNum > 0
    ? "text-emerald-500"
    : pctNum < 0
    ? "text-red-500"
    : "text-muted-foreground";

  const displayName = peer.name || peer.symbol;
  const subline = peer.symbol;

  const resolvedLogo = (peer.logo || logo || '').trim();
  const shouldShowLogo = resolvedLogo !== '' && !imageError;

  return (
    <button
      onClick={onClick}
      className={cn(
        "w-full px-1 py-3 sm:py-3.5",
        "flex items-center gap-3 sm:gap-4 text-left hover:bg-muted/50 rounded-lg transition-colors"
      )}
    >
      {shouldShowLogo ? (
        <div className="h-10 w-10 rounded-lg overflow-hidden bg-muted flex-shrink-0 relative">
          <img
            src={resolvedLogo}
            alt={displayName || peer.symbol}
            className="w-full h-full object-cover"
            onError={() => setImageError(true)}
            loading="lazy"
          />
        </div>
      ) : (
        <div className="h-10 w-10 rounded-lg bg-muted flex items-center justify-center text-xs font-semibold text-foreground/80 flex-shrink-0">
          {displayName.substring(0, 2).toUpperCase()}
        </div>
      )}

      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm sm:text-base font-medium truncate">{displayName}</div>
            <div className="text-xs text-muted-foreground mt-0.5 truncate">{subline}</div>
          </div>

          <div className="text-right flex-shrink-0">
            <div className="text-sm sm:text-base font-medium">{priceText}</div>
            <div className={cn("text-xs mt-0.5", pctColor)}>
              {peer.percentChange ? `${peer.percentChange}%` : "—"}
            </div>
          </div>
        </div>
      </div>
    </button>
  );
}
