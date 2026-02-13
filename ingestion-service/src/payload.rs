//! Capture payload types

use serde::{Deserialize, Serialize};

/// Payload received from any ingestion source (browser, clipboard, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturePayload {
    /// Source type: "slack", "gmail", "jira", "browser", "clipboard", etc.
    pub source: String,

    /// Location identifier (URL, path, or empty string if none)
    pub url: String,

    /// The text content to ingest
    pub content: String,

    /// Optional title/subject
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional author/sender
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Optional channel/project/workspace
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Optional unix timestamp in seconds (defaults to now)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,

    /// Application display name (e.g., "Microsoft Word", "Google Chrome")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    /// Application bundle ID (e.g., "com.microsoft.Word")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
}

/// Response sent back to the caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResponse {
    pub status: ResponseStatus,
    pub action: IngestionAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ehl_doc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IngestionAction {
    Created,
    Updated,
    Skipped,
    Failed,
}

impl IngestionResponse {
    pub fn created(ehl_doc_id: String, chunk_count: usize) -> Self {
        Self {
            status: ResponseStatus::Ok,
            action: IngestionAction::Created,
            ehl_doc_id: Some(ehl_doc_id),
            chunk_count: Some(chunk_count),
            message: None,
        }
    }

    pub fn updated(ehl_doc_id: String, chunk_count: usize) -> Self {
        Self {
            status: ResponseStatus::Ok,
            action: IngestionAction::Updated,
            ehl_doc_id: Some(ehl_doc_id),
            chunk_count: Some(chunk_count),
            message: None,
        }
    }

    pub fn skipped(reason: &str) -> Self {
        Self {
            status: ResponseStatus::Ok,
            action: IngestionAction::Skipped,
            ehl_doc_id: None,
            chunk_count: None,
            message: Some(reason.to_string()),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: ResponseStatus::Error,
            action: IngestionAction::Failed,
            ehl_doc_id: None,
            chunk_count: None,
            message: Some(message.to_string()),
        }
    }
}
