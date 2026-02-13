# Content Ingestion Pipeline - Technical Architecture Documentation

## Executive Summary

This project is a multi-component content ingestion system designed to capture, deduplicate, chunk, and store content from both web browsers and native desktop applications. The system enables a unified content repository for downstream processing (e.g., search, RAG, AI assistants).

The architecture consists of four main components:
1. **Chrome Extension** - Captures content from web pages
2. **Native Host** - Bridges Chrome extension to the ingestion service via native messaging
3. **Ingestion Service** - Processes, deduplicates, chunks, and stores content
4. **Accessibility Extractor** - Extracts content from native macOS applications

---

## System Architecture Overview

```mermaid
flowchart TB
    subgraph Browser["Browser (Chrome)"]
        CE[Chrome Extension]
        SW[Service Worker]
        CS[Content Scripts]
    end
    
    subgraph NativeLayer["Native Layer"]
        NH[Native Host]
        IS[Ingestion Service]
        AXD[AX Daemon]
    end
    
    subgraph Storage["Storage Layer"]
        DB[(SQLite Database)]
    end
    
    subgraph Apps["Desktop Applications"]
        Word[Microsoft Word]
        Excel[Microsoft Excel]
        Pages[Apple Pages]
        Slack[Slack]
        Others[Other Apps...]
    end
    
    CE --> SW
    SW --> CS
    CS --> SW
    SW -->|Native Messaging| NH
    NH -->|Unix Socket| IS
    IS --> DB
    
    AXD -->|Accessibility API| Apps
    AXD --> DB
```

---

## Component Details

### 1. Chrome Extension

**Purpose:** Captures content from web pages when users switch tabs or navigate.

**Trigger Events:**
- Tab activation (user switches tabs)
- Window focus change
- URL change within same tab (SPA navigation)
- Page load complete

**Supported Sources:**
| Source | Identifier | Extraction Method |
|--------|------------|-------------------|
| Slack | `slack` | DOM parsing with message containers |
| Gmail | `gmail` | DOM parsing (inbox list or email body) |
| Outlook | `outlook` | DOM parsing with role-based selectors |
| Jira | `jira` | REST API + DOM fallback |
| Google Docs | `gdocs` | Export API via service worker |
| Google Sheets | `gsheets` | CSV export via service worker |
| Google Slides | `gslides` | Text export via service worker |
| Gemini | `gemini` | DOM parsing for conversations |
| Google AI Mode | `google-ai` | DOM parsing for AI responses |
| Discord | `discord` | DOM parsing |
| Generic | `browser` | Readability.js extraction |

**Rate Limiting:**
- Standard debounce: 1 second
- Google products debounce: 5 seconds
- Google export cooldown: 10 seconds between ANY Google export
- Rate limit penalty: 30 seconds after 429 response

### 2. Native Host (ingestion-host)

**Purpose:** Thin relay between Chrome extension and ingestion service.

**Communication Protocol:**
- **Input:** Chrome Native Messaging (stdin with 4-byte length prefix)
- **Output:** Chrome Native Messaging (stdout with 4-byte length prefix)
- **Backend:** Unix socket to ingestion service

**Socket Path:** `/tmp/clace-ingestion.sock`

**Message Flow:**
```mermaid
sequenceDiagram
    participant CE as Chrome Extension
    participant NH as Native Host
    participant IS as Ingestion Service
    
    CE->>NH: Native Message (4-byte len + JSON)
    NH->>IS: Unix Socket (JSON + newline)
    IS->>IS: Process, Dedup, Chunk, Store
    IS->>NH: Response (JSON + newline)
    NH->>CE: Native Message (4-byte len + JSON)
```

### 3. Ingestion Service

**Purpose:** Core processing engine for content ingestion.

**Key Modules:**
- `server.rs` - Unix socket server
- `payload.rs` - Data structures
- `dedup.rs` - Deduplication logic
- `chunker.rs` - Content chunking
- `storage.rs` - SQLite persistence

### 4. Accessibility Extractor

