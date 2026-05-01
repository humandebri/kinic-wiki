// Where: crates/vfs_types/src/fs.rs
// What: FS-first public types for the reusable VFS node contract.
// Why: Store, runtime, client, and canister layers should share one transport model.
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    #[serde(alias = "File")]
    File,
    #[serde(alias = "Source")]
    Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
#[serde(rename_all = "snake_case")]
pub enum NodeEntryKind {
    #[serde(alias = "Directory")]
    Directory,
    #[serde(alias = "File")]
    File,
    #[serde(alias = "Source")]
    Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct Node {
    pub path: String,
    pub kind: NodeKind,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub etag: String,
    pub metadata_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ListNodesRequest {
    pub prefix: String,
    pub recursive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ListChildrenRequest {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct NodeEntry {
    pub path: String,
    pub kind: NodeEntryKind,
    pub updated_at: i64,
    pub etag: String,
    pub has_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ChildNode {
    pub path: String,
    pub name: String,
    pub kind: NodeEntryKind,
    pub updated_at: Option<i64>,
    pub etag: Option<String>,
    pub size_bytes: Option<u64>,
    pub is_virtual: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct WriteNodeRequest {
    pub path: String,
    pub kind: NodeKind,
    pub content: String,
    pub metadata_json: String,
    pub expected_etag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct NodeMutationAck {
    pub path: String,
    pub kind: NodeKind,
    pub updated_at: i64,
    pub etag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct WriteNodeResult {
    pub node: NodeMutationAck,
    pub created: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct AppendNodeRequest {
    pub path: String,
    pub content: String,
    pub expected_etag: Option<String>,
    pub separator: Option<String>,
    pub metadata_json: Option<String>,
    pub kind: Option<NodeKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct EditNodeRequest {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
    pub expected_etag: Option<String>,
    pub replace_all: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct EditNodeResult {
    pub node: NodeMutationAck,
    pub replacement_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MkdirNodeRequest {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MkdirNodeResult {
    pub path: String,
    pub created: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MoveNodeRequest {
    pub from_path: String,
    pub to_path: String,
    pub expected_etag: Option<String>,
    pub overwrite: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MoveNodeResult {
    pub node: NodeMutationAck,
    pub from_path: String,
    pub overwrote: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
#[serde(rename_all = "snake_case")]
pub enum GlobNodeType {
    #[serde(alias = "File")]
    File,
    #[serde(alias = "Directory")]
    Directory,
    #[serde(alias = "Any")]
    Any,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct GlobNodesRequest {
    pub pattern: String,
    pub path: Option<String>,
    pub node_type: Option<GlobNodeType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct GlobNodeHit {
    pub path: String,
    pub kind: NodeEntryKind,
    pub has_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct RecentNodesRequest {
    pub limit: u32,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct RecentNodeHit {
    pub path: String,
    pub kind: NodeKind,
    pub updated_at: i64,
    pub etag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct IncomingLinksRequest {
    pub path: String,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct OutgoingLinksRequest {
    pub path: String,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct GraphLinksRequest {
    pub prefix: String,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct GraphNeighborhoodRequest {
    pub center_path: String,
    pub depth: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct LinkEdge {
    pub source_path: String,
    pub target_path: String,
    pub raw_href: String,
    pub link_text: String,
    pub link_kind: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct NodeContextRequest {
    pub path: String,
    pub link_limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct NodeContext {
    pub node: Node,
    pub incoming_links: Vec<LinkEdge>,
    pub outgoing_links: Vec<LinkEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MultiEdit {
    pub old_text: String,
    pub new_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MultiEditNodeRequest {
    pub path: String,
    pub edits: Vec<MultiEdit>,
    pub expected_etag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MultiEditNodeResult {
    pub node: NodeMutationAck,
    pub replacement_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct DeleteNodeRequest {
    pub path: String,
    pub expected_etag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct DeleteNodeResult {
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SearchNodesRequest {
    pub query_text: String,
    pub prefix: Option<String>,
    pub top_k: u32,
    #[serde(default)]
    pub preview_mode: Option<SearchPreviewMode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SearchNodePathsRequest {
    pub query_text: String,
    pub prefix: Option<String>,
    pub top_k: u32,
    #[serde(default)]
    pub preview_mode: Option<SearchPreviewMode>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CandidType)]
pub struct SearchNodeHit {
    pub path: String,
    pub kind: NodeKind,
    pub snippet: Option<String>,
    #[serde(default)]
    pub preview: Option<SearchPreview>,
    pub score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
#[serde(rename_all = "snake_case")]
pub enum SearchPreviewMode {
    #[serde(alias = "None")]
    None,
    #[serde(alias = "Light")]
    Light,
    #[serde(alias = "ContentStart")]
    ContentStart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
#[serde(rename_all = "snake_case")]
pub enum SearchPreviewField {
    #[serde(alias = "Content")]
    Content,
    #[serde(alias = "Path")]
    Path,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SearchPreview {
    pub field: SearchPreviewField,
    pub match_reason: String,
    pub char_offset: u32,
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ExportSnapshotRequest {
    pub prefix: Option<String>,
    pub limit: u32,
    pub cursor: Option<String>,
    pub snapshot_revision: Option<String>,
    pub snapshot_session_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ExportSnapshotResponse {
    pub snapshot_revision: String,
    pub snapshot_session_id: Option<String>,
    pub nodes: Vec<Node>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct FetchUpdatesRequest {
    pub known_snapshot_revision: String,
    pub prefix: Option<String>,
    pub limit: u32,
    pub cursor: Option<String>,
    pub target_snapshot_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct FetchUpdatesResponse {
    pub snapshot_revision: String,
    pub changed_nodes: Vec<Node>,
    pub removed_paths: Vec<String>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MemoryRoot {
    pub path: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MemoryCapability {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct CanonicalRole {
    pub name: String,
    pub path_pattern: String,
    pub purpose: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct MemoryManifest {
    pub api_version: String,
    pub purpose: String,
    pub roots: Vec<MemoryRoot>,
    pub capabilities: Vec<MemoryCapability>,
    pub canonical_roles: Vec<CanonicalRole>,
    pub write_policy: String,
    pub recommended_entrypoint: String,
    pub max_depth: u32,
    pub max_query_limit: u32,
    pub budget_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct QueryContextRequest {
    pub task: String,
    pub entities: Vec<String>,
    pub namespace: Option<String>,
    pub budget_tokens: u32,
    pub include_evidence: bool,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CandidType)]
pub struct QueryContext {
    pub namespace: String,
    pub task: String,
    pub search_hits: Vec<SearchNodeHit>,
    pub nodes: Vec<NodeContext>,
    pub graph_links: Vec<LinkEdge>,
    pub evidence: Vec<SourceEvidence>,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SourceEvidenceRequest {
    pub node_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SourceEvidenceRef {
    pub source_path: String,
    pub via_path: String,
    pub raw_href: String,
    pub link_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SourceEvidence {
    pub node_path: String,
    pub refs: Vec<SourceEvidenceRef>,
}
