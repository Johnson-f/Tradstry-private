"use client";

import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { KeyStats } from "./key-stats";
import { IncomeStatement } from "./income-statement";
import { BalanceSheet } from "./balance-sheet";
import { CashFlow } from "./cash-flow";

interface FinancialsProps {
  symbol: string;
  className?: string;
}

export function Financials({ symbol, className }: FinancialsProps) {
  const [frequency, setFrequency] = useState<"annual" | "quarterly">("annual");

  return (
    <div className={cn("space-y-4", className)}>
      <Tabs defaultValue="key-stats" className="w-full">
        {/* Header with Tabs and Controls */}
        <div className="flex items-center justify-between mb-4">
          <TabsList className="bg-muted/50 h-9 inline-flex">
            <TabsTrigger
              value="key-stats"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Key Stats
            </TabsTrigger>
            <TabsTrigger
              value="income-statement"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Income Statement
            </TabsTrigger>
            <TabsTrigger
              value="balance-sheet"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Balance Sheet
            </TabsTrigger>
            <TabsTrigger
              value="cash-flow"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Cash Flow
            </TabsTrigger>
          </TabsList>

          {/* Controls */}
          <div className="flex items-center gap-2">
            <Select value={frequency} onValueChange={(value) => setFrequency(value as "annual" | "quarterly")}>
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="annual">Annual</SelectItem>
                <SelectItem value="quarterly">Quarterly</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* Tab Content */}
        <TabsContent value="key-stats" className="mt-0">
          <KeyStats symbol={symbol} frequency={frequency} />
        </TabsContent>
        <TabsContent value="income-statement" className="mt-0">
          <IncomeStatement symbol={symbol} frequency={frequency} />
        </TabsContent>
        <TabsContent value="balance-sheet" className="mt-0">
          <BalanceSheet symbol={symbol} frequency={frequency} />
        </TabsContent>
        <TabsContent value="cash-flow" className="mt-0">
          <CashFlow symbol={symbol} frequency={frequency} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default Financials;

