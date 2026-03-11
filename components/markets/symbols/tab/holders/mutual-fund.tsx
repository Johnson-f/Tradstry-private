"use client";

import React from "react";
import { cn } from "@/lib/utils";
import { useHolders } from "@/lib/hooks/use-market-data-service";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";

interface MutualFundProps {
  symbol: string;
  className?: string;
}

function formatNumber(num: number): string {
  if (num >= 1e9) return `${(num / 1e9).toFixed(2)}B`;
  if (num >= 1e6) return `${(num / 1e6).toFixed(2)}M`;
  if (num >= 1e3) return `${(num / 1e3).toFixed(2)}K`;
  return num.toLocaleString();
}

function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString);
    return date.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateString;
  }
}

function Loading({ className }: { className?: string }) {
  return (
    <div className={cn("space-y-4", className)}>
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
      <Skeleton className="h-10 w-full" />
    </div>
  );
}

export function MutualFund({ symbol, className }: MutualFundProps) {
  const { holders, isLoading } = useHolders(
    {
      symbol,
      holder_type: "mutualfund",
    },
    !!symbol
  );

  const holdersList = holders?.mutual_fund_holders ?? [];

  if (isLoading) return <Loading className={className} />;

  if (!holdersList || holdersList.length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No mutual fund holders data available.
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <ScrollArea className="w-full">
        <div className="p-4 sm:p-6">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[30%]">Holder</TableHead>
                <TableHead className="text-right">Shares</TableHead>
                <TableHead className="text-right">% Out</TableHead>
                <TableHead className="text-right">Value</TableHead>
                <TableHead className="text-right">Date Reported</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {holdersList.map((holder, index) => (
                <TableRow key={`${holder.organization}-${index}`}>
                  <TableCell className="font-medium">{holder.organization}</TableCell>
                  <TableCell className="text-right">
                    {formatNumber(holder.shares)}
                  </TableCell>
                  <TableCell className="text-right">
                    {holder.percent_held !== null && holder.percent_held !== undefined
                      ? `${holder.percent_held.toFixed(2)}%`
                      : "--"}
                  </TableCell>
                  <TableCell className="text-right">
                    ${formatNumber(holder.value || 0)}
                  </TableCell>
                  <TableCell className="text-right text-muted-foreground">
                    {formatDate(holder.date_reported || '')}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        <ScrollBar orientation="horizontal" />
      </ScrollArea>
    </div>
  );
}

