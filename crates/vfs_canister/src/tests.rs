// Where: crates/vfs_canister/src/tests.rs
// What: Entry-point level tests for the FS-first canister surface.
// Why: Phase 3 replaces the public canister contract, so tests must assert the wrapper behavior directly.
use candid::Principal;
use tempfile::tempdir;
use vfs_runtime::VfsService;
use vfs_types::{
    AppendNodeRequest, DeleteNodeRequest, EditNodeRequest, ExportSnapshotRequest,
    FetchUpdatesRequest, GlobNodeType, GlobNodesRequest, GraphLinksRequest,
    GraphNeighborhoodRequest, IncomingLinksRequest, ListChildrenRequest, ListNodesRequest,
    MkdirNodeRequest, MoveNodeRequest, MultiEdit, MultiEditNodeRequest, NodeContextRequest,
    NodeEntryKind, NodeKind, OutgoingLinksRequest, QueryContextRequest, RecentNodesRequest,
    SearchNodePathsRequest, SearchNodesRequest, SearchPreviewMode, SourceEvidenceRequest,
    WriteNodeRequest,
};

use super::{
    SERVICE, append_node, delete_node, edit_node, enable_path_policy, export_snapshot,
    fetch_updates, glob_nodes, grant_path_policy_role, graph_links, graph_neighborhood,
    incoming_links, list_children, list_nodes, memory_manifest, mkdir_node, move_node,
    multi_edit_node, my_path_policy_roles, outgoing_links, path_policy_entries, query_context,
    read_node, read_node_context, recent_nodes, revoke_path_policy_role, search_node_paths,
    search_nodes, set_test_caller, source_evidence, status, write_node,
};

fn install_test_service() {
    let dir = tempdir().expect("tempdir should create");
    let db_path = dir.keep().join("wiki.sqlite3");
    let service = VfsService::new(db_path);
    service
        .run_fs_migrations()
        .expect("fs migrations should run");
    SERVICE.with(|slot| *slot.borrow_mut() = Some(service));
    set_test_caller(Principal::anonymous());
}

#[test]
fn status_stays_available_after_fs_migrations() {
    install_test_service();

    let current = status();

    assert_eq!(current.file_count, 0);
    assert_eq!(current.source_count, 0);
}

