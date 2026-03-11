'use client';

import React, { useMemo, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { Badge } from '@/components/ui/badge';
import { Info } from 'lucide-react';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useAnalyticsTimeSeries } from '@/lib/hooks/use-analytics';
import {
  format,
  endOfWeek,
  eachWeekOfInterval,
  eachDayOfInterval,
  subMonths,
} from 'date-fns';
import { cn } from '@/lib/utils';

interface DayData {
  date: Date;
  tradeCount: number;
  pnl: number;
}

export function ProgressTracker() {
  const [monthsToShow] = useState(2); // Show last 2 months
  const { data, isLoading, error } = useAnalyticsTimeSeries({
    time_range: 'all_time',
  });

  // Calculate date range (last N months)
  const dateRange = useMemo(() => {
    const end = new Date();
    const start = subMonths(end, monthsToShow);
    return { start, end };
  }, [monthsToShow]);

  // Create a map of date to trade activity
  const activityMap = useMemo(() => {
    const map = new Map<string, { tradeCount: number; pnl: number }>();
    
    if (data?.daily_pnl) {
      data.daily_pnl.forEach((point) => {
        // Normalize date format (handle both YYYY-MM-DD and ISO strings)
        const dateStr = point.date.split('T')[0];
        map.set(dateStr, {
          tradeCount: point.trade_count || 0,
          pnl: point.value || 0,
        });
      });
    }
    
    return map;
  }, [data]);

  // Get all weeks in the date range
  const weeks = useMemo(() => {
    const weekStarts = eachWeekOfInterval(dateRange, { weekStartsOn: 1 }); // Start week on Monday
    return weekStarts;
  }, [dateRange]);

  // Get all days in the date range
  const allDays = useMemo(() => {
    return eachDayOfInterval(dateRange);
  }, [dateRange]);

  // Organize days by week
  const weeksData = useMemo(() => {
    return weeks.map((weekStart) => {
      const weekEnd = endOfWeek(weekStart, { weekStartsOn: 1 });
      const weekDays: DayData[] = [];
      
      // Get all days in this week
      const weekInterval = eachDayOfInterval({ start: weekStart, end: weekEnd });
      
      weekInterval.forEach((day) => {
        // Only include days within our date range
        if (day >= dateRange.start && day <= dateRange.end) {
          const dateStr = format(day, 'yyyy-MM-dd');
          const activity = activityMap.get(dateStr);
          weekDays.push({
            date: day,
            tradeCount: activity?.tradeCount || 0,
            pnl: activity?.pnl || 0,
          });
        } else {
          // Empty day outside range
          weekDays.push({
            date: day,
            tradeCount: 0,
            pnl: 0,
          });
        }
      });
      
      return {
        weekStart,
        days: weekDays,
      };
    });
  }, [weeks, dateRange, activityMap]);

  // Get intensity color based on trade count
  const getIntensityColor = (tradeCount: number): string => {
    if (tradeCount === 0) return 'bg-gray-100 dark:bg-gray-800';
    if (tradeCount === 1) return 'bg-blue-200 dark:bg-blue-900';
    if (tradeCount >= 2 && tradeCount <= 3) return 'bg-blue-400 dark:bg-blue-700';
    if (tradeCount >= 4 && tradeCount <= 5) return 'bg-blue-600 dark:bg-blue-600';
    return 'bg-blue-800 dark:bg-blue-500'; // 6+ trades
  };


  // Calculate stats
  const stats = useMemo(() => {
    const totalDays = allDays.length;
    const daysWithTrades = allDays.filter((day) => {
      const dateStr = format(day, 'yyyy-MM-dd');
      return (activityMap.get(dateStr)?.tradeCount || 0) > 0;
    }).length;
    const totalTrades = Array.from(activityMap.values()).reduce(
      (sum, activity) => sum + activity.tradeCount,
      0
    );
    const avgTradesPerDay = daysWithTrades > 0 ? totalTrades / daysWithTrades : 0;

    return {
      totalDays,
      daysWithTrades,
      totalTrades,
      avgTradesPerDay,
      activityRate: (daysWithTrades / totalDays) * 100,
    };
  }, [allDays, activityMap]);

  // Get month labels for the top
  const monthLabels = useMemo(() => {
    const labels: Array<{ month: string; weekIndex: number }> = [];
    const seenMonths = new Set<string>();
    
    weeks.forEach((weekStart, index) => {
      const month = format(weekStart, 'MMM');
      if (!seenMonths.has(month)) {
        seenMonths.add(month);
        labels.push({ month, weekIndex: index });
      }
    });
    
    return labels;
  }, [weeks]);

  if (error) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-destructive">
            Failed to load progress data: {error.message}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="flex flex-col h-full">
      <CardHeader className="flex-shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CardTitle>Progress Tracker</CardTitle>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                </TooltipTrigger>
                <TooltipContent>
                  <p className="max-w-xs">
                    Track your daily trading activity. Darker colors indicate more trades on that day.
                  </p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
          <Badge variant="secondary" className="text-xs">
            BETA
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-4 flex flex-col flex-1 min-h-0">
        {isLoading ? (
          <Skeleton className="h-full w-full" />
        ) : (
          <div className="flex flex-col h-full">
            {/* Heatmap - Fills all available space */}
            <div className="flex-1 overflow-x-auto overflow-y-auto min-h-0">
              <div className="h-full flex flex-col">
                {/* Month Labels */}
                <div className="flex mb-1 ml-10 relative flex-shrink-0">
                  {monthLabels.map(({ month, weekIndex }) => {
                    // Calculate approximate position (each week is ~11px wide)
                    const leftPosition = weekIndex * 11;
                    return (
                      <div
                        key={`${month}-${weekIndex}`}
                        className="text-[10px] text-muted-foreground absolute"
                        style={{
                          left: `${leftPosition}px`,
                        }}
                      >
                        {month}
                      </div>
                    );
                  })}
                </div>

                {/* Main heatmap container - Fills remaining space */}
                <div className="flex-1 flex gap-0.5 items-stretch min-h-0">
                  {/* Day of Week Labels */}
                  <div className="flex flex-col gap-0.5 mr-1.5 flex-shrink-0 justify-between">
                    {['Mon', '', 'Wed', '', 'Fri', '', 'Sun'].map((day, idx) => (
                      <div 
                        key={idx} 
                        className={cn(
                          "text-[10px] text-muted-foreground flex items-center",
                          day ? "h-full" : "h-[2px]"
                        )}
                      >
                        {day}
                      </div>
                    ))}
                  </div>

                  {/* Calendar Grid - Expands to fill height */}
                  <div className="flex-1 flex gap-0.5 h-full">
                    {weeksData.map((week, weekIndex) => (
                      <div key={weekIndex} className="flex-1 flex flex-col gap-0.5 h-full">
                        {week.days.map((dayData, dayIndex) => {
                          const isInRange = 
                            dayData.date >= dateRange.start && 
                            dayData.date <= dateRange.end;
                          
                          if (!isInRange) {
                            return (
                              <div
                                key={`${weekIndex}-${dayIndex}`}
                                className="flex-1 rounded-sm"
                              />
                            );
                          }

                          return (
                            <TooltipProvider key={`${weekIndex}-${dayIndex}`}>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <div
                                    className={cn(
                                      'flex-1 rounded-sm cursor-pointer hover:ring-2 hover:ring-blue-500 transition-all',
                                      getIntensityColor(dayData.tradeCount)
                                    )}
                                  />
                                </TooltipTrigger>
                                <TooltipContent>
                                  <div className="space-y-1">
                                    <p className="font-semibold">
                                      {format(dayData.date, 'MMM d, yyyy')}
                                    </p>
                                    <p>
                                      {dayData.tradeCount === 0
                                        ? 'No trades'
                                        : `${dayData.tradeCount} trade${dayData.tradeCount !== 1 ? 's' : ''}`}
                                    </p>
                                    {dayData.tradeCount > 0 && (
                                      <p className="text-sm text-muted-foreground">
                                        P&L: {dayData.pnl >= 0 ? '+' : ''}
                                        ${dayData.pnl.toFixed(2)}
                                      </p>
                                    )}
                                  </div>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          );
                        })}
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>

            {/* Stats Summary and Legend - Compact footer */}
            <div className="flex items-center justify-between mt-2 pt-2 border-t flex-shrink-0">
              {/* Stats Summary - Compact */}
              <div className="grid grid-cols-4 gap-3 text-[11px]">
                <div>
                  <p className="text-muted-foreground text-[10px]">Trades</p>
                  <p className="text-xs font-semibold">{stats.totalTrades}</p>
                </div>
                <div>
                  <p className="text-muted-foreground text-[10px]">Active Days</p>
                  <p className="text-xs font-semibold">{stats.daysWithTrades}</p>
                </div>
                <div>
                  <p className="text-muted-foreground text-[10px]">Avg/Day</p>
                  <p className="text-xs font-semibold">{stats.avgTradesPerDay.toFixed(1)}</p>
                </div>
                <div>
                  <p className="text-muted-foreground text-[10px]">Rate</p>
                  <p className="text-xs font-semibold">{stats.activityRate.toFixed(1)}%</p>
                </div>
              </div>

              {/* Legend */}
              <div className="flex items-center gap-2 text-[10px]">
                <span className="text-muted-foreground">Less</span>
                <div className="flex gap-0.5">
                  <div className="w-2.5 h-2.5 rounded-sm bg-gray-100 dark:bg-gray-800" />
                  <div className="w-2.5 h-2.5 rounded-sm bg-blue-200 dark:bg-blue-900" />
                  <div className="w-2.5 h-2.5 rounded-sm bg-blue-400 dark:bg-blue-700" />
                  <div className="w-2.5 h-2.5 rounded-sm bg-blue-600 dark:bg-blue-600" />
                  <div className="w-2.5 h-2.5 rounded-sm bg-blue-800 dark:bg-blue-500" />
                </div>
                <span className="text-muted-foreground">More</span>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}