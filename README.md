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

### 2. Chrome Extension Installation (Step-by-Step)

The Chrome extension enables content extraction from web pages. Follow these steps carefully:

#### Step 2.1: Build the Native Host Binary

```bash
cd native-host
cargo build --release
```

This creates the binary at `native-host/target/release/ingestion-host`.

#### Step 2.2: Load the Chrome Extension

1. Open Chrome and navigate to `chrome://extensions`
2. Enable **Developer mode** (toggle in the top-right corner)
3. Click **"Load unpacked"**
4. Select the `chrome-extension/` directory from this project
5. The extension will appear in your extensions list

#### Step 2.3: Copy Your Extension ID

After loading the extension:
1. Look at the extension card in `chrome://extensions`
2. Find the **ID** field (a 32-character string like `ohimjnpbhdbadinjojadgghoifjejkkm`)
3. Copy this ID — you'll need it in the next step

#### Step 2.4: Update the Native Host Manifest

Edit `native-host/com.clace.extension.json`:

```json
{
  "name": "com.clace.extension",
  "description": "Clace content ingestion native messaging host",
  "path": "/ABSOLUTE/PATH/TO/native-host/target/release/ingestion-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID_HERE/"
  ]
}
```

Replace:
- `/ABSOLUTE/PATH/TO/` with the actual absolute path to your project
- `YOUR_EXTENSION_ID_HERE` with the extension ID you copied in Step 2.3

**Example (macOS):**
```json
{
  "name": "com.clace.extension",
  "description": "Clace content ingestion native messaging host",
  "path": "/Users/yourname/projects/ingestion/native-host/target/release/ingestion-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://ohimjnpbhdbadinjojadgghoifjejkkm/"
  ]
}
```

#### Step 2.5: Install the Native Host Manifest

**macOS:**
```bash
# Create the directory if it doesn't exist
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# Copy the manifest
cp native-host/com.clace.extension.json \
   ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
```

**Linux:**
```bash
# Create the directory if it doesn't exist
mkdir -p ~/.config/google-chrome/NativeMessagingHosts/

# Copy the manifest
cp native-host/com.clace.extension.json \
   ~/.config/google-chrome/NativeMessagingHosts/
```

**Windows:**
1. Place `com.clace.extension.json` in a permanent location (e.g., `C:\Program Files\Clace\`)
2. Open Registry Editor (`regedit`)
3. Navigate to `HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts`
4. Create a new key named `com.clace.extension`
5. Set the default value to the full path of your JSON file

#### Step 2.6: Restart Chrome

Close and reopen Chrome completely for the native messaging host to be recognized.

#### Step 2.7: Verify the Installation

1. Make sure the ingestion service is running:
   ```bash
   ./unified-router/target/release/ingestion
   ```

2. Open any web page in Chrome
3. The extension should automatically extract content when you switch tabs
4. Check the ingestion service logs for incoming content:
   ```
   📥 Received content: chrome - Page Title (1234 chars)
   ✅ Stored: chrome - Page Title
   ```

#### Troubleshooting

**Extension not connecting to native host:**
- Verify the `path` in the manifest is an absolute path and the binary exists
- Verify the `allowed_origins` contains your exact extension ID with trailing slash
- Check Chrome's native messaging logs: `chrome://extensions` → Extension details → "Inspect views"

**"Native host has exited" error:**
- Ensure the ingestion service is running (`/tmp/clace-ingestion.sock` must exist)
- Check that the native host binary has execute permissions: `chmod +x native-host/target/release/ingestion-host`

**Content not being extracted:**
- Open DevTools (F12) on any page and check the Console for errors
- Verify the content script is injected: look for `content/index.js` in Sources panel

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
