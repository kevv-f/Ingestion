# Chrome Extension: Content Ingestion Pipeline

This document provides a comprehensive technical overview of the Chrome extension-based content ingestion system. It covers architecture, installation, development workflows, and the complete data flow from browser to storage.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Installation Guide](#installation-guide)
3. [Component Deep Dive](#component-deep-dive)
4. [Native Messaging Protocol](#native-messaging-protocol)
5. [Ingestion Service](#ingestion-service)
6. [Development Guide](#development-guide)
7. [Supported Platforms](#supported-platforms)
8. [Troubleshooting](#troubleshooting)

---

## Architecture Overview

The content ingestion pipeline consists of three main components that work together to extract, transport, and store content from web pages:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              BROWSER (Chrome)                                │
│  ┌─────────────────┐    ┌──────────────────┐    ┌────────────────────────┐  │
│  │  Content Script │───▶│  Service Worker  │───▶│  Native Messaging API  │  │
│  │  (DOM Extract)  │    │  (Orchestrator)  │    │  (chrome.runtime)      │  │
│  └─────────────────┘    └──────────────────┘    └───────────┬────────────┘  │
└─────────────────────────────────────────────────────────────┼───────────────┘
                                                              │ stdin/stdout
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           NATIVE HOST (Rust Binary)                          │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  Thin relay: reads native messages from stdin, forwards to Unix      │   │
│  │  socket, returns response to stdout                                  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┬───────────────┘
                                                              │ Unix Socket
                                                              │ /tmp/clace-ingestion.sock
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         INGESTION SERVICE (Rust/Tokio)                       │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────────┐   │
│  │  Dedup     │───▶│  Chunker   │───▶│  Storage   │───▶│  SQLite DB     │   │
│  │  Cache     │    │  (1024 tok)│    │  Manager   │    │  content.db    │   │
│  └────────────┘    └────────────┘    └────────────┘    └────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow Summary

1. **User navigates** to a supported page (Slack, Gmail, Jira, Google Docs, etc.)
2. **Service Worker** detects tab activation/URL change and triggers extraction
3. **Content Script** extracts structured content from the DOM
4. **Service Worker** sends payload via Native Messaging to the native host
5. **Native Host** relays the message over Unix socket to the ingestion service
6. **Ingestion Service** deduplicates, chunks, and stores the content in SQLite

---

## Installation Guide

### Prerequisites

- Google Chrome (or Chromium-based browser)
- Rust toolchain (for building native components)
- macOS, Linux, or Windows

### Step 1: Build the Native Host

```bash
cd native-host
cargo build --release
```

The binary will be at `native-host/target/release/ingestion-host`.

### Step 2: Build the Ingestion Service

```bash
cd ingestion-service
cargo build --release
```

### Step 3: Configure the Native Messaging Manifest

Edit `native-host/com.yourapp.ingestion_host.json`:

```json
{
  "name": "com.yourapp.ingestion_host",
  "description": "Content ingestion native messaging host for Tauri app",
  "path": "/absolute/path/to/native-host/target/release/ingestion-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID_HERE/"
  ]
}
```

**Important:** Replace:
- `path` with the absolute path to your built `ingestion-host` binary
- `YOUR_EXTENSION_ID_HERE` with your extension's ID (obtained in Step 5)

### Step 4: Install the Native Messaging Manifest

#### macOS

```bash
# For current user only
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
cp native-host/com.yourapp.ingestion_host.json \
   ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# For all users (requires sudo)
sudo mkdir -p /Library/Google/Chrome/NativeMessagingHosts/
sudo cp native-host/com.yourapp.ingestion_host.json \
   /Library/Google/Chrome/NativeMessagingHosts/
```

#### Linux

```bash
# For current user only
mkdir -p ~/.config/google-chrome/NativeMessagingHosts/
cp native-host/com.yourapp.ingestion_host.json \
   ~/.config/google-chrome/NativeMessagingHosts/

# For all users (requires sudo)
sudo mkdir -p /etc/opt/chrome/native-messaging-hosts/
sudo cp native-host/com.yourapp.ingestion_host.json \
   /etc/opt/chrome/native-messaging-hosts/
```

#### Windows

1. Place the manifest JSON file in a permanent location (e.g., `C:\Program Files\YourApp\`)
2. Add a registry key:
   ```
   HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts\com.yourapp.ingestion_host
   ```
3. Set the default value to the full path of the JSON file

### Step 5: Load the Chrome Extension

1. Open Chrome and navigate to `chrome://extensions`
2. Enable **Developer mode** (toggle in top-right)
3. Click **Load unpacked**
4. Select the `chrome-extension` directory
5. Note the **Extension ID** displayed under the extension name
6. Update `allowed_origins` in the manifest with this ID (Step 3)
7. Re-copy the manifest to the NativeMessagingHosts directory (Step 4)

### Step 6: Start the Ingestion Service

The ingestion service must be running to receive content:

```bash
cd ingestion-service
cargo run --release --bin ingestion-server
```

Or if embedded in a Tauri app, it starts automatically with the application.

### Step 7: Verify Installation

1. Open Chrome DevTools (F12) on any page
2. Check the Console for `[ServiceWorker] Initialized` message
3. Navigate to a supported site (e.g., Google Docs)
4. Check for extraction messages in the console
5. Verify the ingestion service logs show received content

---

## Component Deep Dive

### Service Worker (`background/service-worker.js`)

The service worker is the orchestration layer of the extension. It runs in the background and coordinates content extraction.

#### What is a Service Worker?

A Chrome extension service worker is a background script that:
- Runs independently of any web page
- Is **ephemeral** - Chrome terminates it after ~30 seconds of inactivity
- Handles events like tab changes, messages, and alarms
- Cannot access the DOM directly (must communicate with content scripts)

#### Key Responsibilities

1. **Tab Event Handling**: Listens for tab activation, URL changes, and window focus
2. **Content Script Injection**: Ensures content scripts are loaded in target pages
3. **Native Messaging**: Sends extracted content to the native host
4. **Rate Limiting**: Prevents excessive extraction (debouncing, caching)
5. **Google Export Fetching**: Handles cross-origin requests for Google Docs/Sheets/Slides

#### Event Listeners

```javascript
// Tab activation - user switches tabs
chrome.tabs.onActivated.addListener(handleTabActivated);

// URL changes in SPAs (Jira, Gmail, etc.)
chrome.tabs.onUpdated.addListener(handleTabUpdated);

// Window focus changes
chrome.windows.onFocusChanged.addListener(handleWindowFocusChanged);
```

#### Extraction Triggers

The service worker triggers extraction on:
- **Tab activation**: User switches to a different tab
- **Window focus**: User switches between Chrome windows
- **URL change**: SPA navigation (e.g., clicking issues in Jira)
- **Page load**: Initial page load completion

#### Rate Limiting & Caching

```javascript
// Debounce: 1 second for most sites, 5 seconds for Google products
const DEBOUNCE_MS = 1000;
const GOOGLE_DEBOUNCE_MS = 5000;

// Content cache: Skip re-extraction within 5 minutes
const CONTENT_CACHE_TTL_MS = 300000;

// Google export rate limiting: 10 seconds between exports
const GOOGLE_EXPORT_COOLDOWN_MS = 10000;
```

#### Native Messaging

The service worker uses one-shot messaging (not persistent connections) because service workers are ephemeral:

```javascript
async function sendToNativeHost(data) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(NATIVE_HOST_NAME, data, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      } else {
        resolve(response);
      }
    });
  });
}
```

### Content Script (`content/index.js`)

The content script runs in the context of web pages and extracts structured content from the DOM.

#### Injection

Defined in `manifest.json`:
```json
{
  "content_scripts": [
    {
      "matches": ["<all_urls>"],
      "js": ["content/index.js"],
      "run_at": "document_idle",
      "all_frames": false
    }
  ]
}
```

#### Extractor Router

The content script routes URLs to specialized extractors:

```javascript
function getExtractor(url) {
  const hostname = new URL(url).hostname;
  
  if (hostname.includes('slack.com')) return extractSlack;
  if (hostname.includes('mail.google.com')) return extractGmail;
  if (hostname.includes('atlassian.net')) return extractJira;
  if (hostname.includes('docs.google.com')) return extractGoogleDocs;
  // ... more extractors
  return extractGeneric;
}
```

#### Output Format (CapturePayload)

All extractors output a standardized payload:

```typescript
interface CapturePayload {
  source: string;      // "slack" | "gmail" | "jira" | "gdocs" | "browser"
  url: string;         // Location identifier (may be canonical)
  content: string;     // Extracted text content
  title?: string;      // Document/page title
  author?: string;     // Author/sender
  channel?: string;    // Channel/project/workspace
  timestamp?: number;  // Unix timestamp in seconds
}
```

#### Extraction Strategies

Most extractors use multiple strategies in order of reliability:

1. **REST API** (Jira): Uses session cookies to fetch structured data
2. **Export URL** (Google Docs/Sheets/Slides): Fetches plain text export
3. **DOM Extraction**: Parses page structure with CSS selectors
4. **Accessibility Layer**: Falls back to accessibility elements

#### Message Handler

```javascript
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'ping') {
    sendResponse({ pong: true });
    return false;
  }

  if (message.type === 'extract') {
    (async () => {
      const extractor = getExtractor(message.url);
      const payload = await extractor();
      sendResponse({ success: true, data: payload });
    })();
    return true; // Async response
  }
});
```

### Manifest (`manifest.json`)

```json
{
  "manifest_version": 3,
  "name": "Content Ingestion Pipeline",
  "version": "1.0.0",
  
  "permissions": [
    "tabs",           // Access tab URLs and state
    "activeTab",      // Access active tab content
    "scripting",      // Inject content scripts dynamically
    "nativeMessaging", // Communicate with native host
    "storage"         // Store extension state
  ],
  
  "host_permissions": [
    "<all_urls>"      // Access all websites
  ],
  
  "background": {
    "service_worker": "background/service-worker.js",
    "type": "module"
  },
  
  "content_scripts": [
    {
      "matches": ["<all_urls>"],
      "js": ["content/index.js"],
      "run_at": "document_idle"
    }
  ]
}
```

---

## Native Messaging Protocol

### Overview

Chrome's Native Messaging allows extensions to communicate with native applications. Messages are exchanged via stdin/stdout using a specific binary protocol.

### Message Format

Each message consists of:
1. **4-byte length prefix** (native byte order, uint32)
2. **JSON payload** (UTF-8 encoded)

```
┌──────────────┬─────────────────────────────────────┐
│ Length (4B)  │ JSON Payload (variable length)      │
└──────────────┴─────────────────────────────────────┘
```

### Native Host Implementation (`native-host/src/main.rs`)

The native host is a thin relay that:
1. Reads messages from stdin (Chrome)
2. Forwards them to the ingestion service via Unix socket
3. Returns responses to stdout (Chrome)

```rust
/// Read a native messaging message from stdin
fn read_message() -> io::Result<Option<Vec<u8>>> {
    let mut length_bytes = [0u8; 4];
    io::stdin().read_exact(&mut length_bytes)?;
    
    let length = u32::from_ne_bytes(length_bytes) as usize;
    let mut message = vec![0u8; length];
    io::stdin().read_exact(&mut message)?;
    
    Ok(Some(message))
}

/// Write a native messaging message to stdout
fn write_message(message: &[u8]) -> io::Result<()> {
    let length = message.len() as u32;
    io::stdout().write_all(&length.to_ne_bytes())?;
    io::stdout().write_all(message)?;
    io::stdout().flush()
}
```

### Unix Socket Communication

The native host connects to the ingestion service at `/tmp/clace-ingestion.sock`:

```rust
const SOCKET_PATH: &str = "/tmp/clace-ingestion.sock";

fn forward_to_service(message: &[u8]) -> io::Result<Vec<u8>> {
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    stream.write_all(message)?;
    stream.write_all(b"\n")?;  // Newline delimiter
    stream.flush()?;
    
    // Read newline-delimited response
    let mut response = Vec::new();
    // ... read until newline
    Ok(response)
}
```

### Manifest Configuration

The manifest tells Chrome how to launch the native host:

```json
{
  "name": "com.yourapp.ingestion_host",
  "description": "Content ingestion native messaging host",
  "path": "/absolute/path/to/ingestion-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
```

---

## Ingestion Service

### Overview

The ingestion service is a Rust application that receives content payloads, deduplicates them, chunks them for vector storage, and persists them to SQLite.

### Components

#### Server (`server.rs`)

Listens on a Unix socket and processes incoming payloads:

```rust
pub struct IngestionServer {
    config: ServerConfig,
    state: Arc<Mutex<ServiceState>>,
}

impl IngestionServer {
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = UnixListener::bind(&self.config.socket_path)?;
        
        loop {
            let (stream, _) = listener.accept().await?;
            let state = Arc::clone(&self.state);
            tokio::spawn(handle_connection(stream, state));
        }
    }
}
```

#### Deduplication (`dedup.rs`)

Prevents storing duplicate content using content hashing:

```rust
pub enum DedupResult {
    New,                    // Content never seen before
    Duplicate(String),      // Same content, same hash
    Changed(String),        // Same URL, different content
}

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

The dedup cache uses:
- **In-memory LRU cache** for fast lookups (10,000 entries, 24h TTL)
- **Database fallback** for cache misses

#### Chunking (`chunker.rs`)

Splits content into fixed-size chunks for vector embedding:

```rust
pub struct ChunkerConfig {
    pub max_tokens: usize,      // 1024 tokens per chunk
    pub overlap_tokens: usize,  // 100 token overlap
}
```

Special handling for tabular data (Excel, CSV):
- Preserves row structure
- Chunks at row boundaries
- Maintains header context

#### Storage (`storage.rs`)

SQLite schema for content persistence:

```sql
-- Content sources (tracks what we've ingested)
CREATE TABLE content_sources (
    id INTEGER PRIMARY KEY,
    source_type TEXT NOT NULL,      -- "slack", "gmail", "jira", etc.
    source_path TEXT NOT NULL UNIQUE, -- Canonical URL
    content_hash TEXT NOT NULL,     -- SHA-256 for dedup
    ehl_doc_id TEXT NOT NULL UNIQUE, -- UUID for chunk grouping
    chunk_count INTEGER NOT NULL,
    ingestion_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Chunks (actual content pieces)
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    vector_index INTEGER,           -- Position in vector store
    text TEXT NOT NULL,             -- Chunk content
    meta TEXT NOT NULL,             -- JSON metadata
    is_deleted INTEGER DEFAULT 0    -- Soft delete flag
);
```

#### URL Normalization

The service normalizes URLs to create canonical identifiers:

```rust
fn normalize_source_path(source: &str, url: &str) -> String {
    match source {
        "gdocs" => {
            // Extract doc ID: gdocs://DOC_ID
            if let Some(doc_id) = extract_doc_id(url) {
                return format!("gdocs://{}", doc_id);
            }
        }
        "jira" => {
            // Extract issue key: jira://host:PROJ-123
            if let Some(issue) = extract_issue_key(url) {
                return format!("jira://{}:{}", host, issue);
            }
        }
        // ... other sources
    }
}
```

### Response Format

```rust
pub struct IngestionResponse {
    pub status: ResponseStatus,     // "ok" | "error"
    pub action: IngestionAction,    // "created" | "updated" | "skipped" | "failed"
    pub ehl_doc_id: Option<String>, // Document UUID
    pub chunk_count: Option<usize>, // Number of chunks created
    pub message: Option<String>,    // Error/skip reason
}
```

---

## Development Guide

### Making Changes to the Service Worker

1. **Edit** `chrome-extension/background/service-worker.js`
2. **Reload** the extension:
   - Go to `chrome://extensions`
   - Click the refresh icon on your extension
   - Or click "Update" if available
3. **Test** by navigating to a supported site
4. **Debug** using Chrome DevTools:
   - Click "service worker" link on the extension card
   - Opens DevTools for the service worker context

#### Adding a New Extraction Trigger

```javascript
// In service-worker.js

// Add new event listener
chrome.someEvent.addListener(handleNewEvent);

async function handleNewEvent(eventData) {
  // Get the active tab
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab) {
    await processTab(tab, 'new-trigger');
  }
}
```

#### Adding Rate Limiting for a New Source

```javascript
// Add source-specific debounce
const SOURCE_DEBOUNCE = {
  'default': 1000,
  'google': 5000,
  'newSource': 3000  // Add your source
};

function getDebounceTime(url) {
  if (url.includes('newSource.com')) return SOURCE_DEBOUNCE.newSource;
  if (url.includes('google.com')) return SOURCE_DEBOUNCE.google;
  return SOURCE_DEBOUNCE.default;
}
```

### Making Changes to Content Scripts

1. **Edit** `chrome-extension/content/index.js` or extractors in `content/extractors/`
2. **Reload** the extension (same as above)
3. **Refresh** the target web page (content scripts are injected on page load)
4. **Debug** using the page's DevTools console

#### Adding a New Extractor

```javascript
// 1. Add to the router in content/index.js
function getExtractor(url) {
  const hostname = new URL(url).hostname;
  
  // Add your new extractor
  if (hostname.includes('newsite.com')) return extractNewSite;
  
  // ... existing extractors
  return extractGeneric;
}

// 2. Implement the extractor
async function extractNewSite() {
  // Wait for content to load
  await waitForContent('.main-content');
  
  // Extract structured data
  const title = document.querySelector('h1')?.textContent?.trim();
  const content = document.querySelector('.content')?.textContent?.trim();
  
  return {
    source: 'newsite',
    url: window.location.href,
    content: content,
    title: title,
    timestamp: nowUnixSeconds()
  };
}
```

#### Handling Dynamic Content (SPAs)

```javascript
// Use MutationObserver for React/Vue/Angular apps
function waitForContent(selector, timeout = 5000) {
  return new Promise((resolve) => {
    const element = document.querySelector(selector);
    if (element) {
      resolve(element);
      return;
    }
    
    const observer = new MutationObserver((mutations, obs) => {
      const el = document.querySelector(selector);
      if (el) {
        obs.disconnect();
        resolve(el);
      }
    });
    
    observer.observe(document.body, {
      childList: true,
      subtree: true
    });
    
    setTimeout(() => {
      observer.disconnect();
      resolve(null);
    }, timeout);
  });
}
```

### Testing Changes

#### Manual Testing

1. Load the extension in developer mode
2. Open Chrome DevTools on a target page
3. Check console for `[ContentScript]` and `[ServiceWorker]` logs
4. Verify payloads in the ingestion service logs

#### Debugging Native Messaging

```bash
# Check if native host is registered
ls ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# Test native host directly
echo '{"source":"test","url":"http://test.com","content":"hello"}' | \
  /path/to/ingestion-host

# Check ingestion service logs
tail -f /path/to/ingestion-service.log
```

### Hot Reloading

The extension doesn't support true hot reloading, but you can speed up development:

1. **Service Worker**: Reload extension from `chrome://extensions`
2. **Content Script**: Refresh the target page
3. **Native Host**: Restart the ingestion service

---

## Supported Platforms

| Platform | Extraction Method | Notes |
|----------|------------------|-------|
| Slack | DOM parsing | Messages, channels, threads |
| Gmail | DOM parsing | Inbox list, email detail |
| Outlook | DOM parsing | Inbox list, reading pane |
| Jira | REST API + DOM | Issues, boards, backlog |
| Google Docs | Export API | Full document text |
| Google Sheets | Export API (CSV) | Tabular data preserved |
| Google Slides | Export API | Slide text content |
| Gemini | DOM parsing | Conversation history |
| Google AI Mode | DOM parsing | AI responses and search |
| Discord | DOM parsing | Messages, channels |
| Generic | Readability | Any webpage (fallback) |

---

## Troubleshooting

### Extension Not Extracting

1. Check service worker is running: `chrome://extensions` → "service worker" link
2. Verify content script loaded: DevTools Console → look for `[ContentScript] Loaded`
3. Check for errors in DevTools Console

### Native Messaging Errors

```
"Native host has exited"
```
- Verify the native host binary path in manifest is correct
- Check the binary has execute permissions
- Ensure `allowed_origins` includes your extension ID

```
"Specified native messaging host not found"
```
- Verify manifest is in correct NativeMessagingHosts directory
- Check manifest JSON is valid (no trailing commas)
- Restart Chrome after installing manifest

### Ingestion Service Not Receiving

1. Check Unix socket exists: `ls -la /tmp/clace-ingestion.sock`
2. Verify service is running: `ps aux | grep ingestion`
3. Check service logs for connection errors

### Google Docs/Sheets Export Failing

```
"Rate limited by Google (429)"
```
- Wait 30 seconds before retrying
- The extension automatically backs off on 429 errors

```
"Export returned empty content"
```
- Ensure you're logged into Google
- Check the document isn't empty
- Try refreshing the page

### Content Not Being Deduplicated

1. Check URL normalization is working (logs should show canonical URLs)
2. Verify content hash is being computed correctly
3. Check dedup cache isn't full (10,000 entry limit)
