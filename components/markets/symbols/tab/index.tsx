"use client";

import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Financials } from "./financials";
import { Holders } from "./holders";
import { PriceWithTimeframe } from "./overview/price";
import { Charting } from "./overview/charting";
import { About } from "./overview/about";
import { Developments } from "./overview/developments";
import { CompanyInfo } from "../company-info";
import { Peers } from "../peers";

interface SymbolTabsProps {
  symbol: string;
  className?: string;
}

type Timeframe = "1D" | "5D" | "1M" | "6M" | "YTD" | "1Y" | "5Y" | "MAX";

export function SymbolTabs({ symbol, className }: SymbolTabsProps) {
  const [timeframePeriodChange, setTimeframePeriodChange] = useState<number | null>(null);
  const [timeframeLabel, setTimeframeLabel] = useState<string>("Today");

  const handleTimeframeChange = (timeframe: Timeframe, periodChange: number | null) => {
    setTimeframePeriodChange(periodChange);
    setTimeframeLabel(timeframe);
  };

  return (
    <div className={cn("w-full", className)}>
      <Tabs defaultValue="overview" className="w-full">
        <TabsList className="bg-transparent h-auto p-0 gap-10 inline-flex">
          <TabsTrigger
            value="overview"
            className={cn(
              "rounded-full px-4 py-2 text-sm font-medium transition-all",
              "border border-border bg-muted/50 text-muted-foreground",
              "data-[state=active]:bg-teal-500 data-[state=active]:text-teal-50",
              "data-[state=active]:border-teal-500 data-[state=active]:shadow-none",
              "hover:bg-muted/70"
            )}
          >
            Overview
          </TabsTrigger>
          <TabsTrigger
            value="financials"
            className={cn(
              "rounded-full px-4 py-2 text-sm font-medium transition-all",
              "border border-border bg-muted/50 text-muted-foreground",
              "data-[state=active]:bg-teal-500 data-[state=active]:text-teal-50",
              "data-[state=active]:border-teal-500 data-[state=active]:shadow-none",
              "hover:bg-muted/70"
            )}
          >
            Financials
          </TabsTrigger>
          <TabsTrigger
            value="earnings"
            className={cn(
              "rounded-full px-4 py-2 text-sm font-medium transition-all",
              "border border-border bg-muted/50 text-muted-foreground",
              "data-[state=active]:bg-teal-500 data-[state=active]:text-teal-50",
              "data-[state=active]:border-teal-500 data-[state=active]:shadow-none",
              "hover:bg-muted/70"
            )}
          >
            Earnings
          </TabsTrigger>
          <TabsTrigger
            value="holders"
            className={cn(
              "rounded-full px-4 py-2 text-sm font-medium transition-all",
              "border border-border bg-muted/50 text-muted-foreground",
              "data-[state=active]:bg-teal-500 data-[state=active]:text-teal-50",
              "data-[state=active]:border-teal-500 data-[state=active]:shadow-none",
              "hover:bg-muted/70"
            )}
          >
            Holders
          </TabsTrigger>
        </TabsList>

        <div className="mt-6">
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_400px] gap-6">
            {/* Main Content Area */}
            <div className="min-w-0">
              <TabsContent value="overview" className="mt-0">
                <div className="space-y-6">
                  {/* Price - Dynamic based on timeframe */}
                  <PriceWithTimeframe 
                    symbol={symbol} 
                    timeframePeriodChange={timeframePeriodChange}
                    timeframeLabel={timeframeLabel}
                  />

                  {/* Charting */}
                  <Charting 
                    symbol={symbol} 
                    onTimeframeChange={handleTimeframeChange}
                  />

                  {/* Single Column Layout */}
                  <div className="space-y-6">
                    <About symbol={symbol} />
                    <Developments symbol={symbol} />
                  </div>
                </div>
              </TabsContent>
              <TabsContent value="financials" className="mt-0">
                <Financials symbol={symbol} />
              </TabsContent>
              <TabsContent value="earnings" className="mt-0">
                <div className="text-sm text-muted-foreground">Earnings content coming soon</div>
              </TabsContent>
              <TabsContent value="holders" className="mt-0">
                <Holders symbol={symbol} />
              </TabsContent>
            </div>

            {/* Right Sidebar - CompanyInfo and Peers */}
            <div className="lg:sticky lg:-top-16 h-fit space-y-6">
              <CompanyInfo symbol={symbol} className="-mt-32" />
              <Peers symbol={symbol} />
            </div>
          </div>
        </div>
      </Tabs>
    </div>
  );
}

export default SymbolTabs;

