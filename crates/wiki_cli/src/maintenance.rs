// Where: crates/wiki_cli/src/maintenance.rs
// What: Deterministic wiki maintenance helpers for index rebuild and log append.
// Why: VFS-first CLI keeps system-file updates explicit even after workflow command removal.
use anyhow::{Result, anyhow};
use chrono::Local;
use wiki_types::{ListNodesRequest, Node, NodeEntryKind, NodeKind, WriteNodeRequest};

use crate::client::WikiApi;

const WIKI_PREFIX: &str = "/Wiki";
const INDEX_PATH: &str = "/Wiki/index.md";
const LOG_PATH: &str = "/Wiki/log.md";

pub async fn rebuild_index(client: &impl WikiApi) -> Result<()> {
    let entries = client
        .list_nodes(ListNodesRequest {
            prefix: WIKI_PREFIX.to_string(),
            recursive: true,
        })
        .await?;
    let mut sources = Vec::new();
    let mut entities = Vec::new();
    let mut concepts = Vec::new();
    for entry in entries {
        if entry.kind != NodeEntryKind::File || entry.path == INDEX_PATH || entry.path == LOG_PATH {
            continue;
        }
        let node = match client.read_node(&entry.path).await? {
            Some(node) => node,
            None => continue,
        };
        let rendered = render_index_item(&node);
        if entry.path.starts_with("/Wiki/sources/") {
            sources.push(rendered);
        } else if entry.path.starts_with("/Wiki/entities/") {
            entities.push(rendered);
        } else if entry.path.starts_with("/Wiki/concepts/") {
            concepts.push(rendered);
        }
    }
    let body = render_index(&sources, &entities, &concepts);
    upsert_node(client, INDEX_PATH, NodeKind::File, &body).await
}

pub async fn append_log(
    client: &impl WikiApi,
    kind: &str,
    title: &str,
    target_paths: &[String],
    updated_paths: &[String],
    failure: Option<String>,
) -> Result<()> {
    let normalized_kind = normalize_log_kind(kind)?;
    let existing = client.read_node(LOG_PATH).await?;
    let mut content = existing
        .map(|node| node.content)
        .unwrap_or_else(|| "# Log\n".to_string());
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let stamp = Local::now().format("%Y-%m-%d %H:%M").to_string();
    content.push_str(&format!("## [{stamp}] {normalized_kind} | {title}\n"));
    if !target_paths.is_empty() {
        content.push_str(&format!("target_paths: {}\n", target_paths.join(", ")));
    }
    if !updated_paths.is_empty() {
        content.push_str(&format!("updated_paths: {}\n", updated_paths.join(", ")));
    }
    if let Some(reason) = failure {
        content.push_str(&format!("failure: {reason}\n"));
    }
    content.push('\n');
    upsert_node(client, LOG_PATH, NodeKind::File, &content).await
}

pub fn normalize_log_kind(kind: &str) -> Result<String> {
    let normalized = kind.trim();
    if normalized.is_empty() {
        return Err(anyhow!("kind must not be empty"));
    }
    if normalized.contains('\n') || normalized.contains('\r') {
        return Err(anyhow!("kind must not contain newlines"));
    }
    Ok(normalized.to_string())
}

async fn upsert_node(
    client: &impl WikiApi,
    path: &str,
    kind: NodeKind,
    content: &str,
) -> Result<()> {
    let current = client.read_node(path).await?;
    client
        .write_node(WriteNodeRequest {
            path: path.to_string(),
            kind,
            content: content.to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: current.map(|node| node.etag),
        })
        .await?;
    Ok(())
}

fn render_index(sources: &[String], entities: &[String], concepts: &[String]) -> String {
    let mut out = String::from("# Index\n\n");
    push_index_section(&mut out, "Sources", sources);
    push_index_section(&mut out, "Entities", entities);
    push_index_section(&mut out, "Concepts", concepts);
    out
}

fn push_index_section(out: &mut String, title: &str, items: &[String]) {
    out.push_str(&format!("## {title}\n\n"));
    if items.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for item in items {
        out.push_str(item);
        out.push('\n');
    }
    out.push('\n');
}

fn render_index_item(node: &Node) -> String {
    let label =
        title_from_content(&node.content).unwrap_or_else(|| fallback_label_from_path(&node.path));
    format!(
        "- [{}]({}) — {}",
        label,
        node.path,
        first_summary_line(&node.content)
    )
}

fn title_from_content(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::to_string))
}

fn fallback_label_from_path(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md")
        .to_string()
}

fn first_summary_line(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && *line != "---")
        .unwrap_or("No summary available.")
        .to_string()
}
