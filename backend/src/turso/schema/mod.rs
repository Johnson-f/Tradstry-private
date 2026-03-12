use std::collections::HashSet;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use libsql::Connection;
use log::info;

mod ai_table;
mod brokerage_table;
mod calendar_table;
mod notebook_table;
mod notification_table;
mod option_table;
mod playbook_table;
mod stock_table;
mod tables;

const SCHEMA_VERSION_TABLE: &str = "schema_version";

const CREATE_SCHEMA_VERSION_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_version TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"#;

const CREATE_SCHEMA_VERSION_TABLE_INDEX_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_schema_version_applied_at
ON schema_version (applied_at, id);
"#;

/// Single schema version knob.
///
/// Bump this value when managed table definitions or rename rules change.
const TARGET_SCHEMA_VERSION: &str = "0.1";


// ─── Public types ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct ManagedColumn {
    pub name: &'static str,
    pub definition: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnRenameRule {
    pub from: &'static str,
    pub to: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ManagedTable {
    pub name: &'static str,
    pub create_sql: &'static str,
    pub columns: &'static [ManagedColumn],
    pub column_renames: &'static [ColumnRenameRule],
    pub maintenance_sql_hooks: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct TableRenameRule {
    pub from: &'static str,
    pub to: &'static str,
}

#[derive(Debug, Clone)]
pub struct SchemaVersionRecord {
    pub schema_version: String,
    pub applied_at: String,
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub applied_versions: Vec<String>,
    pub current_version: Option<SchemaVersionRecord>,
}

// ─── Entry point ─────────────────────────────────────────────────

/// Run declarative schema reconciliation.
///
/// Compares the desired table/column definitions against the live database
/// and converges to the target state.  Records `TARGET_SCHEMA_VERSION` in
/// the `schema_version` table when changes are applied.
pub async fn run_migrations(conn: &Connection) -> Result<MigrationReport> {
    configure_sqlite(conn)?;
    validate_target_schema_definition()?;
    ensure_schema_version_table(conn).await?;

    let current_before = load_current_version(conn).await?;
    reconcile_database_schema(conn).await?;

    let applied_versions =
        if current_before.as_ref().map(|v| v.schema_version.as_str()) != Some(TARGET_SCHEMA_VERSION)
        {
            record_schema_version(conn, TARGET_SCHEMA_VERSION).await?;
            info!("Schema updated to version {}", TARGET_SCHEMA_VERSION);
            vec![TARGET_SCHEMA_VERSION.to_string()]
        } else {
            info!("Schema is up to date at version {}", TARGET_SCHEMA_VERSION);
            Vec::new()
        };

    let current_version = load_current_version(conn).await?;
    Ok(MigrationReport {
        applied_versions,
        current_version,
    })
}

// ─── Internal helpers ────────────────────────────────────────────

fn configure_sqlite(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))
        .context("Failed to set SQLite busy_timeout")?;
    Ok(())
}

fn validate_target_schema_definition() -> Result<()> {
    if TARGET_SCHEMA_VERSION.trim().is_empty() {
        bail!("TARGET_SCHEMA_VERSION cannot be empty");
    }
    if tables::MANAGED_TABLES.is_empty() {
        bail!("Managed Turso table list is empty");
    }

    let mut managed_table_names = HashSet::new();
    for table in tables::MANAGED_TABLES {
        if table.name.trim().is_empty() {
            bail!("Managed table name cannot be empty");
        }
        if table.name == SCHEMA_VERSION_TABLE {
            bail!(
                "Managed tables cannot include reserved table `{}`",
                SCHEMA_VERSION_TABLE
            );
        }
        if !managed_table_names.insert(table.name) {
            bail!("Duplicate managed table name in schema: {}", table.name);
        }
        if table.create_sql.trim().is_empty() {
            bail!("create_sql is empty for managed table `{}`", table.name);
        }
        validate_table_column_rules(table)?;
    }

    validate_table_rename_rules(&managed_table_names)?;
    Ok(())
}

