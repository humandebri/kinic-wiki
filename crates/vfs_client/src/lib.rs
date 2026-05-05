// Where: crates/vfs_client/src/lib.rs
// What: Reusable canister client for the VFS public API.
// Why: CLI and non-CLI consumers should share one transport implementation.
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use candid::{Decode, Encode};
use ic_agent::{
    Agent,
    export::Principal,
    identity::{BasicIdentity, Secp256k1Identity},
};
use std::path::Path;
use vfs_types::{
    AppendNodeRequest, CanisterHealth, ChildNode, DeleteNodeRequest, DeleteNodeResult,
    EditNodeRequest, EditNodeResult, ExportSnapshotRequest, ExportSnapshotResponse,
    FetchUpdatesRequest, FetchUpdatesResponse, GlobNodeHit, GlobNodesRequest, GraphLinksRequest,
    GraphNeighborhoodRequest, IncomingLinksRequest, LinkEdge, ListChildrenRequest,
    ListNodesRequest, MemoryManifest, MkdirNodeRequest, MkdirNodeResult, MoveNodeRequest,
    MoveNodeResult, MultiEditNodeRequest, MultiEditNodeResult, Node, NodeContext,
    NodeContextRequest, NodeEntry, OutgoingLinksRequest, PathPolicy, PathPolicyEntry, QueryContext,
    QueryContextRequest, RecentNodeHit, RecentNodesRequest, SearchNodeHit, SearchNodePathsRequest,
    SearchNodesRequest, SourceEvidence, SourceEvidenceRequest, Status, WriteNodeRequest,
    WriteNodeResult,
};

#[async_trait]
pub trait VfsApi: Sync {
    fn local_principal(&self) -> Result<String> {
        Ok("2vxsx-fae".to_string())
    }

