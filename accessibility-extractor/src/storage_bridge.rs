//! Storage bridge for the daemon - handles SQLite storage with deduplication and chunking.
//!
//! This module provides a simplified interface to store extracted content
//! in SQLite, reusing the schema from ingestion-service.
//!
//! For Slack messages, we use message-level deduplication:
//! - Each message gets a hash based on [author + time + content]
//! - New messages are appended, existing ones are skipped
//! - Content is chunked into 1024-token chunks with 100-token overlap

use crate::types::ExtractedContent;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
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

/// Result of deduplication check
#[derive(Debug, Clone)]
pub enum DedupResult {
    /// Content is new
    New(String), // ehl_doc_id
    /// Content was updated (new messages appended)
    Updated(String), // ehl_doc_id
    /// Content is duplicate (all messages already exist)
    Duplicate,
}

/// Chunk metadata stored in the meta JSON field
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ChunkMeta {
    id: String,
    source: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    chunk_index: usize,
    total_chunks: usize,
    source_type: String,
    extraction_method: String,
    app_name: String,
}

/// Chunker configuration
const MAX_TOKENS: usize = 1024;
const OVERLAP_TOKENS: usize = 100;

/// A single chunk of content
#[derive(Debug, Clone)]
struct Chunk {
    text: String,
    chunk_index: usize,
    total_chunks: usize,
    token_count: usize,
}

/// Storage manager for the daemon
pub struct DaemonStorage {
    conn: Connection,
}

impl DaemonStorage {
    /// Open or create the database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
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
            
