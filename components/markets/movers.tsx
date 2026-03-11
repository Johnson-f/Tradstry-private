'use client';

import React, { useMemo, useState, useEffect } from 'react';
import Image from 'next/image';
import { Card, CardContent } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Skeleton } from '@/components/ui/skeleton';
import { useMovers, useSimpleQuotes, useLogo } from '@/lib/hooks/use-market-data-service';
import { cn } from '@/lib/utils';
import type { MoverItem } from '@/lib/types/market-data';

interface MoverRowProps {
  mover: MoverItem;
  logo?: string | null;
  quoteData?: { price?: string | null; percentChange?: string | null; change?: string | null };
}

function MoverRow({ mover, logo, quoteData }: MoverRowProps) {
  const { logo: fetchedLogo } = useLogo(!logo ? mover.symbol : null, !logo);
  const [imageError, setImageError] = useState(false);
  
  // Reset image error when logo changes
  useEffect(() => {
    setImageError(false);
  }, [logo]);
  
  // Use quote data for real-time updates if available, otherwise fall back to mover data
  const price = quoteData?.price ?? mover.price;
  const percentChangeStr = quoteData?.percentChange ?? mover.percentChange;
  
  // Parse percentChange to determine if positive or negative
  const percentChange = percentChangeStr 
    ? parseFloat(percentChangeStr.replace(/[+%]/g, '')) 
    : null;
  const isPositive = percentChange !== null && percentChange > 0;
  
  // Format percentChange - ensure it has + sign and % if missing
  const displayPercentChange = useMemo(() => {
    if (!percentChangeStr) return '0%';
    let formatted = percentChangeStr;
    // Add + sign if positive and missing
    if (percentChange !== null && percentChange > 0 && !formatted.includes('+')) {
      formatted = '+' + formatted;
    }
    // Add % if missing
    if (!formatted.includes('%')) {
      formatted = formatted + '%';
    }
    return formatted;
  }, [percentChangeStr, percentChange]);
  
  const displayPrice = price || 'N/A';

  // Get initials from stock name (prefer name over symbol)
  const initials = useMemo(() => {
    const text = mover.name || mover.symbol || '';
    // Remove common suffixes and clean the name
    const cleanName = text
      .replace(/\s+(Inc|Corp|Corporation|Company|Co|Ltd|Limited|LLC|Holdings|Holdings, Inc\.?)$/i, '')
      .trim();
    
    // Get first two letters, handling single word names
    const words = cleanName.split(/\s+/);
    if (words.length >= 2) {
      // Multi-word: use first letter of first two words
      return (words[0][0] + words[1][0]).toUpperCase();
    } else {
      // Single word: use first two letters
      return cleanName.substring(0, 2).toUpperCase();
    }
  }, [mover.name, mover.symbol]);

  // Determine if we should show logo or fallback
  const resolvedLogo = logo || fetchedLogo || '';
  const shouldShowLogo = resolvedLogo && !imageError && resolvedLogo.trim() !== '';

  return (
    <div className="flex items-center justify-between py-3 border-b border-border last:border-b-0">
      <div className="flex items-center gap-3 flex-1 min-w-0">
        {/* Logo */}
        {shouldShowLogo ? (
          <div className="h-10 w-10 rounded-lg overflow-hidden flex-shrink-0 bg-muted">
            <Image
              src={resolvedLogo}
              alt={mover.name || mover.symbol}
              width={40}
              height={40}
              className="object-cover"
              unoptimized
              onError={() => setImageError(true)}
            />
          </div>
        ) : (
          <div className={cn(
            'h-10 w-10 rounded-lg flex items-center justify-center text-white font-semibold text-xs flex-shrink-0',
            isPositive ? 'bg-green-500' : 'bg-red-500'
          )}>
            {initials}
          </div>
        )}

        {/* Company Info */}
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-foreground truncate">
            {mover.name || mover.symbol}
          </p>
          <p className="text-xs text-muted-foreground">
            {mover.symbol} · NASDAQ
          </p>
        </div>
      </div>

      {/* Price and Change */}
      <div className="text-right flex-shrink-0 ml-4">
        <p className="text-sm font-medium text-foreground">
          ${displayPrice}
        </p>
        <p className={cn(
          'text-xs font-medium bg-transparent',
          isPositive ? 'text-blue-500' : 'text-red-500'
        )}>
          {displayPercentChange}
        </p>
      </div>
    </div>
  );
}

