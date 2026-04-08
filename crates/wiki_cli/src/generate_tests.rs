// Where: crates/wiki_cli/src/generate_tests.rs
// What: Tests for high-level draft generation.
// Why: Draft generation needs deterministic page maps and review-ready file output.
use crate::cli::{GenerateModeArg, GenerateOutputArg};
use crate::client::WikiApi;
use crate::generate::{GenerateDraftRequest, generate_draft};
use async_trait::async_trait;
use std::fs;
use tempfile::tempdir;
use wiki_types::{
    AdoptDraftPageInput, AdoptDraftPageOutput, CommitWikiChangesRequest, CommitWikiChangesResponse,
    CreateSourceInput, ExportWikiSnapshotRequest, ExportWikiSnapshotResponse,
    FetchWikiUpdatesRequest, FetchWikiUpdatesResponse, PageBundle, SearchHit, SearchRequest,
    Status, SystemPage, WikiPageType,
};

#[derive(Default)]
struct FakeApi {
    search_hits: Vec<SearchHit>,
}

#[async_trait]
impl WikiApi for FakeApi {
    async fn adopt_draft_page(
        &self,
        _request: AdoptDraftPageInput,
    ) -> anyhow::Result<AdoptDraftPageOutput> {
        panic!("not used in generate tests")
    }

    async fn create_source(&self, _request: CreateSourceInput) -> anyhow::Result<String> {
        panic!("not used in generate tests")
    }

    async fn lint_health(&self) -> anyhow::Result<wiki_types::HealthCheckReport> {
        panic!("not used in generate tests")
    }

    async fn status(&self) -> anyhow::Result<Status> {
        Ok(Status {
            page_count: 0,
            source_count: 0,
            system_page_count: 0,
        })
    }

    async fn search(&self, _request: SearchRequest) -> anyhow::Result<Vec<SearchHit>> {
        Ok(self.search_hits.clone())
    }

    async fn get_page(&self, _slug: &str) -> anyhow::Result<Option<PageBundle>> {
        Ok(None)
    }

    async fn get_system_page(&self, _slug: &str) -> anyhow::Result<Option<SystemPage>> {
        Ok(None)
    }

    async fn export_wiki_snapshot(
        &self,
        _request: ExportWikiSnapshotRequest,
    ) -> anyhow::Result<ExportWikiSnapshotResponse> {
        panic!("not used in generate tests")
    }

    async fn fetch_wiki_updates(
        &self,
        _request: FetchWikiUpdatesRequest,
    ) -> anyhow::Result<FetchWikiUpdatesResponse> {
        panic!("not used in generate tests")
    }

    async fn commit_wiki_changes(
        &self,
        _request: CommitWikiChangesRequest,
    ) -> anyhow::Result<CommitWikiChangesResponse> {
        panic!("not used in generate tests")
    }
}

#[tokio::test]
async fn generate_draft_creates_single_review_ready_page() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let input_path = dir.path().join("agent-memory.md");
    fs::write(
        &input_path,
        "# Agent Memory\n\nSee [Wiki Sync](./wiki-sync.md).\n",
    )
    .unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path: vault_path.clone(),
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page_map.len(), 1);
    assert_eq!(response.page_map[0].page_type, WikiPageType::Entity);
    let page = fs::read_to_string(vault_path.join("Wiki/pages/agent-memory.md")).unwrap();
    assert!(page.contains("draft: true"));
    assert!(page.contains("page_type: entity"));
    assert!(page.contains("# Agent Memory"));
}

#[tokio::test]
async fn generate_draft_creates_overview_for_multiple_inputs() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let docs_dir = dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    let alpha = docs_dir.join("alpha.md");
    let beta = docs_dir.join("beta.md");
    fs::write(&alpha, "# Alpha\n\nBody.\n").unwrap();
    fs::write(&beta, "# Beta\n\nSee [Alpha](./alpha.md)\n").unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path: vault_path.clone(),
            mirror_root: "Wiki".to_string(),
            inputs: vec![alpha, beta],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page_map.len(), 3);
    let overview = fs::read_to_string(vault_path.join("Wiki/pages/docs-overview.md")).unwrap();
    assert!(overview.contains("[[alpha]]"));
    assert!(overview.contains("Alpha [entity]"));
    let beta_page = fs::read_to_string(vault_path.join("Wiki/pages/beta.md")).unwrap();
    assert!(beta_page.contains("[[alpha]]"));
}

#[tokio::test]
async fn generate_draft_reports_possible_duplicate() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let input_path = dir.path().join("alpha.md");
    fs::write(&input_path, "# Alpha\n\nBody.\n").unwrap();

    let response = generate_draft(
        &FakeApi {
            search_hits: vec![SearchHit {
                slug: "alpha".to_string(),
                title: "Alpha".to_string(),
                page_type: WikiPageType::Entity,
                section_path: None,
                snippet: "Alpha".to_string(),
                score: 1.0,
                match_reasons: vec!["exact slug".to_string()],
            }],
        },
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert_eq!(response.open_questions.len(), 1);
    assert!(response.open_questions[0].contains("exact slug collision"));
}

