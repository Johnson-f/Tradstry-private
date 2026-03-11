<!-- 3defc295-6d71-4c4a-abf2-6d0e177309ab c5cf198b-7125-48bd-88ee-ce65b1f83a5c -->
# Trade Analytics and Vectorization Enhancement Plan (Final)

## Overview

Create structured Qdrant payload with symbol, analytics, and implement **dual-trigger vectorization** so trades are searchable whether they have just mistakes, just notes, or both.

## Understanding Current User Flow

### When User Creates Trade

```typescript
// Frontend: create-trade-modal.tsx
User fills form:
- Symbol: APLD
- Entry/Exit prices, dates
- Mistakes: "Entered too early" ← OPTIONAL, can be added now
→ POST /api/stocks (with mistakes field)
→ Trade saved to database
→ ❌ NO vectorization happens
```

### When User Adds Notes (Later, Maybe Never)

```typescript
// User clicks trade → Adds notes separately
POST /api/trades/stock/123/notes
  → upsert_trade_note_for_trade()
  → trade_notes_service.upsert_trade_note()
  → ✅ Vectorization triggered (mistakes + notes)
```

## Problem Statement

**Current Behavior**:

- Trade with mistakes only → NOT vectorized (not searchable) ❌
- Trade with notes only → Vectorized ✅
- Trade with mistakes + notes → Vectorized ✅

**Desired Behavior**:

- Trade with mistakes only → Vectorized (searchable) ✅
- Trade with notes only → Vectorized ✅  
- Trade with mistakes + notes → Vectorized ✅

## Solution: Dual-Trigger Vectorization

### Trigger 1: On Trade Creation (if mistakes exist)

```rust
// backend/src/routes/stocks.rs - create_stock()
after Stock::create():
  if mistakes.is_some() && !mistakes.as_deref().unwrap_or("").trim().is_empty() {
    // Trigger vectorization immediately
    spawn_vectorization(user_id, stock.id, "stock", conn)
  }
```

### Trigger 2: On Trade Note Upsert (current, keep as-is)

```rust
// backend/src/routes/trade_notes.rs - upsert_trade_note_for_trade()
after trade_notes_service.upsert_trade_note():
  // Triggers vectorization (re-vectorizes with updated content)
  // This already exists, keep it
```

## New Qdrant Payload Structure

```json
{
  "user_id": "f9e31db3-4cd8-48fc-88e0-a52f74ae4b12",
  "id": "trade-123-mistakes-notes",
  "type": "trade",
  "created_at": "2025-11-21T20:26:17Z",
  "symbol": "APLD",
  "trade_type": "stock",
  "trade_id": 123,
  "analytics": {
    "net_pnl": 450.50,
    "net_pnl_percent": 15.2,
    "position_size": 100.0,
    "stop_loss_risk_percent": 5.2,
    "risk_to_reward": 3.1,
    "entry_price": 29.50,
    "exit_price": 34.00,
    "hold_time_days": 5.3
  },
  "content": {
    "mistakes": "Entered too early",
    "trade_notes": "Analysis: Should have waited for confirmation"
  }
}
```

## Implementation Steps

### 1. Create TradeVectorization Service

**New File**: `backend/src/service/ai_service/vector_service/vectors/trade.rs`

```rust
pub struct TradeVectorization {
    voyager_client: Arc<VoyagerClient>,
    qdrant_client: Arc<QdrantDocumentClient>,
}

impl TradeVectorization {
    pub async fn vectorize_trade(
        &self,
        user_id: &str,
        trade_id: i64,
        trade_type: &str,  // "stock" or "option"
        conn: &Connection,
    ) -> Result<()> {
        // Step 1: Fetch trade (Stock or OptionTrade)
        // Step 2: Calculate analytics
        // Step 3: Fetch trade notes (if any)
        // Step 4: Format content for embedding
        // Step 5: Generate embedding
        // Step 6: Create structured payload
        // Step 7: Store in Qdrant
    }

    fn calculate_trade_analytics(/* ... */) -> TradeAnalytics {
        // Calculate: net_pnl, stop_loss_risk_percent, position_size, risk_to_reward, etc.
    }
}
```

### 2. Add Structured Upsert to Qdrant

**File**: `backend/src/service/ai_service/vector_service/qdrant.rs`

```rust
pub async fn upsert_trade_vector_structured(
    &self,
    user_id: &str,
    vector_id: &str,
    symbol: &str,
    trade_type: &str,
    trade_id: i64,
    analytics: HashMap<String, f64>,
    content: HashMap<String, String>,
    embedding: &[f32],
) -> Result<()> {
    // Create payload with all structured fields
    let mut payload = HashMap::new();
    payload.insert("user_id", Value::from(user_id));
    payload.insert("id", Value::from(vector_id));
    payload.insert("type", Value::from("trade"));
    payload.insert("symbol", Value::from(symbol));
    payload.insert("trade_type", Value::from(trade_type));
    payload.insert("trade_id", Value::from(trade_id));
    // ... add analytics and content as nested objects
}
```

