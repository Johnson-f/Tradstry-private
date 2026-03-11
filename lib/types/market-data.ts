// =====================================================
// Market Engine Types (from backend/src/service/market_engine)
// =====================================================

/**
 * Health status response
 */
export interface HealthStatus {
  status: string;
}

/**
 * Market hours information
 */
export interface MarketHours {
  status: string;
  reason?: string | null;
  timestamp: string;
}

/**
 * Company officer (matches backend CompanyOfficer)
 */
export interface CompanyOfficer {
  name?: string | null;
  title?: string | null;
  age?: number | null;
  yearBorn?: number | null;
  totalPay?: {
    raw?: number | null;
    fmt?: string | null;
    longFmt?: string | null;
  } | null;
}

/**
 * Asset profile (matches backend AssetProfile)
 */
export interface AssetProfile {
  address1?: string | null;
  city?: string | null;
  state?: string | null;
  zip?: string | null;
  country?: string | null;
  phone?: string | null;
  website?: string | null;
  industry?: string | null;
  industryDisp?: string | null;
  sector?: string | null;
  sectorDisp?: string | null;
  longBusinessSummary?: string | null;
  fullTimeEmployees?: number | null;
  companyOfficers?: CompanyOfficer[] | null;
}

/**
 * Quote data for a symbol (detailed)
 */
export interface Quote {
  symbol: string;
  name?: string | null;
  price?: string | null;
  afterHoursPrice?: string | null;
  change?: string | null;
  percentChange?: string | null;
  open?: string | null;
  high?: string | null;
  low?: string | null;
  yearHigh?: string | null;
  yearLow?: string | null;
  volume?: number | null;
  avgVolume?: number | null;
  marketCap?: string | null;
  beta?: string | null;
  pe?: string | null;
  previousClose?: string | null;
  dividendYield?: string | null;
  eps?: string | null;
  earningsDate?: string | null;
  sector?: string | null;
  industry?: string | null;
  about?: string | null;
  employees?: string | null;
  fiveDaysReturn?: string | null;
  oneMonthReturn?: string | null;
  threeMonthReturn?: string | null;
  sixMonthReturn?: string | null;
  ytdReturn?: string | null;
  yearReturn?: string | null;
  threeYearReturn?: string | null;
  fiveYearReturn?: string | null;
  tenYearReturn?: string | null;
  maxReturn?: string | null;
  logo?: string | null;
  // Company info from asset profile
  assetProfile?: AssetProfile | null;
}

/**
 * Simple quote data (summary)
 */
export interface SimpleQuote {
  symbol: string;
  name?: string | null;
  price?: string | null;
  afterHoursPrice?: string | null;
  change?: string | null;
  percentChange?: string | null;
  logo?: string | null;
}

// Logo URL for a symbol
export type LogoUrl = string | null;

/**
 * Candle data for historical prices
 */
export interface Candle {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
  adjClose?: number | null;
  volume?: number | null;
}

/**
 * Historical data response
 */
export interface HistoricalResponse {
  symbol: string;
  interval?: string | null;
  candles: Candle[];
}

/**
 * Market mover item (gainer, loser, or most active)
 */
export interface MoverItem {
  symbol: string;
  name?: string | null;
  price?: string | null;
  change?: string | null;
  percentChange?: string | null;
}

/**
 * Market movers response
 */
export interface MoversResponse {
  gainers: MoverItem[];
  losers: MoverItem[];
  mostActive: MoverItem[];
}

/**
 * News item (matches backend NewsArticle)
 */
export interface NewsItem {
  uuid?: string | null;
  title: string;
  publisher?: string | null;
  link: string;
  publish_time?: number | null;
  publish_time_formatted?: string | null;
  thumbnail_url?: string | null;
  related_tickers?: string[];
}

/**
 * News response (matches backend NewsResponse)
 */
export interface NewsResponse {
  symbol?: string | null;
  query?: string | null;
  articles: NewsItem[];
  count: number;
}

/**
 * Index item
 */
export interface IndexItem {
  name: string;
  price?: string | null;
  change?: string | null;
  percentChange?: string | null;
}

/**
 * Sector performance item
 */
export interface SectorPerformanceItem {
  sector: string;
  dayReturn?: string | null;
  ytdReturn?: string | null;
  yearReturn?: string | null;
  threeYearReturn?: string | null;
  fiveYearReturn?: string | null;
}

/**
 * Search result item
 */
export interface SearchItem {
  symbol: string;
  name?: string | null;
  type?: string | null;
  exchange?: string | null;
}

/**
 * Indicator data point
 */
export interface IndicatorPoint {
  time: string;
  value: number;
}

/**
 * Indicator series response
 */
export interface IndicatorSeries {
  symbol: string;
  indicator: string;
  interval?: string | null;
  values: IndicatorPoint[];
}

/**
 * WebSocket quote update (for real-time streaming)
 */
export interface QuoteUpdate {
  symbol: string;
  name: string;
  price: string;
  preMarketPrice?: string | null;
  afterHoursPrice?: string | null;
  change: string | number;
  percentChange: string | number;
  logo?: string | null;
}

/**
 * WebSocket index update (for real-time streaming)
 */
export interface IndexUpdate {
  symbol: string;
  name: string;
  value: number;
  change: string;
  percentChange: string;
}

/**
 * WebSocket movers update (for real-time streaming)
 */
export interface MoversUpdate {
  gainers: MoverItem[];
  losers: MoverItem[];
  actives: MoverItem[];
  timestamp: string;
}

