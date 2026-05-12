// Where: crates/vfs_cli_app/src/aeo_generate/output.rs
// What: Build and write frontend AEO dry-run artifacts.
// Why: Generated files need a stable shape for future publish automation.

use crate::aeo_generate::types::{
    GeneratedOutput, GeneratedOutputKind, Manifest, ManifestAnswer, ManifestWikiPage, SourceEntry,
};
use anyhow::{Result, anyhow};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn build_outputs(
    project_name: &str,
    project_slug: &str,
    sources: &[SourceEntry],
) -> Vec<GeneratedOutput> {
    let source_paths = sources
        .iter()
        .map(|source| source.path.clone())
        .collect::<Vec<_>>();
    let wiki_pages = [
        ("wiki/overview.md", "Product overview"),
        ("wiki/screens.md", "Visible screens"),
        ("wiki/features.md", "Visible features"),
        ("wiki/faq.md", "Product FAQ"),
    ];
    let answers = [
        (
            format!("answers/what-is-{project_slug}.md"),
            format!("what-is-{project_slug}"),
            format!("What is {project_name}?"),
        ),
        (
            format!("answers/how-does-{project_slug}-work.md"),
            format!("how-does-{project_slug}-work"),
            format!("How does {project_name} work?"),
        ),
        (
            format!("answers/{project_slug}-features.md"),
            format!("{project_slug}-features"),
            format!("{project_name} features"),
        ),
    ];
    wiki_pages
        .into_iter()
        .map(|(path, title)| GeneratedOutput {
            kind: GeneratedOutputKind::Wiki,
            path: path.to_string(),
            slug: None,
            title: title.to_string(),
            sources: source_paths.clone(),
        })
        .chain(
            answers
                .into_iter()
                .map(|(path, slug, title)| GeneratedOutput {
                    kind: GeneratedOutputKind::Answer,
                    path,
                    slug: Some(slug),
                    title,
                    sources: source_paths.clone(),
                }),
        )
        .collect()
}

pub fn write_outputs(
    out: &Path,
    project_name: &str,
    project_slug: &str,
    outputs: &[GeneratedOutput],
) -> Result<()> {
    let duplicate_slugs = duplicate_slugs(outputs);
    if !duplicate_slugs.is_empty() {
        return Err(anyhow!("duplicate generated slugs: {:?}", duplicate_slugs));
    }
    for output in outputs {
        let path = out.join(&output.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = match output.kind {
            GeneratedOutputKind::Wiki => wiki_markdown(project_name, output),
            GeneratedOutputKind::Answer => answer_markdown(project_name, project_slug, output),
        };
        fs::write(path, content)?;
    }
    Ok(())
}

pub fn build_manifest(
    project_name: &str,
    project_slug: &str,
    outputs: &[GeneratedOutput],
) -> Manifest {
    Manifest {
        project_name: project_name.to_string(),
        project_slug: project_slug.to_string(),
        framework: "nextjs_app_router".to_string(),
        answers: outputs
            .iter()
            .filter(|output| output.kind == GeneratedOutputKind::Answer)
            .map(|output| ManifestAnswer {
                slug: output.slug.clone().expect("answer output should have slug"),
                title: output.title.clone(),
                path: output.path.clone(),
                sources: output.sources.clone(),
            })
            .collect(),
        wiki_pages: outputs
            .iter()
            .filter(|output| output.kind == GeneratedOutputKind::Wiki)
            .map(|output| ManifestWikiPage {
                title: output.title.clone(),
                path: output.path.clone(),
                sources: output.sources.clone(),
            })
            .collect(),
    }
}

pub fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn duplicate_slugs(outputs: &[GeneratedOutput]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for slug in outputs.iter().filter_map(|output| output.slug.as_ref()) {
        if !seen.insert(slug.clone()) {
            duplicates.insert(slug.clone());
        }
    }
    duplicates.into_iter().collect()
}

fn wiki_markdown(project_name: &str, output: &GeneratedOutput) -> String {
    format!(
        "# {}\n\n{} is described from user-visible frontend sources.\n\n## Sources\n{}\n",
        output.title,
        project_name,
        markdown_source_list(&output.sources)
    )
}

fn answer_markdown(project_name: &str, project_slug: &str, output: &GeneratedOutput) -> String {
    let slug = output
        .slug
        .as_deref()
        .expect("answer output should have slug");
    let subject = if slug == format!("what-is-{project_slug}") {
        project_name.to_string()
    } else {
        format!("{project_name} ({slug})")
    };
    format!(
        "---\ntitle: {}\ndescription: {} answer page generated from visible frontend sources.\nanswer_summary: {} is summarized from visible frontend sources.\nupdated: 2026-05-07\nindex: true\nentities:\n  - {}\nsources:\n{}\n---\n\n# {}\n\n{} is represented here using only user-visible frontend sources.\n\n## Sources\n{}\n",
        output.title,
        project_name,
        project_name,
        project_name,
        frontmatter_source_list(&output.sources),
        output.title,
        subject,
        markdown_source_list(&output.sources)
    )
}

fn frontmatter_source_list(sources: &[String]) -> String {
    sources
        .iter()
        .map(|source| format!("  - {source}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_source_list(sources: &[String]) -> String {
    if sources.is_empty() {
        return "- No source paths found.\n".to_string();
    }
    sources
        .iter()
        .map(|source| format!("- `{source}`"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
