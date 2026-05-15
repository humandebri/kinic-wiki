// Where: crates/vfs_canister/src/lib.rs
// What: ICP canister entrypoints backed by VfsService with an FS-first public API.
// Why: The canister now exposes node-oriented operations directly and keeps the runtime boundary thin.
use std::cell::RefCell;
use std::fs::create_dir_all;
use std::ops::Range;
#[cfg(not(test))]
use std::path::Path;
use std::path::PathBuf;

use candid::{CandidType, Deserialize, Principal, export_service};
use ic_cdk::{init, post_upgrade, query, update};
use ic_http_certification::{
    CERTIFICATE_EXPRESSION_HEADER_NAME, DefaultCelBuilder, DefaultResponseCertification,
    HttpCertification, HttpCertificationPath, HttpCertificationTree, HttpCertificationTreeEntry,
    HttpResponse as CertifiedHttpResponse, utils::add_v2_certificate_header,
};
use ic_stable_structures::DefaultMemoryImpl;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager};
use vfs_runtime::{DatabaseMeta, UsageEvent, VfsService};
use vfs_types::{
    AppendNodeRequest, CanisterHealth, CanonicalRole, ChildNode, DatabaseArchiveChunk,
    DatabaseArchiveInfo, DatabaseMember, DatabaseRestoreChunkRequest, DatabaseRole,
    DatabaseSummary, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest, EditNodeResult,
    ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest, FetchUpdatesResponse,
    GlobNodeHit, GlobNodesRequest, GraphLinksRequest, GraphNeighborhoodRequest,
    IncomingLinksRequest, LinkEdge, ListChildrenRequest, ListNodesRequest, MemoryCapability,
    MemoryManifest, MemoryRoot, MkdirNodeRequest, MkdirNodeResult, MoveNodeRequest, MoveNodeResult,
    MultiEditNodeRequest, MultiEditNodeResult, Node, NodeContext, NodeContextRequest, NodeEntry,
    OpsAnswerSessionCheckRequest, OpsAnswerSessionCheckResult, OpsAnswerSessionRequest,
    OutgoingLinksRequest, QueryContext, QueryContextRequest, RecentNodeHit, RecentNodesRequest,
    SearchNodeHit, SearchNodePathsRequest, SearchNodesRequest, SourceEvidence,
    SourceEvidenceRequest, Status, UrlIngestTriggerSessionCheckRequest,
    UrlIngestTriggerSessionRequest, WriteNodeRequest, WriteNodeResult,
};

const INDEX_DB_PATH: &str = "./DB/index.sqlite3";
const DATABASES_DIR: &str = "./DB/databases";
const II_ALTERNATIVE_ORIGINS_PATH: &str = "/.well-known/ii-alternative-origins";
const II_ALTERNATIVE_ORIGINS_BODY: &str = r#"{"alternativeOrigins":["https://wiki.kinic.xyz","https://kinic.xyz","chrome-extension://jcfniiflikojmbfnaoamlbbddlikchaj","chrome-extension://hbnicbmdodpmihmcnfgejcdgbfmemoci"]}"#;
// WASI filesystem memory is for tmp files and directory metadata, not DB slots.
// SQLite DB files are mounted separately with dedicated MemoryId values.
const WASI_FS_MEMORY_RANGE: Range<u16> = 0..10;
const INDEX_DB_MEMORY_ID: u16 = 10;

#[derive(Clone, Debug, CandidType, Deserialize)]
struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    certificate_version: Option<u16>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct HttpResponse {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    upgrade: Option<bool>,
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static SERVICE: RefCell<Option<VfsService>> = const { RefCell::new(None) };
}

#[init]
fn init_hook() {
    initialize_or_trap();
    certify_http_responses();
}

#[post_upgrade]
fn post_upgrade_hook() {
    initialize_or_trap();
    certify_http_responses();
}

