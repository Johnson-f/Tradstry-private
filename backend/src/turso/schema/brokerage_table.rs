use super::{ManagedColumn, ManagedTable};

pub(super) const BROKERAGE_CONNECTIONS: ManagedTable = ManagedTable {
    name: "brokerage_connections",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "snaptrade_user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "snaptrade_user_secret", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "connection_id", definition: "TEXT" },
        ManagedColumn { name: "brokerage_name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'pending'" },
        ManagedColumn { name: "last_sync_at", definition: "TIMESTAMP" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_brokerage_connections_user_id ON brokerage_connections(user_id);",
    ],
};

pub(super) const BROKERAGE_ACCOUNTS: ManagedTable = ManagedTable {
    name: "brokerage_accounts",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "connection_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "snaptrade_account_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "account_number", definition: "TEXT" },
        ManagedColumn { name: "account_name", definition: "TEXT" },
        ManagedColumn { name: "account_type", definition: "TEXT" },
        ManagedColumn { name: "balance", definition: "REAL" },
        ManagedColumn { name: "currency", definition: "TEXT DEFAULT 'USD'" },
        ManagedColumn { name: "institution_name", definition: "TEXT" },
        ManagedColumn { name: "raw_data", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_brokerage_accounts_user_id ON brokerage_accounts(user_id);",
    ],
};

pub(super) const BROKERAGE_TRANSACTIONS: ManagedTable = ManagedTable {
    name: "brokerage_transactions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "account_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "snaptrade_transaction_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT" },
        ManagedColumn { name: "transaction_type", definition: "TEXT" },
        ManagedColumn { name: "quantity", definition: "REAL" },
        ManagedColumn { name: "price", definition: "REAL" },
        ManagedColumn { name: "amount", definition: "REAL" },
        ManagedColumn { name: "currency", definition: "TEXT DEFAULT 'USD'" },
        ManagedColumn { name: "trade_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "settlement_date", definition: "TIMESTAMP" },
        ManagedColumn { name: "fees", definition: "REAL" },
        ManagedColumn { name: "raw_data", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_brokerage_transactions_user_id ON brokerage_transactions(user_id);",
    ],
};

pub(super) const BROKERAGE_HOLDINGS: ManagedTable = ManagedTable {
    name: "brokerage_holdings",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "account_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "quantity", definition: "REAL NOT NULL" },
        ManagedColumn { name: "average_cost", definition: "REAL" },
        ManagedColumn { name: "current_price", definition: "REAL" },
        ManagedColumn { name: "market_value", definition: "REAL" },
        ManagedColumn { name: "currency", definition: "TEXT DEFAULT 'USD'" },
        ManagedColumn { name: "last_updated", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "raw_data", definition: "TEXT" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_brokerage_holdings_user_id ON brokerage_holdings(user_id);",
    ],
};

pub(super) const UNMATCHED_TRANSACTIONS: ManagedTable = ManagedTable {
    name: "unmatched_transactions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "transaction_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "snaptrade_transaction_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "trade_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "units", definition: "REAL NOT NULL" },
        ManagedColumn { name: "price", definition: "REAL NOT NULL" },
        ManagedColumn { name: "fee", definition: "REAL NOT NULL" },
        ManagedColumn { name: "trade_date", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "brokerage_name", definition: "TEXT" },
        ManagedColumn { name: "raw_data", definition: "TEXT" },
        ManagedColumn { name: "is_option", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "difficulty_reason", definition: "TEXT" },
        ManagedColumn { name: "confidence_score", definition: "REAL" },
        ManagedColumn { name: "suggested_matches", definition: "TEXT" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'pending'" },
        ManagedColumn { name: "resolved_trade_id", definition: "INTEGER" },
        ManagedColumn { name: "resolved_at", definition: "TIMESTAMP" },
        ManagedColumn { name: "created_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
        ManagedColumn { name: "updated_at", definition: "TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_unmatched_transactions_user_id ON unmatched_transactions(user_id);",
    ],
};
