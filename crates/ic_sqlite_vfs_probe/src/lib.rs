// Where: crates/ic_sqlite_vfs_probe/src/lib.rs
// What: Minimal compile/runtime probe for ic-sqlite-vfs DbHandle usage.
// Why: Validate MemoryManager-backed SQLite images before touching VFS storage.
use ic_sqlite_vfs::db::migrate::Migration;
use ic_sqlite_vfs::{DbError, DbHandle, DbMemory, params};

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: "CREATE TABLE probe_kv (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL
    );",
}];

pub fn init_probe(memory: DbMemory) -> Result<DbHandle, DbError> {
    let handle = DbHandle::init(memory)?;
    handle.migrate(MIGRATIONS)?;
    Ok(handle)
}

pub fn put(handle: DbHandle, key: &str, value: &str) -> Result<(), DbError> {
    handle.update(|connection| {
        connection.execute(
            "INSERT INTO probe_kv(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
    })
}

pub fn get(handle: DbHandle, key: &str) -> Result<Option<String>, DbError> {
    handle.query(|connection| {
        connection.query_optional_scalar::<String>(
            "SELECT value FROM probe_kv WHERE key = ?1",
            params![key],
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::DefaultMemoryImpl;
    use ic_stable_structures::memory_manager::{MemoryId, MemoryManager};

    fn get_from_update(handle: DbHandle, key: &str) -> Result<Option<String>, DbError> {
        handle.update(|connection| {
            connection.query_optional_scalar::<String>(
                "SELECT value FROM probe_kv WHERE key = ?1",
                params![key],
            )
        })
    }

    #[test]
    fn db_handles_are_isolated_by_memory_id() {
        let manager = MemoryManager::init(DefaultMemoryImpl::default());
        let first = init_probe(manager.get(MemoryId::new(120))).expect("first handle initializes");
        let second =
            init_probe(manager.get(MemoryId::new(121))).expect("second handle initializes");

        put(first, "shared-key", "first-value").expect("first write succeeds");
        put(second, "shared-key", "second-value").expect("second write succeeds");

        assert_eq!(
            get_from_update(first, "shared-key").expect("first read succeeds"),
            Some("first-value".to_string())
        );
        assert_eq!(
            get_from_update(second, "shared-key").expect("second read succeeds"),
            Some("second-value".to_string())
        );
    }
}
