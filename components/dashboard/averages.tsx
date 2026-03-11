'use client';

import React, { useMemo } from 'react';
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
import { Skeleton } from '@/components/ui/skeleton';
import { Info } from 'lucide-react';
import {
  Tooltip as UITooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics';
import type { TimeRange } from '@/lib/types/analytics';
import { format, parseISO } from 'date-fns';

interface ChartDataPoint {
  date: string;
  winRate: number; // Percentage
  avgWin: number; // Dollars
  avgLoss: number; // Dollars
}

interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{
    dataKey: string;
    value: number;
    color: string;
    payload: ChartDataPoint;
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
          {payload.map((entry, index) => (
            <div key={index}>
              {entry.dataKey === 'winRate' && (
                <p className="text-sm font-semibold text-green-600">
                  Win %: {data.winRate.toFixed(1)}%
                </p>
              )}
              {entry.dataKey === 'avgWin' && (
                <p className="text-sm font-semibold text-blue-600">
                  Avg Win: ${data.avgWin.toFixed(2)}
                </p>
              )}
              {entry.dataKey === 'avgLoss' && (
                <p className="text-sm font-semibold text-red-600">
                  Avg Loss: ${data.avgLoss.toFixed(2)}
                </p>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }
  return null;
};

interface AveragesProps {
  timeRange?: '7d' | '30d' | '90d' | '1y' | 'all_time' | 'custom';
  customStartDate?: Date;
  customEndDate?: Date;
}

export function Averages({ 
  timeRange = '30d',
  customStartDate,
  customEndDate 
}: AveragesProps) {
  // Convert dashboard timeRange to analytics TimeRange format
  const analyticsTimeRange: TimeRange = useMemo(() => {
    if (timeRange === 'custom' && customStartDate && customEndDate) {
      return {
        custom: {
          start_date: customStartDate.toISOString(),
          end_date: customEndDate.toISOString(),
        },
      };
    }
    // Map dashboard timeRange to analytics TimeRange
    if (timeRange === '7d' || timeRange === '30d' || timeRange === '90d' || timeRange === '1y' || timeRange === 'all_time') {
      return timeRange;
    }
    // Default to 30d if invalid
    return '30d';
  }, [timeRange, customStartDate, customEndDate]);

  const { data, isLoading, error } = useAnalyticsTimeSeries({
    time_range: analyticsTimeRange,
  });

  // Calculate rolling average win/loss from daily P&L data
  const chartData: ChartDataPoint[] = useMemo(() => {
    if (!data?.daily_pnl || !data?.rolling_win_rate_50 || data.daily_pnl.length === 0) {
      return [];
    }

    const windowSize = 20; // Rolling window for averaging
    const result: ChartDataPoint[] = [];

    // Create a map of win rates by date
    const winRateMap = new Map<string, number>();
    data.rolling_win_rate_50.forEach((point) => {
      const dateStr = point.date.split('T')[0];
      winRateMap.set(dateStr, point.value);
    });

    // Calculate rolling averages for win/loss
    for (let i = 0; i < data.daily_pnl.length; i++) {
      const currentDate = data.daily_pnl[i].date.split('T')[0];
      
      // Get win rate for this date
      const winRate = winRateMap.get(currentDate) || 0;

      // Calculate rolling average win and loss
      const startIdx = Math.max(0, i - windowSize + 1);
      const windowData = data.daily_pnl.slice(startIdx, i + 1);

      // Separate wins and losses
      const wins: number[] = [];
      const losses: number[] = [];

      windowData.forEach((day) => {
        const dailyPnl = day.value;
        const tradeCount = day.trade_count || 0;

        if (tradeCount > 0) {
          // Approximate: if day is profitable, all trades are wins (avg)
          // If day is losing, all trades are losses (avg)
          // This is an approximation since we don't have individual trade data
          const avgPerTrade = dailyPnl / tradeCount;
          
          if (dailyPnl > 0) {
            // Assume all trades this day were winners
            for (let j = 0; j < tradeCount; j++) {
              wins.push(avgPerTrade);
            }
          } else if (dailyPnl < 0) {
            // Assume all trades this day were losers
            for (let j = 0; j < tradeCount; j++) {
              losses.push(Math.abs(avgPerTrade));
            }
          }
        }
      });

      // Calculate averages
      const avgWin = wins.length > 0
        ? wins.reduce((sum, val) => sum + val, 0) / wins.length
        : 0;

      const avgLoss = losses.length > 0
        ? losses.reduce((sum, val) => sum + val, 0) / losses.length
        : 0;

      result.push({
        date: currentDate,
        winRate,
        avgWin,
        avgLoss: -avgLoss, // Keep as negative for display
      });
    }

    return result;
  }, [data]);

  // Format date labels
  const formatDateLabel = (dateStr: string): string => {
    try {
      const date = parseISO(dateStr);
      
      switch (timeRange) {
        case '7d':
          return format(date, 'MMM d');
        case '30d':
          return format(date, 'MMM d');
        case '90d':
          return format(date, 'MMM d');
        case '1y':
          return format(date, 'MMM yyyy');
        case 'all_time':
          return format(date, 'MMM yyyy');
        case 'custom':
          return format(date, 'MMM d');
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
            Failed to load averages data: {error.message}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <CardTitle>Win % - Avg Win - Avg Loss</CardTitle>
          <TooltipProvider>
            <UITooltip>
              <TooltipTrigger asChild>
                <Info className="h-4 w-4 text-muted-foreground cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="max-w-xs">
                  Track your win rate, average win, and average loss over time. Win rate is shown as a percentage (right axis), while average win/loss are in dollars (left axis).
                </p>
              </TooltipContent>
            </UITooltip>
          </TooltipProvider>
        </div>
      </CardHeader>
      <CardContent className="p-4">
        {isLoading ? (
          <Skeleton className="h-[250px] w-full" />
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-[250px] text-muted-foreground">
            No data available for the selected time range
          </div>
        ) : (
          <div className="h-[250px]">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart
                data={chartData}
                margin={{ top: 5, right: 25, left: 0, bottom: 10 }}
              >
                <CartesianGrid
                  strokeDasharray="3 3"
                  strokeOpacity={0.2}
                  vertical={false}
                />
                
                <XAxis
                  dataKey="date"
                  tickFormatter={formatDateLabel}
                  tick={{ fontSize: 10 }}
                  tickMargin={5}
                  angle={-45}
                  textAnchor="end"
                  height={50}
                />
                
                {/* Left Y-axis for dollar values (Avg Win, Avg Loss) */}
                <YAxis
                  yAxisId="left"
                  tickFormatter={(value) =>
                    `$${value.toLocaleString('en-US', {
                      minimumFractionDigits: 0,
                      maximumFractionDigits: 0,
                    })}`
                  }
                  tick={{ fontSize: 10 }}
                  tickMargin={5}
                  width={50}
                  domain={['auto', 'auto']}
                />
                
                {/* Right Y-axis for percentage (Win Rate) */}
                <YAxis
                  yAxisId="right"
                  orientation="right"
                  tickFormatter={(value) => `${value.toFixed(0)}%`}
                  tick={{ fontSize: 10 }}
                  tickMargin={5}
                  width={50}
                  domain={['auto', 'auto']}
                />
                
                <Tooltip content={<CustomTooltip />} />
                <Legend />
                
                {/* Win Rate Line (Green, Right Y-axis) */}
                <Line
                  yAxisId="right"
                  type="monotone"
                  dataKey="winRate"
                  stroke="#22c55e"
                  strokeWidth={2.5}
                  dot={false}
                  activeDot={{ r: 6 }}
                  name="Win %"
                />
                
                {/* Avg Win Line (Blue, Left Y-axis) */}
                <Line
                  yAxisId="left"
                  type="monotone"
                  dataKey="avgWin"
                  stroke="#3b82f6"
                  strokeWidth={2.5}
                  dot={false}
                  activeDot={{ r: 6 }}
                  name="Avg Win"
                />
                
                {/* Avg Loss Line (Red, Left Y-axis) */}
                <Line
                  yAxisId="left"
                  type="monotone"
                  dataKey="avgLoss"
                  stroke="#ef4444"
                  strokeWidth={2.5}
                  dot={false}
                  activeDot={{ r: 6 }}
                  name="Avg Loss"
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

