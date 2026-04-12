use rusqlite::Connection;
use tempfile::tempdir;
use wiki_store::FsStore;
use wiki_types::{
    DeleteNodeRequest, ExportSnapshotRequest, ListNodesRequest, NodeEntryKind, NodeKind,
    RecentNodesRequest, SearchNodePathsRequest, SearchNodesRequest, WriteNodeRequest,
};

fn new_store() -> (tempfile::TempDir, FsStore) {
    let dir = tempdir().expect("temp dir should exist");
    let store = FsStore::new(dir.path().join("wiki.sqlite3"));
    store
        .run_fs_migrations()
        .expect("fs migrations should succeed");
    (dir, store)
}

fn write_file(store: &FsStore, path: &str, expected_etag: Option<&str>, now: i64) -> String {
    store
        .write_node(
            WriteNodeRequest {
                path: path.to_string(),
                kind: NodeKind::File,
                content: format!("content revision {now}"),
                metadata_json: "{}".to_string(),
                expected_etag: expected_etag.map(str::to_string),
            },
            now,
        )
        .expect("write should succeed")
        .node
        .etag
}

#[test]
fn fs_migrations_create_tables() {
    let (_dir, store) = new_store();
    let conn = Connection::open(store.database_path()).expect("db should open");
    let tables = [
        "fs_nodes",
        "fs_nodes_fts",
        "fs_change_log",
        "schema_migrations",
    ];
    for table in tables {
        let exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE name = ?1 LIMIT 1",
                [table],
                |row| row.get::<_, i64>(0),
            )
            .expect("table lookup should succeed");
        assert_eq!(exists, 1);
    }

    let fs_nodes_columns: Vec<(String, String, i64)> = conn
        .prepare("PRAGMA table_info(fs_nodes)")
        .expect("pragma should prepare")
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?, row.get(5)?)))
        .expect("pragma should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("pragma rows should collect");
    assert!(
        fs_nodes_columns.iter().any(|(name, ty, pk)| {
            name == "id" && ty.eq_ignore_ascii_case("INTEGER") && *pk == 1
        })
    );
    assert!(fs_nodes_columns.iter().any(|(name, _, _)| name == "path"));

    let fts_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'fs_nodes_fts'",
            [],
            |row| row.get(0),
        )
        .expect("fts sql lookup should succeed");
    assert!(fts_sql.contains("content='fs_nodes'"));
    assert!(fts_sql.contains("content_rowid='id'"));

    let versions: Vec<String> = conn
        .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
        .expect("version query should prepare")
        .query_map([], |row| row.get(0))
        .expect("version query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("versions should collect");
    assert_eq!(versions, vec!["wiki_store:000_fs_schema".to_string()]);

    for index in [
        "fs_nodes_visible_path_covering_idx",
        "fs_nodes_path_covering_idx",
        "fs_nodes_visible_recent_covering_idx",
        "fs_nodes_recent_covering_idx",
    ] {
        let exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1 LIMIT 1",
                [index],
                |row| row.get::<_, i64>(0),
            )
            .expect("index lookup should succeed");
        assert_eq!(exists, 1);
    }
}

#[test]
fn list_and_recent_queries_use_covering_indexes() {
    let (_dir, store) = new_store();
    write_file(&store, "/Wiki/indexed.md", None, 10);
    let conn = Connection::open(store.database_path()).expect("db should open");

    let list_plan = explain_query_plan(
        &conn,
        "SELECT path, kind, updated_at, etag, deleted_at
         FROM fs_nodes
         WHERE deleted_at IS NULL
           AND (path = ?1 OR path LIKE ?2)
         ORDER BY path ASC",
        ["/Wiki", "/Wiki/%"],
    );
    assert!(
        list_plan.contains("COVERING INDEX fs_nodes_visible_path_covering_idx"),
        "list should avoid table lookups: {list_plan}"
    );

    let recent_plan = explain_query_plan(
        &conn,
        "SELECT path, kind, updated_at, etag, deleted_at
         FROM fs_nodes
         WHERE deleted_at IS NULL
           AND (path = ?1 OR path LIKE ?2)
         ORDER BY updated_at DESC, path ASC
         LIMIT 10",
        ["/Wiki", "/Wiki/%"],
    );
    assert!(
        recent_plan.contains("COVERING INDEX fs_nodes_visible_recent_covering_idx"),
        "recent should avoid table lookups: {recent_plan}"
    );
}

