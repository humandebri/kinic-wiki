use crate::cli::{SkillRunOutcomeArg, SkillStatusArg};
use crate::skill_registry::{
    SkillRunInput, approve_proposal, find_skills, inspect_skill, propose_improvement,
    record_skill_run, set_skill_status, upsert_skill,
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use vfs_client::VfsApi;
use vfs_types::{
    AppendNodeRequest, ChildNode, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest,
    EditNodeResult, ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest,
    FetchUpdatesResponse, GlobNodeHit, GlobNodesRequest, ListChildrenRequest, ListNodesRequest,
    MkdirNodeRequest, MkdirNodeResult, MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest,
    MultiEditNodeResult, Node, NodeEntry, NodeEntryKind, NodeKind, RecentNodeHit,
    RecentNodesRequest, SearchNodeHit, SearchNodePathsRequest, SearchNodesRequest, Status,
    WriteNodeRequest, WriteNodeResult,
};

#[derive(Default)]
struct SkillMockClient {
    nodes: Mutex<BTreeMap<String, Node>>,
    searches: Mutex<Vec<SearchNodesRequest>>,
    writes: AtomicUsize,
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

    async fn list_nodes(&self, request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
        Ok(self
            .nodes
            .lock()
            .expect("nodes lock")
            .values()
            .filter(|node| node.path.starts_with(&request.prefix))
            .map(|node| NodeEntry {
                path: node.path.clone(),
                kind: match node.kind {
                    NodeKind::File => NodeEntryKind::File,
                    NodeKind::Source => NodeEntryKind::Source,
                },
                updated_at: node.updated_at,
                etag: node.etag.clone(),
                has_children: false,
            })
            .collect())
    }

    async fn list_children(&self, _request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
        Ok(Vec::new())
    }

    async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
        let mut nodes = self.nodes.lock().expect("nodes lock");
        let created = !nodes.contains_key(&request.path);
        if let Some(current) = nodes.get(&request.path) {
            if request.expected_etag.as_deref() != Some(current.etag.as_str()) {
                anyhow::bail!(
                    "expected_etag does not match current etag: {}",
                    request.path
                );
            }
        } else if request.expected_etag.is_some() {
            anyhow::bail!("expected_etag must be None for new node: {}", request.path);
        }
        let write_id = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
        let etag = format!("etag-write-{write_id}");
        let node = Node {
            path: request.path.clone(),
            kind: request.kind.clone(),
            content: request.content,
            created_at: 1,
            updated_at: 2,
            etag: etag.clone(),
            metadata_json: request.metadata_json,
        };
        nodes.insert(request.path.clone(), node);
        Ok(WriteNodeResult {
            created,
            node: vfs_types::NodeMutationAck {
                path: request.path,
                kind: request.kind.clone(),
                updated_at: 2,
                etag,
            },
        })
    }

    async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
        unreachable!("skill tests do not append")
    }

    async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
        unreachable!("skill tests do not edit")
    }

    async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
        let mut nodes = self.nodes.lock().expect("nodes lock");
        let Some(current) = nodes.get(&request.path) else {
            anyhow::bail!("node not found: {}", request.path);
        };
        if request.expected_etag.as_deref() != Some(current.etag.as_str()) {
            anyhow::bail!(
                "expected_etag does not match current etag: {}",
                request.path
            );
        }
        nodes.remove(&request.path);
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
        "# Legal Review\n\nReview redlines.\n\nRead [checklist](ingest.md) and [usage](docs/usage.md).\n\nIgnore [web](https://example.com/remote.md), [absolute](/tmp/secret.md), [parent](../outside.md), and [text](notes.txt).",
    );
    write(temp.path(), "ingest.md", "# Ingest\n\nredlines checklist");
    std::fs::create_dir(temp.path().join("docs")).expect("docs dir");
    write(
        temp.path(),
        "docs/usage.md",
        "# Usage\n\ncontract review usage",
    );
    std::fs::write(
        temp.path().parent().unwrap().join("outside.md"),
        "# Outside",
    )
    .expect("outside");
    write(temp.path(), "manifest.md", &manifest("reviewed"));

    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("upsert");
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/SKILL.md")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/ingest.md")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/docs/usage.md")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/outside.md")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/provenance.md")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/evals.md")
            .await
            .unwrap()
            .is_none()
    );
    write(
        temp.path(),
        "SKILL.md",
        "# Legal Review\n\nReview redlines and contract risks.",
    );
    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("second upsert updates existing skill");
    let updated_skill = client
        .read_node("default", "/Wiki/skills/legal-review/SKILL.md")
        .await
        .expect("read updated skill")
        .expect("skill exists")
        .content;
    assert!(updated_skill.contains("contract risks"));
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/ingest.md")
            .await
            .unwrap()
            .is_some(),
        "stale package files are retained without explicit prune"
    );
    let pruned = upsert_skill(&client, "default", temp.path(), "legal-review", false, true)
        .await
        .expect("prune upsert");
    assert_eq!(
        pruned["pruned_paths"],
        serde_json::json!([
            "/Wiki/skills/legal-review/docs/usage.md",
            "/Wiki/skills/legal-review/ingest.md"
        ])
    );
    assert!(
        client
            .read_node("default", "/Wiki/skills/legal-review/ingest.md")
            .await
            .unwrap()
            .is_none(),
        "explicit prune removes files no longer present in the source package"
    );

    let found = find_skills(&client, "default", "redlines", false, 10)
        .await
        .expect("find");
    assert_eq!(found["hits"][0]["id"], "legal-review");
    assert_eq!(found["hits"][0]["status"], "reviewed");

    let inspected = inspect_skill(&client, "default", "legal-review", false)
        .await
        .expect("inspect");
    assert_eq!(inspected["files"]["evals.md"], false);
    assert_eq!(inspected["files"]["provenance.md"], false);
    assert!(inspected["files"]["ingest.md"].is_null());
    assert!(inspected["files"]["docs/usage.md"].is_null());

    set_skill_status(
        &client,
        "default",
        "legal-review",
        SkillStatusArg::Deprecated,
        None,
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
        .read_node("default", "/Wiki/skills/legal-review/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(updated_manifest.contains("status: deprecated"));

    let notes = temp.path().join("notes.md");
    std::fs::write(&notes, "worked on contract").expect("notes");
    let run = record_skill_run(
        &client,
        SkillRunInput {
            database_id: "default",
            id: "legal-review",
            task: "review contract",
            outcome: SkillRunOutcomeArg::Success,
            notes_file: &notes,
            agent: "cli",
            public: false,
        },
    )
    .await
    .expect("record run");
    assert!(
        run["run_path"]
            .as_str()
            .unwrap()
            .starts_with("/Sources/skill-runs/legal-review/")
    );
    let run_node = client
        .read_node("default", run["run_path"].as_str().unwrap())
        .await
        .expect("read run")
        .expect("run exists")
        .content;
    assert!(run_node.contains("schema_version: 1"));
    assert!(run_node.contains("skill_hash: "));
    assert!(run_node.contains("manifest_hash: "));
    assert!(run_node.contains("task_hash: "));
    assert!(run_node.contains("agent: cli"));

    let shown = find_skills(&client, "default", "redlines", true, 10)
        .await
        .expect("find with run summary");
    assert_eq!(shown["hits"][0]["run_summary"]["runs"], 1);
    assert_eq!(shown["hits"][0]["run_summary"]["success"], 1);

    let inspected = inspect_skill(&client, "default", "legal-review", false)
        .await
        .expect("inspect with run summary");
    assert_eq!(inspected["run_summary"]["runs"], 1);
}

#[tokio::test]
async fn skill_set_status_preserves_manifest_body_and_unknown_frontmatter() {
    let client = SkillMockClient::default();
    let manifest_path = "/Wiki/skills/legal-review/manifest.md";
    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: manifest_path.to_string(),
            kind: NodeKind::File,
            content: concat!(
                "---\n",
                "kind: kinic.skill\n",
                "schema_version: 1\n",
                "id: legal-review\n",
                "version: 0.1.0\n",
                "x-team: acme\n",
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
        "legal-review",
        SkillStatusArg::Promoted,
        None,
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
async fn skill_upsert_uses_skill_frontmatter_to_fill_missing_manifest_fields() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(
        temp.path(),
        "SKILL.md",
        concat!(
            "---\n",
            "name: canister-security\n",
            "description: IC-specific security patterns for canister development\n",
            "license: Apache-2.0\n",
            "metadata:\n",
            "  title: Canister Security\n",
            "  category: Security\n",
            "---\n",
            "# Canister Security\n"
        ),
    );

    upsert_skill(
        &client,
        "default",
        temp.path(),
        "canister-security",
        false,
        false,
    )
    .await
    .expect("upsert");

    let manifest = client
        .read_node("default", "/Wiki/skills/canister-security/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(manifest.contains("title: Canister Security"));
    assert!(manifest.contains("summary: IC-specific security patterns for canister development"));
    assert!(manifest.contains("- Security"));
    assert!(manifest.contains("status: draft"));
    assert!(manifest.contains("license: Apache-2.0"));

    let found = find_skills(&client, "default", "security", false, 10)
        .await
        .expect("find");
    assert_eq!(found["hits"][0]["id"], "canister-security");
    assert_eq!(found["hits"][0]["title"], "Canister Security");
    let inspected = inspect_skill(&client, "default", "canister-security", false)
        .await
        .expect("inspect");
    assert_eq!(inspected["manifest"]["title"], "Canister Security");
}

#[tokio::test]
async fn skill_upsert_preserves_existing_manifest_fields_over_skill_frontmatter() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(
        temp.path(),
        "SKILL.md",
        concat!(
            "---\n",
            "name: legal-review\n",
            "description: Upstream description\n",
            "license: Apache-2.0\n",
            "metadata:\n",
            "  title: Upstream Title\n",
            "  category: Upstream\n",
            "---\n",
            "# Legal Review\n"
        ),
    );
    write(
        temp.path(),
        "manifest.md",
        concat!(
            "---\n",
            "kind: kinic.skill\n",
            "schema_version: 1\n",
            "id: legal-review\n",
            "version: 0.1.0\n",
            "entry: SKILL.md\n",
            "title: KB Title\n",
            "summary: KB summary\n",
            "tags:\n",
            "  - kb-tag\n",
            "status: reviewed\n",
            "provenance:\n",
            "  license: MIT\n",
            "---\n",
            "# Skill Manifest\n"
        ),
    );

    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("upsert");

    let manifest = client
        .read_node("default", "/Wiki/skills/legal-review/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(manifest.contains("title: KB Title"));
    assert!(manifest.contains("summary: KB summary"));
    assert!(manifest.contains("- kb-tag"));
    assert!(manifest.contains("license: MIT"));
    assert!(!manifest.contains("Upstream Title"));
    assert!(!manifest.contains("Upstream description"));
    assert!(!manifest.contains("- Upstream"));
    assert!(!manifest.contains("Apache-2.0"));
}

#[tokio::test]
async fn skill_upsert_allows_upstream_frontmatter_name_to_differ_from_db_id() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(
        temp.path(),
        "SKILL.md",
        concat!(
            "---\n",
            "name: react:components\n",
            "description: React component workflow\n",
            "license: Apache-2.0\n",
            "metadata:\n",
            "  title: React Components\n",
            "  category: React\n",
            "---\n",
            "# React Components\n"
        ),
    );

    upsert_skill(
        &client,
        "default",
        temp.path(),
        "react-components",
        false,
        false,
    )
    .await
    .expect("upstream name does not need to match DB id");
    let manifest = client
        .read_node("default", "/Wiki/skills/react-components/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert!(manifest.contains("id: react-components"));
    assert!(manifest.contains("title: React Components"));
    assert!(manifest.contains("summary: React component workflow"));
    assert!(manifest.contains("- React"));
    assert!(manifest.contains("license: Apache-2.0"));
}

#[tokio::test]
async fn skill_set_status_adds_missing_root_status_without_touching_body() {
    let client = SkillMockClient::default();
    let manifest_path = "/Wiki/skills/legal-review/manifest.md";
    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: manifest_path.to_string(),
            kind: NodeKind::File,
            content: concat!(
                "---\n",
                "kind: kinic.skill\n",
                "schema_version: 1\n",
                "id: legal-review\n",
                "version: 0.1.0\n",
                "x-team: acme\n",
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
        "legal-review",
        SkillStatusArg::Draft,
        None,
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

#[tokio::test]
async fn skill_set_status_records_deprecated_reason() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(temp.path(), "SKILL.md", "# Legal Review\n\nredlines");
    write(temp.path(), "manifest.md", &manifest("reviewed"));
    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("upsert");

    set_skill_status(
        &client,
        "default",
        "legal-review",
        SkillStatusArg::Deprecated,
        Some("replaced by safer workflow"),
        false,
    )
    .await
    .expect("set deprecated");

    let found = find_skills(&client, "default", "redlines", true, 10)
        .await
        .expect("find deprecated");
    assert_eq!(
        found["hits"][0]["deprecated_reason"],
        "replaced by safer workflow"
    );
    let manifest = client
        .read_node("default", "/Wiki/skills/legal-review/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert_rfc3339_field(&manifest, "deprecated_at");
}

#[tokio::test]
async fn skill_set_status_records_promoted_at_as_rfc3339() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(temp.path(), "SKILL.md", "# Legal Review\n\nredlines");
    write(temp.path(), "manifest.md", &manifest("reviewed"));
    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("upsert");

    set_skill_status(
        &client,
        "default",
        "legal-review",
        SkillStatusArg::Promoted,
        None,
        false,
    )
    .await
    .expect("set promoted");

    let manifest = client
        .read_node("default", "/Wiki/skills/legal-review/manifest.md")
        .await
        .expect("read manifest")
        .expect("manifest exists")
        .content;
    assert_rfc3339_field(&manifest, "promoted_at");
}

#[tokio::test]
async fn skill_improvement_proposal_is_recorded_and_approved_without_editing_skill() {
    let client = SkillMockClient::default();
    let temp = tempfile::tempdir().expect("tempdir");
    write(temp.path(), "SKILL.md", "# Legal Review\n\nredlines");
    write(temp.path(), "manifest.md", &manifest("reviewed"));
    upsert_skill(
        &client,
        "default",
        temp.path(),
        "legal-review",
        false,
        false,
    )
    .await
    .expect("upsert");

    let diff = temp.path().join("change.diff");
    std::fs::write(&diff, "- old\n+ new\n").expect("diff");
    let proposal = propose_improvement(
        &client,
        "default",
        "legal-review",
        &["/Sources/skill-runs/legal-review/1.md".to_string()],
        "Tighten contract risk checklist",
        &diff,
        false,
    )
    .await
    .expect("proposal");
    let proposal_path = proposal["proposal_path"].as_str().unwrap();
    let proposal_name = proposal_path
        .strip_prefix("/Wiki/skills/legal-review/improvement-proposals/")
        .expect("proposal path prefix")
        .strip_suffix(".md")
        .expect("proposal extension");
    assert!(proposal_name.chars().all(|ch| ch.is_ascii_digit()));
    let skill_before = client
        .read_node("default", "/Wiki/skills/legal-review/SKILL.md")
        .await
        .unwrap()
        .unwrap()
        .content;

    approve_proposal(&client, "default", "legal-review", proposal_path)
        .await
        .expect("approve");
    let proposal_content = client
        .read_node("default", proposal_path)
        .await
        .unwrap()
        .unwrap()
        .content;
    assert_rfc3339_field(&proposal_content, "created_at");
    let skill_after = client
        .read_node("default", "/Wiki/skills/legal-review/SKILL.md")
        .await
        .unwrap()
        .unwrap()
        .content;
    assert!(proposal_content.contains("status: approved"));
    assert_eq!(skill_before, skill_after);
}

#[tokio::test]
async fn skill_approve_proposal_rejects_wrong_path_and_frontmatter() {
    let client = SkillMockClient::default();
    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: "/Wiki/skills/other/improvement-proposals/1.md".to_string(),
            kind: NodeKind::File,
            content: proposal_content("legal-review", "proposed"),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed wrong path");
    assert!(
        approve_proposal(
            &client,
            "default",
            "legal-review",
            "/Wiki/skills/other/improvement-proposals/1.md"
        )
        .await
        .is_err()
    );

    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: "/Wiki/skills/legal-review/improvement-proposals/1.md".to_string(),
            kind: NodeKind::File,
            content: proposal_content("other", "proposed"),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed wrong skill");
    assert!(
        approve_proposal(
            &client,
            "default",
            "legal-review",
            "/Wiki/skills/legal-review/improvement-proposals/1.md"
        )
        .await
        .is_err()
    );

    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: "/Wiki/skills/legal-review/improvement-proposals/2.md".to_string(),
            kind: NodeKind::File,
            content: proposal_content("legal-review", "approved"),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed approved");
    assert!(
        approve_proposal(
            &client,
            "default",
            "legal-review",
            "/Wiki/skills/legal-review/improvement-proposals/2.md"
        )
        .await
        .is_err()
    );

    client
        .write_node(WriteNodeRequest {
            database_id: "default".to_string(),
            path: "/Wiki/skills/legal-review/SKILL.md".to_string(),
            kind: NodeKind::File,
            content: proposal_content("legal-review", "proposed"),
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await
        .expect("seed non proposal path");
    assert!(
        approve_proposal(
            &client,
            "default",
            "legal-review",
            "/Wiki/skills/legal-review/SKILL.md"
        )
        .await
        .is_err()
    );
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).expect("write fixture");
}

fn assert_rfc3339_field(content: &str, key: &str) {
    let prefix = format!("{key}: ");
    let value = content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("{key} should exist"));
    DateTime::parse_from_rfc3339(value).expect("timestamp should be RFC3339");
    assert!(value.ends_with('Z'));
}

fn proposal_content(skill_id: &str, status: &str) -> String {
    format!(
        "---\nkind: kinic.skill_improvement_proposal\nschema_version: 1\nskill_id: {skill_id}\nstatus: {status}\ncreated_at: 2026-05-08T00:00:00Z\n---\n# Proposal\n"
    )
}

fn manifest(status: &str) -> String {
    format!(
        concat!(
            "---\n",
            "kind: kinic.skill\n",
            "schema_version: 1\n",
            "id: legal-review\n",
            "version: 0.1.0\n",
            "x-team: acme\n",
            "entry: SKILL.md\n",
            "summary: Contract review workflow for spotting redlines, risk clauses, and missing approval context\n",
            "tags:\n",
            "  - legal\n",
            "  - contract\n",
            "  - review\n",
            "  - risk\n",
            "use_cases:\n",
            "  - Review vendor contract redlines before counsel handoff\n",
            "  - Summarize risky clauses and negotiation blockers\n",
            "  - Check whether approval, renewal, and liability terms are documented\n",
            "status: {status}\n",
            "replaces: []\n",
            "related:\n",
            "  - /Wiki/legal/contract-review-playbook.md\n",
            "  - /Sources/github/legal-review\n",
            "knowledge:\n",
            "  - /Wiki/legal/contract-review-playbook.md\n",
            "permissions:\n",
            "  file_read: true\n",
            "  network: false\n",
            "  shell: false\n",
            "provenance:\n",
            "  source: github.com/legal-review\n",
            "  source_ref: demo\n",
            "---\n",
            "# Skill Manifest\n"
        ),
        status = status
    )
}
