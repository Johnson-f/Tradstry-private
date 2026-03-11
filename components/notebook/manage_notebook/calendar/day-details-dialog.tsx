"use client";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Bell, Clock, Calendar, MapPin, FileText, TrendingUp, Building2 } from "lucide-react";
import { useCalendarEventsInRange } from "@/lib/hooks/use-notebook";
import { useEarningsForDate } from "@/lib/hooks/use-market-data-service";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import Image from "next/image";

interface DayDetailsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  date: Date | null;
}

export function DayDetailsDialog({ open, onOpenChange, date }: DayDetailsDialogProps) {
  // Get events for the selected date (full day range)
  const startOfDay = date ? new Date(date.setHours(0, 0, 0, 0)) : new Date();
  const endOfDay = date ? new Date(new Date(date).setHours(23, 59, 59, 999)) : new Date();

  const { data: events = [], isLoading: eventsLoading } = useCalendarEventsInRange(startOfDay, endOfDay, {
    isReminder: true,
  });

  // Get earnings for the selected date
  const { earnings, isLoading: earningsLoading } = useEarningsForDate(date, open && !!date);

  const reminders = events.filter((e) => e.is_reminder);

  const formatTime = (dateStr: string) => {
    return new Date(dateStr).toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
    });
  };

  const formattedDate = date
    ? date.toLocaleDateString("en-US", {
        weekday: "long",
        month: "long",
        day: "numeric",
        year: "numeric",
      })
    : "";

  const hasReminders = reminders.length > 0;
  const hasEarnings = earnings.length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Calendar className="h-5 w-5" />
            {formattedDate}
          </DialogTitle>
          <DialogDescription>
            {hasReminders && hasEarnings
              ? `${reminders.length} reminder${reminders.length !== 1 ? "s" : ""} and ${earnings.length} earnings report${earnings.length !== 1 ? "s" : ""}`
              : hasReminders
                ? `${reminders.length} reminder${reminders.length !== 1 ? "s" : ""} for this day`
                : hasEarnings
                  ? `${earnings.length} compan${earnings.length !== 1 ? "ies" : "y"} reporting earnings`
                  : "View your reminders and earnings reports for this day"}
          </DialogDescription>
        </DialogHeader>

        <Tabs defaultValue={hasEarnings ? "earnings" : "reminders"} className="w-full">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="reminders" className="flex items-center gap-1.5">
              <Bell className="h-3.5 w-3.5" />
              Reminders
              {hasReminders && (
                <span className="ml-1 text-xs bg-primary/10 px-1.5 py-0.5 rounded-full">
                  {reminders.length}
                </span>
              )}
            </TabsTrigger>
            <TabsTrigger value="earnings" className="flex items-center gap-1.5">
              <TrendingUp className="h-3.5 w-3.5" />
              Earnings
              {hasEarnings && (
                <span className="ml-1 text-xs bg-primary/10 px-1.5 py-0.5 rounded-full">
                  {earnings.length}
                </span>
              )}
            </TabsTrigger>
          </TabsList>

          {/* Reminders Tab */}
          <TabsContent value="reminders" className="mt-4">
            {eventsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : reminders.length === 0 ? (
              <div className="text-center py-8">
                <div className="h-16 w-16 rounded-full bg-muted/50 flex items-center justify-center mx-auto mb-4">
                  <Bell className="h-8 w-8 text-muted-foreground/50" />
                </div>
                <h3 className="text-sm font-medium mb-1">No reminders</h3>
                <p className="text-xs text-muted-foreground max-w-[250px] mx-auto">
                  You don&apos;t have any reminders scheduled for this day.
                </p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-3 pr-3">
                  {reminders.map((reminder) => (
                    <div
                      key={reminder.id}
                      className="p-4 rounded-lg border bg-card space-y-2"
                    >
                      <div className="flex items-start justify-between gap-2">
                        <h4 className="text-sm font-medium">{reminder.title}</h4>
                        {reminder.color && (
                          <div
                            className="w-3 h-3 rounded-full flex-shrink-0 mt-1"
                            style={{ backgroundColor: reminder.color }}
                          />
                        )}
                      </div>

                      {reminder.description && (
                        <p className="text-xs text-muted-foreground">{reminder.description}</p>
                      )}

                      <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                        <span className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          {formatTime(reminder.start_time)}
                          {reminder.end_time && ` - ${formatTime(reminder.end_time)}`}
                        </span>

                        {reminder.location && (
                          <span className="flex items-center gap-1">
                            <MapPin className="h-3 w-3" />
                            {reminder.location}
                          </span>
                        )}

                        {reminder.reminder_minutes && (
                          <span className="flex items-center gap-1">
                            <Bell className="h-3 w-3" />
                            {reminder.reminder_minutes} min before
                          </span>
                        )}
                      </div>

                      {reminder.reminder_sent && (
                        <div className="flex items-center gap-1 text-xs text-green-600">
                          <FileText className="h-3 w-3" />
                          Reminder sent
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </TabsContent>

          {/* Earnings Tab */}
          <TabsContent value="earnings" className="mt-4">
            {earningsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : earnings.length === 0 ? (
              <div className="text-center py-8">
                <div className="h-16 w-16 rounded-full bg-muted/50 flex items-center justify-center mx-auto mb-4">
                  <Building2 className="h-8 w-8 text-muted-foreground/50" />
                </div>
                <h3 className="text-sm font-medium mb-1">No earnings reports</h3>
                <p className="text-xs text-muted-foreground max-w-[250px] mx-auto">
                  No companies with market cap above $100M are reporting earnings on this day.
                </p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {earnings
                    .sort((a, b) => b.importance - a.importance)
                    .map((stock) => (
                      <EarningsStockCard key={stock.symbol} stock={stock} />
                    ))}
                </div>
              </ScrollArea>
            )}
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

interface EarningsStockCardProps {
  stock: {
    symbol: string;
    title: string;
    time: string;
    importance: number;
    emoji?: string | null;
    logo?: string | null;
  };
}

function EarningsStockCard({ stock }: EarningsStockCardProps) {
  const timeLabel = stock.time === "bmo" 
    ? "Before Market Open" 
    : stock.time === "amc" 
      ? "After Market Close" 
      : stock.time;

  return (
    <div className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors">
      {/* Logo */}
      <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-muted flex items-center justify-center overflow-hidden">
        {stock.logo ? (
          <Image
            src={stock.logo}
            alt={stock.title}
            width={40}
            height={40}
            className="object-contain"
            onError={(e) => {
              // Fallback to symbol initials on error
              e.currentTarget.style.display = 'none';
              e.currentTarget.parentElement?.classList.add('show-fallback');
            }}
          />
        ) : (
          <span className="text-xs font-semibold text-muted-foreground">
            {stock.symbol.slice(0, 2)}
          </span>
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-sm">{stock.symbol}</span>
          {stock.emoji && <span className="text-sm">{stock.emoji}</span>}
          <ImportanceBadge importance={stock.importance} />
        </div>
        <p className="text-xs text-muted-foreground truncate">{stock.title}</p>
      </div>

      {/* Time */}
      <div className="flex-shrink-0 text-right">
        <span className={cn(
          "text-xs px-2 py-1 rounded-full",
          stock.time === "bmo" && "bg-amber-500/10 text-amber-600",
          stock.time === "amc" && "bg-blue-500/10 text-blue-600",
          stock.time !== "bmo" && stock.time !== "amc" && "bg-muted text-muted-foreground"
        )}>
          {timeLabel}
        </span>
      </div>
    </div>
  );
}

function ImportanceBadge({ importance }: { importance: number }) {
  if (importance < 3) return null;
  
  return (
    <span className={cn(
      "text-[10px] px-1.5 py-0.5 rounded font-medium",
      importance >= 5 && "bg-red-500/10 text-red-600",
      importance >= 3 && importance < 5 && "bg-orange-500/10 text-orange-600"
    )}>
      {importance >= 5 ? "High" : "Med"}
    </span>
  );
}
