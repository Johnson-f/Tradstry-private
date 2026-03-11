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

interface InsiderRosterProps {
  symbol: string;
  className?: string;
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

export function InsiderRoster({ symbol, className }: InsiderRosterProps) {
  const { holders, isLoading } = useHolders(
    {
      symbol,
      holder_type: "insider_roster",
    },
    !!symbol
  );

  const roster = holders?.insider_roster ?? [];

  if (isLoading) return <Loading className={className} />;

  if (!roster || roster.length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No insider roster data available.
      </div>
    );
  }

  // Since insider_roster is unknown[], we'll try to render it as a flexible table
  // that can handle various data structures
  const getTableHeaders = (): string[] => {
    if (roster.length === 0) return [];
    const firstItem = roster[0];
    if (typeof firstItem === "object" && firstItem !== null) {
      return Object.keys(firstItem);
    }
    return [];
  };

  const headers = getTableHeaders();

  if (headers.length === 0) {
    return (
      <div className={cn("text-sm text-muted-foreground", className)}>
        No structured insider roster data available.
      </div>
    );
  }

  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) return "N/A";
    if (typeof value === "string") return value;
    if (typeof value === "number") return value.toLocaleString();
    if (typeof value === "boolean") return value ? "Yes" : "No";
    if (value instanceof Date) {
      return value.toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
    }
    return String(value);
  };

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <ScrollArea className="w-full">
        <div className="p-4 sm:p-6">
          <Table>
            <TableHeader>
              <TableRow>
                {headers.map((header) => (
                  <TableHead key={header} className="capitalize">
                    {header.replace(/_/g, " ")}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {roster.map((item, index) => {
                if (typeof item !== "object" || item === null) {
                  return (
                    <TableRow key={index}>
                      <TableCell colSpan={headers.length} className="text-muted-foreground">
                        {formatValue(item)}
                      </TableCell>
                    </TableRow>
                  );
                }
                const rowData = item as Record<string, unknown>;
                return (
                  <TableRow key={index}>
                    {headers.map((header) => (
                      <TableCell key={header}>{formatValue(rowData[header])}</TableCell>
                    ))}
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
        <ScrollBar orientation="horizontal" />
      </ScrollArea>
    </div>
  );
}