fn validate_table_column_rules(table: &ManagedTable) -> Result<()> {
    let mut desired_column_names = HashSet::new();
    for column in table.columns {
        if column.name.trim().is_empty() {
            bail!(
                "Managed column name cannot be empty (table `{}`)",
                table.name
            );
        }
        if column.definition.trim().is_empty() {
            bail!(
                "Managed column definition cannot be empty (table `{}`, column `{}`)",
                table.name,
                column.name
            );
        }
        if !desired_column_names.insert(column.name) {
            bail!(
                "Duplicate managed column `{}` in table `{}`",
                column.name,
                table.name
            );
        }
    }

    let mut seen_from = HashSet::new();
    let mut seen_to = HashSet::new();
    for rename in table.column_renames {
        if rename.from.trim().is_empty() || rename.to.trim().is_empty() {
            bail!(
                "Column rename rule has empty names in table `{}`",
                table.name
            );
        }
        if rename.from == rename.to {
            bail!(
                "Column rename rule cannot rename `{}` to itself in table `{}`",
                rename.from,
                table.name
            );
        }
        if desired_column_names.contains(rename.from) {
            bail!(
                "Column rename source `{}` is still declared in desired columns for table `{}`",
                rename.from,
                table.name
            );
        }
        if !desired_column_names.contains(rename.to) {
            bail!(
                "Column rename target `{}` is not declared in desired columns for table `{}`",
                rename.to,
                table.name
            );
        }
        if !seen_from.insert(rename.from) {
            bail!(
                "Duplicate column rename source `{}` in table `{}`",
                rename.from,
                table.name
            );
        }
        if !seen_to.insert(rename.to) {
            bail!(
                "Duplicate column rename target `{}` in table `{}`",
                rename.to,
                table.name
            );
        }
    }
    Ok(())
}

fn validate_table_rename_rules(managed_table_names: &HashSet<&str>) -> Result<()> {
    let mut seen_from = HashSet::new();
    let mut seen_to = HashSet::new();
    for rename in tables::TABLE_RENAME_RULES {
        if rename.from.trim().is_empty() || rename.to.trim().is_empty() {
            bail!("Table rename rule has empty names");
        }
        if rename.from == rename.to {
            bail!(
                "Table rename rule cannot rename `{}` to itself",
                rename.from
            );
        }
        if rename.from == SCHEMA_VERSION_TABLE || rename.to == SCHEMA_VERSION_TABLE {
            bail!(
                "Table rename rules cannot reference reserved table `{}`",
                SCHEMA_VERSION_TABLE
            );
        }
        if managed_table_names.contains(rename.from) {
            bail!(
                "Table rename source `{}` is still declared in managed tables",
                rename.from
            );
        }
        if !managed_table_names.contains(rename.to) {
            bail!(
                "Table rename target `{}` is not declared in managed tables",
                rename.to
            );
        }
        if !seen_from.insert(rename.from) {
            bail!("Duplicate table rename source `{}`", rename.from);
        }
        if !seen_to.insert(rename.to) {
            bail!("Duplicate table rename target `{}`", rename.to);
        }
    }
    Ok(())
}

async fn ensure_schema_version_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_SCHEMA_VERSION_TABLE_SQL)
        .await
        .context("Failed to create schema_version table")?;
    conn.execute_batch(CREATE_SCHEMA_VERSION_TABLE_INDEX_SQL)
        .await
        .context("Failed to create schema_version index")?;
    conn.execute("PRAGMA foreign_keys = ON;", ())
        .await
        .context("Failed to enable PRAGMA foreign_keys")?;
    Ok(())
}

