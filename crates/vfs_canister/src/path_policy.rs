// Where: crates/vfs_canister/src/path_policy.rs
// What: Generic path policy Principal guards and result filters.
// Why: Canister entrypoints stay thin while policy storage and leakage control remain centralized.
use std::collections::BTreeSet;

use candid::Principal;
use serde::{Deserialize, Serialize};
use vfs_runtime::VfsService;
use vfs_types::{
    ChildNode, ExportSnapshotResponse, FetchUpdatesResponse, GlobNodeHit, LinkEdge, NodeContext,
    NodeEntry, NodeKind, PathPolicy, PathPolicyEntry, QueryContext, RecentNodeHit, SearchNodeHit,
    SourceEvidence, WriteNodeRequest,
};

pub(crate) const SKILL_REGISTRY_ROOT: &str = wiki_domain::SKILL_REGISTRY_ROOT;
pub(crate) const PUBLIC_SKILL_REGISTRY_ROOT: &str = wiki_domain::PUBLIC_SKILL_REGISTRY_ROOT;
const PATH_POLICIES_PATH: &str = "/System/path-policies.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PathPolicyState {
    pub(crate) path: String,
    mode: String,
    pub(crate) entries: Vec<PathPolicyEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PathPolicyStore {
    version: u32,
    policies: Vec<PathPolicyState>,
}

