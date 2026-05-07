// Where: crates/vfs_runtime/tests/database_service.rs
// What: Multi-database service tests over local SQLite files.
// Why: The canister mount layer depends on runtime index and role semantics being deterministic.
use std::path::PathBuf;

use rusqlite::{Connection, params};
use tempfile::tempdir;
use vfs_runtime::{UsageEvent, VfsService};
use vfs_types::{
    DatabaseRole, DatabaseStatus, NodeKind, SearchNodesRequest, SearchPreviewMode, WriteNodeRequest,
};

fn service() -> VfsService {
    service_with_root().0
}

fn service_with_root() -> (VfsService, PathBuf) {
    let dir = tempdir().expect("tempdir should create");
    let root = dir.keep();
    let service = VfsService::new(root.join("index.sqlite3"), root.join("databases"));
    service
        .run_index_migrations()
        .expect("index migrations should run");
    (service, root)
}

fn assert_restore_size(root: &std::path::Path, database_id: &str, expected: Option<u64>) {
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    let actual: Option<i64> = conn
        .query_row(
            "SELECT restore_size_bytes FROM databases WHERE database_id = ?1",
            params![database_id],
            |row| row.get(0),
        )
        .expect("restore size row should exist");
    assert_eq!(actual.map(|size| size as u64), expected);
}

fn database_index_row(
    root: &std::path::Path,
    database_id: &str,
) -> (String, Option<u16>, u64, Option<u64>) {
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    conn.query_row(
        "SELECT status, active_mount_id, logical_size_bytes, restore_size_bytes
         FROM databases WHERE database_id = ?1",
        params![database_id],
        |row| {
            let active_mount_id: Option<i64> = row.get(1)?;
            let logical_size_bytes: i64 = row.get(2)?;
            let restore_size_bytes: Option<i64> = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                active_mount_id.map(|value| value as u16),
                logical_size_bytes.max(0) as u64,
                restore_size_bytes.map(|value| value.max(0) as u64),
            ))
        },
    )
    .expect("database index row should exist")
}

fn database_member_count(root: &std::path::Path, database_id: &str) -> i64 {
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    conn.query_row(
        "SELECT COUNT(*) FROM database_members WHERE database_id = ?1",
        params![database_id],
        |row| row.get(0),
    )
    .expect("member count should load")
}

fn schema_migration_count(root: &std::path::Path, version: &str) -> i64 {
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    conn.query_row(
        "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
        params![version],
        |row| row.get(0),
    )
    .expect("migration count should load")
}

type UsageEventTuple = (
    String,
    Option<String>,
    String,
    i64,
    i64,
    Option<String>,
    i64,
);

fn usage_event_rows(root: &std::path::Path) -> Vec<UsageEventTuple> {
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    conn.prepare(
        "SELECT method, database_id, caller, success, cycles_delta, error, created_at_ms
         FROM usage_events
         ORDER BY event_id ASC",
    )
    .expect("usage query should prepare")
    .query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })
    .expect("usage query should run")
    .collect::<Result<Vec<_>, _>>()
    .expect("usage rows should collect")
}

fn read_archive_in_chunks(
    service: &VfsService,
    database_id: &str,
    size_bytes: u64,
    chunk_size: u32,
) -> Vec<u8> {
    let mut offset = 0_u64;
    let mut bytes = Vec::new();
    while offset < size_bytes {
        let chunk = service
            .read_database_archive_chunk(database_id, "owner", offset, chunk_size)
            .expect("archive chunk should read");
        assert!(chunk.len() <= chunk_size as usize);
        assert!(!chunk.is_empty());
        offset += chunk.len() as u64;
        bytes.extend(chunk);
    }
    bytes
}

fn archive_bytes_for_chunk_size(
    service: &VfsService,
    database_id: &str,
    size_bytes: u64,
    chunk_size: u32,
) -> Vec<u8> {
    if chunk_size >= size_bytes as u32 {
        return service
            .read_database_archive_chunk(database_id, "owner", 0, chunk_size)
            .expect("single archive chunk should read");
    }
    read_archive_in_chunks(service, database_id, size_bytes, chunk_size)
}

#[test]
fn index_migrations_create_usage_events_once() {
    let (service, root) = service_with_root();

    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'usage_events'",
            [],
            |row| row.get(0),
        )
        .expect("usage table lookup should work");
    assert_eq!(table_exists, 1);
    assert_eq!(
        schema_migration_count(&root, "database_index:004_usage_events"),
        1
    );

    service
        .run_index_migrations()
        .expect("index migrations should be idempotent");
    assert_eq!(
        schema_migration_count(&root, "database_index:004_usage_events"),
        1
    );
}

