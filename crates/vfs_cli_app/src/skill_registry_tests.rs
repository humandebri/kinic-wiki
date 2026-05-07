use crate::cli::{SkillRunOutcomeArg, SkillStatusArg};
use crate::skill_registry::{
    find_skills, inspect_skill, record_skill_run, set_skill_status, upsert_skill,
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use vfs_client::VfsApi;
use vfs_types::{
    AppendNodeRequest, ChildNode, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest,
    EditNodeResult, ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest,
    FetchUpdatesResponse, GlobNodeHit, GlobNodesRequest, ListChildrenRequest, ListNodesRequest,
    MkdirNodeRequest, MkdirNodeResult, MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest,
    MultiEditNodeResult, Node, NodeEntry, NodeKind, RecentNodeHit, RecentNodesRequest,
    SearchNodeHit, SearchNodePathsRequest, SearchNodesRequest, Status, WriteNodeRequest,
    WriteNodeResult,
};

#[derive(Default)]
struct SkillMockClient {
    nodes: Mutex<BTreeMap<String, Node>>,
    searches: Mutex<Vec<SearchNodesRequest>>,
}

#[async_trait]
impl VfsApi for SkillMockClient {
    async fn status(&self, _database_id: &str) -> Result<Status> {
        Ok(Status {
            file_count: 0,
            source_count: 0,
        })
    }

    async fn read_node(&self, _database_id: &str, path: &str) -> Result<Option<Node>> {
        Ok(self.nodes.lock().expect("nodes lock").get(path).cloned())
    }

    async fn list_nodes(&self, _request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
        Ok(Vec::new())
    }

    async fn list_children(&self, _request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
        Ok(Vec::new())
    }

    async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
        let node = Node {
            path: request.path.clone(),
            kind: request.kind,
            content: request.content,
            created_at: 1,
            updated_at: 2,
            etag: "etag-write".to_string(),
            metadata_json: request.metadata_json,
        };
        self.nodes
            .lock()
            .expect("nodes lock")
            .insert(request.path.clone(), node);
        Ok(WriteNodeResult {
            created: true,
            node: ack(&request.path),
        })
    }

    async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
        unreachable!("skill tests do not append")
    }

    async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
        unreachable!("skill tests do not edit")
    }

    async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
        Ok(DeleteNodeResult { path: request.path })
    }

    async fn move_node(&self, _request: MoveNodeRequest) -> Result<MoveNodeResult> {
        unreachable!("skill tests do not move")
    }

    async fn mkdir_node(&self, request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
        Ok(MkdirNodeResult {
            path: request.path,
            created: true,
        })
    }

    async fn glob_nodes(&self, _request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
        Ok(Vec::new())
    }

    async fn recent_nodes(&self, request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
        let prefix = request.path.unwrap_or_default();
        Ok(self
            .nodes
            .lock()
            .expect("nodes lock")
            .values()
            .filter(|node| node.path.starts_with(&prefix))
            .map(|node| RecentNodeHit {
                path: node.path.clone(),
                kind: node.kind.clone(),
                updated_at: node.updated_at,
                etag: node.etag.clone(),
            })
            .collect())
    }

    async fn multi_edit_node(&self, _request: MultiEditNodeRequest) -> Result<MultiEditNodeResult> {
        unreachable!("skill tests do not multi-edit")
    }

    async fn search_nodes(&self, request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
        self.searches
            .lock()
            .expect("search lock")
            .push(request.clone());
        let prefix = request.prefix.unwrap_or_default();
        Ok(self
            .nodes
            .lock()
            .expect("nodes lock")
            .values()
            .filter(|node| {
                node.path.starts_with(&prefix) && node.content.contains(&request.query_text)
            })
            .map(|node| SearchNodeHit {
                path: node.path.clone(),
                kind: node.kind.clone(),
                snippet: Some(node.path.clone()),
                preview: None,
                score: 1.0,
                match_reasons: vec!["content".to_string()],
            })
            .collect())
    }

    async fn search_node_paths(
        &self,
        _request: SearchNodePathsRequest,
    ) -> Result<Vec<SearchNodeHit>> {
        Ok(Vec::new())
    }

    async fn export_snapshot(
        &self,
        _request: ExportSnapshotRequest,
    ) -> Result<ExportSnapshotResponse> {
        Ok(ExportSnapshotResponse {
            snapshot_revision: "snap".to_string(),
            snapshot_session_id: None,
            nodes: Vec::new(),
            next_cursor: None,
        })
    }

    async fn fetch_updates(&self, _request: FetchUpdatesRequest) -> Result<FetchUpdatesResponse> {
        Ok(FetchUpdatesResponse {
            snapshot_revision: "snap".to_string(),
            changed_nodes: Vec::new(),
            removed_paths: Vec::new(),
            next_cursor: None,
        })
    }
}

