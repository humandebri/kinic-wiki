use crate::cli::{SkillCommand, SkillImportCommand, SkillRunOutcomeArg, SkillStatusArg};
use crate::github_source::{
    fetch_github_optional_package_file, fetch_github_skill_package, github_source_string,
    github_source_url, parse_github_skill_source,
};
mod model;
use anyhow::{Context, Result, anyhow};
use model::{
    PRIVATE_ROOT, PUBLIC_ROOT, SkillId, catalog, extract_frontmatter, manifest_for_source,
    normalize_manifest, now_millis, now_rfc3339, parse_skill_source_frontmatter, print,
    run_base_path, set_manifest_provenance_field, set_manifest_status_preserving_content,
    set_root_frontmatter_field_preserving_content, skill_base_path,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
pub(crate) use vfs_cli::skill_kb::{find_skills, inspect_skill};
use vfs_client::VfsApi;
use vfs_types::{
    DeleteNodeRequest, ListNodesRequest, MkdirNodeRequest, NodeEntryKind, NodeKind, WriteNodeItem,
    WriteNodeRequest, WriteNodesRequest,
};

const SKILL_PACKAGE_FILE_LIMIT_MAX: usize = 100;

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
            agent,
            public,
            json,
        } => print(
            record_skill_run(
                client,
                SkillRunInput {
                    database_id,
                    id: &id,
                    task: &task,
                    outcome,
                    notes_file: &notes_file,
                    agent: &agent,
                    public,
                },
            )
            .await?,
            json,
        )?,
        SkillCommand::SetStatus {
            id,
            status,
            reason,
            public,
            json,
        } => print(
            set_skill_status(client, database_id, &id, status, reason.as_deref(), public).await?,
            json,
        )?,
        SkillCommand::Import { source } => match source {
            SkillImportCommand::Github {
                source,
                id,
                reference,
                public,
                prune,
                json,
            } => print(
                import_github_skill(client, database_id, &source, &id, &reference, public, prune)
                    .await?,
                json,
            )?,
        },
        SkillCommand::ProposeImprovement {
            id,
            runs,
            summary,
            diff_file,
            public,
            json,
        } => print(
            propose_improvement(
                client,
                database_id,
                &id,
                &runs,
                &summary,
                &diff_file,
                public,
            )
            .await?,
            json,
        )?,
        SkillCommand::ApproveProposal {
            id,
            proposal_path,
            json,
        } => print(
            approve_proposal(client, database_id, &id, &proposal_path).await?,
            json,
        )?,
        SkillCommand::Install {
            id,
            lockfile,
            public,
            json,
        } => print(
            install_skill_lockfile(client, database_id, &id, &lockfile, public).await?,
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
    let skill = std::fs::read_to_string(source_dir.join("SKILL.md"))
        .with_context(|| format!("missing SKILL.md in {}", source_dir.display()))?;
    let source_frontmatter = parse_skill_source_frontmatter(&skill)?;
    let files = discover_skill_package_files(source_dir, &skill, &skill_id, &source_frontmatter)?;
    write_skill_package(client, database_id, &skill_id, public, prune, files).await
}

async fn write_skill_package(
    client: &impl VfsApi,
    database_id: &str,
    skill_id: &SkillId,
    public: bool,
    prune: bool,
    files: BTreeMap<String, String>,
) -> Result<serde_json::Value> {
    validate_skill_package_file_count(files.len())?;
    let base_path = skill_base_path(skill_id, public);
    let file_names = files.keys().cloned().collect::<BTreeSet<_>>();
    let entries = files.into_iter().collect::<Vec<_>>();
    let paths = entries
        .iter()
        .map(|(name, _)| format!("{base_path}/{name}"))
        .collect::<Vec<_>>();
    ensure_parent_folders_for_paths(client, database_id, &paths).await?;
    let mut written_paths = Vec::new();
    let mut nodes = Vec::new();
    for ((_, content), path) in entries.into_iter().zip(paths) {
        let current = client.read_node(database_id, &path).await?;
        nodes.push(WriteNodeItem {
            path: path.clone(),
            kind: NodeKind::File,
            content,
            metadata_json: "{}".to_string(),
            expected_etag: current.map(|node| node.etag),
        });
        written_paths.push(path);
    }
    client
        .write_nodes(WriteNodesRequest {
            database_id: database_id.to_string(),
            nodes,
        })
        .await?;
    let pruned_paths = if prune {
        prune_package_files(client, database_id, &base_path, &file_names).await?
    } else {
        Vec::new()
    };
    Ok(
        json!({ "id": skill_id.to_string(), "catalog": catalog(public), "base_path": base_path, "written_paths": written_paths, "pruned_paths": pruned_paths }),
    )
}