#[test]
fn records_minimal_usage_events() {
    let (service, root) = service_with_root();

    service
        .record_usage_event(UsageEvent {
            method: "write_node",
            database_id: Some("alpha"),
            caller: "owner",
            success: true,
            cycles_delta: 12,
            error: None,
            now: 10,
        })
        .expect("success event should record");
    service
        .record_usage_event(UsageEvent {
            method: "create_database",
            database_id: None,
            caller: "owner",
            success: false,
            cycles_delta: 34,
            error: Some("database already exists"),
            now: 11,
        })
        .expect("failure event should record");

    let rows = usage_event_rows(&root);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        (
            "write_node".to_string(),
            Some("alpha".to_string()),
            "owner".to_string(),
            1,
            12,
            None,
            10
        )
    );
    assert_eq!(
        rows[1],
        (
            "create_database".to_string(),
            None,
            "owner".to_string(),
            0,
            34,
            Some("database already exists".to_string()),
            11
        )
    );
}

#[test]
fn creates_databases_with_unique_mount_ids() {
    let service = service();

    let alpha = service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    let beta = service
        .create_database("beta", "owner", 2)
        .expect("beta should create");

    assert_eq!(alpha.mount_id, 11);
    assert_eq!(beta.mount_id, 12);
    assert_ne!(alpha.db_file_name, beta.db_file_name);
}

#[test]
fn lists_database_infos_for_caller_memberships_only() {
    let service = service();
    service
        .create_database("alpha", "owner_a", 1)
        .expect("alpha should create");
    service
        .create_database("beta", "owner_b", 2)
        .expect("beta should create");
    service
        .grant_database_access("alpha", "owner_a", "owner_b", DatabaseRole::Reader, 3)
        .expect("shared grant should succeed");

    let owner_a_infos = service
        .list_database_infos_for_caller("owner_a")
        .expect("owner_a infos should load");
    assert_eq!(owner_a_infos.len(), 1);
    assert_eq!(owner_a_infos[0].database_id, "alpha");

    let owner_b_ids = service
        .list_database_infos_for_caller("owner_b")
        .expect("owner_b infos should load")
        .into_iter()
        .map(|info| info.database_id)
        .collect::<Vec<_>>();
    assert_eq!(owner_b_ids, vec!["alpha".to_string(), "beta".to_string()]);

    let outsider_infos = service
        .list_database_infos_for_caller("outsider")
        .expect("outsider infos should load");
    assert!(outsider_infos.is_empty());
}

#[test]
fn discards_failed_database_reservation_for_retry() {
    let (service, root) = service_with_root();
    service
        .reserve_database("retryable", "owner", 1)
        .expect("reservation should create");
    assert_eq!(database_member_count(&root, "retryable"), 1);

    service
        .discard_database_reservation("retryable")
        .expect("reservation should discard");
    assert_eq!(database_member_count(&root, "retryable"), 0);

    let meta = service
        .create_database("retryable", "owner", 2)
        .expect("same database_id should create after discard");
    assert_eq!(meta.database_id, "retryable");
    assert_eq!(database_member_count(&root, "retryable"), 1);
}

#[test]
fn rejects_invalid_database_ids() {
    let service = service();

    for database_id in ["", "../escape", "has/slash", "has.dot", "has space"] {
        let error = service
            .create_database(database_id, "owner", 1)
            .expect_err("invalid database_id should be rejected");
        assert!(
            error.contains("database_id"),
            "error should mention database_id for {database_id:?}: {error}"
        );
    }

    let too_long = "a".repeat(65);
    let error = service
        .create_database(&too_long, "owner", 1)
        .expect_err("too long database_id should be rejected");
    assert!(error.contains("1..64"));
}

#[test]
fn rejects_database_creation_after_mount_capacity() {
    let (service, root) = service_with_root();
    let conn = Connection::open(root.join("index.sqlite3")).expect("index should open");

    for mount_id in 11..32767 {
        conn.execute(
            "INSERT INTO databases
             (database_id, db_file_name, mount_id, active_mount_id, status, schema_version,
              logical_size_bytes, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?3, 'hot', 'vfs_store:current', 0, 1, 1)",
            params![
                format!("reserved_{mount_id}"),
                format!("reserved_{mount_id}.sqlite3"),
                i64::from(mount_id)
            ],
        )
        .expect("reserved mount_id should insert");
    }

    let meta = service
        .create_database("db_32767", "owner", 32767)
        .expect("last mount_id should create");
    assert_eq!(meta.mount_id, 32767);

    let error = service
        .create_database("db_32768", "owner", 32768)
        .expect_err("next database should exceed mount capacity");
    assert_eq!(error, "database mount_id capacity exhausted");
}

