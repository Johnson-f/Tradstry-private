"use client";

import React, { useState, useMemo } from 'react';
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { format, subDays } from "date-fns";
import { ChevronDown, PanelLeft } from "lucide-react";
import { useUserProfile } from "@/hooks/use-user-profile";
import { getTimeBasedGreeting } from "@/lib/utils/greetings";
import { BasicCards } from "@/components/dashboard/basic-cards";
import { ProgressTracker } from "@/components/dashboard/progress-tracker";
import { Averages } from "@/components/dashboard/averages";
import { Calendar as TradingCalendar } from "@/components/dashboard/calendar";
import { Trades } from "@/components/dashboard/trades";
import { AppPageHeader } from '@/components/app-page-header';

type TimeRange = '7d' | '30d' | '90d' | '1y' | 'all_time' | 'custom';

export default function DashboardPage() {
  const [showDatePicker, setShowDatePicker] = useState(false);
  const [timeRange, setTimeRange] = useState<TimeRange>('30d');
  const [customDateRange, setCustomDateRange] = useState<{ start: Date; end: Date } | null>(null);

  // Memoize date range calculation to prevent unnecessary re-renders
  const dateRange = useMemo(() => {
    // Use custom date range if timeRange is 'custom' and customDateRange is set
    if (timeRange === 'custom' && customDateRange) {
      return customDateRange;
    }

    const today = new Date();
    const daysMap: Record<Exclude<TimeRange, 'custom'>, number> = {
      '7d': 7,
      '30d': 30,
      '90d': 90,
      '1y': 365,
      'all_time': 365 * 5,
    };

    const days = daysMap[timeRange as Exclude<TimeRange, 'custom'>] || 30;
    return {
      start: subDays(today, days),
      end: today,
    };
  }, [timeRange, customDateRange]);

  const headerActions = (
    <div className="flex items-center gap-4">
      {/* Date Range Picker */}
      <Popover open={showDatePicker} onOpenChange={setShowDatePicker}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            className="w-[240px] justify-between text-left font-normal"
          >
            {format(dateRange.start, 'MMM d, yyyy')} - {format(dateRange.end, 'MMM d, yyyy')}
            <ChevronDown className="ml-2 h-4 w-4" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0" align="end">
          <Calendar
            mode="range"
            selected={{
              from: dateRange.start,
              to: dateRange.end
            }}
            onSelect={(range) => {
              if (range?.from && range?.to) {
                setCustomDateRange({
                  start: range.from,
                  end: range.to
                });
                setTimeRange('custom');
              }
            }}
          />
        </PopoverContent>
      </Popover>

      {/* Time Range Selector */}
      <Select 
        value={timeRange}
        onValueChange={(value) => {
          setTimeRange(value as TimeRange);
        }}
      >
        <SelectTrigger className="w-[140px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="7d">7 day</SelectItem>
          <SelectItem value="30d">30 day</SelectItem>
          <SelectItem value="90d">90 day</SelectItem>
          <SelectItem value="1y">1 year</SelectItem>
          <SelectItem value="all_time">All time</SelectItem>
          <SelectItem value="custom">Custom range</SelectItem>
        </SelectContent>
      </Select>
    </div>
  );

  return (
    <div className="h-screen flex flex-col bg-background">
      <AppPageHeader title="Dashboard" actions={headerActions} />
      
      {/* Main content - Scrollable area with shadcn ScrollArea */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-8 space-y-8">
            <DashboardGreeting />
            
            {/* Basic Analytics Cards */}
            <BasicCards 
              timeRange={timeRange}
              customStartDate={timeRange === 'custom' ? dateRange.start : undefined}
              customEndDate={timeRange === 'custom' ? dateRange.end : undefined}
            />
            
            {/* Progress Tracker & Averages Chart */}
            <div className="grid gap-6 md:grid-cols-2">
              <ProgressTracker />
              <Averages 
                timeRange={timeRange}
                customStartDate={timeRange === 'custom' ? dateRange.start : undefined}
                customEndDate={timeRange === 'custom' ? dateRange.end : undefined}
              />
            </div>

            {/* Trades & Calendar - Side by side */}
            <div className="grid gap-6 md:grid-cols-2">
              <Trades />
              <TradingCalendar />
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}

const DashboardGreeting = React.memo(function DashboardGreeting() {
  const { firstName, loading, email } = useUserProfile();
  const greetingData = useMemo(() => getTimeBasedGreeting(), []);
  
  // Memoize display name calculation
  const displayName = useMemo(() => {
    return firstName || (email ? email.split('@')[0] : '');
  }, [firstName, email]);

  // Memoize whether to show profile prompt
  const showProfilePrompt = useMemo(() => {
    return !firstName && !!email;
  }, [firstName, email]);
  
  if (loading) {
    return <DashboardGreetingSkeleton />;
  }

  const { timeGreeting, casualGreeting, tradingReminder, marketStatus } = greetingData;

  return (
    <div className="space-y-3">
      <div>
        <h2 className="text-2xl font-semibold">
          {timeGreeting}{displayName ? `, ${displayName}` : ''}!
        </h2>
        {showProfilePrompt && (
          <p className="text-sm text-muted-foreground mt-1">
            Welcome back! Update your profile to personalize your experience.
          </p>
        )}
      </div>
      
      <div className="flex flex-col gap-2">
        <p className="text-base text-foreground/80">
          {casualGreeting}
        </p>
        <p className="text-sm text-muted-foreground italic">
          <span aria-hidden="true">💡</span> {tradingReminder}
        </p>
        <p className="text-xs text-muted-foreground/80 font-medium">
          <span aria-hidden="true">📊</span> {marketStatus}
        </p>
      </div>
    </div>
  );
});

// Extract skeleton to separate component for better organization
function DashboardGreetingSkeleton() {
  return (
    <div className="space-y-2" role="status" aria-label="Loading greeting">
      <div className="h-8 w-64 bg-muted rounded animate-pulse" />
      <div className="h-4 w-96 bg-muted rounded animate-pulse" />
      <div className="h-4 w-80 bg-muted rounded animate-pulse" />
      <span className="sr-only">Loading...</span>
    </div>
  );
}