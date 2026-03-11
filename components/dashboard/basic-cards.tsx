'use client';

import React, { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  ResponsiveContainer,
  Tooltip,
} from 'recharts';
import { useAnalyticsCore, useAnalyticsPerformance } from '@/lib/hooks/use-analytics';
import type { TimeRange } from '@/lib/types/analytics';

interface BasicCardsProps {
  timeRange?: '7d' | '30d' | '90d' | '1y' | 'all_time' | 'custom';
  customStartDate?: Date;
  customEndDate?: Date;
}

export function BasicCards({ 
  timeRange = '30d',
  customStartDate,
  customEndDate 
}: BasicCardsProps) {
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

  const { data: coreData, isLoading: coreLoading } = useAnalyticsCore({
    time_range: analyticsTimeRange,
  });
  
  const { data: performanceData, isLoading: performanceLoading } = useAnalyticsPerformance({
    time_range: analyticsTimeRange,
  });

  const isLoading = coreLoading || performanceLoading;

  // Format currency
  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  // Prepare bar chart data for avg win/loss
  const winLossData = useMemo(() => {
    if (!coreData) return [];
    
    return [
      {
        name: 'Avg Win',
        value: coreData.average_win || 0,
        fill: '#22c55e', // green
      },
      {
        name: 'Avg Loss',
        value: Math.abs(coreData.average_loss || 0),
        fill: '#ef4444', // red
      },
    ];
  }, [coreData]);

  // Calculate max value for chart scale
  const maxChartValue = useMemo(() => {
    if (!coreData) return 100;
    const max = Math.max(
      Math.abs(coreData.average_win || 0),
      Math.abs(coreData.average_loss || 0)
    );
    return max > 0 ? max * 1.2 : 100; // Add 20% padding
  }, [coreData]);

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0];
      return (
        <div className="bg-background border border-border rounded-lg p-2 shadow-lg">
          <p className="text-sm font-medium">{data.payload.name}</p>
          <p className={`text-sm font-semibold ${
            data.payload.name === 'Avg Win' ? 'text-green-600' : 'text-red-600'
          }`}>
            {formatCurrency(data.value)}
          </p>
        </div>
      );
    }
    return null;
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Card key={i}>
            <CardHeader>
              <Skeleton className="h-5 w-32" />
            </CardHeader>
            <CardContent>
              <Skeleton className="h-20 w-full" />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {/* Net P&L Card */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            Net P&L
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            <p className={`text-2xl font-bold ${
              (coreData?.net_profit_loss || 0) >= 0
                ? 'text-green-600 dark:text-green-400'
                : 'text-red-600 dark:text-red-400'
            }`}>
              {formatCurrency(coreData?.net_profit_loss || 0)}
            </p>
            <p className="text-xs text-muted-foreground">
              Total: {formatCurrency(coreData?.total_pnl || 0)}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Trade Expectancy Card */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            Trade Expectancy
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            <p className={`text-2xl font-bold ${
              (performanceData?.performance_metrics?.trade_expectancy || 0) >= 0
                ? 'text-green-600 dark:text-green-400'
                : 'text-red-600 dark:text-red-400'
            }`}>
              {formatCurrency(performanceData?.performance_metrics?.trade_expectancy || 0)}
            </p>
            <p className="text-xs text-muted-foreground">
              Expected value per trade
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Win Rate Card */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            Win Rate
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-1">
            <p className="text-2xl font-bold">
              {(coreData?.win_rate || 0).toFixed(1)}%
            </p>
            <p className="text-xs text-muted-foreground">
              {coreData?.winning_trades || 0} wins / {coreData?.total_trades || 0} trades
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Avg Win/Loss Card with Bar Chart */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            Avg Win / Loss
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div>
                <p className="text-xs text-muted-foreground">Avg Win</p>
                <p className="text-green-600 dark:text-green-400 font-semibold">
                  {formatCurrency(coreData?.average_win || 0)}
                </p>
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Avg Loss</p>
                <p className="text-red-600 dark:text-red-400 font-semibold">
                  {formatCurrency(Math.abs(coreData?.average_loss || 0))}
                </p>
              </div>
            </div>
            {winLossData.length > 0 && (
              <div className="h-24 w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={winLossData}
                    layout="vertical"
                    margin={{ top: 5, right: 5, left: 0, bottom: 5 }}
                  >
                    <XAxis
                      type="number"
                      domain={[0, maxChartValue]}
                      hide
                    />
                    <YAxis
                      type="category"
                      dataKey="name"
                      width={60}
                      tick={{ fontSize: 10 }}
                      axisLine={false}
                      tickLine={false}
                    />
                    <Tooltip content={<CustomTooltip />} />
                    <Bar
                      dataKey="value"
                      radius={[0, 4, 4, 0]}
                    />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

