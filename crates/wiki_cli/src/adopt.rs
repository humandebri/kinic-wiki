// Where: crates/wiki_cli/src/adopt.rs
// What: Draft adoption flow that promotes local review pages into managed mirror pages.
// Why: New page adoption is a separate concern from generic pull/push command dispatch.
use crate::client::WikiApi;
use crate::mirror::{
    MirrorState, adopt_local_draft, is_managed_mirror_content, now_millis, parse_draft_metadata,
    save_state, strip_frontmatter, write_rendered_system_pages,
};
use anyhow::{Result, anyhow};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use wiki_types::{AdoptDraftPageInput, WikiPageType};

#[derive(Debug, Serialize)]
pub struct AdoptDraftResponse {
    pub page_id: String,
    pub slug: String,
    pub revision_id: String,
    pub updated_at: i64,
    pub path: PathBuf,
    pub action: String,
}

pub async fn adopt_draft(
    client: &impl WikiApi,
    mirror_root: &Path,
    slug: &str,
    page_type_override: Option<WikiPageType>,
) -> Result<AdoptDraftResponse> {
    let path = mirror_root.join("pages").join(format!("{slug}.md"));
    let markdown = fs::read_to_string(&path)
        .map_err(|error| anyhow!("failed to read {}: {error}", path.display()))?;
    if is_managed_mirror_content(&markdown) {
        return Err(anyhow!("draft is already managed: {}", path.display()));
    }

    let draft_metadata = parse_draft_metadata(&markdown);
    let page_type = page_type_override
        .or_else(|| {
            draft_metadata
                .as_ref()
                .and_then(|metadata| WikiPageType::from_str(&metadata.page_type))
        })
        .ok_or_else(|| anyhow!("page type is missing; rerun generate-draft or pass --page-type"))?;
    let page_type_name = page_type.as_str().to_string();
    let title = draft_metadata
        .as_ref()
        .map(|metadata| metadata.title.clone())
        .or_else(|| extract_title(&markdown))
        .unwrap_or_else(|| titleize_slug(slug));
    let markdown = strip_frontmatter(&markdown).trim_start().to_string();
    let adopted = client
        .adopt_draft_page(AdoptDraftPageInput {
            slug: slug.to_string(),
            title,
            page_type,
            markdown,
        })
        .await?;
    let written_path = adopt_local_draft(mirror_root, slug, &page_type_name, &adopted)?;
    write_rendered_system_pages(mirror_root, &adopted.index_markdown, &adopted.log_markdown)?;
    save_state(
        mirror_root,
        &MirrorState {
            snapshot_revision: adopted.snapshot_revision.clone(),
            last_synced_at: now_millis(),
        },
    )?;
    Ok(AdoptDraftResponse {
        page_id: adopted.page_id,
        slug: adopted.slug,
        revision_id: adopted.revision_id,
        updated_at: adopted.updated_at,
        path: written_path,
        action: "adopted".to_string(),
    })
}

fn extract_title(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.strip_prefix("# ")
            .map(|value| value.trim().to_string())
    })
}

fn titleize_slug(input: &str) -> String {
    input
        .split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            match characters.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
