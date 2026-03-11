# Turso Module Redesign: Single Shared Database

**Date:** 2026-03-11
**Status:** Approved
**Scope:** `backend/src/turso/` plus minimal required changes in `main.rs` and services that directly depend on removed types/methods (models/routes `WHERE user_id` update deferred)

## Motivation

Enable cross-user features (leaderboards, shared analytics, social) that are impractical with the current database-per-user architecture. No existing users or data to migrate.

## Current Architecture

- Each user gets a separate Turso database created via the Turso API on sign-up
- A central "registry" database tracks user DB URLs/tokens and shared state (public notes, invitations, collaborators)
- ~25+ tables per user DB, ~213 route handlers query without `WHERE user_id` (isolation is at the DB level)
- Background sync task iterates all user DBs on startup to apply schema changes
- `TursoClient` manages Turso API calls for DB creation, token generation, and per-user connection caching

## New Architecture

Single shared Turso database. All user data lives in the same tables, isolated by `user_id` column. Versioned migration system replaces imperative schema creation.

## File-by-File Changes

### config.rs — Simplified

Remove `turso_api_token` and `turso_org` fields. Rename registry DB fields to generic DB fields.

```rust
pub struct TursoConfig {
    pub db_url: String,       // env: DATABASE_URL (single shared database)
    pub db_token: String,     // env: DATABASE_TOKEN
    pub supabase: SupabaseConfig,
    pub google: GoogleConfig,
    pub cron_secret: String,
    pub vector: VectorConfig,
    pub finance_query: FinanceQueryConfig,
    pub web_push: WebPushConfig,
    pub snaptrade_service_url: String,
}
```

**Env var changes:**
- `REGISTRY_DB_URL` -> `DATABASE_URL`
- `REGISTRY_DB_TOKEN` -> `DATABASE_TOKEN`
- `TURSO_API_TOKEN` — removed
- `TURSO_ORG` — removed

All other config structs (`SupabaseConfig`, `GoogleConfig`, `VectorConfig`, `FinanceQueryConfig`, `WebPushConfig`, `SupabaseClaims`, `AmrEntry`) remain unchanged.

### schema.rs — Versioned Migration System

Replace the monolithic `initialize_user_database_schema()` function with a migration system.

**New types:**

```rust
pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub up_sql: &'static str,
}
```

**Schema versions table (created before any migrations run):**

```sql
CREATE TABLE IF NOT EXISTS _schema_versions (
    version INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Core functions:**

- `get_migrations() -> Vec<Migration>` — returns all migrations ordered by version
- `get_current_version(conn) -> i64` — queries `_schema_versions` for max version (0 if empty)
- `migrate(conn) -> Result<()>` — runs pending migrations in a transaction, inserts into `_schema_versions` after each. Fails hard on error.

**Migration v1** creates all existing tables with these changes:
- Every user-data table gets `user_id TEXT NOT NULL` column
- Every user-data table gets `CREATE INDEX idx_{table}_user_id ON {table}(user_id)`
- Tables that already have `user_id` (alert_rules, push_subscriptions, chat_sessions) keep it as-is
- `user_profile` gets `user_id TEXT NOT NULL` added, with a unique constraint on it
- All existing triggers are preserved

**Tables with `user_id` added (41 tables):**
stocks, options, stock_positions, option_positions, position_transactions, trade_notes, playbook, playbook_rules, stock_trade_playbook, option_trade_playbook, stock_trade_rule_compliance, option_trade_rule_compliance, missed_trades, replicache_clients, replicache_space_version, trade_tags, stock_trade_tags, option_trade_tags, notebook_folders, notebook_notes, notebook_tags, notebook_note_tags, notebook_images, note_collaborators, note_invitations, note_share_settings, alert_rules, push_subscriptions, chat_sessions, chat_messages, ai_reports, ai_analysis, user_profile, brokerage_connections, brokerage_accounts, brokerage_transactions, brokerage_holdings, unmatched_transactions, calendar_connections, calendar_sync_cursors, calendar_events

**Global tables (no user_id):**
_schema_versions, notifications, notification_dispatch_log, public_notes_registry, invitations_registry, collaborators_registry

**Dropped tables (no longer created):**
user_databases — the per-user registry table is obsolete; user existence is tracked via `user_profile`

**Removed functions:**
- `initialize_user_database_schema()` — replaced by migration system
- `get_current_schema_version()` / `SchemaVersion` struct — replaced by `get_current_version()`
- `get_expected_schema()` / `get_current_tables()` / `update_table_schema()` / `create_table()` / `ensure_indexes()` / `ensure_triggers()` / `initialize_schema_version_table()` / `update_schema_version()` — all replaced by declarative migrations
- `TableSchema`, `ColumnInfo`, `IndexInfo`, `TriggerInfo` structs — no longer needed
- `get_table_columns()` — no longer needed

### client.rs — Simplified TursoClient

```rust
pub struct TursoClient {
    db: Database,
    config: TursoConfig,
}
```

**Constructor (`new`):**
1. Connect to the single shared database using `db_url` / `db_token`
2. Run `schema::migrate(&conn)` to ensure schema is up to date
3. No HTTP client needed, no registry table creation

**Removed:**
- `http_client: Client` field
- `UserDatabaseEntry` struct
- `TursoCreateDbResponse`, `TursoDatabaseInfo`, `TursoTokenResponse` structs
- `create_user_database()`, `create_database_via_api()`, `create_database_token()`
- `get_existing_database_info()`
- `store_user_database_entry()`, `get_user_database()`, `get_all_user_database_entries()`
- `get_user_database_connection()` — replaced by `get_connection()`
- `delete_user_database()`, `remove_user_database_entry()` — Turso API DB deletion no longer needed
- `sync_user_database_schema()`, `update_registry_schema_version()`
- All Turso API HTTP methods

**Renamed:**
- `get_registry_connection()` -> `get_connection()` — all callers updated (same DB, just a rename)

**New/Changed methods:**
- `get_connection() -> Result<Connection>` — returns connection to the shared DB
- `get_user_db(user_id: &str) -> Result<UserDb>` — returns scoped wrapper (always succeeds since the shared DB always exists; user existence is not validated here)

**Kept (operating on shared DB now, unchanged signatures):**
- `register_public_note()`, `unregister_public_note()`, `get_public_note_entry()`
- `register_invitation()`, `unregister_invitation()`, `get_invitation_entry()`
- `register_collaborator()`, `unregister_collaborator()`, `get_collaborator_entry()`
- `get_shared_notes_for_user()`
- `health_check()`
- `list_all_user_ids()` — now queries `SELECT DISTINCT user_id FROM user_profile`

**Kept structs:**
- `PublicNoteEntry`, `InvitationEntry`, `CollaboratorEntry`

### New: UserDb wrapper (in client.rs or its own file)

```rust
/// Scoped database access for a specific user.
/// Thin data holder — models use conn() and user_id() directly.
pub struct UserDb {
    conn: Connection,
    user_id: String,
}

