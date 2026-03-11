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

interface InsiderTransactionsProps {
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

export function InsiderTransactions({
  symbol,
  className,
}: InsiderTransactionsProps) {
  const { holders, isLoading } = useHolders(
    {
      symbol,
      holder_type: "insider_transactions",
    },
    !!symbol
  );

  const transactions = holders?.insider_transactions ?? [];

  if (isLoading) return <Loading className={className} />;

  if (!transactions || transactions.length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No insider transactions data available.
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
                <TableHead className="w-[25%]">Insider</TableHead>
                <TableHead className="w-[20%]">Transaction Type</TableHead>
                <TableHead className="text-right">Shares</TableHead>
                <TableHead className="text-right">Price</TableHead>
                <TableHead className="text-right">Value</TableHead>
                <TableHead className="text-right">Date</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {transactions.map((transaction, index) => (
                // @ts-ignore
                <TableRow key={`${transaction.filer_name}-${transaction.start_date || transaction.filing_date || index}`}>
                  <TableCell className="font-medium">
                    {transaction.filer_name}
                  </TableCell>
                  <TableCell>
                    <span
                      className={cn(
                        "inline-flex items-center px-2 py-1 rounded-full text-xs font-medium",
                        transaction.transaction_text?.toLowerCase().includes("buy") ||
                          transaction.transaction_text?.toLowerCase().includes("purchase") ||
                          transaction.transaction_text?.toLowerCase().includes("acquired")
                          ? "bg-green-500/10 text-green-600 dark:text-green-400"
                          : transaction.transaction_text?.toLowerCase().includes("sell") ||
                            transaction.transaction_text?.toLowerCase().includes("disposed")
                          ? "bg-red-500/10 text-red-600 dark:text-red-400"
                          : "bg-muted text-muted-foreground"
                      )}
                    >
                      {transaction.transaction_text || transaction.filer_relation || '--'}
                    </span>
                  </TableCell>
                  <TableCell
                    className={cn(
                      "text-right font-medium",
                      (transaction.shares || 0) >= 0
                        ? "text-green-600 dark:text-green-400"
                        : "text-red-600 dark:text-red-400"
                    )}
                  >
                    {formatShares(transaction.shares || 0)}
                  </TableCell>
                  <TableCell className="text-right">
                    {/* @ts-ignore */}
                    {formatPrice(transaction.price || null)}
                  </TableCell>
                  <TableCell className="text-right">
                    {transaction.value !== null && transaction.value !== undefined
                      ? `$${formatNumber(transaction.value)}`
                      : "N/A"}
                  </TableCell>
                  <TableCell className="text-right text-muted-foreground">
                    {/* @ts-ignore */}
                    {formatDate(transaction.start_date || transaction.filing_date || '')}
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

