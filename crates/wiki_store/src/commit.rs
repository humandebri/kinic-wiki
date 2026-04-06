// Where: crates/wiki_store/src/commit.rs
// What: Revision commit flow, section diffing, projection refresh, and system page updates.
// Why: The wiki's compounding behavior depends on deterministic revision storage plus search projection maintenance.
use rusqlite::{Connection, params};
use uuid::Uuid;
use wiki_search::WikiSearch;
use wiki_types::{
    CommitPageRevisionInput, CommitPageRevisionOutput, RevisionCitationInput, SystemPage,
};

use crate::{
    markdown::{ParsedSection, split_markdown},
    projection::build_projection_changes,
    render,
    store::{WikiStore, load_page_by_id},
    system_pages::{
        refresh_system_pages_tx, render_index_page_now, render_log_page_now, system_pages_to_docs,
    },
};

impl WikiStore {
    pub fn commit_page_revision(&self, input: CommitPageRevisionInput) -> Result<CommitPageRevisionOutput, String> {
        let mut conn = self.open()?;
        commit_revision_tx(&mut conn, &input)
    }

    pub fn render_index_page(&self, updated_at: i64) -> Result<SystemPage, String> {
        let conn = self.open()?;
        render_index_page_now(&conn, updated_at)
    }

    pub fn render_log_page(&self, limit: usize, updated_at: i64) -> Result<SystemPage, String> {
        let conn = self.open()?;
        render_log_page_now(&conn, limit, updated_at)
    }

    pub fn refresh_system_pages(&self, updated_at: i64) -> Result<Vec<SystemPage>, String> {
        let mut conn = self.open()?;
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        let pages = refresh_system_pages_tx(&tx, updated_at)?;
        let projection_docs = system_pages_to_docs(&pages);
        if !projection_docs.is_empty() {
            WikiSearch::upsert_docs_in_tx(&tx, &projection_docs)?;
        }
        tx.commit().map_err(|error| error.to_string())?;
        Ok(pages)
    }
}