### 3. Add Vectorization Trigger to Stock Routes

**File**: `backend/src/routes/stocks.rs`

```rust
pub async fn create_stock(
    req: HttpRequest,
    body: web::Bytes,
    app_state: web::Data<AppState>,
    supabase_config: web::Data<SupabaseConfig>,
    trade_vectorization: web::Data<Arc<TradeVectorization>>,  // NEW
    // ... other params
) -> Result<HttpResponse> {
    // ... existing code to create stock ...
    
    match Stock::create(&conn, payload).await {
        Ok(stock) => {
            // ... existing cache invalidation, websocket broadcast ...
            
            // NEW: Trigger vectorization if mistakes exist
            if let Some(ref mistakes) = stock.mistakes {
                if !mistakes.trim().is_empty() {
                    let trade_vec = trade_vectorization.clone();
                    let user_id_clone = user_id.clone();
                    let conn_clone = conn.clone();  // Clone connection
                    
                    tokio::spawn(async move {
                        if let Err(e) = trade_vec.vectorize_trade(
                            &user_id_clone,
                            stock.id,
                            "stock",
                            &conn_clone,
                        ).await {
                            error!("Failed to vectorize stock trade: {}", e);
                        }
                    });
                }
            }
            
            Ok(HttpResponse::Created().json(ApiResponse::success(stock)))
        }
        // ... error handling ...
    }
}
```

### 4. Update Trade Notes Service

**File**: `backend/src/service/trade_notes_service.rs`

Replace call to old `TradeVectorService.vectorize_trade_mistakes_and_notes()` with new `TradeVectorization.vectorize_trade()`.

### 5. Update Module Exports

**Files**:

- `backend/src/service/ai_service/vector_service/vectors/mod.rs`
- `backend/src/service/ai_service/vector_service/mod.rs`

Add `TradeVectorization` to exports.

## Vectorization Scenarios

| Scenario | Trigger | What Gets Vectorized |

|----------|---------|---------------------|

| Create trade with mistakes | Trade creation | Mistakes only (no notes yet) |

| Create trade, add notes later | Trade note upsert | Mistakes + notes (re-vectorizes) |

| Create trade with no mistakes, add notes later | Trade note upsert | Notes only |

| Create trade, update mistakes later | Trade update | Mistakes (re-vectorizes) |

## Content for Embedding

Text format for embedding generation:

```
Trade: APLD (stock #123)
Entry: $29.50, Exit: $34.00
Net P&L: $450.50 (15.2% gain)
Position: 100 shares
Stop Loss Risk: 5.2%
Risk/Reward: 3.1
Hold Time: 5.3 days

Mistakes: Entered too early

Trade Notes:
Analysis: Should have waited for confirmation
```

## Implementation Order

1. Create `vectors/trade.rs` with `TradeVectorization` struct
2. Implement `calculate_trade_analytics()` method
3. Add `upsert_trade_vector_structured()` to `qdrant.rs`
4. Implement `vectorize_trade()` method
5. Update module exports
6. Add vectorization trigger to `create_stock()` in `stocks.rs`
7. Add vectorization trigger to `create_option()` in `options.rs`
8. Update `trade_notes_service.rs` to use new service
9. Test all scenarios

## Files to Create/Modify

### Create

- `backend/src/service/ai_service/vector_service/vectors/trade.rs`

### Modify

- `backend/src/service/ai_service/vector_service/qdrant.rs`
- `backend/src/service/ai_service/vector_service/vectors/mod.rs`
- `backend/src/service/ai_service/vector_service/mod.rs`
- `backend/src/routes/stocks.rs`
- `backend/src/routes/options.rs`
- `backend/src/service/trade_notes_service.rs`
- `backend/src/service/ai_service/interface/chat_service.rs`

## Benefits

1. **Always Searchable**: Trades are vectorized whether they have mistakes, notes, or both
2. **Rich Context**: Chat system gets structured analytics data
3. **Precise Queries**: Can filter by symbol, trade_type, P&L ranges in Qdrant
4. **Incremental Updates**: Re-vectorizes when notes are added without duplicates
5. **Type Safety**: Analytics stored as proper numbers, not strings

### To-dos

- [ ] Add trade_note_vector TEXT column to stocks and options tables in schema.rs with migration logic
- [ ] Add trade_note_vector Option<String> field to Stock and OptionTrade structs, update from_row methods
- [ ] Add stop_loss_risk_percent and position_size_shares to IndividualTradeAnalytics struct and calculation functions
- [ ] Modify vectorize_trade_mistakes_and_notes to update trade record with vector ID after vectorization
- [ ] Ensure trade notes service triggers vectorization and updates trade_note_vector field
- [ ] Enhance chat/QA system to include trade analytics and symbol in context for trade-specific queries