#[query]
fn http_request(request: HttpRequest) -> HttpResponse {
    if request.method != "GET" || request_path(&request.url) != II_ALTERNATIVE_ORIGINS_PATH {
        return HttpResponse {
            status_code: 404,
            headers: vec![(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: b"Not found".to_vec(),
            upgrade: Some(false),
        };
    }

    let (path, entry, tree, mut response) = certified_alternative_origins_response();
    if let Some(certificate) = data_certificate() {
        let witness = tree
            .witness(&entry, II_ALTERNATIVE_ORIGINS_PATH)
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("HTTP certification witness failed: {error}"))
            });
        add_v2_certificate_header(&certificate, &mut response, &witness, &path.to_expr_path());
    }
    http_response_from_certified(response)
}

#[query]
fn status(database_id: String) -> Status {
    with_service(|service| service.status(&database_id, &caller_text()))
        .unwrap_or_else(|error| ic_cdk::trap(&error))
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
fn read_node(database_id: String, path: String) -> Result<Option<Node>, String> {
    with_service(|service| service.read_node(&database_id, &caller_text(), &path))
}

#[query]
fn list_nodes(request: ListNodesRequest) -> Result<Vec<NodeEntry>, String> {
    with_service(|service| service.list_nodes(&caller_text(), request))
}

#[query]
fn list_children(request: ListChildrenRequest) -> Result<Vec<ChildNode>, String> {
    with_service(|service| service.list_children(&caller_text(), request))
}

#[update]
fn create_database() -> Result<String, String> {
    with_usage("create_database", None, |service, caller, now| {
        let meta = service.reserve_generated_database(caller, now)?;
        if let Err(error) = mount_database_file(&meta) {
            let cleanup_error = service
                .discard_database_reservation(&meta.database_id)
                .err();
            return Err(database_create_error(error, cleanup_error));
        }
        if let Err(error) = service.run_database_migrations(&meta.database_id) {
            unmount_database_file(&meta.db_file_name);
            let cleanup_error = service
                .discard_database_reservation(&meta.database_id)
                .err();
            return Err(database_create_error(error, cleanup_error));
        }
        Ok(meta.database_id)
    })
}

#[update]
fn grant_database_access(
    database_id: String,
    principal: String,
    role: DatabaseRole,
) -> Result<(), String> {
    with_usage(
        "grant_database_access",
        Some(database_id.clone()),
        |service, caller, now| {
            let principal = Principal::from_text(&principal)
                .map_err(|error| format!("invalid principal: {error}"))?
                .to_text();
            service.grant_database_access(&database_id, caller, &principal, role, now)
        },
    )
}

#[update]
fn revoke_database_access(database_id: String, principal: String) -> Result<(), String> {
    with_usage(
        "revoke_database_access",
        Some(database_id.clone()),
        |service, caller, _now| {
            let principal = Principal::from_text(&principal)
                .map_err(|error| format!("invalid principal: {error}"))?
                .to_text();
            service.revoke_database_access(&database_id, caller, &principal)
        },
    )
}

#[query]
fn list_database_members(database_id: String) -> Result<Vec<DatabaseMember>, String> {
    with_service(|service| service.list_database_members(&database_id, &caller_text()))
}

#[query]
fn list_databases() -> Result<Vec<DatabaseSummary>, String> {
    with_service(|service| service.list_database_summaries_for_caller(&caller_text()))
}

#[update]
fn delete_database(database_id: String) -> Result<(), String> {
    with_usage(
        "delete_database",
        Some(database_id.clone()),
        |service, caller, now| {
            let meta = service.list_databases().and_then(|databases| {
                databases
                    .into_iter()
                    .find(|meta| meta.database_id == database_id)
                    .ok_or_else(|| format!("database not found: {database_id}"))
            })?;
            service.delete_database(&database_id, caller, now)?;
            unmount_database_file(&meta.db_file_name);
            Ok(())
        },
    )
}

#[update]
fn begin_database_archive(database_id: String) -> Result<DatabaseArchiveInfo, String> {
    with_usage(
        "begin_database_archive",
        Some(database_id.clone()),
        |service, caller, now| service.begin_database_archive(&database_id, caller, now),
    )
}

#[query]
fn read_database_archive_chunk(
    database_id: String,
    offset: u64,
    max_bytes: u32,
) -> Result<DatabaseArchiveChunk, String> {
    with_service(|service| {
        service
            .read_database_archive_chunk(&database_id, &caller_text(), offset, max_bytes)
            .map(|bytes| DatabaseArchiveChunk { bytes })
    })
}

#[update]
fn finalize_database_archive(database_id: String, snapshot_hash: Vec<u8>) -> Result<(), String> {
    with_usage(
        "finalize_database_archive",
        Some(database_id.clone()),
        |service, caller, now| {
            let meta =
                service.finalize_database_archive(&database_id, caller, snapshot_hash, now)?;
            unmount_database_file(&meta.db_file_name);
            Ok(())
        },
    )
}

#[update]
fn cancel_database_archive(database_id: String) -> Result<(), String> {
    with_usage(
        "cancel_database_archive",
        Some(database_id.clone()),
        |service, caller, now| {
            service.cancel_database_archive(&database_id, caller, now)?;
            Ok(())
        },
    )
}

#[update]
fn begin_database_restore(
    database_id: String,
    snapshot_hash: Vec<u8>,
    size_bytes: u64,
) -> Result<(), String> {
    with_usage(
        "begin_database_restore",
        Some(database_id.clone()),
        |service, caller, now| {
            let restore = service.begin_database_restore_session(
                &database_id,
                caller,
                snapshot_hash,
                size_bytes,
                now,
            )?;
            if let Err(error) = mount_database_file(&restore.meta) {
                service
                    .rollback_database_restore_begin(restore.rollback, now)
                    .map_err(|rollback_error| {
                        format!("{error}; restore rollback failed: {rollback_error}")
                    })?;
                return Err(error);
            }
            Ok(())
        },
    )
}

#[update]
fn write_database_restore_chunk(request: DatabaseRestoreChunkRequest) -> Result<(), String> {
    let database_id = request.database_id.clone();
    with_usage(
        "write_database_restore_chunk",
        Some(database_id),
        |service, caller, _now| {
            service.write_database_restore_chunk(
                &request.database_id,
                caller,
                request.offset,
                &request.bytes,
            )
        },
    )
}

#[update]
fn finalize_database_restore(database_id: String) -> Result<(), String> {
    with_usage(
        "finalize_database_restore",
        Some(database_id.clone()),
        |service, caller, now| {
            let meta = service.finalize_database_restore(&database_id, caller, now)?;
            mount_database_file(&meta)
        },
    )
}

#[update]
fn cancel_database_restore(database_id: String) -> Result<(), String> {
    with_usage(
        "cancel_database_restore",
        Some(database_id.clone()),
        |service, caller, now| {
            let meta = service.cancel_database_restore(&database_id, caller, now)?;
            unmount_database_file(&meta.db_file_name);
            Ok(())
        },
    )
}

#[update]
fn write_node(request: WriteNodeRequest) -> Result<WriteNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("write_node", Some(database_id), |service, caller, now| {
        service.write_node(caller, request, now)
    })
}