**Purpose:** Extract content from native macOS desktop applications using the Accessibility API.

**Binaries:**
- `ax-extractor` - CLI tool for manual extraction
- `ax-daemon` - Background daemon for automatic extraction

**Supported Applications:**
| Application | Bundle ID | Source |
|-------------|-----------|--------|
| Microsoft Word | `com.microsoft.Word` | `word` |
| Microsoft Excel | `com.microsoft.Excel` | `excel` |
| Microsoft PowerPoint | `com.microsoft.Powerpoint` | `powerpoint` |
| Apple Pages | `com.apple.iWork.Pages` | `pages` |
| Apple Numbers | `com.apple.iWork.Numbers` | `numbers` |
| Apple Keynote | `com.apple.iWork.Keynote` | `keynote` |
| Slack | `com.tinyspeck.slackmacgap` | `slack` |
| Discord | `com.hnc.Discord` | `discord` |

**Extraction Methods:**
1. **Direct File Extraction** - Parses document files directly (Word, Excel, Pages, etc.)
2. **Accessibility API** - Traverses AXUIElement tree for text content
3. **Electron-specific** - Enables accessibility for Electron apps (Slack, Discord)

---

## Data Schemas

### CapturePayload (Input Schema)

This is the unified payload format used by both the Chrome extension and accessibility extractor.

```json
{
  "source": "string",      // Required: "slack" | "gmail" | "jira" | "gdocs" | "word" | etc.
  "url": "string",         // Required: Location identifier (URL or accessibility:// path)
  "content": "string",     // Required: The text content to ingest
  "title": "string",       // Optional: Document title/subject
  "author": "string",      // Optional: Author/sender
  "channel": "string",     // Optional: Channel/project/workspace
  "timestamp": "number"    // Optional: Unix timestamp in seconds
}
```

**Source Values:**
| Source | Origin |
|--------|--------|
| `slack` | Slack (browser or desktop) |
| `gmail` | Gmail |
| `outlook` | Outlook |
| `jira` | Jira |
| `gdocs` | Google Docs |
| `gsheets` | Google Sheets |
| `gslides` | Google Slides |
| `gemini` | Google Gemini |
| `google-ai` | Google AI Mode |
| `discord` | Discord |
| `browser` | Generic web page |
| `word` | Microsoft Word (desktop) |
| `excel` | Microsoft Excel (desktop) |
| `powerpoint` | Microsoft PowerPoint (desktop) |
| `pages` | Apple Pages (desktop) |
| `numbers` | Apple Numbers (desktop) |
| `keynote` | Apple Keynote (desktop) |

### IngestionResponse (Output Schema)

```json
{
  "status": "ok" | "error",
  "action": "created" | "updated" | "skipped" | "failed",
  "ehl_doc_id": "string",   // Optional: UUID of the document
  "chunk_count": "number",  // Optional: Number of chunks created
  "message": "string"       // Optional: Error or skip reason
}
```

### ExtractedContent (Accessibility Extractor Internal)

```json
{
  "source": "string",           // Application source identifier
  "title": "string | null",     // Document title from window
  "content": "string",          // Extracted text content
  "app_name": "string",         // Full application name
  "timestamp": "number",        // Unix timestamp
  "extraction_method": "accessibility"
}
```

### ChunkMeta (Storage Schema)

```json
{
  "id": "string",           // ehl_doc_id (UUID)
  "source": "string",       // Source type
  "url": "string",          // Source URL/path
  "title": "string | null", // Document title
  "author": "string | null",
  "channel": "string | null",
  "chunk_index": "number",  // 0-based chunk index
  "total_chunks": "number", // Total chunks for document
  "source_type": "string"   // "capture" or "accessibility"
}
```

---

## Database Schema

### content_sources Table
Tracks ingested content sources for deduplication.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Primary key |
| source_type | TEXT | Source identifier (slack, gmail, word, etc.) |
| source_path | TEXT | Normalized URL/path (UNIQUE) |
| content_hash | TEXT | SHA-256 hash of content |
| ehl_doc_id | TEXT | Document UUID (UNIQUE) |
| chunk_count | INTEGER | Number of chunks |
| ingestion_status | TEXT | Status (default: 'ingested') |
| created_at | TEXT | Creation timestamp |
| updated_at | TEXT | Last update timestamp |