#[test]
fn isolates_nodes_between_databases() {
    let service = service();
    service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    service
        .create_database("beta", "owner", 2)
        .expect("beta should create");

    for database_id in ["alpha", "beta"] {
        service
            .write_node(
                "owner",
                WriteNodeRequest {
                    database_id: database_id.to_string(),
                    path: "/Wiki/shared.md".to_string(),
                    kind: NodeKind::File,
                    content: format!("{database_id} body"),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                10,
            )
            .expect("write should succeed");
    }

    let alpha = service
        .read_node("alpha", "owner", "/Wiki/shared.md")
        .expect("alpha read should succeed")
        .expect("alpha node should exist");
    let beta_hits = service
        .search_nodes(
            "owner",
            SearchNodesRequest {
                database_id: "beta".to_string(),
                query_text: "alpha".to_string(),
                prefix: Some("/Wiki".to_string()),
                top_k: 10,
                preview_mode: Some(SearchPreviewMode::None),
            },
        )
        .expect("beta search should succeed");

    assert_eq!(alpha.content, "alpha body");
    assert!(beta_hits.is_empty());
}

#[test]
fn tracks_logical_size_and_reuses_deleted_slots() {
    let (service, root) = service_with_root();
    let alpha = service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    service
        .write_node(
            "owner",
            WriteNodeRequest {
                database_id: "alpha".to_string(),
                path: "/Wiki/a.md".to_string(),
                kind: NodeKind::File,
                content: "alpha body".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            2,
        )
        .expect("write should succeed");

    let alpha_info = service
        .list_database_infos()
        .expect("infos should load")
        .into_iter()
        .find(|info| info.database_id == "alpha")
        .expect("alpha info should exist");
    assert_eq!(alpha_info.status, DatabaseStatus::Hot);
    assert!(alpha_info.logical_size_bytes > 0);

    service
        .delete_database("alpha", "owner", 3)
        .expect("delete should succeed");
    assert_restore_size(&root, "alpha", None);
    assert!(
        service
            .read_node("alpha", "owner", "/Wiki/a.md")
            .expect_err("deleted DB should reject reads")
            .contains("database not found")
    );

    let beta = service
        .create_database("beta", "owner", 4)
        .expect("beta should reuse freed slot");
    assert_eq!(beta.mount_id, alpha.mount_id);
}

#[test]
fn archives_and_restores_database_bytes() {
    let (service, root) = service_with_root();
    service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    service
        .write_node(
            "owner",
            WriteNodeRequest {
                database_id: "alpha".to_string(),
                path: "/Wiki/a.md".to_string(),
                kind: NodeKind::File,
                content: "alpha body".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            2,
        )
        .expect("write should succeed");

    let archive = service
        .begin_database_archive("alpha", "owner")
        .expect("archive should begin");
    assert!(archive.size_bytes > 0);
    let bytes = read_archive_in_chunks(&service, "alpha", archive.size_bytes, 17);
    assert_eq!(bytes.len() as u64, archive.size_bytes);
    assert_eq!(
        archive_bytes_for_chunk_size(&service, "alpha", archive.size_bytes, 64 * 1024),
        bytes
    );
    assert_eq!(
        archive_bytes_for_chunk_size(
            &service,
            "alpha",
            archive.size_bytes,
            archive.size_bytes as u32 + 1
        ),
        bytes
    );
    assert!(
        service
            .read_database_archive_chunk("alpha", "owner", 0, 0)
            .expect("zero-byte archive chunk should read")
            .is_empty()
    );
    assert!(
        service
            .read_database_archive_chunk("alpha", "owner", archive.size_bytes, 17)
            .expect("tail archive chunk should read")
            .is_empty()
    );
    assert!(
        service
            .read_database_archive_chunk("alpha", "owner", archive.size_bytes + 10, 17)
            .expect("out-of-range archive chunk should read")
            .is_empty()
    );
    let full_chunk = service
        .read_database_archive_chunk("alpha", "owner", 0, archive.size_bytes as u32)
        .expect("archive chunk should read");
    assert_eq!(full_chunk, bytes);
    service
        .finalize_database_archive("alpha", "owner", vec![1, 2, 3], 3)
        .expect("archive should finalize");
    assert_eq!(
        database_index_row(&root, "alpha"),
        ("archived".to_string(), None, archive.size_bytes, None)
    );
    assert!(
        service
            .read_node("alpha", "owner", "/Wiki/a.md")
            .expect_err("archived DB should reject reads")
            .contains("database not found")
    );

    service
        .begin_database_restore("alpha", "owner", vec![1, 2, 3], archive.size_bytes, 4)
        .expect("restore should begin");
    let restoring = database_index_row(&root, "alpha");
    assert_eq!(restoring.0, "restoring");
    assert!(restoring.1.is_some());
    assert_eq!(restoring.2, archive.size_bytes);
    assert_eq!(restoring.3, Some(archive.size_bytes));
    let error = service
        .begin_database_restore("alpha", "owner", vec![1, 2, 3], archive.size_bytes, 5)
        .expect_err("restoring DB should reject restore begin");
    assert!(error.contains("database restore can only begin"));
    assert_eq!(
        service
            .list_database_infos()
            .expect("infos should load")
            .into_iter()
            .find(|info| info.database_id == "alpha")
            .expect("alpha info should exist")
            .status,
        DatabaseStatus::Restoring
    );
    assert!(
        service
            .read_node("alpha", "owner", "/Wiki/a.md")
            .expect_err("restoring DB should reject reads")
            .contains("database not found")
    );
    service
        .write_database_restore_chunk("alpha", "owner", 0, &bytes)
        .expect("restore chunk should write");
    service
        .finalize_database_restore("alpha", "owner", 5)
        .expect("restore should finalize");

    let node = service
        .read_node("alpha", "owner", "/Wiki/a.md")
        .expect("restored read should succeed")
        .expect("restored node should exist");
    assert_eq!(node.content, "alpha body");
    let info = service
        .list_database_infos()
        .expect("infos should load")
        .into_iter()
        .find(|info| info.database_id == "alpha")
        .expect("alpha info should exist");
    assert_eq!(info.status, DatabaseStatus::Hot);
    assert_eq!(info.snapshot_hash, Some(vec![1, 2, 3]));
    assert_eq!(info.archived_at_ms, None);
    assert_eq!(info.deleted_at_ms, None);
    assert_restore_size(&root, "alpha", None);
    assert_eq!(
        database_index_row(&root, "alpha").1,
        Some(restoring.1.unwrap())
    );
}

#[test]
fn restore_finalize_rejects_size_mismatch_until_missing_bytes_arrive() {
    let (service, root) = service_with_root();
    service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    service
        .write_node(
            "owner",
            WriteNodeRequest {
                database_id: "alpha".to_string(),
                path: "/Wiki/a.md".to_string(),
                kind: NodeKind::File,
                content: "alpha body".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            2,
        )
        .expect("write should succeed");

    let archive = service
        .begin_database_archive("alpha", "owner")
        .expect("archive should begin");
    let bytes = service
        .read_database_archive_chunk("alpha", "owner", 0, archive.size_bytes as u32)
        .expect("archive chunk should read");
    service
        .finalize_database_archive("alpha", "owner", vec![1, 2, 3], 3)
        .expect("archive should finalize");
    assert_restore_size(&root, "alpha", None);

    service
        .begin_database_restore("alpha", "owner", vec![1, 2, 3], archive.size_bytes, 4)
        .expect("restore should begin");
    assert_restore_size(&root, "alpha", Some(archive.size_bytes));
    let overflow_error = service
        .write_database_restore_chunk("alpha", "owner", archive.size_bytes, &[0])
        .expect_err("restore chunk past declared size should fail");
    assert!(overflow_error.contains("restore chunk exceeds expected size"));

    let split_at = bytes.len() / 2;
    service
        .write_database_restore_chunk("alpha", "owner", 0, &bytes[..split_at])
        .expect("first restore chunk should write");
    let error = service
        .finalize_database_restore("alpha", "owner", 5)
        .expect_err("short restore should fail");
    assert!(error.contains("restore chunks are incomplete"));
    assert_eq!(
        service
            .list_database_infos()
            .expect("infos should load")
            .into_iter()
            .find(|info| info.database_id == "alpha")
            .expect("alpha info should exist")
            .status,
        DatabaseStatus::Restoring
    );

    service
        .write_database_restore_chunk("alpha", "owner", split_at as u64, &bytes[split_at..])
        .expect("second restore chunk should write");
    service
        .finalize_database_restore("alpha", "owner", 6)
        .expect("complete restore should finalize");
    assert_restore_size(&root, "alpha", None);
    let node = service
        .read_node("alpha", "owner", "/Wiki/a.md")
        .expect("restored read should succeed")
        .expect("restored node should exist");
    assert_eq!(node.content, "alpha body");
}

#[test]
fn restore_accepts_in_range_chunks_written_out_of_order() {
    let (service, _root) = service_with_root();
    service
        .create_database("alpha", "owner", 1)
        .expect("alpha should create");
    service
        .write_node(
            "owner",
            WriteNodeRequest {
                database_id: "alpha".to_string(),
                path: "/Wiki/a.md".to_string(),
                kind: NodeKind::File,
                content: "alpha body".repeat(100),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            2,
        )
        .expect("write should succeed");

    let archive = service
        .begin_database_archive("alpha", "owner")
        .expect("archive should begin");
    let bytes = service
        .read_database_archive_chunk("alpha", "owner", 0, archive.size_bytes as u32)
        .expect("archive chunk should read");
    service
        .finalize_database_archive("alpha", "owner", vec![9, 9, 9], 3)
        .expect("archive should finalize");
    service
        .begin_database_restore("alpha", "owner", vec![8, 8, 8], archive.size_bytes, 4)
        .expect("restore should begin");

    let split_at = bytes.len() / 2;
    service
        .write_database_restore_chunk("alpha", "owner", split_at as u64, &bytes[split_at..])
        .expect("second half should write first");
    service
        .write_database_restore_chunk("alpha", "owner", 0, &bytes[..split_at])
        .expect("first half should write second");
    service
        .finalize_database_restore("alpha", "owner", 5)
        .expect("out-of-order restore should finalize");

    let node = service
        .read_node("alpha", "owner", "/Wiki/a.md")
        .expect("restored read should succeed")
        .expect("restored node should exist");
    assert_eq!(node.content, "alpha body".repeat(100));
    let info = service
        .list_database_infos()
        .expect("infos should load")
        .into_iter()
        .find(|info| info.database_id == "alpha")
        .expect("alpha info should exist");
    assert_eq!(info.snapshot_hash, Some(vec![8, 8, 8]));
}

#[test]
fn enforces_reader_writer_owner_roles() {
    let service = service();
    service
        .create_database("shared", "owner", 1)
        .expect("database should create");
    service
        .grant_database_access("shared", "owner", "reader", DatabaseRole::Reader, 2)
        .expect("reader grant should succeed");
    service
        .grant_database_access("shared", "owner", "writer", DatabaseRole::Writer, 3)
        .expect("writer grant should succeed");

    assert!(
        service
            .read_node("shared", "reader", "/Wiki/missing.md")
            .expect("reader read should be authorized")
            .is_none()
    );
    assert!(
        service
            .write_node(
                "reader",
                WriteNodeRequest {
                    database_id: "shared".to_string(),
                    path: "/Wiki/nope.md".to_string(),
                    kind: NodeKind::File,
                    content: "nope".to_string(),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                10,
            )
            .is_err()
    );
    service
        .write_node(
            "writer",
            WriteNodeRequest {
                database_id: "shared".to_string(),
                path: "/Wiki/ok.md".to_string(),
                kind: NodeKind::File,
                content: "ok".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            11,
        )
        .expect("writer write should succeed");
    assert!(
        service
            .grant_database_access("shared", "writer", "other", DatabaseRole::Reader, 12)
            .is_err()
    );
    assert!(
        service
            .grant_database_access("shared", "owner", "owner", DatabaseRole::Reader, 13)
            .expect_err("owner should not downgrade own access")
            .contains("downgrade own access")
    );
    service
        .grant_database_access("shared", "owner", "owner", DatabaseRole::Owner, 14)
        .expect("owner should be allowed to keep own owner access");
    assert!(
        service
            .list_database_members("shared", "writer")
            .expect_err("writer should not list members")
            .contains("lacks required database role")
    );

    let members = service
        .list_database_members("shared", "owner")
        .expect("owner should list members");
    assert_eq!(members.len(), 3);

    service
        .revoke_database_access("shared", "owner", "reader")
        .expect("owner should revoke reader");
    assert!(
        service
            .read_node("shared", "reader", "/Wiki/missing.md")
            .expect_err("revoked reader should lose access")
            .contains("no access")
    );
    assert!(
        service
            .revoke_database_access("shared", "owner", "owner")
            .expect_err("owner should not revoke own access")
            .contains("own access")
    );
}
