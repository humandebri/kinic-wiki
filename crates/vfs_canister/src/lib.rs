// Where: crates/vfs_canister/src/lib.rs
// What: ICP canister entrypoints backed by VfsService with an FS-first public API.
// Why: The canister now exposes node-oriented operations directly and keeps the runtime boundary thin.
use std::cell::RefCell;
use std::fs::create_dir_all;
use std::ops::Range;
use std::path::{Path, PathBuf};

use candid::{Principal, export_service};
use ic_cdk::{init, post_upgrade, query, update};
use ic_stable_structures::DefaultMemoryImpl;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager};
use vfs_runtime::VfsService;
use vfs_types::{
    AppendNodeRequest, CanisterHealth, CanonicalRole, ChildNode, DeleteNodeRequest,
    DeleteNodeResult, EditNodeRequest, EditNodeResult, ExportSnapshotRequest,
    ExportSnapshotResponse, FetchUpdatesRequest, FetchUpdatesResponse, GlobNodeHit,
    GlobNodesRequest, GraphLinksRequest, GraphNeighborhoodRequest, IncomingLinksRequest, LinkEdge,
    ListChildrenRequest, ListNodesRequest, MemoryCapability, MemoryManifest, MemoryRoot,
    MkdirNodeRequest, MkdirNodeResult, MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest,
    MultiEditNodeResult, Node, NodeContext, NodeContextRequest, NodeEntry, OutgoingLinksRequest,
    PathPolicy, PathPolicyEntry, QueryContext, QueryContextRequest, RecentNodeHit,
    RecentNodesRequest, SearchNodeHit, SearchNodePathsRequest, SearchNodesRequest, SourceEvidence,
    SourceEvidenceRequest, Status, WriteNodeRequest, WriteNodeResult,
};
use wiki_domain::validate_source_path_for_kind;

mod path_policy;
use path_policy::{
    SKILL_REGISTRY_ROOT, can_read_policy_store_node, enable_policy_for, ensure_admin,
    ensure_namespace_publish, ensure_namespace_read, ensure_not_policy_store_node,
    ensure_policy_store_node_read, filter_children, filter_entries, filter_export_snapshot,
    filter_fetch_updates, filter_glob_hits, filter_links, filter_node_context,
    filter_query_context, filter_recent_hits, filter_search_hits, filter_source_evidence,
    grant_role, load_path_policy, namespace_only_prefix, namespace_path, namespace_roles,
    normalize_policy_role, policy_from_state, revoke_role, roles_for, save_path_policy,
};

const DB_PATH: &str = "./DB/wiki.sqlite3";
const FS_MEMORY_RANGE: Range<u8> = 200..210;
const DB_MEMORY_ID: u8 = 210;

#[derive(Clone, Copy)]
struct PathPolicyReadAccess {
    restricted: Principal,
    policy_store: bool,
    inherited: bool,
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static SERVICE: RefCell<Option<VfsService>> = const { RefCell::new(None) };
    #[cfg(test)]
    static TEST_CALLER: RefCell<Principal> = const { RefCell::new(Principal::anonymous()) };
}

#[init]
fn init_hook() {
    initialize_or_trap();
}

#[post_upgrade]
fn post_upgrade_hook() {
    initialize_or_trap();
}

#[query]
fn status() -> Status {
    with_service(|service| service.status()).unwrap_or_else(|error| ic_cdk::trap(&error))
}

#[query]
fn canister_health() -> CanisterHealth {
    CanisterHealth {
        cycles_balance: ic_cdk::api::canister_cycle_balance(),
    }
}

#[query]
fn memory_manifest() -> MemoryManifest {
    MemoryManifest {
        api_version: "agent-memory-v1".to_string(),
        purpose: "Canister-backed long-term wiki memory for agents".to_string(),
        roots: vec![
            MemoryRoot {
                path: "/Wiki".to_string(),
                kind: "wiki".to_string(),
            },
            MemoryRoot {
                path: "/Sources".to_string(),
                kind: "raw_sources".to_string(),
            },
        ],
        capabilities: memory_capabilities(),
        canonical_roles: canonical_roles(),
        write_policy: "agent_memory_read_only".to_string(),
        recommended_entrypoint: "query_context".to_string(),
        max_depth: 2,
        max_query_limit: 100,
        budget_unit: "approx_chars_from_tokens".to_string(),
    }
}

