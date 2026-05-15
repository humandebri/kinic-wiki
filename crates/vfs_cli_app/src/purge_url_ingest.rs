// Where: crates/vfs_cli_app/src/purge_url_ingest.rs
// What: Accident-response cleanup for URL ingest artifacts.
// Why: Operators need one dry-run-first command that finds request/source/wiki nodes before deleting them.
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use vfs_client::VfsApi;
use vfs_types::{DeleteNodeRequest, ListNodesRequest, Node, NodeKind};

const REQUEST_PREFIX: &str = "/Sources/ingest-requests";

#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    kind: Option<String>,
    status: Option<String>,
    url: Option<String>,
    source_path: Option<String>,
    target_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MatchedRequest {
    path: String,
    url: String,
    source_path: Option<String>,
    target_path: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct PurgeReport {
    ok: bool,
    dry_run: bool,
    matched_requests: Vec<MatchedRequest>,
    delete_plan: Vec<String>,
    deleted_paths: Vec<String>,
    skipped_paths: Vec<String>,
    errors: Vec<String>,
}

enum SourceLookup {
    Matched(Vec<MatchedRequest>),
    Skipped(String),
}

pub async fn purge_url_ingest(
    client: &impl VfsApi,
    database_id: &str,
    url: Option<&str>,
    source_path: Option<&str>,
    yes: bool,
    json: bool,
) -> Result<()> {
    let normalized_url = url.map(normalize_url).transpose()?;
    let mut matched = BTreeMap::new();
    let mut skipped_paths = Vec::new();
    for request in find_matching_requests(client, database_id, normalized_url.as_deref()).await? {
        matched.insert(request.path.clone(), request);
    }
    if let Some(source_path) = source_path {
        match request_for_source(client, database_id, source_path).await? {
            SourceLookup::Matched(requests) => {
                for request in requests {
                    matched.insert(request.path.clone(), request);
                }
            }
            SourceLookup::Skipped(reason) => skipped_paths.push(reason),
        }
    }
    let matched_requests = matched.into_values().collect::<Vec<_>>();
    let delete_plan = build_delete_plan(client, database_id, &matched_requests).await?;
    let mut report = PurgeReport {
        ok: true,
        dry_run: !yes,
        matched_requests,
        delete_plan,
        deleted_paths: Vec::new(),
        skipped_paths,
        errors: Vec::new(),
    };
    if yes {
        execute_delete_plan(client, database_id, &mut report).await?;
    }
    report.ok = report.errors.is_empty();
    print_report(&report, json)?;
    if yes && !report.ok {
        return Err(anyhow!(
            "purge_url_ingest failed to delete one or more paths"
        ));
    }
    Ok(())
}

async fn find_matching_requests(
    client: &impl VfsApi,
    database_id: &str,
    url: Option<&str>,
) -> Result<Vec<MatchedRequest>> {
    let Some(url) = url else {
        return Ok(Vec::new());
    };
    let entries = client
        .list_nodes(ListNodesRequest {
            database_id: database_id.to_string(),
            prefix: REQUEST_PREFIX.to_string(),
            recursive: true,
        })
        .await?;
    let mut matched = Vec::new();
    for entry in entries {
        let Some(node) = client.read_node(database_id, &entry.path).await? else {
            continue;
        };
        let Some(request) = parse_request_node(&node)? else {
            continue;
        };
        if request.url == url {
            matched.push(request);
        }
    }
    Ok(matched)
}

async fn request_for_source(
    client: &impl VfsApi,
    database_id: &str,
    source_path: &str,
) -> Result<SourceLookup> {
    let Some(source) = client.read_node(database_id, source_path).await? else {
        return Ok(SourceLookup::Skipped(format!(
            "{source_path}: source not found"
        )));
    };
    if source.kind != NodeKind::Source {
        return Ok(SourceLookup::Skipped(format!(
            "{source_path}: node is not a source"
        )));
    };
    let Some(frontmatter) = parse_frontmatter(&source.content)? else {
        return Ok(SourceLookup::Skipped(format!(
            "{source_path}: missing raw web source frontmatter"
        )));
    };
    if frontmatter.kind.as_deref() != Some("kinic.raw_web_source") {
        return Ok(SourceLookup::Skipped(format!(
            "{source_path}: not kinic.raw_web_source"
        )));
    }
    let entries = client
        .list_nodes(ListNodesRequest {
            database_id: database_id.to_string(),
            prefix: REQUEST_PREFIX.to_string(),
            recursive: true,
        })
        .await?;
    let mut matched = Vec::new();
    for entry in entries {
        let Some(node) = client.read_node(database_id, &entry.path).await? else {
            continue;
        };
        let Some(request) = parse_request_node(&node)? else {
            continue;
        };
        if request.source_path.as_deref() == Some(source_path) {
            matched.push(request);
        }
    }
    if !matched.is_empty() {
        return Ok(SourceLookup::Matched(matched));
    }
    Ok(SourceLookup::Skipped(format!(
        "{source_path}: matching ingest request not found"
    )))
}

async fn build_delete_plan(
    client: &impl VfsApi,
    database_id: &str,
    requests: &[MatchedRequest],
) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for request in requests {
        paths.insert(request.path.clone());
        if let Some(source_path) = &request.source_path {
            paths.insert(source_path.clone());
        }
        if let Some(target_path) = &request.target_path {
            for path in list_tree_paths(client, database_id, target_path).await? {
                paths.insert(path);
            }
        }
    }
    let mut ordered = paths.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    Ok(ordered)
}

