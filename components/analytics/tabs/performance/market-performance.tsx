'use client';

import React, { useState, useMemo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics';
import { useHistorical, useQuote } from '@/lib/hooks/use-market-data-service';
import type { TimeRange } from '@/lib/types/analytics';
import { format, parseISO, subDays, startOfYear, differenceInDays } from 'date-fns';

type TimeRangeOption = '7d' | '30d' | '90d' | 'ytd' | '1y' | 'all_time';

const TIME_RANGE_OPTIONS: Array<{ value: TimeRangeOption; label: string }> = [
  { value: '7d', label: '7D' },
  { value: '30d', label: '30D' },
  { value: '90d', label: '90D' },
  { value: 'ytd', label: 'YTD' },
  { value: '1y', label: '1Y' },
  { value: 'all_time', label: 'ALL' },
];

interface ChartDataPoint {
  date: string;
  userPnl: number;
  spyPrice: number;
  spyPercentageChange: number;
}

interface PerformanceMetrics {
  '1d': { user: number; spy: number; diff: number };
  '7d': { user: number; spy: number; diff: number };
  '30d': { user: number; spy: number; diff: number };
  '90d': { user: number; spy: number; diff: number };
  qtd: { user: number; spy: number; diff: number };
  '1y': { user: number; spy: number; diff: number };
}

interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{
    dataKey: string;
    value: number;
    payload: ChartDataPoint;
    color: string;
  }>;
  label?: string;
}

const CustomTooltip = ({ active, payload, label }: CustomTooltipProps) => {
  if (active && payload && payload.length && label) {
    const data = payload[0]?.payload as ChartDataPoint;
    if (!data) return null;

    return (
      <div className="bg-background border border-border rounded-lg p-3 shadow-lg">
        <p className="text-sm text-muted-foreground mb-2 font-medium">
          {format(parseISO(label), 'MMM d, yyyy')}
        </p>
        <div className="space-y-1">
          <p className={`text-sm font-semibold ${
            data.userPnl >= 0 ? 'text-green-600' : 'text-red-600'
          }`}>
            Your P&L: ${data.userPnl.toLocaleString('en-US', {
              minimumFractionDigits: 2,
              maximumFractionDigits: 2,
            })}
          </p>
          <p className={`text-sm font-semibold ${
            data.spyPercentageChange >= 0 ? 'text-blue-600' : 'text-blue-400'
          }`}>
            SPY: {data.spyPercentageChange >= 0 ? '+' : ''}
            {data.spyPercentageChange.toFixed(2)}%
          </p>
        </div>
      </div>
    );
  }
  return null;
};

