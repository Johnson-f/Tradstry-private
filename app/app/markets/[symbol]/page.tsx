"use client";

import { useParams } from "next/navigation";
import { MarketSearch } from "@/components/markets/market-search";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { ChevronRight, Share2, Star } from "lucide-react";
import Link from "next/link";
import Image from "next/image";
import { SymbolTabs } from "@/components/markets/symbols/tab";
import { useQuote } from "@/lib/hooks/use-market-data-service";

export default function MarketsSymbolPage() {
  const params = useParams<{ symbol: string }>();
  const symbol = (params?.symbol || "").toString().toUpperCase();

  return (
    <div className="h-screen flex flex-col">
      {/* Header - Fixed */}
      <Header />
      {/* Main content - Scrollable area with ScrollArea */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-8">
            <CompanyHeader symbol={symbol} />
            <div className="mt-8">
              <SymbolTabs symbol={symbol} />
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}


// Function to render the header
function Header() {
  const params = useParams<{ symbol: string }>();
  const symbol = (params?.symbol || "").toString().toUpperCase();
  const { quote: data } = useQuote(symbol, !!symbol);

  return (
    <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
      <div className="relative flex items-center justify-between">
        {/* Left: Breadcrumb */}
        <div className="flex items-center gap-2 whitespace-nowrap">
          <Link href="/app/markets" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
            Markets
          </Link>
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
          {/* Logo in breadcrumb */}
          {data?.logo ? (
            <div className="relative h-5 w-5 flex-shrink-0 rounded overflow-hidden border bg-muted/50">
              <Image
                src={data.logo}
                alt={data.name || symbol}
                fill
                className="object-contain p-0.5"
                sizes="20px"
              />
            </div>
          ) : (
            <div className="h-5 w-5 flex-shrink-0 rounded border bg-muted/50 flex items-center justify-center">
              <span className="text-xs font-bold text-muted-foreground">
                {symbol.charAt(0)}
              </span>
            </div>
          )}
          <span className="text-sm font-medium">{symbol || "Symbol"}</span>
        </div>

        {/* Center: Search (absolutely centered) */}
        <div className="absolute left-1/2 -translate-x-1/2 w-full max-w-2xl">
          <MarketSearch />
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            <Star className="h-4 w-4 mr-2" />
            Follow
          </Button>
          <Button variant="secondary" size="sm">
            <Share2 className="h-4 w-4 mr-2" />
            Share
          </Button>
        </div>
      </div>
    </div>
  );
}

// Company Header Component
function CompanyHeader({ symbol }: { symbol: string }) {
  const { quote: data, isLoading } = useQuote(symbol, !!symbol);

  if (isLoading) {
    return (
      <div className="flex items-center gap-4">
        <Skeleton className="h-16 w-16 rounded-lg" />
        <div className="space-y-2">
          <Skeleton className="h-6 w-48" />
          <Skeleton className="h-4 w-24" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-4">
      {/* Company Logo */}
      {data?.logo ? (
        <div className="relative h-16 w-16 flex-shrink-0 rounded-lg overflow-hidden border bg-muted/50">
          <Image
            src={data.logo}
            alt={data.name || symbol}
            fill
            className="object-contain p-2"
            sizes="64px"
          />
        </div>
      ) : (
        <div className="h-16 w-16 flex-shrink-0 rounded-lg border bg-muted/50 flex items-center justify-center">
          <span className="text-2xl font-bold text-muted-foreground">
            {symbol.charAt(0)}
          </span>
        </div>
      )}

      {/* Company Name and Ticker */}
      <div className="flex flex-col">
        <h1 className="text-2xl font-semibold text-foreground">
          {data?.name || symbol}
        </h1>
        <span className="text-sm text-muted-foreground font-medium">
          {symbol}
        </span>
      </div>
    </div>
  );
}