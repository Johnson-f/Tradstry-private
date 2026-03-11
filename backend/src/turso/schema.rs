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
    vec![Migration {
        version: 1,
        description: "Initial schema with user_id columns",
        up_sql: MIGRATION_V1,
    }]
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
        .query(
            "SELECT COALESCE(MAX(version), 0) FROM _schema_versions",
            libsql::params![],
        )
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
-- BROKERAGE TABLES
-- =============================================

CREATE TABLE IF NOT EXISTS brokerage_connections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    snaptrade_user_id TEXT NOT NULL,
    snaptrade_user_secret TEXT NOT NULL,
    connection_id TEXT,
    brokerage_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'connected', 'error', 'disconnected')),
    last_sync_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_brokerage_connections_user_id ON brokerage_connections(user_id);

CREATE TABLE IF NOT EXISTS brokerage_accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    snaptrade_account_id TEXT NOT NULL,
    account_number TEXT,
    account_name TEXT,
    account_type TEXT,
    balance REAL,
    currency TEXT DEFAULT 'USD',
    institution_name TEXT,
    raw_data TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (connection_id) REFERENCES brokerage_connections(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_brokerage_accounts_user_id ON brokerage_accounts(user_id);

CREATE TABLE IF NOT EXISTS brokerage_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    snaptrade_transaction_id TEXT NOT NULL,
    symbol TEXT,
    transaction_type TEXT,
    quantity REAL,
    price REAL,
    amount REAL,
    currency TEXT DEFAULT 'USD',
    trade_date TIMESTAMP NOT NULL,
    settlement_date TIMESTAMP,
    fees REAL,
    raw_data TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES brokerage_accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, snaptrade_transaction_id)
);
CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_user_id ON brokerage_transactions(user_id);

CREATE TABLE IF NOT EXISTS brokerage_holdings (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    quantity REAL NOT NULL,
    average_cost REAL,
    current_price REAL,
    market_value REAL,
    currency TEXT DEFAULT 'USD',
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    raw_data TEXT,
    FOREIGN KEY (account_id) REFERENCES brokerage_accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, symbol)
);
CREATE INDEX IF NOT EXISTS idx_brokerage_holdings_user_id ON brokerage_holdings(user_id);

CREATE TABLE IF NOT EXISTS unmatched_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    transaction_id TEXT NOT NULL,
    snaptrade_transaction_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    trade_type TEXT NOT NULL CHECK (trade_type IN ('BUY', 'SELL')),
    units REAL NOT NULL,
    price REAL NOT NULL,
    fee REAL NOT NULL,
    trade_date TIMESTAMP NOT NULL,
    brokerage_name TEXT,
    raw_data TEXT,
    is_option BOOLEAN NOT NULL DEFAULT false,
    difficulty_reason TEXT,
    confidence_score REAL,
    suggested_matches TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'resolved', 'ignored')),
    resolved_trade_id INTEGER,
    resolved_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_user_id ON unmatched_transactions(user_id);

-- =============================================
-- CALENDAR TABLES
-- =============================================

CREATE TABLE IF NOT EXISTS calendar_connections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL CHECK (provider IN ('google', 'microsoft')),
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    token_expires_at TIMESTAMP NOT NULL,
    account_email TEXT NOT NULL,
    account_name TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    last_sync_at TIMESTAMP,
    sync_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_calendar_connections_user_id ON calendar_connections(user_id);

CREATE TABLE IF NOT EXISTS calendar_sync_cursors (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    external_calendar_id TEXT NOT NULL,
    external_calendar_name TEXT,
    sync_token TEXT,
    last_sync_at TIMESTAMP,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (connection_id) REFERENCES calendar_connections(id) ON DELETE CASCADE,
    UNIQUE(connection_id, external_calendar_id)
);
CREATE INDEX IF NOT EXISTS idx_calendar_sync_cursors_user_id ON calendar_sync_cursors(user_id);

CREATE TABLE IF NOT EXISTS calendar_events (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    all_day BOOLEAN NOT NULL DEFAULT false,
    reminder_minutes INTEGER,
    recurrence_rule TEXT,
    source TEXT NOT NULL DEFAULT 'local' CHECK (source IN ('local', 'google', 'microsoft')),
    external_id TEXT,
    external_calendar_id TEXT,
    connection_id TEXT,
    color TEXT,
    location TEXT,
    status TEXT NOT NULL DEFAULT 'confirmed' CHECK (status IN ('confirmed', 'tentative', 'cancelled')),
    is_reminder BOOLEAN NOT NULL DEFAULT false,
    reminder_sent BOOLEAN NOT NULL DEFAULT false,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced_at TIMESTAMP,
    FOREIGN KEY (connection_id) REFERENCES calendar_connections(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_calendar_events_user_id ON calendar_events(user_id);

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

CREATE TRIGGER IF NOT EXISTS update_trade_notes_timestamp
AFTER UPDATE ON trade_notes
FOR EACH ROW
BEGIN
    UPDATE trade_notes SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_trade_tags_timestamp
AFTER UPDATE ON trade_tags
FOR EACH ROW
BEGIN
    UPDATE trade_tags SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_playbook_timestamp
AFTER UPDATE ON playbook
FOR EACH ROW
BEGIN
    UPDATE playbook SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notebook_images_timestamp
AFTER UPDATE ON notebook_images
FOR EACH ROW
BEGIN
    UPDATE notebook_images SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notebook_notes_timestamp
AFTER UPDATE ON notebook_notes
FOR EACH ROW
BEGIN
    UPDATE notebook_notes SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notebook_folders_timestamp
AFTER UPDATE ON notebook_folders
FOR EACH ROW
BEGIN
    UPDATE notebook_folders SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notebook_tags_timestamp
AFTER UPDATE ON notebook_tags
FOR EACH ROW
BEGIN
    UPDATE notebook_tags SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_calendar_connections_timestamp
AFTER UPDATE ON calendar_connections
FOR EACH ROW
BEGIN
    UPDATE calendar_connections SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_calendar_sync_cursors_timestamp
AFTER UPDATE ON calendar_sync_cursors
FOR EACH ROW
BEGIN
    UPDATE calendar_sync_cursors SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_calendar_events_timestamp
AFTER UPDATE ON calendar_events
FOR EACH ROW
BEGIN
    UPDATE calendar_events SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_note_collaborators_timestamp
AFTER UPDATE ON note_collaborators
FOR EACH ROW
BEGIN
    UPDATE note_collaborators SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_note_invitations_timestamp
AFTER UPDATE ON note_invitations
FOR EACH ROW
BEGIN
    UPDATE note_invitations SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_note_share_settings_timestamp
AFTER UPDATE ON note_share_settings
FOR EACH ROW
BEGIN
    UPDATE note_share_settings SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_notifications_timestamp
AFTER UPDATE ON notifications
FOR EACH ROW
BEGIN
    UPDATE notifications SET updated_at = datetime('now') WHERE id = NEW.id;
END;
"#;
