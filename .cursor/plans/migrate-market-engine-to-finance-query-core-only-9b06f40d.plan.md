<!-- 9b06f40d-dd8f-42cc-90df-b89339d5b868 5337cf59-9b2b-4e95-8d81-0e130193e4db -->
# Plan: Migrate market_engine to finance-query-core Only

## Overview

Currently, some modules in `market_engine` use `MarketClient` which fetches from URLs (`finance-query.onrender.com` and `api.tradstry.com`). This plan migrates all modules to use `finance-query-core` crate directly, eliminating external URL dependencies.

## Current State Analysis

### Already Using finance-query-core (but still accept MarketClient parameter):

- `movers.rs` - Uses `YahooFinanceClient` directly
- `indices.rs` - Uses `YahooFinanceClient` directly  
- `quotes.rs` - Uses `YahooFinanceClient` directly
- `historical.rs` - Uses `YahooFinanceClient` directly
- `news.rs` - Uses `YahooFinanceClient` directly
- `financials.rs` - Uses `YahooFinanceClient` directly
- `holders.rs` - Uses `YahooFinanceClient` directly
- `calendar.rs` - Uses `YahooFinanceClient` directly
- `analysts.rs` - Uses `YahooFinanceClient` directly
- `sectors.rs` - Uses `YahooFinanceClient` directly

### Still Using MarketClient.get() (URL fetching):

- `search.rs` - Uses `client.get("/v1/search")`
- `indicators.rs` - Uses `client.get("/v1/indicator")`
- `hours.rs` - Uses `client.get("/hours")`
- `health.rs` - Uses `client.get("/health")`
- `transcripts.rs` - Uses `client.get("/v1/earnings-transcript/{symbol}")`

### Other Files:

- `watchlist_price.rs` - Accepts `MarketClient` but doesn't use it (already uses `get_simple_quotes` from quotes module)
- `client.rs` - The MarketClient implementation itself
- `ws_proxy.rs` - May need review

## Migration Strategy

### Step 1: Create Shared Client Helper

Create a new module `backend/src/service/market_engine/client_helper.rs`:

- Function to initialize `YahooFinanceClient` with proper auth
- Reusable across all modules
- Handles `FetchClient`, `YahooAuthManager`, and `YahooFinanceClient` initialization
- Returns `Arc<YahooFinanceClient>` for sharing

### Step 2: Migrate Remaining Modules

#### 2.1 search.rs

- Replace `MarketClient.get("/v1/search")` with `YahooFinanceClient::search(query, hits)`
- Remove `MarketClient` parameter
- Use shared client helper

#### 2.2 transcripts.rs  

- Replace URL fetching with finance-query-core approach:
  - Use `scrape_earnings_calls_list()` to get earnings calls for symbol
  - Filter by quarter/year if provided
  - Use `get_earnings_transcript(event_id, company_id)` or `scrape_earnings_transcript_from_url()`
- May need to extract event_id and company_id from earnings calls list
- Remove `MarketClient` parameter

#### 2.3 hours.rs

- Replace `client.get("/hours")` with `YahooFinanceClient::get_market_status("us_market")`
- Adapt response format to match existing `MarketHours` struct
- Remove `MarketClient` parameter

#### 2.4 indicators.rs

- **Decision needed**: No direct equivalent in finance-query-core
- Option A: Use `get_fundamentals_timeseries()` if indicators can be mapped to fundamentals
- Option B: Remove indicators endpoint if not critical
- Option C: Implement using `make_request()` for custom Yahoo Finance endpoints
- **Action**: Check if indicators endpoint is used by frontend

#### 2.5 health.rs

- **Decision needed**: This is a custom endpoint, not Yahoo Finance
- Option A: Remove endpoint (health can be checked via other means)
- Option B: Keep as simple status check (no external call needed)
- **Action**: Check if health endpoint is used by frontend

### Step 3: Remove MarketClient Parameter from All Modules

Update all modules that accept `_client: &MarketClient` but don't use it:

- Remove parameter from function signatures
- Use shared client helper internally instead
- Files: `movers.rs`, `indices.rs`, `quotes.rs`, `historical.rs`, `news.rs`, `financials.rs`, `holders.rs`, `calendar.rs`, `analysts.rs`, `sectors.rs`, `watchlist_price.rs`

### Step 4: Update Route Handlers

Update `backend/src/routes/market.rs`:

