// Where: crates/wiki_cli/src/beam_bench/note_views.rs
// What: Additional BEAM note renderers beyond the core conversation/facts/events/profile notes.
// Why: Full BEAM coverage needs extra note roles without bloating the primary renderer file.
use super::dataset::BeamConversation;
use super::note_extract::{
    extract_instruction_lines, extract_preference_lines, extract_summary_lines,
    extract_update_lines,
};
use super::note_support::append_related_section;

pub fn preferences_markdown(conversation: &BeamConversation, base_path: &str) -> String {
    let mut out = String::new();
    out.push_str("# Preferences\n\n");
    append_related_section(
        &mut out,
        base_path,
        &["index.md", "conversation.md", "profile.md", "summary.md"],
    );
    out.push_str("## Stable Preferences\n\n");
    for line in extract_preference_lines(conversation) {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

pub fn instructions_markdown(conversation: &BeamConversation, base_path: &str) -> String {
    let mut out = String::new();
    out.push_str("# Instructions\n\n");
    append_related_section(
        &mut out,
        base_path,
        &["index.md", "conversation.md", "plan.md"],
    );
    out.push_str("## Directives And Constraints\n\n");
    for line in extract_instruction_lines(conversation) {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

pub fn updates_markdown(conversation: &BeamConversation, base_path: &str) -> String {
    let mut out = String::new();
    out.push_str("# Updates\n\n");
    append_related_section(
        &mut out,
        base_path,
        &["index.md", "conversation.md", "facts.md", "events.md"],
    );
    out.push_str("## Latest Versus Previous Values\n\n");
    for line in extract_update_lines(conversation) {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

pub fn summary_markdown(conversation: &BeamConversation, base_path: &str) -> String {
    let mut out = String::new();
    out.push_str("# Summary\n\n");
    append_related_section(
        &mut out,
        base_path,
        &["index.md", "conversation.md", "facts.md", "events.md"],
    );
    out.push_str("## Synthesized Overview\n\n");
    for line in extract_summary_lines(conversation) {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    out
}
