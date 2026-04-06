// Where: crates/wiki_store/src/health.rs
// What: Health and lint checks over the current wiki state.
// Why: The wiki needs a first-party way to surface pages that need maintenance work.
use rusqlite::{Connection, params};
use wiki_types::{HealthCheckReport, HealthIssue, HealthIssueKind};

pub(crate) fn run_health_checks(conn: &Connection) -> Result<HealthCheckReport, String> {
    let mut issues = orphan_page_issues(conn)?;
    issues.extend(unsupported_claim_issues(conn)?);
    issues.extend(explicit_marker_issues(
        conn,
        "contradiction",
        HealthIssueKind::Contradiction,
    )?);
    issues.extend(explicit_marker_issues(
        conn,
        "stale",
        HealthIssueKind::StaleClaim,
    )?);
    Ok(HealthCheckReport { issues })
}

fn orphan_page_issues(conn: &Connection) -> Result<Vec<HealthIssue>, String> {
    let pages = load_current_pages(conn)?;
    let mut issues = Vec::new();
    for (page_id, slug, title, _markdown) in &pages {
        let link_count = pages
            .iter()
            .filter(|(other_page_id, _, _, other_markdown)| {
                other_page_id != page_id && other_markdown.contains(&format!("[[{slug}]]"))
            })
            .count();
        if link_count == 0 {
            issues.push(HealthIssue {
                kind: HealthIssueKind::OrphanPage,
                page_slug: Some(slug.clone()),
                section_path: None,
                message: format!("{title} has no inbound wiki links."),
            });
        }
    }
    Ok(issues)
}

fn unsupported_claim_issues(conn: &Connection) -> Result<Vec<HealthIssue>, String> {
    let pages = load_current_pages(conn)?;
    Ok(pages
        .into_iter()
        .filter(|(_, _, _, markdown)| !markdown.contains("[source:"))
        .map(|(_, slug, title, _)| HealthIssue {
            kind: HealthIssueKind::UnsupportedClaim,
            page_slug: Some(slug),
            section_path: None,
            message: format!("{title} has no visible source markers."),
        })
        .collect())
}

fn explicit_marker_issues(
    conn: &Connection,
    marker: &str,
    kind: HealthIssueKind,
) -> Result<Vec<HealthIssue>, String> {
    let pages = load_current_pages(conn)?;
    let marker_text = marker.to_lowercase();
    Ok(pages
        .into_iter()
        .filter(|(_, _, _, markdown)| markdown.to_lowercase().contains(&marker_text))
        .map(|(_, slug, title, _)| HealthIssue {
            kind: kind.clone(),
            page_slug: Some(slug),
            section_path: None,
            message: format!("{title} contains the marker '{marker}'."),
        })
        .collect())
}

fn load_current_pages(conn: &Connection) -> Result<Vec<(String, String, String, String)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.slug, p.title, r.markdown
             FROM wiki_pages p
             JOIN wiki_revisions r ON r.id = p.current_revision_id
             ORDER BY p.slug",
        )
        .map_err(|error| error.to_string())?;
    stmt.query_map(params![], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })
    .map_err(|error| error.to_string())?
    .collect::<std::result::Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())
}
