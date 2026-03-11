"use client";

import { ReactNode, useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { useAuth } from "@/lib/hooks/use-auth";
import { Loader2 } from "lucide-react";

interface AuthWrapperProps {
  children: ReactNode;
}

export function AuthWrapper({ children }: AuthWrapperProps) {
  const { loading, isAuthenticated } = useAuth();
  const pathname = usePathname();
  const router = useRouter();
  const [isClient, setIsClient] = useState(false);
  const [isRedirecting, setIsRedirecting] = useState(false);

  // Only run client-side logic after hydration
  useEffect(() => {
    setIsClient(true);
  }, []);

  // Handle redirects in useEffect to avoid rendering issues
  useEffect(() => {
    if (!isClient || loading) return;

    const publicRoutes = [
      "/",
      "/auth/login",
      "/auth/sign-up",
      "/auth/forgot-password",
      "/auth/update-password",
      "/auth/confirm",
      "/auth/error",
      "/auth/sign-up-success",
    ];

    const isPublicRoute = publicRoutes.includes(pathname);
    const isAuthPage = pathname.startsWith("/auth");

    // Redirect unauthenticated users trying to access protected routes
    if (!isAuthenticated && !isPublicRoute) {
      setIsRedirecting(true);
      router.replace("/auth/login");
      return;
    }

    // Redirect authenticated users away from auth pages (except error pages)
    if (isAuthenticated && isAuthPage && !pathname.includes("/auth/error")) {
      setIsRedirecting(true);
      router.replace("/app");
      return;
    }

    // Clear redirecting state when we're on the correct page
    if (isRedirecting) {
      setIsRedirecting(false);
    }
  }, [isClient, loading, isAuthenticated, pathname, router, isRedirecting]);

  // Don't render anything during SSR
  if (!isClient) {
    return <>{children}</>;
  }

  // Show loading state while checking authentication or redirecting
  if (loading || isRedirecting) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="flex items-center gap-2">
          <Loader2 className="h-6 w-6 animate-spin" />
          <span>{isRedirecting ? "Redirecting..." : "Loading..."}</span>
        </div>
      </div>
    );
  }

  // No WebSocketProvider here anymore - it's in ProtectedLayout
  return <>{children}</>;
}