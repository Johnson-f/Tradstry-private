"use client";

import React, { useMemo } from "react";
import { cn } from "@/lib/utils";
import { useQuote, useMarketHours } from "@/lib/hooks/use-market-data-service";
import { Skeleton } from "@/components/ui/skeleton";
import { ArrowUpRight, ArrowDownRight, Sun, Moon } from "lucide-react";

interface PriceProps {
  symbol: string;
  className?: string;
}

export function Price({ symbol, className }: PriceProps) {
  const { quote, isLoading } = useQuote(symbol, !!symbol);
  const { hours } = useMarketHours(true);

  const isMarketOpen = (hours?.status || "").toLowerCase() === "open";

  const parsed = useMemo(() => {
    if (!quote) return null;
    const price = toNumber(quote.price);
    const change = toNumber(quote.change);
    const pct = toNumber(quote.percentChange);
    const after = toNumber(quote.afterHoursPrice);
    return { price, change, pct, after };
  }, [quote]);

  if (isLoading) {
    return <Loading />;
  }

  if (!quote || !parsed) {
    return null;
  }

  const positive = (parsed.pct ?? 0) > 0;
  const negative = (parsed.pct ?? 0) < 0;
  const arrow = positive ? <ArrowUpRight className="h-4 w-4 inline align-[-2px]" /> : negative ? <ArrowDownRight className="h-4 w-4 inline align-[-2px]" /> : null;
  const pctColor = positive ? "text-emerald-400" : negative ? "text-red-500" : "text-muted-foreground";

  const primaryCard = (
    <div className="rounded-2xl border bg-card/50 px-4 py-3 sm:px-5 sm:py-4">
      <div className="flex items-baseline gap-3">
        <div className="text-2xl sm:text-3xl font-semibold">{formatMoney(parsed.price)}</div>
        <div className="text-base sm:text-lg text-muted-foreground">{formatSignedMoney(parsed.change)}</div>
        <div className={cn("text-base sm:text-lg font-medium", pctColor)}>
          {arrow} {parsed.pct !== undefined && !Number.isNaN(parsed.pct) ? `${formatPercent(parsed.pct)} Today` : "—"}
        </div>
      </div>
      <div className="mt-2 text-sm text-muted-foreground flex items-center gap-2">
        <span>{formatTimestamp(new Date())}</span>
        {isMarketOpen ? <Sun className="h-4 w-4 text-amber-400" /> : <Moon className="h-4 w-4" />}
      </div>
    </div>
  );

  const showAfter = !isMarketOpen && parsed.after !== undefined && !Number.isNaN(parsed.after);
  const afterCard = showAfter ? (
    <div className="rounded-2xl border bg-card/50 px-4 py-3 sm:px-5 sm:py-4">
      <div className="flex items-baseline gap-3">
        <div className="text-2xl sm:text-3xl font-semibold">{formatMoney(parsed.after)}</div>
        <div className="text-sm text-muted-foreground">After Hours</div>
      </div>
      <div className="mt-2 text-sm text-muted-foreground flex items-center gap-2">
        <span>{formatTimestamp(new Date())}</span>
        <Moon className="h-4 w-4" />
      </div>
    </div>
  ) : null;

  return (
    <div className={cn("flex flex-col sm:flex-row items-stretch gap-3", className)}>
      {primaryCard}
      {afterCard}
    </div>
  );
}

