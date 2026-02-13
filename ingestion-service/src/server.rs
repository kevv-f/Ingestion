//! Unix socket server for receiving capture payloads

use crate::chunker::Chunker;
use crate::dedup::{compute_hash, DedupCache, DedupResult};
use crate::payload::{CapturePayload, IngestionResponse};
use crate::storage::Storage;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Ingestion server configuration
pub struct ServerConfig {
    /// Path to the Unix socket
    pub socket_path: PathBuf,
    /// Path to the SQLite database
    pub db_path: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clace-ingestion");

        Self {
            socket_path: PathBuf::from("/tmp/clace-ingestion.sock"),
            db_path: data_dir.join("content.db"),
        }
    }
}

/// Shared state for the ingestion service
struct ServiceState {
    storage: Storage,
    cache: DedupCache,
    chunker: Chunker,
}

/// Ingestion server that listens on a Unix socket
pub struct IngestionServer {
    config: ServerConfig,
    state: Arc<Mutex<ServiceState>>,
}

impl IngestionServer {
    /// Create a new server with the given configuration
    pub fn new(config: ServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure data directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let storage = Storage::open(&config.db_path)?;
        let cache = DedupCache::with_defaults();
        let chunker = Chunker::with_defaults();

        let state = Arc::new(Mutex::new(ServiceState {
            storage,
            cache,
            chunker,
        }));

        Ok(Self { config, state })
    }

    /// Create a server with default configuration
    pub fn with_defaults() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new(ServerConfig::default())
    }

    /// Start the server and listen for connections
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Remove existing socket file if present
        if self.config.socket_path.exists() {
            std::fs::remove_file(&self.config.socket_path)?;
        }

        let listener = UnixListener::bind(&self.config.socket_path)?;
        info!("Ingestion server listening on {:?}", self.config.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let state = Arc::clone(&self.state);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, state).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.config.socket_path
    }

    /// Process a single payload (for direct integration without socket)
    pub async fn process(&self, payload: CapturePayload) -> IngestionResponse {
        let mut state = self.state.lock().await;
        process_payload(&mut state, payload)
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: UnixStream,
    state: Arc<Mutex<ServiceState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read one JSON payload per line
    while reader.read_line(&mut line).await? > 0 {
        let response = match serde_json::from_str::<CapturePayload>(&line) {
            Ok(payload) => {
                info!("Received: {} - {}", payload.source, payload.url);
                let mut state = state.lock().await;
                process_payload(&mut state, payload)
            }
            Err(e) => {
                warn!("Failed to parse payload: {}", e);
                IngestionResponse::error(&format!("Parse error: {}", e))
            }
        };

        // Send response
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        line.clear();
    }

    Ok(())
}

