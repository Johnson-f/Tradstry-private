"use client";

import { useState, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, FileText, Calendar } from "lucide-react";
import { cn } from "@/lib/utils";
import { useNotebookNotes } from "@/lib/hooks/use-notebook";
import { useEarningsCalendar } from "@/lib/hooks/use-market-data-service";
import type { NotebookNote } from "@/lib/types/notebook";
import { CalendarActions } from "./connect";
import { RemindersDialog } from "./reminders-dialog";
import { DayDetailsDialog } from "./day-details-dialog";
import { X } from "lucide-react";

interface CalendarViewProps {
  onNoteSelect?: (noteId: string) => void;
  onClose?: () => void;
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December"
];

export function CalendarView({ onNoteSelect, onClose }: CalendarViewProps) {
  const { data: notes = [] } = useNotebookNotes();
  const [currentDate, setCurrentDate] = useState(new Date());
  const [selectedDate, setSelectedDate] = useState<Date | null>(null);
  const [remindersOpen, setRemindersOpen] = useState(false);
  const [dayDetailsOpen, setDayDetailsOpen] = useState(false);
  const [dayDetailsDate, setDayDetailsDate] = useState<Date | null>(null);

  const year = currentDate.getFullYear();
  const month = currentDate.getMonth();

  // Get earnings calendar for the current month
  const firstDayOfMonth = `${year}-${String(month + 1).padStart(2, '0')}-01`;
  const lastDayOfMonth = `${year}-${String(month + 1).padStart(2, '0')}-${new Date(year, month + 1, 0).getDate()}`;
  
  const { calendar: earningsCalendar } = useEarningsCalendar({
    fromDate: firstDayOfMonth,
    toDate: lastDayOfMonth,
  });

  // Group notes by date (created_at)
  const notesByDate = useMemo(() => {
    const map = new Map<string, NotebookNote[]>();
    notes.forEach((note) => {
      const dateKey = new Date(note.created_at).toDateString();
      if (!map.has(dateKey)) {
        map.set(dateKey, []);
      }
      map.get(dateKey)!.push(note);
    });
    return map;
  }, [notes]);

  // Get calendar days for current month
  const calendarDays = useMemo(() => {
    const firstDay = new Date(year, month, 1);
    const lastDay = new Date(year, month + 1, 0);
    const startPadding = firstDay.getDay();
    const totalDays = lastDay.getDate();

    const days: (Date | null)[] = [];

    // Add padding for days before the first of the month
    for (let i = 0; i < startPadding; i++) {
      days.push(null);
    }

    // Add all days of the month
    for (let i = 1; i <= totalDays; i++) {
      days.push(new Date(year, month, i));
    }

    return days;
  }, [year, month]);

  const goToPreviousMonth = () => {
    setCurrentDate(new Date(year, month - 1, 1));
    setSelectedDate(null);
  };

  const goToNextMonth = () => {
    setCurrentDate(new Date(year, month + 1, 1));
    setSelectedDate(null);
  };

  const goToToday = () => {
    setCurrentDate(new Date());
    setSelectedDate(new Date());
  };

  const handleDateClick = (date: Date) => {
    setSelectedDate(date);
  };

  const handleDateDoubleClick = (date: Date) => {
    setDayDetailsDate(date);
    setDayDetailsOpen(true);
  };

  const getNotesForDate = (date: Date): NotebookNote[] => {
    return notesByDate.get(date.toDateString()) || [];
  };

  const getEarningsCountForDate = (date: Date): number => {
    const dateStr = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
    return earningsCalendar?.earnings?.[dateStr]?.stocks?.length ?? 0;
  };

  const isToday = (date: Date) => {
    const today = new Date();
    return date.toDateString() === today.toDateString();
  };

  const isSelected = (date: Date) => {
    return selectedDate?.toDateString() === date.toDateString();
  };

  const selectedDateNotes = selectedDate ? getNotesForDate(selectedDate) : [];

  return (
    <div className="flex flex-col h-full">
      {/* Calendar Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b shrink-0">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" onClick={goToPreviousMonth}>
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <h3 className="text-sm font-medium min-w-[140px] text-center">
            {MONTHS[month]} {year}
          </h3>
          <Button variant="ghost" size="icon" onClick={goToNextMonth}>
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="sm" onClick={goToToday}>
            Today
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <CalendarActions onOpenReminders={() => setRemindersOpen(true)} />
          {onClose && (
            <Button variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Reminders Dialog */}
      <RemindersDialog open={remindersOpen} onOpenChange={setRemindersOpen} />

      {/* Day Details Dialog */}
      <DayDetailsDialog
        open={dayDetailsOpen}
        onOpenChange={setDayDetailsOpen}
        date={dayDetailsDate}
      />

      {/* Main Content - Calendar + Sidebar */}
      <div className="flex-1 flex min-h-0">
        {/* Calendar Grid */}
        <div className="flex-1 flex flex-col p-3 min-h-0">
          {/* Day headers */}
          <div className="grid grid-cols-7 gap-1 mb-1">
            {DAYS.map((day) => (
              <div
                key={day}
                className="text-center text-[11px] font-medium text-muted-foreground py-1"
              >
                {day}
              </div>
            ))}
          </div>

          {/* Calendar days - 6 rows to fit all possible month layouts */}
          <div className="flex-1 grid grid-cols-7 grid-rows-6 gap-1">
            {calendarDays.map((date, index) => {
              if (!date) {
                return <div key={`empty-${index}`} className="min-h-[32px]" />;
              }

              const dayNotes = getNotesForDate(date);
              const hasNotes = dayNotes.length > 0;
              const earningsCount = getEarningsCountForDate(date);
              const hasEarnings = earningsCount > 0;

              return (
                <button
                  key={date.toISOString()}
                  onClick={() => handleDateClick(date)}
                  onDoubleClick={() => handleDateDoubleClick(date)}
                  className={cn(
                    "min-h-[32px] flex flex-col items-center justify-center rounded-md text-sm transition-colors relative",
                    "hover:bg-accent",
                    isToday(date) && "bg-primary/10 font-semibold",
                    isSelected(date) && "bg-primary text-primary-foreground hover:bg-primary/90",
                    !isSelected(date) && !isToday(date) && "text-foreground"
                  )}
                >
                  <span>{date.getDate()}</span>
                  {/* Indicator dots */}
                  {(hasNotes || hasEarnings) && (
                    <div className="absolute bottom-0.5 flex items-center gap-0.5">
                      {hasNotes && (
                        <div
                          className={cn(
                            "w-1 h-1 rounded-full",
                            isSelected(date) ? "bg-primary-foreground" : "bg-primary"
                          )}
                        />
                      )}
                      {hasEarnings && (
                        <div
                          className={cn(
                            "w-1 h-1 rounded-full",
                            isSelected(date) ? "bg-primary-foreground/70" : "bg-amber-500"
                          )}
                          title={`${earningsCount} earnings`}
                        />
                      )}
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </div>

        {/* Right Sidebar - Selected Date Details */}
        <div className="w-[200px] border-l flex flex-col shrink-0 bg-muted/20">
          {selectedDate ? (
            <>
              <div className="px-3 py-3 border-b">
                <p className="text-sm font-medium">
                  {selectedDate.toLocaleDateString("en-US", {
                    weekday: "short",
                    month: "short",
                    day: "numeric",
                  })}
                </p>
                <p className="text-[10px] text-muted-foreground mt-0.5">
                  {selectedDateNotes.length} note{selectedDateNotes.length !== 1 ? "s" : ""} • Double-click for details
                </p>
              </div>

              <div className="flex-1 overflow-auto p-2">
                {selectedDateNotes.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                    <FileText className="h-8 w-8 mb-2 opacity-40" />
                    <p className="text-xs text-center">No notes on this day</p>
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    {selectedDateNotes.map((note) => (
                      <button
                        key={note.id}
                        onClick={() => onNoteSelect?.(note.id)}
                        className="w-full text-left px-2 py-1.5 rounded border bg-card hover:bg-accent/50 transition-colors"
                      >
                        <p className="text-xs font-medium truncate">{note.title}</p>
                        <p className="text-[10px] text-muted-foreground">
                          {new Date(note.created_at).toLocaleTimeString("en-US", {
                            hour: "numeric",
                            minute: "2-digit",
                          })}
                        </p>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
              <Calendar className="h-10 w-10 mb-3 opacity-30" />
              <p className="text-xs text-center">Select a date to view details</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
