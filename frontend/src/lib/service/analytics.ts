import type { GraphQLFetcher } from "@/lib/client";
import type {
  AnalyticsTimeFilterInput,
  CalendarAnalytics,
  JournalAnalytics,
} from "@/lib/types/analytics";

const TRADE_OUTCOME_FIELDS = `
  symbol
  symbolName
  amount
`;

const JOURNAL_ANALYTICS_FIELDS = `
  winRate
  cumulativeProfit
  averageRiskToReward
  averageGain
  averageLoss
  profitFactor
  biggestWin {
    ${TRADE_OUTCOME_FIELDS}
  }
  biggestLoss {
    ${TRADE_OUTCOME_FIELDS}
  }
`;

const CALENDAR_DAY_FIELDS = `
  date
  profit
  tradeCount
  winRate
`;

const CALENDAR_WEEK_FIELDS = `
  weekIndex
  weekStart
  weekEnd
  profit
  tradeCount
  tradingDays
`;

const CALENDAR_ANALYTICS_FIELDS = `
  year
  month
  monthProfit
  tradeCount
  tradingDays
  gridStart
  gridEnd
  days {
    ${CALENDAR_DAY_FIELDS}
  }
  weeks {
    ${CALENDAR_WEEK_FIELDS}
  }
`;

const JOURNAL_ANALYTICS_QUERY = `
  query JournalAnalytics($accountId: String!, $timeFilter: AnalyticsTimeFilterInput!) {
    journalAnalytics(accountId: $accountId, timeFilter: $timeFilter) {
      ${JOURNAL_ANALYTICS_FIELDS}
    }
  }
`;

const CALENDAR_ANALYTICS_QUERY = `
  query CalendarAnalytics($accountId: String!, $year: Int!, $month: Int!) {
    calendarAnalytics(accountId: $accountId, year: $year, month: $month) {
      ${CALENDAR_ANALYTICS_FIELDS}
    }
  }
`;

export async function fetchJournalAnalytics(
  fetcher: GraphQLFetcher,
  accountId: string,
  timeFilter: AnalyticsTimeFilterInput,
): Promise<JournalAnalytics> {
  const data = await fetcher<{ journalAnalytics: JournalAnalytics }>(
    JOURNAL_ANALYTICS_QUERY,
    { accountId, timeFilter },
  );
  return data.journalAnalytics;
}

export async function fetchCalendarAnalytics(
  fetcher: GraphQLFetcher,
  accountId: string,
  year: number,
  month: number,
): Promise<CalendarAnalytics> {
  const data = await fetcher<{ calendarAnalytics: CalendarAnalytics }>(
    CALENDAR_ANALYTICS_QUERY,
    { accountId, year, month },
  );
  return data.calendarAnalytics;
}
