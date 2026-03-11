'use client';

import React, { useMemo } from 'react';
import Image from 'next/image';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';
import { useMovers, useQuotes, useHistorical, useNews, useLogo } from '@/lib/hooks/use-market-data-service';
import { format, parseISO } from 'date-fns';
import { cn } from '@/lib/utils';
import { TrendingUp, TrendingDown } from 'lucide-react';
import type { Quote } from '@/lib/types/market-data';

interface ChartDataPoint {
  time: string;
  close: number;
  timestamp: number;
}

interface StandoutStock {
  symbol: string;
  name: string;
  price: number;
  change: number;
  percentChange: number;
  isPositive: boolean;
  quote: Quote;
  exchange?: string;
}

// Helper to parse market cap string (e.g., "55.37B", "1.2T", "500M")
function parseMarketCap(marketCap: string | null | undefined): number {
  if (!marketCap) return 0;
  
  const clean = marketCap.replace(/[^0-9.BMT]/gi, '').toUpperCase();
  const match = clean.match(/^([\d.]+)([BMT]?)$/);
  
  if (!match) return 0;
  
  const value = parseFloat(match[1]);
  const suffix = match[2];
  
  switch (suffix) {
    case 'T':
      return value * 1_000_000_000_000;
    case 'B':
      return value * 1_000_000_000;
    case 'M':
      return value * 1_000_000;
    default:
      return value;
  }
}

// Format market cap for display
function formatMarketCap(value: number): string {
  if (value >= 1_000_000_000_000) {
    return `${(value / 1_000_000_000_000).toFixed(2)}T`;
  }
  if (value >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(2)}B`;
  }
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(2)}M`;
  }
  return value.toFixed(0);
}

// Format volume for display
function formatVolume(volume: number | null | undefined): string {
  if (!volume) return 'N/A';
  if (volume >= 1_000_000_000) {
    return `${(volume / 1_000_000_000).toFixed(2)}B`;
  }
  if (volume >= 1_000_000) {
    return `${(volume / 1_000_000).toFixed(2)}M`;
  }
  if (volume >= 1_000) {
    return `${(volume / 1_000).toFixed(2)}K`;
  }
  return volume.toString();
}

// Extract exchange from sector or symbol
function getExchange(symbol: string): string {
  // Simple heuristic - in production, you'd want to fetch this from API
  if (symbol.includes('.')) {
    const parts = symbol.split('.');
    return parts[parts.length - 1] || 'NASDAQ';
  }
  return 'NYSE'; // Default fallback
}

// Custom tooltip for chart
interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{
    payload: ChartDataPoint;
    value: number;
  }>;
  label?: string;
}

const ChartTooltip = ({ active, payload, label }: CustomTooltipProps) => {
  if (active && payload && payload.length && label) {
    const data = payload[0].payload;
    try {
      const date = parseISO(data.time);
      return (
        <div className="bg-background border border-border rounded-lg p-2 shadow-lg">
          <p className="text-xs text-muted-foreground mb-1">
            {format(date, 'h:mm a')}
          </p>
          <p className="text-sm font-semibold">
            ${data.close.toFixed(2)}
          </p>
        </div>
      );
    } catch {
      return (
        <div className="bg-background border border-border rounded-lg p-2 shadow-lg">
          <p className="text-xs text-muted-foreground mb-1">{label}</p>
          <p className="text-sm font-semibold">
            ${data.close.toFixed(2)}
          </p>
        </div>
      );
    }
  }
  return null;
};

interface StandoutCardProps {
  stock: StandoutStock;
}