fn explain_query_plan(conn: &Connection, sql: &str, params: [&str; 2]) -> String {
    conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .expect("explain should prepare")
        .query_map(params, |row| row.get::<_, String>(3))
        .expect("explain should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("explain rows should collect")
        .join("\n")
}

#[test]
fn status_counts_files_sources_and_tombstones() {
    let (_dir, store) = new_store();
    let file = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/file.md".to_string(),
                kind: NodeKind::File,
                content: "alpha".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            10,
        )
        .expect("file write should succeed");
    let source = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/source.txt".to_string(),
                kind: NodeKind::Source,
                content: "source".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            11,
        )
        .expect("source write should succeed");
    store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/file.md".to_string(),
                expected_etag: Some(file.node.etag),
            },
            12,
        )
        .expect("delete should succeed");

    let status = store.status().expect("status should succeed");
    assert_eq!(status.file_count, 0);
    assert_eq!(status.source_count, 1);
    assert_eq!(status.deleted_count, 1);
    assert_eq!(source.node.kind, NodeKind::Source);
}

#[test]
fn write_update_delete_and_revive_follow_etag_rules() {
    let (_dir, store) = new_store();
    let first = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                kind: NodeKind::File,
                content: "alpha".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            },
            10,
        )
        .expect("first write should succeed");
    assert!(first.created);
    assert_eq!(
        store
            .read_node("/Wiki/foo.md")
            .expect("read should succeed"),
        Some(wiki_types::Node {
            path: first.node.path.clone(),
            kind: first.node.kind.clone(),
            content: "alpha".to_string(),
            created_at: 10,
            updated_at: 10,
            etag: first.node.etag.clone(),
            deleted_at: None,
            metadata_json: "{}".to_string(),
        })
    );

    let stale_error = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                kind: NodeKind::File,
                content: "beta".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: Some("stale".to_string()),
            },
            11,
        )
        .expect_err("stale write should fail");
    assert!(stale_error.contains("expected_etag"));

    let second = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                kind: NodeKind::File,
                content: "beta".to_string(),
                metadata_json: "{\"v\":2}".to_string(),
                expected_etag: Some(first.node.etag.clone()),
            },
            12,
        )
        .expect("update should succeed");
    assert!(!second.created);
    assert_ne!(first.node.etag, second.node.etag);
    let second_node = store
        .read_node("/Wiki/foo.md")
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(second_node.created_at, 10);

    let deleted = store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                expected_etag: Some(second.node.etag.clone()),
            },
            13,
        )
        .expect("delete should succeed");
    assert_eq!(deleted.deleted_at, 13);
    let stale_delete = store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                expected_etag: Some(second.node.etag),
            },
            14,
        )
        .expect_err("stale delete should fail");
    assert!(stale_delete.contains("already deleted"));
    assert!(
        store
            .read_node("/Wiki/foo.md")
            .expect("read after delete should succeed")
            .is_none()
    );

    let revive = store
        .write_node(
            WriteNodeRequest {
                path: "/Wiki/foo.md".to_string(),
                kind: NodeKind::File,
                content: "gamma".to_string(),
                metadata_json: "{}".to_string(),
                expected_etag: Some(deleted.etag),
            },
            15,
        )
        .expect("revive should succeed");
    let revived_node = store
        .read_node("/Wiki/foo.md")
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(revived_node.created_at, 10);
    assert_eq!(revive.node.updated_at, 15);
    assert!(revive.node.deleted_at.is_none());
}

