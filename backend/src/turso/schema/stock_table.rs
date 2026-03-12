use super::{ManagedColumn, ManagedTable};

pub(super) const STOCKS: ManagedTable = ManagedTable {
    name: "stocks",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "INTEGER PRIMARY KEY AUTOINCREMENT" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "trade_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "order_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "entry_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "exit_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "stop_loss", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "commissions", definition: "DECIMAL(10,4) NOT NULL DEFAULT 0.00" },
        ManagedColumn { name: "number_shares", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "take_profit", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "initial_target", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "profit_target", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "trade_ratings", definition: "INTEGER" },
        ManagedColumn { name: "entry_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "exit_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "reviewed", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "mistakes", definition: "TEXT" },
        ManagedColumn { name: "brokerage_name", definition: "TEXT" },
        ManagedColumn { name: "trade_group_id", definition: "TEXT" },
        ManagedColumn { name: "parent_trade_id", definition: "INTEGER" },
        ManagedColumn { name: "total_quantity", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "transaction_sequence", definition: "INTEGER" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "is_deleted", definition: "INTEGER NOT NULL DEFAULT 0" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_stocks_user_id ON stocks(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_trade_type ON stocks(trade_type);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_entry_date ON stocks(entry_date);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_exit_date ON stocks(exit_date);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_entry_price ON stocks(entry_price);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_exit_price ON stocks(exit_price);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_brokerage_name ON stocks(brokerage_name);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_trade_group_id ON stocks(trade_group_id);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_parent_trade_id ON stocks(parent_trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_stocks_is_deleted ON stocks(is_deleted);",
        r#"CREATE TRIGGER IF NOT EXISTS update_stocks_timestamp
AFTER UPDATE ON stocks
FOR EACH ROW
BEGIN
    UPDATE stocks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const USER_PROFILE: ManagedTable = ManagedTable {
    name: "user_profile",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "INTEGER PRIMARY KEY AUTOINCREMENT" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL UNIQUE" },
        ManagedColumn { name: "display_name", definition: "TEXT" },
        ManagedColumn { name: "nickname", definition: "TEXT" },
        ManagedColumn { name: "timezone", definition: "TEXT DEFAULT 'UTC'" },
        ManagedColumn { name: "currency", definition: "TEXT DEFAULT 'USD'" },
        ManagedColumn { name: "profile_picture_uuid", definition: "TEXT" },
        ManagedColumn { name: "trading_experience_level", definition: "TEXT" },
        ManagedColumn { name: "primary_trading_goal", definition: "TEXT" },
        ManagedColumn { name: "asset_types", definition: "TEXT" },
        ManagedColumn { name: "trading_style", definition: "TEXT" },
        ManagedColumn { name: "storage_used_bytes", definition: "INTEGER DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_user_profile_user_id ON user_profile(user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_user_profile_timestamp
AFTER UPDATE ON user_profile
FOR EACH ROW
BEGIN
    UPDATE user_profile SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const STOCK_POSITIONS: ManagedTable = ManagedTable {
    name: "stock_positions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "total_quantity", definition: "DECIMAL(15,8) NOT NULL DEFAULT 0.0" },
        ManagedColumn { name: "average_entry_price", definition: "DECIMAL(15,8) NOT NULL DEFAULT 0.0" },
        ManagedColumn { name: "current_price", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "unrealized_pnl", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "realized_pnl", definition: "DECIMAL(15,8) NOT NULL DEFAULT 0.0" },
        ManagedColumn { name: "brokerage_name", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_stock_positions_user_id ON stock_positions(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_stock_positions_symbol ON stock_positions(symbol);",
        "CREATE INDEX IF NOT EXISTS idx_stock_positions_brokerage_name ON stock_positions(brokerage_name);",
    ],
};

pub(super) const POSITION_TRANSACTIONS: ManagedTable = ManagedTable {
    name: "position_transactions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "position_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "position_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "trade_id", definition: "INTEGER" },
        ManagedColumn { name: "transaction_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "quantity", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "transaction_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_user_id ON position_transactions(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_position_id ON position_transactions(position_id);",
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_position_type ON position_transactions(position_type);",
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_trade_id ON position_transactions(trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_type ON position_transactions(transaction_type);",
        "CREATE INDEX IF NOT EXISTS idx_position_transactions_transaction_date ON position_transactions(transaction_date);",
    ],
};

pub(super) const TRADE_NOTES: ManagedTable = ManagedTable {
    name: "trade_notes",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "content", definition: "TEXT DEFAULT ''" },
        ManagedColumn { name: "trade_type", definition: "TEXT" },
        ManagedColumn { name: "stock_trade_id", definition: "INTEGER" },
        ManagedColumn { name: "option_trade_id", definition: "INTEGER" },
        ManagedColumn { name: "ai_metadata", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_user_id ON trade_notes(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_updated_at ON trade_notes(updated_at);",
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_stock_trade_id ON trade_notes(stock_trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_trade_notes_option_trade_id ON trade_notes(option_trade_id);",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_stock_unique ON trade_notes(stock_trade_id) WHERE stock_trade_id IS NOT NULL;",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_trade_notes_option_unique ON trade_notes(option_trade_id) WHERE option_trade_id IS NOT NULL;",
        r#"CREATE TRIGGER IF NOT EXISTS update_trade_notes_timestamp
AFTER UPDATE ON trade_notes
FOR EACH ROW
BEGIN
    UPDATE trade_notes SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const TRADE_TAGS: ManagedTable = ManagedTable {
    name: "trade_tags",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "category", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "color", definition: "TEXT" },
        ManagedColumn { name: "description", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_trade_tags_user_id ON trade_tags(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_trade_tags_category ON trade_tags(category);",
        "CREATE INDEX IF NOT EXISTS idx_trade_tags_name ON trade_tags(name);",
        r#"CREATE TRIGGER IF NOT EXISTS update_trade_tags_timestamp
AFTER UPDATE ON trade_tags
FOR EACH ROW
BEGIN
    UPDATE trade_tags SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const STOCK_TRADE_TAGS: ManagedTable = ManagedTable {
    name: "stock_trade_tags",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS stock_trade_tags (
    stock_trade_id INTEGER NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (stock_trade_id, tag_id),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
);
"#,
    columns: &[
        ManagedColumn { name: "stock_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "tag_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_user_id ON stock_trade_tags(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_stock_id ON stock_trade_tags(stock_trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_tags_tag_id ON stock_trade_tags(tag_id);",
    ],
};

pub(super) const MISSED_TRADES: ManagedTable = ManagedTable {
    name: "missed_trades",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "playbook_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "trade_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "reason", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "potential_entry_price", definition: "REAL" },
        ManagedColumn { name: "opportunity_date", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "notes", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_missed_trades_user_id ON missed_trades(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_missed_trades_playbook_id ON missed_trades(playbook_id);",
        "CREATE INDEX IF NOT EXISTS idx_missed_trades_opportunity_date ON missed_trades(opportunity_date);",
    ],
};
