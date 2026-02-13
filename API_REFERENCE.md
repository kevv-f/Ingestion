# Content Ingestion Pipeline - API Reference

This document provides API reference documentation for integrating the content ingestion pipeline as a sidecar to a Tauri application. It covers entry points, data formats, deduplication logic, and chunking strategies.

## Table of Contents

1. [Overview](#overview)
2. [Entry Points](#entry-points)
3. [Data Formats](#data-formats)
4. [Deduplication Logic](#deduplication-logic)
5. [Chunking Strategy](#chunking-strategy)
6. [Extractor Types](#extractor-types)
7. [Configuration](#configuration)

---

## Overview

The content ingestion pipeline consists of two main components that can be used as sidecars:

| Component | Binary | Purpose |
|-----------|--------|---------|
| **Unified Router** | `ingestion` | Orchestrates extraction from desktop apps, routes to appropriate extractor |
| **Ingestion Service** | `ingestion-server` | Receives payloads, deduplicates, chunks, and stores content |

### Architecture Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           UNIFIED ROUTER                                 │
│  - Window tracking across all displays                                   │
│  - Perceptual hash change detection                                      │
│  - Routes to: Accessibility, Chrome Extension, or OCR extractor          │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │ CapturePayload (JSON)
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         INGESTION SERVICE                                │
│  - Unix socket listener (/tmp/clace-ingestion.sock)                      │
│  - In-memory dedup cache + SQLite dedup                                  │
│  - Content chunking (1024 tokens, 100 overlap)                           │
│  - SQLite storage (content_sources + chunks tables)                      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Entry Points

### 1. Unified Router Binary (`ingestion`)

The main entry point for content extraction orchestration.

**Location:** `unified-router/target/release/ingestion`

**Usage:**
```bash
# Start with defaults
./ingestion

# With custom options
./ingestion --interval 10 --no-ocr --socket /tmp/custom.sock
```

**Command Line Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--socket <PATH>` | Unix socket path | `/tmp/clace-ingestion.sock` |
| `--db <PATH>` | Database path | `~/Library/Application Support/clace-ingestion/content.db` |
| `--interval <SECS>` | Base capture interval | `5` |
| `--no-accessibility` | Disable accessibility extraction | enabled |
| `--no-ocr` | Disable OCR extraction | enabled |
| `--no-chrome` | Disable Chrome extension support | enabled |
| `-c, --config <PATH>` | Path to configuration file | auto-detected |

**Required Permissions (macOS):**
- Accessibility: System Settings > Privacy & Security > Accessibility
- Screen Recording: System Settings > Privacy & Security > Screen Recording

### 2. Ingestion Service Binary (`ingestion-server`)

Standalone server for receiving and processing content payloads.

**Location:** `ingestion-service/target/release/ingestion-server`

**Usage:**
```bash
./ingestion-server
```

The server listens on `/tmp/clace-ingestion.sock` by default.

### 3. Library Integration (Rust)

For direct integration into a Tauri application without spawning separate processes:

```rust
use ingestion_service::{IngestionServer, ServerConfig, CapturePayload};

// Configure the server
let config = ServerConfig {
    socket_path: "/tmp/clace-ingestion.sock".into(),
    db_path: "path/to/your/content.db".into(),
};

// Create and run the server
let server = IngestionServer::new(config)?;

// Option 1: Run as socket server (background task)
tokio::spawn(async move {
    server.run().await
});

// Option 2: Process payloads directly (no socket)
let payload = CapturePayload {
    source: "word".to_string(),
    url: "accessibility://Microsoft_Word/Document.docx".to_string(),
    content: "Document content here...".to_string(),
    title: Some("Document.docx".to_string()),
    author: None,
    channel: None,
    timestamp: Some(chrono::Utc::now().timestamp()),
    app_name: Some("Microsoft Word".to_string()),
    bundle_id: Some("com.microsoft.Word".to_string()),
};

let response = server.process(payload).await;
```

---

## Data Formats

### CapturePayload (Input)

The unified payload format sent to the ingestion service.

```json
{
  "source": "string",           // Required: Source identifier
  "url": "string",              // Required: Location identifier (URL or accessibility:// path)
  "content": "string",          // Required: The text content to ingest
  "title": "string | null",     // Optional: Document title
  "author": "string | null",    // Optional: Author/sender
  "channel": "string | null",   // Optional: Channel/project/workspace
  "timestamp": "number | null", // Optional: Unix timestamp in seconds
  "app_name": "string | null",  // Optional: Application display name
  "bundle_id": "string | null"  // Optional: Application bundle ID
}
```

**Source Values:**

| Source | Origin | URL Format |
|--------|--------|------------|
| `word` | Microsoft Word | `accessibility://Microsoft_Word/{title}` |
| `excel` | Microsoft Excel | `accessibility://Microsoft_Excel/{title}` |
| `powerpoint` | Microsoft PowerPoint | `accessibility://Microsoft_Powerpoint/{title}` |
| `outlook` | Microsoft Outlook | `accessibility://Microsoft_Outlook/{title}` |
| `teams` | Microsoft Teams | `accessibility://Microsoft_Teams/{title}` |
| `pages` | Apple Pages | `accessibility://Pages/{title}` |
| `numbers` | Apple Numbers | `accessibility://Numbers/{title}` |
| `keynote` | Apple Keynote | `accessibility://Keynote/{title}` |
| `slack` | Slack (browser/desktop) | `slack://{workspace}:/archives/{channel}` |
| `gmail` | Gmail | `https://mail.google.com/...` |
| `gdocs` | Google Docs | `gdocs://{DOC_ID}` |
| `gsheets` | Google Sheets | `gsheets://{SPREADSHEET_ID}` |
| `gslides` | Google Slides | `gslides://{PRESENTATION_ID}` |
| `jira` | Jira | `jira://{host}:{ISSUE_KEY}` |
| `chrome` | Generic web page | Original URL |
| `ocr` | OCR extraction | `ocr://{bundle_id}/{title}/{content_hash}` |

### IngestionResponse (Output)

Response returned after processing a payload.

```json
{
  "status": "ok | error",
  "action": "created | updated | skipped | failed",
  "ehl_doc_id": "string | null",   // UUID of the document (if created/updated)
  "chunk_count": "number | null",  // Number of chunks created
  "message": "string | null"       // Error or skip reason
}
```

**Response Status Codes:**

| Status | Action | Meaning |
|--------|--------|---------|
| `ok` | `created` | New content stored successfully |
| `ok` | `updated` | Existing content updated with new version |
| `ok` | `skipped` | Duplicate content, no action taken |
| `error` | `failed` | Processing failed (see message) |

### ExtractedContent (Internal)

Internal representation used by extractors before conversion to CapturePayload.

```rust
pub struct ExtractedContent {
    pub source: String,              // Source identifier (e.g., "word", "ocr")
    pub title: Option<String>,       // Document title from window
    pub content: String,             // Extracted text content
    pub app_name: String,            // Full application name
    pub bundle_id: Option<String>,   // Application bundle ID
    pub url: Option<String>,         // URL (for web content)
    pub timestamp: i64,              // Unix timestamp
    pub extraction_method: String,   // "accessibility", "chrome_extension", or "ocr"
    pub confidence: Option<f32>,     // Confidence score (0.0-1.0, mainly for OCR)
}
```

---

## Deduplication Logic

The ingestion service uses a two-tier deduplication strategy to efficiently detect and handle duplicate content.

### Tier 1: In-Memory Cache Deduplication (No SQL)

The first tier uses an in-memory LRU cache for fast duplicate detection without database queries.

**Implementation:** `ingestion-service/src/dedup.rs`

```rust
pub struct DedupCache {
    entries: HashMap<String, CacheEntry>,  // source_path → (content_hash, ehl_doc_id, last_seen)
    ttl: Duration,                          // Default: 24 hours
    max_entries: usize,                     // Default: 10,000 entries
}

pub enum DedupResult {
    New,                    // Content not in cache
    Duplicate(String),      // Same content hash, returns ehl_doc_id
    Changed(String),        // Different content hash, returns ehl_doc_id
}
```

**Cache Check Flow:**

1. Compute SHA-256 hash of incoming content
2. Normalize source path (URL) to canonical form
3. Look up source path in cache:
   - **Cache hit + same hash** → Return `Duplicate`, skip processing
   - **Cache hit + different hash** → Return `Changed`, update content
   - **Cache miss** → Return `New`, proceed to Tier 2

**Content Hash Computation:**

```rust
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### Source Path Normalization

Different sources have their URLs normalized to canonical forms for consistent deduplication:

| Source | Raw URL Example | Normalized Path |
|--------|-----------------|-----------------|
| `gdocs` | `https://docs.google.com/document/d/ABC123/edit?tab=...` | `gdocs://ABC123` |
| `gsheets` | `https://docs.google.com/spreadsheets/d/XYZ789/edit#gid=0` | `gsheets://XYZ789` |
| `gslides` | `https://docs.google.com/presentation/d/DEF456/edit` | `gslides://DEF456` |
| `jira` | `https://company.atlassian.net/browse/PROJ-123` | `jira://company.atlassian.net:PROJ-123` |
| `slack` | `https://workspace.slack.com/archives/C123/p456` | `slack://workspace.slack.com:/archives/C123/p456` |
| `gemini` | `https://gemini.google.com/app/abc123` | `gemini://conversation/abc123` |
| `teams` | `accessibility://Microsoft_Teams/Chat` | `accessibility://Microsoft_Teams/Chat` |
| Other | `https://example.com/page?query=1#section` | `https://example.com/page` (query/fragment stripped) |

**Normalization Code:**

```rust
fn normalize_source_path(source: &str, url: &str) -> String {
    match source {
        "gdocs" => {
            // Extract document ID: /document/d/DOC_ID/...
            if let Some(caps) = regex::Regex::new(r"/document/d/([a-zA-Z0-9_-]+)")
                .ok().and_then(|re| re.captures(url)) {
                return format!("gdocs://{}", caps.get(1).unwrap().as_str());
            }
            url.to_string()
        }
        // ... similar for other sources
        _ => {
            // Strip query params and fragments
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
```

### Unified Router Level Deduplication (No SQL)

The unified router performs additional deduplication before sending to the ingestion service:

**1. Perceptual Hash Change Detection**

Uses average hash (aHash) algorithm to detect visual changes in windows without expensive pixel comparison.

```rust
pub fn compute_ahash(image: &DynamicImage) -> u64 {
    // 1. Resize to 8x8
    let resized = image.resize_exact(8, 8, FilterType::Nearest);
    // 2. Convert to grayscale
    let gray = resized.to_luma8();
    // 3. Calculate average brightness
    let avg: u8 = gray.pixels().map(|p| p.0[0] as u32).sum::<u32>() as u8 / 64;
    // 4. Build 64-bit hash: bit=1 if pixel > average
    let mut hash: u64 = 0;
    for (i, pixel) in gray.pixels().enumerate().take(64) {
        if pixel.0[0] > avg { hash |= 1 << i; }
    }
    hash
}

pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()  // Number of differing bits (0-64)
}
```

**Change Detection Flow:**

```
Window Screenshot → Compute aHash → Compare with stored hash
                                          │
                    ┌─────────────────────┴─────────────────────┐
                    │                                           │
            distance < threshold                        distance >= threshold
            (default: 8 bits)                           (content changed)
                    │                                           │
                    ▼                                           ▼
               SKIP extraction                          TRIGGER extraction
```

**2. Content Hash Deduplication**

After extraction, the router computes SHA-256 of the content and compares with the last known hash for that window:

```rust
// In router.rs - handle_extracted_content()
let mut hasher = Sha256::new();
hasher.update(content.content.as_bytes());
let hash = format!("{:x}", hasher.finalize());

// Skip if content hasn't changed
if state.last_content_hash.as_ref() == Some(&hash) {
    debug!("Content unchanged for {}, skipping", window_id);
    return;
}
state.last_content_hash = Some(hash);
```

**3. OCR Content-Based URL Generation**

For OCR extractions, the URL includes a content hash to differentiate different content with the same window title (e.g., different chat conversations in Claude):

```rust
// In types.rs - CapturePayload::from(ExtractedContent)
if content.extraction_method == "ocr" {
    let content_hash = compute_short_hash(&content.content);  // First 12 hex chars of SHA-256
    format!("ocr://{}/{}/{}", app_identifier, encoded_title, content_hash)
} else {
    format!("accessibility://{}/{}", app_identifier, encoded_title)
}
```

This ensures that:
- Same app + same title + same content → Same URL → Deduplicated
- Same app + same title + different content → Different URL → Stored separately

### Tier 2: SQL Database Deduplication

When the in-memory cache misses, the ingestion service queries the SQLite database.

**Database Schema:**

```sql
-- content_sources table (tracks what we've ingested)
CREATE TABLE content_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT NOT NULL,           -- "slack", "word", "gdocs", etc.
    source_path TEXT NOT NULL UNIQUE,    -- Normalized URL/path
    content_hash TEXT NOT NULL,          -- SHA-256 of content
    ehl_doc_id TEXT NOT NULL UNIQUE,     -- UUID for this document
    chunk_count INTEGER NOT NULL DEFAULT 0,
    ingestion_status TEXT NOT NULL DEFAULT 'ingested',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_content_sources_path ON content_sources(source_path);
CREATE INDEX idx_content_sources_hash ON content_sources(content_hash);
```

**SQL Dedup Flow:**

```rust
// In server.rs - process_payload()
match state.storage.find_source_by_path(&source_path) {
    Ok(Some(existing)) => {
        if existing.content_hash == content_hash {
            // Same content - update cache and skip
            state.cache.insert(source_path, content_hash, existing.ehl_doc_id.clone());
            return IngestionResponse::skipped("Content unchanged (db)");
        }
        // Content changed - update
        let chunks = state.chunker.chunk(&payload.content);
        state.storage.update_source(&existing.ehl_doc_id, &payload, &content_hash, &chunks)?;
        state.cache.insert(source_path, content_hash, existing.ehl_doc_id.clone());
        IngestionResponse::updated(existing.ehl_doc_id, chunks.len())
    }
    Ok(None) => {
        // New content - insert
        let ehl_doc_id = uuid::Uuid::new_v4().to_string();
        let chunks = state.chunker.chunk(&payload.content);
        state.storage.insert_source(&payload, &content_hash, &ehl_doc_id, &chunks)?;
        state.cache.insert(source_path, content_hash, ehl_doc_id.clone());
        IngestionResponse::created(ehl_doc_id, chunks.len())
    }
    Err(e) => IngestionResponse::error(&format!("Storage error: {}", e))
}
```

### OCR Incremental Content Appending

For OCR sources, the service supports incremental content appending to avoid re-storing entire documents when only new content is added:

```rust
fn process_ocr_payload(state: &mut ServiceState, payload: CapturePayload, 
                       source_path: &str, content_hash: &str) -> IngestionResponse {
    match state.storage.find_source_by_path(source_path) {
        Ok(Some(existing)) => {
            if existing.content_hash == content_hash {
                return IngestionResponse::skipped("Content unchanged");
            }
            
            // Get existing content and find genuinely new text
            let existing_content = state.storage.get_source_content(&existing.ehl_doc_id)?;
            let new_text = extract_new_content(&existing_content, &payload.content);
            
            if new_text.is_empty() || new_text.len() < 50 {
                return IngestionResponse::skipped("No significant new content");
            }
            
            // Chunk only the new content and append
            let new_chunks = state.chunker.chunk(&new_text);
            state.storage.append_to_source(&existing.ehl_doc_id, &payload, 
                                           &new_text, &combined_hash, &new_chunks)?;
            IngestionResponse::updated(existing.ehl_doc_id, new_chunks.len())
        }
        Ok(None) => { /* Insert new */ }
    }
}

fn extract_new_content(existing: &str, incoming: &str) -> String {
    let existing_lines: HashSet<&str> = existing.lines()
        .map(|l| l.trim()).filter(|l| l.len() > 10).collect();
    let existing_words: HashSet<&str> = existing.split_whitespace().collect();
    
    incoming.lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.len() < 10 || existing_lines.contains(trimmed) { return false; }
            
            // Check word overlap - skip if >80% of words already exist
            let line_words: Vec<&str> = trimmed.split_whitespace().collect();
            let overlap = line_words.iter().filter(|w| existing_words.contains(*w)).count();
            (overlap as f64 / line_words.len() as f64) < 0.8
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Chunking Strategy

The ingestion service splits content into fixed-size chunks for efficient storage and vector search.

**Implementation:** `ingestion-service/src/chunker.rs`

### Configuration

```rust
pub struct ChunkerConfig {
    pub max_tokens: usize,      // Default: 1024 words per chunk
    pub overlap_tokens: usize,  // Default: 100 words overlap between chunks
}
```

### Chunking Algorithm

```
Input Content
      │
      ▼
┌─────────────────┐
│ Is Tabular?     │  (Multiple lines with tabs)
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
   Yes        No
    │         │
    ▼         ▼
Line-Based   Word-Based
Chunking     Chunking
    │         │
    └────┬────┘
         │
         ▼
   Set Metadata
   (chunk_index, total_chunks)
         │
         ▼
   Return Vec<Chunk>
```

**Word-Based Chunking (Default):**

```rust
fn chunk_text(&self, content: &str) -> Vec<Chunk> {
    let words: Vec<&str> = content.split_whitespace().collect();
    
    // If content fits in one chunk, return as-is (preserve formatting)
    if words.len() <= self.config.max_tokens {
        return vec![Chunk {
            text: content.to_string(),
            chunk_index: 0,
            total_chunks: 1,
            token_count: words.len(),
        }];
    }
    
    let mut chunks = Vec::new();
    let mut start = 0;
    let step = self.config.max_tokens - self.config.overlap_tokens;  // 924 words
    
    while start < words.len() {
        let end = (start + self.config.max_tokens).min(words.len());
        let chunk_words = &words[start..end];
        
        chunks.push(Chunk {
            text: chunk_words.join(" "),
            chunk_index: chunks.len(),
            total_chunks: 0,  // Set after loop
            token_count: chunk_words.len(),
        });
        
        start += step;
    }
    
    // Set total_chunks
    let total = chunks.len();
    for chunk in &mut chunks { chunk.total_chunks = total; }
    chunks
}
```

**Tabular Content Detection:**

```rust
fn is_tabular_content(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().take(10).collect();
    if lines.len() < 2 { return false; }
    
    // Check if multiple lines have tabs
    let lines_with_tabs = lines.iter().filter(|l| l.contains('\t')).count();
    lines_with_tabs >= 2
}
```

**Line-Based Chunking (for Excel/CSV):**

Preserves row structure by chunking at line boundaries:

```rust
fn chunk_tabular(&self, content: &str) -> Vec<Chunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_lines = Vec::new();
    let mut current_tokens = 0;
    
    for line in lines {
        let line_tokens = line.split_whitespace().count().max(1);
        
        if current_tokens + line_tokens > self.config.max_tokens && !current_lines.is_empty() {
            // Start new chunk, keep 3 lines overlap for context
            chunks.push(Chunk { text: current_lines.join("\n"), ... });
            let overlap_lines = current_lines.len().min(3);
            current_lines = current_lines.split_off(current_lines.len() - overlap_lines);
            current_tokens = /* recalculate */;
        }
        
        current_lines.push(line);
        current_tokens += line_tokens;
    }
    
    // Add remaining
    if !current_lines.is_empty() {
        chunks.push(Chunk { text: current_lines.join("\n"), ... });
    }
    chunks
}
```

### Chunk Storage

Chunks are stored in the `chunks` table with JSON metadata:

```sql
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vector_index INTEGER,              -- Index for vector search
    text TEXT NOT NULL,                -- Chunk text content
    meta TEXT NOT NULL,                -- JSON metadata (ChunkMeta)
    is_deleted INTEGER DEFAULT 0,      -- Soft delete flag
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**ChunkMeta JSON Structure:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",  // ehl_doc_id (UUID)
  "source": "word",                               // Source type
  "url": "accessibility://Microsoft_Word/Doc.docx",
  "title": "Doc.docx",
  "author": null,
  "channel": null,
  "chunk_index": 0,                               // 0-based index
  "total_chunks": 3,                              // Total chunks for document
  "source_type": "capture",                       // Always "capture"
  "app_name": "Microsoft Word",                   // Display name
  "bundle_id": "com.microsoft.Word"               // Bundle ID for icon lookup
}
```

**Update Flow (Soft Delete + Insert):**

When content changes, old chunks are soft-deleted and new chunks inserted:

```rust
pub fn update_source(&mut self, ehl_doc_id: &str, payload: &CapturePayload,
                     content_hash: &str, chunks: &[Chunk]) -> Result<(), StorageError> {
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
        let meta = ChunkMeta { /* ... */ };
        let meta_json = serde_json::to_string(&meta)?;
        tx.execute("INSERT INTO chunks (text, meta) VALUES (?1, ?2)",
                   params![chunk.text, meta_json])?;
    }
    
    tx.commit()?;
    Ok(())
}
```

---

## Extractor Types

The unified router supports three extraction methods, selected based on application type.

### 1. Accessibility Extractor

For applications with good accessibility API support.

**Supported Applications:**

| Application | Bundle ID | Source |
|-------------|-----------|--------|
| Microsoft Word | `com.microsoft.Word` | `word` |
| Microsoft Excel | `com.microsoft.Excel` | `excel` |
| Microsoft PowerPoint | `com.microsoft.Powerpoint` | `powerpoint` |
| Microsoft Outlook | `com.microsoft.Outlook` | `outlook` |
| Microsoft Teams | `com.microsoft.teams2` | `teams` |
| Apple Pages | `com.apple.iWork.Pages` | `pages` |
| Apple Numbers | `com.apple.iWork.Numbers` | `numbers` |
| Apple Keynote | `com.apple.iWork.Keynote` | `keynote` |
| Apple TextEdit | `com.apple.TextEdit` | `textedit` |
| Apple Notes | `com.apple.Notes` | `notes` |
| Slack | `com.tinyspeck.slackmacgap` | `slack` |

**Binary:** `accessibility-extractor/target/release/ax-extractor`

**CLI Usage:**
```bash
# Extract from specific app
./ax-extractor --app com.microsoft.Word

# Output is JSON:
{
  "source": "word",
  "title": "Document.docx",
  "content": "...",
  "app_name": "Microsoft Word",
  "timestamp": 1707500000,
  "extraction_method": "accessibility"
}
```

### 2. Chrome Extension

For Chromium-based browsers with the extension installed.

**Supported Browsers:**

| Browser | Bundle ID |
|---------|-----------|
| Google Chrome | `com.google.Chrome` |
| Brave | `com.brave.Browser` |
| Microsoft Edge | `com.microsoft.edgemac` |
| Vivaldi | `com.vivaldi.Vivaldi` |
| Opera | `com.operasoftware.Opera` |
| Arc | `com.arc.browser` |

**Communication Protocol:** Chrome Native Messaging (stdin/stdout with 4-byte length prefix)

```rust
// Read message from Chrome
fn read_message() -> io::Result<ChromeMessage> {
    let mut len_bytes = [0u8; 4];
    io::stdin().read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    let mut buffer = vec![0u8; len];
    io::stdin().read_exact(&mut buffer)?;
    
    serde_json::from_slice(&buffer)
}

// Write response to Chrome
fn write_response(response: &ChromeResponse) -> io::Result<()> {
    let json = serde_json::to_vec(response)?;
    let len = (json.len() as u32).to_le_bytes();
    
    io::stdout().write_all(&len)?;
    io::stdout().write_all(&json)?;
    io::stdout().flush()
}
```

### 3. OCR Extractor

Fallback for applications without accessibility support.

**Binary:** `ocr-extractor/.build/release/ocr-extractor` (Swift)

**CLI Usage:**
```bash
# Extract from window by ID
./ocr-extractor --window-id 12345 --json

# Capture only (for change detection)
./ocr-extractor --window-id 12345 --capture-only --output /tmp/capture.png --json

# Extract from image file
./ocr-extractor --image /path/to/screenshot.png --json
```

**Output:**
```json
{
  "text": "Extracted text content...",
  "confidence": 0.95,
  "captured": true
}
```

### Extractor Selection Logic

```rust
pub fn get_extractor_type(&self, bundle_id: &str) -> ExtractorType {
    // Priority 1: Chrome extension for supported browsers
    if self.chrome_browsers.contains(bundle_id) {
        return ExtractorType::Chrome;
    }
    
    // Priority 2: Accessibility for supported apps
    if self.accessibility_apps.contains(bundle_id) {
        return ExtractorType::Accessibility;
    }
    
    // Priority 3: OCR fallback
    ExtractorType::Ocr
}
```

---

## Configuration

### Router Configuration File

**Location:** `~/.config/unified-router/config.toml`

```toml
[general]
enabled = true
log_level = "info"

[timing]
base_interval_seconds = 5      # Capture interval (AC power)
battery_interval_seconds = 15  # Capture interval (battery)
idle_interval_seconds = 60     # Capture interval (user idle)
min_interval_seconds = 3       # Minimum between extractions
max_interval_seconds = 60      # Maximum forced extraction interval

[change_detection]
hash_sensitivity = 8           # Hamming distance threshold (0-64)
title_change_triggers_extract = true

[extractors]
accessibility_enabled = true
chrome_extension_enabled = true
ocr_enabled = true
ocr_fallback_only = true       # Only use OCR when others unavailable

[privacy]
blocked_apps = [
    "com.1password.*",
    "com.agilebits.onepassword*",
    "com.lastpass.LastPass",
    "com.bitwarden.desktop",
    "*banking*",
    "*bank*"
]
redact_credit_cards = true
redact_ssn = true
redact_api_keys = true
redact_emails = false
redact_phone_numbers = false

[multi_display]
enabled = true
capture_all_displays = true
```

### Always-Blacklisted Applications

These applications are always blocked and cannot be unblocked:

```rust
pub const ALWAYS_BLACKLISTED_APPS: &[&str] = &[
    "com.ehl.viewer-app",                    // Our own viewer app
    "com.tauri.dev",                         // Tauri dev mode
    "dev.kiro.app",                          // Kiro IDE
    "com.amazon.kiro",
    "com.blackmagic-design.DaVinciResolve",  // Video editing
    "com.blackmagic-design.DaVinciResolveLite",
];

pub const ALWAYS_BLACKLISTED_PATTERNS: &[&str] = &[
    "*viewer-app*",
    "*viewer_app*",
    "*kiro*",
    "*DaVinciResolve*",
];
```

### Privacy Redaction

PII is automatically redacted before storage:

| Pattern | Replacement |
|---------|-------------|
| Credit card numbers (Luhn-validated) | `[REDACTED_CARD]` |
| SSN (XXX-XX-XXXX) | `[REDACTED_SSN]` |
| API keys/tokens | `[REDACTED_KEY]` |
| AWS access keys | `[REDACTED_AWS_KEY]` |
| Email addresses (optional) | `[REDACTED_EMAIL]` |
| Phone numbers (optional) | `[REDACTED_PHONE]` |

---

## Unix Socket Communication

### Protocol

The ingestion service uses newline-delimited JSON over Unix domain socket:

**Request:** `{JSON payload}\n`
**Response:** `{JSON response}\n`

**Socket Path:** `/tmp/clace-ingestion.sock`

### Client Example (Rust)

```rust
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

async fn send_to_ingestion(payload: &CapturePayload) -> Result<IngestionResponse, Error> {
    let stream = UnixStream::connect("/tmp/clace-ingestion.sock").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    // Send payload
    let json = serde_json::to_string(payload)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    
    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;
    
    serde_json::from_str(&response_line)
}
```

---

## Error Handling

### Extraction Errors

```rust
pub enum ExtractionError {
    WindowNotFound(WindowId),
    AppNotFound(String),
    ExtractionFailed(String),
    PermissionDenied(String),
    Blocked,                    // Privacy filter blocked
    NoContent,
    Io(std::io::Error),
}
```

### Graceful Degradation

| Failure | Fallback |
|---------|----------|
| Accessibility extractor unavailable | Use OCR |
| Chrome extension not responding | Use OCR for browser windows |
| OCR extractor unavailable | Skip extraction, log warning |
| Ingestion service unavailable | Queue locally, retry later |

---

## Performance Characteristics

| Operation | Typical Latency |
|-----------|-----------------|
| Perceptual hash computation | ~1ms |
| Accessibility extraction | 50-200ms |
| OCR extraction | 150-500ms |
| Dedup check (cache hit) | <1ms |
| Dedup check (DB query) | 1-5ms |
| Chunking | 1-10ms |
| SQLite insert (with transaction) | 5-20ms |