### chunks Table
Stores actual content chunks.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Primary key |
| vector_index | INTEGER | Index for vector search |
| text | TEXT | Chunk text content |
| meta | TEXT | JSON metadata (ChunkMeta) |
| is_deleted | INTEGER | Soft delete flag |
| created_at | TEXT | Creation timestamp |

### messages Table (Slack-specific)
Message-level deduplication for Slack.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Primary key |
| source_url | TEXT | Slack channel URL |
| message_hash | TEXT | SHA-256 of message |
| message_text | TEXT | Message content |
| message_order | INTEGER | Time-based ordering |
| created_at | TEXT | Creation timestamp |

---

## Deduplication Logic

### Content-Level Deduplication (Default)

```mermaid
flowchart TD
    A[Receive Payload] --> B[Compute Content Hash]
    B --> C[Normalize Source Path]
    C --> D{Check In-Memory Cache}
    D -->|Cache Hit, Same Hash| E[Return: Skipped - Duplicate]
    D -->|Cache Hit, Different Hash| F[Update Content]
    D -->|Cache Miss| G{Check Database}
    G -->|DB Hit, Same Hash| H[Update Cache, Return: Skipped]
    G -->|DB Hit, Different Hash| I[Update Content]
    G -->|DB Miss| J[Insert New Content]
    F --> K[Soft-delete Old Chunks]
    I --> K
    K --> L[Chunk Content]
    J --> L
    L --> M[Insert New Chunks]
    M --> N[Update Cache]
    N --> O[Return Response]
```

### Source Path Normalization

Different sources have their URLs normalized to canonical forms:

| Source | Raw URL | Normalized Path |
|--------|---------|-----------------|
| gdocs | `https://docs.google.com/document/d/ABC123/edit?...` | `gdocs://ABC123` |
| gsheets | `https://docs.google.com/spreadsheets/d/XYZ789/...` | `gsheets://XYZ789` |
| jira | `https://company.atlassian.net/browse/PROJ-123` | `jira://company.atlassian.net:PROJ-123` |
| slack | `https://workspace.slack.com/archives/C123/p456` | `slack://workspace.slack.com:/archives/C123/p456` |
| accessibility | N/A | `accessibility://Microsoft_Word/Document.docx` |

### Message-Level Deduplication (Slack)

For Slack, individual messages are tracked to enable incremental updates:

1. Parse messages from content (format: `[Author] [Time] Message`)
2. Compute hash for each message
3. Check which messages already exist in `messages` table
4. Insert only new messages
5. Rebuild chunks from ALL messages for the channel

---

## Chunking Strategy

### Configuration
- **Max Tokens:** 1024 words per chunk
- **Overlap:** 100 words between chunks

### Chunking Algorithm

```mermaid
flowchart TD
    A[Input Content] --> B{Is Tabular?}
    B -->|Yes| C[Line-Based Chunking]
    B -->|No| D[Word-Based Chunking]
    C --> E[Preserve Row Structure]
    D --> F{Content <= Max Tokens?}
    F -->|Yes| G[Single Chunk]
    F -->|No| H[Split with Overlap]
    E --> I[Set Chunk Metadata]
    G --> I
    H --> I
    I --> J[Return Chunks]
```

### Tabular Detection
Content is considered tabular if:
- Multiple lines contain tab characters (`\t`)
- At least 2 of the first 10 lines have tabs

---

## Communication Protocols

### Chrome Native Messaging

**Message Format:**
```
[4-byte length (native endian)][JSON payload]
```

**Manifest Configuration:**
```json
{
  "name": "com.clace.extension",
  "description": "Clace content ingestion native messaging host",
  "path": "/path/to/ingestion-host",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://EXTENSION_ID/"]
}
```

### Unix Socket Communication

