// Where: crates/wiki_types/src/health.rs
// What: Health-check contracts for wiki maintenance flows.
// Why: LLM-driven wiki maintenance needs a structured report of issues to inspect or fix.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthIssueKind {
    OrphanPage,
    UnsupportedClaim,
    Contradiction,
    StaleClaim,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthIssue {
    pub kind: HealthIssueKind,
    pub page_slug: Option<String>,
    pub section_path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckReport {
    pub issues: Vec<HealthIssue>,
}