async fn reconcile_database_schema(conn: &Connection) -> Result<()> {
    let mut existing_tables = load_existing_tables(conn).await?;
    apply_table_rename_rules(conn, &existing_tables).await?;
    existing_tables = load_existing_tables(conn).await?;

    let existing_set: HashSet<&str> = existing_tables.iter().map(String::as_str).collect();

    for desired in tables::MANAGED_TABLES {
        let existed_before = existing_set.contains(desired.name);
        if !existed_before {
            conn.execute_batch(desired.create_sql)
                .await
                .with_context(|| format!("Failed to create table `{}`", desired.name))?;
            execute_sql_hooks(conn, desired.name, desired.maintenance_sql_hooks).await?;
            continue;
        }

        reconcile_table_columns(conn, desired).await?;
        execute_sql_hooks(conn, desired.name, desired.maintenance_sql_hooks).await?;
    }

    // Drop any tables that exist in the DB but are not managed (and not schema_version)
    let managed_set: HashSet<&str> = tables::MANAGED_TABLES.iter().map(|t| t.name).collect();
    let final_existing = load_existing_tables(conn).await?;

    for table_name in &final_existing {
        if table_name == SCHEMA_VERSION_TABLE || managed_set.contains(table_name.as_str()) {
            continue;
        }
        let drop_sql = format!("DROP TABLE IF EXISTS {};", quote_ident(table_name));
        conn.execute_batch(&drop_sql)
            .await
            .with_context(|| format!("Failed to drop unmanaged table `{}`", table_name))?;
        info!("Dropped unmanaged table `{}`", table_name);
    }

    // Verify all managed tables exist
    let verify_existing = load_existing_tables(conn).await?;
    let final_set: HashSet<&str> = verify_existing.iter().map(String::as_str).collect();
    for managed in tables::MANAGED_TABLES {
        if !final_set.contains(managed.name) {
            bail!(
                "Managed table `{}` was not created during reconciliation",
                managed.name
            );
        }
    }

    Ok(())
}

async fn apply_table_rename_rules(conn: &Connection, existing_tables: &[String]) -> Result<()> {
    let mut existing_set: HashSet<String> = existing_tables.iter().cloned().collect();
    for rename in tables::TABLE_RENAME_RULES {
        let from_exists = existing_set.contains(rename.from);
        let to_exists = existing_set.contains(rename.to);
        match (from_exists, to_exists) {
            (true, false) => {
                let sql = format!(
                    "ALTER TABLE {} RENAME TO {};",
                    quote_ident(rename.from),
                    quote_ident(rename.to)
                );
                conn.execute_batch(&sql).await.with_context(|| {
                    format!(
                        "Failed to rename table `{}` to `{}`",
                        rename.from, rename.to
                    )
                })?;
                existing_set.remove(rename.from);
                existing_set.insert(rename.to.to_string());
            }
            (true, true) => {
                bail!(
                    "Cannot apply table rename `{}` -> `{}` because both tables already exist",
                    rename.from,
                    rename.to
                );
            }
            (false, _) => {}
        }
    }
    Ok(())
}

async fn load_existing_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC;",
            (),
        )
        .await
        .context("Failed to list existing SQLite tables")?;

    let mut tables = Vec::new();
    while let Some(row) = rows.next().await.context("Failed to read table row")? {
        tables.push(
            row.get::<String>(0)
                .context("Failed to decode table name as text")?,
        );
    }
    Ok(tables)
}

async fn reconcile_table_columns(conn: &Connection, table: &ManagedTable) -> Result<()> {
    let mut existing_columns = load_table_columns(conn, table.name).await?;
    apply_column_rename_rules(conn, table.name, table.column_renames, &existing_columns).await?;

    existing_columns = load_table_columns(conn, table.name).await?;
    let existing_set: HashSet<&str> = existing_columns.iter().map(String::as_str).collect();

    for desired in table.columns {
        if existing_set.contains(desired.name) {
            continue;
        }
        let add_sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            quote_ident(table.name),
            quote_ident(desired.name),
            desired.definition
        );
        conn.execute_batch(&add_sql).await.with_context(|| {
            format!(
                "Failed to add column `{}` to table `{}`",
                desired.name, table.name
            )
        })?;
    }

    existing_columns = load_table_columns(conn, table.name).await?;
    let desired_set: HashSet<&str> = table.columns.iter().map(|col| col.name).collect();
    for existing in existing_columns {
        if desired_set.contains(existing.as_str()) {
            continue;
        }
        let drop_sql = format!(
            "ALTER TABLE {} DROP COLUMN {};",
            quote_ident(table.name),
            quote_ident(existing.as_str())
        );
        conn.execute_batch(&drop_sql).await.with_context(|| {
            format!(
                "Failed to drop removed column `{}` from table `{}`",
                existing, table.name
            )
        })?;
    }

    Ok(())
}

