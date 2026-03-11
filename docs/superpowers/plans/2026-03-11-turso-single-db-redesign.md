# Turso Single Shared Database Redesign — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the database-per-user Turso architecture with a single shared database, versioned migration system, and `UserDb` scoped wrapper.

**Architecture:** Single Turso database with `user_id TEXT NOT NULL` on all user-data tables. Versioned migration system (`_schema_versions` table) runs on startup. `UserDb` wrapper holds `(Connection, user_id)` for scoped access. All Turso API calls for per-user DB creation removed.

**Tech Stack:** Rust, libsql, Turso, actix-web

**Spec:** `docs/superpowers/specs/2026-03-11-turso-single-db-redesign.md`

---

## Chunk 1: Core Turso Module Rewrite

### Task 1: Rewrite config.rs

**Files:**
- Modify: `backend/src/turso/config.rs:1-90`

- [ ] **Step 1: Remove turso_api_token and turso_org, rename registry fields**

Replace the `TursoConfig` struct and its `from_env()`:

```rust
/// Configuration for Turso database connections
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TursoConfig {
    /// Single shared database URL
    pub db_url: String,
    /// Single shared database auth token
    pub db_token: String,
    /// Supabase configuration
    pub supabase: SupabaseConfig,
    /// Google OAuth configuration
    pub google: GoogleConfig,
    /// Cron secret for external sync endpoint
    pub cron_secret: String,
    /// Vector database configuration
    pub vector: VectorConfig,
    /// FinanceQuery market data configuration
    pub finance_query: FinanceQueryConfig,
    /// Web Push (VAPID) configuration
    pub web_push: WebPushConfig,
    /// SnapTrade service URL
    pub snaptrade_service_url: String,
}

impl TursoConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let supabase_config = SupabaseConfig::from_env()?;
        let google_config = GoogleConfig::from_env()?;
        let vector_config = VectorConfig::from_env()?;
        let finance_query_config = FinanceQueryConfig::from_env()?;
        let web_push_config = WebPushConfig::from_env()?;

        Ok(Self {
            db_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL environment variable not set")?,
            db_token: env::var("DATABASE_TOKEN")
                .map_err(|_| "DATABASE_TOKEN environment variable not set")?,
            supabase: supabase_config,
            google: google_config,
            cron_secret: env::var("CRON_SECRET")
                .map_err(|_| "CRON_SECRET environment variable not set")?,
            vector: vector_config,
            finance_query: finance_query_config,
            web_push: web_push_config,
            snaptrade_service_url: env::var("SNAPTRADE_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        })
    }
}
```

All other structs in config.rs (`SupabaseConfig`, `GoogleConfig`, `VectorConfig`, `FinanceQueryConfig`, `WebPushConfig`, `SupabaseClaims`, `AmrEntry`) remain untouched.

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | head -30`
Expected: Errors about `config.turso_api_token`, `config.turso_org`, `config.registry_db_url`, `config.registry_db_token` in client.rs — this is correct, we fix those in the next task.

- [ ] **Step 3: Commit**

```bash
git add backend/src/turso/config.rs
git commit -m "refactor(turso): simplify TursoConfig for single shared database"
```

---

### Task 2: Rewrite schema.rs — Migration System

**Files:**
- Rewrite: `backend/src/turso/schema.rs` (full rewrite — current file is 6158 lines)

This is the largest task. The current file has monolithic `initialize_user_database_schema()` plus schema diffing infrastructure. Replace with a clean migration system.

- [ ] **Step 1: Write the new schema.rs with Migration type and migrate() function**

Replace the entire file. The new file has three parts:
1. `Migration` struct and `get_migrations()` returning all migrations
2. `get_current_version()` and `migrate()` functions
3. Migration v1 SQL string containing all table CREATE statements with `user_id`

```rust
use anyhow::Result;
use libsql::Connection;
use log::info;

/// A single schema migration
pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub up_sql: &'static str,
}

/// Returns all migrations in order
pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "Initial schema with user_id columns",
            up_sql: MIGRATION_V1,
        },
    ]
}

/// Get the current schema version from the database (0 if no migrations applied)
pub async fn get_current_version(conn: &Connection) -> Result<i64> {
    // Ensure the schema versions table exists
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS _schema_versions (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        libsql::params![],
    )
    .await?;

    let mut rows = conn
        .query("SELECT COALESCE(MAX(version), 0) FROM _schema_versions", libsql::params![])
        .await?;

    if let Some(row) = rows.next().await? {
        Ok(row.get::<i64>(0)?)
    } else {
        Ok(0)
    }
}

/// Run all pending migrations. Fails hard on error.
pub async fn migrate(conn: &Connection) -> Result<()> {
    let current_version = get_current_version(conn).await?;
    let migrations = get_migrations();

    let pending: Vec<&Migration> = migrations
        .iter()
        .filter(|m| m.version > current_version)
        .collect();

    if pending.is_empty() {
        info!("Schema is up to date at version {}", current_version);
        return Ok(());
    }

    info!(
        "Running {} pending migration(s) (current: v{}, target: v{})",
        pending.len(),
        current_version,
        pending.last().unwrap().version
    );

    for migration in pending {
        info!(
            "Applying migration v{}: {}",
            migration.version, migration.description
        );

        // Execute all statements in the migration SQL.
        // We split on semicolons but must respect BEGIN...END blocks
        // (triggers contain semicolons inside their body).
        let statements = split_sql_statements(migration.up_sql);
        for statement in &statements {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            conn.execute(trimmed, libsql::params![]).await?;
        }

        // Record the migration
        conn.execute(
            "INSERT INTO _schema_versions (version, description) VALUES (?, ?)",
            libsql::params![migration.version, migration.description],
        )
        .await?;

        info!("Migration v{} applied successfully", migration.version);
    }

    Ok(())
}

