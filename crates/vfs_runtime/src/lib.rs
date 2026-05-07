// Where: crates/vfs_runtime/src/lib.rs
// What: Service orchestration for multiple SQLite-backed VFS databases.
// Why: One canister can host isolated databases while sharing one VFS store implementation.
use std::fs::{File, OpenOptions, create_dir_all, metadata, remove_file};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use vfs_store::FsStore;
use vfs_types::{
    AppendNodeRequest, ChildNode, DatabaseArchiveInfo, DatabaseInfo, DatabaseMember, DatabaseRole,
    DatabaseStatus, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest, EditNodeResult,
    ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest, FetchUpdatesResponse,
    GlobNodeHit, GlobNodesRequest, GraphLinksRequest, GraphNeighborhoodRequest,
    IncomingLinksRequest, LinkEdge, ListChildrenRequest, ListNodesRequest, MkdirNodeRequest,
    MkdirNodeResult, MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest, MultiEditNodeResult,
    Node, NodeContext, NodeContextRequest, NodeEntry, OutgoingLinksRequest, QueryContext,
    QueryContextRequest, RecentNodeHit, RecentNodesRequest, SearchNodeHit, SearchNodePathsRequest,
    SearchNodesRequest, SourceEvidence, SourceEvidenceRequest, Status, WriteNodeRequest,
    WriteNodeResult,
};
use wiki_domain::validate_source_path_for_kind;

const INDEX_SCHEMA_VERSION_INITIAL: &str = "database_index:000_initial";
const INDEX_SCHEMA_VERSION_LIFECYCLE: &str = "database_index:001_lifecycle";
const INDEX_SCHEMA_VERSION_RESTORE_SIZE: &str = "database_index:002_restore_size";
const INDEX_SCHEMA_VERSION_RESTORE_CHUNKS: &str = "database_index:003_restore_chunks";
const INDEX_SCHEMA_VERSION_USAGE_EVENTS: &str = "database_index:004_usage_events";
const DATABASE_SCHEMA_VERSION: &str = "vfs_store:current";
const MIN_DATABASE_MOUNT_ID: u16 = 11;
const MAX_DATABASE_MOUNT_ID: u16 = 32767;
pub const MAX_ARCHIVE_CHUNK_BYTES: u32 = 1024 * 1024;
pub const MAX_RESTORE_CHUNK_BYTES: usize = 1024 * 1024;
pub const MAX_DATABASE_SIZE_BYTES: u64 = i64::MAX as u64;
pub const USAGE_EVENTS_RETENTION_LIMIT: u64 = 100_000;
const USAGE_EVENTS_PURGE_INTERVAL: i64 = 100;
const SHA256_DIGEST_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseMeta {
    pub database_id: String,
    pub db_file_name: String,
    pub mount_id: u16,
    pub schema_version: String,
    pub logical_size_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredRole {
    Reader,
    Writer,
    Owner,
}

pub struct UsageEvent<'a> {
    pub method: &'a str,
    pub database_id: Option<&'a str>,
    pub caller: &'a str,
    pub success: bool,
    pub cycles_delta: u128,
    pub error: Option<&'a str>,
    pub now: i64,
}

pub struct VfsService {
    index_path: PathBuf,
    databases_dir: PathBuf,
}

impl VfsService {
    pub fn new(index_path: PathBuf, databases_dir: PathBuf) -> Self {
        Self {
            index_path,
            databases_dir,
        }
    }

    pub fn run_index_migrations(&self) -> Result<(), String> {
        let mut conn = self.open_index()?;
        run_index_migrations(&mut conn)
    }

    pub fn list_databases(&self) -> Result<Vec<DatabaseMeta>, String> {
        let conn = self.open_index()?;
        load_databases(&conn)
    }

    pub fn list_database_infos(&self) -> Result<Vec<DatabaseInfo>, String> {
        let conn = self.open_index()?;
        load_database_infos(&conn)
    }

    pub fn list_database_infos_for_caller(
        &self,
        caller: &str,
    ) -> Result<Vec<DatabaseInfo>, String> {
        let conn = self.open_index()?;
        load_database_infos_for_caller(&conn, caller)
    }

    pub fn record_usage_event(&self, event: UsageEvent<'_>) -> Result<(), String> {
        let conn = self.open_index()?;
        conn.execute(
            "INSERT INTO usage_events
             (method, database_id, caller, success, cycles_delta, error, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.method,
                event.database_id,
                event.caller,
                if event.success { 1_i64 } else { 0_i64 },
                i64::try_from(event.cycles_delta).unwrap_or(i64::MAX),
                event.error,
                event.now
            ],
        )
        .map_err(|error| error.to_string())?;
        let event_id = conn.last_insert_rowid();
        if event_id % USAGE_EVENTS_PURGE_INTERVAL == 0 {
            let _ = purge_old_usage_events(&conn);
        }
        Ok(())
    }