pub(crate) fn namespace_roles() -> Vec<String> {
    ["Admin", "Writer", "Reader"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn namespace_path(path: &str) -> bool {
    path_under(path, SKILL_REGISTRY_ROOT) || path_under(path, PUBLIC_SKILL_REGISTRY_ROOT)
}

pub(crate) fn namespace_only_prefix(prefix: &str) -> bool {
    namespace_path(prefix)
}

pub(crate) fn ensure_not_policy_store_node(path: &str) -> Result<(), String> {
    if path == PATH_POLICIES_PATH {
        return Err("path policy store is managed by dedicated methods".to_string());
    }
    Ok(())
}

fn default_policy(path: &str) -> PathPolicyState {
    PathPolicyState {
        path: path.to_string(),
        mode: "open".to_string(),
        entries: Vec::new(),
    }
}

fn load_path_policy_store(service: &VfsService) -> Result<PathPolicyStore, String> {
    let Some(node) = service.read_node(PATH_POLICIES_PATH)? else {
        return Ok(PathPolicyStore {
            version: 1,
            policies: Vec::new(),
        });
    };
    let store: PathPolicyStore = serde_json::from_str(&node.content)
        .map_err(|error| format!("path policy store invalid: {error}"))?;
    if store.version != 1 {
        return Err("path policy store version must be 1".to_string());
    }
    Ok(store)
}

fn save_path_policy_store(
    service: &VfsService,
    store: &PathPolicyStore,
    now_millis: i64,
) -> Result<(), String> {
    let expected_etag = service.read_node(PATH_POLICIES_PATH)?.map(|node| node.etag);
    service.write_node(
        WriteNodeRequest {
            path: PATH_POLICIES_PATH.to_string(),
            kind: NodeKind::File,
            content: serde_json::to_string_pretty(store).map_err(|error| error.to_string())?,
            metadata_json: "{}".to_string(),
            expected_etag,
        },
        now_millis,
    )?;
    Ok(())
}

pub(crate) fn load_path_policy(
    service: &VfsService,
    path: &str,
) -> Result<PathPolicyState, String> {
    validate_policy_path(path)?;
    Ok(load_path_policy_store(service)?
        .policies
        .into_iter()
        .find(|policy| policy.path == path)
        .unwrap_or_else(|| default_policy(path)))
}

pub(crate) fn save_path_policy(
    service: &VfsService,
    policy: &PathPolicyState,
    now_millis: i64,
) -> Result<(), String> {
    let mut store = load_path_policy_store(service)?;
    if let Some(existing) = store
        .policies
        .iter_mut()
        .find(|existing| existing.path == policy.path)
    {
        *existing = policy.clone();
    } else {
        store.policies.push(policy.clone());
        store
            .policies
            .sort_by(|left, right| left.path.cmp(&right.path));
    }
    save_path_policy_store(service, &store, now_millis)
}

pub(crate) fn policy_from_state(policy: &PathPolicyState) -> PathPolicy {
    PathPolicy {
        path: policy.path.clone(),
        mode: policy.mode.clone(),
        roles: namespace_roles(),
    }
}

pub(crate) fn enable_policy_for(
    service: &VfsService,
    caller: Principal,
    path: String,
    now_millis: i64,
) -> Result<PathPolicy, String> {
    validate_policy_path(&path)?;
    let mut policy = load_path_policy(service, &path)?;
    if policy.mode == "restricted" {
        ensure_admin(&policy, caller)?;
        return Ok(policy_from_state(&policy));
    }
    if let Some(parent_policy) = load_matching_path_policy(service, &path)? {
        ensure_admin(&parent_policy, caller)?;
    }
    policy.mode = "restricted".to_string();
    policy.entries = vec![PathPolicyEntry {
        principal: caller.to_text(),
        roles: vec!["Admin".to_string()],
    }];
    save_path_policy(service, &policy, now_millis)?;
    Ok(policy_from_state(&policy))
}

pub(crate) fn roles_for(policy: &PathPolicyState, principal: Principal) -> BTreeSet<String> {
    if policy.mode != "restricted" {
        return BTreeSet::from([
            "Admin".to_string(),
            "Writer".to_string(),
            "Reader".to_string(),
        ]);
    }
    policy
        .entries
        .iter()
        .find(|entry| entry.principal == principal.to_text())
        .map(|entry| entry.roles.iter().cloned().collect())
        .unwrap_or_default()
}

pub(crate) fn can_read_policy_store_node(
    service: &VfsService,
    principal: Principal,
) -> Result<bool, String> {
    let store = load_path_policy_store(service)?;
    if store
        .policies
        .iter()
        .all(|policy| policy.mode != "restricted")
    {
        return Ok(true);
    }
    Ok(store
        .policies
        .iter()
        .any(|policy| has_role(policy, principal, "Admin")))
}

pub(crate) fn ensure_policy_store_node_read(
    service: &VfsService,
    principal: Principal,
    path: &str,
) -> Result<(), String> {
    if !policy_store_node_path(path) {
        return Ok(());
    }
    if can_read_policy_store_node(service, principal)? {
        return Ok(());
    }
    Err("path policy access denied: Admin role required".to_string())
}

pub(crate) fn ensure_namespace_read(
    service: &VfsService,
    principal: Principal,
    path: &str,
) -> Result<(), String> {
    let Some(policy) = load_matching_path_policy(service, path)? else {
        return Ok(());
    };
    if has_role(&policy, principal, "Reader") {
        return Ok(());
    }
    Err("path policy access denied: Reader role required".to_string())
}

pub(crate) fn ensure_namespace_publish(
    service: &VfsService,
    principal: Principal,
    path: &str,
) -> Result<(), String> {
    let Some(policy) = load_matching_path_policy(service, path)? else {
        return Ok(());
    };
    if has_role(&policy, principal, "Writer") {
        return Ok(());
    }
    Err("path policy access denied: Writer role required".to_string())
}

pub(crate) fn ensure_admin(policy: &PathPolicyState, principal: Principal) -> Result<(), String> {
    if has_role(policy, principal, "Admin") {
        return Ok(());
    }
    Err("path policy access denied: Admin role required".to_string())
}

pub(crate) fn normalize_policy_role(role: &str) -> Result<String, String> {
    match role {
        "Admin" | "Writer" | "Reader" => Ok(role.to_string()),
        _ => Err("path policy role must be Admin, Writer, or Reader".to_string()),
    }
}

pub(crate) fn grant_role(policy: &mut PathPolicyState, principal: String, role: String) {
    if let Some(entry) = policy
        .entries
        .iter_mut()
        .find(|entry| entry.principal == principal)
    {
        if !entry.roles.contains(&role) {
            entry.roles.push(role);
            entry.roles.sort();
        }
        return;
    }
    policy.entries.push(PathPolicyEntry {
        principal,
        roles: vec![role],
    });
    policy
        .entries
        .sort_by(|left, right| left.principal.cmp(&right.principal));
}

pub(crate) fn revoke_role(policy: &mut PathPolicyState, principal: &str, role: &str) {
    if let Some(entry) = policy
        .entries
        .iter_mut()
        .find(|entry| entry.principal == principal)
    {
        entry.roles.retain(|existing| existing != role);
    }
    policy.entries.retain(|entry| !entry.roles.is_empty());
}

pub(crate) fn filter_entries(
    entries: Vec<NodeEntry>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<NodeEntry> {
    entries
        .into_iter()
        .filter(|entry| visible_path(service, &entry.path, principal, can_read_policy_store))
        .collect()
}

pub(crate) fn filter_children(
    children: Vec<ChildNode>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<ChildNode> {
    children
        .into_iter()
        .filter(|child| visible_path(service, &child.path, principal, can_read_policy_store))
        .collect()
}

pub(crate) fn filter_glob_hits(
    hits: Vec<GlobNodeHit>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<GlobNodeHit> {
    hits.into_iter()
        .filter(|hit| visible_path(service, &hit.path, principal, can_read_policy_store))
        .collect()
}

pub(crate) fn filter_recent_hits(
    hits: Vec<RecentNodeHit>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<RecentNodeHit> {
    hits.into_iter()
        .filter(|hit| visible_path(service, &hit.path, principal, can_read_policy_store))
        .collect()
}

pub(crate) fn filter_search_hits(
    hits: Vec<SearchNodeHit>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<SearchNodeHit> {
    hits.into_iter()
        .filter(|hit| visible_path(service, &hit.path, principal, can_read_policy_store))
        .collect()
}

pub(crate) fn filter_links(
    links: Vec<LinkEdge>,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> Vec<LinkEdge> {
    links
        .into_iter()
        .filter(|link| {
            visible_path(service, &link.source_path, principal, can_read_policy_store)
                && visible_path(service, &link.target_path, principal, can_read_policy_store)
        })
        .collect()
}

pub(crate) fn filter_node_context(
    context: NodeContext,
    principal: Principal,
    can_read_policy_store: bool,
    can_read_inherited: bool,
    service: &VfsService,
) -> Option<NodeContext> {
    if !visible_path(
        service,
        &context.node.path,
        principal,
        can_read_policy_store,
    ) {
        return None;
    }
    Some(NodeContext {
        incoming_links: filter_links(
            context.incoming_links,
            principal,
            can_read_policy_store,
            can_read_inherited,
            service,
        ),
        outgoing_links: filter_links(
            context.outgoing_links,
            principal,
            can_read_policy_store,
            can_read_inherited,
            service,
        ),
        ..context
    })
}

pub(crate) fn filter_query_context(
    mut context: QueryContext,
    principal: Principal,
    can_read_policy_store: bool,
    can_read_inherited: bool,
    service: &VfsService,
) -> QueryContext {
    context.search_hits = filter_search_hits(
        context.search_hits,
        principal,
        can_read_policy_store,
        can_read_inherited,
        service,
    );
    context.nodes = context
        .nodes
        .into_iter()
        .filter_map(|node| {
            filter_node_context(
                node,
                principal,
                can_read_policy_store,
                can_read_inherited,
                service,
            )
        })
        .collect();
    context.graph_links = filter_links(
        context.graph_links,
        principal,
        can_read_policy_store,
        can_read_inherited,
        service,
    );
    context.evidence = context
        .evidence
        .into_iter()
        .map(|item| {
            filter_source_evidence(
                item,
                principal,
                can_read_policy_store,
                can_read_inherited,
                service,
            )
        })
        .filter(|item| visible_path(service, &item.node_path, principal, can_read_policy_store))
        .collect();
    context
}

pub(crate) fn filter_source_evidence(
    mut evidence: SourceEvidence,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> SourceEvidence {
    evidence.refs.retain(|item| {
        visible_path(service, &item.source_path, principal, can_read_policy_store)
            && visible_path(service, &item.via_path, principal, can_read_policy_store)
    });
    evidence
}

pub(crate) fn filter_export_snapshot(
    mut snapshot: ExportSnapshotResponse,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> ExportSnapshotResponse {
    snapshot
        .nodes
        .retain(|node| visible_path(service, &node.path, principal, can_read_policy_store));
    snapshot
}

pub(crate) fn filter_fetch_updates(
    mut updates: FetchUpdatesResponse,
    principal: Principal,
    can_read_policy_store: bool,
    _can_read_inherited: bool,
    service: &VfsService,
) -> FetchUpdatesResponse {
    updates
        .changed_nodes
        .retain(|node| visible_path(service, &node.path, principal, can_read_policy_store));
    updates
        .removed_paths
        .retain(|path| visible_path(service, path, principal, can_read_policy_store));
    updates
}

fn has_role(policy: &PathPolicyState, principal: Principal, role: &str) -> bool {
    let roles = roles_for(policy, principal);
    roles.contains("Admin")
        || (role == "Writer" && roles.contains("Writer"))
        || (role == "Reader" && (roles.contains("Writer") || roles.contains("Reader")))
}

fn policy_store_node_path(path: &str) -> bool {
    path == PATH_POLICIES_PATH
}

fn visible_path(
    service: &VfsService,
    path: &str,
    principal: Principal,
    can_read_policy_store: bool,
) -> bool {
    let restricted = load_matching_path_policy(service, path)
        .map(|policy| policy.is_some())
        .unwrap_or_else(|_| namespace_path(path));
    (can_read_path(service, principal, path).unwrap_or(false) || !restricted)
        && (can_read_policy_store || !policy_store_node_path(path))
}

fn can_read_path(service: &VfsService, principal: Principal, path: &str) -> Result<bool, String> {
    Ok(load_matching_path_policy(service, path)?
        .map(|policy| has_role(&policy, principal, "Reader"))
        .unwrap_or(true))
}

fn policy_applies(policy: &PathPolicyState, path: &str) -> bool {
    policy.mode == "restricted" && path_under(path, &policy.path)
}

fn load_matching_path_policy(
    service: &VfsService,
    path: &str,
) -> Result<Option<PathPolicyState>, String> {
    Ok(load_path_policy_store(service)?
        .policies
        .into_iter()
        .filter(|policy| policy_applies(policy, path))
        .max_by_key(|policy| policy.path.len()))
}

fn path_under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn validate_policy_path(path: &str) -> Result<(), String> {
    if !path.starts_with("/Wiki/") && path != "/Wiki" {
        return Err("path policy path must stay under /Wiki".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(text: &str) -> Principal {
        Principal::from_text(text).expect("principal should parse")
    }

    fn test_service() -> VfsService {
        let dir = tempfile::tempdir().expect("tempdir should create");
        let db_path = dir.path().join("wiki.sqlite3");
        let service = VfsService::new(db_path);
        service.run_fs_migrations().expect("migrations should run");
        std::mem::forget(dir);
        service
    }

    fn restricted_service() -> VfsService {
        let service = test_service();
        save_path_policy(
            &service,
            &PathPolicyState {
                path: SKILL_REGISTRY_ROOT.to_string(),
                mode: "restricted".to_string(),
                entries: vec![PathPolicyEntry {
                    principal: "aaaaa-aa".to_string(),
                    roles: vec!["Admin".to_string()],
                }],
            },
            1,
        )
        .expect("policy should write");
        service
    }

    #[test]
    fn roles_inherit_in_restricted_mode() {
        let admin = principal("aaaaa-aa");
        let publisher = principal("rrkah-fqaaa-aaaaa-aaaaq-cai");
        let viewer = principal("2vxsx-fae");
        let policy = PathPolicyState {
            path: SKILL_REGISTRY_ROOT.to_string(),
            mode: "restricted".to_string(),
            entries: vec![
                PathPolicyEntry {
                    principal: admin.to_text(),
                    roles: vec!["Admin".to_string()],
                },
                PathPolicyEntry {
                    principal: publisher.to_text(),
                    roles: vec!["Writer".to_string()],
                },
                PathPolicyEntry {
                    principal: viewer.to_text(),
                    roles: vec!["Reader".to_string()],
                },
            ],
        };

        assert!(has_role(&policy, admin, "Reader"));
        assert!(has_role(&policy, admin, "Writer"));
        assert!(has_role(&policy, admin, "Admin"));
        assert!(has_role(&policy, publisher, "Reader"));
        assert!(has_role(&policy, publisher, "Writer"));
        assert!(!has_role(&policy, publisher, "Admin"));
        assert!(has_role(&policy, viewer, "Reader"));
        assert!(!has_role(&policy, viewer, "Writer"));
    }

    #[test]
    fn open_mode_grants_all_roles() {
        let policy = PathPolicyState {
            path: SKILL_REGISTRY_ROOT.to_string(),
            mode: "open".to_string(),
            entries: Vec::new(),
        };
        let caller = principal("2vxsx-fae");

        assert!(has_role(&policy, caller, "Admin"));
        assert!(has_role(&policy, caller, "Writer"));
        assert!(has_role(&policy, caller, "Reader"));
    }

    #[test]
    fn path_visibility_hides_skill_and_policy_store_paths_independently() {
        let service = restricted_service();
        let outsider = principal("2vxsx-fae");
        let admin = principal("aaaaa-aa");

        assert!(!visible_path(
            &service,
            "/Wiki/skills/acme/a/SKILL.md",
            outsider,
            true
        ));
        assert!(!visible_path(&service, PATH_POLICIES_PATH, admin, false));
        assert!(visible_path(&service, "/Wiki/public.md", outsider, false));
        assert!(visible_path(
            &service,
            "/Wiki/skills/acme/a/SKILL.md",
            admin,
            false
        ));
        assert!(visible_path(&service, PATH_POLICIES_PATH, outsider, true));
    }

    #[test]
    fn filter_search_hits_removes_hidden_paths() {
        let hits = vec![
            SearchNodeHit {
                path: "/Wiki/skills/acme/a/SKILL.md".to_string(),
                kind: NodeKind::File,
                snippet: None,
                preview: None,
                score: 1.0,
                match_reasons: Vec::new(),
            },
            SearchNodeHit {
                path: PATH_POLICIES_PATH.to_string(),
                kind: NodeKind::File,
                snippet: None,
                preview: None,
                score: 1.0,
                match_reasons: Vec::new(),
            },
            SearchNodeHit {
                path: "/Wiki/public.md".to_string(),
                kind: NodeKind::File,
                snippet: None,
                preview: None,
                score: 1.0,
                match_reasons: Vec::new(),
            },
        ];

        let service = restricted_service();
        let visible = filter_search_hits(hits, principal("2vxsx-fae"), false, true, &service);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].path, "/Wiki/public.md");
    }

    #[test]
    fn longest_prefix_policy_controls_visibility() {
        let service = test_service();
        let admin = principal("aaaaa-aa");
        let reader = principal("2vxsx-fae");
        save_path_policy(
            &service,
            &PathPolicyState {
                path: "/Wiki".to_string(),
                mode: "restricted".to_string(),
                entries: vec![PathPolicyEntry {
                    principal: reader.to_text(),
                    roles: vec!["Reader".to_string()],
                }],
            },
            1,
        )
        .expect("root policy should save");
        save_path_policy(
            &service,
            &PathPolicyState {
                path: "/Wiki/team".to_string(),
                mode: "restricted".to_string(),
                entries: vec![PathPolicyEntry {
                    principal: admin.to_text(),
                    roles: vec!["Admin".to_string()],
                }],
            },
            2,
        )
        .expect("nested policy should save");

        assert!(can_read_path(&service, reader, "/Wiki/public.md").expect("read should check"));
        assert!(
            !can_read_path(&service, reader, "/Wiki/team/private.md").expect("read should check")
        );
    }

    #[test]
    fn nested_policy_creation_requires_parent_admin() {
        let service = test_service();
        let parent_admin = principal("aaaaa-aa");
        let outsider = principal("2vxsx-fae");
        enable_policy_for(&service, parent_admin, "/Wiki/skills".to_string(), 1)
            .expect("parent policy should enable");

        assert!(
            enable_policy_for(&service, outsider, "/Wiki/skills/acme".to_string(), 2,)
                .expect_err("outsider cannot create child policy")
                .contains("Admin")
        );
        enable_policy_for(&service, parent_admin, "/Wiki/skills/acme".to_string(), 3)
            .expect("parent admin can create child policy");
    }

    #[test]
    fn skill_manifest_does_not_affect_core_visibility() {
        let service = test_service();
        service
            .write_node(
                WriteNodeRequest {
                    path: "/Wiki/skills/acme/a/manifest.md".to_string(),
                    kind: NodeKind::File,
                    content: "---\nknowledge: []\n---\n".to_string(),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                1,
            )
            .expect("current manifest should write");
        service
            .write_node(
                WriteNodeRequest {
                    path: "/Wiki/skills/acme/a/versions/20260505T010203Z-etag/manifest.md"
                        .to_string(),
                    kind: NodeKind::File,
                    content: "---\nknowledge:\n  - /Wiki/protected/contracts.md\n---\n".to_string(),
                    metadata_json: "{}".to_string(),
                    expected_etag: None,
                },
                2,
            )
            .expect("archived manifest should write");

        assert!(visible_path(
            &service,
            "/Wiki/protected/contracts.md",
            principal("2vxsx-fae"),
            false
        ));
    }

    #[test]
    fn grant_and_revoke_roles_keep_entries_sorted() {
        let mut policy = PathPolicyState {
            path: SKILL_REGISTRY_ROOT.to_string(),
            mode: "restricted".to_string(),
            entries: Vec::new(),
        };

        grant_role(&mut policy, "b-principal".to_string(), "Reader".to_string());
        grant_role(&mut policy, "a-principal".to_string(), "Writer".to_string());
        grant_role(&mut policy, "a-principal".to_string(), "Reader".to_string());
        revoke_role(&mut policy, "b-principal", "Reader");

        assert_eq!(policy.entries.len(), 1);
        assert_eq!(policy.entries[0].principal, "a-principal");
        assert_eq!(
            policy.entries[0].roles,
            vec!["Reader".to_string(), "Writer".to_string()]
        );
    }
}
