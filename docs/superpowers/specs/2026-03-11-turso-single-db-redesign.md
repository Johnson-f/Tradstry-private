# Turso Module Redesign: Single Shared Database

**Date:** 2026-03-11
**Status:** Approved
**Scope:** `backend/src/turso/` only (models/routes update deferred)

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

**Tables with `user_id` added (33 tables):**
stocks, options, stock_positions, option_positions, position_transactions, trade_notes, playbook, playbook_rules, stock_trade_playbook, option_trade_playbook, stock_trade_rule_compliance, option_trade_rule_compliance, missed_trades, replicache_clients, replicache_space_version, trade_tags, stock_trade_tags, option_trade_tags, notebook_folders, notebook_notes, notebook_tags, notebook_note_tags, notebook_images, note_collaborators, note_invitations, note_share_settings, alert_rules, push_subscriptions, chat_sessions, chat_messages, ai_reports, ai_analysis, user_profile

**Global tables (no user_id):**
_schema_versions, notifications, notification_dispatch_log, public_notes_registry, invitations_registry, collaborators_registry

**Removed functions:**
- `initialize_user_database_schema()` — replaced by migration system
- `get_current_schema_version()` / `SchemaVersion` struct — replaced by `get_current_version()`
- `get_expected_schema()` / `get_current_tables()` / `update_table_schema()` / `create_table()` / `ensure_indexes()` / `ensure_triggers()` / `initialize_schema_version_table()` / `update_schema_version()` — all replaced by declarative migrations
- `TableSchema`, `ColumnInfo`, `IndexInfo`, `TriggerInfo` structs — no longer needed

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
- `store_user_database_entry()`, `get_user_database_entry()`, `get_all_user_database_entries()`
- `get_user_database_connection()` — replaced by `get_connection()`
- `sync_user_database_schema()`, `update_registry_schema_version()`
- All Turso API HTTP methods

**New/Changed methods:**
- `get_connection() -> Result<Connection>` — returns connection to the shared DB
- `get_user_db(user_id: &str) -> Result<UserDb>` — returns scoped wrapper

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

## Not In Scope

- Updating route handlers or model files to use `UserDb` / add `WHERE user_id` clauses
- Building cross-user feature queries
- Data migration (no existing users)

## Migration Strategy

Since there are no existing users, the migration is a clean start:
1. Deploy updated turso module
2. On first startup, migration v1 creates all tables with `user_id` columns
3. Update models/routes incrementally in follow-up work