**Protocol:** Newline-delimited JSON
- Request: `{JSON payload}\n`
- Response: `{JSON response}\n`

**Socket Path:** `/tmp/clace-ingestion.sock`

**Timeout:** 5 seconds for read/write operations

---

## AX Daemon Operation

The accessibility daemon monitors application switches and extracts content automatically.

```mermaid
sequenceDiagram
    participant OS as macOS
    participant AXD as AX Daemon
    participant App as Desktop App
    participant DB as SQLite
    
    OS->>AXD: NSWorkspaceDidActivateApplicationNotification
    AXD->>AXD: Check if supported app
    AXD->>AXD: Schedule extraction (debounce)
    
    OS->>AXD: NSWorkspaceDidDeactivateApplicationNotification
    AXD->>App: Extract via Accessibility API
    App->>AXD: Content
    AXD->>AXD: Compute hash, check dedup
    AXD->>DB: Store if new/changed
```

**Debounce:** 2 seconds between extractions

---

## Dependencies & Libraries

### Accessibility Extractor (Rust)

| Crate | Version | Purpose |
|-------|---------|---------|
| `accessibility` | 0.2 | macOS Accessibility API bindings |
| `accessibility-sys` | 0.2 | Low-level AX bindings |
| `core-foundation` | 0.10 | Core Foundation types |
| `cocoa` | 0.26 | Cocoa framework bindings |
| `objc` | 0.2 | Objective-C runtime |
| `calamine` | 0.26 | Excel file parsing (xlsx, xls, xlsb, ods) |
| `docx-rs` | 0.4 | Word document parsing |
| `snap` | 1.1 | Snappy decompression (iWork files) |
| `rusqlite` | 0.31 | SQLite database |
| `sha2` | 0.10 | SHA-256 hashing |
| `uuid` | 1.0 | UUID generation |
| `serde` | 1.0 | Serialization |
| `chrono` | 0.4 | Date/time handling |
| `regex-lite` | 0.1 | Lightweight regex |

### Ingestion Service (Rust)

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.0 | Async runtime |
| `rusqlite` | 0.31 | SQLite database |
| `sha2` | 0.10 | SHA-256 hashing |
| `uuid` | 1.0 | UUID generation |
| `serde` | 1.0 | Serialization |
| `regex` | 1.10 | URL normalization |
| `url` | 2.5 | URL parsing |
| `tracing` | 0.1 | Logging |

