use super::{ManagedColumn, ManagedTable};

pub(super) const CALENDAR_CONNECTIONS: ManagedTable = ManagedTable {
    name: "calendar_connections",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "provider", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "access_token", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "refresh_token", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "token_expires_at", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "account_email", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "account_name", definition: "TEXT" },
        ManagedColumn { name: "enabled", definition: "BOOLEAN NOT NULL DEFAULT true" },
        ManagedColumn { name: "last_sync_at", definition: "TIMESTAMP" },
        ManagedColumn { name: "sync_error", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_calendar_connections_user_id ON calendar_connections(user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_calendar_connections_timestamp
AFTER UPDATE ON calendar_connections
FOR EACH ROW
BEGIN
    UPDATE calendar_connections SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const CALENDAR_SYNC_CURSORS: ManagedTable = ManagedTable {
    name: "calendar_sync_cursors",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "connection_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "external_calendar_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "external_calendar_name", definition: "TEXT" },
        ManagedColumn { name: "sync_token", definition: "TEXT" },
        ManagedColumn { name: "last_sync_at", definition: "TIMESTAMP" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_calendar_sync_cursors_user_id ON calendar_sync_cursors(user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_calendar_sync_cursors_timestamp
AFTER UPDATE ON calendar_sync_cursors
FOR EACH ROW
BEGIN
    UPDATE calendar_sync_cursors SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const CALENDAR_EVENTS: ManagedTable = ManagedTable {
    name: "calendar_events",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "description", definition: "TEXT" },
        ManagedColumn { name: "start_time", definition: "TIMESTAMP NOT NULL" },
        ManagedColumn { name: "end_time", definition: "TIMESTAMP" },
        ManagedColumn { name: "all_day", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "reminder_minutes", definition: "INTEGER" },
        ManagedColumn { name: "recurrence_rule", definition: "TEXT" },
        ManagedColumn { name: "source", definition: "TEXT NOT NULL DEFAULT 'local'" },
        ManagedColumn { name: "external_id", definition: "TEXT" },
        ManagedColumn { name: "external_calendar_id", definition: "TEXT" },
        ManagedColumn { name: "connection_id", definition: "TEXT" },
        ManagedColumn { name: "color", definition: "TEXT" },
        ManagedColumn { name: "location", definition: "TEXT" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'confirmed'" },
        ManagedColumn { name: "is_reminder", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "reminder_sent", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "metadata", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "synced_at", definition: "TIMESTAMP" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_calendar_events_user_id ON calendar_events(user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_calendar_events_timestamp
AFTER UPDATE ON calendar_events
FOR EACH ROW
BEGIN
    UPDATE calendar_events SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};
