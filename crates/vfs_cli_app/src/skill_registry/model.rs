// Where: crates/vfs_cli_app/src/skill_registry/model.rs
// What: Skill Knowledge Base path, manifest, and ranking helpers.
// Why: The CLI workflow stays thin while manifest handling remains testable and schema-free.
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use vfs_types::SearchNodeHit;

pub(super) const PRIVATE_ROOT: &str = "/Wiki/skills";
pub(super) const PUBLIC_ROOT: &str = "/Wiki/public-skills";
pub(super) const RUN_ROOT: &str = "/Sources/skill-runs";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SkillManifest {
    pub(super) kind: String,
    pub(super) schema_version: u32,
    pub(super) id: String,
    pub(super) version: String,
    pub(super) publisher: String,
    pub(super) entry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) use_cases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) replaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) related: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) knowledge: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) permissions: BTreeMap<String, bool>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) provenance: BTreeMap<String, String>,
}

impl SkillManifest {
    pub(super) fn default_for(id: &SkillId) -> Self {
        Self {
            kind: "kinic.skill".to_string(),
            schema_version: 1,
            id: id.to_string(),
            version: "0.1.0".to_string(),
            publisher: id.publisher.clone(),
            entry: "SKILL.md".to_string(),
            summary: None,
            tags: Vec::new(),
            use_cases: Vec::new(),
            status: Some("draft".to_string()),
            replaces: Vec::new(),
            related: Vec::new(),
            knowledge: Vec::new(),
            permissions: BTreeMap::new(),
            provenance: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SkillId {
    pub(super) publisher: String,
    pub(super) name: String,
}

impl SkillId {
    pub(super) fn parse(value: &str) -> Result<Self> {
        let (publisher, name) = value
            .split_once('/')
            .ok_or_else(|| anyhow!("skill id must use publisher/name"))?;
        if !valid_segment(publisher) || !valid_segment(name) {
            return Err(anyhow!("skill id must use publisher/name"));
        }
        Ok(Self {
            publisher: publisher.to_string(),
            name: name.to_string(),
        })
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.publisher, self.name)
    }
}

#[derive(Default)]
pub(super) struct FindAccumulator {
    pub(super) score: f64,
    pub(super) paths: BTreeSet<String>,
    pub(super) why: BTreeSet<String>,
}

impl FindAccumulator {
    pub(super) fn add_hit(&mut self, hit: SearchNodeHit) {
        self.score += f64::from(hit.score);
        self.paths.insert(hit.path);
        for reason in hit.match_reasons {
            self.why.insert(reason);
        }
    }
}

pub(super) fn normalize_manifest(content: &str, id: &SkillId) -> Result<String> {
    let mut manifest = parse_manifest(content)?;
    if manifest.id != id.to_string() {
        return Err(anyhow!("manifest id must match --id"));
    }
    manifest.publisher = id.publisher.clone();
    if manifest.status.is_none() {
        manifest.status = Some("draft".to_string());
    }
    Ok(render_manifest(&manifest))
}

pub(super) fn parse_manifest(content: &str) -> Result<SkillManifest> {
    let manifest: SkillManifest = serde_yaml::from_str(extract_frontmatter(content)?)?;
    if manifest.kind != "kinic.skill" || manifest.schema_version != 1 {
        return Err(anyhow!("manifest must be kinic.skill schema_version 1"));
    }
    if manifest.entry != "SKILL.md" {
        return Err(anyhow!("manifest entry must be SKILL.md"));
    }
    let id = SkillId::parse(&manifest.id)?;
    if manifest.publisher != id.publisher {
        return Err(anyhow!("manifest publisher must match id"));
    }
    Ok(manifest)
}

pub(super) fn render_manifest(manifest: &SkillManifest) -> String {
    let yaml = serde_yaml::to_string(manifest).expect("manifest should serialize");
    format!("---\n{}---\n# Skill Manifest\n", yaml)
}

pub(super) fn set_manifest_status_preserving_content(
    content: &str,
    status: &str,
) -> Result<String> {
    parse_manifest(content)?;
    let frontmatter = extract_frontmatter(content)?;
    let frontmatter_start = "---\n".len();
    let frontmatter_end = frontmatter_start + frontmatter.len();
    let mut replaced = false;
    let mut updated = String::new();
    for line in frontmatter.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let newline = if line.ends_with('\n') { "\n" } else { "" };
        if !line.starts_with(' ') && line_without_newline.trim_start().starts_with("status:") {
            updated.push_str(&format!("status: {status}{newline}"));
            replaced = true;
        } else {
            updated.push_str(line);
        }
    }
    if !replaced {
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&format!("status: {status}"));
    }
    Ok(format!(
        "{}{}{}",
        &content[..frontmatter_start],
        updated,
        &content[frontmatter_end..]
    ))
}

pub(super) fn skill_base_path(id: &SkillId, public: bool) -> String {
    format!(
        "{}/{}/{}",
        if public { PUBLIC_ROOT } else { PRIVATE_ROOT },
        id.publisher,
        id.name
    )
}

pub(super) fn run_base_path(id: &SkillId) -> String {
    format!("{}/{}/{}", RUN_ROOT, id.publisher, id.name)
}

pub(super) fn skill_id_from_path(path: &str) -> Option<(String, bool)> {
    if let Some(rest) = path.strip_prefix(&format!("{PRIVATE_ROOT}/")) {
        return skill_id_from_registry_path(rest).map(|id| (id, false));
    }
    if let Some(rest) = path.strip_prefix(&format!("{PUBLIC_ROOT}/")) {
        return skill_id_from_registry_path(rest).map(|id| (id, true));
    }
    path.strip_prefix(&format!("{RUN_ROOT}/"))
        .and_then(skill_id_from_registry_path)
        .map(|id| (id, false))
}

pub(super) fn read_optional(source_dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(source_dir.join(name)).ok()
}

pub(super) fn catalog(public: bool) -> &'static str {
    if public { "public" } else { "private" }
}

pub(super) fn json_f64(value: &serde_json::Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or_default()
}

pub(super) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64
}

pub(super) fn print(value: serde_json::Value, _json_output: bool) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn extract_frontmatter(content: &str) -> Result<&str> {
    let rest = content
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("manifest must start with YAML frontmatter"))?;
    let end = rest
        .find("\n---")
        .ok_or_else(|| anyhow!("manifest frontmatter is not closed"))?;
    Ok(&rest[..end])
}

fn skill_id_from_registry_path(rest: &str) -> Option<String> {
    let mut parts = rest.split('/');
    let publisher = parts.next()?;
    let name = parts.next()?;
    if valid_segment(publisher) && valid_segment(name) {
        Some(format!("{publisher}/{name}"))
    } else {
        None
    }
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}