function Loading() {
  return (
    <div className="rounded-2xl border bg-card/50 px-5 py-4 inline-flex gap-6 items-center">
      <Skeleton className="h-7 w-24" />
      <Skeleton className="h-5 w-16" />
      <Skeleton className="h-5 w-28" />
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

function formatSignedMoney(value?: number) {
  if (value === undefined) return "—";
  const sign = value > 0 ? "+" : value < 0 ? "" : "";
  return `${sign}${formatMoney(Math.abs(value))}`;
}

function formatPercent(value?: number) {
  if (value === undefined) return "—";
  return `${value.toFixed(2)}%`;
}

export function formatTimestamp(date: Date) {
  // Example: Nov 5, 10:33:01 AM EST
  try {
    const opts: Intl.DateTimeFormatOptions = {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
      second: "2-digit",
      hour12: true,
      timeZoneName: "short",
    };
    const formatter = new Intl.DateTimeFormat(undefined, opts);
    const parts = formatter.formatToParts(date);
    const map = Object.fromEntries(parts.map(p => [p.type, p.value]));
    return `${map.month} ${map.day}, ${map.hour}:${map.minute}:${map.second} ${map.dayPeriod?.toUpperCase?.() || ""} ${map.timeZoneName}`.replace(/\s+/g, " ").trim();
  } catch {
    return date.toLocaleString();
  }
}

interface PricePropsWithTimeframe extends PriceProps {
  timeframePeriodChange?: number | null;
  timeframeLabel?: string;
}

export function PriceWithTimeframe({ symbol, className, timeframePeriodChange, timeframeLabel }: PricePropsWithTimeframe) {
  const { quote, isLoading } = useQuote(symbol, !!symbol);
  const { hours } = useMarketHours(true);

  const isMarketOpen = (hours?.status || "").toLowerCase() === "open";

  const parsed = useMemo(() => {
    if (!quote) return null;
    const price = toNumber(quote.price);
    const change = toNumber(quote.change);
    const pct = timeframePeriodChange !== undefined && timeframePeriodChange !== null 
      ? timeframePeriodChange 
      : toNumber(quote.percentChange);
    const after = toNumber(quote.afterHoursPrice);
    return { price, change, pct, after };
  }, [quote, timeframePeriodChange]);

  if (isLoading) {
    return <Loading />;
  }

  if (!quote || !parsed) {
    return null;
  }

  const positive = (parsed.pct ?? 0) > 0;
  const negative = (parsed.pct ?? 0) < 0;
  const arrow = positive ? <ArrowUpRight className="h-4 w-4 inline align-[-2px]" /> : negative ? <ArrowDownRight className="h-4 w-4 inline align-[-2px]" /> : null;
  const pctColor = positive ? "text-emerald-400" : negative ? "text-red-500" : "text-muted-foreground";

  const periodLabel = timeframeLabel || "Today";

  const primaryCard = (
    <div className="rounded-2xl border bg-card/50 px-4 py-3 sm:px-5 sm:py-4">
      <div className="flex items-baseline gap-3">
        <div className="text-2xl sm:text-3xl font-semibold">{formatMoney(parsed.price)}</div>
        <div className="text-base sm:text-lg text-muted-foreground">{formatSignedMoney(parsed.change)}</div>
        <div className={cn("text-base sm:text-lg font-medium", pctColor)}>
          {arrow} {parsed.pct !== undefined && !Number.isNaN(parsed.pct) ? `${formatPercent(parsed.pct)} ${periodLabel}` : "—"}
        </div>
      </div>
      <div className="mt-2 text-sm text-muted-foreground flex items-center gap-2">
        <span>{formatTimestamp(new Date())}</span>
        {isMarketOpen ? <Sun className="h-4 w-4 text-amber-400" /> : <Moon className="h-4 w-4" />}
      </div>
    </div>
  );

  const showAfter = !isMarketOpen && parsed.after !== undefined && !Number.isNaN(parsed.after);
  const afterCard = showAfter ? (
    <div className="rounded-2xl border bg-card/50 px-4 py-3 sm:px-5 sm:py-4">
      <div className="flex items-baseline gap-3">
        <div className="text-2xl sm:text-3xl font-semibold">{formatMoney(parsed.after)}</div>
        <div className="text-sm text-muted-foreground">After Hours</div>
      </div>
      <div className="mt-2 text-sm text-muted-foreground flex items-center gap-2">
        <span>{formatTimestamp(new Date())}</span>
        <Moon className="h-4 w-4" />
      </div>
    </div>
  ) : null;

  return (
    <div className={cn("flex flex-col sm:flex-row items-stretch gap-3", className)}>
      {primaryCard}
      {afterCard}
    </div>
  );
}

export default Price;