// =====================================================
// Request Types
// =====================================================

/**
 * Request parameters for getting quotes
 */
export interface GetQuotesRequest {
  symbols?: string[];
}

/**
 * Request parameters for searching symbols
 */
export interface SearchRequest {
  q: string;
  hits?: number;
  yahoo?: boolean;
}

/**
 * Request parameters for getting historical data
 */
export interface GetHistoricalRequest {
  symbol: string;
  range?: string;
  interval?: string;
}

/**
 * Request parameters for getting news
 */
export interface GetNewsRequest {
  symbol?: string;
  limit?: number;
}

/**
 * Request parameters for getting indicators
 */
export interface GetIndicatorRequest {
  symbol: string;
  indicator: string;
  interval?: string;
}

/**
 * Request parameters for subscription
 */
export interface SubscribeRequest {
  symbols: string[];
}

/**
 * Request parameters for unsubscription
 */
export interface UnsubscribeRequest {
  symbols: string[];
}

/**
 * Request parameters for getting financials
 */
export interface GetFinancialsRequest {
  symbol: string;
  statement?: string;
  frequency?: string;
}

/**
 * Financial data point (matches backend FinancialDataPoint)
 */
export interface FinancialDataPoint {
  date: string;
  period_type: string;
  value: number;
  formatted?: string | null;
}

/**
 * Financial statement row (for component compatibility)
 */
export interface FinancialStatementRow {
  Breakdown: string;
  [period: string]: string | number;
}

/**
 * Financial statement response (matches backend FinancialStatement)
 */
export interface FinancialsResponse {
  symbol: string;
  statement_type: string;
  frequency: string;
  metrics: {
    [key: string]: FinancialDataPoint[];
  };
  statement?: {
    [key: string]: FinancialStatementRow;
  };
}

/**
 * Earnings transcript
 */
export interface EarningsTranscript {
  symbol: string;
  quarter: string;
  year: number;
  date: string;
  transcript: string;
  participants: string[];
  metadata: {
    source: string;
    retrieved_at: string;
    transcripts_id: number;
  };
}

/**
 * Earnings transcript response
 */
export interface EarningsTranscriptResponse {
  symbol: string;
  transcripts: EarningsTranscript[];
  metadata: {
    total_transcripts: number;
    filters_applied: {
      quarter: string;
      year: number;
    };
    retrieved_at: string;
  };
}

/**
 * Request parameters for getting earnings transcript
 */
export interface GetEarningsTranscriptRequest {
  symbol: string;
  quarter?: string;
  year?: number;
}

/**
 * Major holders breakdown (matches backend MajorHoldersBreakdown)
 */
export interface MajorHoldersBreakdown {
  insiders_percent_held?: number | null;
  institutions_percent_held?: number | null;
  institutions_float_percent_held?: number | null;
  institutions_count?: number | null;
}

/**
 * Institutional holder (matches backend InstitutionalHolder)
 */
export interface InstitutionalHolder {
  organization: string;
  shares: number;
  date_reported?: string | null;
  percent_held?: number | null;
  value?: number | null;
}

/**
 * Mutual fund holder (matches backend MutualFundHolder)
 */
export interface MutualFundHolder {
  organization: string;
  shares: number;
  date_reported?: string | null;
  percent_held?: number | null;
  value?: number | null;
}

/**
 * Insider transaction (matches backend InsiderTransaction)
 */
export interface InsiderTransaction {
  filer_name: string;
  filer_relation?: string | null;
  transaction_text?: string | null;
  start_date?: string | null;
  shares?: number | null;
  value?: number | null;
  ownership?: string | null;
}

/**
 * Insider purchases summary (matches backend InsiderPurchasesSummary)
 */
export interface InsiderPurchasesSummary {
  purchases?: number | null;
  purchases_shares?: number | null;
  sales?: number | null;
  sales_shares?: number | null;
  net_shares_purchased?: number | null;
  net_shares_purchased_directly?: number | null;
  total_insider_shares?: number | null;
  buy_info_shares?: number | null;
  sell_info_shares?: number | null;
}

/**
 * Holders response (matches backend HolderData)
 */
export interface HoldersResponse {
  symbol: string;
  major_holders?: MajorHoldersBreakdown | null;
  institutional_holders?: InstitutionalHolder[] | null;
  mutual_fund_holders?: MutualFundHolder[] | null;
  insider_transactions?: InsiderTransaction[] | null;
  insider_purchases?: InsiderPurchasesSummary | null;
  insider_roster?: unknown[] | null;
}

/**
 * Request parameters for getting holders
 */
export interface GetHoldersRequest {
  symbol: string;
  holder_type?: string;
}

// =====================================================
// Earnings Calendar Types
// =====================================================

/**
 * Stock reporting earnings
 */
export interface EarningsStock {
  importance: number;
  symbol: string;
  date: string;
  time: string;
  title: string;
  emoji?: string | null;
  logo?: string | null;
}

/**
 * Earnings for a single day
 */
export interface EarningsDay {
  stocks: EarningsStock[];
}

/**
 * Earnings calendar response
 */
export interface EarningsCalendarResponse {
  success: boolean;
  date_from: string;
  date_to: string;
  total_dates?: number | null;
  total_earnings?: number | null;
  earnings: Record<string, EarningsDay>;
}

/**
 * Request parameters for earnings calendar
 */
export interface GetEarningsCalendarRequest {
  fromDate?: string;
  toDate?: string;
}
