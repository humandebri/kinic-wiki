// Where: crates/wiki_store/src/fs_helpers.rs
// What: Shared FS-first helpers for path validation, row loading, and snapshot hashing.
// Why: FsStore behavior must stay deterministic across CRUD, list, search, and sync flows.
use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use wiki_types::{Node, NodeEntry, NodeEntryKind, NodeKind, NodeMutationAck};

use crate::hashing::sha256_hex;

pub(crate) fn normalize_node_path(path: &str, allow_root: bool) -> Result<String, String> {
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if !path.starts_with('/') {
        return Err(format!("path must start with '/': {path}"));
    }
    if path.contains("//") {
        return Err(format!("path must not contain '//': {path}"));
    }
    if path.len() > 1 && path.ends_with('/') {
        return Err(format!("path must not end with '/': {path}"));
    }
    if path == "/" {
        return if allow_root {
            Ok("/".to_string())
        } else {
            Err("root path is not allowed".to_string())
        };
    }
    for segment in path.split('/').skip(1) {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(format!("path contains invalid segment: {path}"));
        }
    }
    Ok(path.to_string())
}

pub(crate) fn compute_node_etag(node: &Node) -> String {
    let deleted = node
        .deleted_at
        .map_or_else(|| "null".to_string(), |value| value.to_string());
    let payload = format!(
        "{}\n{}\n{}\n{}\n{}",
        node.path,
        node_kind_tag(&node.kind),
        node.content,
        node.metadata_json,
        deleted
    );
    format!("v4h:{}", sha256_hex(&payload))
}

pub(crate) fn node_ack(node: &Node) -> NodeMutationAck {
    NodeMutationAck {
        path: node.path.clone(),
        kind: node.kind.clone(),
        updated_at: node.updated_at,
        etag: node.etag.clone(),
        deleted_at: node.deleted_at,
    }
}

pub(crate) struct StoredNode {
    pub(crate) row_id: i64,
    pub(crate) node: Node,
}

fn node_kind_tag(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "file",
        NodeKind::Source => "source",
    }
}

pub(crate) fn load_node(conn: &Connection, path: &str) -> Result<Option<Node>, String> {
    Ok(load_stored_node(conn, path)?.map(|stored| stored.node))
}

pub(crate) fn load_stored_node(
    conn: &Connection,
    path: &str,
) -> Result<Option<StoredNode>, String> {
    conn.query_row(
        "SELECT id, path, kind, content, created_at, updated_at, etag, deleted_at, metadata_json
         FROM fs_nodes WHERE path = ?1",
        params![path],
        map_stored_node,
    )
    .optional()
    .map_err(|error| error.to_string())
}

