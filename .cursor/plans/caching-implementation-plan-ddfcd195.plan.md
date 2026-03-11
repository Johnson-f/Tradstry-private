<!-- ddfcd195-6bcb-411c-ac41-54a3c9f95cc4 909d7889-eff7-42f3-ab2b-56471183caff -->
# Comprehensive Caching Implementation Plan

## Overview

Implement a two-tier caching strategy to reduce database writes and improve performance for 1,000 users:

- **Server-side**: Upstash Redis with Rust backend
- **Client-side**: React Query with stale-while-revalidate
- **Strategy**: Hybrid (TTL + manual invalidation on writes)
- **Scope**: Both user-specific and shared data caching

## Phase 1: Backend Setup - Upstash Redis Integration

### 1.1 Add Upstash Redis Dependencies

Add the Redis client crate to `backend/Cargo.toml`:

- Use `redis` crate (official Rust Redis client)
- Add `serde_json` for serialization (already present)

### 1.2 Create Redis Configuration Module

Create `backend/src/turso/redis.rs`:

- Define `RedisConfig` struct with URL and token from environment
- Implement connection pooling with `r2d2` or `deadpool-redis`
- Add helper methods for common operations (get, set, del, expire)

### 1.3 Create Cache Service Layer

Create `backend/src/service/cache_service.rs`:

- Implement `CacheService` struct with Redis client
- Define cache key patterns (e.g., `user:{userId}:stocks`, `analytics:stocks:{userId}:{timeRange}`)
- Implement generic cache methods: `get<T>()`, `set<T>()`, `invalidate()`, `invalidate_pattern()`
- Add TTL constants for different data types
- **Add `preload_user_data()` method** to fetch and cache all user data at once

### 1.4 Update AppState

Modify `backend/src/turso/mod.rs`:

- Add `redis_client` to `AppState`
- Initialize Redis connection in `AppState::new()`

### 1.5 Enhance `/user/initialize` Endpoint (CACHE PRELOAD)

Update `backend/src/routes/user.rs` - `initialize_user_database()`:

**Current behavior:**

- Creates/checks user database
- Initializes schema
- Returns database URL and token

**New behavior (add after existing logic):**

1. After successful database initialization/check
2. **Preload ALL user data into Redis cache:**

   - Fetch all stocks from `stocks` table
   - Fetch all options from `options` table
   - Fetch all trade notes from `trade_notes` table
   - Fetch all playbooks from `playbook` table
   - Fetch all notebook notes from `notebook_notes` table
   - Fetch all images metadata from `images` table
   - Fetch calendar connections from `external_calendar_connections`

3. **Store in Redis with appropriate TTLs:**

   - `user:{userId}:stocks:all` → 30 min TTL
   - `user:{userId}:options:all` → 30 min TTL
   - `user:{userId}:notes:all` → 30 min TTL
   - `user:{userId}:playbooks:all` → 1 hour TTL
   - `user:{userId}:notebook:all` → 10 min TTL
   - `user:{userId}:images:all` → 1 hour TTL

4. **Pre-compute and cache analytics:**

   - Calculate stock analytics for all time ranges
   - Calculate option analytics for all time ranges
   - Store in `analytics:stocks:{userId}:{timeRange}` with 15 min TTL

5. Return success response with cache status

**Implementation pattern:**

```rust
// After database initialization succeeds
let cache_service = app_state.cache_service.clone();
let user_conn = get_user_db_connection(&user_id, &turso_client).await?;

// Preload all user data in parallel
tokio::spawn(async move {
    cache_service.preload_user_data(&user_conn, &user_id).await;
});
```

## Phase 2: Backend Caching Implementation

### 2.1 Cache Stock Analytics Endpoints

Update `backend/src/routes/stocks.rs`:

- **GET /api/stocks/analytics**: Cache for 15 minutes, invalidate on trade mutations
- **GET /api/stocks/analytics/pnl**: Cache for 30 minutes
- **GET /api/stocks/analytics/profit-factor**: Cache for 15 minutes
- **GET /api/stocks/analytics/win-rate**: Cache for 15 minutes
- Pattern: Check cache → Return if hit → Query DB → Store in cache → Return

### 2.2 Cache Options Analytics Endpoints

Update `backend/src/routes/options.rs`:

- Apply same caching strategy as stocks analytics
- Use separate cache keys: `analytics:options:{userId}:{timeRange}`

### 2.3 Cache Trade Lists

Update `backend/src/routes/stocks.rs` and `backend/src/routes/options.rs`:

- **GET /api/stocks**: Cache for 30 minutes, invalidate on create/update/delete
- **GET /api/options**: Cache for 30 minutes, invalidate on create/update/delete
- Cache key: `trades:stocks:{userId}:list:{filters_hash}` and `trades:options:{userId}:list:{filters_hash}`

### 2.4 Implement Cache Invalidation on Mutations

Update all mutation endpoints:

- **POST /api/stocks**: Invalidate `trades:stocks:{userId}:*` and `analytics:stocks:{userId}:*`
- **PUT /api/stocks/{id}**: Same invalidation pattern
- **DELETE /api/stocks/{id}**: Same invalidation pattern
- **POST /api/options**: Invalidate `trades:options:{userId}:*` and `analytics:options:{userId}:*`
- Use Redis pattern matching for bulk invalidation

### 2.5 Cache Trade Notes & Images

Update `backend/src/routes/trade_notes.rs` and `backend/src/routes/images.rs`:

- **GET /api/trade-notes**: Cache for 30 minutes, TTL: 1800s
- **GET /api/images**: Cache for 1 hour, TTL: 3600s
- Invalidate on create/update/delete operations

### 2.6 Cache Notebook & Calendar Data

Update `backend/src/routes/notebook.rs`:

- **GET /api/notebook/notes**: Cache for 10 minutes, TTL: 600s
- **GET /api/notebook/calendar/events**: Cache for 5 minutes, TTL: 300s
- **GET /api/notebook/holidays**: Cache for 24 hours, TTL: 86400s (rarely changes)

### 2.7 Cache Playbook Data

Update `backend/src/routes/playbook.rs`:

- **GET /api/playbook**: Cache for 1 hour, TTL: 3600s
- Invalidate on playbook mutations

## Phase 3: Client-Side Caching Setup

### 3.1 Update React Query Configuration

Modify `app/providers.tsx`:

- Set default `staleTime`: 5 minutes (300000ms)
- Set default `gcTime`: 15 minutes (900000ms)
- Enable `refetchOnWindowFocus: false` globally
- Configure retry logic for failed requests

### 3.2 Create Cache Utilities

Create `lib/utils/cache-keys.ts`:

- Define cache key factories for all data types
- Example: `stocksKeys.list(filters)`, `analyticsKeys.stocks(userId, timeRange)`
- Ensures consistent cache key generation

### 3.3 Update Analytics Hooks

Modify `lib/hooks/use-analytics.ts`:

- Already uses React Query with 5-minute stale time ✓
- Add cache invalidation helpers
- Implement optimistic updates for mutations

### 3.4 Update Daily PnL Hook

Modify `hooks/use-daily-pnl.ts`:

- Already uses React Query with 5-minute stale time ✓
- Ensure proper cache key structure

### 3.5 Create Mutation Hooks with Cache Invalidation

Create `lib/hooks/use-trade-mutations.ts`:

- Implement `useCreateStock`, `useUpdateStock`, `useDeleteStock`
- Implement `useCreateOption`, `useUpdateOption`, `useDeleteOption`
- Automatically invalidate related queries on success
- Use `queryClient.invalidateQueries()` for cache busting

### 3.6 Implement Optimistic Updates

Update mutation hooks:

- Use `onMutate` to update cache optimistically
- Rollback on error using `onError`
- Refetch on success using `onSuccess`

## Phase 4: Shared Data Caching

### 4.1 Cache Market Data

Update `lib/services/market-data-service.ts`:

- Cache quote data for 2 minutes (external API data)
- Cache market movers for 5 minutes
- Cache earnings data for 1 hour
- Use shared cache keys: `market:quotes:{symbols}`, `market:movers:gainers`

### 4.2 Cache Public Holidays

Already implemented in backend:

- 24-hour TTL for public holidays
- Shared across all users

## Phase 5: Monitoring & Optimization

### 5.1 Add Cache Metrics

Create `backend/src/service/cache_metrics.rs`:

- Track cache hit/miss rates
- Log cache performance
- Add health check endpoint for Redis connection

### 5.2 Add Logging

Update all cached endpoints:

- Log cache hits/misses
- Log invalidation operations
- Monitor cache performance

### 5.3 Create Cache Management Endpoints

Create `backend/src/routes/cache.rs`:

- **POST /api/cache/clear**: Clear user-specific cache (admin/debug)
- **GET /api/cache/stats**: Get cache statistics
- Protected by authentication

## Cache TTL Strategy Summary

| Data Type | TTL | Invalidation | Reason |

|-----------|-----|--------------|--------|

| Stock/Option Lists | 30 min | On mutations | Frequently read, changes on user action |

| Analytics (all) | 15 min | On trade changes | Expensive computation |

| Trade Notes | 30 min | On mutations | Moderate read frequency |

| Images | 1 hour | On mutations | Rarely change |

| Notebook Notes | 10 min | On mutations | Frequently edited |

| Calendar Events | 5 min | On sync | Real-time updates needed |

| Public Holidays | 24 hours | Manual | Rarely changes |

| Market Data | 2-5 min | None (TTL only) | External API, frequent updates |

| Playbooks | 1 hour | On mutations | Rarely change |

## Key Implementation Files

**Backend:**

- `backend/src/turso/redis.rs` - Redis configuration
- `backend/src/service/cache_service.rs` - Cache service layer
- `backend/src/routes/stocks.rs` - Stock endpoints with caching
- `backend/src/routes/options.rs` - Options endpoints with caching
- `backend/src/routes/trade_notes.rs` - Trade notes with caching
- `backend/src/routes/notebook.rs` - Notebook with caching

**Frontend:**

- `app/providers.tsx` - React Query configuration
- `lib/utils/cache-keys.ts` - Cache key factories
- `lib/hooks/use-trade-mutations.ts` - Mutation hooks with invalidation
- `lib/hooks/use-analytics.ts` - Analytics hooks (update)

## Environment Variables

Add to `backend/.env`:

```
UPSTASH_REDIS_REST_URL=your_redis_url
UPSTASH_REDIS_REST_TOKEN=your_redis_token
```

## Testing Strategy

1. Test cache hit/miss rates
2. Verify invalidation on mutations
3. Test TTL expiration
4. Load test with 1,000 concurrent users
5. Monitor database write reduction
6. Verify data consistency after invalidation

### To-dos

- [ ] Add Upstash Redis dependencies and create Redis configuration module
- [ ] Create cache service layer with generic get/set/invalidate methods
- [ ] Implement caching for stock and options analytics endpoints
- [ ] Implement caching for trade lists (stocks and options)
- [ ] Implement cache invalidation on all mutation endpoints
- [ ] Implement caching for trade notes, images, and notebook data
- [ ] Update React Query configuration with optimal defaults
- [ ] Create cache key factories for consistent key generation
- [ ] Create mutation hooks with automatic cache invalidation
- [ ] Implement optimistic updates for better UX
- [ ] Implement shared caching for market data and public holidays
- [ ] Add cache metrics, logging, and management endpoints