#[test]
fn list_search_and_export_respect_deleted_and_prefix() {
    let (_dir, store) = new_store();
    let alpha = write_file(&store, "/Wiki/alpha.md", None, 10);
    let beta = write_file(&store, "/Wiki/nested/beta.md", None, 11);
    write_file(&store, "/Wiki/tree", None, 9);
    write_file(&store, "/Wiki/tree/leaf.md", None, 12);
    write_file(&store, "/Wiki/deleted/leaf.md", None, 13);
    let root_entries = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki".to_string(),
            recursive: false,
            include_deleted: false,
        })
        .expect("root list should succeed");
    assert_eq!(root_entries.len(), 4);
    assert!(
        root_entries
            .iter()
            .any(|entry| entry.path == "/Wiki/alpha.md" && !entry.has_children)
    );
    assert!(root_entries.iter().any(|entry| {
        entry.path == "/Wiki/nested"
            && entry.kind == NodeEntryKind::Directory
            && entry.etag.is_empty()
            && entry.has_children
            && entry.updated_at == 11
    }));
    assert!(root_entries.iter().any(|entry| {
        entry.path == "/Wiki/deleted"
            && entry.kind == NodeEntryKind::Directory
            && entry.etag.is_empty()
            && entry.has_children
            && entry.updated_at == 13
    }));
    assert!(
        root_entries
            .iter()
            .any(|entry| entry.path == "/Wiki/tree" && entry.has_children)
    );

    let nested_entries = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki/nested".to_string(),
            recursive: true,
            include_deleted: false,
        })
        .expect("nested list should succeed");
    assert_eq!(nested_entries.len(), 1);
    assert_eq!(nested_entries[0].path, "/Wiki/nested/beta.md");
    assert_eq!(nested_entries[0].kind, NodeEntryKind::File);

    store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/alpha.md".to_string(),
                expected_etag: Some(alpha),
            },
            12,
        )
        .expect("delete should succeed");
    let deleted_tombstone = store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/deleted/leaf.md".to_string(),
                expected_etag: Some(
                    store
                        .read_node("/Wiki/deleted/leaf.md")
                        .expect("deleted leaf read should succeed")
                        .expect("deleted leaf should exist")
                        .etag,
                ),
            },
            14,
        )
        .expect("deleted leaf tombstone should succeed");
    assert_eq!(deleted_tombstone.deleted_at, 14);
    let visible_after_delete = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki".to_string(),
            recursive: true,
            include_deleted: false,
        })
        .expect("visible list should succeed");
    assert_eq!(visible_after_delete.len(), 3);
    assert!(
        visible_after_delete
            .iter()
            .any(|entry| entry.path == "/Wiki/nested/beta.md")
    );
    assert!(
        visible_after_delete
            .iter()
            .any(|entry| entry.path == "/Wiki/tree")
    );
    assert!(
        visible_after_delete
            .iter()
            .any(|entry| entry.path == "/Wiki/tree/leaf.md")
    );

    let root_after_tombstone = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki".to_string(),
            recursive: false,
            include_deleted: false,
        })
        .expect("root list after tombstone should succeed");
    assert!(
        !root_after_tombstone
            .iter()
            .any(|entry| entry.path == "/Wiki/deleted")
    );

    let deleted_entries = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki".to_string(),
            recursive: true,
            include_deleted: true,
        })
        .expect("deleted list should succeed");
    assert_eq!(deleted_entries.len(), 5);
    assert!(
        deleted_entries
            .iter()
            .any(|entry| entry.path == "/Wiki/alpha.md")
    );
    assert!(
        deleted_entries
            .iter()
            .any(|entry| entry.path == "/Wiki/deleted/leaf.md")
    );

    let deleted_root_entries = store
        .list_nodes(ListNodesRequest {
            prefix: "/Wiki".to_string(),
            recursive: false,
            include_deleted: true,
        })
        .expect("deleted root list should succeed");
    assert!(
        !deleted_root_entries
            .iter()
            .any(|entry| entry.path == "/Wiki/deleted")
    );

    let search_hits = store
        .search_nodes(SearchNodesRequest {
            query_text: "nested".to_string(),
            prefix: Some("/Wiki/nested".to_string()),
            top_k: 5,
        })
        .expect("search should succeed");
    assert!(search_hits.is_empty());

    let path_hits = store
        .search_node_paths(SearchNodePathsRequest {
            query_text: "NeStEd".to_string(),
            prefix: Some("/Wiki".to_string()),
            top_k: 5,
        })
        .expect("path search should succeed");
    assert_eq!(path_hits.len(), 1);
    assert_eq!(path_hits[0].path, "/Wiki/nested/beta.md");
    assert_eq!(path_hits[0].snippet, "/Wiki/nested/beta.md");
    assert_eq!(
        path_hits[0].match_reasons,
        vec!["path_substring".to_string()]
    );

    let missing_hits = store
        .search_nodes(SearchNodesRequest {
            query_text: "alpha".to_string(),
            prefix: Some("/Wiki".to_string()),
            top_k: 5,
        })
        .expect("search should succeed");
    assert!(missing_hits.is_empty());

    let snapshot = store
        .export_snapshot(ExportSnapshotRequest {
            prefix: Some("/Wiki".to_string()),
            include_deleted: true,
        })
        .expect("snapshot should succeed");
    assert_eq!(snapshot.nodes.len(), 5);
    assert!(
        snapshot
            .nodes
            .iter()
            .any(|node| node.path == "/Wiki/alpha.md")
    );
    assert!(
        snapshot
            .nodes
            .iter()
            .any(|node| node.path == "/Wiki/nested/beta.md")
    );
    assert_v4_snapshot_revision_without_state_hash(&snapshot.snapshot_revision);
    assert!(beta.starts_with("v4h:"));
}

