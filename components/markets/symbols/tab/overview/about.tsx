"use client";

import React, { useMemo } from "react";
import { useDetailedQuote } from "@/lib/hooks/use-market-data-service";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableRow,
} from "@/components/ui/table";

interface AboutProps {
  symbol: string;
  className?: string;
}

export function About({ symbol, className }: AboutProps) {
  const { quote, isLoading } = useDetailedQuote(symbol, !!symbol);

  const data = useMemo(() => {
    if (!quote) return null;

    const dayRange = joinRange(toNumber(quote.low), toNumber(quote.high));
    const range52W = joinRange(toNumber(quote.yearLow), toNumber(quote.yearHigh));

    return {
      prevClose: toNumber(quote.previousClose),
      open: toNumber(quote.open),
      dayRange,
      range52W,
      marketCap: quote.marketCap ?? undefined,
      pe: quote.pe ?? undefined,
      dividendYield: quote.dividendYield ?? undefined,
      volume: quote.volume ?? undefined,
      eps: toNumber(quote.eps),
    };
  }, [quote]);

  if (isLoading) return <Loading className={className} />;
  if (!data) return null;

  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <div className="p-5 sm:p-6">
        <Table>
          <TableBody>
            <TableRow>
              <LabelCell>Prev Close</LabelCell>
              <ValueCell>{formatMoney(data.prevClose)}</ValueCell>
              <LabelCell>52W Range</LabelCell>
              <ValueCell>{data.range52W || "—"}</ValueCell>
              <LabelCell>Market Cap</LabelCell>
              <ValueCell className="!text-right">{data.marketCap ?? "—"}</ValueCell>
            </TableRow>
            <TableRow>
              <LabelCell>Open</LabelCell>
              <ValueCell>{formatMoney(data.open)}</ValueCell>
              <LabelCell>P/E Ratio</LabelCell>
              <ValueCell>{data.pe !== undefined ? formatNumber(data.pe) : "—"}</ValueCell>
              <LabelCell>Dividend Yield</LabelCell>
              <ValueCell className="!text-right">{data.dividendYield ?? "—"}</ValueCell>
            </TableRow>
            <TableRow>
              <LabelCell>Day Range</LabelCell>
              <ValueCell>{data.dayRange || "—"}</ValueCell>
              <LabelCell>Volume</LabelCell>
              <ValueCell>{data.volume !== undefined ? formatCompactNumber(data.volume) : "—"}</ValueCell>
              <LabelCell>EPS</LabelCell>
              <ValueCell className="!text-right">{data.eps !== undefined ? formatMoney(data.eps) : "—"}</ValueCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

function LabelCell({ children }: { children: React.ReactNode }) {
  return (
    <TableCell className="w-[20%] text-sm text-muted-foreground align-middle">{children}</TableCell>
  );
}

function ValueCell({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <TableCell className={cn("w-[13.333%] text-sm sm:text-base font-medium align-middle", className)}>
      {children}
    </TableCell>
  );
}

function Loading({ className }: { className?: string }) {
  return (
    <div className={cn("rounded-2xl border bg-card/50", className)}>
      <div className="p-5 sm:p-6 space-y-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="grid grid-cols-6 gap-4">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-4 w-20" />
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-4 w-28" />
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-4 w-20 ml-auto" />
          </div>
        ))}
      </div>
    </div>
  );
}

function toNumber(value?: string | number | null): number | undefined {
  if (value === undefined || value === null) return undefined;
  const num = typeof value === "number" ? value : parseFloat(String(value));
  return Number.isFinite(num) ? num : undefined;
}

function formatMoney(value?: number) {
  if (value === undefined) return "—";
  return `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function joinRange(min?: number, max?: number) {
  if (min === undefined || max === undefined) return "";
  return `${formatMoney(min)} - ${formatMoney(max)}`;
}

function formatCompactNumber(value: number) {
  try {
    return new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 2 }).format(value);
  } catch {
    return value.toLocaleString();
  }
}

function formatNumber(value: number | string) {
  if (typeof value === "string") {
    const num = parseFloat(value);
    return Number.isFinite(num) ? num.toLocaleString(undefined, { maximumFractionDigits: 2 }) : value;
  }
  return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
}

export default About;


