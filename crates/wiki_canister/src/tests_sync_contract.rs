// Where: crates/wiki_canister/src/tests_sync_contract.rs
// What: Additional entry-point tests for search and sync scope behavior.
// Why: The VFS validation phase needs API-boundary coverage for deleted visibility and prefix scoping.
use tempfile::tempdir;
use wiki_runtime::WikiService;
use wiki_types::{
    DeleteNodeRequest, ExportSnapshotRequest, FetchUpdatesRequest, NodeKind,
    SearchNodePathsRequest, SearchNodesRequest, WriteNodeRequest,
};

use super::{
    SERVICE, delete_node, export_snapshot, fetch_updates, search_node_paths, search_nodes,
    write_node,
};

fn install_test_service() {
    let dir = tempdir().expect("tempdir should create");
    let db_path = dir.keep().join("wiki.sqlite3");
    let service = WikiService::new(db_path);
    service
        .run_fs_migrations()
        .expect("fs migrations should run");
    SERVICE.with(|slot| *slot.borrow_mut() = Some(service));
}

#[test]
fn canister_search_respects_prefix_and_hides_deleted_nodes() {
    install_test_service();

    let alpha = write_node(WriteNodeRequest {
        path: "/Wiki/project-alpha/one.md".to_string(),
        kind: NodeKind::File,
        content: "alpha shared term".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("alpha write should succeed");
    write_node(WriteNodeRequest {
        path: "/Wiki/project-beta/two.md".to_string(),
        kind: NodeKind::File,
        content: "beta shared term".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("beta write should succeed");

    delete_node(DeleteNodeRequest {
        path: "/Wiki/project-alpha/one.md".to_string(),
        expected_etag: Some(alpha.node.etag),
    })
    .expect("delete should succeed");

    let hits = search_nodes(SearchNodesRequest {
        query_text: "shared".to_string(),
        prefix: Some("/Wiki/project-alpha".to_string()),
        top_k: 10,
    })
    .expect("search should succeed");
    assert!(hits.is_empty());

    let beta_hits = search_nodes(SearchNodesRequest {
        query_text: "shared".to_string(),
        prefix: Some("/Wiki/project-beta".to_string()),
        top_k: 10,
    })
    .expect("search should succeed");
    assert_eq!(beta_hits.len(), 1);
    assert_eq!(beta_hits[0].path, "/Wiki/project-beta/two.md");

    let path_hits = search_node_paths(SearchNodePathsRequest {
        query_text: "PROJECT-beta".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
    })
    .expect("path search should succeed");
    assert_eq!(path_hits.len(), 1);
    assert_eq!(path_hits[0].path, "/Wiki/project-beta/two.md");
}

#[test]
fn canister_fetch_updates_changes_with_deleted_visibility() {
    install_test_service();

    let created = write_node(WriteNodeRequest {
        path: "/Wiki/scope/item.md".to_string(),
        kind: NodeKind::File,
        content: "scope body".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("write should succeed");

    let without_deleted = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki/scope".to_string()),
        include_deleted: false,
    })
    .expect("snapshot should succeed");
    let with_deleted = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki/scope".to_string()),
        include_deleted: true,
    })
    .expect("snapshot should succeed");

    delete_node(DeleteNodeRequest {
        path: "/Wiki/scope/item.md".to_string(),
        expected_etag: Some(created.node.etag),
    })
    .expect("delete should succeed");

    let hidden = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: without_deleted.snapshot_revision,
        prefix: Some("/Wiki/scope".to_string()),
        include_deleted: false,
    })
    .expect("updates should succeed");
    assert!(hidden.changed_nodes.is_empty());
    assert_eq!(
        hidden.removed_paths,
        vec!["/Wiki/scope/item.md".to_string()]
    );

    let visible = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: with_deleted.snapshot_revision,
        prefix: Some("/Wiki/scope".to_string()),
        include_deleted: true,
    })
    .expect("updates should succeed");
    assert_eq!(visible.changed_nodes.len(), 1);
    assert_eq!(visible.changed_nodes[0].path, "/Wiki/scope/item.md");
    assert_eq!(visible.changed_nodes[0].deleted_at, Some(1_700_000_000_000));
    assert!(visible.removed_paths.is_empty());
}

#[test]
fn canister_fetch_updates_full_refreshes_when_prefix_scope_changes() {
    install_test_service();

    write_node(WriteNodeRequest {
        path: "/Wiki/a/one.md".to_string(),
        kind: NodeKind::File,
        content: "alpha".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("write should succeed");
    write_node(WriteNodeRequest {
        path: "/Wiki/b/two.md".to_string(),
        kind: NodeKind::File,
        content: "beta".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("write should succeed");

    let narrow = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki/a".to_string()),
        include_deleted: false,
    })
    .expect("snapshot should succeed");
    let widened = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: narrow.snapshot_revision,
        prefix: Some("/Wiki".to_string()),
        include_deleted: false,
    })
    .expect("updates should succeed");
    assert_eq!(widened.changed_nodes.len(), 2);
    assert!(
        widened
            .changed_nodes
            .iter()
            .any(|node| node.path == "/Wiki/a/one.md")
    );
    assert!(
        widened
            .changed_nodes
            .iter()
            .any(|node| node.path == "/Wiki/b/two.md")
    );
    assert!(widened.removed_paths.is_empty());
}
