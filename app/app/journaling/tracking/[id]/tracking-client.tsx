'use client';

import React from 'react';
import { useRouter } from 'next/navigation';
import { useStock } from '@/lib/hooks/use-stocks';
import { useOption } from '@/lib/hooks/use-options';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ArrowLeft, Loader2 } from 'lucide-react';
import Stats from '@/components/journaling/tracking/tabs/stats';
import Playbook from '@/components/journaling/tracking/tabs/playbook';
import type { Stock } from '@/lib/types/stocks';
import type { OptionTrade } from '@/lib/types/options';
import { useHistorical } from '@/lib/hooks/use-market-data-service';
import Charting from '@/components/journaling/tracking/charting';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
// import TradeNotes from '@/components/journaling/tracking/trade-notes';

interface TradeTrackingClientProps {
  params: Promise<{ id: string }>;
}

function parseTradeId(id: string): { type: 'stock' | 'option'; numericId: number } | null {
  const match = id.match(/^(stock|option)-(\d+)$/);
  if (!match) return null;
  
  const [, type, numericIdStr] = match;
  const numericId = parseInt(numericIdStr, 10);
  
  if (isNaN(numericId) || numericId <= 0) return null;
  
  return { type: type as 'stock' | 'option', numericId };
}

function formatEntryDate(entryDate: string): string {
  if (!entryDate) return '';
  
  try {
    const date = new Date(entryDate);
    if (isNaN(date.getTime())) return '';
    
    const month = date.getMonth() + 1; // getMonth() returns 0-11
    const day = date.getDate();
    const year = date.getFullYear();
    
    // Format: M/DD/YYYY (month without leading zero, day with leading zero)
    return `${month}/${day.toString().padStart(2, '0')}/${year}`;
  } catch {
    return '';
  }
}

