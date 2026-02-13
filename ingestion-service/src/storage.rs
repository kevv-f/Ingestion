//! SQLite storage for content_sources and chunks

use crate::chunker::Chunk;
use crate::payload::CapturePayload;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Content source record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    pub id: i64,
    pub source_type: String,
    pub source_path: String,
    pub content_hash: String,
    pub ehl_doc_id: String,
    pub chunk_count: i32,
    pub ingestion_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Chunk metadata stored in the meta JSON field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    pub id: String,             // ehl_doc_id
    pub source: String,         // source type
    pub url: String,            // source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub source_type: String,    // "browser_capture"
    /// Application display name (e.g., "Microsoft Word")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Application bundle ID (e.g., "com.microsoft.Word")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
}

/// SQLite storage manager
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open or create the database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            r#"
            -- Content sources table (tracks what we've ingested)
            CREATE TABLE IF NOT EXISTS content_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL,
                source_path TEXT NOT NULL UNIQUE,
                content_hash TEXT NOT NULL,
                ehl_doc_id TEXT NOT NULL UNIQUE,
                chunk_count INTEGER NOT NULL DEFAULT 0,
                ingestion_status TEXT NOT NULL DEFAULT 'ingested',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_content_sources_path ON content_sources(source_path);
            CREATE INDEX IF NOT EXISTS idx_content_sources_ehl_doc_id ON content_sources(ehl_doc_id);
            CREATE INDEX IF NOT EXISTS idx_content_sources_hash ON content_sources(content_hash);

            -- Chunks table (actual content chunks)
            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vector_index INTEGER,
                text TEXT NOT NULL,
                meta TEXT NOT NULL,
                is_deleted INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_deleted ON chunks(is_deleted);
            "#,
        )?;
        Ok(())
    }

    /// Find a content source by its path (URL)
    pub fn find_source_by_path(&self, source_path: &str) -> Result<Option<ContentSource>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_type, source_path, content_hash, ehl_doc_id, chunk_count, 
                    ingestion_status, created_at, updated_at 
             FROM content_sources WHERE source_path = ?1"
        )?;

        let result = stmt.query_row(params![source_path], |row| {
            Ok(ContentSource {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_path: row.get(2)?,
                content_hash: row.get(3)?,
                ehl_doc_id: row.get(4)?,
                chunk_count: row.get(5)?,
                ingestion_status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        });

        match result {
            Ok(source) => Ok(Some(source)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Find sources with similar paths (for OCR metadata-based dedup)
    /// Matches sources where the path starts with the same prefix (source type + title base)
    pub fn find_similar_sources(&self, source_type: &str, title_prefix: &str) -> Result<Vec<ContentSource>, StorageError> {
        // Build a pattern to match similar OCR sources
        // e.g., "ocr://vscode/ocr-extraction-md" should match existing entries for the same document
        let pattern = format!("ocr://{}/%{}%", source_type, title_prefix);
        
        let mut stmt = self.conn.prepare(
            "SELECT id, source_type, source_path, content_hash, ehl_doc_id, chunk_count, 
                    ingestion_status, created_at, updated_at 
             FROM content_sources 
             WHERE source_path LIKE ?1
             ORDER BY updated_at DESC
             LIMIT 10"
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            Ok(ContentSource {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_path: row.get(2)?,
                content_hash: row.get(3)?,
                ehl_doc_id: row.get(4)?,
                chunk_count: row.get(5)?,
                ingestion_status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(row?);
        }
        Ok(sources)
    }

    /// Get the current content for a source (concatenated from chunks)
    pub fn get_source_content(&self, ehl_doc_id: &str) -> Result<String, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT text FROM chunks 
             WHERE json_extract(meta, '$.id') = ?1 AND is_deleted = 0
             ORDER BY json_extract(meta, '$.chunk_index')"
        )?;

        let rows = stmt.query_map(params![ehl_doc_id], |row| {
            row.get::<_, String>(0)
        })?;

        let mut content = String::new();
        for row in rows {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&row?);
        }
        Ok(content)
    }

    /// Append new content to an existing source (for incremental OCR updates)
    pub fn append_to_source(
        &mut self,
        ehl_doc_id: &str,
        payload: &CapturePayload,
        _new_content: &str,  // Content already chunked, kept for potential future use
        new_content_hash: &str,
        chunks: &[Chunk],
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;

        // Get current chunk count to continue indexing
        let current_chunk_count: i32 = tx.query_row(
            "SELECT chunk_count FROM content_sources WHERE ehl_doc_id = ?1",
            params![ehl_doc_id],
            |row| row.get(0),
        )?;

        // Update content source with new hash and chunk count
        tx.execute(
            "UPDATE content_sources SET content_hash = ?1, chunk_count = ?2, 
             updated_at = datetime('now') WHERE ehl_doc_id = ?3",
            params![new_content_hash, current_chunk_count + chunks.len() as i32, ehl_doc_id],
        )?;

        // Append new chunks (don't delete old ones)
        for (i, chunk) in chunks.iter().enumerate() {
            let meta = ChunkMeta {
                id: ehl_doc_id.to_string(),
                source: payload.source.clone(),
                url: payload.url.clone(),
                title: payload.title.clone(),
                author: payload.author.clone(),
                channel: payload.channel.clone(),
                chunk_index: current_chunk_count as usize + i,
                total_chunks: current_chunk_count as usize + chunks.len(),
                source_type: "capture".to_string(),
                app_name: payload.app_name.clone(),
                bundle_id: payload.bundle_id.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;

            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Insert a new content source and its chunks
    pub fn insert_source(
        &mut self,
        payload: &CapturePayload,
        content_hash: &str,
        ehl_doc_id: &str,
        chunks: &[Chunk],
    ) -> Result<i64, StorageError> {
        let tx = self.conn.transaction()?;

        // Insert content source
        tx.execute(
            "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                payload.source,
                payload.url,
                content_hash,
                ehl_doc_id,
                chunks.len() as i32
            ],
        )?;

        let source_id = tx.last_insert_rowid();

        // Insert chunks
        for chunk in chunks {
            let meta = ChunkMeta {
                id: ehl_doc_id.to_string(),
                source: payload.source.clone(),
                url: payload.url.clone(),
                title: payload.title.clone(),
                author: payload.author.clone(),
                channel: payload.channel.clone(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                source_type: "capture".to_string(),
                app_name: payload.app_name.clone(),
                bundle_id: payload.bundle_id.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;

            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        Ok(source_id)
    }

    /// Update an existing content source with new content
    pub fn update_source(
        &mut self,
        ehl_doc_id: &str,
        payload: &CapturePayload,
        content_hash: &str,
        chunks: &[Chunk],
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;

        // Soft-delete old chunks
        tx.execute(
            "UPDATE chunks SET is_deleted = 1 WHERE json_extract(meta, '$.id') = ?1",
            params![ehl_doc_id],
        )?;

        // Update content source
        tx.execute(
            "UPDATE content_sources SET content_hash = ?1, chunk_count = ?2, 
             updated_at = datetime('now') WHERE ehl_doc_id = ?3",
            params![content_hash, chunks.len() as i32, ehl_doc_id],
        )?;

        // Insert new chunks
        for chunk in chunks {
            let meta = ChunkMeta {
                id: ehl_doc_id.to_string(),
                source: payload.source.clone(),
                url: payload.url.clone(),
                title: payload.title.clone(),
                author: payload.author.clone(),
                channel: payload.channel.clone(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                source_type: "capture".to_string(),
                app_name: payload.app_name.clone(),
                bundle_id: payload.bundle_id.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;

            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats, StorageError> {
        let source_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM content_sources",
            [],
            |row| row.get(0),
        )?;

        let chunk_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;

        Ok(StorageStats {
            source_count: source_count as usize,
            chunk_count: chunk_count as usize,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub source_count: usize,
    pub chunk_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::Chunker;

    fn make_payload() -> CapturePayload {
        CapturePayload {
            source: "slack".to_string(),
            url: "https://workspace.slack.com/archives/C123".to_string(),
            content: "Hello world this is a test message".to_string(),
            title: Some("engineering".to_string()),
            author: None,
            channel: Some("engineering".to_string()),
            timestamp: Some(1234567890),
            app_name: Some("Slack".to_string()),
            bundle_id: Some("com.tinyspeck.slackmacgap".to_string()),
        }
    }

    #[test]
    fn test_insert_and_find() {
        let mut storage = Storage::open_in_memory().unwrap();
        let payload = make_payload();
        let chunker = Chunker::with_defaults();
        let chunks = chunker.chunk(&payload.content);

        storage
            .insert_source(&payload, "hash123", "doc-uuid", &chunks)
            .unwrap();

        let found = storage.find_source_by_path(&payload.url).unwrap();
        assert!(found.is_some());

        let source = found.unwrap();
        assert_eq!(source.source_type, "slack");
        assert_eq!(source.content_hash, "hash123");
        assert_eq!(source.ehl_doc_id, "doc-uuid");
    }

    #[test]
    fn test_update_source() {
        let mut storage = Storage::open_in_memory().unwrap();
        let payload = make_payload();
        let chunker = Chunker::with_defaults();
        let chunks = chunker.chunk(&payload.content);

        storage
            .insert_source(&payload, "hash123", "doc-uuid", &chunks)
            .unwrap();

        // Update with new content
        let mut updated_payload = payload.clone();
        updated_payload.content = "Updated content here".to_string();
        let new_chunks = chunker.chunk(&updated_payload.content);

        storage
            .update_source("doc-uuid", &updated_payload, "hash456", &new_chunks)
            .unwrap();

        let found = storage.find_source_by_path(&payload.url).unwrap().unwrap();
        assert_eq!(found.content_hash, "hash456");
    }
}