            -- Messages table for Slack message-level deduplication
            -- Each message is stored individually with a hash for dedup
            -- message_order is used to maintain chronological order (extracted from message time)
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_url TEXT NOT NULL,
                message_hash TEXT NOT NULL,
                message_text TEXT NOT NULL,
                message_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(source_url, message_hash)
            );
            
            CREATE INDEX IF NOT EXISTS idx_messages_source_url ON messages(source_url);
            CREATE INDEX IF NOT EXISTS idx_messages_hash ON messages(message_hash);
            CREATE INDEX IF NOT EXISTS idx_messages_order ON messages(source_url, message_order);
            "#,
        )?;
        Ok(())
    }

    /// Store extracted content with deduplication
    /// For Slack: uses message-level dedup (append new messages)
    /// For other apps: uses content-level dedup (replace if changed)
    pub fn store_content(&mut self, content: &ExtractedContent) -> Result<DedupResult, StorageError> {
        // Generate URL for this content
        let url = format!(
            "accessibility://{}/{}",
            content.app_name.replace(' ', "_"),
            content.title.as_deref().unwrap_or("untitled")
        );

        // For Slack and Teams, use message-level deduplication
        if content.source == "slack" || content.source == "teams" {
            return self.store_messaging_content(content, &url);
        }

        // For other apps, use content-level deduplication (original behavior)
        self.store_content_replace(content, &url)
    }

    /// Store messaging app content (Slack, Teams) with message-level deduplication
    /// New messages are appended, existing ones are skipped
    /// Messages are stored with their time for proper ordering
    fn store_messaging_content(&mut self, content: &ExtractedContent, url: &str) -> Result<DedupResult, StorageError> {
        // Parse messages from content (each line is a message in format [Author] [Time] Message)
        let messages: Vec<&str> = content.content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        if messages.is_empty() {
            return Ok(DedupResult::Duplicate);
        }

        // Compute hash for each message and extract time for ordering
        let message_data: Vec<(String, &str, i32)> = messages
            .iter()
            .map(|msg| {
                let hash = compute_hash(msg);
                let time_order = extract_time_order(msg);
                (hash, *msg, time_order)
            })
            .collect();

        // Find which messages already exist
        let hashes: Vec<(String, &str)> = message_data.iter()
            .map(|(h, m, _)| (h.clone(), *m))
            .collect();
        let existing_hashes = self.get_existing_message_hashes(url, &hashes)?;

        // Filter to only new messages
        let new_messages: Vec<(&str, &str, i32)> = message_data
            .iter()
            .filter(|(hash, _, _)| !existing_hashes.contains(hash))
            .map(|(hash, msg, order)| (hash.as_str(), *msg, *order))
            .collect();

        if new_messages.is_empty() {
            log::debug!("[STORAGE] All {} messages already exist, skipping", messages.len());
            return Ok(DedupResult::Duplicate);
        }

        log::info!("[STORAGE] 📥 Found {} new messages out of {} total", new_messages.len(), messages.len());

        // Insert new messages with time order
        let tx = self.conn.transaction()?;
        
        for (hash, msg, order) in &new_messages {
            tx.execute(
                "INSERT OR IGNORE INTO messages (source_url, message_hash, message_text, message_order) VALUES (?1, ?2, ?3, ?4)",
                params![url, hash, msg, order],
            )?;
        }
        
        tx.commit()?;

        // Now rebuild chunks from ALL messages for this URL
        self.rebuild_chunks_for_url(content, url)
    }

    /// Get existing message hashes for a URL
    fn get_existing_message_hashes(&self, url: &str, messages: &[(String, &str)]) -> Result<std::collections::HashSet<String>, StorageError> {
        let mut existing = std::collections::HashSet::new();
        
        if messages.is_empty() {
            return Ok(existing);
        }

        // Query in batches to avoid SQL limits
        let batch_size = 100;
        for chunk in messages.chunks(batch_size) {
            let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
            let sql = format!(
                "SELECT message_hash FROM messages WHERE source_url = ?1 AND message_hash IN ({})",
                placeholders.join(", ")
            );

            let mut stmt = self.conn.prepare(&sql)?;
            
            // Build params: url + all hashes in this batch
            let hash_values: Vec<&str> = chunk.iter().map(|(h, _)| h.as_str()).collect();
            
            // Execute with dynamic params
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&url];
            for h in &hash_values {
                params_vec.push(h);
            }
            
            let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
                row.get::<_, String>(0)
            })?;

            for hash in rows.flatten() {
                existing.insert(hash);
            }
        }

        Ok(existing)
    }

    /// Rebuild chunks for a URL from all stored messages
    fn rebuild_chunks_for_url(&mut self, content: &ExtractedContent, url: &str) -> Result<DedupResult, StorageError> {
        // Get all messages for this URL
        let messages: Vec<String> = self.get_all_messages_for_url(url)?;

        if messages.is_empty() {
            return Ok(DedupResult::Duplicate);
        }

        // Combine all messages into content
        let combined_content = messages.join("\n");
        let content_hash = compute_hash(&combined_content);

        // Check if we have an existing source
        let existing = self.find_source_by_path(url)?;
        let is_update = existing.is_some();

        let tx = self.conn.transaction()?;

        let ehl_doc_id = match existing {
            Some((existing_id, ref existing_hash)) => {
                if *existing_hash == content_hash {
                    // Content unchanged
                    return Ok(DedupResult::Duplicate);
                }
                
                // Soft-delete old chunks
                tx.execute(
                    "UPDATE chunks SET is_deleted = 1 WHERE json_extract(meta, '$.id') = ?1",
                    params![existing_id],
                )?;
                
                existing_id
            }
            None => {
                generate_doc_id()
            }
        };

        // Chunk the combined content
        let chunks = chunk_content(&combined_content);
        let chunk_count = chunks.len();

        log::info!("[STORAGE] 📦 Rebuilding {} chunks for {} ({} messages)", 
            chunk_count, url, messages.len());

        // Upsert content source
        tx.execute(
            "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(source_path) DO UPDATE SET 
                content_hash = excluded.content_hash,
                chunk_count = excluded.chunk_count,
                updated_at = datetime('now')",
            params![
                content.source,
                url,
                content_hash,
                ehl_doc_id,
                chunk_count
            ],
        )?;

        // Insert new chunks
        for chunk in &chunks {
            let meta = ChunkMeta {
                id: ehl_doc_id.clone(),
                source: content.source.clone(),
                url: url.to_string(),
                title: content.title.clone(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                source_type: "accessibility".to_string(),
                extraction_method: content.extraction_method.clone(),
                app_name: content.app_name.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;

            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        
        log::info!("[STORAGE] ✅ Stored {} chunks for {} ({} total messages)", 
            chunk_count, url, messages.len());

        if is_update {
            Ok(DedupResult::Updated(ehl_doc_id))
        } else {
            Ok(DedupResult::New(ehl_doc_id))
        }
    }
    
    /// Get all messages for a URL, ordered by time
    fn get_all_messages_for_url(&self, url: &str) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT message_text FROM messages WHERE source_url = ?1 ORDER BY message_order ASC, created_at ASC"
        )?;
        
        let messages: Vec<String> = stmt
            .query_map(params![url], |row| row.get::<_, String>(0))?
            .flatten()
            .collect();
        
        Ok(messages)
    }

    /// Store content with replace semantics (for non-Slack apps)
    fn store_content_replace(&mut self, content: &ExtractedContent, url: &str) -> Result<DedupResult, StorageError> {
        let content_hash = compute_hash(&content.content);
        let existing = self.find_source_by_path(url)?;

        match existing {
            Some((ehl_doc_id, existing_hash)) => {
                if existing_hash == content_hash {
                    Ok(DedupResult::Duplicate)
                } else {
                    self.update_source(&ehl_doc_id, content, url, &content_hash)?;
                    Ok(DedupResult::Updated(ehl_doc_id))
                }
            }
            None => {
                let ehl_doc_id = generate_doc_id();
                self.insert_source(content, url, &content_hash, &ehl_doc_id)?;
                Ok(DedupResult::New(ehl_doc_id))
            }
        }
    }

    /// Find a content source by its path (URL)
    fn find_source_by_path(&self, source_path: &str) -> Result<Option<(String, String)>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT ehl_doc_id, content_hash FROM content_sources WHERE source_path = ?1"
        )?;

        let result = stmt.query_row(params![source_path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new content source with chunking
    fn insert_source(
        &mut self,
        content: &ExtractedContent,
        url: &str,
        content_hash: &str,
        ehl_doc_id: &str,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let chunks = chunk_content(&content.content);
        let chunk_count = chunks.len();

        log::info!("[STORAGE] 📦 Chunking content into {} chunks", chunk_count);

        tx.execute(
            "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![content.source, url, content_hash, ehl_doc_id, chunk_count],
        )?;

        for chunk in &chunks {
            let meta = ChunkMeta {
                id: ehl_doc_id.to_string(),
                source: content.source.clone(),
                url: url.to_string(),
                title: content.title.clone(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                source_type: "accessibility".to_string(),
                extraction_method: content.extraction_method.clone(),
                app_name: content.app_name.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;
            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        log::info!("[STORAGE] ✅ Stored {} chunks for {}", chunk_count, url);
        Ok(())
    }

    /// Update an existing content source with new chunks
    fn update_source(
        &mut self,
        ehl_doc_id: &str,
        content: &ExtractedContent,
        url: &str,
        content_hash: &str,
    ) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;

        tx.execute(
            "UPDATE chunks SET is_deleted = 1 WHERE json_extract(meta, '$.id') = ?1",
            params![ehl_doc_id],
        )?;

        let chunks = chunk_content(&content.content);
        let chunk_count = chunks.len();

        log::info!("[STORAGE] 🔄 Updating content with {} chunks", chunk_count);

        tx.execute(
            "UPDATE content_sources SET content_hash = ?1, chunk_count = ?2, 
             updated_at = datetime('now') WHERE ehl_doc_id = ?3",
            params![content_hash, chunk_count, ehl_doc_id],
        )?;

        for chunk in &chunks {
            let meta = ChunkMeta {
                id: ehl_doc_id.to_string(),
                source: content.source.clone(),
                url: url.to_string(),
                title: content.title.clone(),
                chunk_index: chunk.chunk_index,
                total_chunks: chunk.total_chunks,
                source_type: "accessibility".to_string(),
                extraction_method: content.extraction_method.clone(),
                app_name: content.app_name.clone(),
            };

            let meta_json = serde_json::to_string(&meta)?;
            tx.execute(
                "INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                params![chunk.text, meta_json],
            )?;
        }

        tx.commit()?;
        log::info!("[STORAGE] ✅ Updated {} chunks for {}", chunk_count, url);
        Ok(())
    }
}

/// Chunk content into fixed-size token chunks with overlap.
fn chunk_content(content: &str) -> Vec<Chunk> {
    if content.trim().is_empty() {
        return vec![];
    }

    let words: Vec<&str> = content.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    if words.len() <= MAX_TOKENS {
        return vec![Chunk {
            text: content.to_string(),
            chunk_index: 0,
            total_chunks: 1,
            token_count: words.len(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let step = MAX_TOKENS - OVERLAP_TOKENS;

    while start < words.len() {
        let end = (start + MAX_TOKENS).min(words.len());
        let chunk_words = &words[start..end];
        let chunk_text = chunk_words.join(" ");

        chunks.push(Chunk {
            text: chunk_text,
            chunk_index: chunks.len(),
            total_chunks: 0,
            token_count: chunk_words.len(),
        });

        start += step;

        if words.len() - start < OVERLAP_TOKENS && start < words.len() {
            let remaining = &words[start..];
            if let Some(last) = chunks.last_mut() {
                last.text = format!("{} {}", last.text, remaining.join(" "));
                last.token_count += remaining.len();
            }
            break;
        }
    }

    let total = chunks.len();
    for chunk in &mut chunks {
        chunk.total_chunks = total;
    }

    chunks
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_doc_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Extract a sortable time order from a message.
/// Message format: [Author] [Time] Content
/// Time format: "10:59 AM" or "4:30 PM"
/// Returns minutes since midnight for sorting (0-1439)
fn extract_time_order(message: &str) -> i32 {
    // Look for time pattern [HH:MM AM/PM]
    let time_pattern = regex_lite::Regex::new(r"\[(\d{1,2}):(\d{2})\s*(AM|PM)\]");
    
    if let Ok(pattern) = time_pattern {
        if let Some(caps) = pattern.captures(message) {
            let hour: i32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let minute: i32 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let is_pm = caps.get(3).map(|m| m.as_str() == "PM").unwrap_or(false);
            
            // Convert to 24-hour format
            let hour_24 = if is_pm && hour != 12 {
                hour + 12
            } else if !is_pm && hour == 12 {
                0
            } else {
                hour
            };
            
            return hour_24 * 60 + minute;
        }
    }
    
    // Default to 0 if no time found
    0
}
