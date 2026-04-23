// Where: crates/vfs_cli_app/src/facts_policy.rs
// What: Shared policy for deciding which user-authored spans belong in `facts.md`.
// Why: `facts.md` canonicality must stay aligned between BEAM note generation and ingest guidance.
use crate::beam_bench::dataset::BeamConversation;
use crate::beam_bench::note_support::flatten_chat;
use std::collections::BTreeSet;

pub fn extract_fact_lines(conversation: &BeamConversation) -> Vec<String> {
    let mut lines = Vec::new();
    for turn in flatten_chat(&conversation.chat) {
        if turn.label() == "assistant" {
            continue;
        }
        let trimmed = turn.content.trim();
        if let Some(line) = extract_stable_statement_fact(trimmed) {
            lines.push(line);
        }
        for line in extract_intro_fact_fragments(trimmed) {
            lines.push(line);
        }
        for clause in extract_fact_clauses(trimmed) {
            lines.push(clause);
        }
    }
    dedupe_lines(lines)
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

fn extract_stable_statement_fact(text: &str) -> Option<String> {
    let normalized = text
        .trim()
        .trim_end_matches('.')
        .trim_start_matches("Please remember that ")
        .trim_start_matches("please remember that ");
    let lowered = normalized.to_ascii_lowercase();
    if has_non_fact_signal(&lowered) {
        return None;
    }
    if let Some((subject, value)) = split_once_insensitive(normalized, " is on ") {
        let subject = normalize_subject(subject);
        return Some(format!("{subject} date: {}", value.trim()));
    }
    if let Some((subject, value)) = split_once_insensitive(normalized, " is at ") {
        let subject = normalize_subject(subject);
        return Some(format!("{subject} time: {}", value.trim()));
    }
    if let Some((subject, value)) = split_once_insensitive(normalized, " is in ") {
        let subject = normalize_subject(subject);
        return Some(format!("{subject} location: {}", value.trim()));
    }
    if let Some((subject, value)) = split_once_insensitive(normalized, " has ") {
        let subject = normalize_subject(subject);
        let value = value.trim();
        if value
            .chars()
            .next()
            .is_some_and(|char| char.is_ascii_digit())
        {
            return Some(format!("{subject} detail: {value}"));
        }
    }
    if lowered.starts_with("my ") && lowered.contains(" preference is ") {
        return Some(format!("preference: {normalized}"));
    }
    None
}

fn extract_intro_fact_fragments(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if !lowered.starts_with("i'm ") && !lowered.starts_with("i am ") {
        return Vec::new();
    }
    let Some(marker) = lowered.find(", a ") else {
        return Vec::new();
    };
    let descriptor_start = marker + 4;
    let descriptor_tail = &trimmed[descriptor_start..];
    let descriptor_end = [",", ", and ", ", but ", ", so ", ", because ", ", who "]
        .iter()
        .filter_map(|needle| descriptor_tail.to_ascii_lowercase().find(needle))
        .min()
        .unwrap_or(descriptor_tail.len());
    let descriptor = descriptor_tail[..descriptor_end]
        .trim()
        .trim_end_matches(',');
    if descriptor.is_empty() || descriptor.len() > 120 {
        return Vec::new();
    }
    vec![format!("self descriptor: {descriptor}")]
}

fn extract_fact_clauses(text: &str) -> Vec<String> {
    text.split(['\n', ';'])
        .flat_map(|segment| segment.split(". "))
        .map(str::trim)
        .filter(|clause| is_fact_like_clause(clause))
        .map(clean_fact_clause)
        .filter(|clause| !clause.is_empty())
        .collect()
}

fn is_fact_like_clause(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 220 {
        return false;
    }
    if trimmed.contains('?') || trimmed.contains("```") || trimmed.starts_with('#') {
        return false;
    }
    let lowered = trimmed.to_ascii_lowercase();
    if !contains_first_person_marker(&lowered) {
        return false;
    }
    if has_non_fact_signal(&lowered) {
        return false;
    }
    let has_value_anchor = lowered.contains('$')
        || lowered.contains('/')
        || lowered.contains(" per month")
        || lowered.contains(" years old")
        || lowered.contains(" miles")
        || month_name_present(&lowered)
        || contains_digit(&lowered);
    let has_fact_predicate = [
        "met ",
        "work as ",
        "work in ",
        "receipt number ",
        "profession",
        "been with ",
        "live ",
        "lives ",
        "studying ",
        "paying ",
        "costs ",
        "probability ",
        "chose ",
        "switched ",
        "use ",
        "uses ",
        "rate ",
    ]
    .iter()
    .any(|needle| lowered.contains(needle));
    has_value_anchor || has_fact_predicate
}

fn clean_fact_clause(text: &str) -> String {
    text.trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_end_matches('.')
        .trim()
        .to_string()
}

fn contains_first_person_marker(text: &str) -> bool {
    text.starts_with("i ")
        || text.starts_with("i'm ")
        || text.starts_with("im ")
        || text.starts_with("i've ")
        || text.starts_with("my ")
        || text.contains(" i ")
        || text.contains(" i've ")
        || text.contains(" my ")
}

fn has_non_fact_signal(text: &str) -> bool {
    [
        "always ",
        "so ",
        "and for ",
        "i'm trying to ",
        "im trying to ",
        "i am trying to ",
        "i'm curious",
        "i'm considering",
        "i'm wondering",
        "i was wondering",
        "i'm worried",
        "i'm kinda worried",
        "i'm not sure",
        "i'm unsure",
        "i'm thinking maybe",
        "i've got a decision",
        "i've got a deadline",
        "i have a meeting",
        "i had a great review",
        "i celebrated",
        "just to confirm",
        "can you help",
        "what's ",
        "what are ",
        "should i ",
        "thanks",
        "thank you",
        "sounds good",
        "got it",
        "yeah,",
        "yeah ",
        "sure,",
        "sure ",
        "i hope",
        "i think",
        "i'll ",
        "i will ",
        "meeting with ",
        "deadline ",
        "every wednesday",
        "i want to know",
        "->->",
    ]
    .iter()
    .any(|needle| text.contains(needle))
        || text.contains('?')
        || text.starts_with("**")
}

fn contains_digit(text: &str) -> bool {
    text.chars().any(|char| char.is_ascii_digit())
}

fn month_name_present(text: &str) -> bool {
    [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ]
    .iter()
    .any(|month| text.contains(month))
}

fn split_once_insensitive<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let lowered = text.to_ascii_lowercase();
    let index = lowered.find(needle)?;
    Some((&text[..index], &text[index + needle.len()..]))
}

