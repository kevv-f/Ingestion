use serde::{Deserialize, Serialize};
use thiserror::Error;

/// View model for content source list items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSourceView {
    pub id: i64,
    pub source_type: String,
    pub source_path: String,
    pub ehl_doc_id: String,
    pub chunk_count: i32,
    pub created_at: String,
    pub updated_at: String,
    /// Extracted from first chunk's meta
    pub title: Option<String>,
    pub preview_text: String,
    /// Application display name (e.g., "Microsoft Word")
    pub app_name: Option<String>,
    /// Application bundle ID (e.g., "com.microsoft.Word")
    pub bundle_id: Option<String>,
}

/// Full content detail for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDetail {
    pub id: i64,
    pub source_type: String,
    pub source_path: String,
    pub ehl_doc_id: String,
    pub chunk_count: i32,
    pub created_at: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub channel: Option<String>,
    pub full_text: String,
    /// Application display name (e.g., "Microsoft Word")
    pub app_name: Option<String>,
    /// Application bundle ID (e.g., "com.microsoft.Word")
    pub bundle_id: Option<String>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
    pub has_more: bool,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub total_sources: i64,
    pub total_chunks: i64,
}

/// Database error types
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database not found at path: {0}")]
    NotFound(String),

    #[error("Database connection error: {0}")]
    Connection(#[source] rusqlite::Error),

    #[error("Query error: {0}")]
    Query(#[source] rusqlite::Error),

    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),
}