export default function TradeTrackingClient({ params }: TradeTrackingClientProps) {
  const router = useRouter();
  const [resolvedParams, setResolvedParams] = React.useState<{ id: string } | null>(null);
  const [parsedId, setParsedId] = React.useState<{ type: 'stock' | 'option'; numericId: number } | null>(null);

  React.useEffect(() => {
    void params.then(setResolvedParams);
  }, [params]);

  React.useEffect(() => {
    if (resolvedParams) {
      const parsed = parseTradeId(resolvedParams.id);
      setParsedId(parsed);
    }
  }, [resolvedParams]);

  const stockQuery = useStock(parsedId?.numericId ?? 0, parsedId?.type === 'stock' && parsedId !== null);
  const optionQuery = useOption(parsedId?.numericId ?? 0, parsedId?.type === 'option' && parsedId !== null);

  const isLoading = parsedId === null || (parsedId.type === 'stock' ? stockQuery.isLoading : optionQuery.isLoading);
  const error = parsedId === null 
    ? new Error('Invalid trade ID format')
    : (parsedId.type === 'stock' ? stockQuery.error : optionQuery.error);
  const trade = parsedId?.type === 'stock' ? stockQuery.data : optionQuery.data;

  // Derive symbol early for hooks consistency
  const symbol = React.useMemo(() => {
    if (!parsedId || !trade) return '';
    return parsedId.type === 'stock' ? (trade as Stock).symbol : (trade as OptionTrade).symbol;
  }, [parsedId, trade]);

  // ================================
  // Time range / interval controls
  // ================================
  const allowedOptions = React.useMemo(
    () => [
      { label: '1d', range: '1d', interval: '1m' },
      { label: '5d', range: '5d', interval: '5m' },
      { label: '1mo', range: '1mo', interval: '15m' },
      { label: '3mo', range: '3mo', interval: '30m' },
      { label: '6mo', range: '6mo', interval: '1h' },
      { label: 'ytd', range: 'ytd', interval: '1d' },
      { label: '1y', range: '1y', interval: '1mo' },
      { label: '2y', range: '2y', interval: undefined },
      { label: '5y', range: '5y', interval: undefined },
      { label: '10y', range: '10y', interval: undefined },
      { label: 'max', range: 'max', interval: undefined },
    ],
    []
  );

  const [range, setRange] = React.useState<string>('6mo');
  const [interval, setInterval] = React.useState<string | undefined>('1d');

  const setPreset = React.useCallback((label: string) => {
    const found = allowedOptions.find((o) => o.label === label);
    if (found) {
      setRange(found.range);
      // Only set interval if defined for this preset; otherwise leave as-is
      if (typeof found.interval !== 'undefined') {
        setInterval(found.interval);
      }
    }
  }, [allowedOptions]);

  // Allow user to manually choose interval
  const allowedIntervals = React.useMemo(
    () => ['1m', '5m', '15m', '30m', '1h', '1d', '1wk', '1mo'],
    []
  );

  // Always call historical hook (disabled until symbol exists) to keep hook order consistent
  const { historical, isLoading: isHistLoading, error: histError } = useHistorical(
    {
      symbol: symbol || '',
      range,
      // Pass interval only when defined; service omits undefined
      interval,
    },
    !!symbol
  );

  const chartData = React.useMemo(() => {
    const candles = historical?.candles ?? [];
    return candles.map((c) => ({
      time: Number(c.time),
      open: c.open,
      high: c.high,
      low: c.low,
      close: c.close,
      volume: Number(c.volume ?? 0),
    }));
  }, [historical]);

  // Stable trade notes document id from route param
  const notesDocId = React.useMemo(() => {
    return resolvedParams ? `trade-note-${resolvedParams.id}` : undefined;
  }, [resolvedParams]);

  if (!resolvedParams) {
    return (
      <div className="h-screen flex flex-col">
        <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
          <div className="flex items-center gap-4">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => router.push('/app/journaling')}
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-2xl font-bold tracking-tight">Tracking</h1>
          </div>
        </div>
        <div className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-8 flex items-center justify-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          </ScrollArea>
        </div>
      </div>
    );
  }

  if (!parsedId) {
    return (
      <div className="h-screen flex flex-col">
        <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
          <div className="flex items-center gap-4">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => router.push('/app/journaling')}
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-2xl font-bold tracking-tight">Tracking</h1>
          </div>
        </div>
        <div className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-8">
              <p className="text-muted-foreground">
                The trade ID format is invalid. Expected format: <code className="bg-muted px-1 py-0.5 rounded">stock-123</code> or <code className="bg-muted px-1 py-0.5 rounded">option-456</code>
              </p>
              <Button
                className="mt-4"
                onClick={() => router.push('/app/journaling')}
              >
                Back to Journaling
              </Button>
            </div>
          </ScrollArea>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="h-screen flex flex-col">
        <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
          <div className="flex items-center gap-4">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => router.push('/app/journaling')}
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-2xl font-bold tracking-tight">Tracking</h1>
          </div>
        </div>
        <div className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-8 flex items-center justify-center">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          </ScrollArea>
        </div>
      </div>
    );
  }

  if (error || !trade) {
    return (
      <div className="h-screen flex flex-col">
        <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
          <div className="flex items-center gap-4">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => router.push('/app/journaling')}
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-2xl font-bold tracking-tight">Tracking</h1>
          </div>
        </div>
        <div className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-8">
              <p className="text-muted-foreground">
                {error instanceof Error ? error.message : 'The requested trade could not be found.'}
              </p>
              <Button
                className="mt-4"
                onClick={() => router.push('/app/journaling')}
              >
                Back to Journaling
              </Button>
            </div>
          </ScrollArea>
        </div>
      </div>
    );
  }

  const entryDate = parsedId.type === 'stock' ? (trade as Stock).entryDate : (trade as OptionTrade).entryDate;
  const formattedDate = formatEntryDate(entryDate);

  return (
    <div className="h-screen flex flex-col">
      {/* Header - Fixed */}
      <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => router.push('/app/journaling')}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h1 className="text-2xl font-bold tracking-tight">Tracking</h1>
        </div>
      </div>
      
      {/* Secondary Header - Symbol and Entry Date */}
      <div className="w-full border-b bg-background px-8 py-3 flex-shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-lg font-semibold">{symbol}</span>
          {formattedDate && (
            <>
              <span className="text-muted-foreground">•</span>
              <span className="text-sm text-muted-foreground">{formattedDate}</span>
            </>
          )}
        </div>
      </div>
      
      {/* Main content - Two column layout */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-6 grid grid-cols-12 gap-6">
            {/* Left Column - Tabs controlling Stats / Playbook (approximately 1/3 width) */}
            <div className="col-span-12 lg:col-span-4 xl:col-span-3">
              <div className="h-[600px]">
                <Tabs defaultValue="stats" className="w-full h-full flex flex-col">
                  <TabsList className="mb-2 grid w-full grid-cols-2">
                    <TabsTrigger value="stats">Stats</TabsTrigger>
                    <TabsTrigger value="playbook">Playbook</TabsTrigger>
                  </TabsList>

                  <TabsContent value="stats" className="m-0 h-[560px]">
                    <Stats 
                      tradeId={parsedId.numericId}
                      tradeType={parsedId.type}
                      trade={trade}
                    />
                  </TabsContent>

                  <TabsContent value="playbook" className="m-0 h-[560px]">
                    <Playbook className="h-full" />
                  </TabsContent>
                </Tabs>
              </div>
            </div>
            
            {/* Right Column - Chart & Notes Panel (approximately 2/3 width) */}
            <div className="col-span-12 lg:col-span-8 xl:col-span-9 space-y-6">
              {/* Chart Section */}
              <div className="bg-background border rounded-lg h-[500px] p-2">
                {/* Timeframe Toolbar */}
                <div className="flex items-center gap-1 mb-2">
                  {allowedOptions.map((o) => (
                    <Button
                      key={o.label}
                      size="sm"
                      variant={
                        // Active if ranges match and either preset has an interval and matches, or preset has no interval
                        range === o.range && (typeof o.interval === 'undefined' || interval === o.interval)
                          ? 'default'
                          : 'ghost'
                      }
                      onClick={() => setPreset(o.label)}
                    >
                      {o.label}
                    </Button>
                  ))}
                  {/* Interval selector */}
                  <div className="ml-auto flex items-center gap-2">
                    <label className="text-xs text-muted-foreground">Interval</label>
                    <Select value={interval ?? 'default'} onValueChange={(v) => setInterval(v === 'default' ? undefined : v)}>
                      <SelectTrigger className="h-8 w-[110px]">
                        <SelectValue placeholder="default" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="default">default</SelectItem>
                        {allowedIntervals.map((iv) => (
                          <SelectItem key={iv} value={iv}>{iv}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                {histError && (
                  <div className="h-full flex items-center justify-center">
                    <p className="text-sm text-muted-foreground">Failed to load historical data</p>
                  </div>
                )}
                {isHistLoading && !histError && (
                  <div className="h-full flex items-center justify-center">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                )}
                {!isHistLoading && !histError && chartData.length === 0 && (
                  <div className="h-full flex items-center justify-center">
                    <p className="text-sm text-muted-foreground">No historical data available</p>
                  </div>
                )}
                {!isHistLoading && !histError && chartData.length > 0 && (
                  <Charting data={chartData} height={480} className="w-full" />
                )}
              </div>
              
              {/* Trade Notes Section with Tabs (Trade note / Daily Journal) */}
              {/* <div className="bg-background border rounded-lg p-2">
                <Tabs defaultValue="trade-note" className="w-full">
                  <TabsList className="mb-2">
                    <TabsTrigger value="trade-note">Trade note</TabsTrigger>
                    <TabsTrigger value="daily-journal">Daily Journal</TabsTrigger>
                  </TabsList>

                  <TabsContent value="trade-note" className="m-0">
                    <div className="h-[360px] overflow-hidden">
                      {notesDocId ? (
                        <ScrollArea className="h-full">
                          <div className="p-4">
                            <TradeNotes 
                              docId={notesDocId}
                              tradeType={parsedId.type}
                              tradeId={parsedId.numericId}
                              className="h-full"
                            />
                          </div>
                        </ScrollArea>
                      ) : (
                        <div className="h-full flex items-center justify-center">
                          <p className="text-muted-foreground">Loading notes...</p>
                        </div>
                      )}
                    </div>
                  </TabsContent>

                  <TabsContent value="daily-journal">
                    <div className="h-[300px] flex items-center justify-center text-sm text-muted-foreground">
                      Daily Journal coming soon
                    </div>
                  </TabsContent>
                </Tabs>
              </div> */}
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