- Remove `client_from_state()` function
- Remove `MarketClient` creation in handlers
- Update handlers to call module functions without client parameter
- All handlers should work without MarketClient

### Step 5: Remove MarketClient Implementation

- Delete or deprecate `backend/src/service/market_engine/client.rs`
- Remove `FinanceQueryConfig` dependency if no longer needed
- Update `mod.rs` to remove client module export

### Step 6: Update Configuration

- Remove `FinanceQueryConfig` from app state if MarketClient was the only consumer
- Or keep config if needed for other purposes

## Implementation Details

### Shared Client Helper Pattern

```rust
// client_helper.rs
pub async fn get_yahoo_client() -> Result<Arc<YahooFinanceClient>, YahooError> {
    let fetch_client = Arc::new(FetchClient::new(None)?);
    let cookie_jar = fetch_client.cookie_jar().clone();
    let auth_manager = Arc::new(YahooAuthManager::new(None, cookie_jar));
    let client = YahooFinanceClient::new(auth_manager.clone(), fetch_client);
    auth_manager.refresh().await?;
    Ok(Arc::new(client))
}
```

### Module Pattern After Migration

```rust
pub async fn get_xyz() -> Result<Value, YahooError> {
    use super::client_helper::get_yahoo_client;
    let client = get_yahoo_client().await?;
    client.get_xyz().await
}
```

## Files to Modify

1. **New file**: `backend/src/service/market_engine/client_helper.rs`
2. **Modify**: `backend/src/service/market_engine/search.rs`
3. **Modify**: `backend/src/service/market_engine/transcripts.rs`
4. **Modify**: `backend/src/service/market_engine/hours.rs`
5. **Modify**: `backend/src/service/market_engine/indicators.rs` (or remove)
6. **Modify**: `backend/src/service/market_engine/health.rs` (or remove)
7. **Modify**: All modules removing `MarketClient` parameter (10+ files)
8. **Modify**: `backend/src/service/market_engine/watchlist_price.rs`
9. **Modify**: `backend/src/service/market_engine/mod.rs`
10. **Modify**: `backend/src/routes/market.rs`
11. **Delete**: `backend/src/service/market_engine/client.rs` (after migration)

## Testing Considerations

- Verify all endpoints still work after migration
- Test authentication refresh handling
- Test error handling (network errors, parse errors)
- Verify no regressions in existing functionality
- Check frontend compatibility (response formats)

## Questions to Resolve

1. **indicators.rs**: Is this endpoint used? Should we remove it or implement via fundamentals_timeseries?
2. **health.rs**: Is this endpoint used? Should we remove it or keep as simple status?
3. **Client caching**: Should we cache the YahooFinanceClient instance or create new each time?
4. **Error handling**: How should we handle auth failures across modules?

## Success Criteria

- ✅ No `MarketClient.get()` calls remain
- ✅ All modules use `YahooFinanceClient` from finance-query-core
- ✅ `MarketClient` struct removed or deprecated
- ✅ All route handlers work without MarketClient
- ✅ No external URL dependencies (finance-query.onrender.com, api.tradstry.com)
- ✅ All tests pass
- ✅ Frontend continues to work without changes

### To-dos

- [x] Fix unused last_error warnings in model_selector.rs (remove initial assignment)
- [x] Fix serde_json::Error to YahooError conversion errors (use map_err)
- [x] Fix function call errors in holders.rs (remove extra None arguments)
- [x] Fix unused variable warnings (prefix with underscore)
- [x] Fix movers.rs type mismatch (convert tuple to Value)
- [ ] Create shared client helper module (client_helper.rs) for YahooFinanceClient initialization
- [ ] Migrate search.rs to use YahooFinanceClient::search()
- [ ] Migrate transcripts.rs to use scrape_earnings_calls_list() and get_earnings_transcript()
- [ ] Migrate hours.rs to use YahooFinanceClient::get_market_status()
- [ ] Decide on indicators.rs - migrate, remove, or implement via fundamentals_timeseries
- [ ] Decide on health.rs - remove or keep as simple status check
- [ ] Remove MarketClient parameter from all modules (movers, indices, quotes, historical, news, financials, holders, calendar, analysts, sectors)
- [ ] Update watchlist_price.rs to remove MarketClient dependency
- [ ] Update route handlers in market.rs to remove MarketClient creation
- [ ] Remove or deprecate client.rs (MarketClient implementation)
- [ ] Update mod.rs to remove client module export and add client_helper
- [ ] Test all endpoints and verify no regressions