// Where: crates/vfs_cli_app/src/conversation_wiki.rs
// What: Generate a minimal conversation wiki scope from a persisted raw source node.
// Why: Chrome capture should only persist evidence; wiki pages are created on demand.
use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use vfs_client::VfsApi;
use vfs_types::{NodeKind, WriteNodeRequest};
use wiki_domain::RAW_SOURCES_PREFIX;

const CONVERSATION_WIKI_PREFIX: &str = "/Wiki/conversations";
const GENERATED_METADATA_JSON: &str = r#"{"generated_by":"conversation_wiki","schema_version":1}"#;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawConversation {
    source_path: String,
    source_id: String,
    provider: String,
    source_url: String,
    captured_at: String,
    title: String,
    message_count: usize,
}

#[derive(Debug, Deserialize)]
struct RawConversationMetadata {
    provider: String,
    source_url: String,
    conversation_title: String,
    captured_at: String,
    source_id: String,
    message_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WikiDocument {
    path: String,
    content: String,
    overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationWikiResult {
    pub source_path: String,
    pub base_path: String,
    pub written_paths: Vec<String>,
}

pub async fn generate_conversation_wiki(
    client: &impl VfsApi,
    source_path: &str,
    force: bool,
) -> Result<ConversationWikiResult> {
    let source = client
        .read_node(source_path)
        .await?
        .ok_or_else(|| anyhow!("source node not found: {source_path}"))?;
    if source.kind != NodeKind::Source {
        return Err(anyhow!("node is not a source: {source_path}"));
    }
    let raw = parse_raw_conversation(&source.path, &source.content, &source.metadata_json)?;
    let base_path = format!("{CONVERSATION_WIKI_PREFIX}/{}", raw.source_id);
    let documents = build_wiki_documents(&raw, &base_path);

    let mut written_paths = Vec::with_capacity(documents.len() + 1);
    for document in documents {
        if upsert_file(
            client,
            &document.path,
            &document.content,
            document.overwrite || force,
        )
        .await?
        {
            written_paths.push(document.path);
        }
    }
    let log_path = write_log_document(client, &base_path, &raw).await?;
    written_paths.push(log_path);

    Ok(ConversationWikiResult {
        source_path: raw.source_path,
        base_path,
        written_paths,
    })
}

fn parse_raw_conversation(
    source_path: &str,
    content: &str,
    metadata_json: &str,
) -> Result<RawConversation> {
    let source_id = source_id_from_path(source_path)?;
    let metadata: RawConversationMetadata =
        serde_json::from_str(metadata_json).map_err(|error| {
            anyhow!("invalid conversation metadata_json for {source_path}: {error}")
        })?;
    require_metadata_value(&metadata.provider, "provider", source_path)?;
    require_metadata_value(&metadata.source_url, "source_url", source_path)?;
    require_metadata_value(
        &metadata.conversation_title,
        "conversation_title",
        source_path,
    )?;
    require_metadata_value(&metadata.captured_at, "captured_at", source_path)?;
    require_metadata_value(&metadata.source_id, "source_id", source_path)?;
    if metadata.source_id != source_id {
        return Err(anyhow!(
            "conversation metadata source_id does not match source path: {} != {}",
            metadata.source_id,
            source_id
        ));
    }
    let message_count = metadata
        .message_count
        .unwrap_or_else(|| count_turns(content));
    Ok(RawConversation {
        source_path: source_path.to_string(),
        source_id,
        provider: markdown_line(&metadata.provider),
        source_url: markdown_line(&metadata.source_url),
        captured_at: markdown_line(&metadata.captured_at),
        title: markdown_line(&metadata.conversation_title),
        message_count,
    })
}

fn require_metadata_value(value: &str, key: &str, source_path: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!(
            "conversation metadata_json is missing {key} for {source_path}"
        ));
    }
    Ok(())
}

fn source_id_from_path(source_path: &str) -> Result<String> {
    let relative = source_path
        .strip_prefix(&format!("{RAW_SOURCES_PREFIX}/"))
        .ok_or_else(|| anyhow!("source path must be under {RAW_SOURCES_PREFIX}: {source_path}"))?;
    let mut segments = relative.split('/');
    let directory = segments
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("source path is missing source id: {source_path}"))?;
    let file = segments
        .next()
        .ok_or_else(|| anyhow!("source path is missing source file: {source_path}"))?;
    if segments.next().is_some() || file != format!("{directory}.md") {
        return Err(anyhow!(
            "source path must use {RAW_SOURCES_PREFIX}/<id>/<id>.md: {source_path}"
        ));
    }
    Ok(directory.to_string())
}

