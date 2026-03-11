/**
 * Supabase Edge Function: Earnings Calendar Fetcher with Logo Support
 * 
 * This Edge Function fetches earnings calendar data from StockTwits API
 * and optionally fetches company logos, returning them as base64 encoded data.
 */

// CORS headers for handling cross-origin requests
const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
  'Access-Control-Allow-Methods': 'POST, GET, OPTIONS, PUT, DELETE',
};

// Request body interface
interface RequestBody {
  fromDate?: string; // Optional: YYYY-MM-DD format
  toDate?: string;   // Optional: YYYY-MM-DD format
  includeLogos?: boolean; // Optional: whether to fetch logos
}

// StockTwits API configuration for earnings calendar data
const STOCKTWITS_CONFIG = {
  name: 'StockTwits',
  baseUrl: 'https://api.stocktwits.com/api/2',
  endpoints: {
    earningsCalendar: '/discover/earnings_calendar',
  }
};

// Logo sources configuration
const LOGO_SOURCES = [
  (symbol: string) => `https://logo.clearbit.com/${symbol.toLowerCase()}.com`,
  (symbol: string) => `https://financialmodelingprep.com/image-stock/${symbol}.png`,
  (symbol: string) => `https://storage.googleapis.com/iex/api/logos/${symbol}.png`,
  (symbol: string) => `https://img.logo.dev/ticker/${symbol}?token=pk_X-7cEE8hSkKawJLLBC1mIw`,
];

/**
 * Validate and parse date string
 */
function isValidDate(dateString: string): boolean {
  const regex = /^\d{4}-\d{2}-\d{2}$/;
  if (!regex.test(dateString)) return false;
  
  const date = new Date(dateString);
  return date instanceof Date && !isNaN(date.getTime());
}

/**
 * Get date range for earnings calendar
 * If dates are provided in request, use those. Otherwise, default to 1 month into the future.
 */
function getDateRange(requestBody?: RequestBody): { fromDate: string; toDate: string } {
  let fromDate: string;
  let toDate: string;
  
  if (requestBody?.fromDate && requestBody?.toDate) {
    // Validate provided dates
    if (!isValidDate(requestBody.fromDate) || !isValidDate(requestBody.toDate)) {
      throw new Error('Invalid date format. Use YYYY-MM-DD format.');
    }
    
    fromDate = requestBody.fromDate;
    toDate = requestBody.toDate;
    
    // Ensure fromDate is before toDate
    if (new Date(fromDate) > new Date(toDate)) {
      throw new Error('fromDate must be before toDate');
    }
  } else {
    // Default: today to 1 month in the future
    const today = new Date();
    const oneMonthLater = new Date(today);
    oneMonthLater.setMonth(today.getMonth() + 1);
    
    fromDate = today.toISOString().split('T')[0];
    toDate = oneMonthLater.toISOString().split('T')[0];
  }
  
  console.log(`Date range: ${fromDate} to ${toDate}`);
  
  return { fromDate, toDate };
}

/**
 * Fetch earnings calendar data from StockTwits API
 */
async function fetchFromStockTwits(fromDate: string, toDate: string): Promise<unknown> {
  try {
    const url = `${STOCKTWITS_CONFIG.baseUrl}${STOCKTWITS_CONFIG.endpoints.earningsCalendar}?date_from=${fromDate}&date_to=${toDate}`;
    console.log(`StockTwits: Fetching from ${url}`);
    
    const response = await fetch(url);
    console.log(`StockTwits: Response status ${response.status}`);
    
    if (!response.ok) {
      console.log(`StockTwits: Response not ok: ${response.statusText}`);
      return null;
    }
    
    const data = await response.json();
    console.log('StockTwits: Raw response structure:', Object.keys(data));
    
    return data;
  } catch (error) {
    console.error(`StockTwits earnings calendar fetch error:`, error);
    return null;
  }
}

/**
 * Fetch logo for a stock symbol with timeout
 */
