"use client";

import { Button } from "@/components/ui/button";
import { PanelLeft } from "lucide-react";
import { useSidebar } from "@/components/ui/sidebar";

interface AppHeaderProps {
  children?: React.ReactNode;
}

export function AppHeader({ children }: AppHeaderProps) {
  const { toggleSidebar } = useSidebar();

  return (
    <header className="w-full border-b bg-background flex-shrink-0">
      <div className="flex items-center h-14">
        {/* Sidebar Toggle Button - Left Corner */}
        <div className="flex items-center justify-center w-14 h-14 border-r">
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            className="h-9 w-9"
            aria-label="Toggle sidebar"
          >
            <PanelLeft className="h-5 w-5" />
            <span className="sr-only">Toggle Sidebar</span>
          </Button>
        </div>

        {/* Main Header Content */}
        <div className="flex-1 flex items-center justify-between px-8">
          {children}
        </div>
      </div>
    </header>
  );
}
