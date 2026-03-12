use super::{ManagedColumn, ManagedTable};

pub(super) const CHAT_SESSIONS: ManagedTable = ManagedTable {
    name: "chat_sessions",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    title TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    last_message_at TEXT
);
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "message_count", definition: "INTEGER DEFAULT 0" },
        ManagedColumn { name: "last_message_at", definition: "TEXT" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_chat_sessions_user_id ON chat_sessions(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_at ON chat_sessions(updated_at);",
    ],
};

pub(super) const CHAT_MESSAGES: ManagedTable = ManagedTable {
    name: "chat_messages",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "session_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "role", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "content", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "context_vectors", definition: "TEXT" },
        ManagedColumn { name: "token_count", definition: "INTEGER" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_user_id ON chat_messages(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at);",
    ],
};

pub(super) const AI_REPORTS: ManagedTable = ManagedTable {
    name: "ai_reports",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "time_range", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "analytics", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "trades", definition: "TEXT" },
        ManagedColumn { name: "recommendations", definition: "TEXT" },
        ManagedColumn { name: "metrics", definition: "TEXT" },
        ManagedColumn { name: "metadata", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "summary", definition: "TEXT DEFAULT ''" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_ai_reports_user_id ON ai_reports(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_ai_reports_time_range ON ai_reports(time_range);",
        "CREATE INDEX IF NOT EXISTS idx_ai_reports_created_at ON ai_reports(created_at);",
    ],
};

pub(super) const AI_ANALYSIS: ManagedTable = ManagedTable {
    name: "ai_analysis",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS ai_analysis (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    time_range TEXT NOT NULL CHECK (time_range IN ('7d', '30d', '90d', 'ytd', '1y')),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "time_range", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "content", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_ai_analysis_user_id ON ai_analysis(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_ai_analysis_time_range ON ai_analysis(time_range);",
        "CREATE INDEX IF NOT EXISTS idx_ai_analysis_created_at ON ai_analysis(created_at);",
    ],
};
