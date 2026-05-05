// Where: crates/vfs_cli_app/src/github_ingest.rs
// What: GitHub issue and pull-request evidence ingestion into VFS sources.
// Why: GitHub review context is evidence; skill registry packages remain separate.
use crate::cli::{GitHubCommand, GitHubIngestCommand};
use crate::github_source::{classify_gh_command_failure, ensure_gh_ready};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;
use vfs_client::VfsApi;
use vfs_types::{NodeKind, WriteNodeRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubTarget {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueView {
    number: u64,
    title: String,
    state: String,
    url: String,
    author: Option<GitHubUser>,
    body: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullView {
    number: u64,
    title: String,
    state: String,
    url: String,
    author: Option<GitHubUser>,
    body: Option<String>,
    created_at: String,
    updated_at: String,
    base_ref_name: String,
    head_ref_name: String,
}

pub async fn run_github_command(client: &impl VfsApi, command: GitHubCommand) -> Result<()> {
    ensure_gh_ready().await?;
    match command {
        GitHubCommand::Ingest { command } => match command {
            GitHubIngestCommand::Issue { target, json } => {
                let target = parse_github_target(&target)?;
                let content = fetch_issue_markdown(&target).await?;
                let path = github_evidence_path(&target, "issues");
                write_source_node(client, &path, content).await?;
                print_result(json, &path)?;
            }
            GitHubIngestCommand::Pr { target, json } => {
                let target = parse_github_target(&target)?;
                let content = fetch_pull_markdown(&target).await?;
                let path = github_evidence_path(&target, "pulls");
                write_source_node(client, &path, content).await?;
                print_result(json, &path)?;
            }
        },
    }
    Ok(())
}

pub fn parse_github_target(input: &str) -> Result<GitHubTarget> {
    let (repo_part, number_part) = input
        .split_once('#')
        .ok_or_else(|| anyhow!("GitHub target must use owner/repo#number"))?;
    let mut segments = repo_part.split('/');
    let owner = segments.next().unwrap_or_default();
    let repo = segments.next().unwrap_or_default();
    if segments.next().is_some() || !valid_segment(owner) || !valid_segment(repo) {
        return Err(anyhow!("GitHub target must use owner/repo#number"));
    }
    let number = number_part
        .parse::<u64>()
        .map_err(|_| anyhow!("GitHub target number must be numeric"))?;
    if number == 0 {
        return Err(anyhow!("GitHub target number must be positive"));
    }
    Ok(GitHubTarget {
        owner: owner.to_string(),
        repo: repo.to_string(),
        number,
    })
}

fn github_evidence_path(target: &GitHubTarget, kind: &str) -> String {
    format!(
        "/Sources/github/{}/{}/{}/{}.md",
        target.owner, target.repo, kind, target.number
    )
}

async fn fetch_issue_markdown(target: &GitHubTarget) -> Result<String> {
    let output = gh_view(
        "issue",
        target,
        "number,title,state,url,author,body,createdAt,updatedAt",
    )
    .await?;
    let issue: IssueView = serde_json::from_slice(&output)
        .map_err(|error| anyhow!("gh issue view response invalid: {error}"))?;
    Ok(format_issue_markdown(target, &issue))
}

async fn fetch_pull_markdown(target: &GitHubTarget) -> Result<String> {
    let output = gh_view(
        "pr",
        target,
        "number,title,state,url,author,body,createdAt,updatedAt,baseRefName,headRefName",
    )
    .await?;
    let pull: PullView = serde_json::from_slice(&output)
        .map_err(|error| anyhow!("gh pr view response invalid: {error}"))?;
    Ok(format_pull_markdown(target, &pull))
}

async fn gh_view(kind: &str, target: &GitHubTarget, fields: &str) -> Result<Vec<u8>> {
    let repo = format!("{}/{}", target.owner, target.repo);
    let number = target.number.to_string();
    let output = Command::new("gh")
        .args([kind, "view", &number, "--repo", &repo, "--json", fields])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|error| anyhow!("failed to run gh: {error}"))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(anyhow!(
        "{}",
        classify_gh_command_failure(
            &format!("fetch GitHub {kind}"),
            &format!("{repo}#{number}"),
            &String::from_utf8_lossy(&output.stderr)
        )
    ))
}

async fn write_source_node(client: &impl VfsApi, path: &str, content: String) -> Result<()> {
    client
        .write_node(WriteNodeRequest {
            path: path.to_string(),
            kind: NodeKind::Source,
            content,
            metadata_json: "{}".to_string(),
            expected_etag: None,
        })
        .await?;
    Ok(())
}

fn format_issue_markdown(target: &GitHubTarget, issue: &IssueView) -> String {
    format!(
        "---\nsource: github\nkind: issue\nrepo: {}/{}\nnumber: {}\nurl: {}\nstate: {}\ncreated_at: {}\nupdated_at: {}\n---\n# {} #{}\n\n- Author: {}\n- URL: {}\n\n{}\n",
        target.owner,
        target.repo,
        issue.number,
        issue.url,
        issue.state,
        issue.created_at,
        issue.updated_at,
        issue.title,
        issue.number,
        issue
            .author
            .as_ref()
            .map(|author| author.login.as_str())
            .unwrap_or("unknown"),
        issue.url,
        issue.body.as_deref().unwrap_or("")
    )
}

fn format_pull_markdown(target: &GitHubTarget, pull: &PullView) -> String {
    format!(
        "---\nsource: github\nkind: pull_request\nrepo: {}/{}\nnumber: {}\nurl: {}\nstate: {}\ncreated_at: {}\nupdated_at: {}\nbase_ref: {}\nhead_ref: {}\n---\n# {} #{}\n\n- Author: {}\n- URL: {}\n- Base: {}\n- Head: {}\n\n{}\n",
        target.owner,
        target.repo,
        pull.number,
        pull.url,
        pull.state,
        pull.created_at,
        pull.updated_at,
        pull.base_ref_name,
        pull.head_ref_name,
        pull.title,
        pull.number,
        pull.author
            .as_ref()
            .map(|author| author.login.as_str())
            .unwrap_or("unknown"),
        pull.url,
        pull.base_ref_name,
        pull.head_ref_name,
        pull.body.as_deref().unwrap_or("")
    )
}

fn print_result(json: bool, path: &str) -> Result<()> {
    if json {
        println!("{}", serde_json::json!({ "path": path }));
    } else {
        println!("github evidence imported: {path}");
    }
    Ok(())
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

#[cfg(test)]
mod tests {
    use super::{GitHubTarget, parse_github_target};

    #[test]
    fn parses_github_targets() {
        assert_eq!(
            parse_github_target("owner/repo#123").expect("target should parse"),
            GitHubTarget {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                number: 123,
            }
        );
    }

    #[test]
    fn rejects_invalid_targets() {
        assert!(parse_github_target("owner/repo").is_err());
        assert!(parse_github_target("owner/repo#abc").is_err());
        assert!(parse_github_target("owner/repo#0").is_err());
    }
}
