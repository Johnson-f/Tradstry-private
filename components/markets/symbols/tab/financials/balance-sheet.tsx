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

interface BalanceSheetProps {
  symbol: string;
  className?: string;
  frequency?: "annual" | "quarterly";
}

export function BalanceSheet({ symbol, className, frequency = "annual" }: BalanceSheetProps) {
  const { financials, isLoading } = useFinancials(
    {
      symbol,
      statement: "balance",
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

    // Extract rows - Assets
    const assetsHeader = dataRow("Assets", () => null, { isHeader: true, bold: true });
    
    // Current Assets
    const cashRow = findRow(financials, "Cash and Cash Equivalents") || findRow(financials, "Cash & Equivalents");
    const shortTermInvRow = findRow(financials, "Short-Term Investments");
    const receivablesRow = findRow(financials, "Receivables") || findRow(financials, "Accounts Receivable");
    const inventoryRow = findRow(financials, "Inventory");
    const otherCurrAssetsRow = findRow(financials, "Other Current Assets") || findRow(financials, "Other Curr. Assets");

    const cash = dataRow("Cash & Equivalents", (p) => toNumber(val(cashRow, p)));
    const shortTermInv = dataRow("Short-Term Investments", (p) => toNumber(val(shortTermInvRow, p)));
    const receivables = dataRow("Receivables", (p) => toNumber(val(receivablesRow, p)));
    const inventory = dataRow("Inventory", (p) => toNumber(val(inventoryRow, p)));
    const otherCurrAssets = dataRow("Other Curr. Assets", (p) => toNumber(val(otherCurrAssetsRow, p)));

    // Calculate Total Current Assets
    const totalCurrAssets = dataRow("Total Curr. Assets", (_p, idx) => {
      const cashVal = toNumber(cash.values[idx]);
      const stInvVal = toNumber(shortTermInv.values[idx]);
      const recVal = toNumber(receivables.values[idx]);
      const invVal = toNumber(inventory.values[idx]);
      const otherVal = toNumber(otherCurrAssets.values[idx]);
      const sum = (cashVal ?? 0) + (stInvVal ?? 0) + (recVal ?? 0) + (invVal ?? 0) + (otherVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Non-Current Assets
    const ppeRow = findRow(financials, "Property, Plant & Equipment") || findRow(financials, "Property Plant & Equip (Net)");
    const goodwillRow = findRow(financials, "Goodwill");
    const intangiblesRow = findRow(financials, "Intangibles") || findRow(financials, "Intangible Assets");
    const longTermInvRow = findRow(financials, "Long-Term Investments");
    const taxAssetsRow = findRow(financials, "Tax Assets") || findRow(financials, "Deferred Tax Assets");
    const otherNCAssetsRow = findRow(financials, "Other Non-Current Assets") || findRow(financials, "Other NC Assets");

    const ppe = dataRow("Property Plant & Equip (Net)", (p) => toNumber(val(ppeRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const goodwill = dataRow("Goodwill", (p) => toNumber(val(goodwillRow, p)));
    const intangibles = dataRow("Intangibles", (p) => toNumber(val(intangiblesRow, p)));
    const longTermInv = dataRow("Long-Term Investments", (p) => toNumber(val(longTermInvRow, p)));
    const taxAssets = dataRow("Tax Assets", (p) => toNumber(val(taxAssetsRow, p)));
    const otherNCAssets = dataRow("Other NC Assets", (p) => toNumber(val(otherNCAssetsRow, p)), { formatter: (v) => formatNumber(v, 1) });

    // Calculate Total Non-Current Assets
    const totalNCAssets = dataRow("Total NC Assets", (_p, idx) => {
      const ppeVal = toNumber(ppe.values[idx]);
      const goodwillVal = toNumber(goodwill.values[idx]);
      const intangiblesVal = toNumber(intangibles.values[idx]);
      const longTermInvVal = toNumber(longTermInv.values[idx]);
      const taxAssetsVal = toNumber(taxAssets.values[idx]);
      const otherNCVal = toNumber(otherNCAssets.values[idx]);
      const sum = (ppeVal ?? 0) + (goodwillVal ?? 0) + (intangiblesVal ?? 0) + (longTermInvVal ?? 0) + (taxAssetsVal ?? 0) + (otherNCVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    const otherAssets = dataRow("Other Assets", (p) => toNumber(val(null, p))); // Placeholder, may not exist

    // Calculate Total Assets
    const totalAssets = dataRow("Total Assets", (_p, idx) => {
      const currAssetsVal = toNumber(totalCurrAssets.values[idx]);
      const ncAssetsVal = toNumber(totalNCAssets.values[idx]);
      const otherAssetsVal = toNumber(otherAssets.values[idx]);
      const sum = (currAssetsVal ?? 0) + (ncAssetsVal ?? 0) + (otherAssetsVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Liabilities
    const liabilitiesHeader = dataRow("Liabilities", () => null, { isHeader: true, bold: true });

    // Current Liabilities
    const payablesRow = findRow(financials, "Payables") || findRow(financials, "Accounts Payable");
    const shortTermDebtRow = findRow(financials, "Short-Term Debt");
    const taxPayableRow = findRow(financials, "Tax Payable") || findRow(financials, "Income Tax Payable");
    const defRevenueRow = findRow(financials, "Deferred Revenue") || findRow(financials, "Def. Revenue");
    const otherCurrLiabRow = findRow(financials, "Other Current Liabilities") || findRow(financials, "Other Curr. Liab.");

    const payables = dataRow("Payables", (p) => toNumber(val(payablesRow, p)));
    const shortTermDebt = dataRow("Short-Term Debt", (p) => toNumber(val(shortTermDebtRow, p)));
    const taxPayable = dataRow("Tax Payable", (p) => toNumber(val(taxPayableRow, p)));
    const defRevenue = dataRow("Def. Revenue", (p) => toNumber(val(defRevenueRow, p)));
    const otherCurrLiab = dataRow("Other Curr. Liab.", (p) => toNumber(val(otherCurrLiabRow, p)));

    // Calculate Total Current Liabilities
    const totalCurrLiab = dataRow("Total Curr. Liab.", (_p, idx) => {
      const payablesVal = toNumber(payables.values[idx]);
      const stdVal = toNumber(shortTermDebt.values[idx]);
      const taxPayableVal = toNumber(taxPayable.values[idx]);
      const defRevVal = toNumber(defRevenue.values[idx]);
      const otherCurrVal = toNumber(otherCurrLiab.values[idx]);
      const sum = (payablesVal ?? 0) + (stdVal ?? 0) + (taxPayableVal ?? 0) + (defRevVal ?? 0) + (otherCurrVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Non-Current Liabilities
    const ltDebtRow = findRow(financials, "Long-Term Debt") || findRow(financials, "LT Debt");
    const defRevNCRow = findRow(financials, "Deferred Revenue Non-Current") || findRow(financials, "Def. Rev. NC");
    const defTaxLiabNCRow = findRow(financials, "Deferred Tax Liability Non-Current") || findRow(financials, "Def. Tax Liab. NC");
    const otherNCLiabRow = findRow(financials, "Other Non-Current Liabilities") || findRow(financials, "Other NC Liab.");
    const otherLiabRow = findRow(financials, "Other Liabilities") || findRow(financials, "Other Liab.");
    const capLeasesRow = findRow(financials, "Capital Leases") || findRow(financials, "Cap. Leases");

    const ltDebt = dataRow("LT Debt", (p) => toNumber(val(ltDebtRow, p)));
    const defRevNC = dataRow("Def. Rev. NC", (p) => toNumber(val(defRevNCRow, p)));
    const defTaxLiabNC = dataRow("Def. Tax Liab. NC", (p) => toNumber(val(defTaxLiabNCRow, p)));
    const otherNCLiab = dataRow("Other NC Liab.", (p) => toNumber(val(otherNCLiabRow, p)));
    const otherLiab = dataRow("Other Liab.", (p) => toNumber(val(otherLiabRow, p)));
    const capLeases = dataRow("Cap. Leases", (p) => toNumber(val(capLeasesRow, p)), { formatter: (v) => formatNumber(v, 1) });

    // Calculate Total Non-Current Liabilities
    const totalNCLiab = dataRow("Total NC Liab", (_p, idx) => {
      const ltDebtVal = toNumber(ltDebt.values[idx]);
      const defRevNCVal = toNumber(defRevNC.values[idx]);
      const defTaxLiabNCVal = toNumber(defTaxLiabNC.values[idx]);
      const otherNCVal = toNumber(otherNCLiab.values[idx]);
      const otherLiabVal = toNumber(otherLiab.values[idx]);
      const capLeasesVal = toNumber(capLeases.values[idx]);
      const sum = (ltDebtVal ?? 0) + (defRevNCVal ?? 0) + (defTaxLiabNCVal ?? 0) + (otherNCVal ?? 0) + (otherLiabVal ?? 0) + (capLeasesVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Calculate Total Liabilities
    const totalLiab = dataRow("Total Liab.", (_p, idx) => {
      const currLiabVal = toNumber(totalCurrLiab.values[idx]);
      const ncLiabVal = toNumber(totalNCLiab.values[idx]);
      const sum = (currLiabVal ?? 0) + (ncLiabVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Equity
    const equityHeader = dataRow("Equity", () => null, { isHeader: true, bold: true });
    const prefStockRow = findRow(financials, "Preferred Stock") || findRow(financials, "Pref. Stock");
    const commonStockRow = findRow(financials, "Common Stock");
    const retEarningsRow = findRow(financials, "Retained Earnings") || findRow(financials, "Ret. Earnings");
    const aociRow = findRow(financials, "Accumulated Other Comprehensive Income") || findRow(financials, "AOCI");
    const otherEquityRow = findRow(financials, "Other Equity");

    const prefStock = dataRow("Pref. Stock", (p) => toNumber(val(prefStockRow, p)));
    const commonStock = dataRow("Common Stock", (p) => toNumber(val(commonStockRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const retEarnings = dataRow("Ret. Earnings", (p) => toNumber(val(retEarningsRow, p)));
    const aoci = dataRow("AOCI", (p) => toNumber(val(aociRow, p)), { formatter: (v) => formatNumber(v, 1) });
    const otherEquity = dataRow("Other Equity", (p) => toNumber(val(otherEquityRow, p)));

    // Calculate Total Equity
    const totalEquity = dataRow("Total Equity", (_p, idx) => {
      const prefStockVal = toNumber(prefStock.values[idx]);
      const commonStockVal = toNumber(commonStock.values[idx]);
      const retEarningsVal = toNumber(retEarnings.values[idx]);
      const aociVal = toNumber(aoci.values[idx]);
      const otherEquityVal = toNumber(otherEquity.values[idx]);
      const sum = (prefStockVal ?? 0) + (commonStockVal ?? 0) + (retEarningsVal ?? 0) + (aociVal ?? 0) + (otherEquityVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    // Supplemental Information
    const supplementalHeader = dataRow("Supplemental Information", () => null, { isHeader: true, bold: true });
    const minInterestRow = findRow(financials, "Minority Interest") || findRow(financials, "Min. Interest");
    const minInterest = dataRow("Min. Interest", (p) => toNumber(val(minInterestRow, p)));

    // Calculate Total Liabilities & Total Equity
    const totalLiabAndEquity = dataRow("Total Liab. & Tot. Equity", (_p, idx) => {
      const totalLiabVal = toNumber(totalLiab.values[idx]);
      const totalEquityVal = toNumber(totalEquity.values[idx]);
      const sum = (totalLiabVal ?? 0) + (totalEquityVal ?? 0);
      return sum === 0 ? null : sum;
    }, { bold: true });

    const rows = [
      assetsHeader,
      cash,
      shortTermInv,
      receivables,
      inventory,
      otherCurrAssets,
      totalCurrAssets,
      ppe,
      goodwill,
      intangibles,
      longTermInv,
      taxAssets,
      otherNCAssets,
      totalNCAssets,
      otherAssets,
      totalAssets,
      liabilitiesHeader,
      payables,
      shortTermDebt,
      taxPayable,
      defRevenue,
      otherCurrLiab,
      totalCurrLiab,
      ltDebt,
      defRevNC,
      defTaxLiabNC,
      otherNCLiab,
      otherLiab,
      capLeases,
      totalNCLiab,
      totalLiab,
      equityHeader,
      prefStock,
      commonStock,
      retEarnings,
      aoci,
      otherEquity,
      totalEquity,
      supplementalHeader,
      minInterest,
      totalLiabAndEquity,
    ];

    return { periods, rows };
  }, [financials]);

  if (isLoading) {
    return <Loading className={className} />;
  }

  if (!rows?.length || periods.length === 0) {
    return (
      <div className={cn("rounded-2xl border bg-card/50 p-6", className)}>
        <div className="text-sm text-muted-foreground">No balance sheet data available.</div>
      </div>
    );
  }

  return (
    <div className={cn("rounded-2xl border bg-card/50 overflow-x-auto", className)}>
      <div className="p-4 sm:p-6">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[22%]">Balance Sheet</TableHead>
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
        {Array.from({ length: 25 }).map((_, i) => (
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
    // Cash & Equivalents gets $ prefix
    if (label === "Cash & Equivalents") {
      return formatCompactMoney(v);
    }
    // All balance sheet items are monetary values
    return formatCompactMoney(v);
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

export default BalanceSheet;

