use crate::cli::{SkillCommand, SkillRunOutcomeArg, SkillStatusArg};
mod model;
use anyhow::{Context, Result, anyhow};
use model::{
    FindAccumulator, PRIVATE_ROOT, PUBLIC_ROOT, RUN_ROOT, SkillId, SkillManifest, catalog,
    json_f64, normalize_manifest, now_millis, parse_manifest, print, read_optional,
    render_manifest, run_base_path, set_manifest_status_preserving_content, skill_base_path,
    skill_id_from_path,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use vfs_client::VfsApi;
use vfs_types::{
    NodeKind, RecentNodesRequest, SearchNodesRequest, SearchPreviewMode, WriteNodeRequest,
};

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
            json,
        } => print(
            upsert_skill(client, database_id, &source_dir, &id, public).await?,
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
) -> Result<serde_json::Value> {
    let skill_id = SkillId::parse(id)?;
    let base_path = skill_base_path(&skill_id, public);
    let skill = std::fs::read_to_string(source_dir.join("SKILL.md"))
        .with_context(|| format!("missing SKILL.md in {}", source_dir.display()))?;
    let provenance = read_optional(source_dir, "provenance.md")
        .unwrap_or_else(|| format!("# Provenance\n\nsource: {}\n", source_dir.display()));
    let evals = read_optional(source_dir, "evals.md").unwrap_or_else(|| "# Evals\n\n".to_string());
    let manifest = match read_optional(source_dir, "manifest.md") {
        Some(content) => normalize_manifest(&content, &skill_id)?,
        None => render_manifest(&SkillManifest::default_for(&skill_id)),
    };
    for (name, content) in [
        ("manifest.md", manifest),
        ("SKILL.md", skill),
        ("provenance.md", provenance),
        ("evals.md", evals),
    ] {
        write_file_node(client, database_id, &format!("{base_path}/{name}"), content).await?;
    }
    Ok(json!({ "id": id, "catalog": catalog(public), "base_path": base_path }))
}

pub(crate) async fn find_skills(
    client: &impl VfsApi,
    database_id: &str,
    query: &str,
    include_deprecated: bool,
    top_k: u32,
) -> Result<serde_json::Value> {
    let mut grouped: BTreeMap<(String, bool), FindAccumulator> = BTreeMap::new();
    for prefix in [PRIVATE_ROOT, PUBLIC_ROOT, RUN_ROOT] {
        for hit in client
            .search_nodes(SearchNodesRequest {
                database_id: database_id.to_string(),
                query_text: query.to_string(),
                prefix: Some(prefix.to_string()),
                top_k: top_k.clamp(1, 100),
                preview_mode: Some(SearchPreviewMode::Light),
            })
            .await?
        {
            if let Some((id, public)) = skill_id_from_path(&hit.path) {
                grouped.entry((id, public)).or_default().add_hit(hit);
            }
        }
    }
    let mut hits = Vec::new();
    for ((id, public), acc) in grouped {
        let manifest = read_manifest(client, database_id, &id, public).await?;
        let status = manifest
            .as_ref()
            .and_then(|m| m.status.clone())
            .unwrap_or_else(|| "draft".to_string());
        if status == "deprecated" && !include_deprecated {
            continue;
        }
        hits.push(json!({
            "id": id,
            "catalog": catalog(public),
            "status": status,
            "summary": manifest.and_then(|m| m.summary).unwrap_or_default(),
            "score": acc.score,
            "matched_paths": acc.paths.into_iter().collect::<Vec<_>>(),
            "why": acc.why.into_iter().collect::<Vec<_>>()
        }));
    }
    hits.sort_by(|a, b| json_f64(b, "score").total_cmp(&json_f64(a, "score")));
    hits.truncate(top_k.clamp(1, 100) as usize);
    Ok(json!({ "query": query, "hits": hits }))
}

pub(crate) async fn inspect_skill(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    public: bool,
) -> Result<serde_json::Value> {
    SkillId::parse(id)?;
    let base_path = skill_base_path(&SkillId::parse(id)?, public);
    let mut files = BTreeMap::new();
    let mut manifest = None;
    for name in ["manifest.md", "SKILL.md", "provenance.md", "evals.md"] {
        let node = client
            .read_node(database_id, &format!("{base_path}/{name}"))
            .await?;
        if name == "manifest.md" {
            manifest = node
                .as_ref()
                .and_then(|node| parse_manifest(&node.content).ok());
        }
        files.insert(name.to_string(), node.is_some());
    }
    let recent_runs = client
        .recent_nodes(RecentNodesRequest {
            database_id: database_id.to_string(),
            path: Some(run_base_path(&SkillId::parse(id)?)),
            limit: 5,
        })
        .await?
        .into_iter()
        .map(|hit| hit.path)
        .collect::<Vec<_>>();
    Ok(json!({
        "id": id,
        "catalog": catalog(public),
        "base_path": base_path,
        "manifest": manifest,
        "files": files,
        "recent_runs": recent_runs
    }))
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

async fn read_manifest(
    client: &impl VfsApi,
    database_id: &str,
    id: &str,
    public: bool,
) -> Result<Option<SkillManifest>> {
    let skill_id = SkillId::parse(id)?;
    Ok(client
        .read_node(
            database_id,
            &format!("{}/manifest.md", skill_base_path(&skill_id, public)),
        )
        .await?
        .and_then(|node| parse_manifest(&node.content).ok()))
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
