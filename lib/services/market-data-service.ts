import { apiClient } from "./api-client";
import { apiConfig } from "@/lib/config/api";
import type {
  HealthStatus,
  MarketHours,
  Quote,
  SimpleQuote,
  HistoricalResponse,
  MoversResponse,
  MoverItem,
  NewsItem,
  IndexItem,
  SectorPerformanceItem,
  SearchItem,
  IndicatorSeries,
  LogoUrl,
  GetQuotesRequest,
  GetHistoricalRequest,
  GetNewsRequest,
  GetIndicatorRequest,
  GetFinancialsRequest,
  FinancialsResponse,
  GetEarningsTranscriptRequest,
  EarningsTranscriptResponse,
  GetHoldersRequest,
  HoldersResponse,
  GetEarningsCalendarRequest,
  EarningsCalendarResponse,
  SubscribeRequest,
  UnsubscribeRequest,
} from "@/lib/types/market-data";

// Backend API response wrapper
interface BackendApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

class MarketDataService {
  // Helper to unwrap backend ApiResponse wrapper
  private unwrapResponse<T>(response: BackendApiResponse<T>): T {
    if (!response.success || !response.data) {
      throw new Error(response.error || 'API request failed');
    }
    return response.data;
  }

  // =====================================================
  // MARKET ENGINE ENDPOINTS (from backend/src/service/market_engine)
  // =====================================================

  /**
   * Get market health status
   */
  async getHealth(): Promise<HealthStatus> {
    const response = await apiClient.get<BackendApiResponse<HealthStatus>>(
      apiConfig.endpoints.market.health
    );
    return this.unwrapResponse(response);
  }

  /**
   * Get market hours information
   */
  async getHours(): Promise<MarketHours> {
    const response = await apiClient.get<BackendApiResponse<MarketHours>>(
      apiConfig.endpoints.market.hours
    );
    return this.unwrapResponse(response);
  }

