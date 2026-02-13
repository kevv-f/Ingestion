mod types;

pub use types::*;

use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;

/// Get the default database path (~/Library/Application Support/clace-ingestion/content.db)
pub fn get_default_db_path() -> String {
    // Try macOS Application Support directory first
    if let Some(data_dir) = dirs::data_dir() {
        let mut path = PathBuf::from(data_dir);
        path.push("clace-ingestion");
        path.push("content.db");
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }
    
    // Fallback to home directory path
    dirs::home_dir()
        .map(|home| {
            let mut path = PathBuf::from(home);
            path.push("Library");
            path.push("Application Support");
            path.push("clace-ingestion");
            path.push("content.db");
            path.to_string_lossy().to_string()
        })
        .unwrap_or_else(|| "~/Library/Application Support/clace-ingestion/content.db".to_string())
}

/// Database connection and query handler
pub struct ViewerDb {
    conn: Connection,
}

#[cfg(test)]
impl ViewerDb {
    /// Create ViewerDb from an existing connection (for testing only)
    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }
}

impl ViewerDb {
    /// Open database at the specified path
    pub fn open(path: &str) -> Result<Self, DbError> {
        // Expand ~ to home directory
        let expanded_path = if path.starts_with("~/") {
            dirs::home_dir()
                .map(|home| {
                    let mut p = PathBuf::from(home);
                    p.push(&path[2..]);
                    p.to_string_lossy().to_string()
                })
                .unwrap_or_else(|| path.to_string())
        } else {
            path.to_string()
        };

        // Check if file exists
        if !std::path::Path::new(&expanded_path).exists() {
            return Err(DbError::NotFound(expanded_path));
        }

        let conn = Connection::open(&expanded_path).map_err(DbError::Connection)?;

        Ok(Self { conn })
    }