fn normalize_subject(subject: &str) -> String {
    let lowered = subject.trim().to_ascii_lowercase();
    lowered
        .trim_start_matches("the ")
        .trim_start_matches("my ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::extract_fact_lines;
    use crate::beam_bench::dataset::BeamConversation;
    use serde_json::json;

    fn conversation_with_chat(text: &str) -> BeamConversation {
        BeamConversation {
            conversation_id: "Conv 1".to_string(),
            conversation_seed: json!({"title":"Sample","category":"General"}),
            narratives: String::new(),
            user_profile: json!({}),
            conversation_plan: String::new(),
            user_questions: json!([]),
            chat: json!([[{"role":"user","content":text}]]),
            probing_questions: "{}".to_string(),
        }
    }

    #[test]
    fn keeps_numeric_non_temporal_statements() {
        let lines = extract_fact_lines(&conversation_with_chat("The order has 3 items."));
        assert!(lines.contains(&"order detail: 3 items".to_string()));
    }

    #[test]
    fn skips_temporal_ordered_statements() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "The first delivery arrives after the second.",
        ));
        assert!(!lines.iter().any(|line| line.contains("delivery")));
    }

    #[test]
    fn drops_topic_only_requests() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "Can you help me improve the UI/UX before the public launch?",
        ));
        assert!(!lines.iter().any(|line| line.contains("UI/UX")));
    }

    #[test]
    fn keeps_statement_fact_without_seed_dump() {
        let mut conversation =
            conversation_with_chat("Please remember that the meeting is on March 15, 2024.");
        conversation.conversation_plan =
            "BATCH 1 PLAN\n• **Current Situation:** drafting the memo".to_string();
        conversation.narratives = "Label dump".to_string();
        let lines = extract_fact_lines(&conversation);
        assert!(lines.contains(&"meeting date: March 15, 2024".to_string()));
        assert!(!lines.iter().any(|line| line.contains("conversation_plan")));
    }

    #[test]
    fn keeps_settled_selection_and_relationships() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "I chose Adidas Ultraboost after trying both. I've been with Douglas for 3 years and my parents live 12 miles away.",
        ));
        assert!(lines.iter().any(|line| line.contains("Adidas Ultraboost")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Douglas for 3 years"))
        );
        assert!(lines.iter().any(|line| line.contains("12 miles away")));
    }

    #[test]
    fn drops_questions_and_thanks() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "I'm trying to decide if saving $600 is worth it. Thanks for the detailed guide!",
        ));
        assert!(lines.is_empty());
    }

    #[test]
    fn drops_future_meeting_and_deadline_lines() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "I have a meeting with Ashlee at 3 PM on May 14, 2024. I've got a deadline to meet on November 10, 2024.",
        ));
        assert!(lines.is_empty());
    }

    #[test]
    fn keeps_intro_descriptor_without_question_dump() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "I'm Craig, a 44-year-old colour technologist, and I'm trying to learn about probability basics.",
        ));
        assert!(lines.contains(&"self descriptor: 44-year-old colour technologist".to_string()));
        assert!(!lines.iter().any(|line| line.contains("trying to learn")));
    }

    #[test]
    fn drops_preferences_and_reflection_residue() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "Always provide step-by-step explanations. So for conditional probability, I write P(A|B).",
        ));
        assert!(lines.is_empty());
    }

    #[test]
    fn drops_filed_question_residue() {
        let lines = extract_fact_lines(&conversation_with_chat(
            "I filed on May 15, 2024 with receipt number 12345678, and I was wondering what to do next.",
        ));
        assert!(lines.is_empty());
    }
}
