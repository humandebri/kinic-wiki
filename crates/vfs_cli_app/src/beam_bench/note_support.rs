// Where: crates/vfs_cli_app/src/beam_bench/note_support.rs
// What: Shared note rendering helpers and chat flattening for BEAM note generation.
// Why: Rendering concerns should stay separate from canonical `facts.md` classification policy.
use super::dataset::BeamConversation;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct ChatTurn {
    role: Option<String>,
    pub content: String,
    pub chat_id: Option<String>,
    pub index: Option<String>,
    pub question_type: Option<String>,
    pub time_anchor: Option<String>,
}

impl ChatTurn {
    pub fn label(&self) -> String {
        self.role
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("message")
            .to_string()
    }
}

pub fn flatten_chat(value: &Value) -> Vec<ChatTurn> {
    let mut turns = Vec::new();
    collect_chat_messages(value, &mut turns);
    turns
}

pub fn extract_identifier_lines(conversation: &BeamConversation) -> Vec<String> {
    let mut lines = vec![format!("conversation_id: {}", conversation.conversation_id)];
    push_named_scalar(
        &conversation.conversation_seed,
        "title",
        "title",
        &mut lines,
    );
    push_named_scalar(
        &conversation.conversation_seed,
        "category",
        "category",
        &mut lines,
    );
    dedupe_lines(lines)
}

pub fn append_related_section(out: &mut String, base_path: &str, note_names: &[&str]) {
    out.push_str("## Related\n\n");
    for note_name in note_names {
        out.push_str(&format!("- [{note_name}]({base_path}/{note_name})\n"));
    }
    out.push('\n');
}

pub fn append_json_section(out: &mut String, title: &str, value: &Value) {
    out.push_str(&format!("## {title}\n\n"));
    out.push_str(&fenced_json(value));
    out.push_str("\n\n");
}

pub fn append_text_section(out: &mut String, title: &str, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    out.push_str(&format!("## {title}\n\n{trimmed}\n\n"));
}

fn collect_chat_messages(value: &Value, turns: &mut Vec<ChatTurn>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_chat_messages(item, turns);
            }
        }
        Value::Object(object) => {
            if let Some(content) = object.get("content").and_then(Value::as_str) {
                turns.push(ChatTurn {
                    role: object
                        .get("role")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    content: content.to_string(),
                    chat_id: object.get("id").map(scalar_value),
                    index: object.get("index").map(scalar_value),
                    question_type: object.get("question_type").map(scalar_value),
                    time_anchor: object.get("time_anchor").map(scalar_value),
                });
                return;
            }
            if let Some(nested) = object.get("messages") {
                collect_chat_messages(nested, turns);
            }
        }
        Value::String(text) => turns.push(ChatTurn {
            role: None,
            content: text.clone(),
            chat_id: None,
            index: None,
            question_type: None,
            time_anchor: None,
        }),
        _ => {}
    }
}

fn push_named_scalar(value: &Value, key: &str, label: &str, lines: &mut Vec<String>) {
    if let Some(text) = value.get(key).and_then(Value::as_str) {
        push_fact_line(label, None, text, lines);
    }
}

fn push_fact_line(label: &str, prefix: Option<String>, value: &str, lines: &mut Vec<String>) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    match prefix {
        Some(prefix) => lines.push(format!("{label}.{prefix}: {trimmed}")),
        None => lines.push(format!("{label}: {trimmed}")),
    }
}

fn dedupe_lines(lines: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in lines {
        if seen.insert(line.clone()) {
            out.push(line);
        }
    }
    out
}

fn fenced_json(value: &Value) -> String {
    format!(
        "```json\n{}\n```",
        serde_json::to_string_pretty(value).expect("JSON value should serialize")
    )
}

fn scalar_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}
