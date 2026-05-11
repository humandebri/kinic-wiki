// Where: crates/vfs_store/src/schema.rs
// What: Versioned SQL-file migrations for the FS-first SQLite schema.
// Why: The repo now has one node-based schema, so migration history only tracks FS tables.
use rusqlite::{Connection, OptionalExtension, params};

use crate::fs_links::backfill_node_links;

// Keep the persisted version token stable so existing local databases do not
// require a forced migration just because the crate naming moved from wiki_* to vfs_*.
const CURRENT_SCHEMA_VERSION: &str = "wiki_store:001_fs_links";
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "wiki_store:000_fs_schema",
        include_str!("../migrations/000_fs_schema.sql"),
    ),
    (
        CURRENT_SCHEMA_VERSION,
        include_str!("../migrations/001_fs_links.sql"),
    ),
];
const SCHEMA_MIGRATIONS_BOOTSTRAP_SQL: &str =
    include_str!("../migrations/000_schema_migrations.sql");

pub fn run_fs_migrations(conn: &mut Connection) -> Result<(), String> {
    ensure_schema_migrations_table(conn)?;

    let tx = conn.transaction().map_err(|error| error.to_string())?;
    reject_legacy_schema(&tx)?;
    for (version, sql) in MIGRATIONS {
        if migration_already_applied(&tx, version)? {
            continue;
        }
        tx.execute_batch(sql).map_err(|error| error.to_string())?;
        run_post_migration_hook(&tx, version)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
            params![version],
        )
        .map_err(|error| error.to_string())?;
    }
    tx.commit().map_err(|error| error.to_string())
}

fn run_post_migration_hook(conn: &rusqlite::Transaction<'_>, version: &str) -> Result<(), String> {
    if version == "wiki_store:001_fs_links" {
        backfill_node_links(conn)?;
    }
    Ok(())
}

fn ensure_schema_migrations_table(conn: &Connection) -> Result<(), String> {
    if table_exists(conn, "schema_migrations")? {
        return Ok(());
    }
    conn.execute_batch(SCHEMA_MIGRATIONS_BOOTSTRAP_SQL)
        .map_err(|error| error.to_string())
}

fn reject_legacy_schema(conn: &Connection) -> Result<(), String> {
    let versions = applied_versions(conn)?;
    if versions.is_empty() {
        if managed_table_exists(conn)? {
            return Err("legacy wiki_store schema is unsupported; recreate database".to_string());
        }
        return Ok(());
    }
    let known_versions = MIGRATIONS
        .iter()
        .map(|(version, _sql)| version.to_string())
        .collect::<Vec<_>>();
    if !known_versions.starts_with(&versions) || versions.is_empty() {
        return Err(format!(
            "legacy wiki_store schema is unsupported; recreate database: {}",
            versions.join(", ")
        ));
    }
    if !base_schema_shape_is_present(conn)? {
        return Err(format!(
            "legacy wiki_store schema is unsupported; recreate database: {}",
            versions.join(", ")
        ));
    }
    if versions
        .last()
        .is_some_and(|version| version == CURRENT_SCHEMA_VERSION)
        && !current_schema_shape_is_present(conn)?
    {
        return Err(format!(
            "legacy wiki_store schema is unsupported; recreate database: {}",
            versions.join(", ")
        ));
    }
    Ok(())
}

fn migration_already_applied(conn: &Connection, version: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM schema_migrations WHERE version = ?1",
        params![version],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn index_exists(conn: &Connection, index: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1 LIMIT 1",
        params![index],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn current_schema_shape_is_present(conn: &Connection) -> Result<bool, String> {
    if !base_schema_shape_is_present(conn)? {
        return Ok(false);
    }
    for table in ["fs_links"] {
        if !table_exists(conn, table)? {
            return Ok(false);
        }
    }
    for index in ["fs_links_target_path_idx", "fs_links_source_path_idx"] {
        if !index_exists(conn, index)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn base_schema_shape_is_present(conn: &Connection) -> Result<bool, String> {
    for table in ["fs_nodes", "fs_nodes_fts", "fs_change_log", "fs_path_state"] {
        if !table_exists(conn, table)? {
            return Ok(false);
        }
    }
    for index in ["fs_nodes_path_covering_idx", "fs_nodes_recent_covering_idx"] {
        if !index_exists(conn, index)? {
            return Ok(false);
        }
    }
    fts_shape_is_current(conn)
}

fn fts_shape_is_current(conn: &Connection) -> Result<bool, String> {
    let sql = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'fs_nodes_fts'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(sql) = sql else {
        return Ok(false);
    };
    Ok(sql.contains("path") && sql.contains("title") && sql.contains("content"))
}

fn applied_versions(conn: &Connection) -> Result<Vec<String>, String> {
    conn.prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn managed_table_exists(conn: &Connection) -> Result<bool, String> {
    for table in [
        "fs_nodes",
        "fs_nodes_fts",
        "fs_change_log",
        "fs_path_state",
        "fs_links",
    ] {
        if table_exists(conn, table)? {
            return Ok(true);
        }
    }
    Ok(false)
}