    async fn status(&self) -> Result<Status>;
    async fn canister_health(&self) -> Result<CanisterHealth> {
        Err(anyhow!("canister_health is not implemented by this client"))
    }
    async fn memory_manifest(&self) -> Result<MemoryManifest> {
        Err(anyhow!("memory_manifest is not implemented by this client"))
    }
    async fn read_node(&self, path: &str) -> Result<Option<Node>>;
    async fn read_node_context(&self, _request: NodeContextRequest) -> Result<Option<NodeContext>> {
        Err(anyhow!(
            "read_node_context is not implemented by this client"
        ))
    }
    async fn list_nodes(&self, request: ListNodesRequest) -> Result<Vec<NodeEntry>>;
    async fn list_children(&self, request: ListChildrenRequest) -> Result<Vec<ChildNode>>;
    async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult>;
    async fn append_node(&self, request: AppendNodeRequest) -> Result<WriteNodeResult>;
    async fn edit_node(&self, request: EditNodeRequest) -> Result<EditNodeResult>;
    async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult>;
    async fn move_node(&self, request: MoveNodeRequest) -> Result<MoveNodeResult>;
    async fn mkdir_node(&self, request: MkdirNodeRequest) -> Result<MkdirNodeResult>;
    async fn glob_nodes(&self, request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>>;
    async fn recent_nodes(&self, request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>>;
    async fn graph_links(&self, _request: GraphLinksRequest) -> Result<Vec<LinkEdge>> {
        Err(anyhow!("graph_links is not implemented by this client"))
    }
    async fn graph_neighborhood(
        &self,
        _request: GraphNeighborhoodRequest,
    ) -> Result<Vec<LinkEdge>> {
        Err(anyhow!(
            "graph_neighborhood is not implemented by this client"
        ))
    }
    async fn incoming_links(&self, _request: IncomingLinksRequest) -> Result<Vec<LinkEdge>> {
        Err(anyhow!("incoming_links is not implemented by this client"))
    }
    async fn outgoing_links(&self, _request: OutgoingLinksRequest) -> Result<Vec<LinkEdge>> {
        Err(anyhow!("outgoing_links is not implemented by this client"))
    }
    async fn multi_edit_node(&self, request: MultiEditNodeRequest) -> Result<MultiEditNodeResult>;
    async fn search_nodes(&self, request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>>;
    async fn query_context(&self, _request: QueryContextRequest) -> Result<QueryContext> {
        Err(anyhow!("query_context is not implemented by this client"))
    }
    async fn source_evidence(&self, _request: SourceEvidenceRequest) -> Result<SourceEvidence> {
        Err(anyhow!("source_evidence is not implemented by this client"))
    }
    async fn search_node_paths(
        &self,
        request: SearchNodePathsRequest,
    ) -> Result<Vec<SearchNodeHit>>;
    async fn export_snapshot(
        &self,
        request: ExportSnapshotRequest,
    ) -> Result<ExportSnapshotResponse>;
    async fn fetch_updates(&self, request: FetchUpdatesRequest) -> Result<FetchUpdatesResponse>;
    async fn enable_path_policy(&self, _path: &str) -> Result<PathPolicy> {
        Err(anyhow!(
            "enable_path_policy is not implemented by this client"
        ))
    }
    async fn my_path_policy_roles(&self, _path: &str) -> Result<Vec<String>> {
        Err(anyhow!(
            "my_path_policy_roles is not implemented by this client"
        ))
    }
    async fn path_policy_entries(&self, _path: &str) -> Result<Vec<PathPolicyEntry>> {
        Err(anyhow!(
            "path_policy_entries is not implemented by this client"
        ))
    }
    async fn grant_path_policy_role(
        &self,
        _path: &str,
        _principal: String,
        _role: String,
    ) -> Result<()> {
        Err(anyhow!(
            "grant_path_policy_role is not implemented by this client"
        ))
    }
    async fn revoke_path_policy_role(
        &self,
        _path: &str,
        _principal: String,
        _role: String,
    ) -> Result<()> {
        Err(anyhow!(
            "revoke_path_policy_role is not implemented by this client"
        ))
    }
    async fn path_policy(&self, _path: &str) -> Result<PathPolicy> {
        Err(anyhow!("path_policy is not implemented by this client"))
    }
}

#[derive(Clone)]
pub struct CanisterVfsClient {
    agent: Agent,
    canister_id: Principal,
}

impl CanisterVfsClient {
    pub async fn new(replica_host: &str, canister_id: &str) -> Result<Self> {
        Self::new_with_identity(replica_host, canister_id, None).await
    }

    pub async fn new_with_identity(
        replica_host: &str,
        canister_id: &str,
        identity_pem: Option<&Path>,
    ) -> Result<Self> {
        let mut builder = Agent::builder().with_url(replica_host);
        if let Some(path) = identity_pem {
            builder = builder.with_boxed_identity(load_identity(path)?);
        }
        let agent = builder.build().context("failed to build IC agent")?;
        if is_local_replica(replica_host) {
            agent
                .fetch_root_key()
                .await
                .context("failed to fetch local replica root key")?;
        }
        Ok(Self {
            agent,
            canister_id: Principal::from_text(canister_id)
                .context("failed to parse canister principal")?,
        })
    }

    async fn query<Arg, Out>(&self, method: &str, arg: &Arg) -> Result<Out>
    where
        Arg: candid::CandidType,
        Out: for<'de> candid::Deserialize<'de> + candid::CandidType,
    {
        let bytes = self
            .agent
            .query(&self.canister_id, method)
            .with_arg(Encode!(arg).context("failed to encode query args")?)
            .call()
            .await
            .with_context(|| format!("query failed for {method}"))?;
        Decode!(&bytes, Out)
            .with_context(|| format!("failed to decode query response for {method}"))
    }

    async fn update<Arg, Out>(&self, method: &str, arg: &Arg) -> Result<Out>
    where
        Arg: candid::CandidType,
        Out: for<'de> candid::Deserialize<'de> + candid::CandidType,
    {
        let bytes = self
            .agent
            .update(&self.canister_id, method)
            .with_arg(Encode!(arg).context("failed to encode update args")?)
            .call_and_wait()
            .await
            .with_context(|| format!("update failed for {method}"))?;
        Decode!(&bytes, Out)
            .with_context(|| format!("failed to decode update response for {method}"))
    }
}

#[async_trait]
impl VfsApi for CanisterVfsClient {
    fn local_principal(&self) -> Result<String> {
        Ok(self
            .agent
            .get_principal()
            .map_err(|error| anyhow!(error))?
            .to_text())
    }

    async fn status(&self) -> Result<Status> {
        self.query("status", &()).await
    }

    async fn canister_health(&self) -> Result<CanisterHealth> {
        self.query("canister_health", &()).await
    }

    async fn memory_manifest(&self) -> Result<MemoryManifest> {
        self.query("memory_manifest", &()).await
    }

    async fn read_node(&self, path: &str) -> Result<Option<Node>> {
        let result: Result<Option<Node>, String> =
            self.query("read_node", &path.to_string()).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn read_node_context(&self, request: NodeContextRequest) -> Result<Option<NodeContext>> {
        let result: Result<Option<NodeContext>, String> =
            self.query("read_node_context", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn list_nodes(&self, request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
        let result: Result<Vec<NodeEntry>, String> = self.query("list_nodes", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn list_children(&self, request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
        let result: Result<Vec<ChildNode>, String> = self.query("list_children", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
        let result: Result<WriteNodeResult, String> = self.update("write_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn append_node(&self, request: AppendNodeRequest) -> Result<WriteNodeResult> {
        let result: Result<WriteNodeResult, String> = self.update("append_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn edit_node(&self, request: EditNodeRequest) -> Result<EditNodeResult> {
        let result: Result<EditNodeResult, String> = self.update("edit_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
        let result: Result<DeleteNodeResult, String> = self.update("delete_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn move_node(&self, request: MoveNodeRequest) -> Result<MoveNodeResult> {
        let result: Result<MoveNodeResult, String> = self.update("move_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn mkdir_node(&self, request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
        let result: Result<MkdirNodeResult, String> = self.query("mkdir_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn glob_nodes(&self, request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
        let result: Result<Vec<GlobNodeHit>, String> = self.query("glob_nodes", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn recent_nodes(&self, request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
        let result: Result<Vec<RecentNodeHit>, String> =
            self.query("recent_nodes", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn graph_links(&self, request: GraphLinksRequest) -> Result<Vec<LinkEdge>> {
        let result: Result<Vec<LinkEdge>, String> = self.query("graph_links", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn graph_neighborhood(&self, request: GraphNeighborhoodRequest) -> Result<Vec<LinkEdge>> {
        let result: Result<Vec<LinkEdge>, String> =
            self.query("graph_neighborhood", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn incoming_links(&self, request: IncomingLinksRequest) -> Result<Vec<LinkEdge>> {
        let result: Result<Vec<LinkEdge>, String> = self.query("incoming_links", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn outgoing_links(&self, request: OutgoingLinksRequest) -> Result<Vec<LinkEdge>> {
        let result: Result<Vec<LinkEdge>, String> = self.query("outgoing_links", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn multi_edit_node(&self, request: MultiEditNodeRequest) -> Result<MultiEditNodeResult> {
        let result: Result<MultiEditNodeResult, String> =
            self.update("multi_edit_node", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn search_nodes(&self, request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
        let result: Result<Vec<SearchNodeHit>, String> =
            self.query("search_nodes", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn query_context(&self, request: QueryContextRequest) -> Result<QueryContext> {
        let result: Result<QueryContext, String> = self.query("query_context", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn source_evidence(&self, request: SourceEvidenceRequest) -> Result<SourceEvidence> {
        let result: Result<SourceEvidence, String> =
            self.query("source_evidence", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn search_node_paths(
        &self,
        request: SearchNodePathsRequest,
    ) -> Result<Vec<SearchNodeHit>> {
        let result: Result<Vec<SearchNodeHit>, String> =
            self.query("search_node_paths", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn export_snapshot(
        &self,
        request: ExportSnapshotRequest,
    ) -> Result<ExportSnapshotResponse> {
        let result: Result<ExportSnapshotResponse, String> =
            self.update("export_snapshot", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn fetch_updates(&self, request: FetchUpdatesRequest) -> Result<FetchUpdatesResponse> {
        let result: Result<FetchUpdatesResponse, String> =
            self.query("fetch_updates", &request).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn enable_path_policy(&self, path: &str) -> Result<PathPolicy> {
        let result: Result<PathPolicy, String> =
            self.update("enable_path_policy", &path.to_string()).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn my_path_policy_roles(&self, path: &str) -> Result<Vec<String>> {
        self.query("my_path_policy_roles", &path.to_string()).await
    }

    async fn path_policy_entries(&self, path: &str) -> Result<Vec<PathPolicyEntry>> {
        let result: Result<Vec<PathPolicyEntry>, String> =
            self.query("path_policy_entries", &path.to_string()).await?;
        result.map_err(|error| anyhow!(error))
    }

    async fn grant_path_policy_role(
        &self,
        path: &str,
        principal: String,
        role: String,
    ) -> Result<()> {
        let bytes = self
            .agent
            .update(&self.canister_id, "grant_path_policy_role")
            .with_arg(
                Encode!(&path.to_string(), &principal, &role)
                    .context("failed to encode grant_path_policy_role")?,
            )
            .call_and_wait()
            .await
            .context("update failed for grant_path_policy_role")?;
        let result: Result<(), String> = Decode!(&bytes, Result<(), String>)
            .context("failed to decode response for grant_path_policy_role")?;
        result.map_err(|error| anyhow!(error))
    }

    async fn revoke_path_policy_role(
        &self,
        path: &str,
        principal: String,
        role: String,
    ) -> Result<()> {
        let bytes = self
            .agent
            .update(&self.canister_id, "revoke_path_policy_role")
            .with_arg(
                Encode!(&path.to_string(), &principal, &role)
                    .context("failed to encode revoke_path_policy_role")?,
            )
            .call_and_wait()
            .await
            .context("update failed for revoke_path_policy_role")?;
        let result: Result<(), String> = Decode!(&bytes, Result<(), String>)
            .context("failed to decode response for revoke_path_policy_role")?;
        result.map_err(|error| anyhow!(error))
    }

    async fn path_policy(&self, path: &str) -> Result<PathPolicy> {
        self.query("path_policy", &path.to_string()).await
    }
}

fn load_identity(path: &Path) -> Result<Box<dyn ic_agent::Identity>> {
    if let Ok(identity) = Secp256k1Identity::from_pem_file(path) {
        return Ok(Box::new(identity));
    }
    let identity = BasicIdentity::from_pem_file(path)
        .with_context(|| format!("failed to load identity PEM: {}", path.display()))?;
    Ok(Box::new(identity))
}

fn is_local_replica(host: &str) -> bool {
    host.contains("127.0.0.1") || host.contains("localhost")
}