### Native Host (Rust)

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` | 1.0 | Serialization |
| `serde_json` | 1.0 | JSON handling |

### Chrome Extension (JavaScript)

| Library | Purpose |
|---------|---------|
| `Readability.js` | Generic web page content extraction |

---

## File Locations

### Database
- **Browser content:** `~/Library/Application Support/clace-ingestion/content.db`
- **AX Daemon:** Same location

### Unix Socket
- `/tmp/clace-ingestion.sock`

### Native Host Manifest
- `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.clace.extension.json`

---

## Error Handling

### Extraction Errors

| Error | Description |
|-------|-------------|
| `PermissionDenied` | Accessibility permissions not granted |
| `AppNotFound` | Application not running or not found |
| `ElementNotFound` | No focused window or UI element |
| `NoContentFound` | Document is empty |
| `PatternNotSupported` | App doesn't support required accessibility patterns |
| `PlatformError` | macOS-specific API error |
| `Timeout` | Operation timed out |
| `AccessibilityError` | Failed to enable Electron accessibility |

### Response Status Codes

| Status | Action | Meaning |
|--------|--------|---------|
| `ok` | `created` | New content stored |
| `ok` | `updated` | Existing content updated |
| `ok` | `skipped` | Duplicate content, no action |
| `error` | `failed` | Processing failed |

---

## Migration Considerations

When migrating this system to another project, consider:

1. **Schema Compatibility:** The `content_sources` and `chunks` tables follow a specific schema. Ensure the target system can accommodate or adapt to this structure.

2. **Source Identifiers:** The `source` field values are hardcoded. Map these to the target system's taxonomy.

3. **URL Normalization:** The source path normalization logic is source-specific. Review and adapt for new sources.

4. **Deduplication Strategy:** The two-tier dedup (in-memory cache + database) may need adjustment based on scale requirements.

5. **Chunking Parameters:** The 1024-token max with 100-token overlap may need tuning for different embedding models.

6. **Platform Dependencies:** The accessibility extractor is macOS-only. Windows/Linux would require different APIs.

7. **Native Messaging:** The Chrome extension uses native messaging which requires platform-specific manifest installation.


---

## Detailed Flow Diagrams

### Browser Content Ingestion Flow

```mermaid
flowchart TD
    subgraph Browser["Chrome Browser"]
        A[User switches tab] --> B[Service Worker: handleTabActivated]
        B --> C{Is HTTP URL?}
        C -->|No| D[Skip]
        C -->|Yes| E{Recently processed?}
        E -->|Yes| D
        E -->|No| F[Inject Content Script]
        F --> G[Content Script: getExtractor]
        G --> H{Source Type?}
        
        H -->|Slack| I1[extractSlack]
        H -->|Gmail| I2[extractGmail]
        H -->|Jira| I3[extractJira]
        H -->|Google Docs| I4[extractGoogleDocs]
        H -->|Google Sheets| I5[extractGoogleSheets]
        H -->|Generic| I6[extractGeneric]
        
        I1 --> J[Build CapturePayload]
        I2 --> J
        I3 --> J
        I4 --> J
        I5 --> J
        I6 --> J
        
        J --> K[sendToNativeHost]
    end
    
    subgraph Native["Native Layer"]
        K --> L[Native Host: read_message]
        L --> M[forward_to_service]
        M --> N[Unix Socket Write]
        N --> O[Ingestion Service]
    end
    
    subgraph Processing["Ingestion Service"]
        O --> P[Parse CapturePayload]
        P --> Q[compute_hash]
        Q --> R[normalize_source_path]
        R --> S[DedupCache.check]
        S --> T{Result?}
        
        T -->|Duplicate| U[Return: Skipped]
        T -->|Changed| V[Update Flow]
        T -->|New| W[Insert Flow]
        
        V --> X[Soft-delete old chunks]
        X --> Y[Chunker.chunk]
        W --> Y
        Y --> Z[Storage.insert/update]
        Z --> AA[Update cache]
        AA --> AB[Return Response]
    end
    
    AB --> AC[Native Host: write_message]
    AC --> AD[Service Worker receives response]
```

### Desktop Application Extraction Flow

```mermaid
flowchart TD
    subgraph Daemon["AX Daemon"]
        A[NSWorkspace Notification] --> B{App Deactivated?}
        B -->|No| C[Update current_app]
        B -->|Yes| D{Is Supported App?}
        D -->|No| E[Ignore]
        D -->|Yes| F[extract_and_store]
    end
    
    subgraph Extraction["Extraction Process"]
        F --> G{Supports Direct File?}
        G -->|Yes| H[extract_from_office_app]
        H --> I{Success?}
        I -->|Yes| J[Return Content]
        I -->|No| K[Fallback to AX API]
        G -->|No| K
        
        K --> L{Is Electron App?}
        L -->|Yes| M[prepare_electron_app]
        M --> N[Enable AX for Electron]
        L -->|No| O[Standard AX Extraction]
        N --> O
        
        O --> P[get_app_by_bundle_id]
        P --> Q[extract_from_element]
        Q --> R[Get focused window]
        R --> S[extract_text_filtered]
        S --> T[Traverse AX tree]
        T --> U[Filter by role]
        U --> V[Collect text content]
        V --> J
    end
    
    subgraph Storage["Storage"]
        J --> W[DaemonStorage.store_content]
        W --> X{Is Slack?}
        X -->|Yes| Y[Message-level dedup]
        X -->|No| Z[Content-level dedup]
        
        Y --> AA[Parse messages]
        AA --> AB[Hash each message]
        AB --> AC[Find new messages]
        AC --> AD[Insert new messages]
        AD --> AE[Rebuild chunks]
        
        Z --> AF[Hash content]
        AF --> AG{Exists?}
        AG -->|No| AH[Insert new]
        AG -->|Yes, Same| AI[Skip]
        AG -->|Yes, Different| AJ[Update]
        
        AE --> AK[Store in SQLite]
        AH --> AK
        AJ --> AK
    end