#[update]
fn authorize_url_ingest_trigger_session(
    request: UrlIngestTriggerSessionRequest,
) -> Result<(), String> {
    let database_id = request.database_id.clone();
    with_usage(
        "authorize_url_ingest_trigger_session",
        Some(database_id),
        |service, caller, now| service.authorize_url_ingest_trigger_session(caller, request, now),
    )
}

#[query]
fn check_url_ingest_trigger_session(
    request: UrlIngestTriggerSessionCheckRequest,
) -> Result<(), String> {
    with_service(|service| service.check_url_ingest_trigger_session(request, now_millis()))
}

#[update]
fn authorize_ops_answer_session(request: OpsAnswerSessionRequest) -> Result<(), String> {
    let database_id = request.database_id.clone();
    with_usage(
        "authorize_ops_answer_session",
        Some(database_id),
        |service, caller, now| service.authorize_ops_answer_session(caller, request, now),
    )
}

#[query]
fn check_ops_answer_session(
    request: OpsAnswerSessionCheckRequest,
) -> Result<OpsAnswerSessionCheckResult, String> {
    with_service(|service| service.check_ops_answer_session(request, now_millis()))
}

#[update]
fn append_node(request: AppendNodeRequest) -> Result<WriteNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("append_node", Some(database_id), |service, caller, now| {
        service.append_node(caller, request, now)
    })
}