/// Split SQL text into individual statements, respecting BEGIN...END blocks.
/// Triggers contain semicolons inside their body that should not be split on.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32; // Track BEGIN/END nesting

    for line in sql.lines() {
        let trimmed_upper = line.trim().to_uppercase();

        if trimmed_upper.starts_with("BEGIN") {
            depth += 1;
        }

        current.push_str(line);
        current.push('\n');

        if trimmed_upper.starts_with("END") && depth > 0 {
            depth -= 1;
            if depth == 0 {
                // End of trigger block — look for trailing semicolon
                let trimmed = current.trim().trim_end_matches(';').to_string();
                if !trimmed.is_empty() {
                    statements.push(trimmed);
                }
                current.clear();
            }
        } else if depth == 0 && line.trim().ends_with(';') {
            let trimmed = current.trim().trim_end_matches(';').to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        }
    }

    // Handle any remaining content
    let trimmed = current.trim().trim_end_matches(';').to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}
```

- [ ] **Step 2: Write the MIGRATION_V1 constant**

This is the large SQL string containing all table definitions. It must include every table from the current schema.rs with `user_id TEXT NOT NULL` added to all user-data tables. Append this constant to the schema.rs file:

```rust
const MIGRATION_V1: &str = r#"
-- =============================================
-- Migration v1: Initial schema with user_id
-- =============================================

-- Stocks table
CREATE TABLE IF NOT EXISTS stocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    trade_type TEXT NOT NULL CHECK (trade_type IN ('BUY', 'SELL')),
    order_type TEXT NOT NULL CHECK (order_type IN ('MARKET', 'LIMIT', 'STOP', 'STOP_LIMIT')),
    entry_price DECIMAL(15,8) NOT NULL,
    exit_price DECIMAL(15,8) NOT NULL,
    stop_loss DECIMAL(15,8) NOT NULL,
    commissions DECIMAL(10,4) NOT NULL DEFAULT 0.00,
    number_shares DECIMAL(15,8) NOT NULL,
    take_profit DECIMAL(15,8),
    initial_target DECIMAL(15,8),
    profit_target DECIMAL(15,8),
    trade_ratings INTEGER CHECK (trade_ratings >= 1 AND trade_ratings <= 5),
    entry_date TIMESTAMP NOT NULL,
    exit_date TIMESTAMP NOT NULL,
    reviewed BOOLEAN NOT NULL DEFAULT false,
    mistakes TEXT,
    brokerage_name TEXT,
    trade_group_id TEXT,
    parent_trade_id INTEGER,
    total_quantity DECIMAL(15,8),
    transaction_sequence INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_deleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_stocks_user_id ON stocks(user_id);
CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol);
CREATE INDEX IF NOT EXISTS idx_stocks_trade_type ON stocks(trade_type);
CREATE INDEX IF NOT EXISTS idx_stocks_entry_date ON stocks(entry_date);
CREATE INDEX IF NOT EXISTS idx_stocks_exit_date ON stocks(exit_date);
CREATE INDEX IF NOT EXISTS idx_stocks_entry_price ON stocks(entry_price);
CREATE INDEX IF NOT EXISTS idx_stocks_exit_price ON stocks(exit_price);
CREATE INDEX IF NOT EXISTS idx_stocks_brokerage_name ON stocks(brokerage_name);
CREATE INDEX IF NOT EXISTS idx_stocks_trade_group_id ON stocks(trade_group_id);
CREATE INDEX IF NOT EXISTS idx_stocks_parent_trade_id ON stocks(parent_trade_id);
CREATE INDEX IF NOT EXISTS idx_stocks_is_deleted ON stocks(is_deleted);