fn commit_revision_tx(
    conn: &mut Connection,
    input: &CommitPageRevisionInput,
) -> Result<CommitPageRevisionOutput, String> {
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let mut page = load_page_by_id(&tx, &input.page_id)?.ok_or_else(|| "page does not exist".to_string())?;
    if input.markdown.trim().is_empty() {
        return Err("markdown must not be empty".to_string());
    }
    if page.current_revision_id != input.expected_current_revision_id {
        return Err("expected_current_revision_id does not match current revision".to_string());
    }

    let revision_no = next_revision_no(&tx, &page.id)?;
    let revision_id = format!("revision_{}", Uuid::new_v4());
    let new_sections = split_markdown(&input.markdown)?;
    if new_sections.is_empty() {
        return Err("section split produced no sections".to_string());
    }
    let old_sections = load_current_section_rows(&tx, &page.id)?;
    let old_by_path = old_sections
        .iter()
        .map(|section| (section.section_path.clone(), section.content_hash.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    tx.execute(
        "INSERT INTO wiki_revisions (id, page_id, revision_no, markdown, change_reason, author_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            revision_id,
            page.id,
            revision_no,
            input.markdown,
            input.change_reason,
            input.author_type,
            input.updated_at,
        ],
    )
    .map_err(|error| error.to_string())?;

    tx.execute(
        "UPDATE wiki_sections SET is_current = 0 WHERE page_id = ?1 AND is_current = 1",
        params![page.id],
    )
    .map_err(|error| error.to_string())?;
    store_sections(&tx, &page.id, &revision_id, &new_sections)?;
    replace_citations(&tx, &revision_id, &input.citations)?;

    page.current_revision_id = Some(revision_id.clone());
    page.title = input.title.clone();
    page.summary_1line = Some(render::summary_from_title(&input.title, &page.page_type));
    page.updated_at = input.updated_at;
    tx.execute(
        "UPDATE wiki_pages
         SET title = ?1, current_revision_id = ?2, summary_1line = ?3, updated_at = ?4
         WHERE id = ?5",
        params![page.title, revision_id, page.summary_1line, page.updated_at, page.id],
    )
    .map_err(|error| error.to_string())?;

    append_log_event(&tx, &page.id, revision_no, &input.title, input.updated_at)?;
    let (projection_docs, deleted_ids, unchanged_count) =
        build_projection_changes(&page, &revision_id, &new_sections, &old_by_path, input.updated_at);
    let system_pages = refresh_system_pages_tx(&tx, input.updated_at)?;
    if !projection_docs.is_empty() {
        WikiSearch::upsert_docs_in_tx(&tx, &projection_docs)?;
    }
    if !deleted_ids.is_empty() {
        WikiSearch::delete_docs_by_external_ids_in_tx(&tx, &deleted_ids)?;
    }
    let system_projection_docs = system_pages_to_docs(&system_pages);
    if !system_projection_docs.is_empty() {
        WikiSearch::upsert_docs_in_tx(&tx, &system_projection_docs)?;
    }
    tx.commit().map_err(|error| error.to_string())?;

    let mut upserted_ids = projection_docs
        .iter()
        .map(|doc| doc.external_id.clone())
        .collect::<Vec<_>>();
    upserted_ids.push(format!("page:{}:index", page.id));
    Ok(CommitPageRevisionOutput {
        revision_id,
        revision_no,
        section_count: new_sections.len() as u32,
        unchanged_section_count: unchanged_count,
        upserted_projection_ids: upserted_ids,
        deleted_projection_ids: deleted_ids,
        rendered_system_pages: system_pages.iter().map(|page| page.slug.clone()).collect(),
    })
}

#[derive(Clone, Debug)]
struct StoredSection {
    section_path: String,
    content_hash: String,
}

fn load_current_section_rows(conn: &Connection, page_id: &str) -> Result<Vec<StoredSection>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT section_path, content_hash
             FROM wiki_sections WHERE page_id = ?1 AND is_current = 1
             ORDER BY ordinal",
        )
        .map_err(|error| error.to_string())?;
    stmt.query_map(params![page_id], |row| {
        Ok(StoredSection {
            section_path: row.get(0)?,
            content_hash: row.get(1)?,
        })
    })
    .map_err(|error| error.to_string())?
    .collect::<std::result::Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())
}

fn next_revision_no(conn: &Connection, page_id: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(revision_no), 0) + 1 FROM wiki_revisions WHERE page_id = ?1",
        params![page_id],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn store_sections(
    conn: &Connection,
    page_id: &str,
    revision_id: &str,
    sections: &[ParsedSection],
) -> Result<(), String> {
    for section in sections {
        conn.execute(
            "INSERT INTO wiki_sections (
                id, page_id, revision_id, section_path, ordinal, heading, text, content_hash, is_current
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
            params![
                format!("section_{}", Uuid::new_v4()),
                page_id,
                revision_id,
                section.section_path,
                section.ordinal,
                section.heading,
                section.text,
                section.content_hash,
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn replace_citations(conn: &Connection, revision_id: &str, citations: &[RevisionCitationInput]) -> Result<(), String> {
    conn.execute(
        "DELETE FROM revision_citations WHERE revision_id = ?1",
        params![revision_id],
    )
    .map_err(|error| error.to_string())?;
    for citation in citations {
        conn.execute(
            "INSERT INTO revision_citations (id, revision_id, source_id, chunk_id, evidence_kind, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                format!("citation_{}", Uuid::new_v4()),
                revision_id,
                citation.source_id,
                citation.chunk_id,
                citation.evidence_kind,
                citation.note,
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn append_log_event(conn: &Connection, page_id: &str, revision_no: i64, title: &str, created_at: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO log_events (id, event_type, title, body_markdown, related_page_id, created_at)
         VALUES (?1, 'commit_page_revision', ?2, ?3, ?4, ?5)",
        params![
            format!("log_{}", Uuid::new_v4()),
            title,
            format!("Committed revision {revision_no}"),
            page_id,
            created_at,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
