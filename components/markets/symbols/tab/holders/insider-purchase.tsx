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

interface InsiderPurchaseProps {
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

function formatPrice(price: number | null | undefined): string {
  if (price === null || price === undefined) return "N/A";
  return `$${price.toFixed(2)}`;
}

function formatShares(shares: number): string {
  const absShares = Math.abs(shares);
  return `${shares >= 0 ? "+" : ""}${formatNumber(absShares)}`;
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

export function InsiderPurchase({ symbol, className }: InsiderPurchaseProps) {
  const { holders, isLoading } = useHolders(
    {
      symbol,
      holder_type: "insider_purchases",
    },
    !!symbol
  );

  const purchasesSummary = holders?.insider_purchases;

  if (isLoading) return <Loading className={className} />;

  if (!purchasesSummary) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No insider purchases data available.
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
        <div className="p-4 sm:p-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Purchases</p>
            <p className="text-lg font-semibold text-foreground">
              {purchasesSummary.purchases ?? 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Purchase Shares</p>
            <p className="text-lg font-semibold text-green-600 dark:text-green-400">
              {purchasesSummary.purchases_shares !== null && purchasesSummary.purchases_shares !== undefined
                ? formatNumber(purchasesSummary.purchases_shares)
                : 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Sales</p>
            <p className="text-lg font-semibold text-foreground">
              {purchasesSummary.sales ?? 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Sales Shares</p>
            <p className="text-lg font-semibold text-red-600 dark:text-red-400">
              {purchasesSummary.sales_shares !== null && purchasesSummary.sales_shares !== undefined
                ? formatNumber(purchasesSummary.sales_shares)
                : 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Net Shares Purchased</p>
            <p className={cn(
              "text-lg font-semibold",
              (purchasesSummary.net_shares_purchased ?? 0) >= 0
                ? "text-green-600 dark:text-green-400"
                : "text-red-600 dark:text-red-400"
            )}>
              {purchasesSummary.net_shares_purchased !== null && purchasesSummary.net_shares_purchased !== undefined
                ? formatShares(purchasesSummary.net_shares_purchased)
                : 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Net Direct Purchases</p>
            <p className={cn(
              "text-lg font-semibold",
              (purchasesSummary.net_shares_purchased_directly ?? 0) >= 0
                ? "text-green-600 dark:text-green-400"
                : "text-red-600 dark:text-red-400"
            )}>
              {purchasesSummary.net_shares_purchased_directly !== null && purchasesSummary.net_shares_purchased_directly !== undefined
                ? formatShares(purchasesSummary.net_shares_purchased_directly)
                : 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Total Insider Shares</p>
            <p className="text-lg font-semibold text-foreground">
              {purchasesSummary.total_insider_shares !== null && purchasesSummary.total_insider_shares !== undefined
                ? formatNumber(purchasesSummary.total_insider_shares)
                : 'N/A'}
            </p>
          </div>
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">Buy Info Shares</p>
            <p className="text-lg font-semibold text-green-600 dark:text-green-400">
              {purchasesSummary.buy_info_shares !== null && purchasesSummary.buy_info_shares !== undefined
                ? formatNumber(purchasesSummary.buy_info_shares)
                : 'N/A'}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

