"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Chrome, Bell, Loader2, Check, AlertCircle, Unlink } from "lucide-react";
import {
  useCalendarConnections,
  useCalendarAuthUrl,
  useDeleteCalendarConnection,
  useSyncCalendarConnection,
} from "@/lib/hooks/use-notebook";
import { toast } from "sonner";

interface CalendarActionsProps {
  onOpenReminders?: () => void;
}

export function CalendarActions({ onOpenReminders }: CalendarActionsProps) {
  const [googleDialogOpen, setGoogleDialogOpen] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);

  // Hooks
  const { data: connections = [], isLoading: connectionsLoading } = useCalendarConnections();
  const { refetch: fetchAuthUrl, isFetching: isGettingAuthUrl } = useCalendarAuthUrl();
  const deleteConnection = useDeleteCalendarConnection();
  const syncConnection = useSyncCalendarConnection();

  // Check if Google is connected
  const googleConnection = connections.find((c) => c.provider === "google");
  const isConnected = !!googleConnection;

  // Handle Google connect - redirect to OAuth
  const handleGoogleConnect = async () => {
    try {
      const result = await fetchAuthUrl();
      if (result.data) {
        // Redirect to Google OAuth
        window.location.href = result.data;
      }
    } catch (error) {
      toast.error("Failed to start Google connection");
      console.error("Auth URL error:", error);
    }
  };

  // Handle disconnect
  const handleDisconnect = async () => {
    if (!googleConnection) return;

    try {
      await deleteConnection.mutateAsync(googleConnection.id);
      toast.success("Google Calendar disconnected");
    } catch (error) {
      toast.error("Failed to disconnect");
      console.error("Disconnect error:", error);
    }
  };

  // Handle sync
  const handleSync = async () => {
    if (!googleConnection) return;

    setIsSyncing(true);
    try {
      const result = await syncConnection.mutateAsync(googleConnection.id);
      toast.success(`Synced ${result.synced_events || 0} events`);
    } catch (error) {
      toast.error("Failed to sync calendar");
      console.error("Sync error:", error);
    } finally {
      setIsSyncing(false);
    }
  };

  // Check for OAuth callback on mount
  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    const calendarConnected = urlParams.get("calendar_connected");

    if (calendarConnected === "true") {
      toast.success("Google Calendar connected successfully!");
      // Clean up URL
      window.history.replaceState({}, "", window.location.pathname);
    } else if (calendarConnected === "false") {
      toast.error("Failed to connect Google Calendar");
      window.history.replaceState({}, "", window.location.pathname);
    }
  }, []);

  const isLoading = isGettingAuthUrl || deleteConnection.isPending;

  return (
    <>
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setGoogleDialogOpen(true)}
          className="gap-2"
        >
          <Chrome className="h-4 w-4" />
          {isConnected ? "Google Connected" : "Connect to Google"}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onOpenReminders}
          className="gap-2"
        >
          <Bell className="h-4 w-4" />
          Manage Reminders
        </Button>
      </div>

      {/* Google Connect Dialog */}
      <Dialog open={googleDialogOpen} onOpenChange={setGoogleDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Chrome className="h-5 w-5" />
              {isConnected ? "Google Calendar Connected" : "Connect to Google Calendar"}
            </DialogTitle>
            <DialogDescription>
              {isConnected
                ? "Your Google Calendar is synced with your notes calendar."
                : "Sync your notes calendar with Google Calendar to keep everything in one place."}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            {connectionsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : isConnected ? (
              <div className="space-y-4">
                {/* Connected state */}
                <div className="flex flex-col items-center gap-3 py-4">
                  <div className="h-12 w-12 rounded-full bg-green-500/10 flex items-center justify-center">
                    <Check className="h-6 w-6 text-green-500" />
                  </div>
                  <p className="text-sm font-medium text-green-600">Connected</p>
                  {googleConnection.account_email && (
                    <p className="text-xs text-muted-foreground">{googleConnection.account_email}</p>
                  )}
                  {googleConnection.last_sync_at && (
                    <p className="text-xs text-muted-foreground">
                      Last synced: {new Date(googleConnection.last_sync_at).toLocaleString()}
                    </p>
                  )}
                </div>

                {/* Actions */}
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    className="flex-1 gap-2"
                    onClick={handleSync}
                    disabled={isSyncing}
                  >
                    {isSyncing ? (
                      <>
                        <Loader2 className="h-4 w-4 animate-spin" />
                        Syncing...
                      </>
                    ) : (
                      <>
                        <Chrome className="h-4 w-4" />
                        Sync Now
                      </>
                    )}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="gap-2 text-destructive hover:text-destructive"
                    onClick={handleDisconnect}
                    disabled={deleteConnection.isPending}
                  >
                    {deleteConnection.isPending ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Unlink className="h-4 w-4" />
                    )}
                    Disconnect
                  </Button>
                </div>

                <Button
                  variant="ghost"
                  size="sm"
                  className="w-full"
                  onClick={() => setGoogleDialogOpen(false)}
                >
                  Done
                </Button>
              </div>
            ) : (
              <>
                {/* Not connected state */}
                <div className="rounded-lg border p-4 space-y-3">
                  <h4 className="text-sm font-medium">What you&apos;ll get:</h4>
                  <ul className="text-sm text-muted-foreground space-y-2">
                    <li className="flex items-start gap-2">
                      <Check className="h-4 w-4 text-primary mt-0.5 flex-shrink-0" />
                      <span>Sync note creation dates to Google Calendar</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <Check className="h-4 w-4 text-primary mt-0.5 flex-shrink-0" />
                      <span>Set reminders for important notes</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <Check className="h-4 w-4 text-primary mt-0.5 flex-shrink-0" />
                      <span>View your trading journal alongside other events</span>
                    </li>
                  </ul>
                </div>

                <Button
                  className="w-full gap-2"
                  onClick={handleGoogleConnect}
                  disabled={isLoading}
                >
                  {isLoading ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Connecting...
                    </>
                  ) : (
                    <>
                      <Chrome className="h-4 w-4" />
                      Sign in with Google
                    </>
                  )}
                </Button>

                <p className="text-xs text-muted-foreground text-center">
                  By connecting, you agree to allow Tradstry to access your Google Calendar.
                </p>
              </>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
