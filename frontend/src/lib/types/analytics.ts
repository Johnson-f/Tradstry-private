export const ANALYTICS_RANGES = [
  "LAST_7_DAYS",
  "LAST_30_DAYS",
  "YEAR_TO_DATE",
  "LAST_1_YEAR",
  "CUSTOM",
] as const;

export type AnalyticsRange = (typeof ANALYTICS_RANGES)[number];

export interface AnalyticsTimeFilterInput {
  range: AnalyticsRange;
  startDate?: string | null;
  endDate?: string | null;
}

export interface TradeOutcome {
  symbol: string;
  symbolName: string;
  amount: number;
}

export interface JournalAnalytics {
  winRate: number;
  cumulativeProfit: number;
  averageRiskToReward: number;
  averageGain: number;
  averageLoss: number;
  profitFactor: number | null;
  biggestWin: TradeOutcome | null;
  biggestLoss: TradeOutcome | null;
}

export interface CalendarDaySummary {
  date: string;
  profit: number;
  tradeCount: number;
  winRate: number;
}

export interface CalendarWeekSummary {
  weekIndex: number;
  weekStart: string;
  weekEnd: string;
  profit: number;
  tradeCount: number;
  tradingDays: number;
}

export interface CalendarAnalytics {
  year: number;
  month: number;
  monthProfit: number;
  tradeCount: number;
  tradingDays: number;
  gridStart: string;
  gridEnd: string;
  days: CalendarDaySummary[];
  weeks: CalendarWeekSummary[];
}