function StandoutCard({ stock }: StandoutCardProps) {
  const { historical, isLoading: historicalLoading } = useHistorical(
    { symbol: stock.symbol, range: '1d', interval: '5m' },
    true
  );

  const { news } = useNews({ symbol: stock.symbol, limit: 1 });
  const { logo } = useLogo(!stock.quote.logo ? stock.symbol : null, !stock.quote.logo);

  // Transform chart data
  const chartData: ChartDataPoint[] = useMemo(() => {
    if (!historical?.candles || historical.candles.length === 0) return [];

    return historical.candles.map((candle) => ({
      time: candle.time,
      close: candle.close,
      timestamp: new Date(candle.time).getTime(),
    }));
  }, [historical]);

  // Calculate previous close (first candle's close or use open price)
  const previousClose = useMemo(() => {
    if (chartData.length > 0) {
      return chartData[0]?.close || parseFloat(stock.quote.open || stock.price.toString());
    }
    return parseFloat(stock.quote.open || stock.price.toString());
  }, [chartData, stock.quote.open, stock.price]);

  // Format time labels for 5-minute intervals
  const formatTimeLabel = (timeStr: string): string => {
    try {
      const date = parseISO(timeStr);
      return format(date, 'h:mm a');
    } catch {
      return timeStr;
    }
  };

  // Get market cap value
  const marketCapValue = parseMarketCap(stock.quote.marketCap);
  const marketCapDisplay = stock.quote.marketCap || formatMarketCap(marketCapValue);

  // Get dividend yield (may not be in Quote type, using placeholder)
  const dividendYield = 'N/A'; // Placeholder - would need API update for dividend yield

  // Get news summary
  const newsSummary = news && news.length > 0 
    ? news[0].title 
    : `${stock.name} shares ${stock.isPositive ? 'surged' : 'plunged'} today after ${stock.isPositive ? 'positive' : 'negative'} market developments.`;

  if (historicalLoading) {
    return (
      <>
        <CardHeader>
          <Skeleton className="h-6 w-48 mb-2" />
          <Skeleton className="h-4 w-32" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-[300px] w-full" />
        </CardContent>
      </>
    );
  }

  return (
    <>
      <CardHeader className="pb-4">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-3">
            {/* Logo */}
            {stock.quote.logo || logo ? (
              <div className="h-10 w-10 rounded overflow-hidden flex-shrink-0 bg-muted">
                <Image
                  src={stock.quote.logo || logo || ''}
                  alt={stock.name}
                  width={40}
                  height={40}
                  className="object-cover"
                  unoptimized
                />
              </div>
            ) : (
              <div className={cn(
                'h-10 w-10 rounded flex items-center justify-center text-white font-semibold text-xs',
                stock.isPositive ? 'bg-green-500' : 'bg-orange-500'
              )}>
                {stock.name.substring(0, 2).toUpperCase()}
              </div>
            )}
            <div>
              <h3 className="text-lg font-semibold text-foreground">
                {stock.name}
              </h3>
              <p className="text-sm text-muted-foreground">
                {stock.symbol} · {stock.exchange || 'NYSE'}
              </p>
            </div>
          </div>
          <div className="text-right">
            <p className="text-xl font-semibold text-foreground mb-1">
              ${stock.price.toFixed(2)}
            </p>
            <div className={cn(
              'inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium',
              stock.isPositive 
                ? 'text-green-600 bg-green-600/10' 
                : 'text-red-600 bg-red-600/10'
            )}>
              {stock.isPositive ? (
                <TrendingUp className="h-3 w-3" />
              ) : (
                <TrendingDown className="h-3 w-3" />
              )}
              <span>
                {stock.isPositive ? '+' : ''}
                {stock.percentChange.toFixed(2)}%
              </span>
            </div>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-[1fr_200px] gap-6">
          {/* Chart */}
          <div className="space-y-2">
            {chartData.length === 0 ? (
              <div className="h-[250px] flex items-center justify-center text-muted-foreground">
                No chart data available
              </div>
            ) : (
              <div className="h-[250px]">
                <ResponsiveContainer width="100%" height="100%">
                  <AreaChart
                    data={chartData}
                    margin={{ top: 10, right: 10, left: 0, bottom: 20 }}
                  >
                    <defs>
                      <linearGradient
                        id={`gradient-${stock.symbol}-${stock.isPositive ? 'positive' : 'negative'}`}
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor={stock.isPositive ? '#22c55e' : '#ef4444'}
                          stopOpacity={0.3}
                        />
                        <stop
                          offset="100%"
                          stopColor={stock.isPositive ? '#22c55e' : '#ef4444'}
                          stopOpacity={0.05}
                        />
                      </linearGradient>
                    </defs>
                    <XAxis
                      dataKey="time"
                      tickFormatter={formatTimeLabel}
                      tick={{ fontSize: 10 }}
                      tickMargin={8}
                      angle={-45}
                      textAnchor="end"
                      height={60}
                      interval="preserveStartEnd"
                    />
                    <YAxis
                      domain={['dataMin', 'dataMax']}
                      tick={{ fontSize: 10 }}
                      tickMargin={8}
                      width={50}
                    />
                    <Tooltip content={<ChartTooltip />} />
                    {/* Previous close reference line */}
                    <ReferenceLine
                      y={previousClose}
                      stroke="#6b7280"
                      strokeDasharray="3 3"
                      strokeOpacity={0.5}
                    />
                    <Area
                      type="monotone"
                      dataKey="close"
                      stroke={stock.isPositive ? '#22c55e' : '#ef4444'}
                      strokeWidth={2}
                      fill={`url(#gradient-${stock.symbol}-${stock.isPositive ? 'positive' : 'negative'})`}
                      dot={false}
                      activeDot={{ r: 4, fill: stock.isPositive ? '#22c55e' : '#ef4444' }}
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            )}
            {/* Previous close indicator */}
            <div className="flex items-center justify-end gap-2">
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <div className="h-px w-8 bg-muted-foreground/50" style={{ borderStyle: 'dashed' }} />
                <span>Prev close: ${previousClose.toFixed(2)}</span>
              </div>
            </div>
            {/* News summary */}
            <p className="text-sm text-muted-foreground leading-relaxed mt-4">
              {newsSummary}
            </p>
          </div>

          {/* Metrics */}
          <div className="space-y-4">
            <div>
              <p className="text-xs text-muted-foreground mb-1">Volume</p>
              <p className="text-sm font-medium text-foreground">
                {formatVolume(stock.quote.volume)}
              </p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground mb-1">Market Cap</p>
              <p className="text-sm font-medium text-foreground">
                {marketCapDisplay}
              </p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground mb-1">P/E Ratio</p>
              <p className="text-sm font-medium text-foreground">
                {stock.quote.pe || 'N/A'}
              </p>
            </div>
            <div>
              <p className="text-xs text-muted-foreground mb-1">Dividend Yield</p>
              <p className="text-sm font-medium text-foreground">
                {dividendYield}
              </p>
            </div>
          </div>
        </div>
      </CardContent>
    </>
  );
}

