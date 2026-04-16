// Where: crates/wiki_cli/src/beam_bench/import.rs
// What: Deterministic BEAM import that splits conversations into retrieval-friendly notes.
// Why: RAG evaluation needs stable note boundaries so gold evidence can point at a small, explicit set of files.
use crate::client::WikiApi;
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use wiki_types::{NodeKind, WriteNodeRequest};

use super::dataset::BeamConversation;

#[derive(Debug, Clone, Serialize)]
pub struct ImportedConversation {
    pub conversation_id: String,
    pub base_path: String,
    pub note_paths: Vec<String>,
    pub notes: Vec<ImportedNote>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportedNote {
    pub path: String,
    pub content: String,
    pub note_type: String,
}

pub async fn import_conversation(
    client: &impl WikiApi,
    namespace: &str,
    conversation: &BeamConversation,
) -> Result<ImportedConversation> {
    let base_path = format!(
        "/Wiki/beam/{}/{}",
        sanitize_segment(namespace),
        sanitize_segment(&conversation.conversation_id)
    );
    let documents = build_documents(conversation, &base_path);
    let mut note_paths = Vec::with_capacity(documents.len());
    let mut notes = Vec::with_capacity(documents.len());
    for (path, content) in documents {
        client
            .write_node(WriteNodeRequest {
                path: path.clone(),
                kind: NodeKind::File,
                content: content.clone(),
                metadata_json: "{}".to_string(),
                expected_etag: None,
            })
            .await?;
        let note_type = note_type_for_path(&path, &base_path);
        note_paths.push(path);
        notes.push(ImportedNote {
            path: note_paths.last().cloned().expect("path should exist"),
            content,
            note_type,
        });
    }
    Ok(ImportedConversation {
        conversation_id: conversation.conversation_id.clone(),
        base_path,
        note_paths,
        notes,
    })
}

pub fn build_documents(conversation: &BeamConversation, base_path: &str) -> Vec<(String, String)> {
    let mut documents = vec![
        (
            format!("{base_path}/profile.md"),
            json_markdown("Profile", &conversation.user_profile),
        ),
        (
            format!("{base_path}/plan.md"),
            text_markdown("Plan", &conversation.conversation_plan),
        ),
        (
            format!("{base_path}/user_questions.md"),
            json_markdown("User Questions", &conversation.user_questions),
        ),
    ];
    if !conversation.narratives.trim().is_empty() {
        documents.push((
            format!("{base_path}/narratives.md"),
            text_markdown("Narratives", &conversation.narratives),
        ));
    }
    for (index, message) in flatten_chat(&conversation.chat).into_iter().enumerate() {
        let path = format!(
            "{base_path}/messages/{:04}-{}.md",
            index + 1,
            sanitize_segment(&message.label())
        );
        documents.push((path, message_markdown(index + 1, &message)));
    }
    documents
}

fn message_markdown(index: usize, message: &ChatMessage) -> String {
    format!(
        "# Message {index:04}\n\n- role: {}\n\n{}",
        message.label(),
        message.content.trim()
    )
}

fn json_markdown(title: &str, value: &Value) -> String {
    format!("# {title}\n\n{}", fenced_json(value))
}

fn text_markdown(title: &str, value: &str) -> String {
    format!("# {title}\n\n{}", value.trim())
}

fn fenced_json(value: &Value) -> String {
    format!(
        "```json\n{}\n```",
        serde_json::to_string_pretty(value).expect("JSON value should serialize")
    )
}

fn note_type_for_path(path: &str, base_path: &str) -> String {
    let relative = path
        .strip_prefix(base_path)
        .unwrap_or(path)
        .trim_start_matches('/');
    relative
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("root")
        .to_string()
}

fn sanitize_segment(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    trimmed
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: Option<String>,
    content: String,
}

impl ChatMessage {
    fn label(&self) -> String {
        self.role
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("message")
            .to_string()
    }
}

fn flatten_chat(value: &Value) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    collect_chat_messages(value, &mut messages);
    messages
}

fn collect_chat_messages(value: &Value, messages: &mut Vec<ChatMessage>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_chat_messages(item, messages);
            }
        }
        Value::Object(object) => {
            if let Some(content) = object.get("content").and_then(Value::as_str) {
                messages.push(ChatMessage {
                    role: object
                        .get("role")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    content: content.to_string(),
                });
                return;
            }
            if let Some(nested) = object.get("messages") {
                collect_chat_messages(nested, messages);
            }
        }
        Value::String(text) => {
            messages.push(ChatMessage {
                role: None,
                content: text.clone(),
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{build_documents, flatten_chat};
    use crate::beam_bench::dataset::BeamConversation;
    use serde_json::json;

    fn sample_conversation() -> BeamConversation {
        BeamConversation {
            conversation_id: "conv-1".to_string(),
            conversation_seed: json!({"category":"General"}),
            narratives: "narrative".to_string(),
            user_profile: json!({"user_info":"profile"}),
            conversation_plan: "plan".to_string(),
            user_questions: json!([{"messages":["question"]}]),
            chat: json!([[{"role":"user","content":"hello"},{"role":"assistant","content":"hi"}]]),
            probing_questions:
                "{\"factoid\":[{\"question\":\"What did the assistant say?\",\"answer\":\"hi\"}]}"
                    .to_string(),
        }
    }

    #[test]
    fn flatten_chat_preserves_message_order() {
        let messages = flatten_chat(&sample_conversation().chat);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].content, "hi");
    }

    #[test]
    fn build_documents_emits_deterministic_note_set() {
        let documents = build_documents(&sample_conversation(), "/Wiki/beam/run/conv-1");
        let paths = documents
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                "/Wiki/beam/run/conv-1/profile.md".to_string(),
                "/Wiki/beam/run/conv-1/plan.md".to_string(),
                "/Wiki/beam/run/conv-1/user_questions.md".to_string(),
                "/Wiki/beam/run/conv-1/narratives.md".to_string(),
                "/Wiki/beam/run/conv-1/messages/0001-user.md".to_string(),
                "/Wiki/beam/run/conv-1/messages/0002-assistant.md".to_string()
            ]
        );
        assert!(documents[4].1.contains("# Message 0001"));
        assert!(documents[5].1.contains("- role: assistant"));
    }
}