pub(crate) fn load_scoped_nodes(
    conn: &Connection,
    prefix: &str,
    include_deleted: bool,
) -> Result<Vec<Node>, String> {
    let mut sql = String::from(
        "SELECT path, kind, content, created_at, updated_at, etag, deleted_at, metadata_json
         FROM fs_nodes WHERE 1 = 1",
    );
    let mut values = Vec::new();
    if !include_deleted {
        sql.push_str(" AND deleted_at IS NULL");
    }
    if prefix != "/" {
        let (scope_sql, scope_values) = prefix_filter_sql(prefix, values.len() + 1);
        sql.push_str(&scope_sql);
        values.extend(scope_values);
    }
    sql.push_str(" ORDER BY path ASC");
    let mut stmt = conn.prepare(&sql).map_err(|error| error.to_string())?;
    stmt.query_map(rusqlite::params_from_iter(values.iter()), map_node)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(crate) fn build_entries(nodes: &[Node], prefix: &str, recursive: bool) -> Vec<NodeEntry> {
    if recursive {
        return nodes
            .iter()
            .map(|node| NodeEntry {
                path: node.path.clone(),
                kind: entry_kind_from_node_kind(&node.kind),
                updated_at: node.updated_at,
                etag: node.etag.clone(),
                deleted_at: node.deleted_at,
                has_children: has_visible_descendants(nodes, &node.path),
            })
            .collect();
    }

    let mut entries = BTreeMap::new();
    for node in nodes {
        if let Some(child_path) = direct_child_path(prefix, &node.path) {
            if child_path != node.path && node.deleted_at.is_some() {
                continue;
            }
            entries
                .entry(child_path.clone())
                .and_modify(|entry: &mut NodeEntry| {
                    if child_path == node.path {
                        entry.kind = entry_kind_from_node_kind(&node.kind);
                        entry.updated_at = node.updated_at;
                        entry.etag = node.etag.clone();
                        entry.deleted_at = node.deleted_at;
                    } else if entry.kind == NodeEntryKind::Directory {
                        entry.updated_at = entry.updated_at.max(node.updated_at);
                    }
                    entry.has_children = has_visible_descendants(nodes, &child_path);
                })
                .or_insert_with(|| match child_path == node.path {
                    true => NodeEntry {
                        path: node.path.clone(),
                        kind: entry_kind_from_node_kind(&node.kind),
                        updated_at: node.updated_at,
                        etag: node.etag.clone(),
                        deleted_at: node.deleted_at,
                        has_children: has_visible_descendants(nodes, &node.path),
                    },
                    false => NodeEntry {
                        path: child_path.clone(),
                        kind: NodeEntryKind::Directory,
                        updated_at: directory_updated_at(nodes, &child_path),
                        etag: String::new(),
                        deleted_at: None,
                        has_children: true,
                    },
                });
        }
    }

    entries.into_values().collect()
}

pub(crate) fn build_glob_entries(nodes: &[Node], prefix: &str) -> Vec<NodeEntry> {
    let mut entries = BTreeMap::new();
    for node in nodes {
        entries.insert(
            node.path.clone(),
            NodeEntry {
                path: node.path.clone(),
                kind: entry_kind_from_node_kind(&node.kind),
                updated_at: node.updated_at,
                etag: node.etag.clone(),
                deleted_at: node.deleted_at,
                has_children: has_visible_descendants(nodes, &node.path),
            },
        );
        for directory_path in ancestor_directory_paths(prefix, &node.path) {
            entries
                .entry(directory_path.clone())
                .or_insert_with(|| NodeEntry {
                    path: directory_path.clone(),
                    kind: NodeEntryKind::Directory,
                    updated_at: directory_updated_at(nodes, &directory_path),
                    etag: String::new(),
                    deleted_at: None,
                    has_children: true,
                });
        }
    }
    entries.into_values().collect()
}

pub(crate) fn snapshot_state_hash(nodes: &[Node]) -> String {
    let payload = nodes
        .iter()
        .map(snapshot_line)
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&payload)
}

pub(crate) fn snapshot_revision_token(
    prefix: &str,
    include_deleted: bool,
    revision: i64,
    nodes: &[Node],
) -> String {
    let include_deleted_flag = if include_deleted { "1" } else { "0" };
    let prefix_hex = hex_encode(prefix.as_bytes());
    let state_hash = snapshot_state_hash(nodes);
    format!("v3:{revision}:{include_deleted_flag}:{prefix_hex}:{state_hash}")
}

pub(crate) fn relative_to_prefix(prefix: &str, path: &str) -> Option<String> {
    if prefix == "/" {
        return path.strip_prefix('/').map(str::to_string);
    }
    if path == prefix {
        return path.rsplit('/').next().map(str::to_string);
    }
    path.strip_prefix(&format!("{prefix}/")).map(str::to_string)
}

