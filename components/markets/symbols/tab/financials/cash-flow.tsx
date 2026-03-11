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

interface CashFlowProps {
  symbol: string;
  className?: string;
  frequency?: "annual" | "quarterly";
}

export function CashFlow({ symbol, className, frequency = "annual" }: CashFlowProps) {
  const { financials, isLoading } = useFinancials(
    {
      symbol,
      statement: "cashflow",
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

    // Extract rows - Operating Activities
    const netIncomeRow = findRow(financials, "Net Income");
    const depAmortRow = findRow(financials, "Depreciation and Amortization") || findRow(financials, "Depreciation & Amortization");
    const deferredTaxRow = findRow(financials, "Deferred Tax");
    const stockCompRow = findRow(financials, "Stock-Based Compensation") || findRow(financials, "Stock Based Compensation");
    const changeWCRow = findRow(financials, "Change in Working Capital");
    const otherNonCashRow = findRow(financials, "Other Non-Cash") || findRow(financials, "Other Non-Cash Items");
    const operatingCFRow = findRow(financials, "Operating Cash Flow");

    // Investing Activities
    const ppeInvRow = findRow(financials, "PP&E Investment") || findRow(financials, "Property, Plant, & Equipment Investment") || findRow(financials, "Capital Expenditures");
    const netAcquisitionsRow = findRow(financials, "Net Acquisitions");
    const invPurchasesRow = findRow(financials, "Investment Purchases");
    const invSalesRow = findRow(financials, "Investment Sales/Maturities") || findRow(financials, "Investment Sales") || findRow(financials, "Inv. Sales/Matur.");
    const otherInvActRow = findRow(financials, "Other Investing Activities") || findRow(financials, "Other Inv. Act.");

    // Financing Activities
    const debtRepayRow = findRow(financials, "Debt Repayment") || findRow(financials, "Debt Repay.");
    const stockIssuedRow = findRow(financials, "Stock Issued") || findRow(financials, "Common Stock Issued");
    const stockRepurchRow = findRow(financials, "Stock Repurchases") || findRow(financials, "Stock Repurch.") || findRow(financials, "Share Repurchases");
    const dividendsRow = findRow(financials, "Dividends Paid");
    const otherFinActRow = findRow(financials, "Other Financing Activities") || findRow(financials, "Other Fin. Act.");

    // Other items
    const forexRow = findRow(financials, "Forex Effect") || findRow(financials, "Foreign Exchange Effect");
    const netChangeCashRow = findRow(financials, "Net Change in Cash") || findRow(financials, "Net Chg. in Cash");

    // Build rows
    const operatingHeader = dataRow("Operating Activities", () => null, { isHeader: true, bold: true });
    const netIncome = dataRow("Net Income", (p) => toNumber(val(netIncomeRow, p)));
    const depAmort = dataRow("Dep. & Amort.", (p) => toNumber(val(depAmortRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const deferredTax = dataRow("Deferred Tax", (p) => toNumber(val(deferredTaxRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const stockComp = dataRow("Stock-Based Comp.", (p) => toNumber(val(stockCompRow, p)));
    const changeWC = dataRow("Change in WC", (p) => toNumber(val(changeWCRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const otherNonCash = dataRow("Other Non-Cash", (p) => toNumber(val(otherNonCashRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const operatingCF = dataRow("Operating Cash Flow", (p) => toNumber(val(operatingCFRow, p)), { bold: true });

    const investingHeader = dataRow("Investing Activities", () => null, { isHeader: true, bold: true });
    const ppeInv = dataRow("PP&E Inv.", (p) => toNumber(val(ppeInvRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const netAcquisitions = dataRow("Net Acquisitions", (p) => toNumber(val(netAcquisitionsRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const invPurchases = dataRow("Inv. Purchases", (p) => toNumber(val(invPurchasesRow, p)));
    const invSales = dataRow("Inv. Sales/Matur.", (p) => toNumber(val(invSalesRow, p)));
    const otherInvAct = dataRow("Other Inv. Act.", (p) => toNumber(val(otherInvActRow, p)), { formatter: (v) => formatNumber(v, 1) });
    
    // Calculate Investing Cash Flow as sum
    const investingCF = dataRow("Investing Cash Flow", (_p, idx) => {
      const ppeVal = toNumber(ppeInv.values[idx]);
      const netAcqVal = toNumber(netAcquisitions.values[idx]);
      const invPurchVal = toNumber(invPurchases.values[idx]);
      const invSalesVal = toNumber(invSales.values[idx]);
      const otherInvVal = toNumber(otherInvAct.values[idx]);
      const sum = (ppeVal ?? 0) + (netAcqVal ?? 0) + (invPurchVal ?? 0) + (invSalesVal ?? 0) + (otherInvVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    const financingHeader = dataRow("Financing Activities", () => null, { isHeader: true, bold: true });
    const debtRepay = dataRow("Debt Repay.", (p) => toNumber(val(debtRepayRow, p)));
    const stockIssued = dataRow("Stock Issued", (p) => toNumber(val(stockIssuedRow, p)));
    const stockRepurch = dataRow("Stock Repurch.", (p) => toNumber(val(stockRepurchRow, p)));
    const dividends = dataRow("Dividends Paid", (p) => toNumber(val(dividendsRow, p)));
    const otherFinAct = dataRow("Other Fin. Act.", (p) => toNumber(val(otherFinActRow, p)));

    // Calculate Financing Cash Flow as sum
    const financingCF = dataRow("Financing Cash Flow", (_p, idx) => {
      const debtVal = toNumber(debtRepay.values[idx]);
      const stockIssuedVal = toNumber(stockIssued.values[idx]);
      const stockRepurchVal = toNumber(stockRepurch.values[idx]);
      const dividendsVal = toNumber(dividends.values[idx]);
      const otherFinVal = toNumber(otherFinAct.values[idx]);
      const sum = (debtVal ?? 0) + (stockIssuedVal ?? 0) + (stockRepurchVal ?? 0) + (dividendsVal ?? 0) + (otherFinVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    const forex = dataRow("Forex Effect", (p) => toNumber(val(forexRow, p)));
    const netChangeCash = dataRow("Net Chg. in Cash", (p) => toNumber(val(netChangeCashRow, p)), { bold: true });

    const supplementalHeader = dataRow("Supplemental Information", () => null, { isHeader: true, bold: true });
    const supplementalOCF = dataRow("Operating Cash Flow", (p) => toNumber(val(operatingCFRow, p)));

    const rows = [
      operatingHeader,
      netIncome,
      depAmort,
      deferredTax,
      stockComp,
      changeWC,
      otherNonCash,
      operatingCF,
      investingHeader,
      ppeInv,
      netAcquisitions,
      invPurchases,
      invSales,
      otherInvAct,
      investingCF,
      financingHeader,
      debtRepay,
      stockIssued,
      stockRepurch,
      dividends,
      otherFinAct,
      financingCF,
      forex,
      netChangeCash,
      supplementalHeader,
      supplementalOCF,
    ];

    return { periods, rows };
  }, [financials]);

  if (isLoading) {
    return <Loading className={className} />;
  }

  if (!rows?.length || periods.length === 0) {
    return (
      <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
        <div className="text-sm text-muted-foreground">No cash flow data available.</div>
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50 overflow-x-auto", className)}>
      <div className="p-4 sm:p-6">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[22%]">Cash Flow</TableHead>
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
        {Array.from({ length: 20 }).map((_, i) => (
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
  if (typeof v === "number") {
    const moneyRows = [
      "Net Income", "Dep. & Amort.", "Deferred Tax", "Stock-Based Comp.", "Change in WC", "Other Non-Cash",
      "Operating Cash Flow", "PP&E Inv.", "Net Acquisitions", "Inv. Purchases", "Inv. Sales/Matur.", "Other Inv. Act.",
      "Investing Cash Flow", "Debt Repay.", "Stock Issued", "Stock Repurch.", "Dividends Paid", "Other Fin. Act.",
      "Financing Cash Flow", "Forex Effect", "Net Chg. in Cash"
    ];
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

export default CashFlow;