#[update]
fn edit_node(request: EditNodeRequest) -> Result<EditNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("edit_node", Some(database_id), |service, caller, now| {
        service.edit_node(caller, request, now)
    })
}

#[update]
fn delete_node(request: DeleteNodeRequest) -> Result<DeleteNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("delete_node", Some(database_id), |service, caller, now| {
        service.delete_node(caller, request, now)
    })
}

#[update]
fn move_node(request: MoveNodeRequest) -> Result<MoveNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("move_node", Some(database_id), |service, caller, now| {
        service.move_node(caller, request, now)
    })
}

#[update]
fn mkdir_node(request: MkdirNodeRequest) -> Result<MkdirNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage("mkdir_node", Some(database_id), |service, caller, now| {
        service.mkdir_node(caller, request, now)
    })
}

#[query]
fn glob_nodes(request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>, String> {
    with_service(|service| service.glob_nodes(&caller_text(), request))
}

#[query]
fn recent_nodes(request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>, String> {
    with_service(|service| service.recent_nodes(&caller_text(), request))
}

#[query]
fn incoming_links(request: IncomingLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| service.incoming_links(&caller_text(), request))
}

#[query]
fn outgoing_links(request: OutgoingLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| service.outgoing_links(&caller_text(), request))
}

#[query]
fn graph_links(request: GraphLinksRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| service.graph_links(&caller_text(), request))
}

#[query]
fn graph_neighborhood(request: GraphNeighborhoodRequest) -> Result<Vec<LinkEdge>, String> {
    with_service(|service| service.graph_neighborhood(&caller_text(), request))
}

#[query]
fn read_node_context(request: NodeContextRequest) -> Result<Option<NodeContext>, String> {
    with_service(|service| service.read_node_context(&caller_text(), request))
}

#[query]
fn query_context(request: QueryContextRequest) -> Result<QueryContext, String> {
    with_service(|service| service.query_context(&caller_text(), request))
}

#[query]
fn source_evidence(request: SourceEvidenceRequest) -> Result<SourceEvidence, String> {
    with_service(|service| service.source_evidence(&caller_text(), request))
}

#[update]
fn multi_edit_node(request: MultiEditNodeRequest) -> Result<MultiEditNodeResult, String> {
    let database_id = request.database_id.clone();
    with_usage(
        "multi_edit_node",
        Some(database_id),
        |service, caller, now| service.multi_edit_node(caller, request, now),
    )
}

#[query]
fn search_nodes(request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>, String> {
    with_service(|service| service.search_nodes(&caller_text(), request))
}

#[query]
fn search_node_paths(request: SearchNodePathsRequest) -> Result<Vec<SearchNodeHit>, String> {
    with_service(|service| service.search_node_paths(&caller_text(), request))
}

#[query]
fn export_snapshot(request: ExportSnapshotRequest) -> Result<ExportSnapshotResponse, String> {
    with_service(|service| service.export_fs_snapshot(&caller_text(), request))
}

#[query]
fn fetch_updates(request: FetchUpdatesRequest) -> Result<FetchUpdatesResponse, String> {
    with_service(|service| service.fetch_fs_updates(&caller_text(), request))
}

fn initialize_or_trap() {
    initialize_service().unwrap_or_else(|error| ic_cdk::trap(&error));
}

fn initialize_service() -> Result<(), String> {
    initialize_wasi_storage()?;
    let service = VfsService::new(PathBuf::from(INDEX_DB_PATH), PathBuf::from(DATABASES_DIR));
    service.run_index_migrations()?;
    for meta in service.list_databases()? {
        mount_database_file(&meta)?;
    }
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
            WASI_FS_MEMORY_RANGE.clone(),
        );

        create_dir_all("tmp").map_err(|error| error.to_string())?;
        create_dir_all(DATABASES_DIR).map_err(|error| error.to_string())?;

        ic_wasi_polyfill::unmount_memory_file(INDEX_DB_PATH);
        let memory = manager.get(MemoryId::new(INDEX_DB_MEMORY_ID));
        let mount_result = ic_wasi_polyfill::mount_memory_file(
            INDEX_DB_PATH,
            Box::new(memory),
            ic_wasi_polyfill::MountedFileSizePolicy::MemoryPages,
        );
        if mount_result > 0 {
            return Err(format!(
                "failed to mount index database file: {mount_result}"
            ));
        }
        Ok(())
    })
}

