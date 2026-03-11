"use client";

import { SidebarTrigger } from "@/components/ui/sidebar";
import { PanelLeft } from "lucide-react";
import { cn } from "@/lib/utils";

interface AppPageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  height?: string;
}

export function AppPageHeader({ title, subtitle, actions, height = "3.5rem" }: AppPageHeaderProps) {
  return (
    <div className="sticky top-0 z-0 w-full border-b bg-background flex-shrink-0">
      <div 
        className="flex items-center"
        style={{ minHeight: height }}
      >
        {/* Sidebar Toggle Button - Left Corner */}
        <div 
          className={cn("flex items-center justify-center w-14 pl-5 border-r flex-shrink-0")}
          style={{ height }}
        >
          <SidebarTrigger className="h-9 w-9">
            <PanelLeft className="h-5 w-5" />
            <span className="sr-only">Toggle Sidebar</span>
          </SidebarTrigger>
        </div>

        {/* Header Content */}
        <div className="flex-1 px-8 py-4">
          <div className="flex items-start justify-between gap-4">
            <div className="flex flex-col">
            <h1 className="text-2xl font-bold tracking-tight">{title}</h1>
              {subtitle && (
                <p className="text-sm text-muted-foreground">{subtitle}</p>
              )}
            </div>
            {actions && <div className="flex items-center gap-4">{actions}</div>}
          </div>
        </div>
      </div>
    </div>
  );
}
