'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useAnalyticsPerformance } from '@/lib/hooks/use-analytics';
import type { TimeRange } from '@/lib/types/analytics';
import { MarketPerformance } from './market-performance';

type TimeRangeOption = '7d' | '30d' | '90d' | 'ytd' | '1y' | 'all_time';

const TIME_RANGE_OPTIONS: Array<{ value: TimeRangeOption; label: string }> = [
  { value: '7d', label: 'Last 7 Days' },
  { value: '30d', label: 'Last 30 Days' },
  { value: '90d', label: 'Last 90 Days' },
  { value: 'ytd', label: 'Year to Date' },
  { value: '1y', label: 'Last Year' },
  { value: 'all_time', label: 'All Time' },
];

export function DurationPlaybookPerformance() {
  const [selectedTimeRange, setSelectedTimeRange] = useState<TimeRangeOption>('30d');
  
  const { data, isLoading, error } = useAnalyticsPerformance({
    time_range: selectedTimeRange as TimeRange,
  });

  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const formatPercentage = (value: number): string => {
    return `${value.toFixed(1)}%`;
  };

  const formatDays = (value: number): string => {
    return `${value.toFixed(1)} days`;
  };

  const getPerformanceColor = (value: number): string => {
    if (value > 0) return 'text-green-600 dark:text-green-400';
    if (value < 0) return 'text-red-600 dark:text-red-400';
    return 'text-muted-foreground';
  };

  const durationBuckets = data?.duration_performance?.duration_buckets || [];

  // Ensure all buckets are present, even if empty
  const allBuckets = [
    { bucket: '0-1 days', data: durationBuckets.find((b) => b.duration_bucket === '0-1 days') },
    { bucket: '1-7 days', data: durationBuckets.find((b) => b.duration_bucket === '1-7 days') },
    { bucket: '1-4 weeks', data: durationBuckets.find((b) => b.duration_bucket === '1-4 weeks') },
    { bucket: '1-3 months', data: durationBuckets.find((b) => b.duration_bucket === '1-3 months') },
    { bucket: '3+ months', data: durationBuckets.find((b) => b.duration_bucket === '3+ months') },
  ];

  if (error) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-red-500">
            Failed to load duration performance data: {error.message}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Market Performance Chart */}
      <MarketPerformance />

      {/* Header with Time Range Filter */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">Performance by Duration</h3>
          <p className="text-sm text-muted-foreground">
            Analyze trading performance across different hold periods
          </p>
        </div>
        <Select 
          value={selectedTimeRange} 
          onValueChange={(value) => {
            if (TIME_RANGE_OPTIONS.some(opt => opt.value === value)) {
              setSelectedTimeRange(value as TimeRangeOption);
            }
          }}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TIME_RANGE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Duration Performance Breakdown</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-2">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="font-semibold">Duration</TableHead>
                  <TableHead className="text-right">Trades</TableHead>
                  <TableHead className="text-right">Win Rate</TableHead>
                  <TableHead className="text-right">Total P&L</TableHead>
                  <TableHead className="text-right">Avg P&L</TableHead>
                  <TableHead className="text-right">Avg Hold Time</TableHead>
                  <TableHead className="text-right">Best Trade</TableHead>
                  <TableHead className="text-right">Worst Trade</TableHead>
                  <TableHead className="text-right">Profit Factor</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {allBuckets.map(({ bucket, data: bucketData }) => (
                  <TableRow key={bucket}>
                    <TableCell className="font-medium">{bucket}</TableCell>
                    <TableCell className="text-right">
                      {bucketData ? bucketData.trade_count : '-'}
                    </TableCell>
                    <TableCell className="text-right">
                      {bucketData ? (
                        <Badge 
                          variant={bucketData.win_rate >= 50 ? 'default' : 'secondary'}
                          className="font-medium"
                        >
                          {formatPercentage(bucketData.win_rate)}
                        </Badge>
                      ) : (
                        '-'
                      )}
                    </TableCell>
                    <TableCell className={`text-right font-medium ${bucketData ? getPerformanceColor(bucketData.total_pnl) : ''}`}>
                      {bucketData ? formatCurrency(bucketData.total_pnl) : '-'}
                    </TableCell>
                    <TableCell className={`text-right ${bucketData ? getPerformanceColor(bucketData.avg_pnl) : ''}`}>
                      {bucketData ? formatCurrency(bucketData.avg_pnl) : '-'}
                    </TableCell>
                    <TableCell className="text-right text-muted-foreground">
                      {bucketData ? formatDays(bucketData.avg_hold_time_days) : '-'}
                    </TableCell>
                    <TableCell className="text-right text-green-600 dark:text-green-400 font-medium">
                      {bucketData ? formatCurrency(bucketData.best_trade) : '-'}
                    </TableCell>
                    <TableCell className="text-right text-red-600 dark:text-red-400 font-medium">
                      {bucketData ? formatCurrency(bucketData.worst_trade) : '-'}
                    </TableCell>
                    <TableCell className="text-right">
                      {bucketData ? (
                        <Badge 
                          variant={bucketData.profit_factor >= 1 ? 'default' : 'destructive'}
                          className="font-medium"
                        >
                          {bucketData.profit_factor.toFixed(2)}
                        </Badge>
                      ) : (
                        '-'
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}