fn assert_v4_snapshot_revision_without_state_hash(snapshot_revision: &str) {
    let parts = snapshot_revision.split(':').collect::<Vec<_>>();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], "v4");
    assert!(parts[1].parse::<i64>().expect("revision should parse") >= 0);
    assert!(matches!(parts[2], "0" | "1"));
    assert!(!parts[3].is_empty());
}

#[test]
fn search_nodes_clamps_snippets_from_large_single_token_content() {
    let (_dir, store) = new_store();
    let ascii_content = "x".repeat(1024 * 1024);
    let multibyte_content = "検索".repeat(600);

    for (index, (path, content)) in [
        ("/Wiki/large-ascii.md", ascii_content),
        ("/Wiki/large-multibyte.md", multibyte_content),
    ]
    .into_iter()
    .enumerate()
    {
        store
            .write_node(
                WriteNodeRequest {
                    path: path.to_string(),
                    kind: NodeKind::File,
                    content: content.clone(),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                100 + index as i64,
            )
            .expect("large token write should succeed");

        let hits = store
            .search_nodes(SearchNodesRequest {
                query_text: content,
                prefix: Some("/Wiki".to_string()),
                top_k: 5,
            })
            .expect("large token search should succeed");

        assert!(
            hits.iter().any(|hit| hit.path == path),
            "large token search should return the written node"
        );
        for hit in hits {
            assert!(
                hit.snippet.chars().count() <= 243,
                "snippet should be limited to 240 chars plus ellipsis"
            );
            assert!(
                hit.snippet.len() <= 512,
                "snippet should be limited to 512 utf-8 bytes"
            );
        }
    }
}

#[test]
fn query_limits_are_capped_at_one_hundred() {
    let (_dir, store) = new_store();
    for index in 0..150 {
        store
            .write_node(
                WriteNodeRequest {
                    path: format!("/Wiki/capped/node-{index:03}.md"),
                    kind: NodeKind::File,
                    content: format!("shared-cap-token path-cap-{index}"),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                1_000 + index,
            )
            .expect("write should succeed");
    }

    let recent = store
        .recent_nodes(RecentNodesRequest {
            limit: 1_000,
            path: Some("/Wiki/capped".to_string()),
            include_deleted: false,
        })
        .expect("recent should succeed");
    assert_eq!(recent.len(), 100);

    let search = store
        .search_nodes(SearchNodesRequest {
            query_text: "shared-cap-token".to_string(),
            prefix: Some("/Wiki/capped".to_string()),
            top_k: 1_000,
        })
        .expect("search should succeed");
    assert_eq!(search.len(), 100);

    let path_search = store
        .search_node_paths(SearchNodePathsRequest {
            query_text: "node".to_string(),
            prefix: Some("/Wiki/capped".to_string()),
            top_k: 1_000,
        })
        .expect("path search should succeed");
    assert_eq!(path_search.len(), 100);
}

#[test]
fn search_node_paths_filters_deleted_terms_and_orders_deterministically() {
    let (_dir, store) = new_store();
    let first = write_file(&store, "/Wiki/aaa/nested-note.md", None, 10);
    write_file(&store, "/Wiki/nested-note.md", None, 11);
    write_file(&store, "/Wiki/zzz/nested-note.md", None, 12);

    store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/zzz/nested-note.md".to_string(),
                expected_etag: Some(first),
            },
            13,
        )
        .expect_err("mismatched etag should fail");

    let latest = store
        .read_node("/Wiki/zzz/nested-note.md")
        .expect("read should succeed")
        .expect("node should exist");
    store
        .delete_node(
            DeleteNodeRequest {
                path: "/Wiki/zzz/nested-note.md".to_string(),
                expected_etag: Some(latest.etag),
            },
            14,
        )
        .expect("delete should succeed");

    let hits = store
        .search_node_paths(SearchNodePathsRequest {
            query_text: "NESTED note".to_string(),
            prefix: Some("/Wiki".to_string()),
            top_k: 10,
        })
        .expect("path search should succeed");
    let paths = hits.into_iter().map(|hit| hit.path).collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "/Wiki/nested-note.md".to_string(),
            "/Wiki/aaa/nested-note.md".to_string()
        ]
    );
}
