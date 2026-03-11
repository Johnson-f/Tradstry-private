'use client';

import { useState, useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { ChevronLeft, ChevronRight, Settings, Info } from 'lucide-react';
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
  eachWeekOfInterval,
} from 'date-fns';
import { cn } from '@/lib/utils';
import type { TimeRange } from '@/lib/types/analytics';

interface DailyData {
  date: string;
  pnl: number;
  tradeCount: number;
  winRate: number;
}

interface WeeklySummary {
  weekNumber: number;
  totalPnl: number;
  tradingDays: number;
}

export function ProfitCalendar() {
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
    
    if (!timeSeriesData?.daily_pnl || !timeSeriesData?.rolling_win_rate_50) {
      return map;
    }

    // Create a map of win rates by date
    const winRateMap = new Map<string, number>();
    timeSeriesData.rolling_win_rate_50.forEach((point) => {
      winRateMap.set(point.date, point.value);
    });

    // Combine daily P&L with win rates
    timeSeriesData.daily_pnl.forEach((point) => {
      const winRate = winRateMap.get(point.date) || 0;
      map.set(point.date, {
        date: point.date,
        pnl: point.value,
        tradeCount: point.trade_count,
        winRate: winRate,
      });
    });

    return map;
  }, [timeSeriesData]);

  // Filter daily data for current month
  const monthDailyData = useMemo(() => {
    const monthDays = eachDayOfInterval({ start: monthStart, end: monthEnd });
    return monthDays
      .map((day) => {
        const dateStr = format(day, 'yyyy-MM-dd');
        return dailyDataMap.get(dateStr);
      })
      .filter((data): data is DailyData => data !== undefined);
  }, [monthStart, monthEnd, dailyDataMap]);

  // Calculate monthly stats
  const monthlyStats = useMemo(() => {
    const totalPnl = monthDailyData.reduce((sum, day) => sum + day.pnl, 0);
    const tradingDays = monthDailyData.length;
    return { totalPnl, tradingDays };
  }, [monthDailyData]);

  // Calculate weekly summaries
  const weeklySummaries = useMemo(() => {
    const weeks = eachWeekOfInterval(
      { start: monthStart, end: monthEnd },
      { weekStartsOn: 0 }
    );

    return weeks.map((weekStart, index) => {
      const weekEnd = endOfWeek(weekStart, { weekStartsOn: 0 });
      const weekDays = eachDayOfInterval({
        start: Math.max(weekStart.getTime(), monthStart.getTime()),
        end: Math.min(weekEnd.getTime(), monthEnd.getTime()),
      });

      const weekData = weekDays
        .map((day) => dailyDataMap.get(format(day, 'yyyy-MM-dd')))
        .filter((data): data is DailyData => data !== undefined);

      return {
        weekNumber: index + 1,
        totalPnl: weekData.reduce((sum, day) => sum + day.pnl, 0),
        tradingDays: weekData.length,
      };
    });
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

  const handleThisMonth = () => {
    setCurrentMonth(new Date());
  };

  const formatCurrency = (value: number): string => {
    const absValue = Math.abs(value);
    const sign = value >= 0 ? '' : '-';
    
    if (absValue >= 1000) {
      return `${sign}$${(absValue / 1000).toFixed(absValue >= 100000 ? 0 : 1)}K`;
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
    if (pnl === undefined || pnl === 0) return 'bg-blue-50 dark:bg-blue-950/20 border-blue-200 dark:border-blue-800';
    if (pnl > 0) return 'bg-green-50 dark:bg-green-950/20 border-green-200 dark:border-green-800';
    return 'bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-800';
  };

  const formatWinRate = (winRate: number): string => {
    return `${winRate.toFixed(1)}%`;
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
    <div className="space-y-4">
      {/* Header with Navigation and Monthly Stats */}
      <Card>
        <CardHeader>
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
              <Button variant="outline" size="sm" onClick={handleThisMonth}>
                This month
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
                  {monthlyStats.tradingDays} days
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
      </Card>

      <div className="flex gap-4">
        {/* Calendar Grid */}
        <Card className="flex-1">
          <CardContent className="pt-6">
            {isLoading ? (
              <div className="grid grid-cols-7 gap-1">
                {Array.from({ length: 35 }).map((_, i) => (
                  <Skeleton key={i} className="h-24 w-full" />
                ))}
              </div>
            ) : (
              <>
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

                    return (
                      <div
                        key={dateStr}
                        className={cn(
                          "min-h-24 p-2 rounded-md border flex flex-col relative",
                          getCellColor(dayData?.pnl),
                          !isCurrentMonth && "opacity-40",
                          isCurrentDay && isCurrentMonth && "ring-2 ring-purple-500 dark:ring-purple-400"
                        )}
                      >
                        {/* Date Number */}
                        <div className="absolute top-1 right-1 text-xs font-medium text-muted-foreground">
                          {format(day, 'd')}
                        </div>

                        {/* Trading Data */}
                        {dayData ? (
                          <div className="flex-1 flex flex-col justify-center items-center gap-0.5 pt-2">
                            <div className={cn(
                              "text-sm font-semibold",
                              dayData.pnl >= 0
                                ? "text-green-700 dark:text-green-400"
                                : "text-red-700 dark:text-red-400"
                            )}>
                              {formatCurrency(dayData.pnl)}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {dayData.tradeCount} {dayData.tradeCount === 1 ? 'trade' : 'trades'}
                            </div>
                            {dayData.winRate > 0 && (
                              <div className="text-xs text-muted-foreground">
                                {formatWinRate(dayData.winRate)}
                              </div>
                            )}
                          </div>
                        ) : isCurrentMonth ? (
                          <div className="flex-1 flex items-center justify-center pt-2">
                            <span className="text-xs text-muted-foreground">No trades</span>
                          </div>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </CardContent>
        </Card>

        {/* Weekly Summaries Sidebar */}
        <Card className="w-48">
          <CardHeader>
            <CardTitle className="text-sm">Weekly Summary</CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <div className="space-y-2">
                {Array.from({ length: 5 }).map((_, i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : (
              <div className="space-y-2">
                {weeklySummaries.map((week) => (
                  <div
                    key={week.weekNumber}
                    className="flex flex-col gap-1 p-2 rounded-md border bg-muted/50"
                  >
                    <div className="text-xs font-medium text-muted-foreground">
                      Week {week.weekNumber}
                    </div>
                    <div className={cn(
                      "text-sm font-semibold",
                      week.totalPnl >= 0
                        ? "text-green-600 dark:text-green-400"
                        : "text-muted-foreground"
                    )}>
                      {formatCurrency(week.totalPnl)}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {week.tradingDays} {week.tradingDays === 1 ? 'day' : 'days'}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}