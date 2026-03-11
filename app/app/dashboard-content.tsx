"use client";

import EmbeddedAIChat from "@/components/dashboard/embedded-ai-chat";
import RecentChatsSheet from "@/components/dashboard/recent-chats-sheet";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useAIChat } from "@/hooks/use-ai-chat";
import { getTimeBasedGreeting } from "@/lib/utils/greetings";
import { useUserProfile } from "@/hooks/use-user-profile";
import { AppPageHeader } from '@/components/app-page-header';

export default function DashboardContent() {
  return (
    <div className="flex flex-col h-full">
      <DashboardHeader />
      <DashboardMain />
    </div>
  );
}

function DashboardHeader() {
  const { totalSessionsCount, isLoading } = useAIChat();

  return (
    <AppPageHeader
      title="HOME"
      actions={
        <nav className="flex items-center gap-2" aria-label="Dashboard navigation">
          <RecentChatsSheet>
            <Button variant="outline" size="sm" className="relative">
              All Chats
              {!isLoading && totalSessionsCount > 0 && (
                <Badge 
                  variant="secondary" 
                  className="ml-2 h-5 w-5 rounded-full p-0 flex items-center justify-center text-xs"
                  aria-label={`${totalSessionsCount} chat sessions`}
                >
                  {totalSessionsCount}
                </Badge>
              )}
            </Button>
          </RecentChatsSheet>
        </nav>
      }
    />
  );
}

function DashboardMain() {
  return (
    <main className="flex-1 flex items-center justify-center overflow-auto px-8">
      <div className="w-full max-w-5xl py-8 space-y-6">
        <DashboardGreeting />
        <EmbeddedAIChat defaultExpanded={true} className="mb-8" />
      </div>
    </main>
  );
}

function DashboardGreeting() {
  const { firstName, loading, email } = useUserProfile();
  const greetingData = getTimeBasedGreeting();
  
  if (loading) {
    return <DashboardGreetingSkeleton />;
  }

  const displayName = getDisplayName(firstName, email);

  return (
    <section className="space-y-3 text-center" aria-label="Dashboard greeting">
      <GreetingHeader 
        timeGreeting={greetingData.timeGreeting} 
        displayName={displayName}
        showProfilePrompt={!firstName && !!email}
      />
      <GreetingMessages 
        casualGreeting={greetingData.casualGreeting}
        tradingReminder={greetingData.tradingReminder}
        marketStatus={greetingData.marketStatus}
      />
    </section>
  );
}

function DashboardGreetingSkeleton() {
  return (
    <div className="space-y-2 text-center" role="status" aria-label="Loading greeting">
      <div className="h-8 w-64 bg-muted rounded animate-pulse mx-auto" />
      <div className="h-4 w-96 bg-muted rounded animate-pulse mx-auto" />
      <div className="h-4 w-80 bg-muted rounded animate-pulse mx-auto" />
      <span className="sr-only">Loading...</span>
    </div>
  );
}

function GreetingHeader({ 
  timeGreeting, 
  displayName, 
  showProfilePrompt 
}: { 
  timeGreeting: string; 
  displayName: string; 
  showProfilePrompt: boolean;
}) {
  return (
    <div>
      <h2 className="text-2xl font-semibold">
        {timeGreeting}{displayName ? `, ${displayName}` : ''}!
      </h2>
      {showProfilePrompt && (
        <p className="text-sm text-muted-foreground mt-1">
          Welcome back! Update your profile to personalize your experience.
        </p>
      )}
    </div>
  );
}

function GreetingMessages({ 
  casualGreeting, 
  tradingReminder, 
  marketStatus 
}: { 
  casualGreeting: string; 
  tradingReminder: string; 
  marketStatus: string;
}) {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-base text-foreground/80">
        {casualGreeting}
      </p>
      <p className="text-sm text-muted-foreground italic">
        <span aria-hidden="true">💡</span> {tradingReminder}
      </p>
      <p className="text-xs text-muted-foreground/80 font-medium">
        <span aria-hidden="true">📊</span> {marketStatus}
      </p>
    </div>
  );
}

// Helper function to extract display name logic
function getDisplayName(firstName: string | null, email: string | null): string {
  if (firstName) return firstName;
  if (email) return email.split('@')[0];
  return '';
}
