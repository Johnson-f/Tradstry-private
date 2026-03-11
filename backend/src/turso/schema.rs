use anyhow::Result;
use libsql::Connection;
use log::info;

/// Schema version information
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SchemaVersion {
    pub version: String,
    pub description: String,
    pub created_at: String,
}

/// Table schema information for comparison
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub triggers: Vec<TriggerInfo>,
}

/// Column information for schema comparison
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

/// Index information for schema comparison
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

/// Trigger information for schema comparison
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TriggerInfo {
    pub name: String,
    pub table_name: String,
    pub event: String,
    pub timing: String,
    pub action: String,
}

/// Initialize user database with base trading + notebook schema
pub async fn initialize_user_database_schema(db_url: &str, token: &str) -> Result<()> {
    info!(
        "Initializing trading+notebook schema for database: {}",
        db_url
    );

    let user_db = libsql::Builder::new_remote(db_url.to_string(), token.to_string())
        .build()
        .await?;
    let conn = user_db.connect()?;

    // Stocks table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS stocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        )
        "#,
        libsql::params![],
    ).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_trade_type ON stocks(trade_type)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_entry_date ON stocks(entry_date)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_exit_date ON stocks(exit_date)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_entry_price ON stocks(entry_price)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_exit_price ON stocks(exit_price)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_brokerage_name ON stocks(brokerage_name)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_trade_group_id ON stocks(trade_group_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_parent_trade_id ON stocks(parent_trade_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stocks_is_deleted ON stocks(is_deleted)",
        libsql::params![],
    )
    .await?;

    // User profile
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS user_profile (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            display_name TEXT,
            nickname TEXT,
            timezone TEXT DEFAULT 'UTC',
            currency TEXT DEFAULT 'USD',
            profile_picture_uuid TEXT,
            trading_experience_level TEXT,
            primary_trading_goal TEXT,
            asset_types TEXT,
            trading_style TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        libsql::params![],
    )
    .await?;

    // Options
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS options (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_symbol ON options(symbol)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_option_type ON options(option_type)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_status ON options(status)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_entry_date ON options(entry_date)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_exit_date ON options(exit_date)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_expiration_date ON options(expiration_date)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_strike_price ON options(strike_price)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_entry_price ON options(entry_price)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_exit_price ON options(exit_price)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_brokerage_name ON options(brokerage_name)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_trade_group_id ON options(trade_group_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_parent_trade_id ON options(parent_trade_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_options_is_deleted ON options(is_deleted)",
        libsql::params![],
    )
    .await?;

    // Stock positions table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS stock_positions (
            id TEXT PRIMARY KEY,
            symbol TEXT NOT NULL,
            total_quantity DECIMAL(15,8) NOT NULL DEFAULT 0.0,
            average_entry_price DECIMAL(15,8) NOT NULL DEFAULT 0.0,
            current_price DECIMAL(15,8),
            unrealized_pnl DECIMAL(15,8),
            realized_pnl DECIMAL(15,8) NOT NULL DEFAULT 0.0,
            brokerage_name TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_positions_symbol ON stock_positions(symbol)",
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_stock_positions_brokerage_name ON stock_positions(brokerage_name)", libsql::params![]).await?;

    // Option positions table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS option_positions (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_option_positions_symbol ON option_positions(symbol)",
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_option_positions_strike_exp ON option_positions(symbol, strike_price, expiration_date, option_type)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_option_positions_brokerage_name ON option_positions(brokerage_name)", libsql::params![]).await?;

    // Position transactions table (audit log)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS position_transactions (
            id TEXT PRIMARY KEY,
            position_id TEXT NOT NULL,
            position_type TEXT NOT NULL CHECK (position_type IN ('stock', 'option')),
            trade_id INTEGER,
            transaction_type TEXT NOT NULL CHECK (transaction_type IN ('entry', 'exit', 'adjustment')),
            quantity DECIMAL(15,8) NOT NULL,
            price DECIMAL(15,8) NOT NULL,
            transaction_date TIMESTAMP NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        libsql::params![],
    ).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_position_transactions_position_id ON position_transactions(position_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_position_transactions_position_type ON position_transactions(position_type)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_position_transactions_trade_id ON position_transactions(trade_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_type ON position_transactions(transaction_type)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_date ON position_transactions(transaction_date)", libsql::params![]).await?;

    // Trade notes (linked to trades with AI metadata)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS trade_notes (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    ).await?;
    // Migration: Add missing columns if they don't exist (for existing databases)
    // SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN, so we check first
    {
        let check_col = conn
            .prepare(
                "SELECT COUNT(*) FROM pragma_table_info('trade_notes') WHERE name = 'trade_type'",
            )
            .await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                // Column doesn't exist, add it
                conn.execute(
                    "ALTER TABLE trade_notes ADD COLUMN trade_type TEXT",
                    libsql::params![],
                )
                .await
                .ok();
            }
        }
    }

    {
        let check_col = conn.prepare("SELECT COUNT(*) FROM pragma_table_info('trade_notes') WHERE name = 'stock_trade_id'").await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                conn.execute(
                    "ALTER TABLE trade_notes ADD COLUMN stock_trade_id INTEGER",
                    libsql::params![],
                )
                .await
                .ok();
            }
        }
    }

    {
        let check_col = conn.prepare("SELECT COUNT(*) FROM pragma_table_info('trade_notes') WHERE name = 'option_trade_id'").await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                conn.execute(
                    "ALTER TABLE trade_notes ADD COLUMN option_trade_id INTEGER",
                    libsql::params![],
                )
                .await
                .ok();
            }
        }
    }

    {
        let check_col = conn
            .prepare(
                "SELECT COUNT(*) FROM pragma_table_info('trade_notes') WHERE name = 'ai_metadata'",
            )
            .await?;
        let mut rows = check_col.query(libsql::params![]).await?;
        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            if count == 0 {
                conn.execute(
                    "ALTER TABLE trade_notes ADD COLUMN ai_metadata TEXT",
                    libsql::params![],
                )
                .await
                .ok();
            }
        }
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_updated_at ON trade_notes(updated_at)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_stock_trade_id ON trade_notes(stock_trade_id)",
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_trade_notes_option_trade_id ON trade_notes(option_trade_id)", libsql::params![]).await?;
    // Unique constraint: one note per trade
    conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_stock_unique ON trade_notes(stock_trade_id) WHERE stock_trade_id IS NOT NULL", libsql::params![]).await?;
    conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_option_unique ON trade_notes(option_trade_id) WHERE option_trade_id IS NOT NULL", libsql::params![]).await?;

    // Playbook (existing with new fields)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS playbook (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            icon TEXT,
            emoji TEXT,
            color TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            version INTEGER NOT NULL DEFAULT 0
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE TABLE IF NOT EXISTS stock_trade_playbook (stock_trade_id INTEGER NOT NULL, setup_id TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (stock_trade_id, setup_id), FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE, FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE)", libsql::params![]).await?;
    conn.execute("CREATE TABLE IF NOT EXISTS option_trade_playbook (option_trade_id INTEGER NOT NULL, setup_id TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (option_trade_id, setup_id), FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE, FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE)", libsql::params![]).await?;

    // Playbook rules
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS playbook_rules (
            id TEXT PRIMARY KEY,
            playbook_id TEXT NOT NULL,
            rule_type TEXT NOT NULL CHECK (rule_type IN ('entry_criteria', 'exit_criteria', 'market_factor')),
            title TEXT NOT NULL,
            description TEXT,
            order_position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    ).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_playbook_rules_playbook_id ON playbook_rules(playbook_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_playbook_rules_type ON playbook_rules(rule_type)",
        libsql::params![],
    )
    .await?;

    // Trade rule compliance
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS stock_trade_rule_compliance (
            id TEXT PRIMARY KEY,
            stock_trade_id INTEGER NOT NULL,
            playbook_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            is_followed BOOLEAN NOT NULL DEFAULT false,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
            FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE,
            FOREIGN KEY (rule_id) REFERENCES playbook_rules(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS option_trade_rule_compliance (
            id TEXT PRIMARY KEY,
            option_trade_id INTEGER NOT NULL,
            playbook_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            is_followed BOOLEAN NOT NULL DEFAULT false,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
            FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE,
            FOREIGN KEY (rule_id) REFERENCES playbook_rules(id) ON DELETE CASCADE
        )",
        libsql::params![],
    )
    .await?;

    // Missed trades
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS missed_trades (
            id TEXT PRIMARY KEY,
            playbook_id TEXT NOT NULL,
            symbol TEXT NOT NULL,
            trade_type TEXT NOT NULL CHECK (trade_type IN ('stock', 'option')),
            reason TEXT NOT NULL,
            potential_entry_price REAL,
            opportunity_date TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (playbook_id) REFERENCES playbook(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_missed_trades_playbook_id ON missed_trades(playbook_id)",
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_missed_trades_opportunity_date ON missed_trades(opportunity_date)", libsql::params![]).await?;

    // Replicache (existing)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS replicache_clients (
          client_group_id TEXT NOT NULL,
          client_id TEXT NOT NULL,
          last_mutation_id INTEGER NOT NULL DEFAULT 0,
          last_modified_version INTEGER NOT NULL,
          user_id TEXT NOT NULL,
          created_at TEXT NOT NULL DEFAULT (datetime('now')),
          updated_at TEXT NOT NULL DEFAULT (datetime('now')),
          PRIMARY KEY (client_group_id, client_id)
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS replicache_space_version (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          version INTEGER NOT NULL DEFAULT 0
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "INSERT OR IGNORE INTO replicache_space_version (id, version) VALUES (1, 0)",
        libsql::params![],
    )
    .await?;

    // Trade tags
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS trade_tags (
            id TEXT PRIMARY KEY,
            category TEXT NOT NULL,
            name TEXT NOT NULL,
            color TEXT,
            description TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(category, name)
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_trade_tags_category ON trade_tags(category)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_trade_tags_name ON trade_tags(name)",
        libsql::params![],
    )
    .await?;

    // Stock trade tags junction table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS stock_trade_tags (
            stock_trade_id INTEGER NOT NULL,
            tag_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (stock_trade_id, tag_id),
            FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_stock_id ON stock_trade_tags(stock_trade_id)", libsql::params![]).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_tag_id ON stock_trade_tags(tag_id)",
        libsql::params![],
    )
    .await?;

    // Option trade tags junction table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS option_trade_tags (
            option_trade_id INTEGER NOT NULL,
            tag_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (option_trade_id, tag_id),
            FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_option_trade_tags_option_id ON option_trade_tags(option_trade_id)", libsql::params![]).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_option_trade_tags_tag_id ON option_trade_tags(tag_id)",
        libsql::params![],
    )
    .await?;

    // Notebook: folders (created BEFORE notebook_notes so FK reference works)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notebook_folders (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            is_favorite BOOLEAN NOT NULL DEFAULT false,
            parent_folder_id TEXT,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (parent_folder_id) REFERENCES notebook_folders(id) ON DELETE SET NULL
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_name ON notebook_folders(name)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_is_favorite ON notebook_folders(is_favorite)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_parent_folder_id ON notebook_folders(parent_folder_id)",
        libsql::params![],
    )
    .await?;

    // Notebook: notes (stores Lexical editor state as JSON)
    // Lexical format: {"root":{"children":[...],"direction":"ltr","format":"","indent":0,"type":"root","version":1}}
    // Created AFTER notebook_folders, BEFORE notebook_images so FK references work
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notebook_notes (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_title ON notebook_notes(title)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_pinned ON notebook_notes(is_pinned)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_archived ON notebook_notes(is_archived)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_deleted ON notebook_notes(is_deleted)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_folder_id ON notebook_notes(folder_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_created_at ON notebook_notes(created_at)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_updated_at ON notebook_notes(updated_at)",
        libsql::params![],
    )
    .await?;

    // Notebook: tags (user-created tags for organizing notes)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notebook_tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT,
            is_favorite BOOLEAN NOT NULL DEFAULT false,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_tags_name ON notebook_tags(name)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_tags_is_favorite ON notebook_tags(is_favorite)",
        libsql::params![],
    )
    .await?;

    // Notebook: note-tag junction table (many-to-many relationship)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notebook_note_tags (
            note_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (note_id, tag_id),
            FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES notebook_tags(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_note_id ON notebook_note_tags(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_tag_id ON notebook_note_tags(tag_id)",
        libsql::params![],
    )
    .await?;

    // Notebook: images (references notebook_notes)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS notebook_images (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_images_note_id ON notebook_images(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_notebook_images_created_at ON notebook_images(created_at)",
        libsql::params![],
    )
    .await?;

    // Note collaborators table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_collaborators (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            user_email TEXT NOT NULL,
            user_name TEXT,
            role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
            invited_by TEXT NOT NULL,
            joined_at TEXT NOT NULL,
            last_seen_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_note_id ON note_collaborators(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_id ON note_collaborators(user_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_email ON note_collaborators(user_email)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_collaborators_note_user ON note_collaborators(note_id, user_id)",
        libsql::params![],
    )
    .await?;

    // Note invitations table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_invitations (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_note_id ON note_invitations(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_invitee_email ON note_invitations(invitee_email)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_invitations_token ON note_invitations(token)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_status ON note_invitations(status)",
        libsql::params![],
    )
    .await?;

    // Note share settings table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_share_settings (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_note_id ON note_share_settings(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_owner_id ON note_share_settings(owner_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_public_slug ON note_share_settings(public_slug)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_visibility ON note_share_settings(visibility)",
        libsql::params![],
    )
    .await?;

    // Alert rules table (generic system/price alerts with notes and max triggers)
    conn.execute(
        r#"
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules(user_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_symbol ON alert_rules(symbol)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_created_at ON alert_rules(created_at)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_is_active ON alert_rules(is_active)",
        libsql::params![],
    )
    .await?;

    // Web Push subscriptions
    conn.execute(
        r#"
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user_id ON push_subscriptions(user_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_push_subscriptions_created_at ON push_subscriptions(created_at)",
        libsql::params![],
    )
    .await?;

    // AI Chat Tables
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            title TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            last_message_at TEXT
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_sessions_user_id ON chat_sessions(user_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_at ON chat_sessions(updated_at)",
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            context_vectors TEXT, -- JSON array of vector IDs
            token_count INTEGER,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at)",
        libsql::params![],
    )
    .await?;

    // AI Reports (multi-type, structured payloads)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ai_reports (
            id TEXT PRIMARY KEY,
            time_range TEXT NOT NULL CHECK (time_range IN ('7d', '30d', '90d', 'ytd', '1y')),
            title TEXT NOT NULL,
            analytics TEXT NOT NULL,
            trades TEXT,
            recommendations TEXT,
            metrics TEXT,
            metadata TEXT NOT NULL,
            summary TEXT DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ai_reports_time_range ON ai_reports(time_range)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ai_reports_created_at ON ai_reports(created_at)",
        libsql::params![],
    )
    .await?;

    // AI Analysis (streamed markdown, no analytics payload)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ai_analysis (
            id TEXT PRIMARY KEY,
            time_range TEXT NOT NULL CHECK (time_range IN ('7d', '30d', '90d', 'ytd', '1y')),
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ai_analysis_time_range ON ai_analysis(time_range)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ai_analysis_created_at ON ai_analysis(created_at)",
        libsql::params![],
    )
    .await?;

    // Triggers
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_stocks_timestamp
        AFTER UPDATE ON stocks
        FOR EACH ROW
        BEGIN
            UPDATE stocks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_options_timestamp
        AFTER UPDATE ON options
        FOR EACH ROW
        BEGIN
            UPDATE options SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_user_profile_timestamp
        AFTER UPDATE ON user_profile
        FOR EACH ROW
        BEGIN
            UPDATE user_profile SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_trade_notes_timestamp
        AFTER UPDATE ON trade_notes
        FOR EACH ROW
        BEGIN
            UPDATE trade_notes SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_trade_tags_timestamp
        AFTER UPDATE ON trade_tags
        FOR EACH ROW
        BEGIN
            UPDATE trade_tags SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_images_timestamp
        AFTER UPDATE ON images
        FOR EACH ROW
        BEGIN
            UPDATE images SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_playbook_timestamp
        AFTER UPDATE ON playbook
        FOR EACH ROW
        BEGIN
            UPDATE playbook SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_notebook_images_timestamp
        AFTER UPDATE ON notebook_images
        FOR EACH ROW
        BEGIN
            UPDATE notebook_images SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_notebook_notes_timestamp
        AFTER UPDATE ON notebook_notes
        FOR EACH ROW
        BEGIN
            UPDATE notebook_notes SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_notebook_folders_timestamp
        AFTER UPDATE ON notebook_folders
        FOR EACH ROW
        BEGIN
            UPDATE notebook_folders SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_notebook_tags_timestamp
        AFTER UPDATE ON notebook_tags
        FOR EACH ROW
        BEGIN
            UPDATE notebook_tags SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    // Brokerage connections table (SnapTrade integration)
    conn.execute(
        r#"
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
        )
        "#,
        libsql::params![],
    ).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_connections_user_id ON brokerage_connections(user_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_connections_connection_id ON brokerage_connections(connection_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_connections_status ON brokerage_connections(status)", libsql::params![]).await?;

    // Brokerage accounts table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS brokerage_accounts (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_accounts_connection_id ON brokerage_accounts(connection_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_accounts_snaptrade_account_id ON brokerage_accounts(snaptrade_account_id)", libsql::params![]).await?;

    // Brokerage transactions table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS brokerage_transactions (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_account_id ON brokerage_transactions(account_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_trade_date ON brokerage_transactions(trade_date)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_symbol ON brokerage_transactions(symbol)", libsql::params![]).await?;

    // Add is_transformed column if it doesn't exist (migration)
    let check_col = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('brokerage_transactions') WHERE name = 'is_transformed'")
        .await?;
    let mut rows = check_col.query(libsql::params![]).await?;
    if let Some(row) = rows.next().await? {
        let count: i64 = row.get(0)?;
        if count == 0 {
            conn.execute(
                "ALTER TABLE brokerage_transactions ADD COLUMN is_transformed INTEGER DEFAULT 0",
                libsql::params![],
            )
            .await?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_is_transformed ON brokerage_transactions(is_transformed)", libsql::params![]).await?;
        }
    }

    // Brokerage holdings table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS brokerage_holdings (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_brokerage_holdings_account_id ON brokerage_holdings(account_id)", libsql::params![]).await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_brokerage_holdings_symbol ON brokerage_holdings(symbol)",
        libsql::params![],
    )
    .await?;

    // Unmatched transactions table (for manual review of difficult matches)
    conn.execute(
        r#"
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
        )
        "#,
        libsql::params![],
    ).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_user_id ON unmatched_transactions(user_id)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_symbol ON unmatched_transactions(symbol)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_status ON unmatched_transactions(status)", libsql::params![]).await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_trade_date ON unmatched_transactions(trade_date)", libsql::params![]).await?;

    // ==================== CALENDAR TABLES ====================

    // Calendar connections (OAuth connections to Google/Microsoft)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS calendar_connections (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_connections_provider ON calendar_connections(provider)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_connections_enabled ON calendar_connections(enabled)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_connections_provider_email ON calendar_connections(provider, account_email)",
        libsql::params![],
    )
    .await?;

    // Calendar sync cursors (track sync state per external calendar)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS calendar_sync_cursors (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            external_calendar_id TEXT NOT NULL,
            external_calendar_name TEXT,
            sync_token TEXT,
            last_sync_at TIMESTAMP,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (connection_id) REFERENCES calendar_connections(id) ON DELETE CASCADE,
            UNIQUE(connection_id, external_calendar_id)
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_sync_cursors_connection_id ON calendar_sync_cursors(connection_id)",
        libsql::params![],
    )
    .await?;

    // Calendar events (both local reminders and synced events)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS calendar_events (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_start_time ON calendar_events(start_time)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_end_time ON calendar_events(end_time)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_source ON calendar_events(source)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_external_id ON calendar_events(external_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_connection_id ON calendar_events(connection_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_is_reminder ON calendar_events(is_reminder)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_events_external_unique ON calendar_events(source, external_id) WHERE external_id IS NOT NULL",
        libsql::params![],
    )
    .await?;

    // Trigger for calendar_connections updated_at
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_calendar_connections_timestamp
        AFTER UPDATE ON calendar_connections
        FOR EACH ROW
        BEGIN
            UPDATE calendar_connections SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    // Trigger for calendar_sync_cursors updated_at
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_calendar_sync_cursors_timestamp
        AFTER UPDATE ON calendar_sync_cursors
        FOR EACH ROW
        BEGIN
            UPDATE calendar_sync_cursors SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    // Trigger for calendar_events updated_at
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_calendar_events_timestamp
        AFTER UPDATE ON calendar_events
        FOR EACH ROW
        BEGIN
            UPDATE calendar_events SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    // ==================== COLLABORATION TABLES ====================

    // Note collaborators (users who have access to a note)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_collaborators (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            user_email TEXT NOT NULL,
            user_name TEXT,
            role TEXT NOT NULL DEFAULT 'viewer' CHECK (role IN ('owner', 'editor', 'viewer')),
            invited_by TEXT NOT NULL,
            joined_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_seen_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE,
            UNIQUE(note_id, user_id)
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_note_id ON note_collaborators(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_id ON note_collaborators(user_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_role ON note_collaborators(role)",
        libsql::params![],
    )
    .await?;

    // Note invitations (pending invites to collaborate)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_invitations (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            inviter_id TEXT NOT NULL,
            inviter_email TEXT NOT NULL,
            invitee_email TEXT NOT NULL,
            invitee_user_id TEXT,
            role TEXT NOT NULL DEFAULT 'editor' CHECK (role IN ('editor', 'viewer')),
            token TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'expired', 'revoked')),
            message TEXT,
            expires_at TEXT NOT NULL,
            accepted_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_note_id ON note_invitations(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_invitee_email ON note_invitations(invitee_email)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_token ON note_invitations(token)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_status ON note_invitations(status)",
        libsql::params![],
    )
    .await?;

    // Note share settings (visibility and public sharing)
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS note_share_settings (
            id TEXT PRIMARY KEY,
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
        )
        "#,
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_note_id ON note_share_settings(note_id)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_public_slug ON note_share_settings(public_slug)",
        libsql::params![],
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_visibility ON note_share_settings(visibility)",
        libsql::params![],
    )
    .await?;

    // Triggers for collaboration tables
    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_note_collaborators_timestamp
        AFTER UPDATE ON note_collaborators
        FOR EACH ROW
        BEGIN
            UPDATE note_collaborators SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_note_invitations_timestamp
        AFTER UPDATE ON note_invitations
        FOR EACH ROW
        BEGIN
            UPDATE note_invitations SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    conn.execute(
        r#"
        CREATE TRIGGER IF NOT EXISTS update_note_share_settings_timestamp
        AFTER UPDATE ON note_share_settings
        FOR EACH ROW
        BEGIN
            UPDATE note_share_settings SET updated_at = datetime('now') WHERE id = NEW.id;
        END
        "#,
        libsql::params![],
    )
    .await?;

    info!("Trading+notebook+calendar+collaboration schema initialized successfully");
    Ok(())
}


