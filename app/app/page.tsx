"use client";

import DashboardContent from "./dashboard-content";
import { useAuth } from "@/lib/hooks/use-auth";
import { Loader2 } from "lucide-react";

export default function HomePage() {
  const { loading } = useAuth();

  // AuthWrapper already handles authentication and redirects
  // We just need to show loading state while checking
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="flex items-center gap-2">
          <Loader2 className="h-6 w-6 animate-spin" />
          <span>Loading...</span>
        </div>
      </div>
    );
  }

  // If we reach here, user is authenticated (AuthWrapper ensures this)
  return <DashboardContent />;
}