/// Process a single payload
fn process_payload(state: &mut ServiceState, payload: CapturePayload) -> IngestionResponse {
    let content_hash = compute_hash(&payload.content);
    
    // Normalize the URL to create a canonical source path
    // This handles cases like Google Docs where URLs have varying query params
    let source_path = normalize_source_path(&payload.source, &payload.url);

    // For OCR sources, use metadata-based deduplication with content appending
    if payload.source == "ocr-capture" || payload.url.starts_with("ocr://") {
        return process_ocr_payload(state, payload, &source_path, &content_hash);
    }

    // Check in-memory cache first
    let cache_result = state.cache.check(&source_path, &content_hash);

    match cache_result {
        DedupResult::Duplicate(_ehl_doc_id) => {
            info!("Duplicate content (cache hit): {}", source_path);
            IngestionResponse::skipped("Content unchanged (cache)")
        }

        DedupResult::Changed(ehl_doc_id) => {
            info!("Content changed, updating: {}", source_path);

            // Chunk the new content
            let chunks = state.chunker.chunk(&payload.content);

            // Update in storage
            match state
                .storage
                .update_source(&ehl_doc_id, &payload, &content_hash, &chunks)
            {
                Ok(()) => {
                    // Update cache
                    state.cache.insert(
                        source_path,
                        content_hash,
                        ehl_doc_id.clone(),
                    );
                    IngestionResponse::updated(ehl_doc_id, chunks.len())
                }
                Err(e) => {
                    error!("Storage update error: {}", e);
                    IngestionResponse::error(&format!("Storage error: {}", e))
                }
            }
        }

        DedupResult::New => {
            // Check database (cache miss doesn't mean it's truly new)
            match state.storage.find_source_by_path(&source_path) {
                Ok(Some(existing)) => {
                    if existing.content_hash == content_hash {
                        // Same content, just wasn't in cache
                        info!("Duplicate content (db hit): {}", source_path);
                        state.cache.insert(
                            source_path,
                            content_hash,
                            existing.ehl_doc_id.clone(),
                        );
                        return IngestionResponse::skipped("Content unchanged (db)");
                    }

                    // Content changed
                    info!("Content changed (db), updating: {}", source_path);
                    let chunks = state.chunker.chunk(&payload.content);

                    match state.storage.update_source(
                        &existing.ehl_doc_id,
                        &payload,
                        &content_hash,
                        &chunks,
                    ) {
                        Ok(()) => {
                            state.cache.insert(
                                source_path,
                                content_hash,
                                existing.ehl_doc_id.clone(),
                            );
                            IngestionResponse::updated(existing.ehl_doc_id, chunks.len())
                        }
                        Err(e) => {
                            error!("Storage update error: {}", e);
                            IngestionResponse::error(&format!("Storage error: {}", e))
                        }
                    }
                }

                Ok(None) => {
                    // Truly new content
                    info!("New content: {}", source_path);
                    let ehl_doc_id = uuid::Uuid::new_v4().to_string();
                    let chunks = state.chunker.chunk(&payload.content);

                    // Create a modified payload with the normalized URL for storage
                    let mut storage_payload = payload.clone();
                    storage_payload.url = source_path.clone();

                    match state
                        .storage
                        .insert_source(&storage_payload, &content_hash, &ehl_doc_id, &chunks)
                    {
                        Ok(_) => {
                            state.cache.insert(
                                source_path,
                                content_hash,
                                ehl_doc_id.clone(),
                            );
                            IngestionResponse::created(ehl_doc_id, chunks.len())
                        }
                        Err(e) => {
                            error!("Storage insert error: {}", e);
                            IngestionResponse::error(&format!("Storage error: {}", e))
                        }
                    }
                }

                Err(e) => {
                    error!("Storage query error: {}", e);
                    IngestionResponse::error(&format!("Storage error: {}", e))
                }
            }
        }
    }
}

