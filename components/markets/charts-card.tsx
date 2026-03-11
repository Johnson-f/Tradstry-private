'use client';

import React, { useMemo } from 'react';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { useQuotes, useHistorical } from '@/lib/hooks/use-market-data-service';
import { format, parseISO } from 'date-fns';
import { cn } from '@/lib/utils';
import { TrendingUp, TrendingDown } from 'lucide-react';

interface IndexConfig {
  symbol: string;
  displayName: string;
  indexCode: string;
}

const INDICES: IndexConfig[] = [
  { symbol: 'SPY', displayName: 'S&P 500', indexCode: '^SPX · INDEX' },
  { symbol: 'QQQ', displayName: 'NASDAQ', indexCode: '^IXIC · INDEX' },
  { symbol: 'DIA', displayName: 'Dow Jones', indexCode: '^DJI · INDEX' },
] as const;

interface ChartDataPoint {
  time: string;
  close: number;
  timestamp: number;
}

interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{
    payload: ChartDataPoint;
    value: number;
  }>;
  label?: string;
}

const CustomTooltip = ({ active, payload, label }: CustomTooltipProps) => {
  if (active && payload && payload.length && label) {
    const data = payload[0].payload;
    try {
      const date = parseISO(data.time);
      return (
        <div className="bg-background border border-border rounded-lg p-3 shadow-lg">
          <p className="text-sm text-muted-foreground mb-1">
            {format(date, 'MMM d, yyyy h:mm a')}
          </p>
          <p className="text-lg font-semibold">
            ${data.close.toFixed(2)}
          </p>
        </div>
      );
    } catch {
      return (
        <div className="bg-background border border-border rounded-lg p-3 shadow-lg">
          <p className="text-sm text-muted-foreground mb-1">{label}</p>
          <p className="text-lg font-semibold">
            ${data.close.toFixed(2)}
          </p>
        </div>
      );
    }
  }
  return null;
};

interface SymbolChartProps {
  config: IndexConfig;
}

function SymbolChart({ config }: SymbolChartProps) {
  const { quotes, isLoading: quotesLoading } = useQuotes(
    { symbols: [config.symbol] },
    true
  );
  const { historical, isLoading: historicalLoading } = useHistorical(
    { symbol: config.symbol, range: '1d', interval: '15m' },
    true
  );

  const isLoading = quotesLoading || historicalLoading;
  const quote = quotes?.[0];

  // Transform historical data for chart
  const chartData: ChartDataPoint[] = useMemo(() => {
    if (!historical?.candles || historical.candles.length === 0) return [];

    return historical.candles.map((candle) => ({
      time: candle.time,
      close: candle.close,
      timestamp: new Date(candle.time).getTime(),
    }));
  }, [historical]);

  // Calculate price change
  const priceChange = useMemo(() => {
    if (!quote || !chartData.length) return null;

    const currentPrice = parseFloat(quote.price || '0');
    const previousClose = chartData[0]?.close || currentPrice;
    const change = currentPrice - previousClose;
    const percentChange = previousClose !== 0 ? (change / previousClose) * 100 : 0;

    return {
      price: currentPrice,
      change,
      percentChange,
      isPositive: change >= 0,
    };
  }, [quote, chartData]);

  if (isLoading) {
    return (
      <Card className="bg-card">
        <CardHeader className="pb-2">
          <Skeleton className="h-6 w-24 mb-1" />
          <Skeleton className="h-3 w-20" />
        </CardHeader>
        <CardContent className="p-4">
          <Skeleton className="h-[100px] w-full" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-card">
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex flex-col gap-0.5">
            <h3 className="text-lg font-semibold text-foreground">
              {config.displayName}
            </h3>
            <p className="text-[10px] text-muted-foreground">
              {config.indexCode}
            </p>
          </div>
          {priceChange && (
            <div className="flex flex-col items-end gap-0.5">
              <div className={cn(
                'flex items-center gap-0.5 px-1.5 py-0.5 rounded-md',
                priceChange.isPositive ? 'text-green-600 bg-green-600/10' : 'text-red-600 bg-red-600/10'
              )}>
                {priceChange.isPositive ? (
                  <TrendingUp className="h-2.5 w-2.5" />
                ) : (
                  <TrendingDown className="h-2.5 w-2.5" />
                )}
                <span className="text-[10px] font-medium">
                  {priceChange.isPositive ? '+' : ''}
                  {priceChange.percentChange.toFixed(2)}%
                </span>
              </div>
              <p className={cn(
                'text-xs font-medium',
                priceChange.isPositive ? 'text-green-600' : 'text-red-600'
              )}>
                {priceChange.isPositive ? '+' : ''}
                {priceChange.change.toFixed(priceChange.change % 1 !== 0 ? 2 : 0)}
              </p>
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent className="relative p-4">
        {chartData.length === 0 ? (
          <div className="flex items-center justify-center h-[100px] text-muted-foreground text-xs">
            No historical data available
          </div>
        ) : (
          <>
            <div className="h-[100px] -mx-4">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart
                  data={chartData}
                  margin={{ top: 0, right: 0, left: 0, bottom: 0 }}
                >
                  <defs>
                    <linearGradient id={`gradient-${config.symbol}-positive`} x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#22c55e" stopOpacity={0.3} />
                      <stop offset="100%" stopColor="#22c55e" stopOpacity={0.05} />
                    </linearGradient>
                    <linearGradient id={`gradient-${config.symbol}-negative`} x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#ef4444" stopOpacity={0.3} />
                      <stop offset="100%" stopColor="#ef4444" stopOpacity={0.05} />
                    </linearGradient>
                  </defs>
                  <XAxis
                    dataKey="time"
                    hide
                  />
                  <YAxis
                    hide
                    domain={['dataMin', 'dataMax']}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Area
                    type="monotone"
                    dataKey="close"
                    stroke={priceChange?.isPositive ? '#22c55e' : '#ef4444'}
                    strokeWidth={1.5}
                    fill={`url(#gradient-${config.symbol}-${priceChange?.isPositive ? 'positive' : 'negative'})`}
                    dot={false}
                    activeDot={{ r: 3, fill: priceChange?.isPositive ? '#22c55e' : '#ef4444' }}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
            {priceChange && (
              <div className="absolute bottom-2 left-4">
                <p className="text-lg font-semibold text-foreground">
                  {priceChange.price.toLocaleString('en-US', {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 3,
                  })}
                </p>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

export function ChartsCard() {
  return (
    <div className="flex flex-row gap-3">
      {INDICES.map((index) => (
        <div key={index.symbol} className="flex-1 min-w-0 max-w-[320px]">
          <SymbolChart config={index} />
        </div>
      ))}
    </div>
  );
}