async fn list_tree_paths(
    client: &impl VfsApi,
    database_id: &str,
    path: &str,
) -> Result<Vec<String>> {
    let entries = client
        .list_nodes(ListNodesRequest {
            database_id: database_id.to_string(),
            prefix: path.to_string(),
            recursive: true,
        })
        .await?;
    Ok(entries.into_iter().map(|entry| entry.path).collect())
}

async fn execute_delete_plan(
    client: &impl VfsApi,
    database_id: &str,
    report: &mut PurgeReport,
) -> Result<()> {
    for path in report.delete_plan.clone() {
        let Some(node) = client.read_node(database_id, &path).await? else {
            report.skipped_paths.push(path);
            continue;
        };
        match client
            .delete_node(DeleteNodeRequest {
                database_id: database_id.to_string(),
                path: path.clone(),
                expected_etag: Some(node.etag),
                expected_folder_index_etag: None,
            })
            .await
        {
            Ok(result) => report.deleted_paths.push(result.path),
            Err(error) => report.errors.push(error.to_string()),
        }
    }
    Ok(())
}

fn parse_request_node(node: &Node) -> Result<Option<MatchedRequest>> {
    if node.kind != NodeKind::File {
        return Ok(None);
    }
    let Some(frontmatter) = parse_frontmatter(&node.content)? else {
        return Ok(None);
    };
    if frontmatter.kind.as_deref() != Some("kinic.url_ingest_request") {
        return Ok(None);
    }
    let Some(url) = frontmatter.url else {
        return Ok(None);
    };
    Ok(Some(MatchedRequest {
        path: node.path.clone(),
        url,
        source_path: frontmatter.source_path,
        target_path: frontmatter.target_path,
        status: frontmatter.status,
    }))
}

fn parse_frontmatter(content: &str) -> Result<Option<Frontmatter>> {
    let Some(rest) = content.strip_prefix("---\n") else {
        return Ok(None);
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---") else {
        return Ok(None);
    };
    Ok(Some(serde_yaml::from_str(frontmatter)?))
}

fn normalize_url(value: &str) -> Result<String> {
    let mut parsed = reqwest::Url::parse(value).map_err(|error| anyhow!("invalid URL: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(anyhow!("URL must use http or https"));
    }
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn print_report(report: &PurgeReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!("dry_run: {}", report.dry_run);
    for path in &report.delete_plan {
        println!("delete\t{path}");
    }
    for path in &report.deleted_paths {
        println!("deleted\t{path}");
    }
    for path in &report.skipped_paths {
        println!("skipped\t{path}");
    }
    for error in &report.errors {
        println!("error\t{error}");
    }
    Ok(())
}