/// Process OCR payload with metadata-based deduplication and content appending
fn process_ocr_payload(
    state: &mut ServiceState,
    payload: CapturePayload,
    source_path: &str,
    content_hash: &str,
) -> IngestionResponse {
    // First, check for exact path match
    match state.storage.find_source_by_path(source_path) {
        Ok(Some(existing)) => {
            // Found exact match - check if content is different
            if existing.content_hash == content_hash {
                info!("OCR duplicate (exact match): {}", source_path);
                state.cache.insert(
                    source_path.to_string(),
                    content_hash.to_string(),
                    existing.ehl_doc_id.clone(),
                );
                return IngestionResponse::skipped("Content unchanged");
            }

            // Content changed - get existing content and find new text to append
            match state.storage.get_source_content(&existing.ehl_doc_id) {
                Ok(existing_content) => {
                    let new_text = extract_new_content(&existing_content, &payload.content);
                    
                    if new_text.is_empty() || new_text.len() < 50 {
                        info!("OCR no significant new content: {}", source_path);
                        return IngestionResponse::skipped("No significant new content");
                    }

                    info!("OCR appending {} chars to: {}", new_text.len(), source_path);
                    
                    // Chunk only the new content
                    let new_chunks = state.chunker.chunk(&new_text);
                    
                    // Compute hash of combined content
                    let combined_content = format!("{}\n\n{}", existing_content, new_text);
                    let combined_hash = compute_hash(&combined_content);

                    match state.storage.append_to_source(
                        &existing.ehl_doc_id,
                        &payload,
                        &new_text,
                        &combined_hash,
                        &new_chunks,
                    ) {
                        Ok(()) => {
                            state.cache.insert(
                                source_path.to_string(),
                                combined_hash,
                                existing.ehl_doc_id.clone(),
                            );
                            IngestionResponse::updated(existing.ehl_doc_id, new_chunks.len())
                        }
                        Err(e) => {
                            error!("Storage append error: {}", e);
                            IngestionResponse::error(&format!("Storage error: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get existing content: {}", e);
                    IngestionResponse::error(&format!("Storage error: {}", e))
                }
            }
        }
        Ok(None) => {
            // No exact match - create new entry
            info!("New OCR content: {}", source_path);
            let ehl_doc_id = uuid::Uuid::new_v4().to_string();
            let chunks = state.chunker.chunk(&payload.content);

            let mut storage_payload = payload.clone();
            storage_payload.url = source_path.to_string();

            match state.storage.insert_source(&storage_payload, content_hash, &ehl_doc_id, &chunks) {
                Ok(_) => {
                    state.cache.insert(
                        source_path.to_string(),
                        content_hash.to_string(),
                        ehl_doc_id.clone(),
                    );
                    IngestionResponse::created(ehl_doc_id, chunks.len())
                }
                Err(e) => {
                    error!("Storage insert error: {}", e);
                    IngestionResponse::error(&format!("Storage error: {}", e))
                }
            }
        }
        Err(e) => {
            error!("Storage query error: {}", e);
            IngestionResponse::error(&format!("Storage error: {}", e))
        }
    }
}

/// Extract new content that doesn't exist in the existing content
/// Uses word-level comparison to find genuinely new text
fn extract_new_content(existing: &str, incoming: &str) -> String {
    // Split into sentences/lines for comparison
    let existing_lines: std::collections::HashSet<&str> = existing
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.len() > 10)
        .collect();
    
    let existing_words: std::collections::HashSet<&str> = existing
        .split_whitespace()
        .collect();
    
    // Find lines in incoming that are genuinely new
    let mut new_lines: Vec<&str> = Vec::new();
    
    for line in incoming.lines() {
        let trimmed = line.trim();
        if trimmed.len() < 10 {
            continue;
        }
        
        // Check if this exact line exists
        if existing_lines.contains(trimmed) {
            continue;
        }
        
        // Check word overlap - if >80% of words already exist, skip
        let line_words: Vec<&str> = trimmed.split_whitespace().collect();
        if line_words.is_empty() {
            continue;
        }
        
        let overlap_count = line_words.iter()
            .filter(|w| existing_words.contains(*w))
            .count();
        
        let overlap_ratio = overlap_count as f64 / line_words.len() as f64;
        
        // Only include if less than 80% overlap
        if overlap_ratio < 0.8 {
            new_lines.push(trimmed);
        }
    }
    
    new_lines.join("\n")
}

/// Normalize source path to create a canonical identifier
/// This handles URLs with varying query parameters (e.g., Google Docs, Jira)
fn normalize_source_path(source: &str, url: &str) -> String {
    match source {
        "gdocs" => {
            // Extract document ID and create canonical URL
            // Pattern: /document/d/DOC_ID/...
            if let Some(caps) = regex::Regex::new(r"/document/d/([a-zA-Z0-9_-]+)")
                .ok()
                .and_then(|re| re.captures(url))
            {
                if let Some(doc_id) = caps.get(1) {
                    return format!("gdocs://{}", doc_id.as_str());
                }
            }
            url.to_string()
        }
        "gsheets" => {
            // Extract spreadsheet ID and create canonical URL
            // Pattern: /spreadsheets/d/SPREADSHEET_ID/...
            if let Some(caps) = regex::Regex::new(r"/spreadsheets/d/([a-zA-Z0-9_-]+)")
                .ok()
                .and_then(|re| re.captures(url))
            {
                if let Some(sheet_id) = caps.get(1) {
                    return format!("gsheets://{}", sheet_id.as_str());
                }
            }
            url.to_string()
        }
        "gslides" => {
            // Extract presentation ID and create canonical URL
            // Pattern: /presentation/d/PRESENTATION_ID/...
            if let Some(caps) = regex::Regex::new(r"/presentation/d/([a-zA-Z0-9_-]+)")
                .ok()
                .and_then(|re| re.captures(url))
            {
                if let Some(pres_id) = caps.get(1) {
                    return format!("gslides://{}", pres_id.as_str());
                }
            }
            url.to_string()
        }
        "gemini" => {
            // Gemini conversations use canonical URLs like gemini://conversation/ID
            // The content script already provides canonical URLs, so just use them
            // But also handle raw URLs if they come through
            if url.starts_with("gemini://") {
                return url.to_string();
            }
            // Extract conversation ID from raw URL
            if let Some(caps) = regex::Regex::new(r"/(?:app|c)/([a-zA-Z0-9_-]+)")
                .ok()
                .and_then(|re| re.captures(url))
            {
                if let Some(conv_id) = caps.get(1) {
                    return format!("gemini://conversation/{}", conv_id.as_str());
                }
            }
            url.to_string()
        }
        "google-ai" | "google-search" => {
            // Google AI Mode and Search use canonical URLs based on query
            // The content script already provides canonical URLs like google-ai://search/query
            if url.starts_with("google-ai://") || url.starts_with("google://") {
                return url.to_string();
            }
            // Extract query from raw URL and normalize
            if let Ok(parsed) = url::Url::parse(url) {
                if let Some(query) = parsed.query_pairs()
                    .find(|(k, _)| k == "q")
                    .map(|(_, v)| v.to_string())
                {
                    let normalized_query = query.to_lowercase().trim().to_string();
                    let prefix = if source == "google-ai" { "google-ai" } else { "google" };
                    return format!("{}://search/{}", prefix, urlencoding::encode(&normalized_query));
                }
            }
            url.to_string()
        }
        "jira" => {
            // Extract issue key or board ID for canonical path
            // Pattern: selectedIssue=PROJ-123 or /browse/PROJ-123
            if let Ok(parsed) = url::Url::parse(url) {
                // Check for selectedIssue query param
                if let Some(issue) = parsed.query_pairs()
                    .find(|(k, _)| k == "selectedIssue")
                    .map(|(_, v)| v.to_string())
                {
                    let host = parsed.host_str().unwrap_or("jira");
                    return format!("jira://{}:{}", host, issue);
                }
                
                // Check for /browse/PROJ-123 pattern
                if let Some(caps) = regex::Regex::new(r"/browse/([A-Z][A-Z0-9]+-\d+)")
                    .ok()
                    .and_then(|re| re.captures(parsed.path()))
                {
                    if let Some(issue) = caps.get(1) {
                        let host = parsed.host_str().unwrap_or("jira");
                        return format!("jira://{}:{}", host, issue.as_str());
                    }
                }
            }
            url.to_string()
        }
        "slack" => {
            // Normalize Slack URLs to channel + thread
            if let Ok(parsed) = url::Url::parse(url) {
                let path = parsed.path();
                // Pattern: /archives/CHANNEL_ID/pTIMESTAMP
                if path.contains("/archives/") {
                    let host = parsed.host_str().unwrap_or("slack");
                    return format!("slack://{}:{}", host, path);
                }
            }
            url.to_string()
        }
        "teams" => {
            // Normalize Teams URLs - accessibility:// URLs are already canonical
            if url.starts_with("accessibility://") {
                return url.to_string();
            }
            // For web URLs, extract conversation/channel ID
            if let Ok(parsed) = url::Url::parse(url) {
                let path = parsed.path();
                // Pattern: /conversations/CONVERSATION_ID or /channel/CHANNEL_ID
                if path.contains("/conversations/") || path.contains("/channel/") {
                    let host = parsed.host_str().unwrap_or("teams");
                    return format!("teams://{}:{}", host, path);
                }
            }
            url.to_string()
        }
        _ => {
            // For other sources, strip query params and fragments
            if let Ok(mut parsed) = url::Url::parse(url) {
                parsed.set_query(None);
                parsed.set_fragment(None);
                parsed.to_string()
            } else {
                url.to_string()
            }
        }
    }
}