pub(crate) fn build_fts_query(query_text: &str) -> Option<String> {
    let terms = query_text
        .split_whitespace()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

pub(crate) fn prefix_filter_sql(
    prefix: &str,
    start_index: usize,
) -> (String, Vec<rusqlite::types::Value>) {
    prefix_filter_sql_for_column("path", prefix, start_index)
}

pub(crate) fn prefix_filter_sql_for_column(
    column_name: &str,
    prefix: &str,
    start_index: usize,
) -> (String, Vec<rusqlite::types::Value>) {
    let equal_index = start_index;
    let like_index = start_index + 1;
    (
        format!(" AND ({column_name} = ?{equal_index} OR {column_name} LIKE ?{like_index})"),
        vec![
            rusqlite::types::Value::from(prefix.to_string()),
            rusqlite::types::Value::from(format!("{prefix}/%")),
        ],
    )
}

pub(crate) fn node_kind_to_db(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "file",
        NodeKind::Source => "source",
    }
}

pub(crate) fn node_kind_from_db(value: &str) -> Result<NodeKind, rusqlite::Error> {
    match value {
        "file" => Ok(NodeKind::File),
        "source" => Ok(NodeKind::Source),
        _ => Err(rusqlite::Error::InvalidColumnType(
            1,
            "kind".to_string(),
            rusqlite::types::Type::Text,
        )),
    }
}

fn map_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    Ok(Node {
        path: row.get(0)?,
        kind: node_kind_from_db(&row.get::<_, String>(1)?)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        etag: row.get(5)?,
        deleted_at: row.get(6)?,
        metadata_json: row.get(7)?,
    })
}

fn map_stored_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredNode> {
    Ok(StoredNode {
        row_id: row.get(0)?,
        node: Node {
            path: row.get(1)?,
            kind: node_kind_from_db(&row.get::<_, String>(2)?)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            etag: row.get(6)?,
            deleted_at: row.get(7)?,
            metadata_json: row.get(8)?,
        },
    })
}

fn direct_child_path(prefix: &str, path: &str) -> Option<String> {
    if prefix == "/" {
        let remainder = path.strip_prefix('/')?;
        let child = remainder.split('/').next()?;
        if child.is_empty() {
            return None;
        }
        return Some(format!("/{child}"));
    }
    if !path.starts_with(&format!("{prefix}/")) {
        return None;
    }
    let remainder = &path[prefix.len() + 1..];
    let child = remainder.split('/').next()?;
    if child.is_empty() {
        return None;
    }
    Some(format!("{prefix}/{child}"))
}

fn ancestor_directory_paths(prefix: &str, path: &str) -> Vec<String> {
    let relative = match relative_to_prefix(prefix, path) {
        Some(value) => value,
        None => return Vec::new(),
    };
    let segments = relative.split('/').collect::<Vec<_>>();
    if segments.len() <= 1 {
        return Vec::new();
    }
    let mut directories = Vec::new();
    let mut current = if prefix == "/" {
        String::new()
    } else {
        prefix.to_string()
    };
    for segment in segments.iter().take(segments.len() - 1) {
        if current.is_empty() {
            current = format!("/{segment}");
        } else {
            current = format!("{current}/{segment}");
        }
        directories.push(current.clone());
    }
    directories
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn directory_updated_at(nodes: &[Node], child_prefix: &str) -> i64 {
    nodes
        .iter()
        .filter(|node| node.path.starts_with(&format!("{child_prefix}/")))
        .map(|node| node.updated_at)
        .max()
        .unwrap_or(0)
}

fn has_visible_descendants(nodes: &[Node], child_prefix: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.path.starts_with(&format!("{child_prefix}/")))
}

fn snapshot_line(node: &Node) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        node.path,
        node_kind_to_db(&node.kind),
        node.content,
        node.etag,
        node.deleted_at
            .map(|value| value.to_string())
            .unwrap_or_default(),
        node.metadata_json
    )
}

fn entry_kind_from_node_kind(kind: &NodeKind) -> NodeEntryKind {
    match kind {
        NodeKind::File => NodeEntryKind::File,
        NodeKind::Source => NodeEntryKind::Source,
    }
}
