"use client";

import React from "react";
import { cn } from "@/lib/utils";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { InstitutionalHolders } from "./institutional-holders";
import { MutualFund } from "./mutual-fund";
import { InsiderTransactions } from "./insider-transactions";
import { InsiderPurchase } from "./insider-purchase";
import { InsiderRoster } from "./insider-roster";

interface HoldersProps {
  symbol: string;
  className?: string;
}

export function Holders({ symbol, className }: HoldersProps) {
  return (
    <div className={cn("space-y-4", className)}>
      <Tabs defaultValue="institutional" className="w-full">
        {/* Header with Tabs */}
        <div className="flex items-center justify-between mb-4">
          <TabsList className="bg-muted/50 h-9 inline-flex">
            <TabsTrigger
              value="institutional"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Institutional
            </TabsTrigger>
            <TabsTrigger
              value="mutual-fund"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Mutual Fund
            </TabsTrigger>
            <TabsTrigger
              value="insider-transactions"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Insider Transactions
            </TabsTrigger>
            <TabsTrigger
              value="insider-purchases"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Insider Purchases
            </TabsTrigger>
            <TabsTrigger
              value="insider-roster"
              className="data-[state=active]:bg-teal-500 data-[state=active]:text-white px-4"
            >
              Insider Roster
            </TabsTrigger>
          </TabsList>
        </div>

        {/* Tab Content */}
        <TabsContent value="institutional" className="mt-0">
          <InstitutionalHolders symbol={symbol} />
        </TabsContent>
        <TabsContent value="mutual-fund" className="mt-0">
          <MutualFund symbol={symbol} />
        </TabsContent>
        <TabsContent value="insider-transactions" className="mt-0">
          <InsiderTransactions symbol={symbol} />
        </TabsContent>
        <TabsContent value="insider-purchases" className="mt-0">
          <InsiderPurchase symbol={symbol} />
        </TabsContent>
        <TabsContent value="insider-roster" className="mt-0">
          <InsiderRoster symbol={symbol} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default Holders;