```

### Jira Extraction Strategy

```mermaid
flowchart TD
    A[extractJira] --> B[Extract issue key from URL]
    B --> C{Has issue key?}
    
    C -->|Yes| D[Strategy 1: REST API]
    D --> E[fetchJiraIssueViaApi]
    E --> F{Success?}
    F -->|Yes| G[parseJiraApiResponse]
    G --> H[Return CapturePayload]
    
    F -->|No| I[Strategy 2: DOM Extraction]
    C -->|No| I
    
    I --> J[extractJiraFromDom]
    J --> K[Wait for React render]
    K --> L[Extract from selectors]
    L --> M{Success?}
    M -->|Yes| H
    
    M -->|No| N{Is Board View?}
    N -->|Yes| O[extractJiraBoardView]
    O --> P{Success?}
    P -->|Yes| H
    
    N -->|No| Q{Is Backlog?}
    P -->|No| Q
    Q -->|Yes| R[extractJiraBacklogView]
    R --> S{Success?}
    S -->|Yes| H
    S -->|No| T[Return null]
    Q -->|No| T
```

### Google Docs Extraction Strategy

```mermaid
flowchart TD
    A[extractGoogleDocs] --> B[Extract doc ID from URL]
    B --> C{Has doc ID?}
    C -->|No| D[Return null - Home page]
    
    C -->|Yes| E[waitForGoogleDocsReady]
    E --> F[Get title from DOM]
    
    F --> G[Strategy 1: Export API]
    G --> H[requestExportViaServiceWorker]
    H --> I[Service Worker fetches export URL]
    I --> J{Success?}
    J -->|Yes| K[Return content]
    
    J -->|No| L[Strategy 2: DOM Extraction]
    L --> M[extractGoogleDocsFromDom]
    M --> N[Try kix-page elements]
    N --> O{Found content?}
    O -->|Yes| K
    
    O -->|No| P[Try kix-appview-editor]
    P --> Q{Found content?}
    Q -->|Yes| K
    
    Q -->|No| R[Strategy 3: Iframe]
    R --> S[extractGoogleDocsFromIframe]
    S --> T{Found content?}
    T -->|Yes| K
    T -->|No| U[Return null]
```

---

## Security Considerations

1. **Accessibility Permissions:** The AX daemon requires explicit user consent via System Preferences.

2. **Native Messaging:** Only the specified Chrome extension ID can communicate with the native host.

3. **Local Storage:** All data is stored locally in SQLite; no cloud transmission.

4. **Credential Handling:** Google exports use existing browser session cookies (no stored credentials).

5. **Content Isolation:** Each source type has isolated extraction logic to prevent cross-contamination.

---

## Performance Characteristics

| Operation | Typical Latency | Notes |
|-----------|-----------------|-------|
| Tab switch extraction | 100-500ms | Depends on page complexity |
| Google export | 1-3s | Rate limited |
| Jira API extraction | 200-500ms | Depends on issue size |
| AX tree traversal | 50-200ms | Depends on app complexity |
| Dedup check (cache hit) | <1ms | In-memory |
| Dedup check (DB) | 1-5ms | SQLite indexed lookup |
| Chunking | 1-10ms | Depends on content size |
| SQLite insert | 5-20ms | With transaction |

---

## Testing Notes

The project includes property-based tests using `proptest` for:
- Serialization round-trips
- Bundle ID to source mapping
- Error message validation
- Content structure completeness

Test files are located in:
- `accessibility-extractor/tests/cli_tests.rs`
- `accessibility-extractor/src/types.rs` (inline tests)
- `ingestion-service/src/*.rs` (inline tests)