export function Movers() {
  const { movers, isLoading, error } = useMovers();

  // Debug logging
  useEffect(() => {
    if (movers) {
      console.log('📊 Movers data:', {
        gainers: movers.gainers?.length || 0,
        losers: movers.losers?.length || 0,
        mostActive: movers.mostActive?.length || 0,
        data: movers,
      });
    }
  }, [movers]);

  // Extract all symbols from movers
  const allSymbols = useMemo(() => {
    if (!movers) return [];
    return [
      ...(movers.gainers || []).map((m) => m.symbol),
      ...(movers.losers || []).map((m) => m.symbol),
      ...(movers.mostActive || []).map((m) => m.symbol),
    ];
  }, [movers]);

  // Get quotes with logos for all mover symbols
  const { quotes } = useSimpleQuotes(
    { symbols: allSymbols },
    allSymbols.length > 0
  );

  // Create maps for logo and quote data (for real-time price updates)
  const logoMap = useMemo(() => {
    const map = new Map<string, string | null>();
    if (Array.isArray(quotes)) {
    quotes.forEach((quote) => {
      map.set(quote.symbol.toUpperCase(), quote.logo || null);
    });
    }
    return map;
  }, [quotes]);

  // Create a map of symbol to quote data for real-time price updates
  const quoteMap = useMemo(() => {
    const map = new Map<string, { price?: string | null; percentChange?: string | null; change?: string | null }>();
    if (Array.isArray(quotes)) {
    quotes.forEach((quote) => {
      map.set(quote.symbol.toUpperCase(), {
        price: quote.price,
        percentChange: quote.percentChange,
        change: quote.change,
      });
    });
    }
    return map;
  }, [quotes]);

  if (error) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-sm text-destructive">
            Failed to load market movers. Please try again later.
          </p>
          <p className="text-xs text-muted-foreground mt-2">
            Error: {error.message}
          </p>
        </CardContent>
      </Card>
    );
  }

  // Show error if movers is null after loading
  if (!isLoading && !movers) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-sm text-destructive">
            No market movers data available. The API may be unavailable or returned an empty response.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Tabs - Outside the card as per image */}
      <Tabs defaultValue="gainers" className="w-full">
        <TabsList className="bg-transparent border-b border-border rounded-none h-auto p-0 gap-24">
          <TabsTrigger
            value="gainers"
            className="data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:border-b-2 data-[state=active]:border-transparent data-[state=active]:shadow-none rounded-none px-0 py-2 text-muted-foreground hover:text-foreground transition-colors font-medium bg-transparent border-transparent dark:data-[state=active]:bg-transparent dark:data-[state=active]:border-transparent focus-visible:ring-0 focus-visible:outline-none"
          >
            Gainers
          </TabsTrigger>
          <TabsTrigger
            value="losers"
            className="data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:border-b-2 data-[state=active]:border-transparent data-[state=active]:shadow-none rounded-none px-0 py-2 text-muted-foreground hover:text-foreground transition-colors font-medium bg-transparent border-transparent dark:data-[state=active]:bg-transparent dark:data-[state=active]:border-transparent focus-visible:ring-0 focus-visible:outline-none"
          >
            Losers
          </TabsTrigger>
          <TabsTrigger
            value="active"
            className="data-[state=active]:bg-transparent data-[state=active]:text-foreground data-[state=active]:border-b-2 data-[state=active]:border-transparent data-[state=active]:shadow-none rounded-none px-0 py-2 text-muted-foreground hover:text-foreground transition-colors font-medium bg-transparent border-transparent dark:data-[state=active]:bg-transparent dark:data-[state=active]:border-transparent focus-visible:ring-0 focus-visible:outline-none"
          >
            Active
          </TabsTrigger>
        </TabsList>

        {/* Card Content */}
        <Card>
          <CardContent className="p-0">
            {isLoading ? (
              <div className="px-6 pb-6 pt-3 space-y-4">
                {[...Array(5)].map((_, i) => (
                  <div key={i} className="flex items-center justify-between py-3">
                    <div className="flex items-center gap-3 flex-1">
                      <Skeleton className="h-10 w-10 rounded-lg" />
                      <div className="flex-1 space-y-2">
                        <Skeleton className="h-4 w-32" />
                        <Skeleton className="h-3 w-24" />
                      </div>
                    </div>
                    <div className="text-right space-y-2">
                      <Skeleton className="h-4 w-16" />
                      <Skeleton className="h-3 w-12" />
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <>
                {/* Gainers Tab */}
                <TabsContent value="gainers" className="m-0 p-0">
                  <div className="px-6 pb-6 pt-3">
                    {movers?.gainers && Array.isArray(movers.gainers) && movers.gainers.length > 0 ? (
                      movers.gainers.slice(0, 5).map((mover) => (
                        <MoverRow
                          key={mover.symbol}
                          mover={mover}
                          logo={logoMap.get(mover.symbol.toUpperCase())}
                          quoteData={quoteMap.get(mover.symbol.toUpperCase())}
                        />
                      ))
                    ) : (
                      <p className="text-sm text-muted-foreground py-8 text-center">
                        {isLoading ? 'Loading gainers...' : 'No gainers data available'}
                      </p>
                    )}
                  </div>
                </TabsContent>

                {/* Losers Tab */}
                <TabsContent value="losers" className="m-0 p-0">
                  <div className="px-6 pb-6 pt-3">
                    {movers?.losers && Array.isArray(movers.losers) && movers.losers.length > 0 ? (
                      movers.losers.slice(0, 5).map((mover) => (
                        <MoverRow
                          key={mover.symbol}
                          mover={mover}
                          logo={logoMap.get(mover.symbol.toUpperCase())}
                          quoteData={quoteMap.get(mover.symbol.toUpperCase())}
                        />
                      ))
                    ) : (
                      <p className="text-sm text-muted-foreground py-8 text-center">
                        {isLoading ? 'Loading losers...' : 'No losers data available'}
                      </p>
                    )}
                  </div>
                </TabsContent>

                {/* Active Tab */}
                <TabsContent value="active" className="m-0 p-0">
                  <div className="px-6 pb-6 pt-3">
                    {movers?.mostActive && Array.isArray(movers.mostActive) && movers.mostActive.length > 0 ? (
                      movers.mostActive.slice(0, 5).map((mover) => (
                        <MoverRow
                          key={mover.symbol}
                          mover={mover}
                          logo={logoMap.get(mover.symbol.toUpperCase())}
                          quoteData={quoteMap.get(mover.symbol.toUpperCase())}
                        />
                      ))
                    ) : (
                      <p className="text-sm text-muted-foreground py-8 text-center">
                        {isLoading ? 'Loading active stocks...' : 'No active stocks data available'}
                      </p>
                    )}
                  </div>
                </TabsContent>
              </>
            )}
          </CardContent>
        </Card>
      </Tabs>
    </div>
  );
}