async fn apply_column_rename_rules(
    conn: &Connection,
    table_name: &str,
    rules: &[ColumnRenameRule],
    existing_columns: &[String],
) -> Result<()> {
    let existing_set: HashSet<&str> = existing_columns.iter().map(String::as_str).collect();
    for rule in rules {
        let from_exists = existing_set.contains(rule.from);
        let to_exists = existing_set.contains(rule.to);
        match (from_exists, to_exists) {
            (true, false) => {
                let rename_sql = format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {};",
                    quote_ident(table_name),
                    quote_ident(rule.from),
                    quote_ident(rule.to)
                );
                conn.execute_batch(&rename_sql).await.with_context(|| {
                    format!(
                        "Failed to rename column `{}` to `{}` in table `{}`",
                        rule.from, rule.to, table_name
                    )
                })?;
            }
            (true, true) => {
                bail!(
                    "Cannot apply column rename `{}` -> `{}` in table `{}` because both columns exist",
                    rule.from,
                    rule.to,
                    table_name
                );
            }
            (false, _) => {}
        }
    }
    Ok(())
}

async fn execute_sql_hooks(
    conn: &Connection,
    table_name: &str,
    hooks: &[&str],
) -> Result<()> {
    for hook in hooks {
        if hook.trim().is_empty() {
            continue;
        }
        let sql = hook.replace("{table}", &quote_ident(table_name));
        conn.execute_batch(&sql).await.with_context(|| {
            format!(
                "Failed to execute SQL hook for table `{}`",
                table_name
            )
        })?;
    }
    Ok(())
}

async fn load_table_columns(conn: &Connection, table_name: &str) -> Result<Vec<String>> {
    let pragma_sql = format!("PRAGMA table_info({});", quote_ident(table_name));
    let mut rows = conn
        .query(&pragma_sql, ())
        .await
        .with_context(|| format!("Failed to introspect columns for table `{}`", table_name))?;

    let mut columns = Vec::new();
    while let Some(row) = rows.next().await.context("Failed to read column row")? {
        columns.push(
            row.get::<String>(1)
                .context("Failed to decode column name as text")?,
        );
    }
    Ok(columns)
}

async fn record_schema_version(conn: &Connection, version: &str) -> Result<()> {
    let insert_sql = format!(
        "INSERT INTO {} (schema_version) VALUES (?1);",
        quote_ident(SCHEMA_VERSION_TABLE)
    );
    conn.execute(&insert_sql, [version])
        .await
        .with_context(|| format!("Failed to record schema version {}", version))?;
    Ok(())
}

async fn load_current_version(conn: &Connection) -> Result<Option<SchemaVersionRecord>> {
    let query = format!(
        "SELECT schema_version, applied_at FROM {} ORDER BY id DESC LIMIT 1;",
        quote_ident(SCHEMA_VERSION_TABLE)
    );
    let mut rows = conn
        .query(&query, ())
        .await
        .context("Failed to query current schema version")?;

    match rows
        .next()
        .await
        .context("Failed to read current schema row")?
    {
        Some(row) => Ok(Some(SchemaVersionRecord {
            schema_version: row
                .get::<String>(0)
                .context("Failed to decode current schema_version")?,
            applied_at: row
                .get::<String>(1)
                .context("Failed to decode current schema applied_at")?,
        })),
        None => Ok(None),
    }
}

fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}
