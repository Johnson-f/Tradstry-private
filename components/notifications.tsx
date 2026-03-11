"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { Bell, BellOff, Loader2, Send, Settings, ShieldCheck } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useWebPush } from "@/lib/hooks/use-web-push";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

interface NotificationsPreferencesProps {
  className?: string;
}

export function NotificationsPreferences({ className }: NotificationsPreferencesProps) {
  const { requestPermission, subscribe, unsubscribe, sendTest, permission, isSubscribing } = useWebPush();
  const [systemEnabled, setSystemEnabled] = useState(true);
  const [tradeEnabled, setTradeEnabled] = useState(permission === "granted");
  const [isTradePending, setIsTradePending] = useState(false);
  const [isSystemPending, setIsSystemPending] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [autoInitDone, setAutoInitDone] = useState(false);

  const permissionLabel = useMemo(() => {
    if (permission === "granted") return "Notifications enabled";
    if (permission === "denied") return "Notifications blocked";
    return "Permission not requested";
  }, [permission]);

  useEffect(() => {
    if (autoInitDone) return;

    const enableByDefault = async () => {
      if (permission === "granted") {
        setSystemEnabled(true);
        setTradeEnabled(true);
        try {
          await subscribe();
        } catch (error) {
          console.error("Auto-subscribe failed:", error);
        } finally {
          setAutoInitDone(true);
        }
        return;
      }

      if (permission === "default") {
        try {
          const perm = await requestPermission();
          if (perm === "granted") {
            setSystemEnabled(true);
            setTradeEnabled(true);
            await subscribe();
          } else {
            setSystemEnabled(false);
            setTradeEnabled(false);
          }
        } catch (error) {
          console.error("Permission request failed:", error);
          setSystemEnabled(false);
          setTradeEnabled(false);
        } finally {
          setAutoInitDone(true);
        }
        return;
      }

      // permission === "denied"
      setSystemEnabled(false);
      setTradeEnabled(false);
      setAutoInitDone(true);
    };

    void enableByDefault();
  }, [autoInitDone, permission, requestPermission, subscribe]);

  const handleSystemToggle = useCallback(
    async (checked: boolean) => {
      const previous = systemEnabled;
      setIsSystemPending(true);
      try {
      if (checked) {
          const perm = await requestPermission();
          if (perm !== "granted") {
            toast.error("Notifications are blocked. Enable them in your browser settings.");
            setSystemEnabled(false);
            setTradeEnabled(false);
            return;
          }
          await subscribe();
          setSystemEnabled(true);
          setTradeEnabled(true);
          toast.success("Subscribed to system notifications");
      } else {
          await unsubscribe();
          setSystemEnabled(false);
          setTradeEnabled(false);
        toast.message("System notifications disabled");
      }
      } catch (error) {
        console.error("Failed to toggle system notifications:", error);
        toast.error(
          error instanceof Error ? error.message : "Failed to update system notifications"
        );
        setSystemEnabled(previous);
      } finally {
        setIsSystemPending(false);
      }
    },
    [requestPermission, subscribe, unsubscribe, systemEnabled]
  );

  const handleTradeToggle = useCallback(
    async (checked: boolean) => {
      const previous = tradeEnabled;
      setIsTradePending(true);
      try {
        if (checked) {
          const perm = await requestPermission();
          if (perm !== "granted") {
            toast.error("Notifications are blocked. Enable them in your browser settings.");
            setTradeEnabled(false);
            return;
          }
          await subscribe();
          setTradeEnabled(true);
          setSystemEnabled(true);
          toast.success("Subscribed to trade notifications");
        } else {
          await unsubscribe();
          setTradeEnabled(false);
          setSystemEnabled(false);
          toast.success("Unsubscribed from trade notifications");
        }
      } catch (error) {
        console.error("Failed to toggle trade notifications:", error);
        toast.error(
          error instanceof Error ? error.message : "Failed to update trade notifications"
        );
        setTradeEnabled(previous);
      } finally {
        setIsTradePending(false);
      }
    },
    [requestPermission, subscribe, unsubscribe, tradeEnabled]
  );

  const handleTest = useCallback(async () => {
    setIsTesting(true);
    try {
      await sendTest();
      toast.success("Test notification sent! Check your notifications.");
    } catch (error) {
      console.error("Failed to send test push:", error);
      toast.error("Failed to send test notification");
    } finally {
      setIsTesting(false);
    }
  }, [sendTest]);

  const tradeBusy = isTradePending || isSubscribing;
  const systemBusy = isSystemPending || isSubscribing;

  return (
    <div className={cn("space-y-4", className)}>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Notifications
          </CardTitle>
          <CardDescription>
            Manage which alerts you receive. Trades use browser push; system is reserved for account notices.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <ShieldCheck className="h-4 w-4 text-primary" />
                <p className="font-medium">System notifications</p>
              </div>
              <p className="text-sm text-muted-foreground">
                Security and account updates delivered via push.
              </p>
            </div>
            <Switch
              checked={systemEnabled}
              onCheckedChange={handleSystemToggle}
              disabled={systemBusy}
            />
          </div>

          <div className="flex items-start justify-between gap-4">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <Bell className="h-4 w-4 text-primary" />
                <p className="font-medium">Trade notifications</p>
              </div>
              <p className="text-sm text-muted-foreground">
                Price and trade alerts via browser push. Requires notification permission.
              </p>
              <p className="text-xs text-muted-foreground">Status: {permissionLabel}</p>
            </div>
            <Switch
              checked={tradeEnabled}
              onCheckedChange={handleTradeToggle}
              disabled={tradeBusy}
            />
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              onClick={handleTest}
              variant="secondary"
              disabled={!tradeEnabled || isTesting || tradeBusy || systemBusy}
              className="w-full sm:w-auto"
            >
              {isTesting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Sending...
                </>
              ) : (
                <>
                  <Send className="mr-2 h-4 w-4" />
                  Send Test Notification
                </>
              )}
            </Button>
            <Button
              onClick={() => handleTradeToggle(false)}
              variant="ghost"
              disabled={!tradeEnabled || tradeBusy}
              className="w-full sm:w-auto"
            >
              {tradeBusy ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Updating...
                </>
              ) : (
                <>
                  <BellOff className="mr-2 h-4 w-4" />
                  Unsubscribe
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

