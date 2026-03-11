"use client";

import type React from "react";
import { useState, useEffect, useMemo } from "react";
import { Loader2 } from "lucide-react";

import { AppSidebar } from "@/components/app-sidebar";
import { WebSocketProvider } from "@/lib/websocket/provider";
import { WebSocketConnectionControl } from "@/components/websocket-connection-control";
import { useUserInitialization } from "@/hooks/use-user-initialization";
import { Providers } from "../providers";
import { InitializationContext } from "@/contexts/initialization-context";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";

// Constants
const INITIALIZATION_STATUS_TIMEOUT = 3000;

export default function ProtectedLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <Providers>
      <WebSocketProvider>
        <ProtectedLayoutInner>{children}</ProtectedLayoutInner>
      </WebSocketProvider>
    </Providers>
  );
}

function ProtectedLayoutInner({ children }: { children: React.ReactNode }) {
  const { isInitialized, isInitializing, error, needsRefresh } =
    useUserInitialization();
  const [showInitializationStatus, setShowInitializationStatus] =
    useState(false);

  // Memoize context value to prevent unnecessary re-renders
  const initializationContextValue = useMemo(
    () => ({ isInitialized, isInitializing, error }),
    [isInitialized, isInitializing, error]
  );

  useEffect(() => {
    if (isInitializing) {
      setShowInitializationStatus(true);
    } else if (isInitialized || error) {
      const timer = setTimeout(() => {
        setShowInitializationStatus(false);
      }, INITIALIZATION_STATUS_TIMEOUT);
      return () => clearTimeout(timer);
    }
  }, [isInitializing, isInitialized, error]);

  return (
    <InitializationContext.Provider value={initializationContextValue}>
      <WebSocketConnectionControl />
      <SidebarProvider defaultOpen={true}>
        <AppSidebar />
        <SidebarInset className="min-h-screen">
          {showInitializationStatus && (
            <InitializationStatusToast
              isInitializing={isInitializing}
              isInitialized={isInitialized}
              error={error}
              needsRefresh={needsRefresh}
            />
          )}
          {children}
        </SidebarInset>
      </SidebarProvider>
    </InitializationContext.Provider>
  );
}

// Extracted component for better readability
function InitializationStatusToast({
  isInitializing,
  isInitialized,
  error,
  needsRefresh,
}: {
  isInitializing: boolean;
  isInitialized: boolean;
  error: string | null;
  needsRefresh: boolean;
}) {
  return (
    <div className="fixed top-4 right-4 z-50 bg-background border rounded-lg shadow-lg p-4 max-w-sm">
      {isInitializing && (
        <div className="flex items-center gap-2 text-sm">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>Setting up your account...</span>
        </div>
      )}
      {isInitialized && !isInitializing && (
        <div className="flex items-center gap-2 text-sm text-green-600">
          <div className="h-2 w-2 bg-green-600 rounded-full" />
          <span>Account setup complete!</span>
        </div>
      )}
      {error && !isInitializing && (
        <div className="text-sm text-red-600">
          <p>Setup failed: {error}</p>
          <p className="text-xs mt-1 text-muted-foreground">
            {needsRefresh
              ? "Please refresh the page to try again"
              : "You can still use the app"}
          </p>
        </div>
      )}
    </div>
  );
}
