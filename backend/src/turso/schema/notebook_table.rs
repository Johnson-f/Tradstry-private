use super::{ManagedColumn, ManagedTable};

pub(super) const NOTEBOOK_FOLDERS: ManagedTable = ManagedTable {
    name: "notebook_folders",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "is_favorite", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "parent_folder_id", definition: "TEXT" },
        ManagedColumn { name: "position", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_user_id ON notebook_folders(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_name ON notebook_folders(name);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_is_favorite ON notebook_folders(is_favorite);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_folders_parent_folder_id ON notebook_folders(parent_folder_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_notebook_folders_timestamp
AFTER UPDATE ON notebook_folders
FOR EACH ROW
BEGIN
    UPDATE notebook_folders SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTEBOOK_NOTES: ManagedTable = ManagedTable {
    name: "notebook_notes",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "title", definition: "TEXT NOT NULL DEFAULT 'Untitled'" },
        ManagedColumn { name: "content", definition: "TEXT NOT NULL DEFAULT '{}'" },
        ManagedColumn { name: "content_plain_text", definition: "TEXT" },
        ManagedColumn { name: "word_count", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "is_pinned", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "is_archived", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "is_deleted", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "folder_id", definition: "TEXT" },
        ManagedColumn { name: "tags", definition: "TEXT" },
        ManagedColumn { name: "y_state", definition: "BLOB" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "last_synced_at", definition: "TEXT" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_user_id ON notebook_notes(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_title ON notebook_notes(title);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_pinned ON notebook_notes(is_pinned);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_archived ON notebook_notes(is_archived);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_deleted ON notebook_notes(is_deleted);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_folder_id ON notebook_notes(folder_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_created_at ON notebook_notes(created_at);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_notes_updated_at ON notebook_notes(updated_at);",
        r#"CREATE TRIGGER IF NOT EXISTS update_notebook_notes_timestamp
AFTER UPDATE ON notebook_notes
FOR EACH ROW
BEGIN
    UPDATE notebook_notes SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTEBOOK_TAGS: ManagedTable = ManagedTable {
    name: "notebook_tags",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "name", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "color", definition: "TEXT" },
        ManagedColumn { name: "is_favorite", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notebook_tags_user_id ON notebook_tags(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_tags_name ON notebook_tags(name);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_tags_is_favorite ON notebook_tags(is_favorite);",
        r#"CREATE TRIGGER IF NOT EXISTS update_notebook_tags_timestamp
AFTER UPDATE ON notebook_tags
FOR EACH ROW
BEGIN
    UPDATE notebook_tags SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTEBOOK_NOTE_TAGS: ManagedTable = ManagedTable {
    name: "notebook_note_tags",
    create_sql: r#"
CREATE TABLE IF NOT EXISTS notebook_note_tags (
    note_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES notebook_tags(id) ON DELETE CASCADE
);
"#,
    columns: &[
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "tag_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_user_id ON notebook_note_tags(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_note_id ON notebook_note_tags(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_tag_id ON notebook_note_tags(tag_id);",
    ],
};

pub(super) const NOTEBOOK_IMAGES: ManagedTable = ManagedTable {
    name: "notebook_images",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "src", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "storage_path", definition: "TEXT" },
        ManagedColumn { name: "alt_text", definition: "TEXT" },
        ManagedColumn { name: "caption", definition: "TEXT" },
        ManagedColumn { name: "width", definition: "INTEGER" },
        ManagedColumn { name: "height", definition: "INTEGER" },
        ManagedColumn { name: "position", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_notebook_images_user_id ON notebook_images(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_images_note_id ON notebook_images(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_notebook_images_created_at ON notebook_images(created_at);",
        r#"CREATE TRIGGER IF NOT EXISTS update_notebook_images_timestamp
AFTER UPDATE ON notebook_images
FOR EACH ROW
BEGIN
    UPDATE notebook_images SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTE_COLLABORATORS: ManagedTable = ManagedTable {
    name: "note_collaborators",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_email", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "user_name", definition: "TEXT" },
        ManagedColumn { name: "role", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "invited_by", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "joined_at", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "last_seen_at", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_id ON note_collaborators(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_note_id ON note_collaborators(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_collaborators_user_email ON note_collaborators(user_email);",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_collaborators_note_user ON note_collaborators(note_id, user_id);",
        r#"CREATE TRIGGER IF NOT EXISTS update_note_collaborators_timestamp
AFTER UPDATE ON note_collaborators
FOR EACH ROW
BEGIN
    UPDATE note_collaborators SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTE_INVITATIONS: ManagedTable = ManagedTable {
    name: "note_invitations",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "inviter_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "inviter_email", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "invitee_email", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "invitee_user_id", definition: "TEXT" },
        ManagedColumn { name: "role", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "token", definition: "TEXT NOT NULL UNIQUE" },
        ManagedColumn { name: "status", definition: "TEXT NOT NULL DEFAULT 'pending'" },
        ManagedColumn { name: "message", definition: "TEXT" },
        ManagedColumn { name: "expires_at", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "accepted_at", definition: "TEXT" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_user_id ON note_invitations(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_note_id ON note_invitations(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_invitee_email ON note_invitations(invitee_email);",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_invitations_token ON note_invitations(token);",
        "CREATE INDEX IF NOT EXISTS idx_note_invitations_status ON note_invitations(status);",
        r#"CREATE TRIGGER IF NOT EXISTS update_note_invitations_timestamp
AFTER UPDATE ON note_invitations
FOR EACH ROW
BEGIN
    UPDATE note_invitations SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};

pub(super) const NOTE_SHARE_SETTINGS: ManagedTable = ManagedTable {
    name: "note_share_settings",
    create_sql: r#"
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
"#,
    columns: &[
        ManagedColumn { name: "id", definition: "TEXT PRIMARY KEY" },
        ManagedColumn { name: "user_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "note_id", definition: "TEXT NOT NULL UNIQUE" },
        ManagedColumn { name: "owner_id", definition: "TEXT NOT NULL" },
        ManagedColumn { name: "visibility", definition: "TEXT NOT NULL DEFAULT 'private'" },
        ManagedColumn { name: "public_slug", definition: "TEXT UNIQUE" },
        ManagedColumn { name: "allow_comments", definition: "BOOLEAN NOT NULL DEFAULT false" },
        ManagedColumn { name: "allow_copy", definition: "BOOLEAN NOT NULL DEFAULT true" },
        ManagedColumn { name: "password_hash", definition: "TEXT" },
        ManagedColumn { name: "view_count", definition: "INTEGER NOT NULL DEFAULT 0" },
        ManagedColumn { name: "created_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
        ManagedColumn { name: "updated_at", definition: "TEXT NOT NULL DEFAULT (datetime('now'))" },
    ],
    column_renames: &[],
    maintenance_sql_hooks: &[
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_note_id ON note_share_settings(note_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_user_id ON note_share_settings(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_owner_id ON note_share_settings(owner_id);",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_note_share_settings_public_slug ON note_share_settings(public_slug);",
        "CREATE INDEX IF NOT EXISTS idx_note_share_settings_visibility ON note_share_settings(visibility);",
        r#"CREATE TRIGGER IF NOT EXISTS update_note_share_settings_timestamp
AFTER UPDATE ON note_share_settings
FOR EACH ROW
BEGIN
    UPDATE note_share_settings SET updated_at = datetime('now') WHERE id = NEW.id;
END;"#,
    ],
};