impl UserDb {
    pub fn new(conn: Connection, user_id: String) -> Self {
        Self { conn, user_id }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}
```

### sync.rs — Deleted

No longer needed. Single DB migration runs on startup in `TursoClient::new()`.

### mod.rs — Updated

**Changes:**
- Remove `pub mod sync;`
- Remove `sync_all_user_schemas_background` usage
- Replace `AppState::get_user_db_connection()` with `AppState::get_user_db(user_id) -> Result<UserDb>`
- `AppState::new()` simplified — no background sync spawn
- Update re-exports

**Unchanged:**
- All service initializations (AI, cache, rate limiter, etc.)
- `health_check()`
- Public note, invitation, collaborator methods (delegate to TursoClient as before)

### auth.rs — Unchanged

No changes needed.

### redis.rs — Unchanged

No changes needed.

### vector_config.rs — Unchanged

No changes needed.

## Required External Changes (Compile-Breaking Dependencies)

These changes fall outside `backend/src/turso/` but are required because they directly reference removed types/methods and would prevent compilation.

### main.rs

- **`supabase_webhook_handler`** (`handle_user_created`): Currently calls `create_user_database()`. Replace with creating a `user_profile` row in the shared DB (user sign-up no longer creates a database).
- **`get_current_user` handler**: Currently calls `get_user_database()` and reads `UserDatabaseEntry` fields (`db_name`, `email`). Replace with a query to `user_profile` table.
- **`ENABLE_STARTUP_SCHEMA_SYNC` env var and sync spawn**: Remove this code block — sync.rs is deleted, migration runs in `TursoClient::new()`.

### service/utils/account_deletion.rs

- Currently calls `delete_user_database()` and `remove_user_database_entry()`. Replace with deleting user rows from all tables `WHERE user_id = ?` (no Turso API call needed).

### service/utils/storage_quota.rs

- Queries `user_databases.storage_used_bytes` via `get_registry_connection()` — both are removed. Add a `storage_used_bytes INTEGER DEFAULT 0` column to `user_profile` in migration v1. Replace `get_registry_connection()` with `get_connection()` and update queries to read/write `user_profile.storage_used_bytes` instead.

### service/brokerage/sync.rs, service/notifications/system.rs, service/notifications/earnings_calendar.rs

- Currently call `list_all_user_ids()` then `get_user_database_connection()` per user. Replace `get_user_database_connection()` calls with `get_user_db()` returning `UserDb`.
- Also call `get_registry_connection()` for dispatch log queries — replace with `get_connection()` (trivial rename, same DB).

### routes/user.rs, routes/trade_notes.rs

- Currently call `get_user_database()` returning `UserDatabaseEntry`. Replace with `get_user_db()` returning `UserDb`.

### All callers of `get_user_database_connection()` (blanket change)

The removed `TursoClient::get_user_database_connection()` method is called from ~20+ files across routes and services (stocks.rs, options.rs, playbook.rs, brokerage.rs, alerts.rs, push.rs, ai/chat.rs, ai/reports.rs, service/ai_service/interface/analysis_service.rs, service/ai_service/interface/chat_service.rs, service/market_engine/stream_service.rs, and others). Every call site must be mechanically updated:

- `turso_client.get_user_database_connection(&user_id)` -> `turso_client.get_user_db(&user_id)`
- `app_state.get_user_db_connection(&user_id)` -> `app_state.get_user_db(&user_id)`

The returned type changes from `Result<Option<Connection>>` to `Result<UserDb>` (no `Option` — the shared DB always exists). Call sites that unwrap the `Option` with `.ok_or_else(|| "User database not found")?` can drop that check. Call sites that use `conn` directly will use `user_db.conn()` instead (query logic changes deferred).

## Not In Scope

- Updating route handlers or model files to add `WHERE user_id` clauses to queries
- Building cross-user feature queries
- Data migration (no existing users)

## Migration Strategy

Since there are no existing users, the migration is a clean start:
1. Deploy updated turso module
2. On first startup, migration v1 creates all tables with `user_id` columns
3. Update models/routes incrementally in follow-up work