/// Current schema version (bumped for collaboration tables)
pub fn get_current_schema_version() -> SchemaVersion {
    SchemaVersion {
        version: "0.0.52".to_string(),
        description: "Added collaboration tables for note sharing and real-time editing".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Expected schema for synchronization
pub fn get_expected_schema() -> Vec<TableSchema> {
    let mut schemas: Vec<TableSchema> = vec![
        // Stocks
        TableSchema {
            name: "stocks".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "symbol".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "order_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "entry_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "exit_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "stop_loss".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "commissions".to_string(),
                    data_type: "DECIMAL(10,4)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.00".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "number_shares".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "take_profit".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "initial_target".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "profit_target".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_ratings".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "entry_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "exit_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "reviewed".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "mistakes".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "brokerage_name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_group_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "parent_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "total_quantity".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "transaction_sequence".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "is_deleted".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_stocks_symbol".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["symbol".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_trade_type".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["trade_type".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_entry_date".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["entry_date".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_exit_date".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["exit_date".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_entry_price".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["entry_price".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_exit_price".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["exit_price".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_brokerage_name".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["brokerage_name".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_trade_group_id".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["trade_group_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_parent_trade_id".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["parent_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stocks_is_deleted".to_string(),
                    table_name: "stocks".to_string(),
                    columns: vec!["is_deleted".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_stocks_timestamp".to_string(),
                table_name: "stocks".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE stocks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        // Options
        TableSchema {
            name: "options".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "symbol".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "option_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "strike_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "expiration_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "entry_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "exit_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "premium".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "commissions".to_string(),
                    data_type: "DECIMAL(10,4)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.00".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "entry_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "exit_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "status".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("'open'".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "initial_target".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "profit_target".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_ratings".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "reviewed".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "mistakes".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "brokerage_name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_group_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "parent_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "total_quantity".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "transaction_sequence".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "is_deleted".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_options_symbol".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["symbol".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_option_type".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["option_type".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_status".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["status".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_entry_date".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["entry_date".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_exit_date".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["exit_date".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_expiration_date".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["expiration_date".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_strike_price".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["strike_price".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_entry_price".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["entry_price".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_exit_price".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["exit_price".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_brokerage_name".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["brokerage_name".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_trade_group_id".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["trade_group_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_parent_trade_id".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["parent_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_options_is_deleted".to_string(),
                    table_name: "options".to_string(),
                    columns: vec!["is_deleted".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_options_timestamp".to_string(),
                table_name: "options".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE options SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        // Stock positions (aggregate view for position tracking)
        TableSchema {
            name: "stock_positions".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "symbol".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "total_quantity".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "average_entry_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "current_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "unrealized_pnl".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "realized_pnl".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "brokerage_name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_stock_positions_symbol".to_string(),
                    table_name: "stock_positions".to_string(),
                    columns: vec!["symbol".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stock_positions_brokerage_name".to_string(),
                    table_name: "stock_positions".to_string(),
                    columns: vec!["brokerage_name".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_stock_positions_timestamp".to_string(),
                table_name: "stock_positions".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action:
                    "UPDATE stock_positions SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id"
                        .to_string(),
            }],
        },
        // Option positions (aggregate view for position tracking)
        TableSchema {
            name: "option_positions".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "symbol".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "option_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "strike_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "expiration_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "total_quantity".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "average_entry_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "current_price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "unrealized_pnl".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "realized_pnl".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: Some("0.0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "brokerage_name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_option_positions_symbol".to_string(),
                    table_name: "option_positions".to_string(),
                    columns: vec!["symbol".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_option_positions_strike_exp".to_string(),
                    table_name: "option_positions".to_string(),
                    columns: vec![
                        "symbol".to_string(),
                        "strike_price".to_string(),
                        "expiration_date".to_string(),
                        "option_type".to_string(),
                    ],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_option_positions_brokerage_name".to_string(),
                    table_name: "option_positions".to_string(),
                    columns: vec!["brokerage_name".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_option_positions_timestamp".to_string(),
                table_name: "option_positions".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action:
                    "UPDATE option_positions SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id"
                        .to_string(),
            }],
        },
        // Position transactions (audit log for all position changes)
        TableSchema {
            name: "position_transactions".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "position_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "position_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "transaction_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "quantity".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "price".to_string(),
                    data_type: "DECIMAL(15,8)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "transaction_date".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_position_transactions_position_id".to_string(),
                    table_name: "position_transactions".to_string(),
                    columns: vec!["position_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_position_transactions_position_type".to_string(),
                    table_name: "position_transactions".to_string(),
                    columns: vec!["position_type".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_position_transactions_trade_id".to_string(),
                    table_name: "position_transactions".to_string(),
                    columns: vec!["trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_position_transactions_transaction_type".to_string(),
                    table_name: "position_transactions".to_string(),
                    columns: vec!["transaction_type".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_position_transactions_transaction_date".to_string(),
                    table_name: "position_transactions".to_string(),
                    columns: vec!["transaction_date".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        // Trade notes (linked to trades with AI metadata)
        TableSchema {
            name: "trade_notes".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "content".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: Some("''".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "stock_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "option_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "ai_metadata".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_trade_notes_updated_at".to_string(),
                    table_name: "trade_notes".to_string(),
                    columns: vec!["updated_at".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_trade_notes_stock_trade_id".to_string(),
                    table_name: "trade_notes".to_string(),
                    columns: vec!["stock_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_trade_notes_option_trade_id".to_string(),
                    table_name: "trade_notes".to_string(),
                    columns: vec!["option_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_trade_notes_stock_unique".to_string(),
                    table_name: "trade_notes".to_string(),
                    columns: vec!["stock_trade_id".to_string()],
                    is_unique: true,
                },
                IndexInfo {
                    name: "idx_trade_notes_option_unique".to_string(),
                    table_name: "trade_notes".to_string(),
                    columns: vec!["option_trade_id".to_string()],
                    is_unique: true,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_trade_notes_timestamp".to_string(),
                table_name: "trade_notes".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE trade_notes SET updated_at = datetime('now') WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        // User profile
        TableSchema {
            name: "user_profile".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "display_name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "nickname".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "timezone".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: Some("'UTC'".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "currency".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: Some("'USD'".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "profile_picture_uuid".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trading_experience_level".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "primary_trading_goal".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "asset_types".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trading_style".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TIMESTAMP".to_string(),
                    is_nullable: false,
                    default_value: Some("CURRENT_TIMESTAMP".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![],
            triggers: vec![TriggerInfo {
                name: "update_user_profile_timestamp".to_string(),
                table_name: "user_profile".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE user_profile SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        // Trade tags
        TableSchema {
            name: "trade_tags".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "category".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "color".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "description".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_trade_tags_category".to_string(),
                    table_name: "trade_tags".to_string(),
                    columns: vec!["category".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_trade_tags_name".to_string(),
                    table_name: "trade_tags".to_string(),
                    columns: vec!["name".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_trade_tags_category_name_unique".to_string(),
                    table_name: "trade_tags".to_string(),
                    columns: vec!["category".to_string(), "name".to_string()],
                    is_unique: true,
                },
            ],
            triggers: vec![TriggerInfo {
                name: "update_trade_tags_timestamp".to_string(),
                table_name: "trade_tags".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE trade_tags SET updated_at = datetime('now') WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        // Stock trade tags junction
        TableSchema {
            name: "stock_trade_tags".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "stock_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "tag_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_stock_trade_tags_stock_id".to_string(),
                    table_name: "stock_trade_tags".to_string(),
                    columns: vec!["stock_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stock_trade_tags_tag_id".to_string(),
                    table_name: "stock_trade_tags".to_string(),
                    columns: vec!["tag_id".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        // Option trade tags junction
        TableSchema {
            name: "option_trade_tags".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "option_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "tag_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_option_trade_tags_option_id".to_string(),
                    table_name: "option_trade_tags".to_string(),
                    columns: vec!["option_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_option_trade_tags_tag_id".to_string(),
                    table_name: "option_trade_tags".to_string(),
                    columns: vec!["tag_id".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        // Playbook + junction tables
        TableSchema {
            name: "playbook".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "description".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "icon".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "emoji".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "color".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "version".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![IndexInfo {
                name: "idx_playbook_updated_at".to_string(),
                table_name: "playbook".to_string(),
                columns: vec!["updated_at".to_string()],
                is_unique: false,
            }],
            triggers: vec![TriggerInfo {
                name: "update_playbook_timestamp".to_string(),
                table_name: "playbook".to_string(),
                event: "UPDATE".to_string(),
                timing: "AFTER".to_string(),
                action: "UPDATE playbook SET updated_at = datetime('now') WHERE id = NEW.id"
                    .to_string(),
            }],
        },
        TableSchema {
            name: "stock_trade_playbook".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "stock_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "setup_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_stock_trade_playbook_stock_trade_id".to_string(),
                    table_name: "stock_trade_playbook".to_string(),
                    columns: vec!["stock_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_stock_trade_playbook_setup_id".to_string(),
                    table_name: "stock_trade_playbook".to_string(),
                    columns: vec!["setup_id".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        TableSchema {
            name: "option_trade_playbook".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "option_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "setup_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_option_trade_playbook_option_trade_id".to_string(),
                    table_name: "option_trade_playbook".to_string(),
                    columns: vec!["option_trade_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_option_trade_playbook_setup_id".to_string(),
                    table_name: "option_trade_playbook".to_string(),
                    columns: vec!["setup_id".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        // Playbook rules
        TableSchema {
            name: "playbook_rules".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "playbook_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "rule_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "title".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "description".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "order_position".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: Some("0".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "updated_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_playbook_rules_playbook_id".to_string(),
                    table_name: "playbook_rules".to_string(),
                    columns: vec!["playbook_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_playbook_rules_type".to_string(),
                    table_name: "playbook_rules".to_string(),
                    columns: vec!["rule_type".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
        // Trade rule compliance
        TableSchema {
            name: "stock_trade_rule_compliance".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "stock_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "playbook_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "rule_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "is_followed".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "notes".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![],
            triggers: vec![],
        },
        TableSchema {
            name: "option_trade_rule_compliance".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "option_trade_id".to_string(),
                    data_type: "INTEGER".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "playbook_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "rule_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "is_followed".to_string(),
                    data_type: "BOOLEAN".to_string(),
                    is_nullable: false,
                    default_value: Some("false".to_string()),
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "notes".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![],
            triggers: vec![],
        },
        // Missed trades
        TableSchema {
            name: "missed_trades".to_string(),
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: true,
                },
                ColumnInfo {
                    name: "playbook_id".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "symbol".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "trade_type".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "reason".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "potential_entry_price".to_string(),
                    data_type: "REAL".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "opportunity_date".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "notes".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: true,
                    default_value: None,
                    is_primary_key: false,
                },
                ColumnInfo {
                    name: "created_at".to_string(),
                    data_type: "TEXT".to_string(),
                    is_nullable: false,
                    default_value: Some("(datetime('now'))".to_string()),
                    is_primary_key: false,
                },
            ],
            indexes: vec![
                IndexInfo {
                    name: "idx_missed_trades_playbook_id".to_string(),
                    table_name: "missed_trades".to_string(),
                    columns: vec!["playbook_id".to_string()],
                    is_unique: false,
                },
                IndexInfo {
                    name: "idx_missed_trades_opportunity_date".to_string(),
                    table_name: "missed_trades".to_string(),
                    columns: vec!["opportunity_date".to_string()],
                    is_unique: false,
                },
            ],
            triggers: vec![],
        },
    ];

    // Notebook folders table (created BEFORE notebook_notes so FK reference works)
    schemas.push(TableSchema {
        name: "notebook_folders".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_favorite".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "parent_folder_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "position".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_notebook_folders_name".to_string(),
                table_name: "notebook_folders".to_string(),
                columns: vec!["name".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_folders_is_favorite".to_string(),
                table_name: "notebook_folders".to_string(),
                columns: vec!["is_favorite".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_folders_parent_folder_id".to_string(),
                table_name: "notebook_folders".to_string(),
                columns: vec!["parent_folder_id".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_notebook_folders_timestamp".to_string(),
            table_name: "notebook_folders".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE notebook_folders SET updated_at = datetime('now') WHERE id = NEW.id"
                .to_string(),
        }],
    });

    // Notebook notes table (stores Lexical editor state as JSON)
    // Created AFTER notebook_folders, BEFORE notebook_images so FK references work
    schemas.push(TableSchema {
        name: "notebook_notes".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'Untitled'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "content".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'{}'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "content_plain_text".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "word_count".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_pinned".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_archived".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_deleted".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "folder_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "tags".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "y_state".to_string(),
                data_type: "BLOB".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_synced_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_notebook_notes_title".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["title".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_is_pinned".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["is_pinned".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_is_archived".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["is_archived".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_is_deleted".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["is_deleted".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_folder_id".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["folder_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_created_at".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_notes_updated_at".to_string(),
                table_name: "notebook_notes".to_string(),
                columns: vec!["updated_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_notebook_notes_timestamp".to_string(),
            table_name: "notebook_notes".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE notebook_notes SET updated_at = datetime('now') WHERE id = NEW.id"
                .to_string(),
        }],
    });

    // Notebook tags table (user-created tags for organizing notes)
    schemas.push(TableSchema {
        name: "notebook_tags".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "color".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_favorite".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_notebook_tags_name".to_string(),
                table_name: "notebook_tags".to_string(),
                columns: vec!["name".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_tags_is_favorite".to_string(),
                table_name: "notebook_tags".to_string(),
                columns: vec!["is_favorite".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_notebook_tags_timestamp".to_string(),
            table_name: "notebook_tags".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE notebook_tags SET updated_at = datetime('now') WHERE id = NEW.id"
                .to_string(),
        }],
    });

    // Notebook note-tag junction table (many-to-many)
    schemas.push(TableSchema {
        name: "notebook_note_tags".to_string(),
        columns: vec![
            ColumnInfo {
                name: "note_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "tag_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_notebook_note_tags_note_id".to_string(),
                table_name: "notebook_note_tags".to_string(),
                columns: vec!["note_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_note_tags_tag_id".to_string(),
                table_name: "notebook_note_tags".to_string(),
                columns: vec!["tag_id".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    // Notebook images table (references notebook_notes via FK)
    schemas.push(TableSchema {
        name: "notebook_images".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "note_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "src".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "storage_path".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "alt_text".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "caption".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "width".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "height".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "position".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_notebook_images_note_id".to_string(),
                table_name: "notebook_images".to_string(),
                columns: vec!["note_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_notebook_images_created_at".to_string(),
                table_name: "notebook_images".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_notebook_images_timestamp".to_string(),
            table_name: "notebook_images".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE notebook_images SET updated_at = datetime('now') WHERE id = NEW.id"
                .to_string(),
        }],
    });

    // AI Chat Tables
    schemas.push(TableSchema {
        name: "chat_sessions".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "message_count".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_message_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_chat_sessions_user_id".to_string(),
                table_name: "chat_sessions".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_chat_sessions_updated_at".to_string(),
                table_name: "chat_sessions".to_string(),
                columns: vec!["updated_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "chat_messages".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "session_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "role".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "content".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "context_vectors".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "token_count".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_chat_messages_session_id".to_string(),
                table_name: "chat_messages".to_string(),
                columns: vec!["session_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_chat_messages_created_at".to_string(),
                table_name: "chat_messages".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    // AI Reports Tables
    schemas.push(TableSchema {
        name: "ai_reports".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "time_range".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "analytics".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "trades".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "recommendations".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "metrics".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "metadata".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "summary".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: Some("''".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_ai_reports_time_range".to_string(),
                table_name: "ai_reports".to_string(),
                columns: vec!["time_range".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_ai_reports_created_at".to_string(),
                table_name: "ai_reports".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "ai_analysis".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "time_range".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "content".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_ai_analysis_time_range".to_string(),
                table_name: "ai_analysis".to_string(),
                columns: vec!["time_range".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_ai_analysis_created_at".to_string(),
                table_name: "ai_analysis".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    // Brokerage tables (SnapTrade integration)
    schemas.push(TableSchema {
        name: "brokerage_connections".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "snaptrade_user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "snaptrade_user_secret".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "connection_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "brokerage_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "status".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'pending'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_sync_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_brokerage_connections_user_id".to_string(),
                table_name: "brokerage_connections".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_connections_connection_id".to_string(),
                table_name: "brokerage_connections".to_string(),
                columns: vec!["connection_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_connections_status".to_string(),
                table_name: "brokerage_connections".to_string(),
                columns: vec!["status".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "brokerage_accounts".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "connection_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "snaptrade_account_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "account_number".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "account_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "account_type".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "balance".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "currency".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'USD'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "institution_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "raw_data".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_brokerage_accounts_connection_id".to_string(),
                table_name: "brokerage_accounts".to_string(),
                columns: vec!["connection_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_accounts_snaptrade_account_id".to_string(),
                table_name: "brokerage_accounts".to_string(),
                columns: vec!["snaptrade_account_id".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "brokerage_transactions".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "account_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "snaptrade_transaction_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "symbol".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "transaction_type".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "quantity".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "price".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "amount".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "currency".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'USD'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "trade_date".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "settlement_date".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "fees".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "raw_data".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_transformed".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_brokerage_transactions_account_id".to_string(),
                table_name: "brokerage_transactions".to_string(),
                columns: vec!["account_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_transactions_trade_date".to_string(),
                table_name: "brokerage_transactions".to_string(),
                columns: vec!["trade_date".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_transactions_symbol".to_string(),
                table_name: "brokerage_transactions".to_string(),
                columns: vec!["symbol".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_transactions_is_transformed".to_string(),
                table_name: "brokerage_transactions".to_string(),
                columns: vec!["is_transformed".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "brokerage_holdings".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "account_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "symbol".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "quantity".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "average_cost".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "current_price".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "market_value".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "currency".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'USD'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_updated".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "raw_data".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_brokerage_holdings_account_id".to_string(),
                table_name: "brokerage_holdings".to_string(),
                columns: vec!["account_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_brokerage_holdings_symbol".to_string(),
                table_name: "brokerage_holdings".to_string(),
                columns: vec!["symbol".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    schemas.push(TableSchema {
        name: "unmatched_transactions".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "transaction_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "snaptrade_transaction_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "symbol".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "trade_type".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "units".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "price".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "fee".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "trade_date".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "brokerage_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "raw_data".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_option".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "difficulty_reason".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "confidence_score".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "suggested_matches".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "status".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'pending'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "resolved_trade_id".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "resolved_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: Some("CURRENT_TIMESTAMP".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_unmatched_transactions_user_id".to_string(),
                table_name: "unmatched_transactions".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_unmatched_transactions_symbol".to_string(),
                table_name: "unmatched_transactions".to_string(),
                columns: vec!["symbol".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_unmatched_transactions_status".to_string(),
                table_name: "unmatched_transactions".to_string(),
                columns: vec!["status".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_unmatched_transactions_trade_date".to_string(),
                table_name: "unmatched_transactions".to_string(),
                columns: vec!["trade_date".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    // Alert rules
    schemas.push(TableSchema {
        name: "alert_rules".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "alert_type".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "symbol".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "operator".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "threshold".to_string(),
                data_type: "REAL".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "channels".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "cooldown_seconds".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_fired_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "note".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "max_triggers".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "fired_count".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_active".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("1".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "metadata".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_alert_rules_user_id".to_string(),
                table_name: "alert_rules".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_alert_rules_symbol".to_string(),
                table_name: "alert_rules".to_string(),
                columns: vec!["symbol".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_alert_rules_created_at".to_string(),
                table_name: "alert_rules".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_alert_rules_is_active".to_string(),
                table_name: "alert_rules".to_string(),
                columns: vec!["is_active".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_alert_rules_timestamp".to_string(),
            table_name: "alert_rules".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE alert_rules SET updated_at = datetime('now') WHERE id = NEW.id"
                .to_string(),
        }],
    });

    // Push subscriptions
    schemas.push(TableSchema {
        name: "push_subscriptions".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "endpoint".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "p256dh".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "auth".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "ua".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "topics".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_push_subscriptions_user_id".to_string(),
                table_name: "push_subscriptions".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_push_subscriptions_created_at".to_string(),
                table_name: "push_subscriptions".to_string(),
                columns: vec!["created_at".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![],
    });

    // ==================== CALENDAR TABLES ====================

    // Calendar connections (OAuth connections to Google/Microsoft)
    schemas.push(TableSchema {
        name: "calendar_connections".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "provider".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "access_token".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "refresh_token".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "token_expires_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "account_email".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "account_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "enabled".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("true".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_sync_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "sync_error".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_calendar_connections_provider".to_string(),
                table_name: "calendar_connections".to_string(),
                columns: vec!["provider".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_connections_enabled".to_string(),
                table_name: "calendar_connections".to_string(),
                columns: vec!["enabled".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_calendar_connections_timestamp".to_string(),
            table_name: "calendar_connections".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE calendar_connections SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    // Calendar sync cursors (track sync state per external calendar)
    schemas.push(TableSchema {
        name: "calendar_sync_cursors".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "connection_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "external_calendar_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "external_calendar_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "sync_token".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_sync_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![IndexInfo {
            name: "idx_calendar_sync_cursors_connection_id".to_string(),
            table_name: "calendar_sync_cursors".to_string(),
            columns: vec!["connection_id".to_string()],
            is_unique: false,
        }],
        triggers: vec![TriggerInfo {
            name: "update_calendar_sync_cursors_timestamp".to_string(),
            table_name: "calendar_sync_cursors".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE calendar_sync_cursors SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    // Calendar events (both local reminders and synced events)
    schemas.push(TableSchema {
        name: "calendar_events".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "title".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "description".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "start_time".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "end_time".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "all_day".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "reminder_minutes".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "recurrence_rule".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "source".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'local'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "external_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "external_calendar_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "connection_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "color".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "location".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "status".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'confirmed'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "is_reminder".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "reminder_sent".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "metadata".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "synced_at".to_string(),
                data_type: "TIMESTAMP".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_calendar_events_start_time".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["start_time".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_events_end_time".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["end_time".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_events_source".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["source".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_events_external_id".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["external_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_events_connection_id".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["connection_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_calendar_events_is_reminder".to_string(),
                table_name: "calendar_events".to_string(),
                columns: vec!["is_reminder".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_calendar_events_timestamp".to_string(),
            table_name: "calendar_events".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE calendar_events SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    // Note collaborators table
    schemas.push(TableSchema {
        name: "note_collaborators".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "note_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "user_email".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "user_name".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "role".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "invited_by".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "joined_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "last_seen_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_note_collaborators_note_id".to_string(),
                table_name: "note_collaborators".to_string(),
                columns: vec!["note_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_collaborators_user_id".to_string(),
                table_name: "note_collaborators".to_string(),
                columns: vec!["user_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_collaborators_user_email".to_string(),
                table_name: "note_collaborators".to_string(),
                columns: vec!["user_email".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_collaborators_note_user".to_string(),
                table_name: "note_collaborators".to_string(),
                columns: vec!["note_id".to_string(), "user_id".to_string()],
                is_unique: true,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_note_collaborators_timestamp".to_string(),
            table_name: "note_collaborators".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE note_collaborators SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    // Note invitations table
    schemas.push(TableSchema {
        name: "note_invitations".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "note_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "inviter_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "inviter_email".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "invitee_email".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "invitee_user_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "role".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "token".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "status".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'pending'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "message".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "expires_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "accepted_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_note_invitations_note_id".to_string(),
                table_name: "note_invitations".to_string(),
                columns: vec!["note_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_invitations_invitee_email".to_string(),
                table_name: "note_invitations".to_string(),
                columns: vec!["invitee_email".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_invitations_token".to_string(),
                table_name: "note_invitations".to_string(),
                columns: vec!["token".to_string()],
                is_unique: true,
            },
            IndexInfo {
                name: "idx_note_invitations_status".to_string(),
                table_name: "note_invitations".to_string(),
                columns: vec!["status".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_note_invitations_timestamp".to_string(),
            table_name: "note_invitations".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE note_invitations SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    // Note share settings table
    schemas.push(TableSchema {
        name: "note_share_settings".to_string(),
        columns: vec![
            ColumnInfo {
                name: "id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: true,
            },
            ColumnInfo {
                name: "note_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "owner_id".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "visibility".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("'private'".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "public_slug".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "allow_comments".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("false".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "allow_copy".to_string(),
                data_type: "BOOLEAN".to_string(),
                is_nullable: false,
                default_value: Some("true".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "password_hash".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: true,
                default_value: None,
                is_primary_key: false,
            },
            ColumnInfo {
                name: "view_count".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: false,
                default_value: Some("0".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "created_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
            ColumnInfo {
                name: "updated_at".to_string(),
                data_type: "TEXT".to_string(),
                is_nullable: false,
                default_value: Some("(datetime('now'))".to_string()),
                is_primary_key: false,
            },
        ],
        indexes: vec![
            IndexInfo {
                name: "idx_note_share_settings_note_id".to_string(),
                table_name: "note_share_settings".to_string(),
                columns: vec!["note_id".to_string()],
                is_unique: true,
            },
            IndexInfo {
                name: "idx_note_share_settings_owner_id".to_string(),
                table_name: "note_share_settings".to_string(),
                columns: vec!["owner_id".to_string()],
                is_unique: false,
            },
            IndexInfo {
                name: "idx_note_share_settings_public_slug".to_string(),
                table_name: "note_share_settings".to_string(),
                columns: vec!["public_slug".to_string()],
                is_unique: true,
            },
            IndexInfo {
                name: "idx_note_share_settings_visibility".to_string(),
                table_name: "note_share_settings".to_string(),
                columns: vec!["visibility".to_string()],
                is_unique: false,
            },
        ],
        triggers: vec![TriggerInfo {
            name: "update_note_share_settings_timestamp".to_string(),
            table_name: "note_share_settings".to_string(),
            event: "UPDATE".to_string(),
            timing: "AFTER".to_string(),
            action: "UPDATE note_share_settings SET updated_at = datetime('now') WHERE id = NEW.id".to_string(),
        }],
    });

    schemas
}

/// Initialize the schema version table if needed
pub async fn initialize_schema_version_table(conn: &Connection) -> Result<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            description TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
        libsql::params![],
    )
    .await?;
    Ok(())
}

/// Update schema version in the database
pub async fn update_schema_version(conn: &Connection, version: &SchemaVersion) -> Result<()> {
    conn.execute(
        "INSERT INTO schema_version (version, description, created_at) VALUES (?, ?, ?)",
        libsql::params![
            version.version.clone(),
            version.description.clone(),
            version.created_at.clone()
        ],
    )
    .await?;
    Ok(())
}

/// Ensure indexes for a table
pub async fn ensure_indexes(conn: &Connection, table_schema: &TableSchema) -> Result<()> {
    for index in &table_schema.indexes {
        let mut rows = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name=?")
            .await?
            .query(libsql::params![index.name.clone()])
            .await?;
        if rows.next().await?.is_none() {
            let create_index_sql = format!(
                "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
                index.name,
                index.table_name,
                index.columns.join(", ")
            );
            conn.execute(&create_index_sql, libsql::params![]).await?;
        }
    }
    Ok(())
}

/// Ensure triggers for a table
pub async fn ensure_triggers(conn: &Connection, table_schema: &TableSchema) -> Result<()> {
    for trigger in &table_schema.triggers {
        let mut rows = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND name=?")
            .await?
            .query(libsql::params![trigger.name.clone()])
            .await?;
        if rows.next().await?.is_none() {
            let create_trigger_sql = format!(
                "CREATE TRIGGER IF NOT EXISTS {} {} {} ON {} FOR EACH ROW BEGIN {}; END",
                trigger.name, trigger.timing, trigger.event, trigger.table_name, trigger.action
            );
            conn.execute(&create_trigger_sql, libsql::params![]).await?;
        }
    }
    Ok(())
}

/// Get list of current tables in the database
pub async fn get_current_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut tables = Vec::new();
    let mut rows = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .await?
        .query(libsql::params![])
        .await?;
    while let Some(row) = rows.next().await? {
        tables.push(row.get(0)?);
    }
    Ok(tables)
}

/// Create a table based on schema definition
pub async fn create_table(conn: &Connection, table_schema: &TableSchema) -> Result<()> {
    let mut create_sql = format!("CREATE TABLE IF NOT EXISTS {} (", table_schema.name);
    let primary_keys: Vec<String> = table_schema
        .columns
        .iter()
        .filter(|c| c.is_primary_key)
        .map(|c| c.name.clone())
        .collect();
    let column_definitions: Vec<String> = table_schema
        .columns
        .iter()
        .map(|col| {
            let mut def = format!("{} {}", col.name, col.data_type);
            if col.is_primary_key && primary_keys.len() == 1 {
                if col.data_type.to_uppercase().contains("INTEGER") {
                    def.push_str(" PRIMARY KEY AUTOINCREMENT");
                } else {
                    def.push_str(" PRIMARY KEY");
                }
            } else if !col.is_nullable {
                def.push_str(" NOT NULL");
            }
            if let Some(default) = &col.default_value {
                def.push_str(&format!(" DEFAULT {}", default));
            }
            def
        })
        .collect();
    create_sql.push_str(&column_definitions.join(", "));
    if primary_keys.len() > 1 {
        create_sql.push_str(&format!(", PRIMARY KEY ({})", primary_keys.join(", ")));
    }
    create_sql.push(')');
    conn.execute(&create_sql, libsql::params![]).await?;
    Ok(())
}

/// Get current columns for a table
pub async fn get_table_columns(conn: &Connection, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let mut columns = Vec::new();
    let mut rows = conn
        .prepare(&format!("PRAGMA table_info({})", table_name))
        .await?
        .query(libsql::params![])
        .await?;
    while let Some(row) = rows.next().await? {
        columns.push(ColumnInfo {
            name: row.get(1)?,
            data_type: row.get(2)?,
            is_nullable: row.get::<i32>(3)? == 0,
            default_value: row.get(4)?,
            is_primary_key: row.get::<i32>(5)? == 1,
        });
    }
    Ok(columns)
}

/// Update table schema if needed
pub async fn update_table_schema(conn: &Connection, table_schema: &TableSchema) -> Result<()> {
    let current_columns = get_table_columns(conn, &table_schema.name).await?;

    // Handle column renames: map old column names to new ones
    let mut column_rename_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if table_schema.name == "options" {
        // Map number_of_contracts -> total_quantity
        let has_number_of_contracts = current_columns
            .iter()
            .any(|c| c.name == "number_of_contracts");
        let has_total_quantity = current_columns.iter().any(|c| c.name == "total_quantity");
        let has_quantity = current_columns.iter().any(|c| c.name == "quantity");
        if has_number_of_contracts && !has_total_quantity {
            column_rename_map.insert(
                "number_of_contracts".to_string(),
                "total_quantity".to_string(),
            );
        } else if has_quantity && !has_total_quantity {
            // Also handle migration from quantity -> total_quantity
            column_rename_map.insert("quantity".to_string(), "total_quantity".to_string());
        }
    }

    // Add missing columns (skip if they're being renamed from an old column)
    for expected_col in &table_schema.columns {
        let is_renamed = column_rename_map
            .values()
            .any(|new_name| new_name == &expected_col.name);
        if !current_columns.iter().any(|c| c.name == expected_col.name) && !is_renamed {
            let mut alter_sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                table_schema.name, expected_col.name, expected_col.data_type
            );

            // For NOT NULL columns without explicit defaults, provide appropriate defaults
            if !expected_col.is_nullable {
                if let Some(default) = &expected_col.default_value {
                    alter_sql.push_str(&format!(" NOT NULL DEFAULT {}", default));
                } else {
                    // Provide default values for NOT NULL columns based on data type
                    match expected_col.data_type.to_uppercase().as_str() {
                        "TEXT" | "VARCHAR" => alter_sql.push_str(" NOT NULL DEFAULT ''"),
                        "INTEGER" => alter_sql.push_str(" NOT NULL DEFAULT 0"),
                        "REAL" | "DECIMAL" => alter_sql.push_str(" NOT NULL DEFAULT 0.0"),
                        "BOOLEAN" => alter_sql.push_str(" NOT NULL DEFAULT false"),
                        "DATE" => alter_sql.push_str(" NOT NULL DEFAULT '1970-01-01'"),
                        "TIME" => alter_sql.push_str(" NOT NULL DEFAULT '00:00:00'"),
                        _ => alter_sql.push_str(" NOT NULL DEFAULT ''"),
                    }
                }
            } else if let Some(default) = &expected_col.default_value {
                alter_sql.push_str(&format!(" DEFAULT {}", default));
            }

            conn.execute(&alter_sql, libsql::params![]).await?;
        }
    }

    // Remove columns that are not in the expected schema (excluding renamed columns)
    let expected_names: std::collections::HashSet<String> = table_schema
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let renamed_old_names: std::collections::HashSet<String> =
        column_rename_map.keys().cloned().collect();
    let columns_to_remove: Vec<String> = current_columns
        .iter()
        .filter(|c| {
            !expected_names.contains(&c.name)
                && !renamed_old_names.contains(&c.name)
                && !c.is_primary_key
        })
        .map(|c| c.name.clone())
        .collect();

    // Recreate table if we need to remove columns OR rename columns
    if !columns_to_remove.is_empty() || !column_rename_map.is_empty() {
        if !column_rename_map.is_empty() {
            log::info!(
                "Renaming columns in {}: {:?}",
                table_schema.name,
                column_rename_map
            );
        }
        if !columns_to_remove.is_empty() {
            log::info!(
                "Removing obsolete columns from {}: {:?}",
                table_schema.name,
                columns_to_remove
            );
        }

        // SQLite doesn't support DROP COLUMN or RENAME COLUMN directly, so we need to recreate the table
        // First, create a backup of existing data
        let backup_table = format!("{}_backup", table_schema.name);
        conn.execute(
            &format!(
                "CREATE TABLE {} AS SELECT * FROM {}",
                backup_table, table_schema.name
            ),
            libsql::params![],
        )
        .await?;

        // Drop the original table
        conn.execute(
            &format!("DROP TABLE {}", table_schema.name),
            libsql::params![],
        )
        .await?;

        // Recreate the table with the correct schema
        create_table(conn, table_schema).await?;

        // Copy data back, handling column renames
        let mut select_columns = Vec::new();
        let mut insert_columns = Vec::new();

        for current_col in &current_columns {
            if let Some(new_name) = column_rename_map.get(&current_col.name) {
                // This column was renamed
                if expected_names.contains(new_name) {
                    select_columns.push(current_col.name.clone());
                    insert_columns.push(new_name.clone());
                }
            } else if expected_names.contains(&current_col.name) {
                // Column exists in both schemas with same name
                select_columns.push(current_col.name.clone());
                insert_columns.push(current_col.name.clone());
            }
        }

        if !insert_columns.is_empty() {
            let select_str = select_columns.join(", ");
            let insert_str = insert_columns.join(", ");
            conn.execute(
                &format!(
                    "INSERT INTO {} ({}) SELECT {} FROM {}",
                    table_schema.name, insert_str, select_str, backup_table
                ),
                libsql::params![],
            )
            .await?;
        }

        // Drop the backup table
        conn.execute(&format!("DROP TABLE {}", backup_table), libsql::params![])
            .await?;

        // Recreate indexes and triggers
        ensure_indexes(conn, table_schema).await?;
        ensure_triggers(conn, table_schema).await?;
    }

    Ok(())
}
