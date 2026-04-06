// Where: crates/wiki_store/src/source_upload.rs
// What: Upload-only source assembly before final persistence into sources/source_bodies.
// Why: Large source payloads may arrive in chunks, but the source of truth stays as one stored body.
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;
use wiki_types::{
    AppendSourceChunkInput, BeginSourceUploadInput, FinalizeSourceUploadInput,
    FinalizeSourceUploadOutput, SourceUploadStatus,
};

use crate::source::insert_source_row;

#[derive(Clone, Debug)]
struct StoredUpload {
    source_type: String,
    title: Option<String>,
    canonical_uri: Option<String>,
    sha256: String,
    mime_type: Option<String>,
    imported_at: i64,
    metadata_json: String,
}

pub(crate) fn begin_source_upload_row(
    conn: &Connection,
    input: BeginSourceUploadInput,
) -> Result<String, String> {
    if input.source_type.trim().is_empty() {
        return Err("source_type must not be empty".to_string());
    }
    if input.sha256.trim().is_empty() {
        return Err("sha256 must not be empty".to_string());
    }
    let upload_id = format!("upload_{}", Uuid::new_v4());
    conn.execute(
        "INSERT INTO source_uploads (
            id, source_type, title, canonical_uri, sha256, mime_type, imported_at, metadata_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            upload_id,
            input.source_type,
            input.title,
            input.canonical_uri,
            input.sha256,
            input.mime_type,
            input.imported_at,
            input.metadata_json,
            unix_timestamp_now(),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(upload_id)
}

pub(crate) fn append_source_chunk_row(
    conn: &Connection,
    input: AppendSourceChunkInput,
) -> Result<SourceUploadStatus, String> {
    if input.chunk_text.is_empty() {
        return Err("chunk_text must not be empty".to_string());
    }
    ensure_upload_exists(conn, &input.upload_id)?;
    let ordinal = next_chunk_ordinal(conn, &input.upload_id)?;
    conn.execute(
        "INSERT INTO source_upload_chunks (upload_id, ordinal, chunk_text) VALUES (?1, ?2, ?3)",
        params![input.upload_id, ordinal, input.chunk_text],
    )
    .map_err(|error| error.to_string())?;
    load_upload_status(conn, &input.upload_id)
}

pub(crate) fn finalize_source_upload_row(
    conn: &mut Connection,
    input: FinalizeSourceUploadInput,
) -> Result<FinalizeSourceUploadOutput, String> {
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let upload =
        load_upload(&tx, &input.upload_id)?.ok_or_else(|| "upload does not exist".to_string())?;
    let chunks = load_chunks(&tx, &input.upload_id)?;
    if chunks.is_empty() {
        return Err("upload has no chunks".to_string());
    }
    let chunk_count = u32::try_from(chunks.len()).map_err(|_| "too many chunks".to_string())?;
    let body_text = chunks.join("");
    let source_id = insert_source_row(
        &tx,
        &upload.source_type,
        upload.title,
        upload.canonical_uri,
        &upload.sha256,
        upload.mime_type,
        upload.imported_at,
        &upload.metadata_json,
        &body_text,
    )?;
    tx.execute(
        "DELETE FROM source_upload_chunks WHERE upload_id = ?1",
        params![input.upload_id],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM source_uploads WHERE id = ?1",
        params![input.upload_id],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(FinalizeSourceUploadOutput {
        source_id,
        chunk_count,
    })
}

fn load_upload(conn: &Connection, upload_id: &str) -> Result<Option<StoredUpload>, String> {
    conn.query_row(
        "SELECT source_type, title, canonical_uri, sha256, mime_type, imported_at, metadata_json
         FROM source_uploads WHERE id = ?1",
        params![upload_id],
        |row| {
            Ok(StoredUpload {
                source_type: row.get(0)?,
                title: row.get(1)?,
                canonical_uri: row.get(2)?,
                sha256: row.get(3)?,
                mime_type: row.get(4)?,
                imported_at: row.get(5)?,
                metadata_json: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn load_chunks(conn: &Connection, upload_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT chunk_text FROM source_upload_chunks
             WHERE upload_id = ?1 ORDER BY ordinal",
        )
        .map_err(|error| error.to_string())?;
    stmt.query_map(params![upload_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn ensure_upload_exists(conn: &Connection, upload_id: &str) -> Result<(), String> {
    if load_upload(conn, upload_id)?.is_none() {
        return Err("upload does not exist".to_string());
    }
    Ok(())
}

fn next_chunk_ordinal(conn: &Connection, upload_id: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(ordinal), -1) + 1 FROM source_upload_chunks WHERE upload_id = ?1",
        params![upload_id],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn load_upload_status(conn: &Connection, upload_id: &str) -> Result<SourceUploadStatus, String> {
    let chunks = load_chunks(conn, upload_id)?;
    let byte_count = chunks.iter().try_fold(0_u64, |total, chunk| {
        let chunk_bytes =
            u64::try_from(chunk.len()).map_err(|_| "chunk is too large".to_string())?;
        total
            .checked_add(chunk_bytes)
            .ok_or_else(|| "upload byte_count overflow".to_string())
    })?;
    Ok(SourceUploadStatus {
        upload_id: upload_id.to_string(),
        chunk_count: u32::try_from(chunks.len()).map_err(|_| "too many chunks".to_string())?,
        byte_count,
    })
}

fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