#[tokio::test]
async fn skill_upsert_find_inspect_status_and_run_use_vfs_nodes() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(
        temp.path(),
        "SKILL.md",
        "# Legal Review\n\nReview redlines.",
    );
    write(temp.path(), "manifest.md", &manifest("reviewed"));

    upsert_skill(&client, "default", temp.path(), "acme/legal-review", false)
        .await
        .expect("upsert");
    assert!(
        client
            .read_node("default", "/Wiki/skills/acme/legal-review/SKILL.md")
            .await
            .unwrap()
            .is_some()
    );

    let found = find_skills(&client, "default", "redlines", false, 10)
        .await
        .expect("find");
    assert_eq!(found["hits"][0]["id"], "acme/legal-review");
    assert_eq!(found["hits"][0]["status"], "reviewed");

    let inspected = inspect_skill(&client, "default", "acme/legal-review", false)
        .await
        .expect("inspect");
    assert_eq!(inspected["files"]["evals.md"], true);

    set_skill_status(
        &client,
        "default",
        "acme/legal-review",
        SkillStatusArg::Deprecated,
        false,
    )
    .await
    .expect("set status");
    let hidden = find_skills(&client, "default", "redlines", false, 10)
        .await
        .expect("find");
    assert_eq!(hidden["hits"].as_array().unwrap().len(), 0);
    let shown = find_skills(&client, "default", "redlines", true, 10)
        .await
        .expect("find");
    assert_eq!(shown["hits"][0]["status"], "deprecated");
    let updated_manifest = client
        .read_node("default", "/Wiki/skills/acme/legal-review/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(updated_manifest.contains("status: deprecated"));

    let notes = temp.path().join("notes.md");
    std::fs::write(&notes, "worked on contract").expect("notes");
    let run = record_skill_run(
        &client,
        "default",
        "acme/legal-review",
        "review contract",
        SkillRunOutcomeArg::Success,
        &notes,
    )
    .await
    .expect("record run");
    assert!(
        run["run_path"]
            .as_str()
            .unwrap()
            .starts_with("/Sources/skill-runs/acme/legal-review/")
    );
}

#[tokio::test]
async fn skill_set_status_preserves_manifest_body_and_unknown_frontmatter() {
    let client = SkillMockClient::default();
    let manifest_path = "/Wiki/skills/acme/legal-review/manifest.md";
    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: manifest_path.to_string(),
            kind: NodeKind::File,
            content: concat!(
                "---\n",
                "kind: kinic.skill\n",
                "schema_version: 1\n",
                "id: acme/legal-review\n",
                "version: 0.1.0\n",
                "publisher: acme\n",
                "entry: SKILL.md\n",
                "x-team-note: keep this\n",
                "provenance:\n",
                "  status: upstream-reviewed\n",
                "status: reviewed # old comment\n",
                "---\n",
                "# Skill Manifest\n",
                "\n",
                "Human-maintained notes stay here.\n"
            )
            .to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed manifest");

    set_skill_status(
        &client,
        "default",
        "acme/legal-review",
        SkillStatusArg::Promoted,
        false,
    )
    .await
    .expect("set status");

    let updated = client
        .read_node("default", manifest_path)
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(updated.contains("x-team-note: keep this"));
    assert!(updated.contains("  status: upstream-reviewed"));
    assert!(updated.contains("status: promoted\n"));
    assert!(updated.contains("# Skill Manifest\n\nHuman-maintained notes stay here.\n"));
    assert!(!updated.contains("status: reviewed # old comment"));
}

#[tokio::test]
async fn skill_set_status_adds_missing_root_status_without_touching_body() {
    let client = SkillMockClient::default();
    let manifest_path = "/Wiki/skills/acme/legal-review/manifest.md";
    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: manifest_path.to_string(),
            kind: NodeKind::File,
            content: concat!(
                "---\n",
                "kind: kinic.skill\n",
                "schema_version: 1\n",
                "id: acme/legal-review\n",
                "version: 0.1.0\n",
                "publisher: acme\n",
                "entry: SKILL.md\n",
                "provenance:\n",
                "  status: upstream-reviewed\n",
                "---\n",
                "# Body\n"
            )
            .to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed manifest");

    set_skill_status(
        &client,
        "default",
        "acme/legal-review",
        SkillStatusArg::Draft,
        false,
    )
    .await
    .expect("set status");

    let updated = client
        .read_node("default", manifest_path)
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(updated.contains("  status: upstream-reviewed\nstatus: draft\n---\n# Body\n"));
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).expect("write fixture");
}

fn manifest(status: &str) -> String {
    format!(
        "---\nkind: kinic.skill\nschema_version: 1\nid: acme/legal-review\nversion: 0.1.0\npublisher: acme\nentry: SKILL.md\nsummary: Contract review\ntags:\n  - legal\nuse_cases:\n  - Review redlines\nstatus: {status}\n---\n# Skill Manifest\n"
    )
}

fn ack(path: &str) -> vfs_types::NodeMutationAck {
    vfs_types::NodeMutationAck {
        path: path.to_string(),
        kind: NodeKind::File,
        updated_at: 2,
        etag: "etag".to_string(),
    }
}