export function Standouts() {
  const { movers, isLoading: moversLoading } = useMovers();
  
  // Get all standout symbols (gainers and losers)
  const standoutSymbols = useMemo(() => {
    if (!movers) return [];
    
    const allMovers = [...movers.gainers, ...movers.losers];
    return allMovers
      .filter(mover => {
        const percentChange = parseFloat(mover.percentChange || '0');
        return Math.abs(percentChange) >= 5;
      })
      .map(mover => mover.symbol);
  }, [movers]);

  // Fetch detailed quotes for standout symbols
  const { quotes, isLoading: quotesLoading } = useQuotes(
    { symbols: standoutSymbols },
    standoutSymbols.length > 0
  );

  // Filter and combine data - separate gainers and losers, limit to 2 each
  const { gainers, losers } = useMemo(() => {
    if (!movers || !quotes || !Array.isArray(quotes) || quotes.length === 0) return { gainers: [], losers: [] };

    const gainerStocks: StandoutStock[] = [];
    const loserStocks: StandoutStock[] = [];

    // Process gainers
    for (const mover of movers.gainers) {
      if (gainerStocks.length >= 2) break;

      const percentChange = parseFloat(mover.percentChange || '0');
      
      // Filter by >= 5%
      if (percentChange < 5) continue;

      const quote = quotes.find(q => q.symbol === mover.symbol);
      if (!quote) continue;

      // Filter by market cap > $5B
      const marketCapValue = parseMarketCap(quote.marketCap);
      if (marketCapValue < 5_000_000_000) continue;

      const price = parseFloat(quote.price || '0');
      const change = parseFloat(quote.change || '0');

      gainerStocks.push({
        symbol: mover.symbol,
        name: quote.name || mover.name || mover.symbol,
        price,
        change,
        percentChange,
        isPositive: true,
        quote,
        exchange: getExchange(mover.symbol),
      });
    }

    // Process losers
    for (const mover of movers.losers) {
      if (loserStocks.length >= 2) break;

      const percentChange = parseFloat(mover.percentChange || '0');
      
      // Filter by <= -5%
      if (percentChange > -5) continue;

      const quote = Array.isArray(quotes) ? quotes.find(q => q.symbol === mover.symbol) : null;
      if (!quote) continue;

      // Filter by market cap > $5B
      const marketCapValue = parseMarketCap(quote.marketCap);
      if (marketCapValue < 5_000_000_000) continue;

      const price = parseFloat(quote.price || '0');
      const change = parseFloat(quote.change || '0');

      loserStocks.push({
        symbol: mover.symbol,
        name: quote.name || mover.name || mover.symbol,
        price,
        change,
        percentChange,
        isPositive: false,
        quote,
        exchange: getExchange(mover.symbol),
      });
    }

    // Sort by absolute percent change (descending)
    gainerStocks.sort((a, b) => Math.abs(b.percentChange) - Math.abs(a.percentChange));
    loserStocks.sort((a, b) => Math.abs(b.percentChange) - Math.abs(a.percentChange));

    return { gainers: gainerStocks.slice(0, 2), losers: loserStocks.slice(0, 2) };
  }, [movers, quotes]);

  // Combine all stocks (gainers first, then losers)
  const allStandoutStocks = useMemo(() => {
    return [...gainers, ...losers];
  }, [gainers, losers]);

  const isLoading = moversLoading || quotesLoading;

  if (isLoading) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold text-foreground">Standouts</h2>
        <Card>
          <CardContent className="p-0">
            {[1, 2, 3, 4].map((i) => (
              <div key={i}>
                <CardHeader>
                  <Skeleton className="h-6 w-48 mb-2" />
                  <Skeleton className="h-4 w-32" />
                </CardHeader>
                <CardContent>
                  <Skeleton className="h-[300px] w-full" />
                </CardContent>
                {i < 4 && <Separator />}
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!allStandoutStocks || allStandoutStocks.length === 0) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold text-foreground">Standouts</h2>
        <Card>
          <CardContent className="pt-6">
            <p className="text-muted-foreground">No standout stocks found</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold text-foreground">Standouts</h2>
      <Card>
        <CardContent className="p-0">
          {allStandoutStocks.map((stock, index) => (
            <div key={stock.symbol}>
              <StandoutCard stock={stock} />
              {index < allStandoutStocks.length - 1 && <Separator />}
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}
