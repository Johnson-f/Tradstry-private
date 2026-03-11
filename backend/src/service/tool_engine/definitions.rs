use crate::service::ai_service::model_connection::openrouter::{
    FunctionDefinition, ToolDefinition,
};
use serde_json::json;

/// Get all tool definitions for market_engine functions
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_stock_quote".to_string(),
                description: "Get real-time stock price, market cap, PE ratio, and other detailed quote information for one or more stock symbols. Returns comprehensive market data including price, volume, returns, and company information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbols": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Array of stock ticker symbols (e.g., ['AAPL', 'MSFT'])"
                        }
                    },
                    "required": ["symbols"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_financials".to_string(),
                description: "Get financial statements (income statement, balance sheet, or cash flow) for a company. Can retrieve annual or quarterly data. Use this to answer questions about revenue, earnings, financial metrics, or quarterly/annual performance.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        },
                        "statement": {
                            "type": "string",
                            "enum": ["income", "balance", "cashflow"],
                            "description": "Type of financial statement: 'income' for income statement, 'balance' for balance sheet, 'cashflow' for cash flow statement",
                            "default": "income"
                        },
                        "frequency": {
                            "type": "string",
                            "enum": ["annual", "quarterly"],
                            "description": "Frequency of the data: 'annual' for yearly data, 'quarterly' for quarterly data",
                            "default": "annual"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_market_news".to_string(),
                description: "Get recent market news articles for a specific stock symbol or general market news. Returns news headlines, sources, links, and timestamps. Use this to answer questions about recent news, market events, or company announcements.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol to get news for (e.g., 'AAPL'). If not provided, returns general market news."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of news articles to return",
                            "default": 10,
                            "minimum": 1,
                            "maximum": 50
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_earnings_transcript".to_string(),
                description: "Get earnings call transcripts for a company. Returns full transcripts of earnings calls with participant information. Use this to answer questions about what was discussed in earnings calls, management commentary, or quarterly results discussions.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        },
                        "quarter": {
                            "type": "string",
                            "description": "Quarter identifier (e.g., 'Q1', 'Q2', 'Q3', 'Q4')",
                            "enum": ["Q1", "Q2", "Q3", "Q4"]
                        },
                        "year": {
                            "type": "integer",
                            "description": "Year of the earnings call (e.g., 2024)"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_symbol".to_string(),
                description: "Search for stock symbols by company name or ticker. Returns matching symbols with company names, types, and exchanges. Use this when the user mentions a company name but you need the ticker symbol, or to verify a symbol exists.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query - company name or ticker symbol (e.g., 'Apple', 'AAPL')"
                        },
                        "hits": {
                            "type": "integer",
                            "description": "Maximum number of results to return",
                            "default": 10,
                            "minimum": 1,
                            "maximum": 50
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_historical_data".to_string(),
                description: "Get historical price data (OHLCV - Open, High, Low, Close, Volume) for a stock symbol. Returns candlestick data for specified time range and interval. Use this to answer questions about price history, trends, or to analyze past performance.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        },
                        "range": {
                            "type": "string",
                            "description": "Time range for historical data (e.g., '1d', '5d', '1mo', '3mo', '6mo', '1y', '2y', '5y', '10y', 'ytd', 'max')",
                            "default": "1mo"
                        },
                        "interval": {
                            "type": "string",
                            "description": "Data interval (e.g., '1m', '5m', '15m', '30m', '1h', '1d', '1wk', '1mo')",
                            "default": "1d"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        // Phase 1: Calendar Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_earnings_calendar".to_string(),
                description: "Get earnings calendar information for a stock symbol. Returns earnings dates, estimates (EPS and revenue), and date ranges. Use this to answer questions about when a company reports earnings or what analysts expect.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_dividend_calendar".to_string(),
                description: "Get dividend calendar information for a stock symbol. Returns dividend dates, rates, and ex-dividend dates. Use this to answer questions about dividend payments, ex-dividend dates, or dividend yields.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_split_info".to_string(),
                description: "Get stock split information for a symbol. Returns last split date and split ratio. Use this to answer questions about stock splits, reverse splits, or split history.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_full_calendar".to_string(),
                description: "Get all calendar data for a stock symbol including earnings, dividends, and stock splits in one call. Use this when you need comprehensive calendar information for a company.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_calendars_for_symbols".to_string(),
                description: "Get calendar data (earnings, dividends, splits) for multiple stock symbols at once. Returns calendar data for each symbol. Use this to compare calendar events across multiple companies.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbols": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Array of stock ticker symbols (e.g., ['AAPL', 'MSFT', 'GOOGL'])"
                        }
                    },
                    "required": ["symbols"]
                }),
            },
        },
        // Phase 2: Holders Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_major_holders".to_string(),
                description: "Get major holders breakdown showing ownership percentages (institutional vs insider ownership). Returns percentages held by institutions, mutual funds, and insiders. Use this to answer questions about ownership structure or who owns the company.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_institutional_holders".to_string(),
                description: "Get top institutional holders (hedge funds, pension funds, etc.) for a stock symbol. Returns list of institutions with their holdings and share counts. Use this to see which major institutions own the stock.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_mutual_fund_holders".to_string(),
                description: "Get top mutual fund holders for a stock symbol. Returns list of mutual funds with their holdings. Use this to see which mutual funds own significant positions in the stock.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_insider_transactions".to_string(),
                description: "Get recent insider transactions (buys and sells) for a stock symbol. Returns transactions with dates, transaction types, and share counts. Use this to track insider buying or selling activity.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of transactions to return",
                            "default": 10,
                            "minimum": 1,
                            "maximum": 100
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_insider_purchases".to_string(),
                description: "Get insider purchases summary showing aggregated insider buying activity. Returns net share purchase activity and buy/sell counts. Use this to see if insiders are net buyers or sellers.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_insider_roster".to_string(),
                description: "Get company insider roster listing all company insiders with their positions and holdings. Returns list of insiders with their titles and share ownership. Use this to see who the company insiders are and their positions.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_holders".to_string(),
                description: "Get all holder data for a stock symbol including major holders, institutional holders, mutual fund holders, and insider information in one call. Use this when you need comprehensive ownership information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_custom_holders".to_string(),
                description: "Get custom holder data by type for a stock symbol. Can fetch specific holder types: major, institutional, mutual_fund, insider_transactions, insider_purchases, or insider_roster. Use this to get specific holder information by type.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        },
                        "holder_type": {
                            "type": "string",
                            "enum": ["major", "institutional", "mutual_fund", "insider_transactions", "insider_purchases", "insider_roster"],
                            "description": "Type of holder data to fetch"
                        }
                    },
                    "required": ["symbol", "holder_type"]
                }),
            },
        },
        // Phase 3: Analysts Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_recommendations".to_string(),
                description: "Get analyst recommendations for a stock symbol. Returns consensus recommendations (buy, hold, sell) and breakdown by rating. Use this to see what analysts recommend for a stock.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_upgrades_downgrades".to_string(),
                description: "Get recent analyst upgrades and downgrades for a stock symbol. Returns rating changes with firm names, dates, and new ratings. Use this to track recent analyst rating changes.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_price_targets".to_string(),
                description: "Get analyst price targets for a stock symbol. Returns average, high, and low price targets with upside potential. Use this to see what analysts think the stock price should be.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_earnings_history".to_string(),
                description: "Get earnings history showing past earnings results vs estimates. Returns historical earnings beats/misses with dates and actual vs estimated values. Use this to see how a company has performed relative to estimates historically.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_earnings_estimates".to_string(),
                description: "Get earnings estimates for a stock symbol. Returns future earnings estimates by period with high, low, and average estimates. Use this to see what analysts expect for future earnings.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_revenue_estimates".to_string(),
                description: "Get revenue estimates for a stock symbol. Returns future revenue estimates by period with high, low, and average estimates. Use this to see what analysts expect for future revenue.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_eps_trend".to_string(),
                description: "Get EPS trend data showing how earnings per period have changed over time. Returns trend data showing estimate revisions. Use this to see if earnings estimates are being revised up or down.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_eps_revisions".to_string(),
                description: "Get EPS revisions showing estimate changes over different time periods (7d, 30d, 60d, 90d). Returns revision counts and net revisions. Use this to track how earnings estimates are being revised.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_growth_estimates".to_string(),
                description: "Get growth estimates for a stock symbol showing expected growth rates. Returns growth estimates for earnings, revenue, and other metrics. Use this to see expected growth rates for a company.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_analyst_data".to_string(),
                description: "Get all analyst data for a stock symbol in one call including recommendations, upgrades/downgrades, price targets, earnings history, estimates, trends, and revisions. Use this when you need comprehensive analyst information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Stock ticker symbol (e.g., 'AAPL')"
                        }
                    },
                    "required": ["symbol"]
                }),
            },
        },
        // Phase 4: Sectors Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_sectors".to_string(),
                description: "Get list of all available market sectors. Returns list of sectors with their identifiers. Use this to see what sectors are available or to validate a sector name before querying sector performance.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_sector_performance".to_string(),
                description: "Get performance data for a specific sector. Returns day return, YTD return, and other performance metrics. Use this to see how a specific sector is performing. Sector must be one of: technology, healthcare, financial, consumer_cyclical, communication, industrials, consumer_defensive, energy, utilities, real_estate, basic_materials.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "sector": {
                            "type": "string",
                            "description": "Sector name (e.g., 'technology', 'healthcare', 'financial')"
                        }
                    },
                    "required": ["sector"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_all_sectors_performance".to_string(),
                description: "Get performance data for all sectors at once. Returns performance metrics for all sectors. Use this to compare sector performance or see overall market sector trends.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_sectors".to_string(),
                description: "Get sectors overview with performance data. Alias for get_all_sectors_performance. Returns performance data for all sectors. Use this to get a comprehensive view of sector performance.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_sector_top_companies".to_string(),
                description: "Get top companies in a specific sector. Returns list of top companies by market cap or performance in the sector. Use this to see which companies are leaders in a sector. Sector must be one of: technology, healthcare, financial, consumer_cyclical, communication, industrials, consumer_defensive, energy, utilities, real_estate, basic_materials.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "sector": {
                            "type": "string",
                            "description": "Sector name (e.g., 'technology', 'healthcare', 'financial')"
                        },
                        "count": {
                            "type": "integer",
                            "description": "Number of top companies to return",
                            "default": 25,
                            "minimum": 1,
                            "maximum": 25
                        }
                    },
                    "required": ["sector"]
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_industry".to_string(),
                description: "Get industry information by industry key. Returns industry details and performance data. Use this to get information about a specific industry.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "industry_key": {
                            "type": "string",
                            "description": "Industry key identifier"
                        }
                    },
                    "required": ["industry_key"]
                }),
            },
        },
        // Phase 5: Market Movers Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_movers".to_string(),
                description: "Get all market movers including gainers, losers, and most active stocks. Returns comprehensive mover data. Use this to see overall market movement and activity.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_gainers".to_string(),
                description: "Get top gainers (stocks with largest price increases). Returns list of stocks with biggest gains. Use this to see which stocks are rising the most. Optional count parameter (25, 50, or 100).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of gainers to return (25, 50, or 100)",
                            "default": 25,
                            "enum": [25, 50, 100]
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_losers".to_string(),
                description: "Get top losers (stocks with largest price decreases). Returns list of stocks with biggest losses. Use this to see which stocks are falling the most. Optional count parameter (25, 50, or 100).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of losers to return (25, 50, or 100)",
                            "default": 25,
                            "enum": [25, 50, 100]
                        }
                    },
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_most_active".to_string(),
                description: "Get most active stocks (highest trading volume). Returns list of stocks with highest volume. Use this to see which stocks are trading most actively. Optional count parameter (25, 50, or 100).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of most active stocks to return (25, 50, or 100)",
                            "default": 25,
                            "enum": [25, 50, 100]
                        }
                    },
                    "required": []
                }),
            },
        },
        // Phase 6: Indices & Hours Tools
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_indices".to_string(),
                description: "Get market indices data including S&P 500, Dow Jones, and Nasdaq. Returns current values and changes for major market indices. Use this to see overall market performance.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_market_hours".to_string(),
                description: "Get market hours information and current market status. Returns whether market is open, closed, pre-market, or after-hours. Use this to check if markets are currently trading.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
}
