// Where: crates/wiki_store/src/projection.rs
// What: Search projection builders for wiki sections and page index summaries.
// Why: commit flow should delegate projection shape details to a dedicated module.
use std::collections::HashMap;

use wiki_types::{SearchDocKind, SearchProjectionDoc, WikiPage};

use crate::markdown::ParsedSection;

pub fn build_projection_changes(
    page: &WikiPage,
    revision_id: &str,
    new_sections: &[ParsedSection],
    old_by_path: &HashMap<String, String>,
    updated_at: i64,
) -> (Vec<SearchProjectionDoc>, Vec<String>, u32) {
    let mut docs = Vec::new();
    let mut deleted = old_by_path.keys().cloned().collect::<Vec<_>>();
    let mut unchanged = 0_u32;
    for section in new_sections {
        if old_by_path.get(&section.section_path) == Some(&section.content_hash) {
            unchanged += 1;
        } else {
            docs.push(SearchProjectionDoc {
                external_id: format!("page:{}:section:{}", page.id, section.section_path),
                kind: SearchDocKind::WikiSection,
                page_id: Some(page.id.clone()),
                revision_id: Some(revision_id.to_string()),
                section_path: Some(section.section_path.clone()),
                title: page.title.clone(),
                snippet: section.heading.clone().unwrap_or_else(|| page.title.clone()),
                citation: format!("wiki://{}#{}", page.slug, section.section_path),
                content: section.text.clone(),
                section: Some(section.section_path.clone()),
                tags: vec![page.page_type.as_str().to_string()],
                updated_at,
            });
        }
        deleted.retain(|path| path != &section.section_path);
    }
    docs.push(SearchProjectionDoc {
        external_id: format!("page:{}:index", page.id),
        kind: SearchDocKind::IndexPage,
        page_id: Some(page.id.clone()),
        revision_id: Some(revision_id.to_string()),
        section_path: None,
        title: page.title.clone(),
        snippet: page.summary_1line.clone().unwrap_or_else(|| page.title.clone()),
        citation: format!("wiki://{}", page.slug),
        content: page.summary_1line.clone().unwrap_or_else(|| page.title.clone()),
        section: None,
        tags: vec![page.page_type.as_str().to_string()],
        updated_at,
    });
    (
        docs,
        deleted
            .into_iter()
            .map(|path| format!("page:{}:section:{}", page.id, path))
            .collect(),
        unchanged,
    )
}
