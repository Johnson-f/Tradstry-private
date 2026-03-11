"use client";

import React, { useMemo } from "react";
import { cn } from "@/lib/utils";
import { useFinancials, useQuote } from "@/lib/hooks/use-market-data-service";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

interface KeyStatsProps {
  symbol: string;
  className?: string;
  frequency?: "annual" | "quarterly"; // visual only
}

export function KeyStats({ symbol, className, frequency = "annual" }: KeyStatsProps) {
  // Fetch financial statements
  const income = useFinancials({ symbol, statement: "income", frequency: frequency === "annual" ? "annual" : "quarterly" }, !!symbol);
  const balance = useFinancials({ symbol, statement: "balance", frequency: frequency === "annual" ? "annual" : "quarterly" }, !!symbol);
  const cash = useFinancials({ symbol, statement: "cashflow", frequency: frequency === "annual" ? "annual" : "quarterly" }, !!symbol);
  const { quote } = useQuote(symbol, !!symbol);

  const isLoading = income.isLoading || balance.isLoading || cash.isLoading;

  const { periods, rows } = useMemo((): { periods: string[]; rows: { label: string; values: Array<number | string | null> }[] } => {
    // Helper to get row by fuzzy name
    type StatementContainer = { statement?: Record<string, Record<string, unknown>> } | null | undefined;
    const getStatement = (s: StatementContainer) => (s && typeof s === 'object' ? s.statement : undefined);

    const findRow = (stmt: StatementContainer, name: string): Record<string, unknown> | null => {
      const st = getStatement(stmt);
      if (!st) return null;
      const values = Object.values(st) as Record<string, unknown>[];
      const match = values.find((r) => typeof r?.Breakdown === "string" && normalize(r.Breakdown) === normalize(name))
        || values.find((r) => typeof r?.Breakdown === "string" && normalize(r.Breakdown).includes(normalize(name)));
      return match || null;
    };

    const collectPeriods = (stmt: StatementContainer): string[] => {
      const st = getStatement(stmt);
      if (!st) return [];
      const first = Object.values(st)[0] as Record<string, unknown> | undefined;
      if (!first) return [];
      return Object.keys(first)
        .filter((k) => k !== "Breakdown")
        .sort((a, b) => new Date(a).getTime() - new Date(b).getTime());
    };

    // Build unified period list (ascending), then take last 5
    const allPeriods = Array.from(new Set<string>([
      ...collectPeriods(income.financials),
      ...collectPeriods(balance.financials),
      ...collectPeriods(cash.financials),
    ])).sort((a, b) => new Date(a).getTime() - new Date(b).getTime());
    const periods = allPeriods.slice(-5);

    // Extract base metrics
    const revenueRow = findRow(income.financials, "Revenue") || findRow(income.financials, "Total Revenue");
    const grossRow = findRow(income.financials, "Gross Profit");
    const ebitdaRow = findRow(income.financials, "EBITDA");
    const netIncomeRow = findRow(income.financials, "Net Income");
    const epsDilutedRow = findRow(income.financials, "EPS Diluted") || findRow(income.financials, "Diluted EPS");

    const cashRow = findRow(balance.financials, "Cash") || findRow(balance.financials, "Cash and Cash Equivalents");
    const debtRow = findRow(balance.financials, "Debt") || findRow(balance.financials, "Total Debt");

    const ocfRow = findRow(cash.financials, "Operating Cash Flow");
    const capexRow = findRow(cash.financials, "Capital Expenditures");

    const dataRow = (label: string, getter: (p: string, idx: number) => number | string | null) => ({ label, values: periods.map((p, idx) => getter(p, idx)) });
    const val = (row: Record<string, unknown> | null, p: string) => {
      if (!row) return null;
      const direct = row[p];
      const alt = row[p.replace(/\//g, "-")];
      return (direct !== undefined && direct !== null) ? direct : (alt !== undefined && alt !== null ? alt : null);
    };

    // Derived helpers
    const marketCapRow = dataRow("Market Cap", () => quote?.marketCap ?? null);
    const cashVals = dataRow("- Cash", (p) => toNumber(val(cashRow, p)));
    const debtVals = dataRow("+ Debt", (p) => toNumber(val(debtRow, p)));
    const enterpriseValue = dataRow("Enterprise Value", (p, idx) => {
      const mc = toNumber(marketCapRow.values[idx]);
      const c = toNumber(cashVals.values[idx]);
      const d = toNumber(debtVals.values[idx]);
      if (mc == null) return null;
      return (mc - (c ?? 0) + (d ?? 0));
    });

    const revenue = dataRow("Revenue", (p) => toNumber(val(revenueRow, p)));
    const revenueGrowth = dataRow("% Growth", (_p, idx) => pctGrowth(revenue.values, idx));

    const grossProfit = dataRow("Gross Profit", (p) => toNumber(val(grossRow, p)));
    const grossMargin = dataRow("% Margin", (_p, idx) => pctMargin(grossProfit.values[idx], revenue.values[idx]));

    const ebitda = dataRow("EBITDA", (p) => toNumber(val(ebitdaRow, p)));
    const ebitdaMargin = dataRow("% Margin", (_p, idx) => pctMargin(ebitda.values[idx], revenue.values[idx]));

    const netIncome = dataRow("Net Income", (p) => toNumber(val(netIncomeRow, p)));
    const netMargin = dataRow("% Margin", (_p, idx) => pctMargin(netIncome.values[idx], revenue.values[idx]));

    const eps = dataRow("EPS Diluted", (p) => toNumber(val(epsDilutedRow, p)));
    const epsGrowth = dataRow("% Growth", (_p, idx) => pctGrowth(eps.values, idx));

    const ocf = dataRow("Operating Cash Flow", (p) => toNumber(val(ocfRow, p)));
    const capex = dataRow("Capital Expenditures", (p) => toNumber(val(capexRow, p)));
    const fcf = dataRow("Free Cash Flow", (_p, idx) => sumNumbers(ocf.values[idx], negateIfNumber(capex.values[idx])));

    const rows = [
      marketCapRow,
      cashVals,
      debtVals,
      enterpriseValue,
      revenue,
      revenueGrowth,
      grossProfit,
      grossMargin,
      ebitda,
      ebitdaMargin,
      netIncome,
      netMargin,
      eps,
      epsGrowth,
      ocf,
      capex,
      fcf,
    ];

    return { periods, rows };
  }, [income.financials, balance.financials, cash.financials, quote]);

  if (isLoading) return <Loading className={className} />;
  if (!rows?.length) return null;

  return (
    <div className={cn("rounded-2xl border bg-card/50 overflow-x-auto", className)}>
      <div className="p-4 sm:p-6">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[22%]">Key Stats</TableHead>
              {periods.map((p) => (
                <TableHead key={p} className="text-right whitespace-nowrap">{formatPeriod(p)}</TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((r, idx) => (
              <TableRow key={`${r.label}-${idx}`}>
                <TableCell className={cn("font-medium", isSectionHeader(r.label) && "text-foreground font-semibold")}>{r.label}</TableCell>
                {r.values.map((v: number | string | null, i: number) => (
                  <TableCell key={i} className="text-right tabular-nums">
                    {formatValue(r.label, v)}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

function Loading({ className }: { className?: string }) {
  return (
    <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
      <div className="space-y-3">
        <Skeleton className="h-6 w-36" />
        {Array.from({ length: 10 }).map((_, i) => (
          <div key={i} className="grid grid-cols-6 gap-3">
            <Skeleton className="h-4 w-36" />
            {Array.from({ length: 5 }).map((__, j) => (
              <Skeleton key={j} className="h-4 w-full" />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------- helpers ----------

function normalize(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

function toNumber(value: unknown): number | null {
  if (value == null) return null;
  const n = typeof value === "number" ? value : parseFloat(String(value));
  return Number.isFinite(n) ? n : null;
}

function pctGrowth(series: (number | string | null)[], idx: number): string | null {
  const current = toNumber(series[idx]);
  const prev = toNumber(series[idx - 1]);
  if (current == null || prev == null || prev === 0) return null;
  return ((current - prev) / Math.abs(prev) * 100).toFixed(1) + "%";
}

function pctMargin(value: unknown, revenue: unknown): string | null {
  const v = toNumber(value);
  const r = toNumber(revenue);
  if (v == null || r == null || r === 0) return null;
  return ((v / r) * 100).toFixed(1) + "%";
}

function sumNumbers(a: unknown, b: unknown): number | null {
  const x = toNumber(a);
  const y = toNumber(b);
  if (x == null && y == null) return null;
  return (x ?? 0) + (y ?? 0);
}

function negateIfNumber(v: unknown): number | null {
  const n = toNumber(v);
  return n == null ? null : -Math.abs(n);
}

function formatValue(label: string, v: unknown): React.ReactNode {
  if (label.includes("%")) {
    return typeof v === "string" ? v : v == null ? "-" : `${v}%`;
  }
  if (typeof v === "number") {
    // For money-like rows, use compact USD formatting
    const moneyRows = ["Market Cap", "Enterprise Value", "Revenue", "Gross Profit", "EBITDA", "Net Income", "Operating Cash Flow", "Capital Expenditures", "Free Cash Flow"];
    if (moneyRows.some((m) => label.startsWith(m))) {
      return formatCompactMoney(v);
    }
    return v.toLocaleString();
  }
  if (v == null) return "-";
  return String(v);
}

function formatCompactMoney(v: number): string {
  try {
    return "$" + new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 2 }).format(v);
  } catch {
    return "$" + v.toLocaleString();
  }
}

function formatPeriod(p: string): string {
  try {
    const d = new Date(p);
    if (!isNaN(d.getTime())) {
      return d.toLocaleDateString(undefined, { year: "numeric", month: "2-digit", day: "2-digit" });
    }
    return p;
  } catch {
    return p;
  }
}

function isSectionHeader(label: string): boolean {
  return ["Enterprise Value", "Free Cash Flow"].includes(label);
}

export default KeyStats;


