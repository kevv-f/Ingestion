# Browser Content Ingestion Pipeline

A modular browser content extraction system that captures visible content from web applications and native desktop apps, deduplicates it, chunks it for vector search, and stores it in SQLite.

## Prerequisites

- Rust (1.70+)
- Swift (5.9+) - macOS only
- Node.js (18+)
- Chrome browser

## Quick Start

```bash
# Build all components
./scripts/build-all.sh --release

# Start the unified service
./scripts/ingestion start

# Or run directly
./unified-router/target/release/ingestion
```

## Build Commands

### Build All Components

```bash
# Debug build
./scripts/build-all.sh

# Release build
./scripts/build-all.sh --release
```

### Individual Components

**Unified Router** (main ingestion binary)
```bash
cd unified-router
cargo build --release
```

**Ingestion Service**
```bash
cd ingestion-service
cargo build --release
```

**Accessibility Extractor** (macOS desktop app extraction)
```bash
cd accessibility-extractor
cargo build --release
```

**Native Host** (Chrome extension bridge)
```bash
cd native-host
cargo build --release
```

**OCR Extractor** (Swift, macOS only)
```bash
cd ocr-extractor
swift build -c release
```

**Viewer App** (Tauri + React frontend)
```bash
cd viewer-app
npm install
npm run build          # Build frontend
npm run tauri build    # Build Tauri app
```

### Running Tests

```bash
# Rust tests
cd accessibility-extractor && cargo test
cd ingestion-service && cargo test

# Frontend tests
cd viewer-app && npm run test:run

# Swift tests
cd ocr-extractor && swift test
```

## Binary Locations

After building with `--release`:

| Component | Binary Path |
|-----------|-------------|
| Unified Router | `unified-router/target/release/ingestion` |
| Ingestion Server | `ingestion-service/target/release/ingestion-server` |
| AX Extractor | `accessibility-extractor/target/release/ax-extractor` |
| AX Daemon | `accessibility-extractor/target/release/ax-daemon` |
| Native Host | `native-host/target/release/ingestion-host` |
| OCR Extractor | `ocr-extractor/.build/release/OCRExtractor` |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Chrome Extension                             │
│  - Detects tab focus changes                                     │
│  - Extracts content via site-specific extractors                 │
│  - Outputs flat BrowserCapturePayload                            │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Native Messaging (stdio)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Native Host (thin relay)                     │
│  - Receives JSON from Chrome                                     │
│  - Forwards to Unix socket                                       │
│  - Returns response to Chrome                                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Unix Domain Socket
                           │ /tmp/clace-ingestion.sock
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              Ingestion Service (Rust library/binary)             │
│                                                                  │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐        │
│  │ Socket       │──▶│ Dedup Cache  │──▶│ Chunker      │        │
│  │ Listener     │   │ (in-memory)  │   │ (1024 tokens)│        │
│  └──────────────┘   └──────────────┘   └──────┬───────┘        │
│                                               │                 │
│                     ┌──────────────┐          │                 │
│                     │ SQLite       │◀─────────┘                 │
│                     │ Storage      │                            │
│                     │              │                            │
│                     │ content_sources + chunks                  │
│                     └──────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### Chrome Extension (`chrome-extension/`)

Manifest V3 extension that extracts content from:
- Slack (channel messages)
- Gmail (inbox list, email content)
- Outlook (inbox list, email content)
- Jira (issues, boards)
- Google Docs (document text)
- Discord (channel messages)
- Generic websites (article content)

### Native Host (`native-host/`)

Thin relay binary that forwards messages from Chrome to the ingestion service via Unix socket.

### Ingestion Service (`ingestion-service/`)

Rust library/binary that:
- Listens on Unix socket for incoming payloads
- Deduplicates content using in-memory cache + SQLite
- Chunks content into 1024-token segments
- Stores in SQLite (content_sources + chunks tables)

## Payload Format

```json
{
  "source": "slack",
  "url": "https://workspace.slack.com/archives/C123/p1234567890",
  "content": "[john 10:30] Hey, did you see the PR?\n[jane 10:32] Yeah, looks good.",
  "title": "engineering",
  "channel": "engineering",
  "threadId": "1234567890.000000",
  "timestamp": 1234567890000
}
```

## Setup

### 1. Start the Ingestion Service

```bash
cd ingestion-service
cargo run --release --bin ingestion-server
```

This starts the Unix socket server at `/tmp/clace-ingestion.sock`.

### 2. Build and Register the Native Host

```bash
cd native-host
cargo build --release

# Update the manifest with your extension ID
# Then copy to Chrome's native messaging directory:
cp com.yourapp.ingestion_host.json \
   ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
```

### 3. Load the Chrome Extension

1. Open `chrome://extensions`
2. Enable Developer mode
3. Click "Load unpacked" and select `chrome-extension/`
4. Note the extension ID and update the native host manifest

## Data Storage

SQLite database at `~/Library/Application Support/clace-ingestion/browser_content.db`

### Tables

**content_sources** - Tracks ingested sources
```sql
id, source_type, source_path, content_hash, ehl_doc_id, chunk_count, 
ingestion_status, created_at, updated_at
```

**chunks** - Actual content chunks for vector search
```sql
id, vector_index, text, meta (JSON), is_deleted, created_at
```

## Deduplication Logic

1. Compute SHA-256 hash of content
2. Check in-memory cache (URL → hash)
   - Cache hit + same hash → skip
   - Cache hit + different hash → update
   - Cache miss → check SQLite
3. Check SQLite by source_path (URL)
   - Found + same hash → skip, update cache
   - Found + different hash → soft-delete old chunks, insert new
   - Not found → insert new source + chunks

## Integration with Tauri

The `ingestion-service` is designed as a library. To integrate with your Tauri app:

```rust
use ingestion_service::{IngestionServer, ServerConfig};

// In your Tauri setup
let config = ServerConfig {
    socket_path: "/tmp/clace-ingestion.sock".into(),
    db_path: "path/to/your/browser_content.db".into(),
};

let server = IngestionServer::new(config)?;

// Run in background
tokio::spawn(async move {
    server.run().await
});
```

Or use the `process()` method directly without the socket:

```rust
let response = server.process(payload).await;
```
