// Where: crates/vfs_cli_core/src/skill_kb.rs
// What: Read-only Skill Knowledge Base helpers shared by CLI and agent tools.
// Why: Human CLI and agent runtime must rank and inspect skill packages identically.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use vfs_client::VfsApi;
use vfs_types::{
    ListNodesRequest, NodeEntryKind, RecentNodesRequest, SearchNodesRequest, SearchPreviewMode,
};

const PRIVATE_SKILL_ROOT: &str = "/Wiki/skills";
const PUBLIC_SKILL_ROOT: &str = "/Wiki/public-skills";
const SKILL_RUN_ROOT: &str = "/Sources/skill-runs";

#[derive(Default)]
struct SkillHitAccumulator {
    best_score: Option<f64>,
    matched_paths: BTreeSet<String>,
    why: BTreeSet<String>,
}

impl SkillHitAccumulator {
    fn add(&mut self, hit: vfs_types::SearchNodeHit) {
        let score = f64::from(hit.score);
        self.best_score = Some(
            self.best_score
                .map(|current| current.min(score))
                .unwrap_or(score),
        );
        self.matched_paths.insert(hit.path);
        self.why.extend(hit.match_reasons);
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SkillManifestView {
    pub id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub status: Option<String>,
    pub tags: Vec<String>,
    pub use_cases: Vec<String>,
}

pub async fn find_skills(
    client: &impl VfsApi,
    database_id: &str,
    query_text: &str,
    include_deprecated: bool,
    top_k: u32,
) -> Result<Value> {
    let top_k = top_k.clamp(1, 20);
    let mut grouped: BTreeMap<(String, bool), SkillHitAccumulator> = BTreeMap::new();
    for prefix in [PRIVATE_SKILL_ROOT, PUBLIC_SKILL_ROOT, SKILL_RUN_ROOT] {
        for hit in client
            .search_nodes(SearchNodesRequest {
                database_id: database_id.to_string(),
                query_text: query_text.to_string(),
                prefix: Some(prefix.to_string()),
                top_k,
                preview_mode: Some(SearchPreviewMode::Light),
            })
            .await?
        {
            if let Some((id, public)) = skill_id_from_path(&hit.path) {
                grouped.entry((id, public)).or_default().add(hit);
            }
        }
    }

    let mut hits = Vec::new();
    for ((id, public), acc) in grouped {
        let manifest = read_skill_manifest(client, database_id, &id, public).await?;
        let status = manifest
            .status
            .clone()
            .unwrap_or_else(|| "draft".to_string());
        if status == "deprecated" && !include_deprecated {
            continue;
        }
        hits.push(json!({
            "id": id,
            "catalog": skill_catalog(public),
            "status": status,
            "title": manifest.title.unwrap_or_default(),
            "summary": manifest.summary.unwrap_or_default(),
            "score": acc.best_score.unwrap_or_default(),
            "matched_paths": acc.matched_paths.into_iter().collect::<Vec<_>>(),
            "why": acc.why.into_iter().collect::<Vec<_>>()
        }));
    }
    hits.sort_by(|left, right| {
        left["score"]
            .as_f64()
            .unwrap_or_default()
            .total_cmp(&right["score"].as_f64().unwrap_or_default())
    });
    hits.truncate(top_k as usize);
    Ok(json!({ "query": query_text, "hits": hits }))
}

pub async fn inspect_skill(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    public: bool,
) -> Result<Value> {
    validate_skill_id(id)?;
    let base_path = skill_base_path(id, public);
    let manifest = read_skill_manifest(client, database_id, id, public).await?;
    let mut files = BTreeMap::new();
    for name in ["manifest.md", "SKILL.md", "provenance.md", "evals.md"] {
        files.insert(name.to_string(), false);
    }
    for entry in client
        .list_nodes(ListNodesRequest {
            database_id: database_id.to_string(),
            prefix: base_path.clone(),
            recursive: true,
        })
        .await?
    {
        if entry.kind != NodeEntryKind::File {
            continue;
        }
        if let Some(relative_path) = entry.path.strip_prefix(&format!("{base_path}/")) {
            files.insert(relative_path.to_string(), true);
        }
    }
    let recent_runs = client
        .recent_nodes(RecentNodesRequest {
            database_id: database_id.to_string(),
            path: Some(format!("{SKILL_RUN_ROOT}/{id}")),
            limit: 5,
        })
        .await?
        .into_iter()
        .map(|hit| hit.path)
        .collect::<Vec<_>>();
    Ok(json!({
        "id": id,
        "catalog": skill_catalog(public),
        "base_path": base_path,
        "manifest": manifest,
        "files": files,
        "recent_runs": recent_runs
    }))
}

pub async fn read_skill_file(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    file: &str,
    public: bool,
) -> Result<Value> {
    validate_skill_id(id)?;
    let file = validate_package_file(file)?;
    let path = format!("{}/{}", skill_base_path(id, public), file);
    Ok(json!({ "node": client.read_node(database_id, &path).await? }))
}

async fn read_skill_manifest(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    public: bool,
) -> Result<SkillManifestView> {
    validate_skill_id(id)?;
    let Some(node) = client
        .read_node(
            database_id,
            &format!("{}/manifest.md", skill_base_path(id, public)),
        )
        .await?
    else {
        return Ok(SkillManifestView::default());
    };
    Ok(parse_manifest_view(&node.content))
}

fn parse_manifest_view(content: &str) -> SkillManifestView {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return SkillManifestView::default();
    };
    let mut manifest = SkillManifestView::default();
    let mut current_list: Option<&str> = None;
    for line in frontmatter.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(key) = current_list
            && (line.starts_with("  - ") || line.starts_with("- "))
        {
            let value = clean_yaml_value(line.trim_start_matches("  - ").trim_start_matches("- "));
            if key == "tags" {
                manifest.tags.push(value);
            } else if key == "use_cases" {
                manifest.use_cases.push(value);
            }
            continue;
        }
        current_list = None;
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = clean_yaml_value(value);
        match key {
            "id" => manifest.id = non_empty(value),
            "title" => manifest.title = non_empty(value),
            "summary" => manifest.summary = non_empty(value),
            "status" => manifest.status = non_empty(value),
            "tags" | "use_cases" if value.is_empty() => current_list = Some(key),
            _ => {}
        }
    }
    manifest
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn clean_yaml_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn skill_id_from_path(path: &str) -> Option<(String, bool)> {
    if let Some(rest) = path.strip_prefix(&format!("{PRIVATE_SKILL_ROOT}/")) {
        return first_skill_segment(rest).map(|id| (id, false));
    }
    if let Some(rest) = path.strip_prefix(&format!("{PUBLIC_SKILL_ROOT}/")) {
        return first_skill_segment(rest).map(|id| (id, true));
    }
    path.strip_prefix(&format!("{SKILL_RUN_ROOT}/"))
        .and_then(first_skill_segment)
        .map(|id| (id, false))
}

fn first_skill_segment(rest: &str) -> Option<String> {
    let id = rest.split('/').next()?;
    validate_skill_id(id).ok()?;
    Some(id.to_string())
}

fn skill_base_path(id: &str, public: bool) -> String {
    format!(
        "{}/{}",
        if public {
            PUBLIC_SKILL_ROOT
        } else {
            PRIVATE_SKILL_ROOT
        },
        id
    )
}

fn skill_catalog(public: bool) -> &'static str {
    if public { "public" } else { "private" }
}

fn validate_skill_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        anyhow::bail!("skill id must use a single path-safe name");
    }
    Ok(())
}

fn validate_package_file(file: &str) -> Result<String> {
    let file = file.trim();
    if file.is_empty()
        || file.starts_with('/')
        || file.contains("://")
        || file
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        anyhow::bail!("skill file must be a package-local relative path");
    }
    Ok(file.to_string())
}