#[query]
fn read_node(path: String) -> Result<Option<Node>, String> {
    with_service(|service| {
        ensure_policy_store_node_read(service, current_caller(), &path)?;
        ensure_namespace_read(service, current_caller(), &path)?;
        service.read_node(&path)
    })
}

#[query]
fn list_nodes(request: ListNodesRequest) -> Result<Vec<NodeEntry>, String> {
    with_service(|service| {
        if namespace_only_prefix(request.prefix.as_str()) {
            ensure_namespace_read(service, current_caller(), &request.prefix)?;
        }
        let access = path_policy_read_access(service)?;
        service.list_nodes(request).map(|entries| {
            filter_entries(
                entries,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn list_children(request: ListChildrenRequest) -> Result<Vec<ChildNode>, String> {
    with_service(|service| {
        if namespace_path(request.path.as_str()) {
            ensure_namespace_read(service, current_caller(), &request.path)?;
        }
        let access = path_policy_read_access(service)?;
        service.list_children(request).map(|children| {
            filter_children(
                children,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[update]
fn write_node(request: WriteNodeRequest) -> Result<WriteNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    validate_source_path_for_kind(&request.path, &request.kind)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        service.write_node(request, now_millis())
    })
}

#[update]
fn append_node(request: AppendNodeRequest) -> Result<WriteNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        validate_append_source_path(service, &request)?;
        service.append_node(request, now_millis())
    })
}

#[update]
fn edit_node(request: EditNodeRequest) -> Result<EditNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        service.edit_node(request, now_millis())
    })
}

#[update]
fn delete_node(request: DeleteNodeRequest) -> Result<DeleteNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        service.delete_node(request, now_millis())
    })
}

#[update]
fn move_node(request: MoveNodeRequest) -> Result<MoveNodeResult, String> {
    ensure_not_policy_store_node(&request.from_path)?;
    ensure_not_policy_store_node(&request.to_path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.from_path)?;
        ensure_namespace_publish(service, current_caller(), &request.to_path)?;
        validate_move_source_path(service, &request)?;
        service.move_node(request, now_millis())
    })
}

#[query]
fn mkdir_node(request: MkdirNodeRequest) -> Result<MkdirNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        service.mkdir_node(request)
    })
}

