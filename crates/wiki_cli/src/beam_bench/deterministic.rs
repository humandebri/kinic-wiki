// Where: crates/wiki_cli/src/beam_bench/deterministic.rs
// What: Deterministic retrieval and short-answer evaluation for BEAM RAG benchmarks.
// Why: Benchmark results should reflect llm-wiki retrieval quality, not free-form model reasoning variance.
use crate::client::WikiApi;
use anyhow::Result;
use std::time::Instant;
use wiki_types::SearchNodesRequest;

use super::dataset::BeamQuestion;
use super::import::{ImportedConversation, ImportedNote};
use super::model::ToolCallRecord;
use super::report::{FailureReason, QuestionResult};

pub async fn run_question(
    client: &impl WikiApi,
    conversation_id: &str,
    imported: &ImportedConversation,
    question: BeamQuestion,
    top_k: u32,
    include_in_primary_metrics: bool,
    answer_enabled: bool,
) -> Result<QuestionResult> {
    let started_at = Instant::now();
    let effective_gold_paths = resolve_gold_paths(imported, &question);
    let effective_gold_spans =
        resolve_gold_spans(imported, &question, effective_gold_paths.as_slice());
    let mut tool_calls = Vec::new();
    let search_request = SearchNodesRequest {
        query_text: question.prompt.clone(),
        prefix: Some(imported.base_path.clone()),
        top_k,
    };
    let search_hits = client.search_nodes(search_request.clone()).await?;
    tool_calls.push(ToolCallRecord {
        name: "search_nodes".to_string(),
        arguments: serde_json::to_string(&search_request).unwrap_or_else(|_| "{}".to_string()),
        is_error: false,
    });
    let retrieved_paths = search_hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<Vec<_>>();
    let mut docs_read_count = 0;
    let mut matched_gold_path = None;
    let mut matched_gold_span = None;
    let mut predicted_answer = None;

    for path in &retrieved_paths {
        let Some(gold_note) = imported.notes.iter().find(|note| &note.path == path) else {
            continue;
        };
        let node = client.read_node(path).await?;
        docs_read_count += 1;
        tool_calls.push(ToolCallRecord {
            name: "read_node".to_string(),
            arguments: path.clone(),
            is_error: node.is_none(),
        });
        let content = node
            .as_ref()
            .map(|value| value.content.as_str())
            .unwrap_or(gold_note.content.as_str());
        if effective_gold_paths.iter().any(|gold| gold == path) {
            matched_gold_path = Some(path.clone());
            if let Some(span) = effective_gold_spans
                .iter()
                .find(|span| content.contains(span.as_str()))
            {
                matched_gold_span = Some(span.clone());
                if answer_enabled {
                    predicted_answer = Some(span.clone());
                }
                break;
            }
        }
    }

    let retrieval_hit = matched_gold_path.is_some();
    let answer_exact_match = question
        .reference_answer
        .as_deref()
        .zip(predicted_answer.as_deref())
        .map(|(expected, actual)| expected.trim() == actual.trim())
        .unwrap_or(false);
    let answer_normalized_match = question
        .reference_answer
        .as_deref()
        .zip(predicted_answer.as_deref())
        .map(|(expected, actual)| normalize_text(expected) == normalize_text(actual))
        .unwrap_or(false);
    let failure_reason = determine_failure_reason(
        retrieval_hit,
        matched_gold_span.is_some(),
        answer_enabled,
        predicted_answer.is_some(),
        answer_normalized_match,
    );
    Ok(QuestionResult {
        conversation_id: conversation_id.to_string(),
        question_id: question.question_id,
        question_type: question.question_type,
        question_class: question.question_class,
        prompt: question.prompt,
        reference_answer: question.reference_answer,
        predicted_answer,
        gold_paths: effective_gold_paths,
        gold_spans: effective_gold_spans,
        retrieved_paths,
        matched_gold_path: matched_gold_path.clone(),
        matched_gold_span: matched_gold_span.clone(),
        source_note_type: matched_gold_path
            .as_deref()
            .map(|path| note_type_for_path(path, &imported.notes)),
        included_in_primary_metrics: include_in_primary_metrics,
        retrieval_evaluable: true,
        retrieval_hit,
        answer_exact_match,
        answer_normalized_match,
        tool_call_count: tool_calls.len(),
        tool_error_count: tool_calls.iter().filter(|call| call.is_error).count(),
        docs_read_count,
        input_tokens: Some(0),
        output_tokens: Some(0),
        total_tokens: Some(0),
        latency_ms: started_at.elapsed().as_millis(),
        failure_reason,
        tool_calls,
        raw_events: Vec::new(),
    })
}