-- User profile
CREATE TABLE IF NOT EXISTS user_profile (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL UNIQUE,
    display_name TEXT,
    nickname TEXT,
    timezone TEXT DEFAULT 'UTC',
    currency TEXT DEFAULT 'USD',
    profile_picture_uuid TEXT,
    trading_experience_level TEXT,
    primary_trading_goal TEXT,
    asset_types TEXT,
    trading_style TEXT,
    storage_used_bytes INTEGER DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_user_profile_user_id ON user_profile(user_id);

-- Options
CREATE TABLE IF NOT EXISTS options (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    option_type TEXT NOT NULL CHECK (option_type IN ('Call', 'Put')),
    strike_price DECIMAL(15,8) NOT NULL,
    expiration_date TIMESTAMP NOT NULL,
    entry_price DECIMAL(15,8) NOT NULL,
    exit_price DECIMAL(15,8) NOT NULL,
    premium DECIMAL(15,8) NOT NULL,
    commissions DECIMAL(10,4) NOT NULL DEFAULT 0.00,
    entry_date TIMESTAMP NOT NULL,
    exit_date TIMESTAMP NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
    initial_target DECIMAL(15,8),
    profit_target DECIMAL(15,8),
    trade_ratings INTEGER CHECK (trade_ratings >= 1 AND trade_ratings <= 5),
    reviewed BOOLEAN NOT NULL DEFAULT false,
    mistakes TEXT,
    brokerage_name TEXT,
    trade_group_id TEXT,
    parent_trade_id INTEGER,
    total_quantity DECIMAL(15,8),
    transaction_sequence INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_deleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_options_user_id ON options(user_id);
CREATE INDEX IF NOT EXISTS idx_options_symbol ON options(symbol);
CREATE INDEX IF NOT EXISTS idx_options_option_type ON options(option_type);
CREATE INDEX IF NOT EXISTS idx_options_status ON options(status);
CREATE INDEX IF NOT EXISTS idx_options_entry_date ON options(entry_date);
CREATE INDEX IF NOT EXISTS idx_options_exit_date ON options(exit_date);
CREATE INDEX IF NOT EXISTS idx_options_expiration_date ON options(expiration_date);
CREATE INDEX IF NOT EXISTS idx_options_strike_price ON options(strike_price);
CREATE INDEX IF NOT EXISTS idx_options_entry_price ON options(entry_price);
CREATE INDEX IF NOT EXISTS idx_options_exit_price ON options(exit_price);
CREATE INDEX IF NOT EXISTS idx_options_brokerage_name ON options(brokerage_name);
CREATE INDEX IF NOT EXISTS idx_options_trade_group_id ON options(trade_group_id);
CREATE INDEX IF NOT EXISTS idx_options_parent_trade_id ON options(parent_trade_id);
CREATE INDEX IF NOT EXISTS idx_options_is_deleted ON options(is_deleted);

-- Stock positions
CREATE TABLE IF NOT EXISTS stock_positions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    total_quantity DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    average_entry_price DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    current_price DECIMAL(15,8),
    unrealized_pnl DECIMAL(15,8),
    realized_pnl DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    brokerage_name TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_stock_positions_user_id ON stock_positions(user_id);
CREATE INDEX IF NOT EXISTS idx_stock_positions_symbol ON stock_positions(symbol);
CREATE INDEX IF NOT EXISTS idx_stock_positions_brokerage_name ON stock_positions(brokerage_name);

-- Option positions
CREATE TABLE IF NOT EXISTS option_positions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    option_type TEXT NOT NULL CHECK (option_type IN ('Call', 'Put')),
    strike_price DECIMAL(15,8) NOT NULL,
    expiration_date TIMESTAMP NOT NULL,
    total_quantity DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    average_entry_price DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    current_price DECIMAL(15,8),
    unrealized_pnl DECIMAL(15,8),
    realized_pnl DECIMAL(15,8) NOT NULL DEFAULT 0.0,
    brokerage_name TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_option_positions_user_id ON option_positions(user_id);
CREATE INDEX IF NOT EXISTS idx_option_positions_symbol ON option_positions(symbol);
CREATE INDEX IF NOT EXISTS idx_option_positions_strike_exp ON option_positions(symbol, strike_price, expiration_date, option_type);
CREATE INDEX IF NOT EXISTS idx_option_positions_brokerage_name ON option_positions(brokerage_name);

-- Position transactions (audit log)
CREATE TABLE IF NOT EXISTS position_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    position_id TEXT NOT NULL,
    position_type TEXT NOT NULL CHECK (position_type IN ('stock', 'option')),
    trade_id INTEGER,
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('entry', 'exit', 'adjustment')),
    quantity DECIMAL(15,8) NOT NULL,
    price DECIMAL(15,8) NOT NULL,
    transaction_date TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_position_transactions_user_id ON position_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_position_transactions_position_id ON position_transactions(position_id);
CREATE INDEX IF NOT EXISTS idx_position_transactions_position_type ON position_transactions(position_type);
CREATE INDEX IF NOT EXISTS idx_position_transactions_trade_id ON position_transactions(trade_id);
CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_type ON position_transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_date ON position_transactions(transaction_date);

-- Trade notes
CREATE TABLE IF NOT EXISTS trade_notes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT DEFAULT '',
    trade_type TEXT CHECK (trade_type IN ('stock', 'option')),
    stock_trade_id INTEGER,
    option_trade_id INTEGER,
    ai_metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (
        (trade_type = 'stock' AND stock_trade_id IS NOT NULL AND option_trade_id IS NULL) OR
        (trade_type = 'option' AND option_trade_id IS NOT NULL AND stock_trade_id IS NULL) OR
        (trade_type IS NULL AND stock_trade_id IS NULL AND option_trade_id IS NULL)
    ),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_trade_notes_user_id ON trade_notes(user_id);
CREATE INDEX IF NOT EXISTS idx_trade_notes_updated_at ON trade_notes(updated_at);
CREATE INDEX IF NOT EXISTS idx_trade_notes_stock_trade_id ON trade_notes(stock_trade_id);
CREATE INDEX IF NOT EXISTS idx_trade_notes_option_trade_id ON trade_notes(option_trade_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_stock_unique ON trade_notes(stock_trade_id) WHERE stock_trade_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_option_unique ON trade_notes(option_trade_id) WHERE option_trade_id IS NOT NULL;

-- Playbook
CREATE TABLE IF NOT EXISTS playbook (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    emoji TEXT,
    color TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_playbook_user_id ON playbook(user_id);

CREATE TABLE IF NOT EXISTS stock_trade_playbook (
    stock_trade_id INTEGER NOT NULL,
    setup_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (stock_trade_id, setup_id),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_stock_trade_playbook_user_id ON stock_trade_playbook(user_id);

CREATE TABLE IF NOT EXISTS option_trade_playbook (
    option_trade_id INTEGER NOT NULL,
    setup_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (option_trade_id, setup_id),
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
    FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_option_trade_playbook_user_id ON option_trade_playbook(user_id);

-- Playbook rules
CREATE TABLE IF NOT EXISTS playbook_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    playbook_id TEXT NOT NULL,
    rule_type TEXT NOT NULL CHECK (rule_type IN ('entry_criteria', 'exit_criteria', 'market_factor')),
    title TEXT NOT NULL,
    description TEXT,
    order_position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_playbook_rules_user_id ON playbook_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_playbook_rules_playbook_id ON playbook_rules(playbook_id);
CREATE INDEX IF NOT EXISTS idx_playbook_rules_type ON playbook_rules(rule_type);

-- Trade rule compliance
CREATE TABLE IF NOT EXISTS stock_trade_rule_compliance (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    stock_trade_id INTEGER NOT NULL,
    playbook_id TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    is_followed BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES playbook_rules(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_stock_trade_rule_compliance_user_id ON stock_trade_rule_compliance(user_id);

CREATE TABLE IF NOT EXISTS option_trade_rule_compliance (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    option_trade_id INTEGER NOT NULL,
    playbook_id TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    is_followed BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
    FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES playbook_rules(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_option_trade_rule_compliance_user_id ON option_trade_rule_compliance(user_id);

-- Missed trades
CREATE TABLE IF NOT EXISTS missed_trades (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    playbook_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    trade_type TEXT NOT NULL CHECK (trade_type IN ('stock', 'option')),
    reason TEXT NOT NULL,
    potential_entry_price REAL,
    opportunity_date TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_missed_trades_user_id ON missed_trades(user_id);
CREATE INDEX IF NOT EXISTS idx_missed_trades_playbook_id ON missed_trades(playbook_id);
CREATE INDEX IF NOT EXISTS idx_missed_trades_opportunity_date ON missed_trades(opportunity_date);

-- Replicache
CREATE TABLE IF NOT EXISTS replicache_clients (
    client_group_id TEXT NOT NULL,
    client_id TEXT NOT NULL,
    last_mutation_id INTEGER NOT NULL DEFAULT 0,
    last_modified_version INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (client_group_id, client_id)
);
CREATE INDEX IF NOT EXISTS idx_replicache_clients_user_id ON replicache_clients(user_id);

-- NOTE: replicache_space_version was a singleton table (id=1).
-- It becomes per-user now. The old INSERT OR IGNORE seed row is removed.
-- Any code that assumes a singleton row (INSERT OR IGNORE ... VALUES (1, 0))
-- must be updated to be per-user (deferred to model update phase).
CREATE TABLE IF NOT EXISTS replicache_space_version (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    user_id TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_replicache_space_version_user_id ON replicache_space_version(user_id);

-- Trade tags
CREATE TABLE IF NOT EXISTS trade_tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, category, name)
);
CREATE INDEX IF NOT EXISTS idx_trade_tags_user_id ON trade_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_trade_tags_category ON trade_tags(category);
CREATE INDEX IF NOT EXISTS idx_trade_tags_name ON trade_tags(name);

CREATE TABLE IF NOT EXISTS stock_trade_tags (
    stock_trade_id INTEGER NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (stock_trade_id, tag_id),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_user_id ON stock_trade_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_stock_id ON stock_trade_tags(stock_trade_id);
CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_tag_id ON stock_trade_tags(tag_id);

CREATE TABLE IF NOT EXISTS option_trade_tags (
    option_trade_id INTEGER NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (option_trade_id, tag_id),
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_option_trade_tags_user_id ON option_trade_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_option_trade_tags_option_id ON option_trade_tags(option_trade_id);
CREATE INDEX IF NOT EXISTS idx_option_trade_tags_tag_id ON option_trade_tags(tag_id);

-- Notebook: folders
CREATE TABLE IF NOT EXISTS notebook_folders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    is_favorite BOOLEAN NOT NULL DEFAULT false,
    parent_folder_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (parent_folder_id) REFERENCES notebook_folders(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_notebook_folders_user_id ON notebook_folders(user_id);
CREATE INDEX IF NOT EXISTS idx_notebook_folders_name ON notebook_folders(name);
CREATE INDEX IF NOT EXISTS idx_notebook_folders_is_favorite ON notebook_folders(is_favorite);
CREATE INDEX IF NOT EXISTS idx_notebook_folders_parent_folder_id ON notebook_folders(parent_folder_id);

-- Notebook: notes
CREATE TABLE IF NOT EXISTS notebook_notes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'Untitled',
    content TEXT NOT NULL DEFAULT '{}',
    content_plain_text TEXT,
    word_count INTEGER NOT NULL DEFAULT 0,
    is_pinned BOOLEAN NOT NULL DEFAULT false,
    is_archived BOOLEAN NOT NULL DEFAULT false,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    folder_id TEXT,
    tags TEXT,
    y_state BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_synced_at TEXT,
    FOREIGN KEY (folder_id) REFERENCES notebook_folders(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_user_id ON notebook_notes(user_id);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_title ON notebook_notes(title);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_pinned ON notebook_notes(is_pinned);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_archived ON notebook_notes(is_archived);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_deleted ON notebook_notes(is_deleted);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_folder_id ON notebook_notes(folder_id);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_created_at ON notebook_notes(created_at);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_updated_at ON notebook_notes(updated_at);

-- Notebook: tags
CREATE TABLE IF NOT EXISTS notebook_tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    is_favorite BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, name)
);
CREATE INDEX IF NOT EXISTS idx_notebook_tags_user_id ON notebook_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_notebook_tags_name ON notebook_tags(name);
CREATE INDEX IF NOT EXISTS idx_notebook_tags_is_favorite ON notebook_tags(is_favorite);

-- Notebook: note-tag junction
CREATE TABLE IF NOT EXISTS notebook_note_tags (
    note_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES notebook_tags(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_user_id ON notebook_note_tags(user_id);
CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_note_id ON notebook_note_tags(note_id);
CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_tag_id ON notebook_note_tags(tag_id);

-- Notebook: images
CREATE TABLE IF NOT EXISTS notebook_images (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    src TEXT NOT NULL,
    storage_path TEXT,
    alt_text TEXT,
    caption TEXT,
    width INTEGER,
    height INTEGER,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_notebook_images_user_id ON notebook_images(user_id);
CREATE INDEX IF NOT EXISTS idx_notebook_images_note_id ON notebook_images(note_id);
CREATE INDEX IF NOT EXISTS idx_notebook_images_created_at ON notebook_images(created_at);

-- Note collaborators
CREATE TABLE IF NOT EXISTS note_collaborators (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    user_email TEXT NOT NULL,
    user_name TEXT,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    invited_by TEXT NOT NULL,
    joined_at TEXT NOT NULL,
    last_seen_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_id ON note_collaborators(user_id);
CREATE INDEX IF NOT EXISTS idx_note_collaborators_note_id ON note_collaborators(note_id);
CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_email ON note_collaborators(user_email);
CREATE UNIQUE INDEX IF NOT EXISTS idx_note_collaborators_note_user ON note_collaborators(note_id, user_id);

-- Note invitations
CREATE TABLE IF NOT EXISTS note_invitations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    inviter_id TEXT NOT NULL,
    inviter_email TEXT NOT NULL,
    invitee_email TEXT NOT NULL,
    invitee_user_id TEXT,
    role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    token TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'expired', 'revoked')),
    message TEXT,
    expires_at TEXT NOT NULL,
    accepted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_note_invitations_user_id ON note_invitations(user_id);
CREATE INDEX IF NOT EXISTS idx_note_invitations_note_id ON note_invitations(note_id);
CREATE INDEX IF NOT EXISTS idx_note_invitations_invitee_email ON note_invitations(invitee_email);
CREATE UNIQUE INDEX IF NOT EXISTS idx_note_invitations_token ON note_invitations(token);
CREATE INDEX IF NOT EXISTS idx_note_invitations_status ON note_invitations(status);

-- Note share settings
CREATE TABLE IF NOT EXISTS note_share_settings (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    note_id TEXT NOT NULL UNIQUE,
    owner_id TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'shared', 'public')),
    public_slug TEXT UNIQUE,
    allow_comments BOOLEAN NOT NULL DEFAULT false,
    allow_copy BOOLEAN NOT NULL DEFAULT true,
    password_hash TEXT,
    view_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_note_id ON note_share_settings(note_id);
CREATE INDEX IF NOT EXISTS idx_note_share_settings_user_id ON note_share_settings(user_id);
CREATE INDEX IF NOT EXISTS idx_note_share_settings_owner_id ON note_share_settings(owner_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_public_slug ON note_share_settings(public_slug);
CREATE INDEX IF NOT EXISTS idx_note_share_settings_visibility ON note_share_settings(visibility);

-- Alert rules
CREATE TABLE IF NOT EXISTS alert_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    symbol TEXT,
    operator TEXT,
    threshold REAL,
    channels TEXT NOT NULL,
    cooldown_seconds INTEGER,
    last_fired_at TEXT,
    note TEXT,
    max_triggers INTEGER,
    fired_count INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_symbol ON alert_rules(symbol);
CREATE INDEX IF NOT EXISTS idx_alert_rules_created_at ON alert_rules(created_at);
CREATE INDEX IF NOT EXISTS idx_alert_rules_is_active ON alert_rules(is_active);

-- Web Push subscriptions
CREATE TABLE IF NOT EXISTS push_subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    ua TEXT,
    topics TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user_id ON push_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_push_subscriptions_created_at ON push_subscriptions(created_at);

-- AI Chat
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    title TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    last_message_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_user_id ON chat_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_at ON chat_sessions(updated_at);

CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    context_vectors TEXT,
    token_count INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_chat_messages_user_id ON chat_messages(user_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at);

-- AI Reports
CREATE TABLE IF NOT EXISTS ai_reports (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    time_range TEXT NOT NULL CHECK (time_range IN ('7d', '30d', '90d', 'ytd', '1y')),
    title TEXT NOT NULL,
    analytics TEXT NOT NULL,
    trades TEXT,
    recommendations TEXT,
    metrics TEXT,
    metadata TEXT NOT NULL,
    summary TEXT DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_ai_reports_user_id ON ai_reports(user_id);
CREATE INDEX IF NOT EXISTS idx_ai_reports_time_range ON ai_reports(time_range);
CREATE INDEX IF NOT EXISTS idx_ai_reports_created_at ON ai_reports(created_at);

-- AI Analysis
CREATE TABLE IF NOT EXISTS ai_analysis (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    time_range TEXT NOT NULL CHECK (time_range IN ('7d', '30d', '90d', 'ytd', '1y')),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_ai_analysis_user_id ON ai_analysis(user_id);
CREATE INDEX IF NOT EXISTS idx_ai_analysis_time_range ON ai_analysis(time_range);
CREATE INDEX IF NOT EXISTS idx_ai_analysis_created_at ON ai_analysis(created_at);

-- =============================================
-- BROKERAGE & CALENDAR TABLES
-- IMPORTANT: The schemas below are PLACEHOLDERS.
-- The implementer MUST copy the EXACT table definitions
-- from the current schema.rs (lines ~1309-1594) and only
-- add `user_id TEXT NOT NULL` + the user_id index.
-- The actual tables have snaptrade-specific columns,
-- CHECK constraints, and UNIQUE constraints that
-- differ significantly from generic schemas.
-- =============================================

-- Copy brokerage_connections from schema.rs ~line 1312, add user_id TEXT NOT NULL
-- Copy brokerage_accounts from schema.rs ~line 1350, add user_id TEXT NOT NULL
-- Copy brokerage_transactions from schema.rs ~line 1390, add user_id TEXT NOT NULL
-- Copy brokerage_holdings from schema.rs ~line 1415, add user_id TEXT NOT NULL
-- Copy unmatched_transactions from schema.rs ~line 1434, add user_id TEXT NOT NULL
-- Copy calendar_connections from schema.rs ~line 1470, add user_id TEXT NOT NULL
-- Copy calendar_sync_cursors from schema.rs ~line 1510, add user_id TEXT NOT NULL
-- Copy calendar_events from schema.rs ~line 1540, add user_id TEXT NOT NULL
-- For each table, also add: CREATE INDEX IF NOT EXISTS idx_{table}_user_id ON {table}(user_id);

-- =============================================
-- Global tables (no user_id)
-- =============================================

-- System notifications
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    data TEXT,
    status TEXT NOT NULL DEFAULT 'template',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type);
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);

-- Notification dispatch log
CREATE TABLE IF NOT EXISTS notification_dispatch_log (
    id TEXT PRIMARY KEY,
    notification_id TEXT,
    notification_type TEXT NOT NULL,
    slot_key TEXT NOT NULL UNIQUE,
    sent_count INTEGER DEFAULT 0,
    failed_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notification_dispatch_type ON notification_dispatch_log(notification_type);

-- Public notes registry
CREATE TABLE IF NOT EXISTS public_notes_registry (
    slug TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'Untitled',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_public_notes_owner ON public_notes_registry(owner_id);

-- Invitations registry
CREATE TABLE IF NOT EXISTS invitations_registry (
    token TEXT PRIMARY KEY,
    inviter_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    invitee_email TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_invitations_inviter ON invitations_registry(inviter_id);
CREATE INDEX IF NOT EXISTS idx_invitations_invitee ON invitations_registry(invitee_email);

-- Collaborators registry
CREATE TABLE IF NOT EXISTS collaborators_registry (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    collaborator_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'editor',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(note_id, collaborator_id)
);
CREATE INDEX IF NOT EXISTS idx_collaborators_note ON collaborators_registry(note_id);
CREATE INDEX IF NOT EXISTS idx_collaborators_owner ON collaborators_registry(owner_id);
CREATE INDEX IF NOT EXISTS idx_collaborators_user ON collaborators_registry(collaborator_id);

-- =============================================
-- Triggers
-- IMPORTANT: The implementer MUST copy ALL triggers
-- from the current schema.rs (lines ~1170-1300+).
-- The current schema has ~15 triggers covering:
-- stocks, options, user_profile, trade_notes, trade_tags,
-- playbook, notebook_images, notebook_notes, notebook_folders,
-- notebook_tags, calendar_connections, calendar_sync_cursors,
-- calendar_events, note_collaborators, note_invitations,
-- note_share_settings, notifications.
-- Copy them verbatim — they don't need user_id changes.
-- =============================================

CREATE TRIGGER IF NOT EXISTS update_stocks_timestamp
AFTER UPDATE ON stocks
FOR EACH ROW
BEGIN
    UPDATE stocks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_options_timestamp
AFTER UPDATE ON options
FOR EACH ROW
BEGIN
    UPDATE options SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_user_profile_timestamp
AFTER UPDATE ON user_profile
FOR EACH ROW
BEGIN
    UPDATE user_profile SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notifications_timestamp
AFTER UPDATE ON notifications
FOR EACH ROW
BEGIN
    UPDATE notifications SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- ... (copy remaining ~11 triggers from schema.rs verbatim)
"#;
```

**Important:** The implementer must cross-reference the current `schema.rs` (lines 1-6158) to ensure ALL tables are present, including any tables added after the brokerage/calendar tables. Check lines 1200-6158 of the current file for any remaining tables or triggers not listed above.

- [ ] **Step 3: Verify compilation of schema.rs alone**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | head -30`
Expected: Errors in client.rs about removed schema imports — this is expected and fixed in Task 3.

- [ ] **Step 4: Commit**

```bash
git add backend/src/turso/schema.rs
git commit -m "refactor(turso): replace schema with versioned migration system"
```

---

### Task 3: Rewrite client.rs

**Files:**
- Rewrite: `backend/src/turso/client.rs` (current: 1220 lines)

- [ ] **Step 1: Write the new client.rs**

The new file is dramatically simpler. Key changes:
- Remove `http_client`, all Turso API structs, all per-user DB methods
- `new()` connects to single DB and runs `migrate()`
- Add `get_connection()`, `get_user_db()`
- Keep registry query methods (public notes, invitations, collaborators) — they now query the same DB
- `list_all_user_ids()` queries `user_profile` instead of `user_databases`
- `health_check()` simplified to just check one DB

```rust
use anyhow::{Context, Result};
use libsql::{Builder, Connection, Database};
use log::info;
use serde::{Deserialize, Serialize};

use super::config::TursoConfig;
use super::schema::migrate;

/// Turso client for the single shared database
pub struct TursoClient {
    db: Database,
    #[allow(dead_code)]
    config: TursoConfig,
}

/// Public note registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicNoteEntry {
    pub slug: String,
    pub note_id: String,
    pub owner_id: String,
    pub title: String,
}

/// Invitation registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvitationEntry {
    pub token: String,
    pub inviter_id: String,
    pub note_id: String,
    pub invitee_email: String,
}

/// Collaborator registry entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CollaboratorEntry {
    pub id: String,
    pub note_id: String,
    pub owner_id: String,
    pub collaborator_id: String,
    pub role: String,
}

/// Scoped database access for a specific user
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

impl TursoClient {
    /// Create a new Turso client and run migrations
    pub async fn new(config: TursoConfig) -> Result<Self> {
        let db = Builder::new_remote(
            config.db_url.clone(),
            config.db_token.clone(),
        )
        .build()
        .await
        .context("Failed to connect to database")?;

        // Run schema migrations on startup
        let conn = db
            .connect()
            .context("Failed to get database connection for migration")?;

        migrate(&conn).await.context("Schema migration failed")?;

        info!("Database connection established and schema migrated");

        Ok(Self { db, config })
    }

    /// Get a connection to the shared database
    pub fn get_connection(&self) -> Result<Connection> {
        self.db
            .connect()
            .context("Failed to get database connection")
    }

    /// Get a scoped UserDb for a specific user
    pub fn get_user_db(&self, user_id: &str) -> Result<UserDb> {
        let conn = self.get_connection()?;
        Ok(UserDb::new(conn, user_id.to_string()))
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("SELECT 1", libsql::params![]).await?;
        Ok(())
    }

    /// List all user IDs
    pub async fn list_all_user_ids(&self) -> Result<Vec<String>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query("SELECT DISTINCT user_id FROM user_profile", libsql::params![])
            .await?;

        let mut ids = Vec::new();
        while let Some(row) = rows.next().await? {
            ids.push(row.get::<String>(0)?);
        }
        Ok(ids)
    }

    // --- Public notes registry ---

    pub async fn register_public_note(
        &self,
        slug: &str,
        note_id: &str,
        owner_id: &str,
        title: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO public_notes_registry (slug, note_id, owner_id, title) VALUES (?, ?, ?, ?)",
            libsql::params![slug, note_id, owner_id, title],
        ).await?;
        Ok(())
    }

    pub async fn unregister_public_note(&self, slug: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM public_notes_registry WHERE slug = ?",
            libsql::params![slug],
        ).await?;
        Ok(())
    }

    pub async fn get_public_note_entry(&self, slug: &str) -> Result<Option<PublicNoteEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT slug, note_id, owner_id, title FROM public_notes_registry WHERE slug = ?",
                libsql::params![slug],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(PublicNoteEntry {
                slug: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                title: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    // --- Invitations registry ---

    pub async fn register_invitation(
        &self,
        token: &str,
        inviter_id: &str,
        note_id: &str,
        invitee_email: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO invitations_registry (token, inviter_id, note_id, invitee_email) VALUES (?, ?, ?, ?)",
            libsql::params![token, inviter_id, note_id, invitee_email],
        ).await?;
        Ok(())
    }

    pub async fn unregister_invitation(&self, token: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM invitations_registry WHERE token = ?",
            libsql::params![token],
        ).await?;
        Ok(())
    }

    pub async fn get_invitation_entry(&self, token: &str) -> Result<Option<InvitationEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT token, inviter_id, note_id, invitee_email FROM invitations_registry WHERE token = ?",
                libsql::params![token],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(InvitationEntry {
                token: row.get(0)?,
                inviter_id: row.get(1)?,
                note_id: row.get(2)?,
                invitee_email: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    // --- Collaborators registry ---

    pub async fn register_collaborator(
        &self,
        id: &str,
        note_id: &str,
        owner_id: &str,
        collaborator_id: &str,
        role: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO collaborators_registry (id, note_id, owner_id, collaborator_id, role) VALUES (?, ?, ?, ?, ?)",
            libsql::params![id, note_id, owner_id, collaborator_id, role],
        ).await?;
        Ok(())
    }

    pub async fn unregister_collaborator(
        &self,
        note_id: &str,
        collaborator_id: &str,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?",
            libsql::params![note_id, collaborator_id],
        ).await?;
        Ok(())
    }

    pub async fn get_collaborator_entry(
        &self,
        note_id: &str,
        user_id: &str,
    ) -> Result<Option<CollaboratorEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE note_id = ? AND collaborator_id = ?",
                libsql::params![note_id, user_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(CollaboratorEntry {
                id: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                collaborator_id: row.get(3)?,
                role: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_shared_notes_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<CollaboratorEntry>> {
        let conn = self.get_connection()?;
        let mut rows = conn
            .query(
                "SELECT id, note_id, owner_id, collaborator_id, role FROM collaborators_registry WHERE collaborator_id = ?",
                libsql::params![user_id],
            )
            .await?;

        let mut entries = Vec::new();
        while let Some(row) = rows.next().await? {
            entries.push(CollaboratorEntry {
                id: row.get(0)?,
                note_id: row.get(1)?,
                owner_id: row.get(2)?,
                collaborator_id: row.get(3)?,
                role: row.get(4)?,
            });
        }
        Ok(entries)
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | head -50`
Expected: Errors in mod.rs and external files about removed types/methods — fixed in following tasks.

- [ ] **Step 3: Commit**

```bash
git add backend/src/turso/client.rs
git commit -m "refactor(turso): rewrite client for single shared database"
```

---

### Task 4: Delete sync.rs and Update mod.rs

**Files:**
- Delete: `backend/src/turso/sync.rs`
- Modify: `backend/src/turso/mod.rs`

- [ ] **Step 1: Delete sync.rs**

```bash
rm backend/src/turso/sync.rs
```

- [ ] **Step 2: Update mod.rs**

Key changes:
- Remove `pub mod sync;`
- Remove `sync` import from schema (the old schema functions are gone)
- Update `AppState` to replace `get_user_db_connection()` with `get_user_db()`
- Update re-exports: remove `SchemaVersion` related items, add `UserDb`

Replace `mod.rs` with updated version. Keep all service initializations unchanged. The main changes are:

1. Remove `pub mod sync;` (line 7)
2. Update re-exports (line 16-18) — add `UserDb`, remove old schema re-exports
3. Replace `get_user_db_connection` method (lines 240-248) with:

```rust
/// Get scoped UserDb for a specific user
pub fn get_user_db(
    &self,
    user_id: &str,
) -> Result<client::UserDb, Box<dyn std::error::Error>> {
    Ok(self.turso_client.get_user_db(user_id)?)
}
```

4. Keep everything else in mod.rs unchanged (all service initializations, health_check, public note methods, etc.)

- [ ] **Step 3: Verify turso module compiles**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | head -50`
Expected: Errors only in files OUTSIDE the turso module (main.rs, routes, services). The turso module itself should compile.

- [ ] **Step 4: Commit**

```bash
git add backend/src/turso/sync.rs backend/src/turso/mod.rs
git commit -m "refactor(turso): remove sync module, update mod.rs with UserDb"
```

---

## Chunk 2: External Dependency Fixes

### Task 5: Fix main.rs compile-breaking changes

**Files:**
- Modify: `backend/src/main.rs`

- [ ] **Step 1: Remove ENABLE_STARTUP_SCHEMA_SYNC block**

Find and remove the code block around line 126 that checks `ENABLE_STARTUP_SCHEMA_SYNC` and calls `sync_all_user_schemas_background`. Also remove the `use crate::turso::sync::*` import if present.

- [ ] **Step 2: Update supabase_webhook_handler / handle_user_created**

Around line 613, `handle_user_created` calls `turso_client.create_user_database(user_id, email)`. Replace with inserting a `user_profile` row:

```rust
// Create user profile in shared database (no per-user DB needed)
let conn = turso_client.get_connection()
    .map_err(|e| format!("Failed to get DB connection: {}", e))?;
conn.execute(
    "INSERT OR IGNORE INTO user_profile (user_id, display_name, created_at, updated_at) VALUES (?, ?, datetime('now'), datetime('now'))",
    libsql::params![user_id, email],
).await
.map_err(|e| format!("Failed to create user profile: {}", e))?;
```

- [ ] **Step 3: Update get_current_user handler**

Around line 487, this handler calls `get_user_database()` and reads `UserDatabaseEntry` fields. Replace with querying `user_profile`:

```rust
let conn = app_state.turso_client.get_connection()
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("DB error: {}", e)))?;
let mut rows = conn.query(
    "SELECT user_id, display_name, timezone, currency FROM user_profile WHERE user_id = ?",
    libsql::params![user_id],
).await
.map_err(|e| actix_web::error::ErrorInternalServerError(format!("Query error: {}", e)))?;
```

Adjust the response JSON to use user_profile fields instead of UserDatabaseEntry fields.

- [ ] **Step 4: Fix any remaining get_user_data handler**

If there's a `get_user_data` placeholder handler, update it to use `get_user_db()` instead of `get_user_db_connection()`.

- [ ] **Step 5: Verify compilation progress**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | grep "error\[" | wc -l`
Expected: Fewer errors than before. Remaining errors should be in routes/services.

- [ ] **Step 6: Commit**

```bash
git add backend/src/main.rs
git commit -m "refactor(main): update for single shared database"
```

---

### Task 6: Fix service compile-breaking changes

**Files:**
- Modify: `backend/src/service/utils/account_deletion.rs`
- Modify: `backend/src/service/utils/storage_quota.rs`
- Modify: `backend/src/service/brokerage/sync.rs`
- Modify: `backend/src/service/notifications/system.rs`
- Modify: `backend/src/service/notifications/earnings_calendar.rs`

- [ ] **Step 1: Fix account_deletion.rs**

Around lines 58 and 88, replace `delete_user_database()` and `remove_user_database_entry()` with deleting user data from all tables:

```rust
// Delete all user data from shared database
let conn = self.turso_client.get_connection()?;
let tables = [
    "stocks", "options", "stock_positions", "option_positions",
    "position_transactions", "trade_notes", "playbook_rules",
    "stock_trade_rule_compliance", "option_trade_rule_compliance",
    "stock_trade_playbook", "option_trade_playbook",
    "missed_trades", "trade_tags", "stock_trade_tags", "option_trade_tags",
    "notebook_note_tags", "notebook_images", "note_collaborators",
    "note_invitations", "note_share_settings", "notebook_notes",
    "notebook_folders", "notebook_tags",
    "replicache_clients", "replicache_space_version",
    "alert_rules", "push_subscriptions",
    "chat_messages", "chat_sessions", "ai_reports", "ai_analysis",
    "brokerage_holdings", "brokerage_transactions", "brokerage_accounts",
    "brokerage_connections", "unmatched_transactions",
    "calendar_events", "calendar_sync_cursors", "calendar_connections",
    "playbook", "user_profile",
];
for table in &tables {
    conn.execute(
        &format!("DELETE FROM {} WHERE user_id = ?", table),
        libsql::params![user_id],
    ).await.ok(); // Ignore errors for tables that may not have data
}
```

- [ ] **Step 2: Fix storage_quota.rs**

Around lines 127-153, replace `get_registry_connection()` with `get_connection()` and update queries from `user_databases` to `user_profile`:
- `SELECT storage_used_bytes FROM user_databases WHERE user_id = ?` -> `SELECT storage_used_bytes FROM user_profile WHERE user_id = ?`
- `UPDATE user_databases SET storage_used_bytes = ? WHERE user_id = ?` -> `UPDATE user_profile SET storage_used_bytes = ? WHERE user_id = ?`

- [ ] **Step 3: Fix brokerage/sync.rs**

Around line 67, replace `get_user_database_connection(&user_id)` with `get_user_db(&user_id)`. Update the variable from `conn` to use `user_db.conn()`.

- [ ] **Step 4: Fix notifications/system.rs**

Around lines 47 and 76:
- Replace `get_registry_connection()` with `get_connection()`
- Replace `get_user_database_connection(&user_id)` with `get_user_db(&user_id)`, use `user_db.conn()`

- [ ] **Step 5: Fix notifications/earnings_calendar.rs**

Around lines 83 and 180:
- Replace `get_registry_connection()` with `get_connection()`
- Replace `get_user_database_connection(&user_id)` with `get_user_db(&user_id)`, use `user_db.conn()`

- [ ] **Step 6: Verify compilation progress**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1 | grep "error\[" | wc -l`
Expected: Remaining errors should only be in route files (blanket `get_user_database_connection` -> `get_user_db` rename).

- [ ] **Step 7: Commit**

```bash
git add backend/src/service/
git commit -m "refactor(services): update for single shared database"
```

---

### Task 7: Blanket rename across all route files and remaining services

**Files:**
- All files in `backend/src/routes/` that call `get_user_database_connection()` or `get_user_db_connection()`
- Any remaining service files (ai_service, market_engine)

This is a mechanical search-and-replace across ~20 files.

- [ ] **Step 1: Find all remaining call sites**

Run: `cd /Users/user/Tradstry/backend && grep -rn "get_user_database_connection\|get_user_db_connection" src/ --include="*.rs" | grep -v "turso/"`

This shows every file and line that needs updating.

- [ ] **Step 2: For each file, apply these mechanical changes**

Pattern A (via TursoClient directly):
```
// Before:
let conn = app_state.turso_client.get_user_database_connection(&user_id).await?
    .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

// After:
let user_db = app_state.turso_client.get_user_db(&user_id)?;
let conn = user_db.conn();
```

Pattern B (via AppState wrapper):
```
// Before:
let conn = app_state.get_user_db_connection(&user_id).await?
    .ok_or_else(|| actix_web::error::ErrorNotFound("User database not found"))?;

// After:
let user_db = app_state.get_user_db(&user_id)?;
let conn = user_db.conn();
```

Note: `get_user_db()` is NOT async (no `.await`), and returns `Result<UserDb>` (no `Option`, so drop `.ok_or_else()`).

**Ownership note:** `user_db.conn()` returns `&Connection` (borrowed), not an owned `Connection`. This works for most call sites since `user_db` lives for the function scope and model methods take `&Connection`. If any call site passes `conn` by value (owned), the implementer will get a compile error and should adjust that call site to accept `&Connection` instead.

In both patterns, existing code that uses `&conn` or passes `conn` to functions taking `&Connection` continues to work.

- [ ] **Step 3: Also replace any `get_registry_connection()` calls**

Run: `grep -rn "get_registry_connection" src/ --include="*.rs" | grep -v "turso/"`

Replace each with `get_connection()`.

- [ ] **Step 4: Verify full compilation**

Run: `cd /Users/user/Tradstry/backend && cargo check 2>&1`
Expected: Clean compilation (0 errors). Warnings are acceptable.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/ backend/src/service/
git commit -m "refactor(routes,services): blanket rename to get_user_db()"
```

---

### Task 8: Final verification

- [ ] **Step 1: Full cargo check**

Run: `cd /Users/user/Tradstry/backend && cargo check`
Expected: Success with 0 errors.

- [ ] **Step 2: Verify no references to removed items**

```bash
cd /Users/user/Tradstry/backend
grep -rn "UserDatabaseEntry\|create_user_database\|get_user_database_connection\|registry_db_url\|registry_db_token\|turso_api_token\|TURSO_ORG\|sync_all_user_schemas" src/ --include="*.rs"
```

Expected: No matches.

- [ ] **Step 3: Verify sync.rs is deleted**

```bash
test ! -f backend/src/turso/sync.rs && echo "PASS: sync.rs deleted"
```

- [ ] **Step 4: Commit any final cleanup**

```bash
git add -A && git status
# Only commit if there are changes
```