#[tokio::test]
async fn generate_draft_infers_query_and_comparison_types_from_signals() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let query_path = dir.path().join("open-question.md");
    let compare_path = dir.path().join("sync-vs-rag.md");
    fs::write(&query_path, "# Open Question\n\nInvestigation notes.\n").unwrap();
    fs::write(&compare_path, "# Sync vs RAG\n\nTradeoff summary.\n").unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![query_path, compare_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert!(
        response.page_map.iter().any(
            |entry| entry.slug == "open-question" && entry.page_type == WikiPageType::QueryNote
        )
    );
    assert!(response
        .page_map
        .iter()
        .any(|entry| entry.slug == "sync-vs-rag" && entry.page_type == WikiPageType::Comparison));
}

#[tokio::test]
async fn generate_draft_infers_concept_from_heading_and_intro() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let input_path = dir.path().join("workflow.md");
    fs::write(
        &input_path,
        "# Coordination Workflow\n\nThis mechanism explains the system.\n",
    )
    .unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page_map[0].page_type, WikiPageType::Concept);
}

#[tokio::test]
async fn generate_draft_uses_intro_and_source_summary_signal() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let input_path = dir.path().join("paper-summary.md");
    fs::write(&input_path, "A useful note about a paper.\n").unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path: vault_path.clone(),
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert_eq!(response.page_map[0].page_type, WikiPageType::SourceSummary);
    let page = fs::read_to_string(vault_path.join("Wiki/pages/paper-summary.md")).unwrap();
    assert!(page.contains("This draft captures the main points"));
    assert!(page.contains("# Paper Summary"));
}

#[tokio::test]
async fn generate_draft_reports_local_title_collision() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let alpha_one = dir.path().join("alpha-one.md");
    let alpha_two = dir.path().join("alpha-two.md");
    fs::write(&alpha_one, "# Shared Title\n\nBody.\n").unwrap();
    fs::write(&alpha_two, "# Shared Title\n\nOther body.\n").unwrap();

    let response = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![alpha_one, alpha_two],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert!(
        response
            .open_questions
            .iter()
            .any(|question| question.contains("title collision in local draft set"))
    );
}

#[tokio::test]
async fn generate_draft_reports_remote_title_collision_and_overlap() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let input_path = dir.path().join("beta.md");
    fs::write(&input_path, "# Beta\n\nBody.\n").unwrap();

    let response = generate_draft(
        &FakeApi {
            search_hits: vec![
                SearchHit {
                    slug: "other-beta".to_string(),
                    title: "Beta".to_string(),
                    page_type: WikiPageType::Entity,
                    section_path: None,
                    snippet: "Beta".to_string(),
                    score: 1.0,
                    match_reasons: vec!["title".to_string()],
                },
                SearchHit {
                    slug: "neighbor".to_string(),
                    title: "Neighbor".to_string(),
                    page_type: WikiPageType::Entity,
                    section_path: None,
                    snippet: "nearby".to_string(),
                    score: 0.6,
                    match_reasons: vec!["overlap".to_string()],
                },
            ],
        },
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap();

    assert!(
        response
            .open_questions
            .iter()
            .any(|question| question.contains("title collision with remote page"))
    );
}

#[tokio::test]
async fn generate_draft_rejects_existing_tracked_local_mirror_page() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault");
    let pages_dir = vault_path.join("Wiki/pages");
    fs::create_dir_all(&pages_dir).unwrap();
    fs::write(
        pages_dir.join("alpha.md"),
        "---\npage_id: page_1\nslug: alpha\npage_type: entity\nrevision_id: rev_1\nupdated_at: 1\nmirror: true\n---\n\n# Alpha\n",
    )
    .unwrap();
    let input_path = dir.path().join("alpha.md");
    fs::write(&input_path, "# Alpha\n\nBody.\n").unwrap();

    let error = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path,
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::Direct,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("tracked local mirror page already exists")
    );
}

#[tokio::test]
async fn graph_assisted_mode_is_not_implemented() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("alpha.md");
    fs::write(&input_path, "# Alpha\n\nBody.\n").unwrap();

    let error = generate_draft(
        &FakeApi::default(),
        GenerateDraftRequest {
            vault_path: dir.path().join("vault"),
            mirror_root: "Wiki".to_string(),
            inputs: vec![input_path],
            mode: GenerateModeArg::GraphAssisted,
            output: GenerateOutputArg::LocalDraft,
        },
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("not implemented"));
}
