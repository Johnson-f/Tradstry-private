<!-- 07deeb8f-bf5e-445a-b36c-a0d05ab9056d d7006ca4-3e67-4bfd-9490-b94557ff10da -->
# Add All Market Engine Functions as Tools

## Overview

Expand the tool_engine to include all available market_engine functions, enabling the AI chat service to access comprehensive market data including calendar events, holder information, analyst data, sector performance, market movers, indices, and market hours.

## Current State

**Currently Implemented (6 tools):**

- `get_stock_quote` - Real-time stock quotes
- `get_financials` - Financial statements
- `get_market_news` - Market news
- `get_earnings_transcript` - Earnings transcripts
- `search_symbol` - Symbol search
- `get_historical_data` - Historical price data

**Missing (30+ functions across 7 modules):**

- Calendar: 5 functions
- Holders: 8 functions
- Analysts: 10 functions
- Sectors: 6 functions
- Movers: 4 functions
- Indices: 1 function
- Hours: 1 function

## Implementation Plan

### Phase 1: Calendar Tools (5 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_earnings_calendar` - Get earnings calendar for a symbol
2. `get_dividend_calendar` - Get dividend calendar for a symbol
3. `get_split_info` - Get stock split information
4. `get_full_calendar` - Get all calendar data (earnings, dividend, split)
5. `get_calendars_for_symbols` - Get calendar data for multiple symbols

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `calendar` module and routing to:

- `calendar::get_earnings_calendar`
- `calendar::get_dividend_calendar`
- `calendar::get_split_info`
- `calendar::get_full_calendar`
- `calendar::get_calendars_for_symbols`

### Phase 2: Holders Tools (8 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_major_holders` - Get major holders breakdown
2. `get_institutional_holders` - Get top institutional holders
3. `get_mutual_fund_holders` - Get top mutual fund holders
4. `get_insider_transactions` - Get recent insider transactions
5. `get_insider_purchases` - Get insider purchases summary
6. `get_insider_roster` - Get company insider roster
7. `get_all_holders` - Get all holder data
8. `get_custom_holders` - Get custom holder data by type

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `holders` module and routing to respective functions. Handle optional parameters like `limit` for `get_insider_transactions` and `holder_type` for `get_custom_holders`.

### Phase 3: Analysts Tools (10 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_recommendations` - Get analyst recommendations
2. `get_upgrades_downgrades` - Get upgrades and downgrades
3. `get_price_targets` - Get price targets
4. `get_earnings_history` - Get earnings history
5. `get_earnings_estimates` - Get earnings estimates
6. `get_revenue_estimates` - Get revenue estimates
7. `get_eps_trend` - Get EPS trend data
8. `get_eps_revisions` - Get EPS revisions
9. `get_growth_estimates` - Get growth estimates
10. `get_all_analyst_data` - Get all analyst data in one call

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `analysts` module. Most functions take just a `symbol` parameter, making them straightforward to implement.

### Phase 4: Sectors Tools (6 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_all_sectors` - Get list of all sectors
2. `get_sector_performance` - Get performance for a specific sector
3. `get_all_sectors_performance` - Get performance for all sectors
4. `get_sectors` - Get sectors overview (alias)
5. `get_sector_top_companies` - Get top companies in a sector
6. `get_industry` - Get industry information

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `sectors` module. Handle `Sector` enum conversion from string input for `get_sector_performance` and `get_sector_top_companies`.

### Phase 5: Market Movers Tools (4 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_movers` - Get all market movers
2. `get_gainers` - Get top gainers
3. `get_losers` - Get top losers
4. `get_most_active` - Get most active stocks

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `movers` module. Handle optional `count` parameter (defaults to 25).

### Phase 6: Indices & Hours Tools (2 tools)

**File:** `backend/src/service/tool_engine/definitions.rs`

Add tool definitions for:

1. `get_indices` - Get market indices data
2. `get_market_hours` - Get market hours information

**File:** `backend/src/service/tool_engine/executor.rs`

Add handlers importing `indices` and `hours` modules.

## Implementation Details

### Tool Definition Pattern

Each tool definition should follow this structure:

```rust
ToolDefinition {
    r#type: "function".to_string(),
    function: FunctionDefinition {
        name: "tool_name".to_string(),
        description: "Clear description of what the tool does and when to use it".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                // Parameter definitions
            },
            "required": ["required_param"]
        }),
    },
}
```

### Executor Handler Pattern

Each handler should:

1. Parse JSON arguments
2. Extract and validate required parameters
3. Handle optional parameters with defaults
4. Call the market_engine function
5. Convert result to JSON Value
6. Return error with context if execution fails

Example:

```rust
"get_earnings_calendar" => {
    let symbol = args["symbol"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("symbol is required"))?
        .to_string();
    
    let earnings = calendar::get_earnings_calendar(&symbol).await?;
    serde_json::to_value(earnings)?
}
```

### Special Considerations

1. **Sector Enum Handling**: For `get_sector_performance` and `get_sector_top_companies`, accept sector as string and convert to `Sector` enum using `sectors::get_all_sectors()` for validation.

2. **Multi-symbol Functions**: Functions like `get_calendars_for_symbols` accept arrays of symbols - validate array is not empty.

3. **Optional Parameters**: Many functions have optional parameters (like `limit`, `count`, `quarter`, `year`) - use `.get()` with `.and_then()` pattern.

4. **Error Handling**: All handlers should use `?` operator for error propagation and wrap in `anyhow::Context` for better error messages.

## Files to Modify

1. **`backend/src/service/tool_engine/definitions.rs`**

   - Add ~35 new tool definitions (keep existing 6)
   - Total: ~41 tool definitions

2. **`backend/src/service/tool_engine/executor.rs`**

   - Add imports for new modules: `calendar`, `holders`, `analysts`, `sectors`, `movers`, `indices`, `hours`
   - Add ~35 new match arms in `execute()` method
   - Handle parameter parsing and validation for each tool

## Testing Considerations

- Verify all tools compile without errors
- Test parameter parsing for each tool type
- Ensure error messages are clear when parameters are missing or invalid
- Test with actual LLM to verify tool selection works correctly

## Notes

- **Watchlist module excluded**: The `watchlist_price` module contains database operations that require user context and database connections, so these are not suitable for tool exposure (they're user-specific data operations, not market data).

- **Stream service excluded**: The `stream_service` is for WebSocket streaming, not suitable for one-time tool calls.

- **Caching module excluded**: The `caching` module appears to be empty/incomplete.