#[test]
fn memory_entrypoints_return_agent_memory_contract() {
    install_test_service();

    let manifest = memory_manifest();
    assert_eq!(manifest.api_version, "agent-memory-v1");
    assert_eq!(manifest.write_policy, "agent_memory_read_only");
    assert_eq!(manifest.recommended_entrypoint, "query_context");
    assert_eq!(manifest.max_depth, 2);
    assert!(manifest.roots.iter().any(|root| root.path == "/Wiki"));

    for (path, content) in [
        ("/Wiki/scope/index.md", "# Index\n\n[Overview](overview.md)"),
        (
            "/Wiki/scope/overview.md",
            "# Overview\n\nbeam memory [Raw](/Sources/raw/a/a.md)",
        ),
        ("/Wiki/scope/schema.md", "# Schema\n\nread-only"),
        (
            "/Wiki/scope/provenance.md",
            "# Provenance\n\n[Raw](/Sources/raw/a/a.md)",
        ),
        ("/Sources/raw/a/a.md", "raw source"),
    ] {
        write_node(WriteNodeRequest {
            path: path.to_string(),
            kind: if path.starts_with("/Sources/") {
                NodeKind::Source
            } else {
                NodeKind::File
            },
            content: content.to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .expect("write should succeed");
    }

    let context = query_context(QueryContextRequest {
        task: "beam memory".to_string(),
        entities: Vec::new(),
        namespace: Some("/Wiki/scope".to_string()),
        budget_tokens: 1_000,
        include_evidence: true,
        depth: 1,
    })
    .expect("query context should load");
    assert!(
        context
            .nodes
            .iter()
            .any(|node| node.node.path == "/Wiki/scope/overview.md")
    );
    assert!(!context.evidence.is_empty());

    let evidence = source_evidence(SourceEvidenceRequest {
        node_path: "/Wiki/scope/overview.md".to_string(),
    })
    .expect("evidence should load");
    assert!(
        evidence
            .refs
            .iter()
            .any(|item| item.source_path == "/Sources/raw/a/a.md")
    );
}

#[test]
fn fs_entrypoints_cover_crud_search_and_sync() {
    install_test_service();

    let created = write_node(WriteNodeRequest {
        path: "/Wiki/foo.md".to_string(),
        kind: NodeKind::File,
        content: "# Foo\n\nalpha body".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("write should succeed");
    assert!(created.created);

    write_node(WriteNodeRequest {
        path: "/Wiki/nested/bar.md".to_string(),
        kind: NodeKind::File,
        content: "# Bar\n\nbeta body".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("nested write should succeed");

    let node = read_node("/Wiki/foo.md".to_string())
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(node.kind, NodeKind::File);

    let stale_write = write_node(WriteNodeRequest {
        path: "/Wiki/foo.md".to_string(),
        kind: NodeKind::File,
        content: "# Foo\n\nrewrite".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: Some("stale".to_string()),
    });
    assert!(stale_write.is_err());

    let entries = list_nodes(ListNodesRequest {
        prefix: "/Wiki".to_string(),
        recursive: false,
    })
    .expect("list should succeed");
    assert!(
        entries.iter().any(|entry| {
            entry.path == "/Wiki/nested" && entry.kind == NodeEntryKind::Directory
        })
    );

    let children = list_children(ListChildrenRequest {
        path: "/Wiki".to_string(),
    })
    .expect("children should list");
    assert!(children.iter().any(|child| {
        child.path == "/Wiki/nested" && child.kind == NodeEntryKind::Directory && child.is_virtual
    }));
    assert!(children.iter().any(|child| {
        child.path == "/Wiki/foo.md"
            && child.kind == NodeEntryKind::File
            && child.etag.as_deref() == Some(created.node.etag.as_str())
    }));

    let hits = search_nodes(SearchNodesRequest {
        query_text: "alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 5,
        preview_mode: Some(SearchPreviewMode::None),
    })
    .expect("search should succeed");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "/Wiki/foo.md");

    let path_hits = search_node_paths(SearchNodePathsRequest {
        query_text: "NeStEd".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 5,
        preview_mode: None,
    })
    .expect("path search should succeed");
    assert_eq!(path_hits.len(), 1);
    assert_eq!(path_hits[0].path, "/Wiki/nested/bar.md");

    let snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("snapshot should export");
    assert_eq!(snapshot.nodes.len(), 2);

    let empty_delta = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: snapshot.snapshot_revision.clone(),
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        target_snapshot_revision: None,
    })
    .expect("matching snapshot should produce empty delta");
    assert!(empty_delta.changed_nodes.is_empty());
    assert!(empty_delta.removed_paths.is_empty());

    let invalid_delta = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: "missing".to_string(),
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        target_snapshot_revision: None,
    });
    assert_eq!(
        invalid_delta.expect_err("unknown snapshot should fail"),
        "known_snapshot_revision is invalid"
    );

    let deleted = delete_node(DeleteNodeRequest {
        path: "/Wiki/foo.md".to_string(),
        expected_etag: Some(created.node.etag.clone()),
    })
    .expect("delete should succeed");
    assert_eq!(deleted.path, "/Wiki/foo.md");

    let deleted_read = read_node("/Wiki/foo.md".to_string()).expect("read should succeed");
    assert!(deleted_read.is_none());

    let stale_delete = delete_node(DeleteNodeRequest {
        path: "/Wiki/nested/bar.md".to_string(),
        expected_etag: Some("stale".to_string()),
    });
    assert!(stale_delete.is_err());
}

#[test]
fn path_policy_entries_restricts_reads_and_writes_by_role() {
    install_test_service();
    let admin = Principal::from_text("aaaaa-aa").expect("principal should parse");
    let viewer = Principal::from_text("2vxsx-fae").expect("principal should parse");
    let publisher =
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("principal should parse");

    set_test_caller(admin);
    enable_path_policy("/Wiki/skills".to_string()).expect("policy should enable");
    assert_eq!(
        my_path_policy_roles("/Wiki/skills".to_string()),
        vec!["Admin".to_string()]
    );
    grant_path_policy_role(
        "/Wiki/skills".to_string(),
        viewer.to_text(),
        "Reader".to_string(),
    )
    .expect("grant viewer");
    enable_path_policy("/Wiki/protected".to_string()).expect("protected policy should enable");
    grant_path_policy_role(
        "/Wiki/protected".to_string(),
        viewer.to_text(),
        "Reader".to_string(),
    )
    .expect("grant protected reader");
    grant_path_policy_role(
        "/Wiki/skills".to_string(),
        publisher.to_text(),
        "Writer".to_string(),
    )
    .expect("grant publisher");

    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
        kind: NodeKind::File,
        content: "# Skill".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("admin can publish");

    set_test_caller(viewer);
    assert!(
        read_node("/Wiki/skills/acme/legal-review/SKILL.md".to_string())
            .expect("viewer can read")
            .is_some()
    );
    assert!(
        write_node(WriteNodeRequest {
            path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
            kind: NodeKind::File,
            content: "# Changed".to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .expect_err("viewer cannot publish")
        .contains("Writer")
    );

    set_test_caller(publisher);
    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/legal-review/provenance.md".to_string(),
        kind: NodeKind::File,
        content: "# Provenance".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("publisher can publish");
    assert!(
        path_policy_entries("/Wiki/skills".to_string())
            .expect_err("publisher cannot manage policy")
            .contains("Admin")
    );

    revoke_path_policy_role(
        "/Wiki/skills".to_string(),
        viewer.to_text(),
        "Reader".to_string(),
    )
    .expect_err("publisher cannot revoke");
}

#[test]
fn public_skill_catalog_write_uses_public_path_policy() {
    install_test_service();
    let private_admin = Principal::from_text("aaaaa-aa").expect("principal should parse");
    let public_admin =
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("principal should parse");
    let public_writer =
        Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").expect("principal should parse");
    let outsider = Principal::from_text("2vxsx-fae").expect("principal should parse");

    set_test_caller(private_admin);
    enable_path_policy("/Wiki/skills".to_string()).expect("policy should enable");
    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/private/SKILL.md".to_string(),
        kind: NodeKind::File,
        content: "# Private Skill\n\nprivate-alpha".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("private admin can publish private skill");

    set_test_caller(public_admin);
    enable_path_policy("/Wiki/public-skills".to_string()).expect("public policy should enable");
    grant_path_policy_role(
        "/Wiki/public-skills".to_string(),
        public_writer.to_text(),
        "Writer".to_string(),
    )
    .expect("grant public writer");

    set_test_caller(private_admin);
    assert!(
        read_node("/Wiki/public-skills/acme/legal-review/SKILL.md".to_string())
            .expect_err("private admin cannot read restricted public catalog")
            .contains("Reader")
    );
    assert!(
        write_node(WriteNodeRequest {
            path: "/Wiki/public-skills/acme/legal-review/SKILL.md".to_string(),
            kind: NodeKind::File,
            content: "# Private Admin Public Write".to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .expect_err("private admin cannot write public catalog")
        .contains("Writer")
    );

    set_test_caller(public_writer);
    write_node(WriteNodeRequest {
        path: "/Wiki/public-skills/acme/legal-review/SKILL.md".to_string(),
        kind: NodeKind::File,
        content: "# Public Skill\n\npublic-alpha".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("public writer can write public catalog");
    assert!(
        read_node("/Wiki/public-skills/acme/legal-review/SKILL.md".to_string())
            .expect("public writer can read public catalog")
            .is_some()
    );

    set_test_caller(outsider);
    assert!(
        write_node(WriteNodeRequest {
            path: "/Wiki/public-skills/acme/legal-review/SKILL.md".to_string(),
            kind: NodeKind::File,
            content: "# Outsider".to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .expect_err("outsider cannot write restricted public catalog")
        .contains("Writer")
    );

    assert!(
        read_node("/Wiki/public-skills/acme/legal-review/SKILL.md".to_string())
            .expect_err("outsider cannot read restricted public catalog")
            .contains("Reader")
    );
    assert!(
        read_node("/Wiki/skills/acme/private/SKILL.md".to_string())
            .expect_err("private skill stays protected")
            .contains("Reader")
    );
    let hits = search_nodes(SearchNodesRequest {
        query_text: "public-alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("restricted public catalog search should succeed");
    assert!(
        hits.iter()
            .all(|hit| { hit.path != "/Wiki/public-skills/acme/legal-review/SKILL.md" })
    );
}

#[test]
fn open_skill_registry_mode_keeps_existing_vfs_behavior() {
    install_test_service();

    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
        kind: NodeKind::File,
        content: "# Open Skill\n\nopen-alpha".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("open mode allows skill writes");
    write_node(WriteNodeRequest {
        path: "/Wiki/public.md".to_string(),
        kind: NodeKind::File,
        content: "# Public\n\nopen-alpha".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("open mode allows public writes");

    let hits = search_nodes(SearchNodesRequest {
        query_text: "open-alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("open mode search should succeed");
    assert!(hits.iter().any(|hit| hit.path.starts_with("/Wiki/skills/")));
    assert!(hits.iter().any(|hit| hit.path == "/Wiki/public.md"));

    let children = list_children(ListChildrenRequest {
        path: "/Wiki".to_string(),
    })
    .expect("open mode parent list should succeed");
    assert!(children.iter().any(|child| child.path == "/Wiki/skills"));
}

#[test]
fn restricted_skill_registry_does_not_leak_to_unauthorized_callers() {
    install_test_service();
    let admin = Principal::from_text("aaaaa-aa").expect("principal should parse");
    let outsider = Principal::from_text("2vxsx-fae").expect("principal should parse");

    set_test_caller(admin);
    enable_path_policy("/Wiki/skills".to_string()).expect("policy should enable");
    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/legal-review/SKILL.md".to_string(),
        kind: NodeKind::File,
        content: "# Secret Skill\n\nalpha-private [Public](/Wiki/public.md) [Raw](/Sources/raw/secret/secret.md)".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("admin can publish");
    write_node(WriteNodeRequest {
        path: "/Wiki/public.md".to_string(),
        kind: NodeKind::File,
        content: "# Public\n\nalpha-public [Secret](/Wiki/skills/acme/legal-review/SKILL.md)"
            .to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("public write remains open");
    write_node(WriteNodeRequest {
        path: "/Sources/raw/secret/secret.md".to_string(),
        kind: NodeKind::Source,
        content: "raw secret".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("source write remains open");

    let snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("admin snapshot should load");

    set_test_caller(outsider);
    assert!(
        read_node("/Wiki/skills/acme/legal-review/SKILL.md".to_string())
            .expect_err("outsider cannot read")
            .contains("Reader")
    );
    assert!(
        read_node("/System/path-policies.json".to_string())
            .expect_err("outsider cannot read policy store")
            .contains("Admin")
    );
    let children = list_children(ListChildrenRequest {
        path: "/Wiki".to_string(),
    })
    .expect("parent list should succeed");
    assert_no_skill_paths(children.iter().map(|child| child.path.as_str()));

    let entries = list_nodes(ListNodesRequest {
        prefix: "/Wiki".to_string(),
        recursive: true,
    })
    .expect("recursive list should succeed");
    assert_no_skill_paths(entries.iter().map(|entry| entry.path.as_str()));

    let hits = search_nodes(SearchNodesRequest {
        query_text: "alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("search should succeed");
    assert_no_skill_paths(hits.iter().map(|hit| hit.path.as_str()));
    assert!(hits.iter().any(|hit| hit.path == "/Wiki/public.md"));

    let path_hits = search_node_paths(SearchNodePathsRequest {
        query_text: "legal-review".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("path search should succeed");
    assert_no_skill_paths(path_hits.iter().map(|hit| hit.path.as_str()));

    let recent = recent_nodes(RecentNodesRequest {
        path: Some("/Wiki".to_string()),
        limit: 10,
    })
    .expect("recent should succeed");
    assert_no_skill_paths(recent.iter().map(|hit| hit.path.as_str()));

    let glob = glob_nodes(GlobNodesRequest {
        path: Some("/Wiki".to_string()),
        pattern: "**/*.md".to_string(),
        node_type: None,
    })
    .expect("glob should succeed");
    assert_no_skill_paths(glob.iter().map(|hit| hit.path.as_str()));

    let links = graph_links(GraphLinksRequest {
        prefix: "/Wiki".to_string(),
        limit: 100,
    })
    .expect("graph links should succeed");
    assert_no_skill_paths(
        links
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
    );

    let neighborhood = graph_neighborhood(GraphNeighborhoodRequest {
        center_path: "/Wiki/public.md".to_string(),
        depth: 1,
        limit: 100,
    })
    .expect("graph neighborhood should succeed");
    assert_no_skill_paths(
        neighborhood
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
    );

    let node_context = read_node_context(NodeContextRequest {
        path: "/Wiki/public.md".to_string(),
        link_limit: 100,
    })
    .expect("node context should succeed")
    .expect("public node should exist");
    assert_no_skill_paths(
        node_context
            .incoming_links
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
    );
    assert_no_skill_paths(
        node_context
            .outgoing_links
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
    );

    let context = query_context(QueryContextRequest {
        task: "alpha".to_string(),
        entities: Vec::new(),
        namespace: Some("/Wiki".to_string()),
        budget_tokens: 1_000,
        include_evidence: true,
        depth: 1,
    })
    .expect("query context should succeed");
    assert_no_skill_paths(context.search_hits.iter().map(|hit| hit.path.as_str()));
    assert_no_skill_paths(context.nodes.iter().map(|node| node.node.path.as_str()));
    assert_no_skill_paths(
        context
            .graph_links
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
    );
    assert_no_skill_paths(
        context
            .evidence
            .iter()
            .map(|evidence| evidence.node_path.as_str()),
    );
    assert_no_skill_paths(context.evidence.iter().flat_map(|evidence| {
        evidence
            .refs
            .iter()
            .flat_map(|item| [item.source_path.as_str(), item.via_path.as_str()])
    }));

    let evidence = source_evidence(SourceEvidenceRequest {
        node_path: "/Wiki/public.md".to_string(),
    })
    .expect("source evidence should succeed");
    assert_no_skill_paths(std::iter::once(evidence.node_path.as_str()));
    assert_no_skill_paths(
        evidence
            .refs
            .iter()
            .flat_map(|item| [item.source_path.as_str(), item.via_path.as_str()]),
    );

    let outsider_snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("snapshot should succeed");
    assert_no_skill_paths(
        outsider_snapshot
            .nodes
            .iter()
            .map(|node| node.path.as_str()),
    );

    let updates = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: snapshot.snapshot_revision,
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        target_snapshot_revision: None,
    })
    .expect("fetch updates should succeed");
    assert_no_skill_paths(updates.changed_nodes.iter().map(|node| node.path.as_str()));
    assert_no_skill_paths(updates.removed_paths.iter().map(String::as_str));
    assert!(
        updates
            .removed_paths
            .iter()
            .all(|path| path != "/System/path-policies.json")
    );
}

#[test]
fn policy_store_admin_does_not_bypass_unrelated_path_policy() {
    install_test_service();
    let team_a_admin = Principal::from_text("aaaaa-aa").expect("principal should parse");
    let team_b_admin =
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("principal should parse");

    set_test_caller(team_a_admin);
    enable_path_policy("/Wiki/team-a".to_string()).expect("team-a policy should enable");
    write_node(WriteNodeRequest {
        path: "/Wiki/team-a/plan.md".to_string(),
        kind: NodeKind::File,
        content: "# Team A\n\nalpha-team-a".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("team-a admin can write");

    set_test_caller(team_b_admin);
    enable_path_policy("/Wiki/team-b".to_string()).expect("team-b policy should enable");
    write_node(WriteNodeRequest {
        path: "/Wiki/team-b/plan.md".to_string(),
        kind: NodeKind::File,
        content: "# Team B\n\nalpha-team-b".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("team-b admin can write");

    set_test_caller(team_a_admin);
    assert!(
        read_node("/System/path-policies.json".to_string())
            .expect("policy store admin can read policy store")
            .is_some()
    );
    assert!(
        read_node("/Wiki/team-b/plan.md".to_string())
            .expect_err("team-a admin cannot read team-b")
            .contains("Reader")
    );
    let hits = search_nodes(SearchNodesRequest {
        query_text: "alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("search should succeed");
    assert!(hits.iter().any(|hit| hit.path == "/Wiki/team-a/plan.md"));
    assert!(!hits.iter().any(|hit| hit.path == "/Wiki/team-b/plan.md"));
    let snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("snapshot should succeed");
    assert!(
        !snapshot
            .nodes
            .iter()
            .any(|node| node.path == "/Wiki/team-b/plan.md")
    );
    let baseline = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("baseline snapshot should succeed");
    let updates = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: baseline.snapshot_revision,
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        target_snapshot_revision: None,
    })
    .expect("updates should succeed");
    assert!(
        !updates
            .changed_nodes
            .iter()
            .any(|node| node.path == "/Wiki/team-b/plan.md")
    );
}

#[test]
fn protected_skill_knowledge_does_not_leak_to_unauthorized_callers() {
    install_test_service();
    let admin = Principal::from_text("aaaaa-aa").expect("principal should parse");
    let viewer = Principal::from_text("2vxsx-fae").expect("principal should parse");
    let outsider =
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("principal should parse");

    set_test_caller(admin);
    enable_path_policy("/Wiki/skills".to_string()).expect("policy should enable");
    enable_path_policy("/Wiki/protected".to_string()).expect("knowledge policy should enable");
    grant_path_policy_role(
        "/Wiki/skills".to_string(),
        viewer.to_text(),
        "Reader".to_string(),
    )
    .expect("grant viewer");
    grant_path_policy_role(
        "/Wiki/protected".to_string(),
        viewer.to_text(),
        "Reader".to_string(),
    )
    .expect("grant viewer knowledge read");
    write_node(WriteNodeRequest {
        path: "/Wiki/skills/acme/legal-review/manifest.md".to_string(),
        kind: NodeKind::File,
        content: "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nknowledge:\n  - /Wiki/protected/contracts.md\npermissions: {}\nprovenance:\n  source: local\n---\n# Manifest\n".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("admin can publish manifest");
    write_node(WriteNodeRequest {
        path: "/Wiki/protected/contracts.md".to_string(),
        kind: NodeKind::File,
        content: "# Contracts\n\nalpha-protected".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("admin can write protected knowledge");
    write_node(WriteNodeRequest {
        path: "/Wiki/public.md".to_string(),
        kind: NodeKind::File,
        content: "# Public\n\nalpha-public [Protected](/Wiki/protected/contracts.md)".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("admin can write public wiki");
    let snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("admin snapshot should load");

    set_test_caller(outsider);
    assert!(
        read_node("/Wiki/protected/contracts.md".to_string())
            .expect_err("outsider cannot read protected knowledge")
            .contains("Reader")
    );
    let search = search_nodes(SearchNodesRequest {
        query_text: "alpha".to_string(),
        prefix: Some("/Wiki".to_string()),
        top_k: 10,
        preview_mode: None,
    })
    .expect("search should succeed");
    assert_no_paths(
        search.iter().map(|hit| hit.path.as_str()),
        "/Wiki/protected",
    );
    assert!(search.iter().any(|hit| hit.path == "/Wiki/public.md"));
    let context = query_context(QueryContextRequest {
        task: "alpha".to_string(),
        entities: Vec::new(),
        namespace: Some("/Wiki".to_string()),
        budget_tokens: 1_000,
        include_evidence: true,
        depth: 1,
    })
    .expect("context should succeed");
    assert_no_paths(
        context.search_hits.iter().map(|hit| hit.path.as_str()),
        "/Wiki/protected",
    );
    assert_no_paths(
        context.nodes.iter().map(|node| node.node.path.as_str()),
        "/Wiki/protected",
    );
    assert_no_paths(
        context
            .graph_links
            .iter()
            .flat_map(|edge| [edge.source_path.as_str(), edge.target_path.as_str()]),
        "/Wiki/protected",
    );
    let outsider_snapshot = export_snapshot(ExportSnapshotRequest {
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        snapshot_revision: None,
        snapshot_session_id: None,
    })
    .expect("snapshot should succeed");
    assert_no_paths(
        outsider_snapshot
            .nodes
            .iter()
            .map(|node| node.path.as_str()),
        "/Wiki/protected",
    );
    let updates = fetch_updates(FetchUpdatesRequest {
        known_snapshot_revision: snapshot.snapshot_revision,
        prefix: Some("/Wiki".to_string()),
        limit: 100,
        cursor: None,
        target_snapshot_revision: None,
    })
    .expect("updates should succeed");
    assert_no_paths(
        updates.changed_nodes.iter().map(|node| node.path.as_str()),
        "/Wiki/protected",
    );

    set_test_caller(viewer);
    assert!(
        read_node("/Wiki/protected/contracts.md".to_string())
            .expect("viewer can read")
            .is_some()
    );
}

fn assert_no_skill_paths<'a>(paths: impl IntoIterator<Item = &'a str>) {
    for path in paths {
        assert!(
            !path.starts_with("/Wiki/skills"),
            "path policy path leaked: {path}"
        );
        assert_ne!(
            path, "/System/path-policies.json",
            "policy store path leaked"
        );
    }
}

fn assert_no_paths<'a>(paths: impl IntoIterator<Item = &'a str>, hidden_prefix: &str) {
    for path in paths {
        assert!(
            !path.starts_with(hidden_prefix),
            "protected path leaked: {path}"
        );
    }
}

#[test]
fn fs_entrypoints_cover_backlink_queries() {
    install_test_service();

    write_node(WriteNodeRequest {
        path: "/Wiki/topic/source.md".to_string(),
        kind: NodeKind::File,
        content: "[Target](../target.md) and [[/Wiki/target.md]]".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("source write should succeed");

    let incoming = incoming_links(IncomingLinksRequest {
        path: "/Wiki/target.md".to_string(),
        limit: 10,
    })
    .expect("incoming links should load");
    assert_eq!(incoming.len(), 2);
    assert!(
        incoming
            .iter()
            .all(|edge| edge.source_path == "/Wiki/topic/source.md")
    );

    let outgoing = outgoing_links(OutgoingLinksRequest {
        path: "/Wiki/topic/source.md".to_string(),
        limit: 10,
    })
    .expect("outgoing links should load");
    assert_eq!(outgoing.len(), 2);

    let graph = graph_links(GraphLinksRequest {
        prefix: "/Wiki/topic".to_string(),
        limit: 10,
    })
    .expect("graph links should load");
    assert_eq!(graph.len(), 2);

    let context = read_node_context(NodeContextRequest {
        path: "/Wiki/topic/source.md".to_string(),
        link_limit: 10,
    })
    .expect("context should load")
    .expect("node should exist");
    assert_eq!(context.node.path, "/Wiki/topic/source.md");
    assert_eq!(context.outgoing_links.len(), 2);

    let neighborhood = graph_neighborhood(GraphNeighborhoodRequest {
        center_path: "/Wiki/target.md".to_string(),
        depth: 1,
        limit: 10,
    })
    .expect("neighborhood should load");
    assert_eq!(neighborhood.len(), 2);
}

#[test]
fn fs_entrypoints_cover_append_edit_and_mkdir() {
    install_test_service();

    let mkdir = mkdir_node(MkdirNodeRequest {
        path: "/Wiki/work".to_string(),
    })
    .expect("mkdir should succeed");
    assert!(mkdir.created);
    assert_eq!(mkdir.path, "/Wiki/work");

    let appended = append_node(AppendNodeRequest {
        path: "/Wiki/work/log.md".to_string(),
        content: "alpha".to_string(),
        expected_etag: None,
        separator: None,
        metadata_json: None,
        kind: None,
    })
    .expect("append create should succeed");
    assert!(appended.created);

    let appended_again = append_node(AppendNodeRequest {
        path: "/Wiki/work/log.md".to_string(),
        content: "beta".to_string(),
        expected_etag: Some(appended.node.etag.clone()),
        separator: Some("\n".to_string()),
        metadata_json: None,
        kind: None,
    })
    .expect("append update should succeed");
    let appended_node = read_node("/Wiki/work/log.md".to_string())
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(appended_node.content, "alpha\nbeta");

    let edited = edit_node(EditNodeRequest {
        path: "/Wiki/work/log.md".to_string(),
        old_text: "beta".to_string(),
        new_text: "gamma".to_string(),
        expected_etag: Some(appended_again.node.etag.clone()),
        replace_all: false,
    })
    .expect("edit should succeed");
    assert_eq!(edited.replacement_count, 1);
    let edited_node = read_node("/Wiki/work/log.md".to_string())
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(edited_node.content, "alpha\ngamma");
}

#[test]
fn fs_entrypoints_reject_noncanonical_source_paths() {
    install_test_service();

    let write_error = write_node(WriteNodeRequest {
        path: "/Sources/raw/source.md".to_string(),
        kind: NodeKind::Source,
        content: "source".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect_err("noncanonical source write should fail");
    assert!(write_error.contains("source path must"));

    write_node(WriteNodeRequest {
        path: "/Sources/raw/source/source.md".to_string(),
        kind: NodeKind::Source,
        content: "source".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("canonical source write should succeed");

    let append_error = append_node(AppendNodeRequest {
        path: "/Wiki/topic.md".to_string(),
        content: "next".to_string(),
        expected_etag: None,
        separator: None,
        metadata_json: None,
        kind: Some(NodeKind::Source),
    })
    .expect_err("noncanonical source append should fail");
    assert!(append_error.contains("source path must"));

    let created = write_node(WriteNodeRequest {
        path: "/Sources/raw/keep/keep.md".to_string(),
        kind: NodeKind::Source,
        content: "keep".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("canonical source write should succeed");

    let move_error = move_node(MoveNodeRequest {
        from_path: "/Sources/raw/keep/keep.md".to_string(),
        to_path: "/Sources/raw/renamed/wrong.md".to_string(),
        expected_etag: Some(created.node.etag),
        overwrite: false,
    })
    .expect_err("noncanonical source move should fail");
    assert!(move_error.contains("source path must"));
}

#[test]
fn fs_entrypoints_search_large_hits_without_trap() {
    install_test_service();

    let payload = format!("shared-bench-search {}", "x".repeat(1024 * 1024 - 20));
    for index in 0..10 {
        write_node(WriteNodeRequest {
            path: format!("/Wiki/large/node-{index:03}.md"),
            kind: NodeKind::File,
            content: payload.clone(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .expect("large write should succeed");
    }

    let hits = search_nodes(SearchNodesRequest {
        query_text: "shared-bench-search".to_string(),
        prefix: Some("/Wiki/large".to_string()),
        top_k: 10,
        preview_mode: Some(SearchPreviewMode::None),
    })
    .expect("large search should succeed");

    assert_eq!(hits.len(), 10);
    for window in hits.windows(2) {
        assert!(window[0].score <= window[1].score);
    }
    for hit in hits {
        assert!(hit.path.starts_with("/Wiki/large/"));
        assert!(hit.snippet.is_none());
        assert!(hit.preview.is_none());
    }
}

#[test]
fn fs_entrypoints_cover_move_glob_recent_and_multi_edit() {
    install_test_service();

    let created = write_node(WriteNodeRequest {
        path: "/Wiki/work/item.md".to_string(),
        kind: NodeKind::File,
        content: "alpha beta".to_string(),
        metadata_json: "{}".to_string(),
        expected_etag: None,
    })
    .expect("write should succeed");

    let moved = move_node(MoveNodeRequest {
        from_path: "/Wiki/work/item.md".to_string(),
        to_path: "/Wiki/archive/item.md".to_string(),
        expected_etag: Some(created.node.etag.clone()),
        overwrite: false,
    })
    .expect("move should succeed");
    assert_eq!(moved.from_path, "/Wiki/work/item.md");
    assert_eq!(moved.node.path, "/Wiki/archive/item.md");

    let globbed = glob_nodes(GlobNodesRequest {
        pattern: "**".to_string(),
        path: Some("/Wiki".to_string()),
        node_type: Some(GlobNodeType::Directory),
    })
    .expect("glob should succeed");
    assert!(
        globbed
            .iter()
            .any(|hit| hit.path == "/Wiki/archive" && hit.kind == NodeEntryKind::Directory)
    );

    let recent = recent_nodes(RecentNodesRequest {
        limit: 5,
        path: Some("/Wiki".to_string()),
    })
    .expect("recent should succeed");
    assert_eq!(recent[0].path, "/Wiki/archive/item.md");

    let edited = multi_edit_node(MultiEditNodeRequest {
        path: "/Wiki/archive/item.md".to_string(),
        edits: vec![
            MultiEdit {
                old_text: "alpha".to_string(),
                new_text: "one".to_string(),
            },
            MultiEdit {
                old_text: "beta".to_string(),
                new_text: "two".to_string(),
            },
        ],
        expected_etag: Some(moved.node.etag),
    })
    .expect("multi edit should succeed");
    assert_eq!(edited.replacement_count, 2);
    let edited_node = read_node("/Wiki/archive/item.md".to_string())
        .expect("read should succeed")
        .expect("node should exist");
    assert_eq!(edited_node.content, "one two");
}