    pub fn usage_event_count(&self) -> Result<u64, String> {
        let conn = self.open_index()?;
        conn.query_row("SELECT COUNT(*) FROM usage_events", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|count| count.max(0) as u64)
        .map_err(|error| error.to_string())
    }

    pub fn create_database(
        &self,
        database_id: &str,
        caller: &str,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        let meta = self.reserve_database(database_id, caller, now)?;
        self.run_database_migrations(database_id)?;
        Ok(meta)
    }

    pub fn reserve_database(
        &self,
        database_id: &str,
        caller: &str,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        validate_database_id(database_id)?;
        let mut conn = self.open_index()?;
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        if database_exists(&tx, database_id)? {
            return Err(format!("database already exists: {database_id}"));
        }
        let mount_id = allocate_mount_id(&tx)?;
        let db_file_name = database_file_name(&self.databases_dir, database_id)?;
        tx.execute(
            "INSERT INTO databases
             (database_id, db_file_name, mount_id, active_mount_id, status, schema_version,
              logical_size_bytes, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?3, 'hot', ?4, 0, ?5, ?5)",
            params![
                database_id,
                db_file_name,
                i64::from(mount_id),
                DATABASE_SCHEMA_VERSION,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO database_members
             (database_id, principal, role, created_at_ms)
             VALUES (?1, ?2, 'owner', ?3)",
            params![database_id, caller, now],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        Ok(DatabaseMeta {
            database_id: database_id.to_string(),
            db_file_name,
            mount_id,
            schema_version: DATABASE_SCHEMA_VERSION.to_string(),
            logical_size_bytes: 0,
        })
    }

    pub fn discard_database_reservation(&self, database_id: &str) -> Result<(), String> {
        let mut conn = self.open_index()?;
        let db_file_name: Option<String> = conn
            .query_row(
                "SELECT db_file_name
                 FROM databases
                 WHERE database_id = ?1",
                params![database_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute(
            "DELETE FROM database_members WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "DELETE FROM database_restore_chunks WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "DELETE FROM databases WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        if let Some(db_file_name) = db_file_name
            && let Err(error) = remove_file(&db_file_name)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.to_string());
        }
        Ok(())
    }

    pub fn run_database_migrations(&self, database_id: &str) -> Result<(), String> {
        let meta = self.database_meta(database_id)?;
        if let Some(parent) = Path::new(&meta.db_file_name).parent() {
            create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let result = FsStore::new(PathBuf::from(&meta.db_file_name)).run_fs_migrations();
        if result.is_ok() {
            self.refresh_logical_size(database_id)?;
        }
        result
    }

    pub fn delete_database(&self, database_id: &str, caller: &str, now: i64) -> Result<(), String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta(database_id)?;
        let _ = remove_file(&meta.db_file_name);
        let conn = self.open_index()?;
        conn.execute(
            "UPDATE databases
             SET status = 'deleted',
                 active_mount_id = NULL,
                 logical_size_bytes = 0,
                 restore_size_bytes = NULL,
                 deleted_at_ms = ?2,
                 updated_at_ms = ?2
             WHERE database_id = ?1",
            params![database_id, now],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn begin_database_archive(
        &self,
        database_id: &str,
        caller: &str,
    ) -> Result<DatabaseArchiveInfo, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta(database_id)?;
        let size_bytes = file_size(&meta.db_file_name)?;
        let conn = self.open_index()?;
        conn.execute(
            "UPDATE databases
             SET status = 'archiving'
             WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        Ok(DatabaseArchiveInfo {
            database_id: database_id.to_string(),
            size_bytes,
        })
    }

    pub fn read_database_archive_chunk(
        &self,
        database_id: &str,
        caller: &str,
        offset: u64,
        max_bytes: u32,
    ) -> Result<Vec<u8>, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta_with_statuses(database_id, &[DatabaseStatus::Archiving])?;
        if max_bytes == 0 {
            return Ok(Vec::new());
        }
        if max_bytes > MAX_ARCHIVE_CHUNK_BYTES {
            return Err(format!(
                "archive chunk size exceeds limit: {max_bytes} > {MAX_ARCHIVE_CHUNK_BYTES}"
            ));
        }
        let size = file_size(&meta.db_file_name)?;
        if offset >= size {
            return Ok(Vec::new());
        }
        let mut file = File::open(&meta.db_file_name).map_err(|error| error.to_string())?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        let remaining = size.saturating_sub(offset);
        let chunk_len = remaining.min(u64::from(max_bytes));
        let mut bytes = Vec::with_capacity(chunk_len as usize);
        file.take(chunk_len)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        Ok(bytes)
    }

    pub fn finalize_database_archive(
        &self,
        database_id: &str,
        caller: &str,
        snapshot_hash: Vec<u8>,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta_with_statuses(database_id, &[DatabaseStatus::Archiving])?;
        validate_snapshot_hash(&snapshot_hash)?;
        let actual_hash = file_sha256(&meta.db_file_name)?;
        if actual_hash != snapshot_hash {
            return Err("snapshot_hash does not match archived database bytes".to_string());
        }
        let conn = self.open_index()?;
        conn.execute(
            "UPDATE databases
             SET status = 'archived',
                 active_mount_id = NULL,
                 snapshot_hash = ?2,
                 restore_size_bytes = NULL,
                 archived_at_ms = ?3,
                 updated_at_ms = ?3
             WHERE database_id = ?1",
            params![database_id, snapshot_hash, now],
        )
        .map_err(|error| error.to_string())?;
        Ok(meta)
    }

    pub fn cancel_database_archive(
        &self,
        database_id: &str,
        caller: &str,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta_with_statuses(database_id, &[DatabaseStatus::Archiving])?;
        let conn = self.open_index()?;
        conn.execute(
            "UPDATE databases
             SET status = 'hot',
                 updated_at_ms = ?2
             WHERE database_id = ?1",
            params![database_id, now],
        )
        .map_err(|error| error.to_string())?;
        Ok(meta)
    }

    pub fn begin_database_restore(
        &self,
        database_id: &str,
        caller: &str,
        snapshot_hash: Vec<u8>,
        size_bytes: u64,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        validate_snapshot_hash(&snapshot_hash)?;
        if size_bytes > MAX_DATABASE_SIZE_BYTES {
            return Err(format!(
                "database size exceeds limit: {size_bytes} > {MAX_DATABASE_SIZE_BYTES}"
            ));
        }
        let current_status = self.database_status(database_id)?;
        if !matches!(
            current_status,
            DatabaseStatus::Archived | DatabaseStatus::Deleted
        ) {
            return Err(
                "database restore can only begin from archived or deleted status".to_string(),
            );
        }
        let mut conn = self.open_index()?;
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        let mount_id = allocate_mount_id(&tx)?;
        tx.execute(
            "DELETE FROM database_restore_chunks WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "UPDATE databases
             SET status = 'restoring',
                 active_mount_id = ?2,
                 snapshot_hash = ?3,
                 archived_at_ms = NULL,
                 deleted_at_ms = NULL,
                 restore_size_bytes = ?4,
                 updated_at_ms = ?5
             WHERE database_id = ?1",
            params![
                database_id,
                i64::from(mount_id),
                snapshot_hash,
                i64::try_from(size_bytes).map_err(|error| error.to_string())?,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        let meta = self.database_meta_allowing_restoring(database_id)?;
        let _ = remove_file(&meta.db_file_name);
        Ok(meta)
    }

    pub fn write_database_restore_chunk(
        &self,
        database_id: &str,
        caller: &str,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        if bytes.len() > MAX_RESTORE_CHUNK_BYTES {
            return Err(format!(
                "restore chunk size exceeds limit: {} > {MAX_RESTORE_CHUNK_BYTES}",
                bytes.len()
            ));
        }
        let meta = self.database_meta_with_statuses(database_id, &[DatabaseStatus::Restoring])?;
        let expected_size = self.restore_size_bytes(database_id)?;
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| "restore chunk range overflows u64".to_string())?;
        if end > expected_size {
            return Err(format!(
                "restore chunk exceeds expected size: end {end} > {expected_size}"
            ));
        }
        if let Some(parent) = Path::new(&meta.db_file_name).parent() {
            create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&meta.db_file_name)
            .map_err(|error| error.to_string())?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        let conn = self.open_index()?;
        conn.execute(
            "INSERT OR REPLACE INTO database_restore_chunks (database_id, offset_bytes, end_bytes)
             VALUES (?1, ?2, ?3)",
            params![
                database_id,
                i64::try_from(offset).map_err(|error| error.to_string())?,
                i64::try_from(end).map_err(|error| error.to_string())?
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn finalize_database_restore(
        &self,
        database_id: &str,
        caller: &str,
        now: i64,
    ) -> Result<DatabaseMeta, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        let meta = self.database_meta_with_statuses(database_id, &[DatabaseStatus::Restoring])?;
        let expected_size = self.restore_size_bytes(database_id)?;
        if !restore_chunks_cover_expected_size(&self.open_index()?, database_id, expected_size)? {
            return Err(format!(
                "restore chunks are incomplete for expected size {expected_size} bytes"
            ));
        }
        OpenOptions::new()
            .write(true)
            .open(&meta.db_file_name)
            .and_then(|file| file.set_len(expected_size))
            .map_err(|error| error.to_string())?;
        let size = file_size(&meta.db_file_name)?;
        if size != expected_size {
            return Err(format!(
                "restore size mismatch: expected {expected_size} bytes, got {size} bytes"
            ));
        }
        let expected_hash = self.restore_snapshot_hash(database_id)?;
        let actual_hash = file_sha256(&meta.db_file_name)?;
        if actual_hash != expected_hash {
            return Err("snapshot_hash does not match restored database bytes".to_string());
        }
        FsStore::new(PathBuf::from(&meta.db_file_name)).run_fs_migrations()?;
        let mut conn = self.open_index()?;
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute(
            "DELETE FROM database_restore_chunks WHERE database_id = ?1",
            params![database_id],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "UPDATE databases
             SET status = 'hot',
                 logical_size_bytes = ?2,
                 restore_size_bytes = NULL,
                 updated_at_ms = ?3
             WHERE database_id = ?1",
            params![
                database_id,
                i64::try_from(size).map_err(|error| error.to_string())?,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        self.database_meta(database_id)
    }

    pub fn grant_database_access(
        &self,
        database_id: &str,
        caller: &str,
        principal: &str,
        role: DatabaseRole,
        now: i64,
    ) -> Result<(), String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        if caller == principal && role != DatabaseRole::Owner {
            return Err("owner cannot downgrade own access".to_string());
        }
        let conn = self.open_index()?;
        conn.execute(
            "INSERT INTO database_members (database_id, principal, role, created_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(database_id, principal)
             DO UPDATE SET role = excluded.role",
            params![database_id, principal, role_to_db(role), now],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn revoke_database_access(
        &self,
        database_id: &str,
        caller: &str,
        principal: &str,
    ) -> Result<(), String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        self.database_meta(database_id)?;
        if caller == principal {
            return Err("owner cannot revoke own access".to_string());
        }
        let conn = self.open_index()?;
        conn.execute(
            "DELETE FROM database_members WHERE database_id = ?1 AND principal = ?2",
            params![database_id, principal],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn list_database_members(
        &self,
        database_id: &str,
        caller: &str,
    ) -> Result<Vec<DatabaseMember>, String> {
        self.require_role(database_id, caller, RequiredRole::Owner)?;
        self.database_meta(database_id)?;
        let conn = self.open_index()?;
        conn.prepare(
            "SELECT database_id, principal, role, created_at_ms
             FROM database_members
             WHERE database_id = ?1
             ORDER BY principal ASC",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![database_id], |row| {
            Ok(DatabaseMember {
                database_id: row.get(0)?,
                principal: row.get(1)?,
                role: role_from_db(&row.get::<_, String>(2)?)?,
                created_at_ms: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
    }

    pub fn status(&self, database_id: &str, caller: &str) -> Result<Status, String> {
        self.with_database_store(database_id, caller, RequiredRole::Reader, |store| {
            store.status()
        })
    }

    pub fn read_node(
        &self,
        database_id: &str,
        caller: &str,
        path: &str,
    ) -> Result<Option<Node>, String> {
        self.with_database_store(database_id, caller, RequiredRole::Reader, |store| {
            store.read_node(path)
        })
    }

    pub fn list_nodes(
        &self,
        caller: &str,
        request: ListNodesRequest,
    ) -> Result<Vec<NodeEntry>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.list_nodes(request)
        })
    }

    pub fn list_children(
        &self,
        caller: &str,
        request: ListChildrenRequest,
    ) -> Result<Vec<ChildNode>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.list_children(request)
        })
    }

    pub fn write_node(
        &self,
        caller: &str,
        request: WriteNodeRequest,
        now: i64,
    ) -> Result<WriteNodeResult, String> {
        validate_source_path_for_kind(&request.path, &request.kind)?;
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.write_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn delete_node(
        &self,
        caller: &str,
        request: DeleteNodeRequest,
        now: i64,
    ) -> Result<DeleteNodeResult, String> {
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.delete_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn append_node(
        &self,
        caller: &str,
        request: AppendNodeRequest,
        now: i64,
    ) -> Result<WriteNodeResult, String> {
        if let Some(kind) = request.kind.as_ref() {
            validate_source_path_for_kind(&request.path, kind)?;
        }
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.append_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn edit_node(
        &self,
        caller: &str,
        request: EditNodeRequest,
        now: i64,
    ) -> Result<EditNodeResult, String> {
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.edit_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn mkdir_node(
        &self,
        caller: &str,
        request: MkdirNodeRequest,
    ) -> Result<MkdirNodeResult, String> {
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.mkdir_node(request)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn move_node(
        &self,
        caller: &str,
        request: MoveNodeRequest,
        now: i64,
    ) -> Result<MoveNodeResult, String> {
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                if let Some(node) = store.read_node(&request.from_path)? {
                    validate_source_path_for_kind(&request.to_path, &node.kind)?;
                }
                store.move_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn glob_nodes(
        &self,
        caller: &str,
        request: GlobNodesRequest,
    ) -> Result<Vec<GlobNodeHit>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.glob_nodes(request)
        })
    }

    pub fn recent_nodes(
        &self,
        caller: &str,
        request: RecentNodesRequest,
    ) -> Result<Vec<RecentNodeHit>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.recent_nodes(request)
        })
    }

    pub fn incoming_links(
        &self,
        caller: &str,
        request: IncomingLinksRequest,
    ) -> Result<Vec<LinkEdge>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.incoming_links(request)
        })
    }

    pub fn outgoing_links(
        &self,
        caller: &str,
        request: OutgoingLinksRequest,
    ) -> Result<Vec<LinkEdge>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.outgoing_links(request)
        })
    }

    pub fn graph_links(
        &self,
        caller: &str,
        request: GraphLinksRequest,
    ) -> Result<Vec<LinkEdge>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.graph_links(request)
        })
    }

    pub fn graph_neighborhood(
        &self,
        caller: &str,
        request: GraphNeighborhoodRequest,
    ) -> Result<Vec<LinkEdge>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.graph_neighborhood(request)
        })
    }

    pub fn read_node_context(
        &self,
        caller: &str,
        request: NodeContextRequest,
    ) -> Result<Option<NodeContext>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.read_node_context(request)
        })
    }

    pub fn query_context(
        &self,
        caller: &str,
        request: QueryContextRequest,
    ) -> Result<QueryContext, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.query_context(request)
        })
    }

    pub fn source_evidence(
        &self,
        caller: &str,
        request: SourceEvidenceRequest,
    ) -> Result<SourceEvidence, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.source_evidence(request)
        })
    }

    pub fn multi_edit_node(
        &self,
        caller: &str,
        request: MultiEditNodeRequest,
        now: i64,
    ) -> Result<MultiEditNodeResult, String> {
        let database_id = request.database_id.clone();
        let result =
            self.with_database_store(&database_id, caller, RequiredRole::Writer, |store| {
                store.multi_edit_node(request, now)
            });
        if result.is_ok() {
            self.refresh_logical_size(&database_id)?;
        }
        result
    }

    pub fn search_nodes(
        &self,
        caller: &str,
        request: SearchNodesRequest,
    ) -> Result<Vec<SearchNodeHit>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.search_nodes(request)
        })
    }

    pub fn search_node_paths(
        &self,
        caller: &str,
        request: SearchNodePathsRequest,
    ) -> Result<Vec<SearchNodeHit>, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.search_node_paths(request)
        })
    }

    pub fn export_fs_snapshot(
        &self,
        caller: &str,
        request: ExportSnapshotRequest,
    ) -> Result<ExportSnapshotResponse, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.export_snapshot(request)
        })
    }

    pub fn fetch_fs_updates(
        &self,
        caller: &str,
        request: FetchUpdatesRequest,
    ) -> Result<FetchUpdatesResponse, String> {
        let database_id = request.database_id.clone();
        self.with_database_store(&database_id, caller, RequiredRole::Reader, |store| {
            store.fetch_updates(request)
        })
    }

    fn with_database_store<T>(
        &self,
        database_id: &str,
        caller: &str,
        required_role: RequiredRole,
        f: impl FnOnce(&FsStore) -> Result<T, String>,
    ) -> Result<T, String> {
        self.require_role(database_id, caller, required_role)?;
        let meta = self.database_meta(database_id)?;
        let store = FsStore::new(PathBuf::from(meta.db_file_name));
        f(&store)
    }

    fn require_role(
        &self,
        database_id: &str,
        caller: &str,
        required_role: RequiredRole,
    ) -> Result<(), String> {
        let conn = self.open_index()?;
        let role = load_member_role(&conn, database_id, caller)?
            .ok_or_else(|| format!("principal has no access to database: {database_id}"))?;
        if role_allows(role, required_role) {
            Ok(())
        } else {
            Err(format!(
                "principal lacks required database role: {database_id}"
            ))
        }
    }

    fn database_meta(&self, database_id: &str) -> Result<DatabaseMeta, String> {
        let conn = self.open_index()?;
        load_database(&conn, database_id)?.ok_or_else(|| database_meta_error(&conn, database_id))
    }

    fn database_meta_allowing_restoring(&self, database_id: &str) -> Result<DatabaseMeta, String> {
        self.database_meta_with_statuses(
            database_id,
            &[DatabaseStatus::Hot, DatabaseStatus::Restoring],
        )
    }

    fn database_meta_with_statuses(
        &self,
        database_id: &str,
        statuses: &[DatabaseStatus],
    ) -> Result<DatabaseMeta, String> {
        let conn = self.open_index()?;
        load_database_with_statuses(&conn, database_id, statuses)?
            .ok_or_else(|| database_meta_error(&conn, database_id))
    }

    fn database_status(&self, database_id: &str) -> Result<DatabaseStatus, String> {
        let conn = self.open_index()?;
        conn.query_row(
            "SELECT status FROM databases WHERE database_id = ?1",
            params![database_id],
            |row| status_from_db(&row.get::<_, String>(0)?),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("database not found: {database_id}"))
    }

    fn restore_size_bytes(&self, database_id: &str) -> Result<u64, String> {
        let conn = self.open_index()?;
        let size: Option<i64> = conn
            .query_row(
                "SELECT restore_size_bytes FROM databases WHERE database_id = ?1",
                params![database_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("database not found: {database_id}"))?;
        size.map(|size| size.max(0) as u64)
            .ok_or_else(|| format!("restore size is missing: {database_id}"))
    }

    fn restore_snapshot_hash(&self, database_id: &str) -> Result<Vec<u8>, String> {
        let conn = self.open_index()?;
        let hash: Option<Vec<u8>> = conn
            .query_row(
                "SELECT snapshot_hash FROM databases WHERE database_id = ?1",
                params![database_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("database not found: {database_id}"))?;
        hash.ok_or_else(|| format!("snapshot_hash is missing: {database_id}"))
    }

    fn refresh_logical_size(&self, database_id: &str) -> Result<(), String> {
        let meta = self.database_meta_allowing_restoring(database_id)?;
        let size = file_size(&meta.db_file_name)?;
        let conn = self.open_index()?;
        conn.execute(
            "UPDATE databases
             SET logical_size_bytes = ?2
             WHERE database_id = ?1",
            params![database_id, i64::try_from(size).unwrap_or(i64::MAX)],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn open_index(&self) -> Result<Connection, String> {
        Connection::open(&self.index_path).map_err(|error| error.to_string())
    }
}

fn run_index_migrations(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version TEXT PRIMARY KEY,
           applied_at INTEGER NOT NULL
         );",
    )
    .map_err(|error| error.to_string())?;
    if !migration_applied(conn, INDEX_SCHEMA_VERSION_INITIAL)? {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute_batch(
            "CREATE TABLE databases (
               database_id TEXT PRIMARY KEY,
               db_file_name TEXT NOT NULL,
               mount_id INTEGER NOT NULL,
               schema_version TEXT NOT NULL,
               created_at_ms INTEGER NOT NULL,
               updated_at_ms INTEGER NOT NULL
             );
             CREATE UNIQUE INDEX databases_mount_id_idx ON databases(mount_id);
             CREATE TABLE database_members (
               database_id TEXT NOT NULL,
               principal TEXT NOT NULL,
               role TEXT NOT NULL,
               created_at_ms INTEGER NOT NULL,
               PRIMARY KEY (database_id, principal),
               FOREIGN KEY (database_id) REFERENCES databases(database_id)
             );",
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
            params![INDEX_SCHEMA_VERSION_INITIAL],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
    }
    if !migration_applied(conn, INDEX_SCHEMA_VERSION_LIFECYCLE)? {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute_batch(
            "DROP INDEX IF EXISTS databases_mount_id_idx;
             ALTER TABLE databases ADD COLUMN active_mount_id INTEGER;
             ALTER TABLE databases ADD COLUMN status TEXT NOT NULL DEFAULT 'hot';
             ALTER TABLE databases ADD COLUMN logical_size_bytes INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE databases ADD COLUMN snapshot_hash BLOB;
             ALTER TABLE databases ADD COLUMN archived_at_ms INTEGER;
             ALTER TABLE databases ADD COLUMN deleted_at_ms INTEGER;
             UPDATE databases SET active_mount_id = mount_id WHERE active_mount_id IS NULL;
             CREATE UNIQUE INDEX databases_active_mount_id_idx
               ON databases(active_mount_id)
               WHERE active_mount_id IS NOT NULL;",
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
            params![INDEX_SCHEMA_VERSION_LIFECYCLE],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
    }
    if !migration_applied(conn, INDEX_SCHEMA_VERSION_RESTORE_SIZE)? {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute_batch("ALTER TABLE databases ADD COLUMN restore_size_bytes INTEGER;")
            .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
            params![INDEX_SCHEMA_VERSION_RESTORE_SIZE],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
    }
    if !migration_applied(conn, INDEX_SCHEMA_VERSION_RESTORE_CHUNKS)? {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        tx.execute_batch(
            "CREATE TABLE database_restore_chunks (
               database_id TEXT NOT NULL,
               offset_bytes INTEGER NOT NULL,
               end_bytes INTEGER NOT NULL,
               PRIMARY KEY (database_id, offset_bytes, end_bytes),
               FOREIGN KEY (database_id) REFERENCES databases(database_id)
             );
             CREATE INDEX database_restore_chunks_database_id_idx
               ON database_restore_chunks(database_id, offset_bytes);",
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
            params![INDEX_SCHEMA_VERSION_RESTORE_CHUNKS],
        )
        .map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
    }
    if migration_applied(conn, INDEX_SCHEMA_VERSION_USAGE_EVENTS)? {
        return Ok(());
    }
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    tx.execute_batch(
        "CREATE TABLE usage_events (
           event_id INTEGER PRIMARY KEY AUTOINCREMENT,
           method TEXT NOT NULL,
           database_id TEXT,
           caller TEXT NOT NULL,
           success INTEGER NOT NULL,
           cycles_delta INTEGER NOT NULL,
           error TEXT,
           created_at_ms INTEGER NOT NULL
         );
         CREATE INDEX usage_events_database_id_created_at_idx
           ON usage_events(database_id, created_at_ms);
         CREATE INDEX usage_events_caller_created_at_idx
           ON usage_events(caller, created_at_ms);",
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%s','now'))",
        params![INDEX_SCHEMA_VERSION_USAGE_EVENTS],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

fn migration_applied(conn: &Connection, version: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM schema_migrations WHERE version = ?1",
        params![version],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn restore_chunks_cover_expected_size(
    conn: &Connection,
    database_id: &str,
    expected_size: u64,
) -> Result<bool, String> {
    if expected_size == 0 {
        return Ok(true);
    }
    let chunks = conn
        .prepare(
            "SELECT offset_bytes, end_bytes
             FROM database_restore_chunks
             WHERE database_id = ?1
             ORDER BY offset_bytes ASC, end_bytes ASC",
        )
        .map_err(|error| error.to_string())?
        .query_map(params![database_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut covered_end = 0_u64;
    for (offset, end) in chunks {
        let offset = u64::try_from(offset).map_err(|error| error.to_string())?;
        let end = u64::try_from(end).map_err(|error| error.to_string())?;
        if offset > covered_end {
            return Ok(false);
        }
        if end > expected_size {
            return Ok(false);
        }
        covered_end = covered_end.max(end);
        if covered_end == expected_size {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_database_id(database_id: &str) -> Result<(), String> {
    if database_id.is_empty() || database_id.len() > 64 {
        return Err("database_id must be 1..64 characters".to_string());
    }
    if !database_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("database_id may only contain ASCII letters, digits, '-' and '_'".to_string());
    }
    Ok(())
}

fn database_file_name(databases_dir: &Path, database_id: &str) -> Result<String, String> {
    validate_database_id(database_id)?;
    Ok(databases_dir
        .join(format!("{database_id}.sqlite3"))
        .to_string_lossy()
        .into_owned())
}

fn database_exists(conn: &Connection, database_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM databases WHERE database_id = ?1",
        params![database_id],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn allocate_mount_id(conn: &Connection) -> Result<u16, String> {
    let used = conn
        .prepare(
            "SELECT mount_id AS used_mount_id FROM databases
             UNION
             SELECT active_mount_id AS used_mount_id
             FROM databases
             WHERE active_mount_id IS NOT NULL
             ORDER BY used_mount_id ASC",
        )
        .map_err(|error| error.to_string())?
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut used = used.into_iter().map(mount_id_from_db).peekable();
    for mount_id in MIN_DATABASE_MOUNT_ID..=MAX_DATABASE_MOUNT_ID {
        while let Some(used_mount_id) = used.peek() {
            match used_mount_id {
                Ok(used_mount_id) if *used_mount_id < mount_id => {
                    used.next();
                }
                Ok(used_mount_id) if *used_mount_id == mount_id => break,
                Ok(_) => return Ok(mount_id),
                Err(error) => return Err(error.to_string()),
            }
        }
        if used.peek().is_none() {
            return Ok(mount_id);
        }
        used.next();
    }
    Err("database mount_id capacity exhausted".to_string())
}

fn validate_snapshot_hash(snapshot_hash: &[u8]) -> Result<(), String> {
    if snapshot_hash.len() == SHA256_DIGEST_BYTES {
        Ok(())
    } else {
        Err(format!(
            "snapshot_hash must be a {SHA256_DIGEST_BYTES}-byte SHA-256 digest"
        ))
    }
}

fn file_sha256(path: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_vec())
}

fn purge_old_usage_events(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM usage_events
         WHERE event_id <= (
           SELECT COALESCE(MAX(event_id), 0) - ?1 FROM usage_events
         )",
        params![i64::try_from(USAGE_EVENTS_RETENTION_LIMIT).unwrap_or(i64::MAX)],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn database_meta_error(conn: &Connection, database_id: &str) -> String {
    match conn
        .query_row(
            "SELECT status FROM databases WHERE database_id = ?1",
            params![database_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
    {
        Ok(Some(status))
            if status == "hot"
                || status == "archived"
                || status == "archiving"
                || status == "restoring"
                || status == "deleted" =>
        {
            format!("database is {status}: {database_id}")
        }
        _ => format!("database not found: {database_id}"),
    }
}

fn load_database(conn: &Connection, database_id: &str) -> Result<Option<DatabaseMeta>, String> {
    load_database_with_statuses(conn, database_id, &[DatabaseStatus::Hot])
}

fn load_database_with_statuses(
    conn: &Connection,
    database_id: &str,
    statuses: &[DatabaseStatus],
) -> Result<Option<DatabaseMeta>, String> {
    conn.query_row(
        "SELECT database_id, db_file_name, active_mount_id, schema_version, logical_size_bytes, status
         FROM databases
         WHERE database_id = ?1",
        params![database_id],
        |row| map_database_meta_with_statuses(row, statuses),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn load_databases(conn: &Connection) -> Result<Vec<DatabaseMeta>, String> {
    conn.prepare(
        "SELECT database_id, db_file_name, active_mount_id, schema_version, logical_size_bytes, status
         FROM databases
         WHERE status IN ('hot', 'archiving', 'restoring') AND active_mount_id IS NOT NULL
         ORDER BY mount_id ASC",
    )
    .map_err(|error| error.to_string())?
    .query_map([], map_database_meta)
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())
}

fn load_database_infos(conn: &Connection) -> Result<Vec<DatabaseInfo>, String> {
    conn.prepare(
        "SELECT database_id, status, active_mount_id, schema_version, logical_size_bytes,
                snapshot_hash, archived_at_ms, deleted_at_ms
         FROM databases
         ORDER BY database_id ASC",
    )
    .map_err(|error| error.to_string())?
    .query_map([], |row| {
        let mount_id: Option<i64> = row.get(2)?;
        let logical_size_bytes: i64 = row.get(4)?;
        Ok(DatabaseInfo {
            database_id: row.get(0)?,
            status: status_from_db(&row.get::<_, String>(1)?)?,
            mount_id: mount_id.map(mount_id_from_db).transpose()?,
            schema_version: row.get(3)?,
            logical_size_bytes: logical_size_bytes.max(0) as u64,
            snapshot_hash: row.get(5)?,
            archived_at_ms: row.get(6)?,
            deleted_at_ms: row.get(7)?,
        })
    })
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())
}

fn load_database_infos_for_caller(
    conn: &Connection,
    caller: &str,
) -> Result<Vec<DatabaseInfo>, String> {
    conn.prepare(
        "SELECT d.database_id, d.status, d.active_mount_id, d.schema_version, d.logical_size_bytes,
                d.snapshot_hash, d.archived_at_ms, d.deleted_at_ms
         FROM databases d
         INNER JOIN database_members m ON m.database_id = d.database_id
         WHERE m.principal = ?1
         ORDER BY d.database_id ASC",
    )
    .map_err(|error| error.to_string())?
    .query_map(params![caller], |row| {
        let mount_id: Option<i64> = row.get(2)?;
        let logical_size_bytes: i64 = row.get(4)?;
        Ok(DatabaseInfo {
            database_id: row.get(0)?,
            status: status_from_db(&row.get::<_, String>(1)?)?,
            mount_id: mount_id.map(mount_id_from_db).transpose()?,
            schema_version: row.get(3)?,
            logical_size_bytes: logical_size_bytes.max(0) as u64,
            snapshot_hash: row.get(5)?,
            archived_at_ms: row.get(6)?,
            deleted_at_ms: row.get(7)?,
        })
    })
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())
}

fn map_database_meta_with_statuses(
    row: &rusqlite::Row<'_>,
    statuses: &[DatabaseStatus],
) -> rusqlite::Result<DatabaseMeta> {
    let status: String = row.get(5).unwrap_or_else(|_| "hot".to_string());
    let status = status_from_db(&status)?;
    if !statuses.contains(&status) {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    map_database_meta(row)
}

fn map_database_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<DatabaseMeta> {
    let mount_id: Option<i64> = row.get(2)?;
    let mount_id = mount_id.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let logical_size_bytes: i64 = row.get(4)?;
    Ok(DatabaseMeta {
        database_id: row.get(0)?,
        db_file_name: row.get(1)?,
        mount_id: mount_id_from_db(mount_id)?,
        schema_version: row.get(3)?,
        logical_size_bytes: logical_size_bytes.max(0) as u64,
    })
}

fn mount_id_from_db(mount_id: i64) -> rusqlite::Result<u16> {
    u16::try_from(mount_id).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, mount_id))
}

fn load_member_role(
    conn: &Connection,
    database_id: &str,
    principal: &str,
) -> Result<Option<DatabaseRole>, String> {
    conn.query_row(
        "SELECT role FROM database_members WHERE database_id = ?1 AND principal = ?2",
        params![database_id, principal],
        |row| role_from_db(&row.get::<_, String>(0)?),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn role_from_db(role: &str) -> rusqlite::Result<DatabaseRole> {
    match role {
        "owner" => Ok(DatabaseRole::Owner),
        "writer" => Ok(DatabaseRole::Writer),
        "reader" => Ok(DatabaseRole::Reader),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn role_to_db(role: DatabaseRole) -> &'static str {
    match role {
        DatabaseRole::Owner => "owner",
        DatabaseRole::Writer => "writer",
        DatabaseRole::Reader => "reader",
    }
}

fn status_from_db(status: &str) -> rusqlite::Result<DatabaseStatus> {
    match status {
        "hot" => Ok(DatabaseStatus::Hot),
        "archiving" => Ok(DatabaseStatus::Archiving),
        "archived" => Ok(DatabaseStatus::Archived),
        "deleted" => Ok(DatabaseStatus::Deleted),
        "restoring" => Ok(DatabaseStatus::Restoring),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn role_allows(role: DatabaseRole, required_role: RequiredRole) -> bool {
    match required_role {
        RequiredRole::Reader => matches!(
            role,
            DatabaseRole::Reader | DatabaseRole::Writer | DatabaseRole::Owner
        ),
        RequiredRole::Writer => matches!(role, DatabaseRole::Writer | DatabaseRole::Owner),
        RequiredRole::Owner => role == DatabaseRole::Owner,
    }
}

fn file_size(path: &str) -> Result<u64, String> {
    metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| error.to_string())
}
