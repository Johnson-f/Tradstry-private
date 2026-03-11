<!-- 57f831a4-e9ff-4ec9-a9b8-3b3aed5c649f b329312e-500e-44de-b4ca-63b5ae879b26 -->
# Notebook Backend Implementation Plan

## Overview

Build a Notion-style notebook system with unlimited nested notes, tagging, templates, reminders, and external calendar integration.

## 1. Code Refactoring - Extract Schema Functions

**File: `backend/src/turso/schema.rs` (new file)**

Create a new module to house schema-related functions currently in `client.rs`:

- Move `initialize_user_database_schema()` (lines 273-699)
- Move `get_current_schema_version()` (lines 775-781)
- Move `get_expected_schema()` (lines 784-1008)

Update imports and make these functions public methods that accept necessary parameters (db connection, etc.).

**File: `backend/src/turso/mod.rs`**

Add `pub mod schema;` to expose the new module.

**File: `backend/src/turso/client.rs`**

- Remove the three functions moved to `schema.rs`
- Import from `schema` module: `use super::schema::{initialize_user_database_schema, get_current_schema_version, get_expected_schema};`
- Update all function calls to use the imported functions

## 2. Database Schema Design

**File: `backend/database/07_notebook/01_notebook_notes_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS notebook_notes (
    id TEXT PRIMARY KEY,
    parent_id TEXT,
    title TEXT NOT NULL,
    content TEXT DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (parent_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notebook_notes_parent_id ON notebook_notes(parent_id);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_is_deleted ON notebook_notes(is_deleted);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_created_at ON notebook_notes(created_at);
CREATE INDEX IF NOT EXISTS idx_notebook_notes_position ON notebook_notes(parent_id, position);

CREATE TRIGGER IF NOT EXISTS update_notebook_notes_timestamp
AFTER UPDATE ON notebook_notes
FOR EACH ROW
BEGIN
    UPDATE notebook_notes SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

**File: `backend/database/07_notebook/02_notebook_tags_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS notebook_tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#gray',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS notebook_note_tags (
    note_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES notebook_tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_note_id ON notebook_note_tags(note_id);
CREATE INDEX IF NOT EXISTS idx_notebook_note_tags_tag_id ON notebook_note_tags(tag_id);
```

**File: `backend/database/07_notebook/03_notebook_templates_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS notebook_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notebook_templates_created_at ON notebook_templates(created_at);

CREATE TRIGGER IF NOT EXISTS update_notebook_templates_timestamp
AFTER UPDATE ON notebook_templates
FOR EACH ROW
BEGIN
    UPDATE notebook_templates SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

