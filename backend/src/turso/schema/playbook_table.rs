use super::{ManagedColumn, ManagedTable};

pub(super) const PLAYBOOK: ManagedTable = ManagedTable {
    name: "playbook",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "description", definition: "TEXT" },
        ManagedColumn { name: "icon", definition: "TEXT" },
        ManagedColumn { name: "emoji", definition: "TEXT" },
        ManagedColumn { name: "color", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "version", definition: "INTEGER NOT NULL DEFAULT 0" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_playbook_user_id ON playbook(user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_playbook_timestamp
AFTER UPDATE ON playbook
FOR EACH ROW
BEGIN
    UPDATE playbook SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const STOCK_TRADE_PLAYBOOK: ManagedTable = ManagedTable {
    name: "stock_trade_playbook",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS stock_trade_playbook (
    stock_trade_id INTEGER NOT NULL,
    setup_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (stock_trade_id, setup_id),
    FOREIGN KEY (stock_trade_id) REFERENCES stocks(id) ON DELETE CASCADE,
    FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE
);
"#,
    columns: &[
        ManagedColumn { name: "stock_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "setup_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_playbook_user_id ON stock_trade_playbook(user_id);",
    ],
};

pub(super) const OPTION_TRADE_PLAYBOOK: ManagedTable = ManagedTable {
    name: "option_trade_playbook",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS option_trade_playbook (
    option_trade_id INTEGER NOT NULL,
    setup_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (option_trade_id, setup_id),
    FOREIGN KEY (option_trade_id) REFERENCES options(id) ON DELETE CASCADE,
    FOREIGN KEY (setup_id) REFERENCES playbook(id) ON DELETE CASCADE
);
"#,
    columns: &[
        ManagedColumn { name: "option_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "setup_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_option_trade_playbook_user_id ON option_trade_playbook(user_id);",
    ],
};

pub(super) const PLAYBOOK_RULES: ManagedTable = ManagedTable {
    name: "playbook_rules",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "playbook_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "rule_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "description", definition: "TEXT" },
        ManagedColumn { name: "order_position", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_playbook_rules_user_id ON playbook_rules(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_playbook_rules_playbook_id ON playbook_rules(playbook_id);",
        "CREATE INDEX IF NOT EXISTS idx_playbook_rules_type ON playbook_rules(rule_type);",
    ],
};

pub(super) const STOCK_TRADE_RULE_COMPLIANCE: ManagedTable = ManagedTable {
    name: "stock_trade_rule_compliance",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "stock_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "playbook_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "rule_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "is_followed", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "notes", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_stock_trade_rule_compliance_user_id ON stock_trade_rule_compliance(user_id);",
    ],
};

pub(super) const OPTION_TRADE_RULE_COMPLIANCE: ManagedTable = ManagedTable {
    name: "option_trade_rule_compliance",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "option_trade_id", definition: "INTEGER NOT NULL" },
        ManagedColumn { name: "playbook_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "rule_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "is_followed", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "notes", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_option_trade_rule_compliance_user_id ON option_trade_rule_compliance(user_id);",
    ],
};