#[cfg(not(test))]
fn mount_database_file(meta: &DatabaseMeta) -> Result<(), String> {
    MEMORY_MANAGER.with(|manager| {
        let manager = manager.borrow();
        if let Some(parent) = Path::new(&meta.db_file_name).parent() {
            create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        ic_wasi_polyfill::unmount_memory_file(&meta.db_file_name);
        let memory = manager.get(MemoryId::new(meta.mount_id));
        let mount_result = ic_wasi_polyfill::mount_memory_file(
            &meta.db_file_name,
            Box::new(memory),
            ic_wasi_polyfill::MountedFileSizePolicy::MemoryPages,
        );
        if mount_result > 0 {
            return Err(format!(
                "failed to mount database file {}: {}",
                meta.database_id, mount_result
            ));
        }
        Ok(())
    })
}

#[cfg(test)]
fn mount_database_file(_meta: &DatabaseMeta) -> Result<(), String> {
    if TEST_MOUNT_DATABASE_FILE_FAIL_ONCE.with(|flag| flag.replace(false)) {
        return Err("test mount failure".to_string());
    }
    Ok(())
}

#[cfg(not(test))]
fn unmount_database_file(db_file_name: &str) {
    ic_wasi_polyfill::unmount_memory_file(db_file_name);
}

#[cfg(test)]
fn unmount_database_file(_db_file_name: &str) {}

#[cfg(test)]
thread_local! {
    static TEST_MOUNT_DATABASE_FILE_FAIL_ONCE: RefCell<bool> = const { RefCell::new(false) };
}

#[cfg(test)]
fn fail_next_mount_database_file_for_test() {
    TEST_MOUNT_DATABASE_FILE_FAIL_ONCE.with(|flag| flag.replace(true));
}

fn database_create_error(error: String, cleanup_error: Option<String>) -> String {
    match cleanup_error {
        Some(cleanup_error) => format!("{error}; cleanup failed: {cleanup_error}"),
        None => error,
    }
}

fn caller_text() -> String {
    #[cfg(test)]
    {
        "2vxsx-fae".to_string()
    }
    #[cfg(not(test))]
    {
        ic_cdk::api::msg_caller().to_text()
    }
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

fn cycle_balance() -> u128 {
    #[cfg(test)]
    {
        1_000_000_000_000
    }
    #[cfg(not(test))]
    {
        ic_cdk::api::canister_cycle_balance()
    }
}

fn with_usage<T, F>(method: &str, database_id: Option<String>, f: F) -> Result<T, String>
where
    F: FnOnce(&VfsService, &str, i64) -> Result<T, String>,
{
    let caller = caller_text();
    let now = now_millis();
    let before_cycles = cycle_balance();
    SERVICE.with(|slot| {
        let borrowed = slot.borrow();
        let service = borrowed
            .as_ref()
            .ok_or_else(|| "wiki service is not initialized".to_string())?;
        let result = f(service, &caller, now);
        let after_cycles = cycle_balance();
        let cycles_delta = before_cycles.saturating_sub(after_cycles);
        let error = result.as_ref().err().map(String::as_str);
        let _ = service.record_usage_event(UsageEvent {
            method,
            database_id: database_id.as_deref(),
            caller: &caller,
            success: result.is_ok(),
            cycles_delta,
            error,
            now,
        });
        result
    })
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

fn certify_http_responses() {
    let (_, _, tree, _) = certified_alternative_origins_response();
    set_certified_data(tree.root_hash());
}

fn certified_alternative_origins_response() -> (
    HttpCertificationPath<'static>,
    HttpCertificationTreeEntry<'static>,
    HttpCertificationTree,
    CertifiedHttpResponse<'static>,
) {
    let cel_expr = DefaultCelBuilder::response_only_certification()
        .with_response_certification(DefaultResponseCertification::certified_response_headers(
            vec![
                "Content-Type",
                "Cache-Control",
                "Access-Control-Allow-Origin",
            ],
        ))
        .build();
    let response = alternative_origins_response(cel_expr.to_string());
    let certification = HttpCertification::response_only(&cel_expr, &response, None)
        .unwrap_or_else(|error| ic_cdk::trap(format!("HTTP certification failed: {error}")));
    let path = HttpCertificationPath::exact(II_ALTERNATIVE_ORIGINS_PATH);
    let entry = HttpCertificationTreeEntry::new(path.clone(), certification);
    let mut tree = HttpCertificationTree::default();
    tree.insert(&entry);
    (path, entry, tree, response)
}

fn alternative_origins_response(certificate_expression: String) -> CertifiedHttpResponse<'static> {
    CertifiedHttpResponse::ok(
        II_ALTERNATIVE_ORIGINS_BODY.as_bytes().to_vec(),
        vec![
            (
                "Content-Type".to_string(),
                "application/json; charset=utf-8".to_string(),
            ),
            (
                "Cache-Control".to_string(),
                "public, max-age=300".to_string(),
            ),
            ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
            (
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                certificate_expression,
            ),
        ],
    )
    .with_upgrade(false)
    .build()
}

fn http_response_from_certified(response: CertifiedHttpResponse<'static>) -> HttpResponse {
    HttpResponse {
        status_code: response.status_code().as_u16(),
        headers: response.headers().to_vec(),
        body: response.body().to_vec(),
        upgrade: response.upgrade(),
    }
}

fn request_path(url: &str) -> &str {
    url.split_once('?').map_or(url, |(path, _)| path)
}

#[cfg(target_arch = "wasm32")]
fn set_certified_data(data: impl AsRef<[u8]>) {
    ic_cdk::api::certified_data_set(data);
}

#[cfg(not(target_arch = "wasm32"))]
fn set_certified_data(_data: impl AsRef<[u8]>) {}

#[cfg(target_arch = "wasm32")]
fn data_certificate() -> Option<Vec<u8>> {
    ic_cdk::api::data_certificate()
}

#[cfg(not(target_arch = "wasm32"))]
fn data_certificate() -> Option<Vec<u8>> {
    None
}

export_service!();

pub fn candid_interface() -> String {
    normalize_candid_interface(__export_service())
}

fn normalize_candid_interface(interface: String) -> String {
    let normalized = normalize_candid_method_input(
        &interface,
        "outgoing_links",
        "IncomingLinksRequest",
        "OutgoingLinksRequest",
    );
    ensure_outgoing_links_request(normalized)
}

fn normalize_candid_method_input(
    interface: &str,
    method: &str,
    exported_input: &str,
    public_input: &str,
) -> String {
    let mut normalized = interface
        .lines()
        .map(|line| {
            let prefix = format!("  {method} : ({exported_input}) -> (");
            if line.starts_with(&prefix) && line.ends_with(" query;") {
                line.replacen(
                    &format!("{method} : ({exported_input})"),
                    &format!("{method} : ({public_input})"),
                    1,
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if interface.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

fn ensure_outgoing_links_request(interface: String) -> String {
    if interface.contains("type OutgoingLinksRequest = record {") {
        return interface;
    }
    interface.replace(
        "type LinkEdge = record {",
        "type OutgoingLinksRequest = record { path : text; limit : nat32; database_id : text };\ntype LinkEdge = record {",
    )
}

#[cfg(feature = "canbench-rs")]
mod benches;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_sync_contract;