**File: `backend/database/07_notebook/04_notebook_reminders_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS notebook_reminders (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    reminder_time TEXT NOT NULL,
    is_completed BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (note_id) REFERENCES notebook_notes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notebook_reminders_note_id ON notebook_reminders(note_id);
CREATE INDEX IF NOT EXISTS idx_notebook_reminders_reminder_time ON notebook_reminders(reminder_time);
CREATE INDEX IF NOT EXISTS idx_notebook_reminders_is_completed ON notebook_reminders(is_completed);

CREATE TRIGGER IF NOT EXISTS update_notebook_reminders_timestamp
AFTER UPDATE ON notebook_reminders
FOR EACH ROW
BEGIN
    UPDATE notebook_reminders SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

**File: `backend/database/07_notebook/05_calendar_events_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS calendar_events (
    id TEXT PRIMARY KEY,
    reminder_id TEXT NOT NULL,
    event_title TEXT NOT NULL,
    event_description TEXT,
    event_time TEXT NOT NULL,
    is_synced BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (reminder_id) REFERENCES notebook_reminders(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_calendar_events_reminder_id ON calendar_events(reminder_id);
CREATE INDEX IF NOT EXISTS idx_calendar_events_event_time ON calendar_events(event_time);

CREATE TRIGGER IF NOT EXISTS update_calendar_events_timestamp
AFTER UPDATE ON calendar_events
FOR EACH ROW
BEGIN
    UPDATE calendar_events SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

**File: `backend/database/07_notebook/06_external_calendars_table.sql` (new)**

```sql
CREATE TABLE IF NOT EXISTS external_calendar_connections (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL CHECK (provider IN ('google', 'microsoft')),
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    token_expiry TEXT NOT NULL,
    calendar_id TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS external_calendar_events (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    external_event_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    location TEXT,
    last_synced_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (connection_id) REFERENCES external_calendar_connections(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_external_calendar_events_connection_id ON external_calendar_events(connection_id);
CREATE INDEX IF NOT EXISTS idx_external_calendar_events_start_time ON external_calendar_events(start_time);
```

## 3. Data Models

**File: `backend/src/models/notebook/mod.rs` (new)**

```rust
pub mod notebook_note;
pub mod tag;
pub mod template;
pub mod reminder;
pub mod calendar;

pub use notebook_note::*;
pub use tag::*;
pub use template::*;
pub use reminder::*;
pub use calendar::*;
```

**File: `backend/src/models/notebook/notebook_note.rs` (new)**

Define `NotebookNote`, `CreateNoteRequest`, `UpdateNoteRequest`, and CRUD operations following the pattern in `backend/src/models/notes/trade_notes.rs`. Include methods for:

- `create()` - Generate UUID, handle parent_id, position
- `find_by_id()`
- `find_all()` with filtering by parent_id
- `find_children()` - Get direct children of a note
- `find_tree()` - Get full nested tree from a root note
- `update()`
- `soft_delete()` - Set is_deleted = true
- `reorder()` - Update position within siblings

**File: `backend/src/models/notebook/tag.rs` (new)**

Define `NotebookTag`, `CreateTagRequest`, `UpdateTagRequest`, and operations:

- `create()`
- `find_by_id()`
- `find_all()`
- `update()`
- `delete()`
- `tag_note()` - Create association in notebook_note_tags
- `untag_note()` - Remove association
- `get_note_tags()` - Get all tags for a note
- `get_notes_by_tag()` - Get all notes with a specific tag

**File: `backend/src/models/notebook/template.rs` (new)**

Define `NotebookTemplate`, `CreateTemplateRequest`, `UpdateTemplateRequest`:

- `create()`
- `find_by_id()`
- `find_all()`
- `update()`
- `delete()`

**File: `backend/src/models/notebook/reminder.rs` (new)**

Define `NotebookReminder`, `CreateReminderRequest`, `UpdateReminderRequest`:

- `create()` - Also create corresponding calendar_event
- `find_by_id()`
- `find_by_note_id()`
- `find_upcoming()` - Get reminders within a time range
- `update()`
- `delete()` - Also delete calendar_event
- `mark_completed()`

**File: `backend/src/models/notebook/calendar.rs` (new)**

Define `CalendarEvent`, `ExternalCalendarConnection`, `ExternalCalendarEvent`:

- Calendar event CRUD (auto-created from reminders)
- External connection management (store OAuth tokens)
- External event sync methods

**File: `backend/src/models/mod.rs`**

Add: `pub mod notebook;`

## 4. API Routes

**File: `backend/src/routes/notebook.rs` (new)**

Create route handlers following the pattern in `backend/src/routes/notes.rs`:

### Notes endpoints:

- `POST /api/notebook/notes` - Create note
- `GET /api/notebook/notes` - List notes (with ?parent_id filter)
- `GET /api/notebook/notes/:id` - Get note
- `GET /api/notebook/notes/:id/tree` - Get note with full nested children
- `PUT /api/notebook/notes/:id` - Update note
- `DELETE /api/notebook/notes/:id` - Soft delete note
- `POST /api/notebook/notes/:id/reorder` - Reorder note position

### Tags endpoints:

- `POST /api/notebook/tags` - Create tag
- `GET /api/notebook/tags` - List all tags
- `GET /api/notebook/tags/:id` - Get tag
- `PUT /api/notebook/tags/:id` - Update tag
- `DELETE /api/notebook/tags/:id` - Delete tag
- `POST /api/notebook/notes/:note_id/tags/:tag_id` - Tag note
- `DELETE /api/notebook/notes/:note_id/tags/:tag_id` - Untag note

### Templates endpoints:

- `POST /api/notebook/templates` - Create template
- `GET /api/notebook/templates` - List templates
- `GET /api/notebook/templates/:id` - Get template
- `PUT /api/notebook/templates/:id` - Update template
- `DELETE /api/notebook/templates/:id` - Delete template

### Reminders endpoints:

- `POST /api/notebook/reminders` - Create reminder (auto-creates calendar event)
- `GET /api/notebook/reminders` - List reminders
- `GET /api/notebook/reminders/:id` - Get reminder
- `PUT /api/notebook/reminders/:id` - Update reminder
- `DELETE /api/notebook/reminders/:id` - Delete reminder
- `POST /api/notebook/reminders/:id/complete` - Mark completed

### Calendar endpoints:

- `GET /api/notebook/calendar/events` - Get all calendar events (internal + external)
- `POST /api/notebook/calendar/connect/google` - OAuth callback for Google
- `POST /api/notebook/calendar/connect/microsoft` - OAuth callback for Microsoft
- `DELETE /api/notebook/calendar/connections/:id` - Disconnect calendar
- `POST /api/notebook/calendar/sync` - Manually trigger external calendar sync

**File: `backend/src/routes/mod.rs`**

Add: `pub mod notebook;` and register routes in main.rs

## 5. Update Schema Initialization

**File: `backend/src/turso/schema.rs`**

Update `initialize_user_database_schema()` to include all notebook tables (notes, tags, templates, reminders, calendar events, external calendars).

Update `get_current_schema_version()` to bump version to "0.0.7" with description including notebook feature.

Update `get_expected_schema()` to include all new TableSchema definitions for the 6 new tables.

## 6. External Calendar Integration

**File: `backend/src/service/calendar_service.rs` (new)**

Implement OAuth2 flow and API integration:

- Google Calendar API integration using `google-calendar3` crate
- Microsoft Graph API integration using `graph-rs-sdk` crate
- Token refresh logic
- Event fetching and caching
- Sync scheduler (background task)

Add dependencies to `backend/Cargo.toml`:

```toml
google-calendar3 = "5.0"
graph-rs-sdk = "0.2"
oauth2 = "4.4"
```

**File: `backend/src/service/mod.rs`**

Add: `pub mod calendar_service;`

## 7. Main Application Updates

**File: `backend/src/main.rs`**

Register notebook routes in the ActixWeb app configuration.

## Implementation Order

1. Refactor schema functions to `schema.rs`
2. Create database schema SQL files
3. Create data models (notes → tags → templates → reminders → calendar)
4. Update schema initialization in `schema.rs`
5. Implement route handlers
6. Implement calendar service for external integration
7. Test all endpoints

### To-dos

- [ ] Extract schema functions from client.rs to new schema.rs module
- [ ] Create all 6 SQL schema files for notebook tables
- [ ] Implement all notebook models (notes, tags, templates, reminders, calendar)
- [ ] Update schema initialization to include notebook tables and bump version
- [ ] Create notebook.rs routes with all CRUD endpoints
- [ ] Implement external calendar OAuth and sync service
- [ ] Register notebook routes in main.rs