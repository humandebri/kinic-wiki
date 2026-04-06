// Where: crates/wiki_agent_schema/src/lib.rs
// What: Default agent-facing wiki maintenance rules and templates.
// Why: The runtime can expose a stable schema layer without hard-coding long prompts into application logic.
pub const INGEST_STEPS: &[&str] = &[
    "Read the raw source body before editing any wiki page.",
    "Create or update a source summary page when the source adds durable knowledge.",
    "Update related entity, concept, overview, and comparison pages in one pass when the source changes them.",
    "Write citations directly in markdown near the claims they support.",
];

pub const QUERY_FILE_BACK_RULES: &[&str] = &[
    "Read index.md before broad exploration.",
    "Use search as a supplement when index.md and direct page reads are not enough.",
    "File durable answers back into the wiki as comparison or query-note pages when useful.",
];

pub const LINT_CHECKS: &[&str] = &[
    "Find orphan pages with no inbound wiki links.",
    "Find pages that make claims without visible source markers.",
    "Surface pages that explicitly mention contradictions or staleness markers.",
];

pub const PAGE_TEMPLATES: &[(&str, &str)] = &[
    (
        "entity",
        "# Title\n\n## Summary\n\n## Details\n\n## Sources",
    ),
    ("concept", "# Title\n\n## Thesis\n\n## Notes\n\n## Sources"),
    (
        "overview",
        "# Title\n\n## Scope\n\n## Key Pages\n\n## Sources",
    ),
    (
        "comparison",
        "# Title\n\n## Compared Items\n\n## Differences\n\n## Sources",
    ),
    (
        "query_note",
        "# Title\n\n## Question\n\n## Answer\n\n## Sources",
    ),
    (
        "source_summary",
        "# Title\n\n## Source\n\n## Key Takeaways\n\n## Cross-References",
    ),
];
