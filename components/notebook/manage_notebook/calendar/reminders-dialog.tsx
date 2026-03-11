"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Bell, Plus, Trash2, Clock, Calendar, Loader2 } from "lucide-react";
import {
  useCalendarEvents,
  useCreateReminder,
  useDeleteCalendarEvent,
} from "@/lib/hooks/use-notebook";
import { toast } from "sonner";

interface RemindersDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function RemindersDialog({ open, onOpenChange }: RemindersDialogProps) {
  const [newTitle, setNewTitle] = useState("");
  const [newDate, setNewDate] = useState("");
  const [newTime, setNewTime] = useState("");

  // Hooks
  const { data: events = [], isLoading: eventsLoading } = useCalendarEvents({ is_reminder: true });
  const createReminder = useCreateReminder();
  const deleteEvent = useDeleteCalendarEvent();

  // Filter to only show reminders
  const reminders = events.filter((e) => e.is_reminder);

  const handleAddReminder = async () => {
    if (!newTitle.trim() || !newDate || !newTime) return;

    try {
      // Combine date and time into a Date object
      const startTime = new Date(`${newDate}T${newTime}`);

      await createReminder.mutateAsync({
        title: newTitle.trim(),
        startTime,
        reminderMinutes: 15, // Default 15 min reminder
      });

      toast.success("Reminder created");
      setNewTitle("");
      setNewDate("");
      setNewTime("");
    } catch (error) {
      toast.error("Failed to create reminder");
      console.error("Create reminder error:", error);
    }
  };

  const handleDeleteReminder = async (id: string) => {
    try {
      await deleteEvent.mutateAsync(id);
      toast.success("Reminder deleted");
    } catch (error) {
      toast.error("Failed to delete reminder");
      console.error("Delete reminder error:", error);
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString("en-US", {
      weekday: "short",
      month: "short",
      day: "numeric",
    });
  };

  const formatTime = (dateStr: string) => {
    return new Date(dateStr).toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
    });
  };

  const isSubmitting = createReminder.isPending;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bell className="h-5 w-5" />
            Manage Reminders
          </DialogTitle>
          <DialogDescription>
            Set reminders for your trading notes and journal entries.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Add new reminder form */}
          <div className="space-y-3 p-4 rounded-lg border bg-muted/30">
            <h4 className="text-sm font-medium">Add New Reminder</h4>
            <div className="space-y-2">
              <Label htmlFor="reminder-title" className="text-xs">
                Title
              </Label>
              <Input
                id="reminder-title"
                placeholder="e.g., Review trading journal"
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <Label htmlFor="reminder-date" className="text-xs">
                  Date
                </Label>
                <Input
                  id="reminder-date"
                  type="date"
                  value={newDate}
                  onChange={(e) => setNewDate(e.target.value)}
                  min={new Date().toISOString().split("T")[0]}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="reminder-time" className="text-xs">
                  Time
                </Label>
                <Input
                  id="reminder-time"
                  type="time"
                  value={newTime}
                  onChange={(e) => setNewTime(e.target.value)}
                />
              </div>
            </div>
            <Button
              size="sm"
              onClick={handleAddReminder}
              disabled={!newTitle.trim() || !newDate || !newTime || isSubmitting}
              className="w-full gap-2"
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Adding...
                </>
              ) : (
                <>
                  <Plus className="h-4 w-4" />
                  Add Reminder
                </>
              )}
            </Button>
          </div>

          {/* Reminders list */}
          <div className="space-y-2">
            <h4 className="text-sm font-medium">Your Reminders</h4>
            {eventsLoading ? (
              <div className="flex items-center justify-center py-6">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : reminders.length === 0 ? (
              <div className="text-center py-6 border rounded-lg bg-muted/20">
                <Bell className="h-8 w-8 mx-auto text-muted-foreground/50 mb-2" />
                <p className="text-sm text-muted-foreground">No reminders yet</p>
                <p className="text-xs text-muted-foreground/70">
                  Add a reminder to get notified
                </p>
              </div>
            ) : (
              <ScrollArea className="h-[200px]">
                <div className="space-y-2 pr-3">
                  {reminders.map((reminder) => (
                    <div
                      key={reminder.id}
                      className="flex items-center justify-between p-3 rounded-lg border bg-card"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{reminder.title}</p>
                        <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Calendar className="h-3 w-3" />
                            {formatDate(reminder.start_time)}
                          </span>
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {formatTime(reminder.start_time)}
                          </span>
                        </div>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-destructive hover:text-destructive"
                        onClick={() => handleDeleteReminder(reminder.id)}
                        disabled={deleteEvent.isPending}
                      >
                        {deleteEvent.isPending ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Trash2 className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
