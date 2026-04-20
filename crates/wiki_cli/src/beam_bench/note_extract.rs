// Where: crates/wiki_cli/src/beam_bench/note_extract.rs
// What: Role-specific extraction for BEAM structured notes beyond facts/events/plan/profile.
// Why: Full BEAM question coverage needs dedicated preference, instruction, update, and summary notes.
use super::dataset::BeamConversation;
use super::note_support::{ChatTurn, extract_identifier_lines, flatten_chat};
use std::collections::BTreeMap;

pub fn extract_preference_lines(conversation: &BeamConversation) -> Vec<String> {
    let mut lines = Vec::new();
    append_profile_preferences(&conversation.user_profile.to_string(), &mut lines);
    for turn in flatten_chat(&conversation.chat)
        .into_iter()
        .filter(|turn| turn.label() == "user")
    {
        let text = turn.content.trim();
        let lowered = text.to_ascii_lowercase();
        if lowered.contains("prefer ")
            || lowered.contains("i like ")
            || lowered.contains("i dislike ")
            || lowered.contains("favorite ")
            || lowered.contains("lightweight")
            || lowered.contains("easy to maintain")
        {
            lines.push(text.to_string());
        }
    }
    dedupe(lines)
}

pub fn extract_instruction_lines(conversation: &BeamConversation) -> Vec<String> {
    let mut lines = Vec::new();
    let plan = conversation.conversation_plan.trim();
    if !plan.is_empty() {
        lines.push(format!("plan directive: {plan}"));
    }
    for turn in flatten_chat(&conversation.chat)
        .into_iter()
        .filter(|turn| turn.label() == "user")
    {
        let text = turn.content.trim();
        let lowered = text.to_ascii_lowercase();
        if lowered.starts_with("please ")
            || lowered.contains("always ")
            || lowered.contains("never ")
            || lowered.contains("do not ")
            || lowered.contains("don't ")
            || lowered.contains("remember ")
            || lowered.contains("when i ask")
        {
            lines.push(text.to_string());
        }
    }
    dedupe(lines)
}

pub fn extract_update_lines(conversation: &BeamConversation) -> Vec<String> {
    let mut history = BTreeMap::<String, Vec<String>>::new();
    for turn in flatten_chat(&conversation.chat)
        .into_iter()
        .filter(|turn| turn.label() == "user")
    {
        if let Some((subject, value)) = extract_subject_value(&turn.content) {
            history.entry(subject).or_default().push(value);
        }
    }
    let mut lines = Vec::new();
    for (subject, values) in history {
        if values.is_empty() {
            continue;
        }
        if values.len() == 1 {
            lines.push(format!("{subject} latest: {}", values[0]));
            continue;
        }
        let latest = values.last().cloned().unwrap_or_default();
        let previous = values[..values.len() - 1]
            .iter()
            .rev()
            .find(|value| **value != latest)
            .cloned()
            .unwrap_or_else(|| values[0].clone());
        if previous == latest {
            lines.push(format!("{subject} latest: {latest}"));
            continue;
        }
        lines.push(format!("{subject} previous: {previous}"));
        lines.push(format!("{subject} latest: {latest}"));
        lines.push(format!("{subject} update: {previous} -> {latest}"));
    }
    dedupe(lines)
}

pub fn extract_summary_lines(conversation: &BeamConversation) -> Vec<String> {
    let turns = flatten_chat(&conversation.chat);
    let mut lines = Vec::new();
    let identifiers = extract_identifier_lines(conversation);
    if let Some(first) = identifiers.first() {
        lines.push(first.clone());
    }
    if !conversation.narratives.trim().is_empty() {
        lines.push(format!("narrative summary: {}", conversation.narratives.trim()));
    }
    if !conversation.conversation_plan.trim().is_empty() {
        lines.push(format!("plan summary: {}", conversation.conversation_plan.trim()));
    }
    if !turns.is_empty() {
        lines.push(format!("turn count: {}", turns.len()));
    }
    if let Some(first_user) = turns.iter().find(|turn| turn.label() == "user") {
        lines.push(format!("initial user focus: {}", squeeze(&first_user.content)));
    }
    if let Some(last_user) = turns.iter().rev().find(|turn| turn.label() == "user") {
        lines.push(format!("latest user focus: {}", squeeze(&last_user.content)));
    }
    dedupe(lines)
}

pub fn render_turn_reference(turn: &ChatTurn, ordinal: usize) -> String {
    let mut parts = vec![format!("Turn {ordinal:04}")];
    if let Some(chat_id) = &turn.chat_id {
        parts.push(format!("chat_id {}", chat_id));
    }
    if let Some(index) = &turn.index {
        parts.push(format!("index {}", index));
    }
    if let Some(anchor) = &turn.time_anchor {
        parts.push(format!("time_anchor {}", anchor));
    }
    if let Some(question_type) = &turn.question_type {
        parts.push(format!("question_type {}", question_type));
    }
    parts.join(" | ")
}

fn append_profile_preferences(text: &str, lines: &mut Vec<String>) {
    let lowered = text.to_ascii_lowercase();
    for needle in [
        "prefer",
        "preference",
        "favorite",
        "lightweight",
        "simple",
        "maintain",
    ] {
        if lowered.contains(needle) {
            lines.push(text.to_string());
            return;
        }
    }
}

fn extract_subject_value(text: &str) -> Option<(String, String)> {
    for separator in [" is on ", " is at ", " is in ", " averages ", " average is ", " is "] {
        let lowered = text.to_ascii_lowercase();
        let index = lowered.find(separator)?;
        let subject = text[..index]
            .trim()
            .trim_start_matches("please remember that ")
            .trim_start_matches("Please remember that ")
            .trim_start_matches("the ")
            .trim_start_matches("my ")
            .trim()
            .to_ascii_lowercase();
        let value = text[index + separator.len()..].trim().trim_end_matches('.');
        if subject.is_empty() || value.is_empty() {
            return None;
        }
        return Some((subject, value.to_string()));
    }
    None
}

fn squeeze(text: &str) -> String {
    let mut out = text.trim().replace('\n', " ");
    if out.len() > 160 {
        out.truncate(157);
        out.push_str("...");
    }
    out
}

fn dedupe(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        if !out.contains(&line) {
            out.push(line);
        }
    }
    out
}