async function fetchLogoWithTimeout(symbol: string, timeoutMs: number = 2000): Promise<string | null> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  
  try {
    for (const getUrl of LOGO_SOURCES) {
      const url = getUrl(symbol);
      
      try {
        const response = await fetch(url, {
          signal: controller.signal,
          headers: {
            'User-Agent': 'Mozilla/5.0 (compatible; EarningsBot/1.0)',
          }
        });
        
        if (response.ok) {
          const arrayBuffer = await response.arrayBuffer();
          const base64 = btoa(
            new Uint8Array(arrayBuffer).reduce(
              (data, byte) => data + String.fromCharCode(byte),
              ''
            )
          );
          
          // Get content type for proper data URI
          const contentType = response.headers.get('content-type') || 'image/png';
          clearTimeout(timeoutId);
          
          return `data:${contentType};base64,${base64}`;
        }
      } catch (err) {
        // Try next source
        continue;
      }
    }
    
    return null;
  } catch (error) {
    console.error(`Error fetching logo for ${symbol}:`, error);
    return null;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Fetch logos for multiple symbols in parallel with concurrency limit
 */
async function fetchLogos(symbols: string[]): Promise<Record<string, string>> {
  console.log(`Fetching logos for ${symbols.length} symbols...`);
  
  const logos: Record<string, string> = {};
  const concurrencyLimit = 10;
  
  // Process symbols in batches
  for (let i = 0; i < symbols.length; i += concurrencyLimit) {
    const batch = symbols.slice(i, i + concurrencyLimit);
    
    const results = await Promise.allSettled(
      batch.map(async (symbol) => {
        const logo = await fetchLogoWithTimeout(symbol, 2000);
        return { symbol, logo };
      })
    );
    
    for (const result of results) {
      if (result.status === 'fulfilled' && result.value.logo) {
        logos[result.value.symbol] = result.value.logo;
      }
    }
  }
  
  console.log(`Successfully fetched ${Object.keys(logos).length} logos out of ${symbols.length}`);
  
  return logos;
}

/**
 * Main Edge Function handler
 */
Deno.serve(async (req) => {
  // Handle CORS preflight requests
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders });
  }
  
  try {
    console.log('Starting earnings calendar fetch from StockTwits...');
    
    // Parse request body or query parameters for custom dates
    let requestBody: RequestBody | undefined;
    try {
      if (req.method === 'POST') {
        const contentType = req.headers.get('content-type');
        if (contentType?.includes('application/json')) {
          requestBody = await req.json();
        }
      } else if (req.method === 'GET') {
        // Parse query parameters for GET requests
        const url = new URL(req.url);
        const fromDateParam = url.searchParams.get('fromDate');
        const toDateParam = url.searchParams.get('toDate');
        const includeLogosParam = url.searchParams.get('includeLogos');
        
        if (fromDateParam || toDateParam || includeLogosParam) {
          requestBody = {
            fromDate: fromDateParam || undefined,
            toDate: toDateParam || undefined,
            includeLogos: includeLogosParam === 'true'
          };
        }
      }
    } catch {
      console.log('No valid JSON body or query params provided, using default dates');
    }
    
    // Get date range (from request or use defaults)
    let fromDate: string;
    let toDate: string;
    
    try {
      const dateRange = getDateRange(requestBody);
      fromDate = dateRange.fromDate;
      toDate = dateRange.toDate;
    } catch (dateError) {
      return new Response(
        JSON.stringify({ 
          success: false, 
          error: (dateError as Error).message,
          message: 'Invalid date parameters'
        }),
        { 
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          status: 400
        }
      );
    }
    
    console.log(`Fetching earnings calendar data from ${fromDate} to ${toDate}`);
    
    try {
      // Fetch data from StockTwits
      const rawData = await fetchFromStockTwits(fromDate, toDate);
      
      if (rawData && typeof rawData === 'object' && 'earnings' in rawData) {
        const earningsData = rawData as { earnings?: Record<string, { stocks?: Array<{
          symbol: string;
          time: string;
          title: string;
          importance: number;
          emoji?: string;
        }> }> };
        
        // Collect all unique symbols
        const allSymbols = new Set<string>();
        
        if (earningsData.earnings) {
          for (const dateData of Object.values(earningsData.earnings)) {
            if (dateData?.stocks && Array.isArray(dateData.stocks)) {
              for (const stock of dateData.stocks) {
                allSymbols.add(stock.symbol);
              }
            }
          }
        }
        
        // Fetch logos if requested
        let logoMap: Record<string, string> = {};
        if (requestBody?.includeLogos) {
          console.log('Logo fetching requested...');
          logoMap = await fetchLogos(Array.from(allSymbols));
        }
        
        // Transform to formatted response with logos embedded in each stock
        const formattedEarnings: Record<string, { stocks: Array<{
          importance: number;
          symbol: string;
          date: string;
          time: string;
          title: string;
          emoji?: string;
          logo?: string;
        }> }> = {};
        
        let totalStocks = 0;
        
        if (earningsData.earnings) {
          for (const [date, dateData] of Object.entries(earningsData.earnings)) {
            if (dateData?.stocks && Array.isArray(dateData.stocks)) {
              formattedEarnings[date] = {
                stocks: dateData.stocks.map(stock => {
                  const stockData: {
                    importance: number;
                    symbol: string;
                    date: string;
                    time: string;
                    title: string;
                    emoji?: string;
                    logo?: string;
                  } = {
                    importance: stock.importance,
                    symbol: stock.symbol,
                    date: date,
                    time: stock.time,
                    title: stock.title,
                  };
                  
                  if (stock.emoji) {
                    stockData.emoji = stock.emoji;
                  }
                  
                  // Add logo if available
                  if (requestBody?.includeLogos && logoMap[stock.symbol]) {
                    stockData.logo = logoMap[stock.symbol];
                  }
                  
                  return stockData;
                })
              };
              totalStocks += dateData.stocks.length;
            }
          }
        }
        
        const response = {
          success: true,
          date_from: fromDate,
          date_to: toDate,
          total_dates: Object.keys(formattedEarnings).length,
          total_earnings: totalStocks,
          earnings: formattedEarnings
        };
        
        console.log(`Successfully fetched ${totalStocks} earnings across ${Object.keys(formattedEarnings).length} dates`);
        if (requestBody?.includeLogos) {
          console.log(`Fetched ${Object.keys(logoMap).length} logos`);
        }
        
        return new Response(
          JSON.stringify(response),
          { 
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
            status: 200
          }
        );
      } else {
        return new Response(
          JSON.stringify({ 
            success: false, 
            message: 'StockTwits API returned no earnings data for the specified date range'
          }),
          { 
            headers: { ...corsHeaders, 'Content-Type': 'application/json' },
            status: 404
          }
        );
      }
    } catch (error) {
      console.error('StockTwits earnings calendar fetch error:', error);
      return new Response(
        JSON.stringify({ 
          success: false, 
          message: `StockTwits API error: ${(error as Error).message}`
        }),
        { 
          headers: { ...corsHeaders, 'Content-Type': 'application/json' },
          status: 500
        }
      );
    }
    
  } catch (error) {
    console.error('Edge function error:', error);
    
    return new Response(
      JSON.stringify({ 
        success: false, 
        error: (error as Error).message,
        message: 'Internal server error'
      }),
      { 
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        status: 500
      }
    );
  }
});