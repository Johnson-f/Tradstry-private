<!-- 47f4eeb4-4cab-4a73-8f32-b73b372877bb 30df6f19-48b5-4a6b-91ba-d420395df62d -->
# Add Open/Closed Trade Filtering for Stocks and Options

## Overview

Add `openOnly` query parameter to `/api/stocks` and `/api/options` endpoints to filter open or closed trades. When fetching open trades, return only essential fields (symbol, entry_price, entry_date) for performance.

## Implementation Steps

### 1. Add Response Types for Simplified Open Trades

**File**: `backend/src/models/stock/stocks.rs`

- Add `OpenStockTrade` struct with only `symbol`, `entry_price`, `entry_date`
- Add `OpenOptionTrade` struct with only `symbol`, `entry_price`, `entry_date`

### 2. Update StockQuery Structure

**File**: `backend/src/models/stock/stocks.rs`

- Add `open_only: Option<bool>` field to `StockQuery` struct (line ~273)
- Update `find_all` method (line ~431) to:
  - Filter by `exit_date IS NULL` when `open_only == Some(true)`
  - Filter by `exit_date IS NOT NULL` when `open_only == Some(false)`
  - No filter when `open_only == None`

### 3. Update OptionQuery Structure

**File**: `backend/src/models/options/option_trade.rs`

- Add `open_only: Option<bool>` field to `OptionQuery` struct (line ~180)
- Update `find_all` method (line ~312) to:
  - Filter by `(status = 'open' OR exit_date IS NULL)` when `open_only == Some(true)`
  - Filter by `(status = 'closed' OR exit_date IS NOT NULL)` when `open_only == Some(false)`
  - No filter when `open_only == None`

### 4. Add Helper Methods for Simplified Responses

**File**: `backend/src/models/stock/stocks.rs`

- Add `find_all_open_summary` method that:
  - Queries only `symbol`, `entry_price`, `entry_date` columns
  - Filters by `exit_date IS NULL`
  - Returns `Vec<OpenStockTrade>`

**File**: `backend/src/models/options/option_trade.rs`

- Add `find_all_open_summary` method that:
  - Queries only `symbol`, `entry_price`, `entry_date` columns
  - Filters by `(status = 'open' OR exit_date IS NULL)`
  - Returns `Vec<OpenOptionTrade>`

### 5. Update Stock Route Handler

**File**: `backend/src/routes/stocks.rs`

- Update `get_all_stocks` handler (line ~387) to:
  - Check if `query.open_only == Some(true)`
  - If true, call `Stock::find_all_open_summary()` and return simplified response
  - If false or None, use existing `Stock::find_all()` logic

### 6. Update Option Route Handler

**File**: `backend/src/routes/options.rs`

- Find `get_all_options` handler (similar to stocks)
- Update to:
  - Check if `query.open_only == Some(true)`
  - If true, call `OptionTrade::find_all_open_summary()` and return simplified response
  - If false or None, use existing `OptionTrade::find_all()` logic

### 7. Update Frontend Types (Optional)

**Files**: Frontend TypeScript types if needed

- Add `openOnly?: boolean` to query parameter types
- Add `OpenStockTrade` and `OpenOptionTrade` response types

## Database Query Logic

### Stocks

- **Open**: `WHERE exit_date IS NULL`
- **Closed**: `WHERE exit_date IS NOT NULL`

### Options

- **Open**: `WHERE (status = 'open' OR exit_date IS NULL)`
- **Closed**: `WHERE (status = 'closed' OR exit_date IS NOT NULL)`

## API Usage Examples

```
GET /api/stocks?openOnly=true          # Open trades (simplified)
GET /api/stocks?openOnly=false         # Closed trades (full)
GET /api/stocks                        # All trades (full)

GET /api/options?openOnly=true         # Open trades (simplified)
GET /api/options?openOnly=false        # Closed trades (full)
GET /api/options                       # All trades (full)
```

## Response Format

### Open Trades (openOnly=true)

```json
{
  "success": true,
  "data": [
    {
      "symbol": "AAPL",
      "entry_price": 150.0,
      "entry_date": "2024-01-15T10:00:00Z"
    }
  ]
}
```

### Closed/All Trades (openOnly=false or omitted)

```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "symbol": "AAPL",
      "entry_price": 150.0,
      "exit_price": 155.0,
      "entry_date": "2024-01-15T10:00:00Z",
      "exit_date": "2024-01-20T15:00:00Z",
      // ... all other fields
    }
  ]
}
```