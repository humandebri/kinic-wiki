use crate::cli::{SkillCommand, SkillRunOutcomeArg, SkillStatusArg};
mod model;
use anyhow::{Context, Result, anyhow};
use model::{
    SkillId, catalog, manifest_for_source, normalize_manifest, now_millis,
    parse_skill_source_frontmatter, print, run_base_path, set_manifest_status_preserving_content,
    skill_base_path,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
pub(crate) use vfs_cli::skill_kb::{find_skills, inspect_skill};
use vfs_client::VfsApi;
use vfs_types::{DeleteNodeRequest, ListNodesRequest, NodeEntryKind, NodeKind, WriteNodeRequest};

pub async fn run_skill_command(
    client: &impl VfsApi,
    database_id: &str,
    command: SkillCommand,
) -> Result<()> {
    match command {
        SkillCommand::Upsert {
            source_dir,
            id,
            public,
            prune,
            json,
        } => print(
            upsert_skill(client, database_id, &source_dir, &id, public, prune).await?,
            json,
        )?,
        SkillCommand::Find {
            query,
            include_deprecated,
            top_k,
            json,
        } => print(
            find_skills(client, database_id, &query, include_deprecated, top_k).await?,
            json,
        )?,
        SkillCommand::Inspect { id, public, json } => {
            print(inspect_skill(client, database_id, &id, public).await?, json)?
        }
        SkillCommand::RecordRun {
            id,
            task,
            outcome,
            notes_file,
            json,
        } => print(
            record_skill_run(client, database_id, &id, &task, outcome, &notes_file).await?,
            json,
        )?,
        SkillCommand::SetStatus {
            id,
            status,
            public,
            json,
        } => print(
            set_skill_status(client, database_id, &id, status, public).await?,
            json,
        )?,
    }
    Ok(())
}

pub(crate) async fn upsert_skill(
    client: &impl VfsApi,
    database_id: &str,
    source_dir: &Path,
    id: &str,
    public: bool,
    prune: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let base_path = skill_base_path(&skill_id, public);
    let skill = std::fs::read_to_string(source_dir.join("SKILL.md"))
        .with_context(|| format!("missing SKILL.md in {}", source_dir.display()))?;
    let source_frontmatter = parse_skill_source_frontmatter(&skill)?;
    let files = discover_skill_package_files(source_dir, &skill, &skill_id, &source_frontmatter)?;
    let file_names = files.keys().cloned().collect::<BTreeSet<_>>();
    let mut written_paths = Vec::new();
    for (name, content) in files {
        write_file_node(client, database_id, &format!("{base_path}/{name}"), content).await?;
        written_paths.push(format!("{base_path}/{name}"));
    }
    let pruned_paths = if prune {
        prune_package_files(client, database_id, &base_path, &file_names).await?
    } else {
        Vec::new()
    };
    Ok(
        json!({ "id": id, "catalog": catalog(public), "base_path": base_path, "written_paths": written_paths, "pruned_paths": pruned_paths }),
    )
}

pub(crate) async fn record_skill_run(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    task: &str,
    outcome: SkillRunOutcomeArg,
    notes_file: &Path,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let notes = std::fs::read_to_string(notes_file)
        .with_context(|| format!("failed to read {}", notes_file.display()))?;
    let run_path = format!("{}/{}.md", run_base_path(&skill_id), now_millis());
    let outcome = outcome.as_str();
    let content = format!(
        "---\nkind: kinic.skill_run\nskill_id: {id}\noutcome: {outcome}\n---\n# Skill Run\n\n## Task\n\n{task}\n\n## Notes\n\n{notes}\n"
    );
    client
        .write_node(WriteNodeRequest {
            database_id: database_id.to_string(),
            path: run_path.clone(),
            kind: NodeKind::Source,
            content,
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await?;
    Ok(json!({ "id": id, "run_path": run_path, "outcome": outcome }))
}

pub(crate) async fn set_skill_status(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    status: SkillStatusArg,
    public: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let path = format!("{}/manifest.md", skill_base_path(&skill_id, public));
    let node = client
        .read_node(database_id, &path)
        .await?
        .ok_or_else(|| anyhow!("manifest not found: {path}"))?;
    let content = set_manifest_status_preserving_content(&node.content, status.as_str())?;
    client
        .write_node(WriteNodeRequest {
            database_id: database_id.to_string(),
            path: path.clone(),
            kind: NodeKind::File,
            content,
            metadata_json: node.metadata_json,
            expected_etag: Some(node.etag),
        })
        .await?;
    Ok(json!({ "id": id, "catalog": catalog(public), "status": status.as_str(), "path": path }))
}

async fn write_file_node(
    client: &impl VfsApi,
    database_id: &str,
    path: &str,
    content: String,
) -> Result<()> {
    let current = client.read_node(database_id, path).await?;
    client
        .write_node(WriteNodeRequest {
            database_id: database_id.to_string(),
            path: path.to_string(),
            kind: NodeKind::File,
            content,
            metadata_json: "{}".to_string(),
            expected_etag: current.map(|node| node.etag),
        })
        .await?;
    Ok(())
}

async fn prune_package_files(
    client: &impl VfsApi,
    database_id: &str,
    base_path: &str,
    keep_files: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let mut pruned_paths = Vec::new();
    for entry in client
        .list_nodes(ListNodesRequest {
            database_id: database_id.to_string(),
            prefix: base_path.to_string(),
            recursive: true,
        })
        .await?
    {
        if entry.kind != NodeEntryKind::File {
            continue;
        }
        let Some(relative_path) = entry.path.strip_prefix(&format!("{base_path}/")) else {
            continue;
        };
        if keep_files.contains(relative_path) {
            continue;
        }
        client
            .delete_node(DeleteNodeRequest {
                database_id: database_id.to_string(),
                path: entry.path.clone(),
                expected_etag: Some(entry.etag),
            })
            .await?;
        pruned_paths.push(entry.path);
    }
    Ok(pruned_paths)
}

fn discover_skill_package_files(
    source_dir: &Path,
    skill: &str,
    id: &SkillId,
    source_frontmatter: &model::SkillSourceFrontmatter,
) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    files.insert("SKILL.md".to_string(), skill.to_string());
    let manifest = match read_optional(source_dir, "manifest.md") {
        Some(content) => normalize_manifest(&content, id, source_frontmatter)?,
        None => manifest_for_source(id, source_frontmatter)?,
    };
    files.insert("manifest.md".to_string(), manifest);
    for name in ["provenance.md", "evals.md"] {
        if let Some(content) = read_optional(source_dir, name) {
            files.insert(name.to_string(), content);
        }
    }
    for relative_path in referenced_markdown_files(source_dir, skill)? {
        if files.contains_key(&relative_path) {
            continue;
        }
        if let Some(content) = read_optional(source_dir, &relative_path) {
            files.insert(relative_path, content);
        }
    }
    Ok(files)
}

fn referenced_markdown_files(source_dir: &Path, skill: &str) -> Result<Vec<String>> {
    let canonical_source_dir = source_dir
        .canonicalize()
        .with_context(|| format!("failed to read {}", source_dir.display()))?;
    let mut files = Vec::new();
    for target in markdown_link_targets(skill) {
        if let Some(relative_path) = package_relative_markdown_path(&canonical_source_dir, &target)?
        {
            files.push(relative_path);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn markdown_link_targets(content: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        targets.push(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    targets
}

fn package_relative_markdown_path(
    canonical_source_dir: &Path,
    raw_target: &str,
) -> Result<Option<String>> {
    let Some(target) = clean_markdown_link_target(raw_target) else {
        return Ok(None);
    };
    let path = PathBuf::from(target);
    if path.is_absolute() {
        return Ok(None);
    }
    let candidate = canonical_source_dir.join(path);
    if !candidate.is_file() {
        return Ok(None);
    }
    let canonical_candidate = candidate
        .canonicalize()
        .with_context(|| format!("failed to read {}", candidate.display()))?;
    let Ok(relative_path) = canonical_candidate.strip_prefix(canonical_source_dir) else {
        return Ok(None);
    };
    Ok(path_to_package_key(relative_path))
}

fn clean_markdown_link_target(raw_target: &str) -> Option<String> {
    let target = raw_target.split_whitespace().next()?.trim();
    let target = target.split(['#', '?']).next()?.trim();
    if target.is_empty()
        || target.starts_with('#')
        || target.starts_with('/')
        || target.contains("://")
        || !target.ends_with(".md")
    {
        return None;
    }
    Some(target.to_string())
}

fn path_to_package_key(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let std::path::Component::Normal(part) = component else {
            return None;
        };
        parts.push(part.to_str()?.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn read_optional(source_dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(source_dir.join(name)).ok()
}

impl SkillStatusArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Reviewed => "reviewed",
            Self::Promoted => "promoted",
            Self::Deprecated => "deprecated",
        }
    }
}

impl SkillRunOutcomeArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Fail => "fail",
        }
    }
}
