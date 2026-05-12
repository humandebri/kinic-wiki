// Where: crates/vfs_cli_app/src/aeo_generate/types.rs
// What: Shared data contracts for frontend AEO dry-run generation.
// Why: Manifest, validation, and CLI output need stable structured shapes.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AeoGenerateArgs {
    pub repo: PathBuf,
    pub out: PathBuf,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AeoGenerationReport {
    pub project_name: String,
    pub project_slug: String,
    pub framework: String,
    pub sources: Vec<SourceEntry>,
    pub outputs: Vec<GeneratedOutput>,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceEntry {
    pub path: String,
    pub kind: SourceKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Readme,
    PublicDoc,
    NextAppPage,
    NextAppLayout,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedOutput {
    pub kind: GeneratedOutputKind,
    pub path: String,
    pub slug: Option<String>,
    pub title: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedOutputKind {
    Wiki,
    Answer,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ValidationReport {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub project_name: String,
    pub project_slug: String,
    pub framework: String,
    pub answers: Vec<ManifestAnswer>,
    pub wiki_pages: Vec<ManifestWikiPage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestAnswer {
    pub slug: String,
    pub title: String,
    pub path: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestWikiPage {
    pub title: String,
    pub path: String,
    pub sources: Vec<String>,
}
