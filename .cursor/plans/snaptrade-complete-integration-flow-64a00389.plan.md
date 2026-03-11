<!-- 64a00389-c2d2-4def-81db-fd8194d1c9b2 7e69c181-fc17-4e71-a4dd-69226d958a0d -->
# Complete SnapTrade Integration Flow

## Current State Analysis

**Existing Implementation:**

- User registration (has "user already exists" issue)
- Connection portal URL generation
- Connection status checking
- Basic account listing and syncing
- Holdings and transactions fetching (partial)

**Missing/Incomplete:**

- Proper handling when user already exists in SnapTrade
- Automatic post-connection flow (list accounts → get details → get positions → get transactions)
- Separate equity and option positions endpoints
- Account detail endpoint in Rust routes
- Proper transaction pagination support
- Error handling for empty positions/transactions

## Implementation Plan

### Phase 1: Fix User Already Exists Issue

**File: `backend/src/routes/brokerage.rs`**

- Update `get_or_create_snaptrade_user()` function
- When 400 error occurs (user exists), check if we have credentials stored
- If not stored, return clear error message explaining user needs to reconnect
- Store user_id and user_secret immediately after successful registration

**File: `backend/snaptrade-service/handlers/user.go`**

- Improve error message when user already exists
- Return structured error response

### Phase 2: Add Account Detail Endpoint

**File: `backend/src/routes/brokerage.rs`**

- Add `get_account_detail()` route handler
- Calls Go service `/api/v1/accounts/{accountId}`
- Returns full account details including balance, sync status, etc.

**File: `backend/src/routes/brokerage.rs` (route configuration)**

- Add route: `GET /api/brokerage/accounts/{id}/detail`

### Phase 3: Enhance Positions Endpoints

**File: `backend/snaptrade-service/client/snaptrade.go`**

- Verify `GetHoldings()` returns both equity and option positions
- Check if separate option positions endpoint exists in SDK
- If needed, add `GetOptionPositions()` method

**File: `backend/snaptrade-service/handlers/transaction.go`**

- Update `GetHoldings()` handler to separate equity vs options
- Add endpoint for option positions only: `GET /api/v1/accounts/{accountId}/holdings/options`
- Handle empty positions gracefully

**File: `backend/src/routes/brokerage.rs`**

- Add `get_account_positions()` route (equity)
- Add `get_account_option_positions()` route (options)
- Handle empty responses gracefully
- Store positions in database with proper type distinction

### Phase 4: Enhance Transactions Endpoint

**File: `backend/snaptrade-service/handlers/transaction.go`**

- Update `GetTransactions()` to support proper pagination
- Add query params: `offset`, `limit`, `start_date`, `end_date`
- Return pagination metadata in response

**File: `backend/src/routes/brokerage.rs`**

- Update `get_transactions()` to pass pagination params
- Store transactions with proper date parsing
- Handle empty transaction lists

### Phase 5: Post-Connection Automatic Sync Flow

**File: `backend/src/routes/brokerage.rs`**

- Add new route: `POST /api/brokerage/connections/{id}/complete`
- This endpoint is called after user returns from portal
- Flow:

  1. Verify connection status is "connected"
  2. List all accounts for the user
  3. For each account:

     - Get account detail
     - Get equity positions (handle empty)
     - Get option positions (handle empty)
     - Get historical transactions (with pagination)

  1. Store all data in database
  2. Return summary of synced data

**File: `backend/src/routes/brokerage.rs`**

- Update `get_connection_status()` to automatically trigger sync when status becomes "connected"
- Or create separate endpoint for manual sync trigger

### Phase 6: Database Schema Updates

**File: `backend/database/` (migration files)**

- Ensure `brokerage_accounts` table has all fields from account detail response
- Ensure `brokerage_holdings` table distinguishes equity vs options
- Ensure `brokerage_transactions` table supports all transaction fields
- Add indexes for performance

### Phase 7: Error Handling & Edge Cases

**All files:**

- Handle empty account lists
- Handle accounts with no positions (equity or options)
- Handle accounts with no transactions
- Handle API rate limits
- Handle network timeouts
- Log all errors with context

## Implementation Details

### Key Endpoints to Implement/Update

1. **GET /api/brokerage/accounts/{id}/detail** - Get account details
2. **GET /api/brokerage/accounts/{id}/positions** - Get equity positions
3. **GET /api/brokerage/accounts/{id}/positions/options** - Get option positions  
4. **GET /api/brokerage/accounts/{id}/transactions** - Get transactions (with pagination)
5. **POST /api/brokerage/connections/{id}/complete** - Complete post-connection sync

### Data Flow

```
User clicks "Connect" 
→ Register user (or get existing)
→ Generate portal URL
→ User completes connection in portal
→ User returns to app
→ Call /connections/{id}/complete
  → List accounts
  → For each account:
    → Get account detail
    → Get equity positions
    → Get option positions  
    → Get transactions
  → Store all in database
→ Return success with summary
```

### Response Formats

All endpoints should return consistent `ApiResponse<T>` format:

```rust
{
  "success": true,
  "data": { ... },
  "message": null
}
```

Error responses:

```rust
{
  "success": false,
  "data": null,
  "message": "Error description"
}
```

## Testing Considerations

- Test with user who has no accounts
- Test with user who has accounts but no positions
- Test with user who has equity but no options
- Test with user who has options but no equity
- Test with user who has no transactions
- Test pagination with large transaction lists
- Test "user already exists" scenario