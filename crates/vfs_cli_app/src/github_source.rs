// Where: crates/vfs_cli_app/src/github_source.rs
// What: GitHub-backed source loading for Skill Registry imports.
// Why: GitHub is an external provenance source; VFS remains the approved registry.
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::io::ErrorKind;
use std::process::Stdio;
use tokio::process::Command;

pub const GITHUB_SHA_LEN: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubSkillSource {
    pub owner: String,
    pub repo: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubSkillPackage {
    pub source: GitHubSkillSource,
    pub resolved_ref: String,
    pub skill: String,
    pub manifest: Option<String>,
    pub provenance: Option<String>,
    pub evals: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommitResponse {
    sha: String,
}

pub fn parse_github_skill_source(input: &str, path: Option<&str>) -> Result<GitHubSkillSource> {
    let (repo_part, inline_path) = input.split_once(':').unwrap_or((input, ""));
    let mut segments = repo_part.split('/');
    let owner = segments
        .next()
        .ok_or_else(|| anyhow!("GitHub source must use owner/repo"))?;
    let repo = segments
        .next()
        .ok_or_else(|| anyhow!("GitHub source must use owner/repo"))?;
    if segments.next().is_some() || !valid_github_segment(owner) || !valid_github_segment(repo) {
        return Err(anyhow!("GitHub source must use owner/repo"));
    }
    let source_path = match (inline_path.is_empty(), path) {
        (false, Some(_)) => return Err(anyhow!("use either owner/repo:path or --path, not both")),
        (false, None) => Some(clean_github_path(inline_path)?),
        (true, Some(path)) => Some(clean_github_path(path)?),
        (true, None) => None,
    };
    Ok(GitHubSkillSource {
        owner: owner.to_string(),
        repo: repo.to_string(),
        path: source_path,
    })
}

pub async fn fetch_github_skill_package(
    source: GitHubSkillSource,
    requested_ref: &str,
) -> Result<GitHubSkillPackage> {
    ensure_gh_ready().await?;
    let resolved_ref = resolve_commit_sha(&source, requested_ref).await?;
    let skill = fetch_required_file(&source, &resolved_ref, "SKILL.md").await?;
    let manifest = fetch_optional_file(&source, &resolved_ref, "manifest.md").await?;
    let provenance = fetch_optional_file(&source, &resolved_ref, "provenance.md").await?;
    let evals = fetch_optional_file(&source, &resolved_ref, "evals.md").await?;
    Ok(GitHubSkillPackage {
        source,
        resolved_ref,
        skill,
        manifest,
        provenance,
        evals,
    })
}

pub async fn fetch_github_optional_package_file(
    source: &GitHubSkillSource,
    sha: &str,
    file: &str,
) -> Result<Option<String>> {
    fetch_optional_file(source, sha, file).await
}

pub async fn ensure_gh_ready() -> Result<()> {
    run_gh_version().await?;
    run_gh_auth_status().await
}

pub fn is_commit_sha(value: &str) -> bool {
    value.len() == GITHUB_SHA_LEN && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn github_source_string(source: &GitHubSkillSource) -> String {
    match &source.path {
        Some(path) => format!("github.com/{}/{}/{}", source.owner, source.repo, path),
        None => format!("github.com/{}/{}", source.owner, source.repo),
    }
}

pub fn github_source_url(source: &GitHubSkillSource, sha: &str) -> String {
    match &source.path {
        Some(path) => format!(
            "https://github.com/{}/{}/tree/{}/{}",
            source.owner, source.repo, sha, path
        ),
        None => format!(
            "https://github.com/{}/{}/tree/{}",
            source.owner, source.repo, sha
        ),
    }
}

pub fn parse_github_provenance_source(value: &str) -> Result<GitHubSkillSource> {
    let rest = value
        .strip_prefix("github.com/")
        .ok_or_else(|| anyhow!("skill provenance source is not GitHub: {value}"))?;
    let mut parts = rest.splitn(3, '/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if !valid_github_segment(owner) || !valid_github_segment(repo) {
        return Err(anyhow!(
            "GitHub provenance source must use github.com/owner/repo"
        ));
    }
    let path = parts
        .next()
        .filter(|path| !path.is_empty())
        .map(clean_github_path)
        .transpose()?;
    Ok(GitHubSkillSource {
        owner: owner.to_string(),
        repo: repo.to_string(),
        path,
    })
}

async fn resolve_commit_sha(source: &GitHubSkillSource, requested_ref: &str) -> Result<String> {
    let endpoint = format!(
        "repos/{}/{}/commits/{}",
        source.owner, source.repo, requested_ref
    );
    let output = gh_api_json(&endpoint, "resolve GitHub ref").await?;
    let response: CommitResponse = serde_json::from_slice(&output)
        .map_err(|error| anyhow!("GitHub commit response invalid: {error}"))?;
    if !is_commit_sha(&response.sha) {
        return Err(anyhow!("GitHub commit response returned invalid sha"));
    }
    Ok(response.sha)
}

async fn fetch_required_file(source: &GitHubSkillSource, sha: &str, file: &str) -> Result<String> {
    fetch_optional_file(source, sha, file)
        .await?
        .ok_or_else(|| anyhow!("{file} missing in GitHub source"))
}

async fn fetch_optional_file(
    source: &GitHubSkillSource,
    sha: &str,
    file: &str,
) -> Result<Option<String>> {
    let path = github_file_path(source, file);
    let endpoint = format!(
        "repos/{}/{}/contents/{}?ref={sha}",
        source.owner, source.repo, path
    );
    let output = Command::new("gh")
        .args(["api", &endpoint, "-H", "Accept: application/vnd.github.raw"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(gh_spawn_error)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Not Found") || stderr.contains("HTTP 404") {
            return Ok(None);
        }
        return Err(anyhow!(
            "{}",
            classify_gh_command_failure("fetch GitHub file", &path, &stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map(Some)
        .map_err(|error| anyhow!("GitHub file is not UTF-8: {error}"))
}

async fn gh_api_json(endpoint: &str, action: &str) -> Result<Vec<u8>> {
    let output = Command::new("gh")
        .args(["api", endpoint])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(gh_spawn_error)?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(anyhow!(
        "{}",
        classify_gh_command_failure(action, endpoint, &String::from_utf8_lossy(&output.stderr))
    ))
}

async fn run_gh_version() -> Result<()> {
    let output = Command::new("gh")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(gh_spawn_error)?;
    if output.status.success() {
        return Ok(());
    }
    Err(anyhow!(
        "GitHub CLI check failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

async fn run_gh_auth_status() -> Result<()> {
    let output = Command::new("gh")
        .args(["auth", "status", "-h", "github.com"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(gh_spawn_error)?;
    if output.status.success() {
        return Ok(());
    }
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Err(anyhow!("{}", classify_gh_auth_failure(&combined)))
}

fn gh_spawn_error(error: std::io::Error) -> anyhow::Error {
    if error.kind() == ErrorKind::NotFound {
        return anyhow!("GitHub CLI `gh` is not installed or is not on PATH");
    }
    anyhow!("failed to run gh: {error}")
}

pub fn classify_gh_auth_failure(output: &str) -> String {
    let lower = output.to_ascii_lowercase();
    if lower.contains("token") && (lower.contains("invalid") || lower.contains("expired")) {
        return "GitHub CLI authentication is invalid; run `gh auth login -h github.com`"
            .to_string();
    }
    if lower.contains("not logged") || lower.contains("no github hosts") {
        return "GitHub CLI is not authenticated; run `gh auth login -h github.com`".to_string();
    }
    if lower.contains("insufficient") || lower.contains("permission") || lower.contains("scope") {
        return "GitHub CLI token lacks required permissions for this repository".to_string();
    }
    format!(
        "GitHub CLI authentication check failed; run `gh auth login -h github.com`: {}",
        output.trim()
    )
}

pub fn classify_gh_command_failure(action: &str, target: &str, stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("http 404") || lower.contains("not found") {
        return format!("GitHub {action} failed for {target}: repository, ref, or path not found");
    }
    if lower.contains("http 403") || lower.contains("forbidden") || lower.contains("permission") {
        return format!("GitHub {action} failed for {target}: permission denied");
    }
    if lower.contains("bad credentials") || lower.contains("token") {
        return format!(
            "GitHub {action} failed for {target}: authentication failed; run `gh auth login -h github.com`"
        );
    }
    format!("GitHub {action} failed for {target}: {}", stderr.trim())
}

fn github_file_path(source: &GitHubSkillSource, file: &str) -> String {
    match &source.path {
        Some(path) => format!("{path}/{file}"),
        None => file.to_string(),
    }
}

fn clean_github_path(path: &str) -> Result<String> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty()
        || trimmed
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(anyhow!("GitHub path must be a relative repository path"));
    }
    Ok(trimmed.to_string())
}

fn valid_github_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

#[cfg(test)]
mod tests {
    use super::{
        GitHubSkillSource, classify_gh_auth_failure, classify_gh_command_failure, gh_spawn_error,
        github_source_string, github_source_url, is_commit_sha, parse_github_provenance_source,
        parse_github_skill_source,
    };
    use std::io::ErrorKind;

    #[test]
    fn parses_repo_and_optional_paths() {
        assert_eq!(
            parse_github_skill_source("owner/repo", None).expect("source should parse"),
            GitHubSkillSource {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                path: None,
            }
        );
        assert_eq!(
            parse_github_skill_source("owner/repo:skills/foo", None)
                .expect("inline path should parse")
                .path
                .as_deref(),
            Some("skills/foo")
        );
        assert_eq!(
            parse_github_skill_source("owner/repo", Some("/skills/foo/"))
                .expect("path flag should parse")
                .path
                .as_deref(),
            Some("skills/foo")
        );
    }

    #[test]
    fn rejects_ambiguous_or_invalid_sources() {
        assert!(parse_github_skill_source("owner/repo:path", Some("other")).is_err());
        assert!(parse_github_skill_source("owner", None).is_err());
        assert!(parse_github_skill_source("owner/repo", Some("../secret")).is_err());
    }

    #[test]
    fn detects_commit_sha() {
        assert!(is_commit_sha("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_commit_sha("main"));
        assert!(!is_commit_sha("012345"));
    }

    #[test]
    fn formats_and_parses_provenance() {
        let source = GitHubSkillSource {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            path: Some("skills/foo".to_string()),
        };
        assert_eq!(
            github_source_string(&source),
            "github.com/owner/repo/skills/foo"
        );
        assert_eq!(
            github_source_url(&source, "abc"),
            "https://github.com/owner/repo/tree/abc/skills/foo"
        );
        assert_eq!(
            parse_github_provenance_source("github.com/owner/repo/skills/foo")
                .expect("provenance should parse"),
            source
        );
    }

    #[test]
    fn classifies_gh_auth_failures() {
        assert!(
            classify_gh_auth_failure("The token in default is invalid.")
                .contains("authentication is invalid")
        );
        assert!(
            classify_gh_auth_failure("You are not logged into any GitHub hosts")
                .contains("not authenticated")
        );
    }

    #[test]
    fn classifies_gh_command_failures() {
        assert!(
            classify_gh_command_failure("resolve GitHub ref", "repos/o/r/commits/main", "HTTP 404")
                .contains("not found")
        );
        assert!(
            classify_gh_command_failure("fetch GitHub file", "SKILL.md", "HTTP 403 Forbidden")
                .contains("permission denied")
        );
        assert!(
            classify_gh_command_failure("fetch GitHub file", "SKILL.md", "Bad credentials")
                .contains("authentication failed")
        );
    }

    #[test]
    fn classifies_missing_gh_binary() {
        let error = gh_spawn_error(std::io::Error::new(ErrorKind::NotFound, "missing"));
        assert!(error.to_string().contains("not installed"));
    }
}
