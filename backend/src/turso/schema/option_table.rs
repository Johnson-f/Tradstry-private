use super::{ManagedColumn, ManagedTable};

pub(super) const OPTIONS: ManagedTable = ManagedTable {
    name: "options",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "INTEGER PRIMARY KEY AUTOINCREMENT" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "option_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "strike_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "expiration_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "entry_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "exit_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "premium", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "commissions", definition: "DECIMAL(10,4) NOT NULL DEFAULT 0.00" },
        ManagedColumn { name: "entry_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "exit_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'open'" },
        ManagedColumn { name: "initial_target", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "profit_target", definition: "DECIMAL(15,8)" },
        ManagedColumn { name: "trade_ratings", definition: "INTEGER" },
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
        "CREATE INDEX IF NOT EXISTS idx_options_user_id ON options(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_options_symbol ON options(symbol);",
        "CREATE INDEX IF NOT EXISTS idx_options_option_type ON options(option_type);",
        "CREATE INDEX IF NOT EXISTS idx_options_status ON options(status);",
        "CREATE INDEX IF NOT EXISTS idx_options_entry_date ON options(entry_date);",
        "CREATE INDEX IF NOT EXISTS idx_options_exit_date ON options(exit_date);",
        "CREATE INDEX IF NOT EXISTS idx_options_expiration_date ON options(expiration_date);",
        "CREATE INDEX IF NOT EXISTS idx_options_strike_price ON options(strike_price);",
        "CREATE INDEX IF NOT EXISTS idx_options_entry_price ON options(entry_price);",
        "CREATE INDEX IF NOT EXISTS idx_options_exit_price ON options(exit_price);",
        "CREATE INDEX IF NOT EXISTS idx_options_brokerage_name ON options(brokerage_name);",
        "CREATE INDEX IF NOT EXISTS idx_options_trade_group_id ON options(trade_group_id);",
        "CREATE INDEX IF NOT EXISTS idx_options_parent_trade_id ON options(parent_trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_options_is_deleted ON options(is_deleted);",
        r#"CREATE TRIGGER IF NOT EXISTS update_options_timestamp
AFTER UPDATE ON options
FOR EACH ROW
BEGIN
    UPDATE options SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const OPTION_POSITIONS: ManagedTable = ManagedTable {
    name: "option_positions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "option_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "strike_price", definition: "DECIMAL(15,8) NOT NULL" },
        ManagedColumn { name: "expiration_date", definition: "TIMESTAMP NOT NULL" },
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
        "CREATE INDEX IF NOT EXISTS idx_option_positions_user_id ON option_positions(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_option_positions_symbol ON option_positions(symbol);",
        "CREATE INDEX IF NOT EXISTS idx_option_positions_strike_exp ON option_positions(symbol, strike_price, expiration_date, option_type);",
        "CREATE INDEX IF NOT EXISTS idx_option_positions_brokerage_name ON option_positions(brokerage_name);",
    ],
};

pub(super) const OPTION_TRADE_TAGS: ManagedTable = ManagedTable {
    name: "option_trade_tags",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS option_trade_tags (
    option_trade_id INTEGER NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (option_trade_id, tag_id),
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES trade_tags(id) ON DELETE CASCADE
);
"#,
    columns: &[
        ManagedColumn { name: "option_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "tag_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_option_trade_tags_user_id ON option_trade_tags(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_option_trade_tags_option_id ON option_trade_tags(option_trade_id);",
        "CREATE INDEX IF NOT EXISTS idx_option_trade_tags_tag_id ON option_trade_tags(tag_id);",
    ],
};
