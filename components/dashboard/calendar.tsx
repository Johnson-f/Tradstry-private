'use client';

import { useState, useMemo } from 'react';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { ChevronLeft, ChevronRight, Settings, Info, Receipt } from 'lucide-react';
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics';
import {
  format,
  startOfMonth,
  endOfMonth,
  eachDayOfInterval,
  isToday,
  addMonths,
  subMonths,
  startOfWeek,
  endOfWeek,
} from 'date-fns';
import { cn } from '@/lib/utils';
import type { TimeRange } from '@/lib/types/analytics';

interface DailyData {
  date: string;
  pnl: number;
  tradeCount: number;
}

export function Calendar() {
  const [currentMonth, setCurrentMonth] = useState<Date>(new Date());

  // Fetch time series data for the current month
  const monthStart = startOfMonth(currentMonth);
  const monthEnd = endOfMonth(currentMonth);
  
  // Calculate time range that covers the current month
  const timeRange: TimeRange = useMemo(() => {
    const now = new Date();
    const monthDiff = 
      (currentMonth.getFullYear() - now.getFullYear()) * 12 + 
      (currentMonth.getMonth() - now.getMonth());
    
    if (monthDiff === 0) return '30d';
    if (monthDiff === -1) return '90d';
    return '1y';
  }, [currentMonth]);

  const { data: timeSeriesData, isLoading, error } = useAnalyticsTimeSeries({
    time_range: timeRange,
  });

  // Create a map of date to daily data
  const dailyDataMap = useMemo(() => {
    const map = new Map<string, DailyData>();
    
    if (!timeSeriesData?.daily_pnl) {
      return map;
    }

    // Map daily P&L data
    timeSeriesData.daily_pnl.forEach((point) => {
      // Normalize date format (handle both YYYY-MM-DD and ISO strings)
      const dateStr = point.date.split('T')[0];
      map.set(dateStr, {
        date: dateStr,
        pnl: point.value || 0,
        tradeCount: point.trade_count || 0,
      });
    });

    return map;
  }, [timeSeriesData]);

  // Calculate monthly stats
  const monthlyStats = useMemo(() => {
    const monthDays = eachDayOfInterval({ start: monthStart, end: monthEnd });
    const monthData: DailyData[] = [];
    
    monthDays.forEach((day) => {
      const dateStr = format(day, 'yyyy-MM-dd');
      const dayData = dailyDataMap.get(dateStr);
      if (dayData) {
        monthData.push(dayData);
      }
    });

    const totalPnl = monthData.reduce((sum, day) => sum + day.pnl, 0);
    const tradingDays = monthData.length;
    const totalTrades = monthData.reduce((sum, day) => sum + day.tradeCount, 0);

    return { totalPnl, tradingDays, totalTrades };
  }, [monthStart, monthEnd, dailyDataMap]);

  // Generate calendar grid
  const calendarDays = useMemo(() => {
    const monthStartDay = startOfWeek(monthStart, { weekStartsOn: 0 });
    const monthEndDay = endOfWeek(monthEnd, { weekStartsOn: 0 });
    return eachDayOfInterval({ start: monthStartDay, end: monthEndDay });
  }, [monthStart, monthEnd]);

  const handlePreviousMonth = () => {
    setCurrentMonth(subMonths(currentMonth, 1));
  };

  const handleNextMonth = () => {
    setCurrentMonth(addMonths(currentMonth, 1));
  };

  const formatCurrency = (value: number): string => {
    const absValue = Math.abs(value);
    const sign = value >= 0 ? '+' : '-';
    
    if (absValue >= 1000) {
      // Format as $2.9K, $4.7K, etc.
      const thousands = absValue / 1000;
      return `${sign}$${thousands.toFixed(thousands >= 10 ? 0 : 1)}K`;
    }
    return `${sign}$${absValue.toFixed(0)}`;
  };

  const formatCurrencyFull = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const getCellColor = (pnl: number | undefined): string => {
    if (pnl === undefined || pnl === 0) return '';
    if (pnl > 0) return 'bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800';
    return 'bg-red-50 dark:bg-red-950/30 border-red-200 dark:border-red-800';
  };

  if (error) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-red-500">
            Failed to load calendar data: {error.message}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        {/* Header with Navigation and Monthly Stats */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Button
              variant="ghost"
              size="icon"
              onClick={handlePreviousMonth}
              className="h-8 w-8"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <h2 className="text-xl font-semibold">
              {format(currentMonth, 'MMMM yyyy')}
            </h2>
            <Button
              variant="ghost"
              size="icon"
              onClick={handleNextMonth}
              className="h-8 w-8"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="sm">
              Button
            </Button>
          </div>
          <div className="flex items-center gap-4">
            <div className="text-right">
              <span className="text-sm text-muted-foreground">Monthly stats: </span>
              <span className={cn(
                "text-lg font-semibold",
                monthlyStats.totalPnl >= 0 
                  ? "text-green-600 dark:text-green-400" 
                  : "text-red-600 dark:text-red-400"
              )}>
                {formatCurrencyFull(monthlyStats.totalPnl)}
              </span>
              <span className="text-sm text-muted-foreground ml-2">
                {monthlyStats.tradingDays} {monthlyStats.tradingDays === 1 ? 'day' : 'days'}
              </span>
              <span className="text-sm text-muted-foreground ml-2">
                {monthlyStats.totalTrades} {monthlyStats.totalTrades === 1 ? 'trade' : 'trades'}
              </span>
            </div>
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <Settings className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <Info className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-4">
            <div className="grid grid-cols-7 gap-1">
              {Array.from({ length: 35 }).map((_, i) => (
                <Skeleton key={i} className="h-24 w-full" />
              ))}
            </div>
          </div>
        ) : (
          <div>
            {/* Calendar Grid */}
            <div className="w-full">
              {/* Day Headers */}
              <div className="grid grid-cols-7 gap-1 mb-1">
                {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map((day) => (
                  <div
                    key={day}
                    className="text-xs font-medium text-center text-muted-foreground py-1"
                  >
                    {day}
                  </div>
                ))}
              </div>

              {/* Calendar Grid */}
              <div className="grid grid-cols-7 gap-1">
                {calendarDays.map((day) => {
                  const dateStr = format(day, 'yyyy-MM-dd');
                  const dayData = dailyDataMap.get(dateStr);
                  const isCurrentDay = isToday(day);
                  const isCurrentMonth = day.getMonth() === currentMonth.getMonth();
                  const hasTrades = dayData && dayData.tradeCount > 0;

                  return (
                    <div
                      key={dateStr}
                      className={cn(
                        "min-h-[70px] p-2 rounded-md border flex flex-col relative",
                        hasTrades ? getCellColor(dayData?.pnl) : "bg-background border-border",
                        !isCurrentMonth && "opacity-40",
                        isCurrentDay && isCurrentMonth && "ring-2 ring-primary"
                      )}
                    >
                      {/* Date Number - Top Left */}
                      <div className="absolute top-2 left-2 text-xs font-medium text-muted-foreground">
                        {format(day, 'd')}
                      </div>

                      {/* Trading Icon - Top Right (only if has trades) */}
                      {hasTrades && (
                        <div className="absolute top-2 right-2">
                          <Receipt className="h-4 w-4 text-muted-foreground" />
                        </div>
                      )}

                      {/* Trading Data - Center */}
                      {hasTrades ? (
                        <div className="flex-1 flex flex-col justify-center items-center gap-1 mt-4">
                          <div className={cn(
                            "text-base font-bold",
                            dayData.pnl >= 0
                              ? "text-green-600 dark:text-green-400"
                              : "text-red-600 dark:text-red-400"
                          )}>
                            {formatCurrency(dayData.pnl)}
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {dayData.tradeCount} {dayData.tradeCount === 1 ? 'trade' : 'trades'}
                          </div>
                        </div>
                      ) : isCurrentMonth ? (
                        <div className="flex-1 flex items-center justify-center mt-4">
                          <span className="text-xs text-muted-foreground opacity-50">No trades</span>
                        </div>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

