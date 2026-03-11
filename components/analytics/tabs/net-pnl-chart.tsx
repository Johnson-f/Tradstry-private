'use client';

import React, { useState, useMemo } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics';
import type { TimeRange } from '@/lib/types/analytics';
import { format, parseISO } from 'date-fns';

type TimeRangeOption = '7d' | '30d' | '90d' | '1y' | 'ytd' | 'all_time';

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
  cumulativePnl: number;
  dailyPnl: number;
  tradeCount: number;
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
    const date = parseISO(label);
    
    return (
      <div className="bg-background border border-border rounded-lg p-3 shadow-lg">
        <p className="text-sm text-muted-foreground mb-1">
          {format(date, 'MMM d, yyyy')}
        </p>
        <p className={`text-lg font-semibold ${
          data.cumulativePnl >= 0 ? 'text-green-600' : 'text-red-600'
        }`}>
          ${data.cumulativePnl.toLocaleString('en-US', {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          })}
        </p>
        <p className={`text-sm ${
          data.dailyPnl >= 0 ? 'text-green-600' : 'text-red-600'
        }`}>
          Daily: {data.dailyPnl >= 0 ? '+' : ''}${data.dailyPnl.toLocaleString('en-US', {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          })}
        </p>
        {data.tradeCount > 0 && (
          <p className="text-xs text-muted-foreground mt-1">
            {data.tradeCount} trade{data.tradeCount !== 1 ? 's' : ''}
          </p>
        )}
      </div>
    );
  }
  return null;
};

export function NetPnlChart() {
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('30d');

  const { data, isLoading, error } = useAnalyticsTimeSeries({
    time_range: selectedTimeRange as TimeRange,
  });

  // Transform time series data for chart
  const chartData: ChartDataPoint[] = useMemo(() => {
    if (!data?.daily_pnl || data.daily_pnl.length === 0) return [];

    return data.daily_pnl.map((point) => ({
      date: point.date,
      cumulativePnl: point.cumulative_value,
      dailyPnl: point.value,
      tradeCount: point.trade_count,
    }));
  }, [data]);

  // Calculate overall stats
  const stats = useMemo(() => {
    if (chartData.length === 0) {
      return {
        currentPnl: 0,
        totalChange: 0,
        percentageChange: 0,
        isPositive: true,
        startPnl: 0,
      };
    }

    const currentPnl = chartData[chartData.length - 1]?.cumulativePnl || 0;
    const startPnl = chartData[0]?.cumulativePnl || 0;
    const totalChange = currentPnl - startPnl;
    const percentageChange = startPnl !== 0 ? (totalChange / Math.abs(startPnl)) * 100 : 0;

    return {
      currentPnl,
      totalChange,
      percentageChange,
      isPositive: totalChange >= 0,
      startPnl,
    };
  }, [chartData]);

  // Format date labels based on time range
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

  if (error) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-destructive">
            <p className="mb-4">Failed to load Net P&L data: {error.message}</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Net P&L Chart</CardTitle>
          {!isLoading && chartData.length > 0 && (
            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="text-sm text-muted-foreground">Current Net P&L</p>
                <p className={`text-2xl font-bold ${
                  stats.isPositive ? 'text-green-600' : 'text-red-600'
                }`}>
                  ${stats.currentPnl.toLocaleString('en-US', {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}
                </p>
              </div>
              <div className="text-right">
                <p className="text-sm text-muted-foreground">Period Change</p>
                <p className={`text-lg font-semibold ${
                  stats.isPositive ? 'text-green-600' : 'text-red-600'
                }`}>
                  {stats.isPositive ? '+' : ''}${stats.totalChange.toLocaleString('en-US', {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}
                </p>
                <p className={`text-sm ${
                  stats.isPositive ? 'text-green-600' : 'text-red-600'
                }`}>
                  ({stats.isPositive ? '+' : ''}{stats.percentageChange.toFixed(2)}%)
                </p>
              </div>
            </div>
          )}
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

        {/* Chart */}
        {isLoading ? (
          <div className="space-y-4">
            <Skeleton className="h-[400px] w-full" />
          </div>
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-[400px] text-muted-foreground">
            No trading data available for the selected time range
          </div>
        ) : (
          <div className="h-[400px]">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart
                data={chartData}
                margin={{ top: 10, right: 30, left: 0, bottom: 20 }}
              >
                <defs>
                  <linearGradient
                    id="pnlGradient"
                    x1="0"
                    y1="0"
                    x2="0"
                    y2="1"
                  >
                    <stop
                      offset="5%"
                      stopColor={stats.isPositive ? '#22c55e' : '#ef4444'}
                      stopOpacity={0.3}
                    />
                    <stop
                      offset="95%"
                      stopColor={stats.isPositive ? '#22c55e' : '#ef4444'}
                      stopOpacity={0.05}
                    />
                  </linearGradient>
                </defs>
                
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
                
                <Tooltip content={<CustomTooltip />} />
                
                {/* Zero line reference */}
                <Area
                  type="monotone"
                  dataKey={() => 0}
                  stroke="#6b7280"
                  strokeWidth={1}
                  strokeDasharray="5 5"
                  fill="none"
                />
                
                <Area
                  type="monotone"
                  dataKey="cumulativePnl"
                  stroke={stats.isPositive ? '#22c55e' : '#ef4444'}
                  strokeWidth={2.5}
                  fill="url(#pnlGradient)"
                  dot={false}
                  activeDot={{ r: 6, fill: stats.isPositive ? '#22c55e' : '#ef4444' }}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

