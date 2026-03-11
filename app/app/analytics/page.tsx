"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { AppPageHeader } from "@/components/app-page-header";
import { ProfitCalendar } from "@/components/analytics/tabs/profit-calendar";
import { DurationPlaybookPerformance } from "@/components/analytics/tabs/performance/duration-playbook";
import { OpenTradesTable } from "@/components/analytics/tabs/open-trades-table";
import { NetPnlChart } from "@/components/analytics/tabs/net-pnl-chart";

export default function AnalyticsSectionPage() {
  return (
    <div className="h-screen flex flex-col bg-background">
      <AppPageHeader title="Analytics" />

      {/* Main content - Scrollable area with shadcn ScrollArea */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-8">
            <Tabs defaultValue="overview" className="w-full">
              <TabsList className="grid w-full grid-cols-4">
                <TabsTrigger value="overview">Overview</TabsTrigger>
                <TabsTrigger value="calendar">Calendar</TabsTrigger>
                <TabsTrigger value="performance">Performance</TabsTrigger>
                <TabsTrigger value="risk">Risk</TabsTrigger>
              </TabsList>

              <div className="mt-6">
                <TabsContent value="overview" className="mt-0 space-y-6">
                  <NetPnlChart />
                  <OpenTradesTable />
                </TabsContent>

                <TabsContent value="calendar" className="mt-0">
                  <ProfitCalendar />
                </TabsContent>

                <TabsContent value="performance" className="mt-0">
                  <DurationPlaybookPerformance />
                </TabsContent>

                <TabsContent value="risk" className="mt-0">
                  <div className="text-center py-12 text-muted-foreground">
                    Risk content coming soon
                  </div>
                </TabsContent>
              </div>
            </Tabs>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}