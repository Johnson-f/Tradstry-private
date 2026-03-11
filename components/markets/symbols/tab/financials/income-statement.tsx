"use client";

import React, { useMemo } from "react";
import { cn } from "@/lib/utils";
import { useFinancials } from "@/lib/hooks/use-market-data-service";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

interface IncomeStatementProps {
  symbol: string;
  className?: string;
  frequency?: "annual" | "quarterly";
}

export function IncomeStatement({ symbol, className, frequency = "annual" }: IncomeStatementProps) {
  const { financials, isLoading } = useFinancials(
    {
      symbol,
      statement: "income",
      frequency: frequency === "annual" ? "annual" : "quarterly",
    },
    !!symbol
  );

  const { periods, rows } = useMemo((): {
    periods: string[];
    rows: Array<{
      label: string;
      values: Array<number | string | null>;
      indent?: boolean;
      bold?: boolean;
      isHeader?: boolean;
      formatter?: (val: number | string | null) => string;
    }>;
  } => {
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

    const periods = collectPeriods(financials).slice(-4); // Last 4 periods

    const val = (row: Record<string, unknown> | null, p: string) => {
      if (!row) return null;
      const direct = row[p];
      const alt = row[p.replace(/\//g, "-")];
      return (direct !== undefined && direct !== null) ? direct : (alt !== undefined && alt !== null ? alt : null);
    };

    const dataRow = (
      label: string,
      getter: (p: string, idx: number) => number | string | null,
      options?: { indent?: boolean; bold?: boolean; isHeader?: boolean; formatter?: (val: number | string | null) => string }
    ) => ({
      label,
      values: periods.map((p, idx) => getter(p, idx)),
      ...options,
    });

    // Extract rows
    const revenueRow = findRow(financials, "Revenue") || findRow(financials, "Total Revenue");
    const cogsRow = findRow(financials, "Cost of Goods Sold") || findRow(financials, "COGS");
    const grossRow = findRow(financials, "Gross Profit");
    const rdRow = findRow(financials, "Research and Development") || findRow(financials, "R&D Expenses");
    const gaRow = findRow(financials, "General and Administrative") || findRow(financials, "G&A Expenses");
    const sgaRow = findRow(financials, "Selling, General and Administrative") || findRow(financials, "SG&A Expenses");
    const salesMktgRow = findRow(financials, "Sales & Marketing") || findRow(financials, "Sales & Mktg Exp");
    const otherOpExpRow = findRow(financials, "Other Operating Expenses");
    const operatingIncomeRow = findRow(financials, "Operating Income");
    const otherIncomeRow = findRow(financials, "Other Income/Expense") || findRow(financials, "Other Income/Exp. Net");
    const preTaxRow = findRow(financials, "Pretax Income") || findRow(financials, "Pre-Tax Income");
    const taxRow = findRow(financials, "Tax Expense") || findRow(financials, "Income Tax Expense");
    const netIncomeRow = findRow(financials, "Net Income");
    const epsRow = findRow(financials, "EPS") || findRow(financials, "Earnings Per Share");
    const epsDilutedRow = findRow(financials, "EPS Diluted") || findRow(financials, "Diluted EPS");
    const sharesRow = findRow(financials, "Weighted Average Shares Outstanding") || findRow(financials, "Weighted Avg Shares Out");
    const sharesDilRow = findRow(financials, "Weighted Average Shares Outstanding Diluted") || findRow(financials, "Weighted Avg Shares Out Dil");
    const interestIncomeRow = findRow(financials, "Interest Income");

    const revenue = dataRow("Revenue", (p) => toNumber(val(revenueRow, p)));
    const revenueGrowth = dataRow("% Growth", (_p, idx) => pctGrowth(revenue.values, idx), { indent: true });
    const cogs = dataRow("Cost of Goods Sold", (p) => toNumber(val(cogsRow, p)));
    const grossProfit = dataRow("Gross Profit", (p) => toNumber(val(grossRow, p)), { bold: true });
    const grossMargin = dataRow("% Margin", (_p, idx) => pctMargin(grossProfit.values[idx], revenue.values[idx]), { indent: true });
    const rd = dataRow("R&D Expenses", (p) => toNumber(val(rdRow, p)));
    const ga = dataRow("G&A Expenses", (p) => toNumber(val(gaRow, p)));
    const sga = dataRow("SG&A Expenses", (p) => toNumber(val(sgaRow, p)));
    const salesMktg = dataRow("Sales & Mktg Exp.", (p) => toNumber(val(salesMktgRow, p)), { indent: true });
    const otherOpExp = dataRow("Other Operating Expenses", (p) => toNumber(val(otherOpExpRow, p)), { indent: true });

    // Calculate Operating Expenses as sum
    const operatingExpenses = dataRow("Operating Expenses", (_p, idx) => {
      const rdVal = toNumber(rd.values[idx]);
      const gaVal = toNumber(ga.values[idx]);
      const sgaVal = toNumber(sga.values[idx]);
      const salesMktgVal = toNumber(salesMktg.values[idx]);
      const otherOpExpVal = toNumber(otherOpExp.values[idx]);
      const sum = (rdVal ?? 0) + (gaVal ?? 0) + (sgaVal ?? 0) + (salesMktgVal ?? 0) + (otherOpExpVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    const operatingIncome = dataRow("Operating Income", (p) => toNumber(val(operatingIncomeRow, p)), { bold: true });
    const operatingMargin = dataRow("% Margin", (_p, idx) => pctMargin(operatingIncome.values[idx], revenue.values[idx]), { indent: true });
    const otherIncome = dataRow("Other Income/Exp. Net", (p) => toNumber(val(otherIncomeRow, p)));
    const preTax = dataRow("Pre-Tax Income", (p) => toNumber(val(preTaxRow, p)));
    const tax = dataRow("Tax Expense", (p) => toNumber(val(taxRow, p)));
    const netIncome = dataRow("Net Income", (p) => toNumber(val(netIncomeRow, p)), { bold: true });
    const netMargin = dataRow("% Margin", (_p, idx) => pctMargin(netIncome.values[idx], revenue.values[idx]), { indent: true });
    const eps = dataRow("EPS", (p) => toNumber(val(epsRow, p)), { formatter: (v) => formatNumber(v, 2) });
    const epsGrowth = dataRow("% Growth", (_p, idx) => pctGrowth(eps.values, idx), { indent: true });
    const epsDiluted = dataRow("EPS Diluted", (p) => toNumber(val(epsDilutedRow, p)), { formatter: (v) => formatNumber(v, 2) });
    const shares = dataRow("Weighted Avg Shares Out", (p) => toNumber(val(sharesRow, p)), { formatter: (v) => formatNumber(v, 0) });
    const sharesDil = dataRow("Weighted Avg Shares Out Dil", (p) => toNumber(val(sharesDilRow, p)), { formatter: (v) => formatNumber(v, 0) });
    const supplementalHeader = dataRow("Supplemental Information", () => null, { isHeader: true, bold: true });
    const interestIncome = dataRow("Interest Income", (p) => toNumber(val(interestIncomeRow, p)), { formatter: (v) => formatNumber(v, 1) });

    const rows = [
      revenue,
      revenueGrowth,
      cogs,
      grossProfit,
      grossMargin,
      rd,
      ga,
      sga,
      salesMktg,
      otherOpExp,
      operatingExpenses,
      operatingIncome,
      operatingMargin,
      otherIncome,
      preTax,
      tax,
      netIncome,
      netMargin,
      eps,
      epsGrowth,
      epsDiluted,
      shares,
      sharesDil,
      supplementalHeader,
      interestIncome,
    ];

    return { periods, rows };
  }, [financials]);

  if (isLoading) {
    return <Loading className={className} />;
  }

  if (!rows?.length || periods.length === 0) {
    return (
      <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
        <div className="text-sm text-muted-foreground">No income statement data available.</div>
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50 overflow-x-auto", className)}>
      <div className="p-4 sm:p-6">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[22%]">Income Statement</TableHead>
              {periods.map((p) => (
                <TableHead key={p} className="text-right whitespace-nowrap">
                  {formatPeriod(p)}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((r, idx) => (
              <TableRow key={`${r.label}-${idx}`} className={cn(r.isHeader && "bg-muted/20 hover:bg-muted/20")}>
                <TableCell className={cn("font-medium", r.indent && "pl-8", r.bold && "font-semibold", r.isHeader && "font-semibold text-base")}>
                  {r.label}
                </TableCell>
                {r.values.map((v: number | string | null, i: number) => (
                  <TableCell key={i} className="text-right tabular-nums">
                    {r.formatter ? r.formatter(v) : formatValue(r.label, v)}
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
        {Array.from({ length: 15 }).map((_, i) => (
          <div key={i} className="grid grid-cols-5 gap-3">
            <Skeleton className="h-4 w-48" />
            {Array.from({ length: 4 }).map((__, j) => (
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
  if (idx === 0) return null; // No growth for first period
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

function formatNumber(value: number | string | null | undefined, decimals = 0): string {
  if (value == null) return "—";
  const n = typeof value === "number" ? value : parseFloat(String(value));
  if (!Number.isFinite(n)) return "—";
  return n.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

function formatValue(label: string, v: unknown): string {
  if (label.includes("%")) {
    return typeof v === "string" ? v : v == null ? "—" : `${v}%`;
  }
  if (typeof v === "number") {
    const moneyRows = ["Revenue", "Cost of Goods Sold", "Gross Profit", "R&D Expenses", "G&A Expenses", "SG&A Expenses", "Sales & Mktg Exp.", "Other Operating Expenses", "Operating Expenses", "Operating Income", "Other Income/Exp. Net", "Pre-Tax Income", "Tax Expense", "Net Income", "Interest Income"];
    if (moneyRows.some((m) => label.startsWith(m))) {
      return formatCompactMoney(v);
    }
    return v.toLocaleString();
  }
  if (v == null) return "—";
  return String(v);
}

function formatCompactMoney(v: number): string {
  try {
    return "$" + new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(v);
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

export default IncomeStatement;

