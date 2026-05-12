// Where: crates/vfs_cli_app/src/aeo_generate/collect.rs
// What: Collect and validate user-visible frontend source paths.
// Why: AEO generation must ignore implementation-only repository content.

use crate::aeo_generate::types::{SourceEntry, SourceKind, ValidationReport};
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const MAX_SOURCE_BYTES: u64 = 512 * 1024;

pub fn collect_frontend_sources(repo: &Path) -> Result<Vec<SourceEntry>> {
    let mut sources = BTreeMap::<String, SourceKind>::new();
    let readme = repo.join("README.md");
    if readme.is_file() {
        sources.insert("README.md".to_string(), SourceKind::Readme);
    }
    collect_docs(repo, repo, &mut sources)?;
    collect_next_app(repo, repo, &mut sources)?;
    Ok(sources
        .into_iter()
        .map(|(path, kind)| SourceEntry { path, kind })
        .collect())
}

pub fn validate_source_pack(sources: &[SourceEntry], repo: &Path) -> ValidationReport {
    let mut report = ValidationReport {
        passed: true,
        errors: Vec::new(),
        warnings: Vec::new(),
    };
    if !sources
        .iter()
        .any(|source| source.kind == SourceKind::NextAppPage)
    {
        report
            .errors
            .push("missing Next.js App Router page source".to_string());
    }
    if sources.is_empty() {
        report
            .errors
            .push("no frontend AEO sources found".to_string());
    }
    for source in sources {
        let path = repo.join(&source.path);
        if let Ok(content) = fs::read_to_string(path)
            && contains_secret_pattern(&content)
        {
            report
                .errors
                .push(format!("secret-like pattern found in {}", source.path));
        }
    }
    report.passed = report.errors.is_empty();
    report
}

fn collect_docs(
    repo: &Path,
    root: &Path,
    sources: &mut BTreeMap<String, SourceKind>,
) -> Result<()> {
    let docs = repo.join("docs");
    if !docs.is_dir() {
        return Ok(());
    }
    collect_files(
        &docs,
        root,
        sources,
        |relative| relative.ends_with(".md") || relative.ends_with(".mdx"),
        SourceKind::PublicDoc,
    )
}

fn collect_next_app(
    repo: &Path,
    root: &Path,
    sources: &mut BTreeMap<String, SourceKind>,
) -> Result<()> {
    let app = repo.join("app");
    if !app.is_dir() {
        return Ok(());
    }
    collect_files(
        &app,
        root,
        sources,
        |relative| {
            (relative.ends_with("/page.tsx") || relative == "app/page.tsx")
                || (relative.ends_with("/layout.tsx") || relative == "app/layout.tsx")
        },
        SourceKind::NextAppPage,
    )?;
    for (path, kind) in sources.iter_mut() {
        if path.ends_with("/layout.tsx") || path == "app/layout.tsx" {
            *kind = SourceKind::NextAppLayout;
        }
    }
    Ok(())
}

fn collect_files(
    dir: &Path,
    root: &Path,
    sources: &mut BTreeMap<String, SourceKind>,
    include: impl Fn(&str) -> bool + Copy,
    kind: SourceKind,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if should_skip_name(&file_name) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, root, sources, include, kind)?;
            continue;
        }
        if !path.is_file() || path.metadata()?.len() > MAX_SOURCE_BYTES {
            continue;
        }
        let relative = repo_relative(root, &path)?;
        if include(&relative) && !is_hidden_admin_surface(&relative) {
            sources.insert(relative, kind);
        }
    }
    Ok(())
}

fn should_skip_name(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules"
                | "target"
                | ".next"
                | "dist"
                | "build"
                | "coverage"
                | "__tests__"
                | "tests"
                | "fixtures"
        )
}

fn is_hidden_admin_surface(relative: &str) -> bool {
    relative
        .split('/')
        .any(|segment| segment == "admin" || segment == "(admin)" || segment.starts_with("_"))
}

fn repo_relative(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn contains_secret_pattern(content: &str) -> bool {
    content.contains("-----BEGIN PRIVATE KEY-----")
        || content.contains("sk-")
        || content.contains("AKIA")
        || content.contains("SECRET_KEY=")
        || content.contains("NEXT_PUBLIC_") && content.contains("PRIVATE")
}
