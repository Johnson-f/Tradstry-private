<!-- 7afd3ee7-cad5-4fd1-8787-3863b2081319 c1c83692-52f1-4493-b406-2b4e9bc62c57 -->
# Storage Quota Implementation Plan

## Overview

Implement a per-user storage quota system that limits each user's database to 18 MB. Users cannot save new data once the quota is exceeded, but can delete existing data to free up space.

## Implementation Approach

### 1. Storage Quota Service (`backend/src/service/storage_quota.rs`)

   - Similar pattern to `RateLimiter`
   - Methods:
     - `check_storage_quota()` - Calculate current database size and check if within limit
     - `get_storage_usage()` - Get current usage and remaining quota
     - `calculate_database_size()` - Query SQLite `page_count * page_size` to get actual size
   - Quota limit: 18 MB (18,874,368 bytes)

### 2. Database Schema Update

   - Add `storage_used_bytes` column to `user_databases` table in registry DB
   - Initialize to 0 for new users
   - Track actual size from SQLite `PRAGMA page_count` and `PRAGMA page_size`

### 3. Storage Check Integration Points

   - **Before INSERT operations**: Check quota in all create handlers:
     - `routes/stocks.rs` - `create_stock()`
     - `routes/options.rs` - `create_option()`
     - `routes/trade_notes.rs` - create handlers
     - `routes/images.rs` - image upload handlers
     - `routes/notebook.rs` - notebook entry handlers
     - `routes/ai_insights.rs` - AI chat message handlers
   - **Before UPDATE operations**: Decide based on user preference (updates may not increase size)
   - Return appropriate HTTP error (413 Payload Too Large or 507 Insufficient Storage) if quota exceeded

### 4. Quota Update Strategy

   - **Option A (Real-time)**: Calculate size before each write operation (accurate but slower)
   - **Option B (Cached)**: Cache size calculation, update after INSERT/DELETE operations (faster but may drift)
   - **Recommendation**: Hybrid approach - check cached value, verify periodically

### 5. Helper Functions

   - Create `get_user_db_size()` function that queries:
     ```sql
     PRAGMA page_count;
     PRAGMA page_size;
     -- size = page_count * page_size
     ```

   - Update `storage_used_bytes` in registry after successful operations
   - Recalculate on DELETE operations to free up quota

### 6. Error Handling

   - Create `StorageQuotaError` enum:
     - `QuotaExceeded { used: u64, limit: u64 }`
     - `DatabaseError` for calculation failures
   - Return user-friendly error messages in API responses

### 7. Initialization

   - Update existing users' `storage_used_bytes` via migration
   - Calculate initial size for all existing databases on first quota check

## Files to Modify

1. `backend/database/01_users_table.sql` - Add `storage_used_bytes` column migration
2. `backend/src/service/storage_quota.rs` - New service file
3. `backend/src/service/mod.rs` - Export storage quota service
4. `backend/src/turso/mod.rs` - Add storage quota service to AppState
5. `backend/src/routes/stocks.rs` - Add quota check in create handler
6. `backend/src/routes/options.rs` - Add quota check in create handler
7. `backend/src/routes/trade_notes.rs` - Add quota check in create handlers
8. `backend/src/routes/images.rs` - Add quota check in upload handlers
9. `backend/src/routes/notebook.rs` - Add quota check in create handlers
10. `backend/src/routes/ai_insights.rs` - Add quota check in chat message handlers
11. `backend/src/models/*/delete.rs` - Update quota after successful deletions

## Open Questions for User

1. Should UPDATE operations also check quota? (Updates may not increase storage)
2. Should all tables count, or exclude system tables?
3. Preference for real-time vs cached quota tracking?
4. What HTTP status code for quota exceeded? (413 or 507)

### To-dos

- [ ] Create StorageQuotaService with methods to calculate database size and check quota limits
- [ ] Add storage_used_bytes column to user_databases table in registry database
- [ ] Add quota checks before INSERT operations in all create route handlers (stocks, options, trade_notes, images, notebook, ai_insights)
- [ ] Update storage_used_bytes after successful INSERT and DELETE operations
- [ ] Implement StorageQuotaError enum and return appropriate HTTP error responses
- [ ] Create migration script to calculate and set initial storage_used_bytes for existing users