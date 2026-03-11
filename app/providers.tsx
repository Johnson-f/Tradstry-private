"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import { ReactNode, useState } from "react";
import { Toaster } from "sonner";

// Create a stable QueryClient instance that works in both SSR and client
function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 2 * 60 * 1000, // 2 minutes - balance freshness vs performance
        gcTime: 5 * 60 * 1000, // 5 minutes - garbage collection time
        refetchOnWindowFocus: true, // Refetch on focus for fresh data
        refetchOnMount: true, // Refetch on mount if data is stale
        refetchOnReconnect: true, // Refetch when reconnecting
        refetchInterval: false, // No automatic polling (use WebSocket instead)
        retry: 2, // Retry twice on failure
        retryDelay: (attemptIndex) =>
          Math.min(1000 * 2 ** attemptIndex, 10000), // Cap at 10s
        networkMode: "online",
      },
      mutations: {
        retry: 1, // Retry mutations once
        networkMode: "online",
      },
    },
  });
}

function getQueryClient() {
  // Always create a new instance to prevent SSR/client mismatch issues
  return makeQueryClient();
}

export function Providers({ children }: { children: ReactNode }) {
  const [queryClient] = useState(() => getQueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        {children}
        <Toaster position="top-right" duration={5000} />
      </ThemeProvider>
    </QueryClientProvider>
  );
}