fn resolve_gold_paths(imported: &ImportedConversation, question: &BeamQuestion) -> Vec<String> {
    if !question.gold_paths.is_empty() {
        return question
            .gold_paths
            .iter()
            .map(|path| {
                if path.starts_with('/') {
                    path.clone()
                } else {
                    format!("{}/{}", imported.base_path, path.trim_start_matches('/'))
                }
            })
            .collect();
    }
    if let Some(reference_answer) = question.reference_answer.as_deref() {
        return imported
            .notes
            .iter()
            .filter(|note| note.content.contains(reference_answer))
            .map(|note| note.path.clone())
            .collect();
    }
    Vec::new()
}

fn resolve_gold_spans(
    imported: &ImportedConversation,
    question: &BeamQuestion,
    gold_paths: &[String],
) -> Vec<String> {
    if !question.gold_spans.is_empty() {
        return question.gold_spans.clone();
    }
    let Some(reference_answer) = question.reference_answer.as_deref() else {
        return Vec::new();
    };
    gold_paths
        .iter()
        .filter_map(|path| imported.notes.iter().find(|note| &note.path == path))
        .filter(|note| note.content.contains(reference_answer))
        .map(|_| reference_answer.to_string())
        .collect()
}

fn note_type_for_path(path: &str, notes: &[ImportedNote]) -> String {
    notes
        .iter()
        .find(|note| note.path == path)
        .map(|note| note.note_type.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn determine_failure_reason(
    retrieval_hit: bool,
    span_hit: bool,
    answer_enabled: bool,
    predicted_answer: bool,
    answer_match: bool,
) -> Option<FailureReason> {
    if !retrieval_hit {
        return Some(FailureReason::MissedGoldPath);
    }
    if answer_enabled && !span_hit {
        return Some(FailureReason::ReadWithoutSpan);
    }
    if answer_enabled && predicted_answer && !answer_match {
        return Some(FailureReason::WrongShortAnswer);
    }
    None
}

fn normalize_text(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_space = false;
    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            last_was_space = false;
            continue;
        }
        if !last_was_space {
            normalized.push(' ');
            last_was_space = true;
        }
    }
    normalized.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::run_question;
    use crate::beam_bench::dataset::{BeamQuestion, BeamQuestionClass};
    use crate::beam_bench::import::{ImportedConversation, ImportedNote};
    use crate::client::WikiApi;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use wiki_types::{
        AppendNodeRequest, DeleteNodeRequest, DeleteNodeResult, EditNodeRequest, EditNodeResult,
        ExportSnapshotRequest, ExportSnapshotResponse, FetchUpdatesRequest, FetchUpdatesResponse,
        GlobNodeHit, GlobNodesRequest, ListNodesRequest, MkdirNodeRequest, MkdirNodeResult,
        MoveNodeRequest, MoveNodeResult, MultiEditNodeRequest, MultiEditNodeResult, Node,
        NodeEntry, RecentNodeHit, RecentNodesRequest, SearchNodeHit, SearchNodePathsRequest,
        SearchNodesRequest, Status, WriteNodeRequest, WriteNodeResult,
    };

    struct MockClient {
        nodes: BTreeMap<String, Node>,
        search_hits: Vec<SearchNodeHit>,
    }

    #[async_trait]
    impl WikiApi for MockClient {
        async fn status(&self) -> Result<Status> {
            unreachable!()
        }
        async fn read_node(&self, path: &str) -> Result<Option<Node>> {
            Ok(self.nodes.get(path).cloned())
        }
        async fn list_nodes(&self, _request: ListNodesRequest) -> Result<Vec<NodeEntry>> {
            unreachable!()
        }
        async fn write_node(&self, _request: WriteNodeRequest) -> Result<WriteNodeResult> {
            unreachable!()
        }
        async fn append_node(&self, _request: AppendNodeRequest) -> Result<WriteNodeResult> {
            unreachable!()
        }
        async fn edit_node(&self, _request: EditNodeRequest) -> Result<EditNodeResult> {
            unreachable!()
        }
        async fn delete_node(&self, _request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
            unreachable!()
        }
        async fn move_node(&self, _request: MoveNodeRequest) -> Result<MoveNodeResult> {
            unreachable!()
        }
        async fn mkdir_node(&self, _request: MkdirNodeRequest) -> Result<MkdirNodeResult> {
            unreachable!()
        }
        async fn glob_nodes(&self, _request: GlobNodesRequest) -> Result<Vec<GlobNodeHit>> {
            unreachable!()
        }
        async fn recent_nodes(&self, _request: RecentNodesRequest) -> Result<Vec<RecentNodeHit>> {
            unreachable!()
        }
        async fn multi_edit_node(
            &self,
            _request: MultiEditNodeRequest,
        ) -> Result<MultiEditNodeResult> {
            unreachable!()
        }
        async fn search_nodes(&self, _request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
            Ok(self.search_hits.clone())
        }
        async fn search_node_paths(
            &self,
            _request: SearchNodePathsRequest,
        ) -> Result<Vec<SearchNodeHit>> {
            unreachable!()
        }
        async fn export_snapshot(
            &self,
            _request: ExportSnapshotRequest,
        ) -> Result<ExportSnapshotResponse> {
            unreachable!()
        }
        async fn fetch_updates(
            &self,
            _request: FetchUpdatesRequest,
        ) -> Result<FetchUpdatesResponse> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn run_question_separates_retrieval_and_answering() {
        let path = "/Wiki/beam/run/conv/messages/0002-assistant.md".to_string();
        let imported = ImportedConversation {
            conversation_id: "conv".to_string(),
            base_path: "/Wiki/beam/run/conv".to_string(),
            note_paths: vec![path.clone()],
            notes: vec![ImportedNote {
                path: path.clone(),
                content: "# Message 0002\n\nMeeting is on March 15, 2024.".to_string(),
                note_type: "messages".to_string(),
            }],
        };
        let client = MockClient {
            nodes: BTreeMap::from([(
                path.clone(),
                Node {
                    path: path.clone(),
                    kind: wiki_types::NodeKind::File,
                    content: "# Message 0002\n\nMeeting is on March 15, 2024.".to_string(),
                    created_at: 0,
                    updated_at: 0,
                    etag: "etag".to_string(),
                    metadata_json: "{}".to_string(),
                },
            )]),
            search_hits: vec![SearchNodeHit {
                path: path.clone(),
                kind: wiki_types::NodeKind::File,
                snippet: Some("March 15, 2024".to_string()),
                score: 1.0,
                match_reasons: vec!["content".to_string()],
            }],
        };
        let result = run_question(
            &client,
            "conv",
            &imported,
            BeamQuestion {
                question_id: "factoid-000".to_string(),
                question_type: "factoid".to_string(),
                question_class: BeamQuestionClass::Factoid,
                prompt: "When is the meeting?".to_string(),
                reference_answer: Some("March 15, 2024".to_string()),
                gold_paths: vec!["messages/0002-assistant.md".to_string()],
                gold_spans: vec!["March 15, 2024".to_string()],
                raw: serde_json::json!({}),
            },
            3,
            true,
            true,
        )
        .await
        .expect("question should run");
        assert!(result.retrieval_hit);
        assert!(result.retrieval_evaluable);
        assert!(result.answer_normalized_match);
        assert_eq!(result.source_note_type.as_deref(), Some("messages"));
    }
}