  /**
   * Get quotes for symbols
   */
  async getQuotes(params?: GetQuotesRequest): Promise<Quote[]> {
    const queryParams = params?.symbols?.length
      ? { symbols: params.symbols.join(",") }
      : undefined;
    const response = await apiClient.get<BackendApiResponse<Quote[]>>(
      apiConfig.endpoints.market.quotes, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get detailed quote with company info for a single symbol
   * Returns quote data with asset profile (company info)
   */
  async getDetailedQuote(symbol: string): Promise<Quote> {
    // Backend returns raw JSON from finance-query-core
    // Structure: { quoteSummary: { result: [{ assetProfile: {...}, price: {...}, ... }] } }
    const response = await apiClient.get<BackendApiResponse<any>>(
      apiConfig.endpoints.market.detailedQuote, {
      params: { symbol },
    });
    
    const data = this.unwrapResponse(response);
    
    // Extract quote summary result
    const result = data?.quoteSummary?.result?.[0];
    if (!result) {
      throw new Error('No quote data found');
    }
    
    const assetProfile = result.assetProfile;
    const price = result.price;
    const summaryDetail = result.summaryDetail;
    const defaultKeyStatistics = result.defaultKeyStatistics;
    
    // Transform to Quote format with asset profile
    const quote: Quote = {
      symbol: price?.symbol || symbol,
      name: price?.longName || price?.shortName || null,
      price: price?.regularMarketPrice?.fmt || price?.regularMarketPrice?.raw?.toString() || null,
      afterHoursPrice: price?.postMarketPrice?.fmt || price?.postMarketPrice?.raw?.toString() || null,
      change: price?.regularMarketChange?.fmt || price?.regularMarketChange?.raw?.toString() || null,
      percentChange: price?.regularMarketChangePercent?.fmt || price?.regularMarketChangePercent?.raw?.toString() || null,
      open: summaryDetail?.regularMarketOpen?.fmt || summaryDetail?.open?.fmt || price?.regularMarketOpen?.fmt || price?.regularMarketOpen?.raw?.toString() || null,
      high: summaryDetail?.regularMarketDayHigh?.fmt || summaryDetail?.dayHigh?.fmt || price?.regularMarketDayHigh?.fmt || price?.regularMarketDayHigh?.raw?.toString() || null,
      low: summaryDetail?.regularMarketDayLow?.fmt || summaryDetail?.dayLow?.fmt || price?.regularMarketDayLow?.fmt || price?.regularMarketDayLow?.raw?.toString() || null,
      yearHigh: summaryDetail?.fiftyTwoWeekHigh?.fmt || price?.fiftyTwoWeekHigh?.fmt || price?.fiftyTwoWeekHigh?.raw?.toString() || null,
      yearLow: summaryDetail?.fiftyTwoWeekLow?.fmt || price?.fiftyTwoWeekLow?.fmt || price?.fiftyTwoWeekLow?.raw?.toString() || null,
      volume: summaryDetail?.regularMarketVolume?.raw || summaryDetail?.volume?.raw || price?.regularMarketVolume?.raw || null,
      avgVolume: summaryDetail?.averageVolume?.raw || summaryDetail?.averageVolume10days?.raw || null,
      marketCap: summaryDetail?.marketCap?.fmt || summaryDetail?.marketCap?.raw?.toString() || null,
      beta: summaryDetail?.beta?.fmt || summaryDetail?.beta?.raw?.toString() || null,
      pe: summaryDetail?.trailingPE?.fmt || summaryDetail?.trailingPE?.raw?.toString() || null,
      previousClose: summaryDetail?.regularMarketPreviousClose?.fmt || summaryDetail?.previousClose?.fmt || summaryDetail?.regularMarketPreviousClose?.raw?.toString() || summaryDetail?.previousClose?.raw?.toString() || null,
      dividendYield: summaryDetail?.dividendYield?.fmt || summaryDetail?.trailingAnnualDividendYield?.fmt || summaryDetail?.dividendYield?.raw?.toString() || summaryDetail?.trailingAnnualDividendYield?.raw?.toString() || null,
      eps: defaultKeyStatistics?.trailingEps?.fmt || defaultKeyStatistics?.trailingEps?.raw?.toString() || null,
      earningsDate: summaryDetail?.earningsTimestamp?.fmt || null,
      sector: assetProfile?.sectorDisp || assetProfile?.sector || null,
      industry: assetProfile?.industryDisp || assetProfile?.industry || null,
      about: assetProfile?.longBusinessSummary || null,
      employees: assetProfile?.fullTimeEmployees?.toString() || null,
      logo: null, // Logo is fetched separately
      assetProfile: assetProfile ? {
        address1: assetProfile.address1 || null,
        city: assetProfile.city || null,
        state: assetProfile.state || null,
        zip: assetProfile.zip || null,
        country: assetProfile.country || null,
        phone: assetProfile.phone || null,
        website: assetProfile.website || null,
        industry: assetProfile.industry || null,
        industryDisp: assetProfile.industryDisp || null,
        sector: assetProfile.sector || null,
        sectorDisp: assetProfile.sectorDisp || null,
        longBusinessSummary: assetProfile.longBusinessSummary || null,
        fullTimeEmployees: assetProfile.fullTimeEmployees || null,
        companyOfficers: assetProfile.companyOfficers?.map((o: any) => ({
          name: o.name || null,
          title: o.title || null,
          age: o.age || null,
          yearBorn: o.yearBorn || null,
          totalPay: o.totalPay ? {
            raw: o.totalPay.raw || null,
            fmt: o.totalPay.fmt || null,
            longFmt: o.totalPay.longFmt || null,
          } : null,
        })) || null,
      } : null,
    };
    
    return quote;
  }

  /**
   * Get simple quotes for symbols (summary data)
   */
  async getSimpleQuotes(params?: GetQuotesRequest): Promise<SimpleQuote[]> {
    const queryParams = params?.symbols?.length
      ? { symbols: params.symbols.join(",") }
      : undefined;
    const response = await apiClient.get<BackendApiResponse<SimpleQuote[]>>(
      apiConfig.endpoints.market.simpleQuotes, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get similar quotes to a symbol
   */
  async getSimilar(symbol: string): Promise<SimpleQuote[]> {
    // Backend returns raw JSON Value from finance-query-core
    // Structure: { finance: { result: [{ symbol: string, recommended_symbols: [{ symbol: string, score: number }] }] } }
    const response = await apiClient.get<BackendApiResponse<any>>(
      apiConfig.endpoints.market.similar, {
      params: { symbol },
    });
    
    const data = this.unwrapResponse(response);
    
    // Parse the finance-query-core response structure
    if (data?.finance?.result && Array.isArray(data.finance.result) && data.finance.result.length > 0) {
      const recommendationResult = data.finance.result[0];
      if (recommendationResult?.recommended_symbols && Array.isArray(recommendationResult.recommended_symbols)) {
        const similarSymbols = recommendationResult.recommended_symbols
          .map((rec: { symbol: string; score: number }) => rec.symbol)
          .filter((s: string) => s && s.length > 0)
          .slice(0, 10); // Limit to 10
        
        // Fetch quotes for similar symbols
        if (similarSymbols.length > 0) {
          try {
            const quotes = await this.getSimpleQuotes({ symbols: similarSymbols });
            return quotes;
          } catch (error) {
            console.warn('Failed to fetch quotes for similar symbols:', error);
            // Return empty array if quote fetch fails
            return [];
          }
        }
      }
    }
    
    // Fallback: return empty array if structure doesn't match
    return [];
  }

  /**
   * Get logo URL for a symbol
   */
  async getLogo(symbol: string): Promise<LogoUrl> {
    const response = await apiClient.get<BackendApiResponse<LogoUrl>>(
      apiConfig.endpoints.market.logo, {
      params: { symbol },
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get historical data for a symbol
   * Backend returns transformed JSON with RFC 3339 formatted timestamps
   * Structure: { chart: { result: [{ meta, timestamp: [string (RFC 3339)], indicators: { quote: [{ open, high, low, close, volume }], adjclose: [{ adjclose }] } }] } }
   */
  async getHistorical(params: GetHistoricalRequest): Promise<HistoricalResponse> {
    const queryParams: Record<string, string> = {
      symbol: params.symbol,
    };
    if (params.range) queryParams.range = params.range;
    if (params.interval) queryParams.interval = params.interval;

    // Backend returns raw JSON Value from finance-query-core
    const response = await apiClient.get<BackendApiResponse<any>>(
      apiConfig.endpoints.market.historical,
      { params: queryParams }
    );
    
    const data = this.unwrapResponse(response);
    
    // Parse the chart response structure
    const chartResult = data?.chart?.result?.[0];
    if (!chartResult) {
      throw new Error('No chart data available');
    }
    
    const timestamps = chartResult.timestamp || [];
    const quote = chartResult.indicators?.quote?.[0];
    const adjclose = chartResult.indicators?.adjclose?.[0];
    
    if (!quote || !timestamps.length) {
      return {
        symbol: params.symbol,
        interval: params.interval || null,
        candles: [],
      };
    }
    
    // Transform backend structure to frontend candles
    // Backend now returns RFC 3339 formatted timestamps (ISO 8601 strings)
    const candles = timestamps.map((timestamp: string | number, index: number) => {
      // Backend returns RFC 3339 strings, but handle both formats for compatibility
      const time = typeof timestamp === "string" 
        ? timestamp 
        : new Date(timestamp * 1000).toISOString(); // Fallback for Unix timestamps
      
      return {
        time,
        open: quote.open?.[index] ?? 0,
        high: quote.high?.[index] ?? 0,
        low: quote.low?.[index] ?? 0,
        close: quote.close?.[index] ?? 0,
        adjClose: adjclose?.adjclose?.[index] ?? null,
        volume: quote.volume?.[index] ?? null,
      };
    });
    
    return {
      symbol: chartResult.meta?.symbol || params.symbol,
      interval: chartResult.meta?.dataGranularity || params.interval || null,
      candles,
    };
  }

  /**
   * Get market movers (gainers, losers, most active)
   */
  async getMovers(): Promise<MoversResponse> {
    const response = await apiClient.get<BackendApiResponse<MoversResponse>>(
      apiConfig.endpoints.market.movers
    );
    return this.unwrapResponse(response);
  }

  /**
   * Get top gainers
   */
  async getGainers(count?: number): Promise<MoverItem[]> {
    const queryParams = count ? { count: count.toString() } : undefined;
    const response = await apiClient.get<BackendApiResponse<MoverItem[]>>(
      apiConfig.endpoints.market.gainers, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get top losers
   */
  async getLosers(count?: number): Promise<MoverItem[]> {
    const queryParams = count ? { count: count.toString() } : undefined;
    const response = await apiClient.get<BackendApiResponse<MoverItem[]>>(
      apiConfig.endpoints.market.losers, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get most active stocks
   */
  async getActives(count?: number): Promise<MoverItem[]> {
    const queryParams = count ? { count: count.toString() } : undefined;
    const response = await apiClient.get<BackendApiResponse<MoverItem[]>>(
      apiConfig.endpoints.market.actives, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get market news
   */
  async getNews(params?: GetNewsRequest): Promise<NewsItem[]> {
    const queryParams: Record<string, string> = {};
    if (params?.symbol) queryParams.symbol = params.symbol;
    if (params?.limit) queryParams.limit = params.limit.toString();

    // Backend returns NewsResponse with articles array
    interface BackendNewsResponse {
      symbol?: string | null;
      query?: string | null;
      articles: Array<{
        uuid?: string | null;
        title: string;
        publisher?: string | null;
        link: string;
        publish_time?: number | null;
        publish_time_formatted?: string | null;
        thumbnail_url?: string | null;
        related_tickers?: string[];
      }>;
      count: number;
    }

    const response = await apiClient.get<BackendApiResponse<BackendNewsResponse>>(
      apiConfig.endpoints.market.news, {
      params: Object.keys(queryParams).length > 0 ? queryParams : undefined,
    });
    
    const newsResponse = this.unwrapResponse(response);
    
    // Return articles directly from backend
    return newsResponse.articles.map((article) => ({
      uuid: article.uuid,
      title: article.title,
      publisher: article.publisher,
      link: article.link,
      publish_time: article.publish_time,
      publish_time_formatted: article.publish_time_formatted,
      thumbnail_url: article.thumbnail_url,
      related_tickers: article.related_tickers || [],
    }));
  }

  /**
   * Get market indices
   */
  async getIndices(): Promise<IndexItem[]> {
    const response = await apiClient.get<BackendApiResponse<IndexItem[]>>(apiConfig.endpoints.market.indices);
    return this.unwrapResponse(response);
  }

  /**
   * Get sector performance
   */
  async getSectors(): Promise<SectorPerformanceItem[]> {
    const response = await apiClient.get<BackendApiResponse<SectorPerformanceItem[]>>(
      apiConfig.endpoints.market.sectors
    );
    return this.unwrapResponse(response);
  }

  /**
   * Search for symbols
   */
  async search(query: string, params?: { hits?: number; yahoo?: boolean }): Promise<SearchItem[]> {
    const queryParams: Record<string, string> = { q: query };
    if (params?.hits !== undefined) queryParams.hits = params.hits.toString();
    if (params?.yahoo !== undefined) queryParams.yahoo = params.yahoo.toString();
    const response = await apiClient.get<BackendApiResponse<SearchItem[]>>(
      apiConfig.endpoints.market.search, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get indicator data for a symbol
   */
  async getIndicator(
    params: GetIndicatorRequest
  ): Promise<IndicatorSeries> {
    const queryParams: Record<string, string> = {
      symbol: params.symbol,
      indicator: params.indicator,
    };
    if (params.interval) queryParams.interval = params.interval;

    const response = await apiClient.get<BackendApiResponse<IndicatorSeries>>(
      apiConfig.endpoints.market.indicators,
      { params: queryParams }
    );
    return this.unwrapResponse(response);
  }

  /**
   * Subscribe to market updates for symbols via WebSocket
   */
  async subscribe(params: SubscribeRequest): Promise<void> {
    await apiClient.post<BackendApiResponse<void>>(
      apiConfig.endpoints.market.subscribe, {
      symbols: params.symbols,
    });
    // Void response doesn't need unwrapping
    return;
  }

  /**
   * Unsubscribe from market updates for symbols via WebSocket
   */
  async unsubscribe(params: UnsubscribeRequest): Promise<void> {
    await apiClient.post<BackendApiResponse<void>>(
      apiConfig.endpoints.market.unsubscribe, {
      symbols: params.symbols,
    });
    // Void response doesn't need unwrapping
    return;
  }

  /**
   * Get financial statements for a symbol
   */
  async getFinancials(params: GetFinancialsRequest): Promise<FinancialsResponse> {
    const queryParams: Record<string, string> = {
      symbol: params.symbol,
    };
    if (params.statement) queryParams.statement = params.statement;
    if (params.frequency) queryParams.frequency = params.frequency;
    
    // Backend returns FinancialStatement serialized as JSON
    // Structure: { symbol: string, statement_type: string, frequency: string, metrics: { [key: string]: FinancialDataPoint[] } }
    const response = await apiClient.get<BackendApiResponse<any>>(
      apiConfig.endpoints.market.financials, {
      params: queryParams,
    });
    
    const data = this.unwrapResponse(response);
    
    // Transform metrics to statement format for component compatibility
    // Components expect: { [metricName]: { Breakdown: string, [date]: value } }
    const statement: Record<string, any> = {};
    if (data.metrics && typeof data.metrics === 'object') {
      Object.entries(data.metrics).forEach(([metricName, points]: [string, any]) => {
        if (Array.isArray(points) && points.length > 0) {
          const row: any = { Breakdown: metricName };
          points.forEach((point: any) => {
            if (point.date) {
              // Use date as period key, formatted value if available, otherwise raw value
              row[point.date] = point.formatted || point.value || point.value || 0;
            }
          });
          statement[metricName] = row;
        }
      });
    }
    
    return {
      symbol: data.symbol || params.symbol,
      statement_type: data.statement_type || params.statement || 'all',
      frequency: data.frequency || params.frequency || 'annual',
      metrics: data.metrics || {},
      statement: Object.keys(statement).length > 0 ? statement : undefined,
    };
  }

  /**
   * Get earnings transcript for a symbol
   */
  async getEarningsTranscript(params: GetEarningsTranscriptRequest): Promise<EarningsTranscriptResponse> {
    const queryParams: Record<string, string> = {
      symbol: params.symbol,
    };
    if (params.quarter) queryParams.quarter = params.quarter;
    if (params.year) queryParams.year = params.year.toString();
    const response = await apiClient.get<BackendApiResponse<EarningsTranscriptResponse>>(
      apiConfig.endpoints.market.earningsTranscript, {
      params: queryParams,
    });
    return this.unwrapResponse(response);
  }

  /**
   * Get holders data for a symbol
   */
  async getHolders(params: GetHoldersRequest): Promise<HoldersResponse> {
    const queryParams: Record<string, string> = {
      symbol: params.symbol,
    };
    if (params.holder_type) queryParams.holder_type = params.holder_type;
    
    // Backend returns HolderData serialized as JSON
    // Structure: { symbol: string, major_holders?: ..., institutional_holders?: ..., etc. }
    const response = await apiClient.get<BackendApiResponse<any>>(
      apiConfig.endpoints.market.holders, {
      params: queryParams,
    });
    
    const data = this.unwrapResponse(response);
    
    // Return backend HolderData directly
    return {
      symbol: data.symbol || params.symbol,
      major_holders: data.major_holders || null,
      institutional_holders: data.institutional_holders || null,
      mutual_fund_holders: data.mutual_fund_holders || null,
      insider_transactions: data.insider_transactions || null,
      insider_purchases: data.insider_purchases || null,
      insider_roster: data.insider_roster || null,
    };
  }

  /**
   * Get earnings calendar for a date range
   * Returns companies reporting earnings filtered by market cap >= $100M
   */
  async getEarningsCalendar(params?: GetEarningsCalendarRequest): Promise<EarningsCalendarResponse> {
    const queryParams: Record<string, string> = {};
    if (params?.fromDate) queryParams.from_date = params.fromDate;
    if (params?.toDate) queryParams.to_date = params.toDate;

    const response = await apiClient.get<BackendApiResponse<EarningsCalendarResponse>>(
      apiConfig.endpoints.market.earningsCalendar, {
      params: Object.keys(queryParams).length > 0 ? queryParams : undefined,
    });
    
    return this.unwrapResponse(response);
  }
}

// Export singleton instance
export const marketDataService = new MarketDataService();
export default marketDataService;