export function MarketPerformance() {
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('30d');

  // Fetch user's Net P&L data
  const { data: userData, isLoading: userLoading, error: userError } = useAnalyticsTimeSeries({
    time_range: selectedTimeRange as TimeRange,
  });

  // Determine SPY historical range based on selected time range
  const spyTimeRange = useMemo(() => {
    switch (selectedTimeRange) {
      case '7d':
        return '5d';
      case '30d':
        return '1mo';
      case '90d':
        return '3mo';
      case 'ytd':
        return 'ytd';
      case '1y':
        return '1y';
      case 'all_time':
        return 'max';
      default:
        return '1mo';
    }
  }, [selectedTimeRange]);

  // Fetch SPY historical data
  const { historical: spyHistorical, isLoading: spyHistoricalLoading } = useHistorical(
    {
      symbol: 'SPY',
      range: spyTimeRange as any,
      interval: '1d',
    },
    true
  );

  // Fetch real-time SPY quote for latest price
  const { quote: spyQuote } = useQuote('SPY', true);

  // Merge historical SPY data with real-time quote
  const spyDataWithRealTime = useMemo(() => {
    if (!spyHistorical?.candles) return [];
    
    const candles = [...spyHistorical.candles];
    
    // If we have a real-time quote and the last candle date doesn't match today, add it
    if (spyQuote?.price) {
      const today = new Date().toISOString().split('T')[0];
      const lastCandleDate = candles[candles.length - 1]?.time?.split('T')[0];
      
      if (lastCandleDate !== today) {
        candles.push({
          time: new Date().toISOString(),
          close: Number(spyQuote.price),
          open: Number(spyQuote.price),
          high: Number(spyQuote.price),
          low: Number(spyQuote.price),
        });
      } else if (candles.length > 0) {
        // Update last candle with real-time price
        candles[candles.length - 1] = {
          ...candles[candles.length - 1],
          close: Number(spyQuote.price),
        };
      }
    }
    
    return candles;
  }, [spyHistorical, spyQuote]);

  // Transform and align data for chart
  const chartData: ChartDataPoint[] = useMemo(() => {
    if (!userData?.daily_pnl || userData.daily_pnl.length === 0) return [];
    if (!spyDataWithRealTime || spyDataWithRealTime.length === 0) return [];

    // Create a map of user P&L by date
    const userPnlMap = new Map<string, number>();
    userData.daily_pnl.forEach((point) => {
      const dateKey = point.date.split('T')[0]; // Use date part only
      userPnlMap.set(dateKey, point.cumulative_value);
    });

    // Get first SPY price as baseline
    const firstSpyPrice = Number(spyDataWithRealTime[0]?.close) || 0;
    if (firstSpyPrice === 0) return [];

    // Create aligned data points
    const alignedData: ChartDataPoint[] = [];
    const processedDates = new Set<string>();

    // Process SPY data and align with user P&L dates
    spyDataWithRealTime.forEach((candle) => {
      const dateKey = candle.time.split('T')[0];
      const userPnl = userPnlMap.get(dateKey);

      // Only include dates where we have user P&L data
      if (userPnl !== undefined && !processedDates.has(dateKey)) {
        const spyPrice = Number(candle.close) || firstSpyPrice;
        const spyPercentageChange = ((spyPrice - firstSpyPrice) / firstSpyPrice) * 100;

        alignedData.push({
          date: candle.time,
          userPnl,
          spyPrice,
          spyPercentageChange,
        });

        processedDates.add(dateKey);
      }
    });

    // Also include user P&L dates that might not be in SPY data
    userData.daily_pnl.forEach((point) => {
      const dateKey = point.date.split('T')[0];
      if (!processedDates.has(dateKey)) {
        // Find nearest SPY price for this date
        const userDate = parseISO(point.date);
        let nearestSpyPrice = firstSpyPrice;

        for (const candle of spyDataWithRealTime) {
          const candleDate = parseISO(candle.time);
          const daysDiff = Math.abs(differenceInDays(userDate, candleDate));
          if (daysDiff <= 1) {
            nearestSpyPrice = Number(candle.close) || firstSpyPrice;
            break;
          }
        }

        const spyPercentageChange = ((nearestSpyPrice - firstSpyPrice) / firstSpyPrice) * 100;

        alignedData.push({
          date: point.date,
          userPnl: point.cumulative_value,
          spyPrice: nearestSpyPrice,
          spyPercentageChange,
        });

        processedDates.add(dateKey);
      }
    });

    // Sort by date
    return alignedData.sort((a, b) => 
      new Date(a.date).getTime() - new Date(b.date).getTime()
    );
  }, [userData, spyDataWithRealTime]);

  // Calculate performance metrics
  const performanceMetrics: PerformanceMetrics = useMemo(() => {
    if (chartData.length === 0) {
      return {
        '1d': { user: 0, spy: 0, diff: 0 },
        '7d': { user: 0, spy: 0, diff: 0 },
        '30d': { user: 0, spy: 0, diff: 0 },
        '90d': { user: 0, spy: 0, diff: 0 },
        qtd: { user: 0, spy: 0, diff: 0 },
        '1y': { user: 0, spy: 0, diff: 0 },
      };
    }

    const now = new Date();
    const latestData = chartData[chartData.length - 1];
    
    const getMetric = (daysAgo: number) => {
      const targetDate = subDays(now, daysAgo);
      
      // Find closest data point to target date
      let startData: ChartDataPoint | undefined;
      let minDiff = Infinity;
      
      for (const point of chartData) {
        const pointDate = parseISO(point.date);
        const diff = Math.abs(differenceInDays(pointDate, targetDate));
        if (diff < minDiff) {
          minDiff = diff;
          startData = point;
        }
      }
      
      if (!startData || !latestData) {
        return { user: 0, spy: 0, diff: 0 };
      }

      const userChange = latestData.userPnl - startData.userPnl;
      const spyChange = latestData.spyPercentageChange - startData.spyPercentageChange;
      
      // Calculate what SPY performance would be worth in dollars based on user's starting P&L
      const spyEquivalent = startData.userPnl !== 0 
        ? (startData.userPnl * spyChange / 100)
        : 0;
      const diff = userChange - spyEquivalent;

      return { user: userChange, spy: spyChange, diff };
    };

    const qtdStart = startOfYear(now);
    let qtdData: ChartDataPoint | undefined;
    let qtdMinDiff = Infinity;
    
    for (const point of chartData) {
      const pointDate = parseISO(point.date);
      const diff = Math.abs(differenceInDays(pointDate, qtdStart));
      if (diff < qtdMinDiff) {
        qtdMinDiff = diff;
        qtdData = point;
      }
    }

    const qtdMetric = qtdData && latestData ? {
      user: latestData.userPnl - qtdData.userPnl,
      spy: latestData.spyPercentageChange - qtdData.spyPercentageChange,
      diff: (latestData.userPnl - qtdData.userPnl) - 
            (qtdData.userPnl !== 0 ? (qtdData.userPnl * (latestData.spyPercentageChange - qtdData.spyPercentageChange) / 100) : 0),
    } : { user: 0, spy: 0, diff: 0 };

    return {
      '1d': getMetric(1),
      '7d': getMetric(7),
      '30d': getMetric(30),
      '90d': getMetric(90),
      qtd: qtdMetric,
      '1y': getMetric(365),
    };
  }, [chartData]);

  // Format date labels
  const formatDateLabel = (dateStr: string): string => {
    try {
      const date = parseISO(dateStr);
      
      switch (selectedTimeRange) {
        case '7d':
          return format(date, 'MMM d');
        case '30d':
          return format(date, 'MMM d');
        case '90d':
          return format(date, 'MMM d');
        case 'ytd':
        case '1y':
          return format(date, 'MMM yyyy');
        case 'all_time':
          return format(date, 'MMM yyyy');
        default:
          return format(date, 'MMM d');
      }
    } catch {
      return dateStr;
    }
  };

  const isLoading = userLoading || spyHistoricalLoading;
  const error = userError;

  if (error) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-destructive">
            <p>Failed to load market performance data: {error.message}</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Performance vs SPY</CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        {/* Time range selector */}
        <div className="flex items-center gap-2 mb-6">
          {TIME_RANGE_OPTIONS.map((option) => (
            <Button
              key={option.value}
              variant={selectedTimeRange === option.value ? 'default' : 'outline'}
              size="sm"
              onClick={() => setSelectedTimeRange(option.value)}
            >
              {option.label}
            </Button>
          ))}
        </div>

        {/* Performance metrics */}
        {!isLoading && chartData.length > 0 && (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-6">
            {Object.entries(performanceMetrics).map(([period, metrics]) => (
              <div key={period} className="border rounded-lg p-3">
                <p className="text-xs text-muted-foreground mb-2 uppercase">{period}</p>
                <div className="space-y-1">
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">You</span>
                    <span className={`text-sm font-semibold ${
                      metrics.user >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {metrics.user >= 0 ? '+' : ''}${metrics.user.toFixed(2)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-muted-foreground">SPY</span>
                    <span className={`text-sm font-semibold ${
                      metrics.spy >= 0 ? 'text-blue-600' : 'text-blue-400'
                    }`}>
                      {metrics.spy >= 0 ? '+' : ''}{metrics.spy.toFixed(2)}%
                    </span>
                  </div>
                  <div className="flex justify-between items-center pt-1 border-t">
                    <span className="text-xs font-medium">Diff</span>
                    <span className={`text-xs font-semibold ${
                      metrics.diff >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {metrics.diff >= 0 ? '+' : ''}${metrics.diff.toFixed(2)}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Chart */}
        {isLoading ? (
          <div className="space-y-4">
            <Skeleton className="h-[400px] w-full" />
          </div>
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-[400px] text-muted-foreground">
            No data available for the selected time range
          </div>
        ) : (
          <div className="h-[400px]">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart
                data={chartData}
                margin={{ top: 10, right: 30, left: 0, bottom: 20 }}
              >
                <CartesianGrid
                  strokeDasharray="3 3"
                  strokeOpacity={0.2}
                  vertical={false}
                />
                
                <XAxis
                  dataKey="date"
                  tickFormatter={formatDateLabel}
                  tick={{ fontSize: 12 }}
                  tickMargin={10}
                  angle={-45}
                  textAnchor="end"
                  height={80}
                />
                
                <YAxis
                  yAxisId="left"
                  tickFormatter={(value) =>
                    `$${value.toLocaleString('en-US', {
                      minimumFractionDigits: 0,
                      maximumFractionDigits: 0,
                    })}`
                  }
                  tick={{ fontSize: 12 }}
                  tickMargin={10}
                  width={80}
                />
                
                <YAxis
                  yAxisId="right"
                  orientation="right"
                  tickFormatter={(value) => `${value.toFixed(1)}%`}
                  tick={{ fontSize: 12 }}
                  tickMargin={10}
                  width={60}
                />
                
                <Tooltip content={<CustomTooltip />} />
                <Legend />
                
                <Line
                  yAxisId="left"
                  type="monotone"
                  dataKey="userPnl"
                  stroke="#22c55e"
                  strokeWidth={2.5}
                  dot={false}
                  activeDot={{ r: 6 }}
                  name="Your P&L"
                />
                
                <Line
                  yAxisId="right"
                  type="monotone"
                  dataKey="spyPercentageChange"
                  stroke="#3b82f6"
                  strokeWidth={2.5}
                  dot={false}
                  activeDot={{ r: 6 }}
                  name="SPY %"
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
