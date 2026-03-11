'use client';

import React, { useState, useCallback, useRef, useEffect, useMemo, useDeferredValue } from 'react';
import Image from 'next/image';
import { Search, X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useSymbolSearch, useSimpleQuotes, useLogo } from '@/lib/hooks/use-market-data-service';
import { cn } from '@/lib/utils';
import { useRouter } from 'next/navigation';

export function MarketSearch() {
  const [searchQuery, setSearchQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const router = useRouter();

  const deferredQuery = useDeferredValue(searchQuery);

  const { results, isLoading } = useSymbolSearch(
    deferredQuery,
    { hits: 10 },
    deferredQuery.length >= 2
  );

  // Extract symbols from search results
  const resultSymbols = useMemo(() => {
    return results.map((item) => item.symbol);
  }, [results]);

  // Fetch quotes with logos for search results
  const { quotes } = useSimpleQuotes(
    { symbols: resultSymbols },
    resultSymbols.length > 0 && deferredQuery.length >= 2
  );

  // Create a map of symbol to logo
  const logoMap = useMemo(() => {
    const map = new Map<string, string | null>();
    if (Array.isArray(quotes)) {
    quotes.forEach((quote) => {
      map.set(quote.symbol.toUpperCase(), quote.logo || null);
    });
    }
    return map;
  }, [quotes]);

  const handleSelect = useCallback((symbol: string) => {
    setIsOpen(false);
    setSearchQuery('');
    // Navigate to symbol detail page or handle selection
    // For now, we'll just log it - you can navigate to a symbol page if needed
    router.push(`/app/markets/${symbol}`);
  }, [router]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      setIsOpen(false);
      setSearchQuery('');
    }
  }, []);

  // Close popover when clicking outside
  useEffect(() => {
    if (!isOpen) {
      setSearchQuery('');
    }
  }, [isOpen]);

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <div ref={containerRef} className="relative w-full max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
          <Input
            ref={inputRef}
            type="text"
            placeholder="Search symbols..."
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setIsOpen(true);
            }}
            onKeyDown={handleKeyDown}
            onFocus={() => searchQuery.length >= 2 && setIsOpen(true)}
            className="pl-9 pr-9 w-full"
            autoComplete="off"
            spellCheck={false}
          />
          {searchQuery && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setSearchQuery('');
                setIsOpen(false);
              }}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
      </PopoverTrigger>
      <PopoverContent 
        className="w-[var(--radix-popover-trigger-width)] p-0 max-w-md" 
        align="start"
        onOpenAutoFocus={(e) => e.preventDefault()}
        onCloseAutoFocus={(e) => e.preventDefault()}
      >
        {isLoading && deferredQuery.length >= 2 ? (
          <div className="p-4 text-sm text-muted-foreground text-center">
            Searching...
          </div>
        ) : results.length > 0 ? (
          <ScrollArea className="h-[300px]">
            <div className="pr-4">
              {results.map((item) => {
                const quotedLogo = logoMap.get(item.symbol.toUpperCase());
                return (
                  <SearchResultItem
                    key={item.symbol}
                    symbol={item.symbol}
                    name={item.name}
                    exchange={item.exchange as string | undefined}
                    quotedLogo={quotedLogo || undefined}
                    onSelect={handleSelect}
                  />
                );
              })}
            </div>
          </ScrollArea>
        ) : deferredQuery.length >= 2 ? (
          <div className="p-4 text-sm text-muted-foreground text-center">
            No results found
          </div>
        ) : (
          <div className="p-4 text-sm text-muted-foreground text-center">
            Type at least 2 characters to search
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}

function SearchResultItem({
  symbol,
  name,
  exchange,
  quotedLogo,
  onSelect,
}: {
  symbol: string;
  name?: string | null;
  exchange?: string | null;
  quotedLogo?: string | null;
  onSelect: (symbol: string) => void;
}) {
  const { logo } = useLogo(!quotedLogo ? symbol : null, !quotedLogo);
  const displayName = name || symbol;
  const fallbackText = displayName.substring(0, 2).toUpperCase();
  const [imageError, setImageError] = React.useState(false);
  React.useEffect(() => { setImageError(false); }, [logo, quotedLogo]);

  const resolvedLogo = (logo || quotedLogo || '').trim();
  const shouldShowLogo = resolvedLogo !== '' && !imageError;

  return (
    <button
      onClick={() => onSelect(symbol)}
      className={cn(
        'w-full text-left px-4 py-3 hover:bg-muted transition-colors',
        'flex items-center gap-3'
      )}
    >
      {shouldShowLogo ? (
        <div className="relative h-10 w-10 rounded-lg overflow-hidden border bg-muted/50 flex-shrink-0">
          <Image
            src={resolvedLogo}
            alt={displayName}
            fill
            className="object-contain p-1.5"
            sizes="40px"
            unoptimized
            onError={() => setImageError(true)}
          />
        </div>
      ) : (
        <div className="h-10 w-10 rounded-lg border bg-muted/50 flex items-center justify-center flex-shrink-0">
          <span className="text-sm font-bold text-muted-foreground">
            {fallbackText}
          </span>
        </div>
      )}

      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2">
          <span className="font-medium text-sm">{symbol}</span>
          {exchange && (
            <span className="text-xs text-muted-foreground">{exchange}</span>
          )}
        </div>
        {name && (
          <span className="text-xs text-muted-foreground truncate block">
            {name}
          </span>
        )}
      </div>
    </button>
  );
}