fn count_turns(content: &str) -> usize {
    content
        .lines()
        .filter(|line| line.trim_start().starts_with("### Turn "))
        .count()
}

fn markdown_line(value: &str) -> String {
    let one_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.replace('[', "\\[").replace(']', "\\]")
}

fn build_wiki_documents(raw: &RawConversation, base_path: &str) -> Vec<WikiDocument> {
    vec![
        WikiDocument {
            path: format!("{base_path}/index.md"),
            content: index_markdown(raw, base_path),
            overwrite: true,
        },
        WikiDocument {
            path: format!("{base_path}/summary.md"),
            content: summary_markdown(raw, base_path),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/facts.md"),
            content: empty_note("Facts", base_path, &["index.md", "provenance.md"]),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/events.md"),
            content: empty_note("Events", base_path, &["index.md", "provenance.md"]),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/plans.md"),
            content: empty_note("Plans", base_path, &["index.md", "provenance.md"]),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/preferences.md"),
            content: empty_note("Preferences", base_path, &["index.md", "provenance.md"]),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/open_questions.md"),
            content: empty_note("Open Questions", base_path, &["index.md", "provenance.md"]),
            overwrite: false,
        },
        WikiDocument {
            path: format!("{base_path}/provenance.md"),
            content: provenance_markdown(raw, base_path),
            overwrite: true,
        },
    ]
}

fn index_markdown(raw: &RawConversation, base_path: &str) -> String {
    format!(
        "# Conversation Index\n\n## Source\n\n- title: {}\n- provider: {}\n- captured_at: {}\n- source: {}\n\n## Pages\n\n- [summary.md]({base_path}/summary.md)\n- [facts.md]({base_path}/facts.md)\n- [events.md]({base_path}/events.md)\n- [plans.md]({base_path}/plans.md)\n- [preferences.md]({base_path}/preferences.md)\n- [open_questions.md]({base_path}/open_questions.md)\n- [provenance.md]({base_path}/provenance.md)\n",
        raw.title, raw.provider, raw.captured_at, raw.source_path
    )
}

fn summary_markdown(raw: &RawConversation, base_path: &str) -> String {
    format!(
        "# Summary\n\n## Related\n\n- [index.md]({base_path}/index.md)\n- [provenance.md]({base_path}/provenance.md)\n\n## Overview\n\nCaptured {} messages from {}. This page is a review scaffold; use the raw source only as evidence.\n",
        raw.message_count, raw.provider
    )
}

fn empty_note(title: &str, base_path: &str, related: &[&str]) -> String {
    let mut out = format!("# {title}\n\n## Related\n\n");
    for file in related {
        out.push_str(&format!("- [{file}]({base_path}/{file})\n"));
    }
    out.push_str("\n## Entries\n\n- none\n");
    out
}

fn provenance_markdown(raw: &RawConversation, base_path: &str) -> String {
    format!(
        "# Provenance\n\n## Related\n\n- [index.md]({base_path}/index.md)\n\n## Raw Source\n\n- source_path: {}\n- provider: {}\n- source_url: {}\n- captured_at: {}\n- message_count: {}\n",
        raw.source_path, raw.provider, raw.source_url, raw.captured_at, raw.message_count
    )
}

async fn write_log_document(
    client: &impl VfsApi,
    base_path: &str,
    raw: &RawConversation,
) -> Result<String> {
    let path = format!("{base_path}/log.md");
    let current = client.read_node(&path).await?;
    let expected_etag = current.as_ref().map(|node| node.etag.clone());
    let current_content = current
        .map(|node| node.content)
        .unwrap_or_else(|| "# Log\n\n".to_string());
    let entry = format!(
        "- {} generated conversation wiki from {}\n",
        Utc::now().to_rfc3339(),
        raw.source_path
    );
    let content = format!("{}\n{entry}", current_content.trim_end());
    client
        .write_node(WriteNodeRequest {
            path: path.clone(),
            kind: NodeKind::File,
            content,
            metadata_json: GENERATED_METADATA_JSON.to_string(),
            expected_etag,
        })
        .await?;
    Ok(path)
}