    /// Get paginated content sources with preview
    pub fn get_sources(
        &self,
        page: i32,
        limit: i32,
    ) -> Result<PaginatedResponse<ContentSourceView>, DbError> {
        let offset = page * limit;

        // Get total count
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM content_sources", [], |row| {
                row.get(0)
            })
            .map_err(DbError::Query)?;

        // Get paginated sources
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT 
                    cs.id,
                    cs.source_type,
                    cs.source_path,
                    cs.ehl_doc_id,
                    cs.chunk_count,
                    cs.created_at,
                    cs.updated_at
                FROM content_sources cs
                ORDER BY cs.updated_at DESC
                LIMIT ?1 OFFSET ?2
                "#,
            )
            .map_err(DbError::Query)?;

        let sources: Vec<ContentSourceView> = stmt
            .query_map([limit, offset], |row| {
                let ehl_doc_id: String = row.get(3)?;
                Ok(ContentSourceView {
                    id: row.get(0)?,
                    source_type: row.get(1)?,
                    source_path: row.get(2)?,
                    ehl_doc_id,
                    chunk_count: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    title: None,       // Will be populated from chunk meta
                    preview_text: String::new(), // Will be populated from first chunk
                    app_name: None,    // Will be populated from chunk meta
                    bundle_id: None,   // Will be populated from chunk meta
                })
            })
            .map_err(DbError::Query)?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(DbError::Query)?;

        // Populate title, preview, app_name, and bundle_id from chunks
        let sources = sources
            .into_iter()
            .map(|mut source| {
                if let Ok((title, preview, app_name, bundle_id)) = self.get_source_preview(&source.ehl_doc_id) {
                    source.title = title;
                    source.preview_text = preview;
                    source.app_name = app_name;
                    source.bundle_id = bundle_id;
                }
                source
            })
            .collect();

        let has_more = (offset + limit) < total as i32;

        Ok(PaginatedResponse {
            items: sources,
            total,
            page,
            page_size: limit,
            has_more,
        })
    }

    /// Get preview text, title, app_name, and bundle_id from first chunk
    fn get_source_preview(&self, ehl_doc_id: &str) -> Result<(Option<String>, String, Option<String>, Option<String>), DbError> {
        let result: Result<(String, String), _> = self.conn.query_row(
            r#"
            SELECT text, meta FROM chunks 
            WHERE json_extract(meta, '$.id') = ?1 
            AND is_deleted = 0
            ORDER BY json_extract(meta, '$.chunk_index') ASC
            LIMIT 1
            "#,
            [ehl_doc_id],
            |row| {
                let text: String = row.get(0)?;
                let meta: String = row.get(1)?;
                Ok((text, meta))
            },
        );

        match result {
            Ok((text, meta_json)) => {
                let meta_value = serde_json::from_str::<serde_json::Value>(&meta_json).ok();
                
                let title = meta_value.as_ref()
                    .and_then(|v| v.get("title").and_then(|t| t.as_str()).map(String::from));
                
                let app_name = meta_value.as_ref()
                    .and_then(|v| v.get("app_name").and_then(|t| t.as_str()).map(String::from));
                
                let bundle_id = meta_value.as_ref()
                    .and_then(|v| v.get("bundle_id").and_then(|t| t.as_str()).map(String::from));

                // Truncate preview to ~150 chars (safely handle UTF-8)
                let preview = if text.chars().count() > 150 {
                    let truncated: String = text.chars().take(147).collect();
                    format!("{}...", truncated)
                } else {
                    text
                };

                Ok((title, preview, app_name, bundle_id))
            }
            Err(_) => Ok((None, String::new(), None, None)),
        }
    }

    /// Get full content detail by ehl_doc_id
    pub fn get_detail(&self, ehl_doc_id: &str) -> Result<ContentDetail, DbError> {
        // Get content source
        let source: (i64, String, String, i32, String, String) = self
            .conn
            .query_row(
                r#"
                SELECT id, source_type, source_path, chunk_count, created_at, updated_at
                FROM content_sources
                WHERE ehl_doc_id = ?1
                "#,
                [ehl_doc_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|_| DbError::NotFound(format!("Content source not found: {}", ehl_doc_id)))?;

        // Get all chunks for this document
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT text, meta FROM chunks
                WHERE json_extract(meta, '$.id') = ?1
                AND is_deleted = 0
                ORDER BY json_extract(meta, '$.chunk_index') ASC
                "#,
            )
            .map_err(DbError::Query)?;

        let chunks: Vec<(String, String)> = stmt
            .query_map([ehl_doc_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(DbError::Query)?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(DbError::Query)?;

        // Extract metadata from first chunk
        let (title, author, channel, app_name, bundle_id) = chunks
            .first()
            .and_then(|(_, meta_json)| {
                serde_json::from_str::<serde_json::Value>(meta_json).ok()
            })
            .map(|meta| {
                (
                    meta.get("title").and_then(|v| v.as_str()).map(String::from),
                    meta.get("author").and_then(|v| v.as_str()).map(String::from),
                    meta.get("channel").and_then(|v| v.as_str()).map(String::from),
                    meta.get("app_name").and_then(|v| v.as_str()).map(String::from),
                    meta.get("bundle_id").and_then(|v| v.as_str()).map(String::from),
                )
            })
            .unwrap_or((None, None, None, None, None));

        // Reconstruct full text (use newline separator between chunks)
        let full_text = chunks
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ContentDetail {
            id: source.0,
            source_type: source.1,
            source_path: source.2,
            ehl_doc_id: ehl_doc_id.to_string(),
            chunk_count: source.3,
            created_at: source.4,
            updated_at: source.5,
            title,
            author,
            channel,
            full_text,
            app_name,
            bundle_id,
        })
    }

    /// Get total source count
    pub fn get_source_count(&self) -> Result<i64, DbError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM content_sources", [], |row| {
                row.get(0)
            })
            .map_err(DbError::Query)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DbStats, DbError> {
        let total_sources = self.get_source_count()?;

        let total_chunks: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE is_deleted = 0",
                [],
                |row| row.get(0),
            )
            .map_err(DbError::Query)?;

        Ok(DbStats {
            total_sources,
            total_chunks,
        })
    }

    /// Delete a content source and its associated chunks by ehl_doc_id
    pub fn delete_content_source(&self, ehl_doc_id: &str) -> Result<(), DbError> {
        // Mark chunks as deleted (soft delete)
        self.conn
            .execute(
                r#"
                UPDATE chunks 
                SET is_deleted = 1 
                WHERE json_extract(meta, '$.id') = ?1
                "#,
                [ehl_doc_id],
            )
            .map_err(DbError::Query)?;

        // Delete the content source record
        self.conn
            .execute(
                "DELETE FROM content_sources WHERE ehl_doc_id = ?1",
                [ehl_doc_id],
            )
            .map_err(DbError::Query)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