pub(crate) async fn record_skill_run(
    client: &impl VfsApi,
    input: SkillRunInput<'_>,
) -> Result<serde_json::Value> {
    let SkillRunInput {
        database_id,
        id,
        task,
        outcome,
        notes_file,
        agent,
        public,
    } = input;
    let notes = std::fs::read_to_string(notes_file)
        .with_context(|| format!("failed to read {}", notes_file.display()))?;
    vfs_cli::skill_kb::record_skill_run(
        client,
        vfs_cli::skill_kb::SkillRunRecord {
            database_id,
            id,
            task,
            outcome: outcome.into(),
            notes: &notes,
            agent,
            public,
        },
    )
    .await
}

pub(crate) struct SkillRunInput<'a> {
    pub(crate) database_id: &'a str,
    pub(crate) id: &'a str,
    pub(crate) task: &'a str,
    pub(crate) outcome: SkillRunOutcomeArg,
    pub(crate) notes_file: &'a Path,
    pub(crate) agent: &'a str,
    pub(crate) public: bool,
}

pub(crate) async fn set_skill_status(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    status: SkillStatusArg,
    reason: Option<&str>,
    public: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let path = format!("{}/manifest.md", skill_base_path(&skill_id, public));
    let node = client
        .read_node(database_id, &path)
        .await?
        .ok_or_else(|| anyhow!("manifest not found: {path}"))?;
    let mut content = set_manifest_status_preserving_content(&node.content, status.as_str())?;
    let timestamp = now_rfc3339();
    match status {
        SkillStatusArg::Promoted => {
            content =
                set_root_frontmatter_field_preserving_content(&content, "promoted_at", &timestamp)?;
        }
        SkillStatusArg::Deprecated => {
            if let Some(reason) = reason {
                content = set_root_frontmatter_field_preserving_content(
                    &content,
                    "deprecated_reason",
                    reason,
                )?;
            }
            content = set_root_frontmatter_field_preserving_content(
                &content,
                "deprecated_at",
                &timestamp,
            )?;
        }
        SkillStatusArg::Draft | SkillStatusArg::Reviewed => {}
    }
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

async fn import_github_skill(
    client: &impl VfsApi,
    database_id: &str,
    source: &str,
    id: &str,
    reference: &str,
    public: bool,
    prune: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let source = parse_github_skill_source(source, None)?;
    let package = fetch_github_skill_package(source, reference).await?;
    let source_frontmatter = parse_skill_source_frontmatter(&package.skill)?;
    let mut files = BTreeMap::new();
    files.insert("SKILL.md".to_string(), package.skill);
    let mut manifest = match package.manifest {
        Some(content) => normalize_manifest(&content, &skill_id, &source_frontmatter)?,
        None => manifest_for_source(&skill_id, &source_frontmatter)?,
    };
    manifest =
        set_manifest_provenance_field(&manifest, "source", &github_source_string(&package.source))?;
    manifest = set_manifest_provenance_field(
        &manifest,
        "source_url",
        &github_source_url(&package.source, &package.resolved_ref),
    )?;
    manifest = set_manifest_provenance_field(&manifest, "revision", &package.resolved_ref)?;
    files.insert("manifest.md".to_string(), manifest);
    if let Some(provenance) = package.provenance {
        files.insert("provenance.md".to_string(), provenance);
    }
    if let Some(evals) = package.evals {
        files.insert("evals.md".to_string(), evals);
    }
    for target in markdown_link_targets(files.get("SKILL.md").expect("SKILL.md should exist")) {
        let Some(relative_path) = markdown_target_package_key(&target) else {
            continue;
        };
        if files.contains_key(&relative_path) {
            continue;
        }
        if let Some(content) = fetch_github_optional_package_file(
            &package.source,
            &package.resolved_ref,
            &relative_path,
        )
        .await?
        {
            files.insert(relative_path, content);
        }
    }
    write_skill_package(client, database_id, &skill_id, public, prune, files).await
}

pub(crate) async fn propose_improvement(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    runs: &[String],
    summary: &str,
    diff_file: &Path,
    public: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    for run in runs {
        if !run.starts_with(&format!("{}/", run_base_path(&skill_id))) {
            return Err(anyhow!(
                "proposal run path must belong to skill {id}: {run}"
            ));
        }
    }
    let diff = std::fs::read_to_string(diff_file)
        .with_context(|| format!("failed to read {}", diff_file.display()))?;
    let path_timestamp = now_millis();
    let created_at = now_rfc3339();
    let proposal_path = format!(
        "{}/improvement-proposals/{path_timestamp}.md",
        skill_base_path(&skill_id, public)
    );
    let source_runs = runs
        .iter()
        .map(|run| format!("  - {run}"))
        .collect::<Vec<_>>()
        .join("\n");
    let evidence_links = runs
        .iter()
        .map(|run| format!("- [{run}]({run})"))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "---\nkind: kinic.skill_improvement_proposal\nschema_version: 1\nskill_id: {id}\nstatus: proposed\nsource_runs:\n{source_runs}\ncreated_at: {created_at}\ncreated_by: cli\n---\n# Skill Improvement Proposal\n\n## Summary\n\n{summary}\n\n## Evidence\n\n{evidence_links}\n\n## Proposed Diff\n\n```diff\n{diff}\n```\n"
    );
    ensure_parent_folders(client, database_id, &proposal_path).await?;
    client
        .write_node(WriteNodeRequest {
            database_id: database_id.to_string(),
            path: proposal_path.clone(),
            kind: NodeKind::File,
            content,
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await?;
    Ok(json!({ "id": id, "proposal_path": proposal_path, "status": "proposed" }))
}

pub(crate) async fn approve_proposal(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    proposal_path: &str,
) -> Result<serde_json::Value> {
    validate_proposal_target(id, proposal_path)?;
    let node = client
        .read_node(database_id, proposal_path)
        .await?
        .ok_or_else(|| anyhow!("proposal not found: {proposal_path}"))?;
    validate_proposal_frontmatter(id, &node.content)?;
    let content =
        set_root_frontmatter_field_preserving_content(&node.content, "status", "approved")?;
    client
        .write_node(WriteNodeRequest {
            database_id: database_id.to_string(),
            path: proposal_path.to_string(),
            kind: NodeKind::File,
            content,
            metadata_json: node.metadata_json,
            expected_etag: Some(node.etag),
        })
        .await?;
    Ok(json!({ "id": id, "proposal_path": proposal_path, "status": "approved" }))
}

pub(crate) async fn install_skill_lockfile(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    lockfile: &Path,
    public: bool,
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let base_path = skill_base_path(&skill_id, public);
    let manifest_path = format!("{base_path}/manifest.md");
    let entry_path = format!("{base_path}/SKILL.md");
    let manifest = client
        .read_node(database_id, &manifest_path)
        .await?
        .ok_or_else(|| anyhow!("manifest not found: {manifest_path}"))?;
    let entry = client
        .read_node(database_id, &entry_path)
        .await?
        .ok_or_else(|| anyhow!("SKILL.md not found: {entry_path}"))?;
    let value = json!({
        "schema_version": 1,
        "database_id": database_id,
        "id": skill_id.to_string(),
        "public": public,
        "manifest_path": manifest_path,
        "entry_path": entry_path,
        "manifest_etag": manifest.etag.clone(),
        "entry_etag": entry.etag.clone(),
        "manifest_hash": sha256_hex(&manifest.content),
        "entry_hash": sha256_hex(&entry.content),
        "installed_at": now_rfc3339()
    });
    std::fs::write(lockfile, serde_json::to_string_pretty(&value)?)
        .with_context(|| format!("failed to write {}", lockfile.display()))?;
    Ok(json!({
        "id": skill_id.to_string(),
        "catalog": catalog(public),
        "lockfile": lockfile.display().to_string(),
        "manifest_path": value["manifest_path"],
        "entry_path": value["entry_path"]
    }))
}

#[derive(Deserialize)]
struct ProposalFrontmatter {
    kind: String,
    schema_version: u32,
    skill_id: String,
    status: String,
}

fn validate_proposal_target(id: &str, proposal_path: &str) -> Result<()> {
    let skill_id = SkillId::parse(id)?;
    let private_prefix = format!("{}/{}/improvement-proposals/", PRIVATE_ROOT, skill_id);
    let public_prefix = format!("{}/{}/improvement-proposals/", PUBLIC_ROOT, skill_id);
    if proposal_path.starts_with(&private_prefix) || proposal_path.starts_with(&public_prefix) {
        return Ok(());
    }
    Err(anyhow!(
        "proposal path must belong to skill {id} improvement-proposals"
    ))
}

fn validate_proposal_frontmatter(id: &str, content: &str) -> Result<()> {
    let frontmatter: ProposalFrontmatter = serde_yaml::from_str(extract_frontmatter(content)?)?;
    if frontmatter.kind != "kinic.skill_improvement_proposal" {
        return Err(anyhow!(
            "proposal kind must be kinic.skill_improvement_proposal"
        ));
    }
    if frontmatter.schema_version != 1 {
        return Err(anyhow!("proposal schema_version must be 1"));
    }
    if frontmatter.skill_id != id {
        return Err(anyhow!("proposal skill_id must match id"));
    }
    if frontmatter.status != "proposed" {
        return Err(anyhow!("proposal status must be proposed"));
    }
    Ok(())
}

async fn ensure_parent_folders(client: &impl VfsApi, database_id: &str, path: &str) -> Result<()> {
    ensure_parent_folders_for_paths(client, database_id, &[path.to_string()]).await
}

async fn ensure_parent_folders_for_paths(
    client: &impl VfsApi,
    database_id: &str,
    paths: &[String],
) -> Result<()> {
    let mut folders = BTreeSet::new();
    for path in paths {
        collect_parent_folders(path, &mut folders);
    }
    for folder in folders {
        client
            .mkdir_node(MkdirNodeRequest {
                database_id: database_id.to_string(),
                path: folder,
            })
            .await?;
    }
    Ok(())
}

fn validate_skill_package_file_count(count: usize) -> Result<()> {
    if count == 0 || count > SKILL_PACKAGE_FILE_LIMIT_MAX {
        return Err(anyhow!(
            "skill package file count must be between 1 and {SKILL_PACKAGE_FILE_LIMIT_MAX}"
        ));
    }
    Ok(())
}

fn collect_parent_folders(path: &str, folders: &mut BTreeSet<String>) {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut current = String::new();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        current.push('/');
        current.push_str(segment);
        folders.insert(current.clone());
    }
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

pub(crate) fn markdown_target_package_key(raw_target: &str) -> Option<String> {
    let target = clean_markdown_link_target(raw_target)?;
    path_to_package_key(Path::new(&target))
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
        match component {
            std::path::Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            std::path::Component::CurDir => {}
            _ => return None,
        }
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

fn sha256_hex(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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

impl From<SkillRunOutcomeArg> for vfs_cli::skill_kb::SkillRunOutcome {
    fn from(value: SkillRunOutcomeArg) -> Self {
        match value {
            SkillRunOutcomeArg::Success => Self::Success,
            SkillRunOutcomeArg::Partial => Self::Partial,
            SkillRunOutcomeArg::Fail => Self::Fail,
        }
    }
}