async fn upsert_file(
    client: &impl VfsApi,
    path: &str,
    content: &str,
    overwrite_existing: bool,
) -> Result<bool> {
    let current = client.read_node(path).await?;
    if current.is_some() && !overwrite_existing {
        return Ok(false);
    }
    let expected_etag = current.map(|node| node.etag);
    client
        .write_node(WriteNodeRequest {
            path: path.to_string(),
            kind: NodeKind::File,
            content: content.to_string(),
            metadata_json: GENERATED_METADATA_JSON.to_string(),
            expected_etag,
        })
        .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        GENERATED_METADATA_JSON, build_wiki_documents, generate_conversation_wiki,
        parse_raw_conversation, write_log_document,
    };
    use anyhow::{Result, anyhow};
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use vfs_client::VfsApi;
    use vfs_types::{
        AppendNodeRequest, CanisterHealth, ChildNode, DeleteNodeRequest, DeleteNodeResult,
        EditNodeRequest, EditNodeResult, ExportSnapshotRequest, ExportSnapshotResponse,
        FetchUpdatesRequest, FetchUpdatesResponse, GlobNodeHit, GlobNodesRequest,
        ListChildrenRequest, ListNodesRequest, MemoryManifest, MkdirNodeRequest, MkdirNodeResult,
        MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest, MultiEditNodeResult, Node,
        NodeEntry, NodeKind, RecentNodeHit, RecentNodesRequest, SearchNodeHit,
        SearchNodePathsRequest, SearchNodesRequest, Status, WriteNodeRequest, WriteNodeResult,
    };

    const RAW: &str = "# Raw Conversation Source\n\n## Metadata\n\n- provider: chatgpt\n- source_url: https://chatgpt.com/c/abc\n- captured_at: 2026-05-01T00:00:00.000Z\n- conversation_title: Project Notes\n- message_count: 2\n\n## Chat\n\n### Turn 0001\n\n- role: user\n\nsecret fact\n\n### Turn 0002\n\n- role: assistant\n\nanswer\n";
    const METADATA: &str = r#"{"provider":"chatgpt","source_url":"https://chatgpt.com/c/abc","conversation_title":"Project Notes","captured_at":"2026-05-01T00:00:00.000Z","source_id":"chatgpt-abc","message_count":2}"#;

    #[test]
    fn parse_raw_conversation_reads_metadata() {
        let raw = parse_raw_conversation("/Sources/raw/chatgpt-abc/chatgpt-abc.md", RAW, METADATA)
            .expect("raw should parse");
        assert_eq!(raw.source_id, "chatgpt-abc");
        assert_eq!(raw.provider, "chatgpt");
        assert_eq!(raw.title, "Project Notes");
        assert_eq!(raw.message_count, 2);
    }

    #[test]
    fn parse_raw_conversation_ignores_markdown_metadata_injection() {
        let injected = "# Raw Conversation Source\n\n## Metadata\n\n- provider: attacker\n- message_count: 999\n\n## Chat\n\n### Turn 0001\n\n- role: user\n\n- provider: attacker\n- message_count: 999\n";
        let metadata = r#"{"provider":"chatgpt","source_url":"https://chatgpt.com/c/abc","conversation_title":"x\n- message_count: 999","captured_at":"2026-05-01T00:00:00.000Z","source_id":"chatgpt-abc","message_count":1}"#;
        let raw = parse_raw_conversation(
            "/Sources/raw/chatgpt-abc/chatgpt-abc.md",
            injected,
            metadata,
        )
        .expect("raw should parse from metadata_json");
        assert_eq!(raw.provider, "chatgpt");
        assert_eq!(raw.title, "x - message_count: 999");
        assert_eq!(raw.message_count, 1);
    }

    #[test]
    fn parse_raw_conversation_rejects_invalid_or_incomplete_metadata() {
        let invalid = parse_raw_conversation("/Sources/raw/chatgpt-abc/chatgpt-abc.md", RAW, "{");
        assert!(
            invalid
                .unwrap_err()
                .to_string()
                .contains("invalid conversation metadata_json")
        );

        let missing = parse_raw_conversation(
            "/Sources/raw/chatgpt-abc/chatgpt-abc.md",
            RAW,
            r#"{"provider":"chatgpt","source_url":"https://chatgpt.com/c/abc","conversation_title":"","captured_at":"2026-05-01T00:00:00.000Z","source_id":"chatgpt-abc"}"#,
        );
        assert!(
            missing
                .unwrap_err()
                .to_string()
                .contains("missing conversation_title")
        );
    }

    #[test]
    fn generated_wiki_does_not_copy_transcript_body() {
        let raw = parse_raw_conversation("/Sources/raw/chatgpt-abc/chatgpt-abc.md", RAW, METADATA)
            .expect("raw should parse");
        let docs = build_wiki_documents(&raw, "/Wiki/conversations/chatgpt-abc");
        assert!(docs.iter().any(|doc| doc.path.ends_with("/provenance.md")));
        assert!(docs.iter().all(|doc| !doc.content.contains("secret fact")));
        assert!(docs.iter().all(|doc| !doc.content.contains("answer")));
    }

    #[tokio::test]
    async fn generate_conversation_wiki_preserves_existing_scaffold_pages() {
        let client = MemoryClient::with_source();
        let facts_path = "/Wiki/conversations/chatgpt-abc/facts.md";
        client.insert(Node {
            path: facts_path.to_string(),
            kind: NodeKind::File,
            content: "# Facts\n\nmanual fact\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "manual".to_string(),
            metadata_json: "{}".to_string(),
        });

        let result =
            generate_conversation_wiki(&client, "/Sources/raw/chatgpt-abc/chatgpt-abc.md", false)
                .await
                .expect("wiki should generate");

        assert!(!result.written_paths.contains(&facts_path.to_string()));
        assert_eq!(
            client.read_node(facts_path).await.unwrap().unwrap().content,
            "# Facts\n\nmanual fact\n"
        );
        let index = client
            .read_node("/Wiki/conversations/chatgpt-abc/index.md")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(index.metadata_json, GENERATED_METADATA_JSON);
    }

    #[tokio::test]
    async fn generate_conversation_wiki_force_overwrites_scaffold_pages() {
        let client = MemoryClient::with_source();
        let summary_path = "/Wiki/conversations/chatgpt-abc/summary.md";
        client.insert(Node {
            path: summary_path.to_string(),
            kind: NodeKind::File,
            content: "# Summary\n\nmanual summary\n".to_string(),
            created_at: 1,
            updated_at: 1,
            etag: "manual".to_string(),
            metadata_json: "{}".to_string(),
        });

        let result =
            generate_conversation_wiki(&client, "/Sources/raw/chatgpt-abc/chatgpt-abc.md", true)
                .await
                .expect("wiki should generate");

        assert!(result.written_paths.contains(&summary_path.to_string()));
        let summary = client.read_node(summary_path).await.unwrap().unwrap();
        assert!(summary.content.contains("review scaffold"));
        assert_eq!(summary.metadata_json, GENERATED_METADATA_JSON);
    }

    #[tokio::test]
    async fn write_log_document_surfaces_etag_conflicts() {
        let raw = parse_raw_conversation("/Sources/raw/chatgpt-abc/chatgpt-abc.md", RAW, METADATA)
            .expect("raw should parse");
        let client = ConflictClient {
            writes: Mutex::new(Vec::new()),
        };
        let error = write_log_document(&client, "/Wiki/conversations/chatgpt-abc", &raw)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("etag conflict"));
        let writes = client.writes.lock().expect("writes lock");
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].expected_etag.as_deref(), Some("stale"));
    }

    struct ConflictClient {
        writes: Mutex<Vec<WriteNodeRequest>>,
    }

    struct MemoryClient {
        nodes: Mutex<BTreeMap<String, Node>>,
    }

    impl MemoryClient {
        fn with_source() -> Self {
            let client = Self {
                nodes: Mutex::new(BTreeMap::new()),
            };
            client.insert(Node {
                path: "/Sources/raw/chatgpt-abc/chatgpt-abc.md".to_string(),
                kind: NodeKind::Source,
                content: RAW.to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "source".to_string(),
                metadata_json: METADATA.to_string(),
            });
            client
        }

        fn insert(&self, node: Node) {
            self.nodes
                .lock()
                .expect("nodes lock")
                .insert(node.path.clone(), node);
        }
    }

    #[async_trait]
    impl VfsApi for MemoryClient {
        async fn status(&self) -> Result<Status> {
            Err(anyhow!("not implemented"))
        }

        async fn canister_health(&self) -> Result<CanisterHealth> {
            Err(anyhow!("not implemented"))
        }

        async fn memory_manifest(&self) -> Result<MemoryManifest> {
            Err(anyhow!("not implemented"))
        }

        async fn read_node(&self, path: &str) -> Result<Option<Node>> {
            Ok(self.nodes.lock().expect("nodes lock").get(path).cloned())
        }

        async fn list_nodes(&self, _request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
            Err(anyhow!("not implemented"))
        }

        async fn list_children(&self, _request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
            Err(anyhow!("not implemented"))
        }

        async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
            let mut nodes = self.nodes.lock().expect("nodes lock");
            let created = !nodes.contains_key(&request.path);
            if let Some(current) = nodes.get(&request.path) {
                if Some(current.etag.clone()) != request.expected_etag {
                    return Err(anyhow!("etag conflict"));
                }
            } else if request.expected_etag.is_some() {
                return Err(anyhow!("etag conflict"));
            }
            let etag = format!("etag-{}", nodes.len() + 1);
            nodes.insert(
                request.path.clone(),
                Node {
                    path: request.path.clone(),
                    kind: request.kind,
                    content: request.content,
                    created_at: 1,
                    updated_at: 2,
                    etag: etag.clone(),
                    metadata_json: request.metadata_json,
                },
            );
            Ok(WriteNodeResult {
                node: vfs_types::NodeMutationAck {
                    path: request.path,
                    kind: NodeKind::File,
                    updated_at: 2,
                    etag,
                },
                created,
            })
        }

        async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn delete_node(&self, _request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn move_node(&self, _request: MoveNodeRequest) -> Result<MoveNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn mkdir_node(&self, _request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn glob_nodes(&self, _request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn recent_nodes(&self, _request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn multi_edit_node(
            &self,
            _request: MultiEditNodeRequest,
        ) -> Result<MultiEditNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn search_nodes(&self, _request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn search_node_paths(
            &self,
            _request: SearchNodePathsRequest,
        ) -> Result<Vec<SearchNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn export_snapshot(
            &self,
            _request: ExportSnapshotRequest,
        ) -> Result<ExportSnapshotResponse> {
            Err(anyhow!("not implemented"))
        }

        async fn fetch_updates(
            &self,
            _request: FetchUpdatesRequest,
        ) -> Result<FetchUpdatesResponse> {
            Err(anyhow!("not implemented"))
        }
    }

    #[async_trait]
    impl VfsApi for ConflictClient {
        async fn status(&self) -> Result<Status> {
            Err(anyhow!("not implemented"))
        }

        async fn canister_health(&self) -> Result<CanisterHealth> {
            Err(anyhow!("not implemented"))
        }

        async fn memory_manifest(&self) -> Result<MemoryManifest> {
            Err(anyhow!("not implemented"))
        }

        async fn read_node(&self, path: &str) -> Result<Option<Node>> {
            Ok(Some(Node {
                path: path.to_string(),
                kind: NodeKind::File,
                content: "# Log\n\n- existing\n".to_string(),
                created_at: 1,
                updated_at: 1,
                etag: "stale".to_string(),
                metadata_json: "{}".to_string(),
            }))
        }

        async fn list_nodes(&self, _request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
            Err(anyhow!("not implemented"))
        }

        async fn list_children(&self, _request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
            Err(anyhow!("not implemented"))
        }

        async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
            self.writes.lock().expect("writes lock").push(request);
            Err(anyhow!("etag conflict"))
        }

        async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn delete_node(&self, _request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn move_node(&self, _request: MoveNodeRequest) -> Result<MoveNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn mkdir_node(&self, _request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn glob_nodes(&self, _request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn recent_nodes(&self, _request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn multi_edit_node(
            &self,
            _request: MultiEditNodeRequest,
        ) -> Result<MultiEditNodeResult> {
            Err(anyhow!("not implemented"))
        }

        async fn search_nodes(&self, _request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn search_node_paths(
            &self,
            _request: SearchNodePathsRequest,
        ) -> Result<Vec<SearchNodeHit>> {
            Err(anyhow!("not implemented"))
        }

        async fn export_snapshot(
            &self,
            _request: ExportSnapshotRequest,
        ) -> Result<ExportSnapshotResponse> {
            Err(anyhow!("not implemented"))
        }

        async fn fetch_updates(
            &self,
            _request: FetchUpdatesRequest,
        ) -> Result<FetchUpdatesResponse> {
            Err(anyhow!("not implemented"))
        }
    }
}