#[query]
fn glob_nodes(request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>, String> {
    with_service(|service| {
        if request.path.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.path.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.glob_nodes(request).map(|hits| {
            filter_glob_hits(
                hits,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn recent_nodes(request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>, String> {
    with_service(|service| {
        if request.path.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.path.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.recent_nodes(request).map(|hits| {
            filter_recent_hits(
                hits,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn incoming_links(request: IncomingLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| {
        ensure_namespace_read(service, current_caller(), &request.path)?;
        let access = path_policy_read_access(service)?;
        service.incoming_links(request).map(|links| {
            filter_links(
                links,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn outgoing_links(request: OutgoingLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| {
        ensure_namespace_read(service, current_caller(), &request.path)?;
        let access = path_policy_read_access(service)?;
        service.outgoing_links(request).map(|links| {
            filter_links(
                links,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn graph_links(request: GraphLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| {
        if namespace_only_prefix(request.prefix.as_str()) {
            ensure_namespace_read(service, current_caller(), &request.prefix)?;
        }
        let access = path_policy_read_access(service)?;
        service.graph_links(request).map(|links| {
            filter_links(
                links,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn graph_neighborhood(request: GraphNeighborhoodRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| {
        ensure_namespace_read(service, current_caller(), &request.center_path)?;
        let access = path_policy_read_access(service)?;
        service.graph_neighborhood(request).map(|links| {
            filter_links(
                links,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn read_node_context(request: NodeContextRequest) -> Result<Option<NodeContext>, String> {
    with_service(|service| {
        ensure_namespace_read(service, current_caller(), &request.path)?;
        let access = path_policy_read_access(service)?;
        service.read_node_context(request).map(|context| {
            context.and_then(|item| {
                filter_node_context(
                    item,
                    access.restricted,
                    access.policy_store,
                    access.inherited,
                    service,
                )
            })
        })
    })
}

#[query]
fn query_context(request: QueryContextRequest) -> Result<QueryContext, String> {
    with_service(|service| {
        if request
            .namespace
            .as_deref()
            .is_some_and(namespace_only_prefix)
        {
            ensure_namespace_read(
                service,
                current_caller(),
                request.namespace.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.query_context(request).map(|context| {
            filter_query_context(
                context,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn source_evidence(request: SourceEvidenceRequest) -> Result<SourceEvidence, String> {
    with_service(|service| {
        ensure_namespace_read(service, current_caller(), &request.node_path)?;
        let access = path_policy_read_access(service)?;
        service.source_evidence(request).map(|evidence| {
            filter_source_evidence(
                evidence,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[update]
fn multi_edit_node(request: MultiEditNodeRequest) -> Result<MultiEditNodeResult, String> {
    ensure_not_policy_store_node(&request.path)?;
    with_service(|service| {
        ensure_namespace_publish(service, current_caller(), &request.path)?;
        service.multi_edit_node(request, now_millis())
    })
}

#[query]
fn search_nodes(request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>, String> {
    with_service(|service| {
        if request.prefix.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.prefix.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.search_nodes(request).map(|hits| {
            filter_search_hits(
                hits,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn search_node_paths(request: SearchNodePathsRequest) -> Result<Vec<SearchNodeHit>, String> {
    with_service(|service| {
        if request.prefix.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.prefix.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.search_node_paths(request).map(|hits| {
            filter_search_hits(
                hits,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[update]
fn export_snapshot(request: ExportSnapshotRequest) -> Result<ExportSnapshotResponse, String> {
    with_service(|service| {
        if request.prefix.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.prefix.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.export_fs_snapshot(request).map(|snapshot| {
            filter_export_snapshot(
                snapshot,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[query]
fn fetch_updates(request: FetchUpdatesRequest) -> Result<FetchUpdatesResponse, String> {
    with_service(|service| {
        if request.prefix.as_deref().is_some_and(namespace_only_prefix) {
            ensure_namespace_read(
                service,
                current_caller(),
                request.prefix.as_deref().unwrap_or("/Wiki"),
            )?;
        }
        let access = path_policy_read_access(service)?;
        service.fetch_fs_updates(request).map(|updates| {
            filter_fetch_updates(
                updates,
                access.restricted,
                access.policy_store,
                access.inherited,
                service,
            )
        })
    })
}

#[update]
fn enable_path_policy(path: String) -> Result<PathPolicy, String> {
    with_service(|service| enable_policy_for(service, current_caller(), path, now_millis()))
}

#[query]
fn my_path_policy_roles(_path: String) -> Vec<String> {
    with_service(|service| {
        let policy = load_path_policy(service, &_path)?;
        Ok(roles_for(&policy, current_caller()).into_iter().collect())
    })
    .unwrap_or_default()
}

#[query]
fn path_policy_entries(_path: String) -> Result<Vec<PathPolicyEntry>, String> {
    with_service(|service| {
        let policy = load_path_policy(service, &_path)?;
        ensure_admin(&policy, current_caller())?;
        Ok(policy.entries)
    })
}

#[update]
fn grant_path_policy_role(path: String, principal: String, role: String) -> Result<(), String> {
    with_service(|service| {
        let mut policy = load_path_policy(service, &path)?;
        ensure_admin(&policy, current_caller())?;
        let role = normalize_policy_role(&role)?;
        Principal::from_text(&principal).map_err(|error| format!("invalid principal: {error}"))?;
        grant_role(&mut policy, principal, role);
        save_path_policy(service, &policy, now_millis())
    })
}

#[update]
fn revoke_path_policy_role(path: String, principal: String, role: String) -> Result<(), String> {
    with_service(|service| {
        let mut policy = load_path_policy(service, &path)?;
        ensure_admin(&policy, current_caller())?;
        let role = normalize_policy_role(&role)?;
        revoke_role(&mut policy, &principal, &role);
        save_path_policy(service, &policy, now_millis())
    })
}

#[query]
fn path_policy(_path: String) -> PathPolicy {
    with_service(|service| {
        let policy = load_path_policy(service, &_path)?;
        Ok(policy_from_state(&policy))
    })
    .unwrap_or_else(|_| PathPolicy {
        path: SKILL_REGISTRY_ROOT.to_string(),
        mode: "open".to_string(),
        roles: namespace_roles(),
    })
}

fn initialize_or_trap() {
    initialize_service().unwrap_or_else(|error| ic_cdk::trap(&error));
}

fn initialize_service() -> Result<(), String> {
    initialize_wasi_storage()?;
    let service = VfsService::new(PathBuf::from(DB_PATH));
    service.run_fs_migrations()?;
    SERVICE.with(|slot| *slot.borrow_mut() = Some(service));
    Ok(())
}

fn initialize_wasi_storage() -> Result<(), String> {
    MEMORY_MANAGER.with(|manager| {
        let manager = manager.borrow();
        ic_wasi_polyfill::init_with_memory_manager(
            &[0u8; 32],
            &[("SQLITE_TMPDIR", "tmp")],
            &manager,
            FS_MEMORY_RANGE.clone(),
        );

        create_dir_all("tmp").map_err(|error| error.to_string())?;
        let db_parent = Path::new(DB_PATH)
            .parent()
            .ok_or_else(|| "database path is missing parent directory".to_string())?;
        create_dir_all(db_parent).map_err(|error| error.to_string())?;

        ic_wasi_polyfill::unmount_memory_file(DB_PATH);
        let memory = manager.get(MemoryId::new(DB_MEMORY_ID));
        let mount_result = ic_wasi_polyfill::mount_memory_file(
            DB_PATH,
            Box::new(memory),
            ic_wasi_polyfill::MountedFileSizePolicy::MemoryPages,
        );
        if mount_result > 0 {
            return Err(format!("failed to mount database file: {mount_result}"));
        }
        Ok(())
    })
}

fn now_millis() -> i64 {
    #[cfg(test)]
    {
        1_700_000_000_000
    }
    #[cfg(not(test))]
    {
        (ic_cdk::api::time() / 1_000_000) as i64
    }
}

fn with_service<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce(&VfsService) -> Result<T, String>,
{
    SERVICE.with(|slot| {
        let borrowed = slot.borrow();
        let service = borrowed
            .as_ref()
            .ok_or_else(|| "wiki service is not initialized".to_string())?;
        f(service)
    })
}

fn current_caller() -> Principal {
    #[cfg(test)]
    {
        TEST_CALLER.with(|caller| *caller.borrow())
    }
    #[cfg(not(test))]
    {
        ic_cdk::api::msg_caller()
    }
}

#[cfg(test)]
fn set_test_caller(principal: Principal) {
    TEST_CALLER.with(|caller| *caller.borrow_mut() = principal);
}

fn path_policy_read_access(service: &VfsService) -> Result<PathPolicyReadAccess, String> {
    let caller = current_caller();
    Ok(PathPolicyReadAccess {
        restricted: caller,
        policy_store: can_read_policy_store_node(service, caller)?,
        inherited: true,
    })
}

fn memory_capabilities() -> Vec<MemoryCapability> {
    [
        (
            "query_context",
            "Primary agent-memory entrypoint for task-scoped context bundles",
        ),
        ("source_evidence", "Read source-path evidence for one node"),
        (
            "memory_manifest",
            "Discover memory API shape, limits, and policy",
        ),
        (
            "read_node_context",
            "Auxiliary node read with incoming and outgoing links",
        ),
        ("search_nodes", "Auxiliary search with lightweight previews"),
        (
            "graph_neighborhood",
            "Auxiliary local link graph around one node",
        ),
        ("recent_nodes", "Auxiliary recent live-node listing"),
    ]
    .into_iter()
    .map(|(name, description)| MemoryCapability {
        name: name.to_string(),
        description: description.to_string(),
    })
    .collect()
}

fn canonical_roles() -> Vec<CanonicalRole> {
    [
        (
            "index",
            "index.md",
            "Content-oriented catalog of pages in a scope",
        ),
        (
            "overview",
            "overview.md",
            "Corpus-level synthesis maintained by agents",
        ),
        ("log", "log.md", "Append-only chronological mutation log"),
        (
            "schema",
            "schema.md",
            "Scope-local conventions and write rules",
        ),
        ("topics", "topics/*.md", "Topic-level synthesis pages"),
        (
            "provenance",
            "provenance.md",
            "Source-path provenance for a scope or node",
        ),
    ]
    .into_iter()
    .map(|(name, path_pattern, purpose)| CanonicalRole {
        name: name.to_string(),
        path_pattern: path_pattern.to_string(),
        purpose: purpose.to_string(),
    })
    .collect()
}

fn validate_append_source_path(
    service: &VfsService,
    request: &AppendNodeRequest,
) -> Result<(), String> {
    if let Some(kind) = request.kind.as_ref() {
        validate_source_path_for_kind(&request.path, kind)?;
        return Ok(());
    }
    let existing = service.read_node(&request.path)?;
    if let Some(node) = existing {
        validate_source_path_for_kind(&request.path, &node.kind)?;
    }
    Ok(())
}

fn validate_move_source_path(
    service: &VfsService,
    request: &MoveNodeRequest,
) -> Result<(), String> {
    let current = service
        .read_node(&request.from_path)?
        .ok_or_else(|| format!("node does not exist: {}", request.from_path))?;
    validate_source_path_for_kind(&request.to_path, &current.kind)
}

export_service!();

pub fn candid_interface() -> String {
    normalize_candid_interface(__export_service())
}

fn normalize_candid_interface(interface: String) -> String {
    // Where: canister Candid export path.
    // What: Restore public nominal request names for path-only queries.
    // Why: candid::export_service() deduplicates identical record shapes and
    //      rewrites path-only requests to DeleteNodeResult.
    let normalized = interface
        .replace(
            "list_children : (DeleteNodeResult) -> (Result_7) query;",
            "list_children : (ListChildrenRequest) -> (Result_7) query;",
        )
        .replace(
            "list_children : (DeleteNodeResult) -> (Result_9) query;",
            "list_children : (ListChildrenRequest) -> (Result_9) query;",
        )
        .replace(
            "mkdir_node : (DeleteNodeResult) -> (Result_9) query;",
            "mkdir_node : (MkdirNodeRequest) -> (Result_9) query;",
        )
        .replace(
            "mkdir_node : (DeleteNodeResult) -> (Result_10) query;",
            "mkdir_node : (MkdirNodeRequest) -> (Result_10) query;",
        )
        .replace(
            "mkdir_node : (DeleteNodeResult) -> (Result_11) query;",
            "mkdir_node : (MkdirNodeRequest) -> (Result_11) query;",
        )
        .replace(
            "outgoing_links : (IncomingLinksRequest) -> (Result_6) query;",
            "outgoing_links : (OutgoingLinksRequest) -> (Result_6) query;",
        )
        .replace(
            "outgoing_links : (IncomingLinksRequest) -> (Result_8) query;",
            "outgoing_links : (OutgoingLinksRequest) -> (Result_8) query;",
        );
    let normalized = if normalized.contains("type ListChildrenRequest = record { path : text };") {
        normalized
    } else {
        normalized.replace(
            "type ListNodesRequest = record { recursive : bool; prefix : text };",
            "type ListChildrenRequest = record { path : text };\ntype ListNodesRequest = record { recursive : bool; prefix : text };",
        )
    };
    if normalized.contains("type MkdirNodeRequest = record { path : text };") {
        return ensure_outgoing_links_request(normalized);
    }
    ensure_outgoing_links_request(normalized.replace(
        "type MkdirNodeResult = record { created : bool; path : text };",
        "type MkdirNodeRequest = record { path : text };\ntype MkdirNodeResult = record { created : bool; path : text };",
    ))
}

fn ensure_outgoing_links_request(interface: String) -> String {
    if interface.contains("type OutgoingLinksRequest = record { path : text; limit : nat32 };") {
        return interface;
    }
    interface.replace(
        "type LinkEdge = record {",
        "type OutgoingLinksRequest = record { path : text; limit : nat32 };\ntype LinkEdge = record {",
    )
}

#[cfg(feature = "canbench-rs")]
mod benches;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_sync_contract;
