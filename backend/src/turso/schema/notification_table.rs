use super::{ManagedColumn, ManagedTable};

pub(super) const ALERT_RULES: ManagedTable = ManagedTable {
    name: "alert_rules",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "alert_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "symbol", definition: "TEXT" },
        ManagedColumn { name: "operator", definition: "TEXT" },
        ManagedColumn { name: "threshold", definition: "REAL" },
        ManagedColumn { name: "channels", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "cooldown_seconds", definition: "INTEGER" },
        ManagedColumn { name: "last_fired_at", definition: "TEXT" },
        ManagedColumn { name: "note", definition: "TEXT" },
        ManagedColumn { name: "max_triggers", definition: "INTEGER" },
        ManagedColumn { name: "fired_count", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "is_active", definition: "BOOLEAN NOT NULL DEFAULT 1" },
        ManagedColumn { name: "metadata", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_symbol ON alert_rules(symbol);",
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_created_at ON alert_rules(created_at);",
        "CREATE INDEX IF NOT EXISTS idx_alert_rules_is_active ON alert_rules(is_active);",
    ],
};

pub(super) const PUSH_SUBSCRIPTIONS: ManagedTable = ManagedTable {
    name: "push_subscriptions",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "endpoint", definition: "TEXT NOT NULL UNIQUE" },
        ManagedColumn { name: "p256dh", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "auth", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "ua", definition: "TEXT" },
        ManagedColumn { name: "topics", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user_id ON push_subscriptions(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_push_subscriptions_created_at ON push_subscriptions(created_at);",
    ],
};

pub(super) const NOTIFICATIONS: ManagedTable = ManagedTable {
    name: "notifications",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "notification_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "body", definition: "TEXT" },
        ManagedColumn { name: "data", definition: "TEXT" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'template'" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type);",
        "CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);",
        "CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);",
        r#"CREATE TRIGGER IF NOT EXISTS update_notifications_timestamp
AFTER UPDATE ON notifications
FOR EACH ROW
BEGIN
    UPDATE notifications SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTIFICATION_DISPATCH_LOG: ManagedTable = ManagedTable {
    name: "notification_dispatch_log",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS notification_dispatch_log (
    id TEXT PRIMARY KEY,
    notification_id TEXT,
    notification_type TEXT NOT NULL,
    slot_key TEXT NOT NULL UNIQUE,
    sent_count INTEGER DEFAULT 0,
    failed_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "notification_id", definition: "TEXT" },
        ManagedColumn { name: "notification_type", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "slot_key", definition: "TEXT NOT NULL UNIQUE" },
        ManagedColumn { name: "sent_count", definition: "INTEGER DEFAULT 0" },
        ManagedColumn { name: "failed_count", definition: "INTEGER DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notification_dispatch_type ON notification_dispatch_log(notification_type);",
    ],
};

pub(super) const PUBLIC_NOTES_REGISTRY: ManagedTable = ManagedTable {
    name: "public_notes_registry",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS public_notes_registry (
    slug TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'Untitled',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#,
    columns: &[
        ManagedColumn { name: "slug", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "owner_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL DEFAULT 'Untitled'" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_public_notes_owner ON public_notes_registry(owner_id);",
    ],
};

pub(super) const INVITATIONS_REGISTRY: ManagedTable = ManagedTable {
    name: "invitations_registry",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS invitations_registry (
    token TEXT PRIMARY KEY,
    inviter_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    invitee_email TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#,
    columns: &[
        ManagedColumn { name: "token", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "inviter_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "invitee_email", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_invitations_inviter ON invitations_registry(inviter_id);",
        "CREATE INDEX IF NOT EXISTS idx_invitations_invitee ON invitations_registry(invitee_email);",
    ],
};

pub(super) const COLLABORATORS_REGISTRY: ManagedTable = ManagedTable {
    name: "collaborators_registry",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS collaborators_registry (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    collaborator_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'editor',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(note_id, collaborator_id)
);
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "owner_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "collaborator_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "role", definition: "TEXT NOT NULL DEFAULT 'editor'" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_collaborators_note ON collaborators_registry(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_collaborators_owner ON collaborators_registry(owner_id);",
        "CREATE INDEX IF NOT EXISTS idx_collaborators_user ON collaborators_registry(collaborator_id);",
    ],
};
