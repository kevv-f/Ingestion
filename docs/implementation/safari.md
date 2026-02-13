# Safari Web Extension: Content Ingestion Pipeline

This document provides comprehensive technical documentation for implementing the content ingestion pipeline as a Safari Web Extension with XPC-based communication to the native ingestion service.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Safari vs Chrome: Key Differences](#safari-vs-chrome-key-differences)
3. [Content Script Transferability](#content-script-transferability)
4. [XPC Communication Architecture](#xpc-communication-architecture)
5. [Implementation Guide](#implementation-guide)
6. [Project Structure](#project-structure)
7. [Installation & Distribution](#installation--distribution)
8. [Development Workflow](#development-workflow)
9. [Troubleshooting](#troubleshooting)

---

## Architecture Overview

Safari Web Extensions require a fundamentally different architecture than Chrome extensions due to Apple's sandboxing requirements. The extension must be packaged within a native macOS application and uses Apple's extension APIs for native communication.

### High-Level Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SAFARI BROWSER                                  │
│  ┌─────────────────┐    ┌──────────────────┐    ┌────────────────────────┐  │
│  │  Content Script │───▶│  Background Page │───▶│ browser.runtime        │  │
│  │  (DOM Extract)  │    │  (Orchestrator)  │    │ .sendNativeMessage()   │  │
│  └─────────────────┘    └──────────────────┘    └───────────┬────────────┘  │
└─────────────────────────────────────────────────────────────┼───────────────┘
                                                              │ NSExtension IPC
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    APP EXTENSION (Sandboxed Process)                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  SafariWebExtensionHandler.swift                                     │   │
│  │  - Receives messages via beginRequest(with:)                         │   │
│  │  - Writes payloads to App Group shared container                     │   │
│  │  - Returns response to JavaScript                                    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┬───────────────┘
                                                              │ NSFileCoordinator
                                                              │ (App Group Container)
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CONTAINER APP (macOS Application)                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  IngestionBridge.swift                                               │   │
│  │  - Monitors App Group for new payloads (NSFilePresenter)             │   │
│  │  - Forwards to ingestion service via XPC or Unix socket              │   │
│  │  - Manages launchd agent lifecycle                                   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┬───────────────┘
                                                              │ XPC Named Endpoint
                                                              │ (group.com.yourapp.xpc)
                                                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    LAUNCHD AGENT (Background Service)                        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  IngestionAgent                                                      │   │
│  │  - Runs as user-level daemon (launchd)                               │   │
│  │  - Exposes XPC endpoint within App Group                             │   │
│  │  - Connects to ingestion service Unix socket                         │   │
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

### Why This Architecture?

Apple's sandboxing model creates constraints that require this multi-hop architecture:

1. **Safari Web Extension** runs in Safari's process, can only communicate with its App Extension
2. **App Extension** (SafariWebExtensionHandler) is sandboxed and cannot use XPC Services directly
3. **Container App** can communicate with external processes but isn't always running
4. **Launchd Agent** provides always-available XPC endpoint that survives app restarts

---

## Safari vs Chrome: Key Differences

| Aspect | Chrome Extension | Safari Web Extension |
|--------|-----------------|---------------------|
| **Distribution** | Chrome Web Store ($5 one-time) | Mac App Store ($99/year Apple Developer Program) |
| **Packaging** | Standalone extension | Requires native macOS app wrapper |
| **Native Communication** | Native Messaging (stdin/stdout) | SafariWebExtensionHandler (NSExtension) |
| **Background Script** | Service Worker (ephemeral) | Background Page (persistent) |
| **API Namespace** | `chrome.*` | `browser.*` (also supports `chrome.*`) |
| **Content Scripts** | ✅ Identical API | ✅ Identical API |
| **Manifest Version** | Manifest V3 | Manifest V2/V3 (both supported) |
| **Code Signing** | Not required | Required (Developer ID or App Store) |
| **Sandboxing** | Process isolation | App Sandbox + App Groups |

### API Compatibility

Safari supports most WebExtensions APIs with some notable exceptions:

**Fully Supported:**
- `browser.runtime.sendMessage()` / `onMessage`
- `browser.tabs.*` (query, sendMessage, onActivated, onUpdated)
- `browser.storage.*` (local, sync)
- `browser.scripting.executeScript()`
- Content script injection via manifest

**Partially Supported:**
- `browser.runtime.sendNativeMessage()` - Routes to SafariWebExtensionHandler
- `browser.webRequest.*` - Observation only, **blocking not supported**

**Not Supported:**
- `browser.webRequest.onBeforeRequest` with blocking
- Some `chrome.debugger.*` APIs
- `chrome.declarativeNetRequest` (use Safari Content Blockers instead)

---

## Content Script Transferability

**Your existing Chrome content scripts are highly transferable to Safari.**

The WebExtensions content script API is identical between Chrome and Safari. Your extractors for Slack, Jira, Google Docs, Gmail, etc. will work without modification.

### What Transfers Directly (No Changes)

```javascript
// All of these work identically in Safari:

// DOM manipulation
document.querySelector('.selector');
document.querySelectorAll('.items');

// MutationObserver for SPAs
const observer = new MutationObserver((mutations) => { ... });
observer.observe(document.body, { childList: true, subtree: true });

// Fetch API for REST calls (Jira extraction)
const response = await fetch('/rest/api/3/issue/KEY-123', {
  credentials: 'include'
});

// Message passing to background
browser.runtime.sendMessage({ type: 'extract', data: payload });

// Listening for extraction requests
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'extract') {
    extractContent().then(data => sendResponse({ success: true, data }));
    return true; // Async response
  }
});
```

### Extractors That Work As-Is

| Extractor | Status | Notes |
|-----------|--------|-------|
| Slack | ✅ Works | DOM parsing unchanged |
| Gmail | ✅ Works | DOM parsing unchanged |
| Outlook | ✅ Works | DOM parsing unchanged |
| Jira | ✅ Works | REST API + DOM fallback |
| Google Docs | ✅ Works | Export URL fetching works |
| Google Sheets | ✅ Works | CSV export works |
| Google Slides | ✅ Works | Text export works |
| Discord | ✅ Works | DOM parsing unchanged |
| Generic | ✅ Works | Readability extraction |

### Required Adaptations

1. **Background Script**: Convert Service Worker to Background Page
2. **Native Messaging**: Route through SafariWebExtensionHandler instead of stdin/stdout
3. **Manifest**: Add Safari-specific keys and App Extension configuration

---

## XPC Communication Architecture

### The Sandbox Challenge

Safari App Extensions are sandboxed and **cannot directly use XPC Services**. Apple's Quinn "The Eskimo" explains:

> "You can't use an XPC service for this task because there's no way that the same XPC service can be visible to both your app and your appex."

### Solution: App Group + Launchd Agent

The recommended architecture uses:

1. **App Group Container**: Shared file system location accessible by both App Extension and Container App
2. **NSFileCoordinator/NSFilePresenter**: For coordinated file access and change notifications
3. **Launchd Agent**: Background process that exposes XPC endpoint within the App Group namespace

### App Group Configuration

App Groups allow sandboxed processes to share data. The group identifier must be prefixed with `group.`:

```
group.com.yourapp.ingestion
```

This creates a shared container at:
```
~/Library/Group Containers/group.com.yourapp.ingestion/
```

### XPC Endpoint Naming

For sandboxed apps to connect via XPC, the endpoint must be within the App Group namespace:

```
group.com.yourapp.ingestion.xpc
```

---

## Implementation Guide

### Step 1: Convert Chrome Extension to Safari

Use Apple's conversion tool:

```bash
xcrun safari-web-extension-converter \
  --project-location ./safari-extension \
  --app-name "Content Ingestion" \
  --bundle-identifier com.yourapp.ingestion \
  --swift \
  --macos-only \
  ./chrome-extension
```

This creates an Xcode project with:
- Container macOS app
- Safari Web Extension target
- Converted extension resources

### Step 2: Configure App Groups

In Xcode, for both the Container App and Extension targets:

1. Select target → Signing & Capabilities
2. Click "+ Capability" → App Groups
3. Add: `group.com.yourapp.ingestion`

### Step 3: Implement SafariWebExtensionHandler

```swift
// SafariWebExtensionHandler.swift
import SafariServices
import os.log

class SafariWebExtensionHandler: NSObject, NSExtensionRequestHandling {
    
    private let logger = Logger(subsystem: "com.yourapp.ingestion", category: "ExtensionHandler")
    private let appGroupID = "group.com.yourapp.ingestion"
    
    func beginRequest(with context: NSExtensionContext) {
        // Extract message from JavaScript
        guard let item = context.inputItems.first as? NSExtensionItem,
              let message = item.userInfo?[SFExtensionMessageKey] as? [String: Any] else {
            completeWithError(context, message: "Invalid message format")
            return
        }
        
        logger.info("Received message: \(String(describing: message))")
        
        // Handle different message types
        if let action = message["action"] as? String {
            switch action {
            case "ingest":
                handleIngest(message: message, context: context)
            case "ping":
                completeWithSuccess(context, response: ["status": "pong"])
            default:
                completeWithError(context, message: "Unknown action: \(action)")
            }
        } else {
            // Legacy: treat entire message as payload
            handleIngest(message: message, context: context)
        }
    }
    
    private func handleIngest(message: [String: Any], context: NSExtensionContext) {
        // Write payload to App Group container
        guard let groupURL = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: appGroupID
        ) else {
            completeWithError(context, message: "Cannot access App Group container")
            return
        }
        
        let payloadURL = groupURL.appendingPathComponent("pending_payload.json")
        
        do {
            // Add timestamp for ordering
            var payload = message
            payload["_receivedAt"] = Date().timeIntervalSince1970
            
            let jsonData = try JSONSerialization.data(withJSONObject: payload, options: .prettyPrinted)
            
            // Use file coordinator for safe concurrent access
            let coordinator = NSFileCoordinator()
            var coordinatorError: NSError?
            
            coordinator.coordinate(writingItemAt: payloadURL, options: .forReplacing, error: &coordinatorError) { url in
                do {
                    try jsonData.write(to: url)
                    self.logger.info("Payload written to App Group: \(jsonData.count) bytes")
                } catch {
                    self.logger.error("Failed to write payload: \(error.localizedDescription)")
                }
            }
            
            if let error = coordinatorError {
                throw error
            }
            
            completeWithSuccess(context, response: [
                "status": "ok",
                "action": "queued",
                "message": "Payload queued for ingestion"
            ])
            
        } catch {
            logger.error("Failed to serialize payload: \(error.localizedDescription)")
            completeWithError(context, message: "Serialization failed: \(error.localizedDescription)")
        }
    }
    
    private func completeWithSuccess(_ context: NSExtensionContext, response: [String: Any]) {
        let item = NSExtensionItem()
        item.userInfo = [SFExtensionMessageKey: response]
        context.completeRequest(returningItems: [item], completionHandler: nil)
    }
    
    private func completeWithError(_ context: NSExtensionContext, message: String) {
        let item = NSExtensionItem()
        item.userInfo = [SFExtensionMessageKey: ["status": "error", "message": message]]
        context.completeRequest(returningItems: [item], completionHandler: nil)
    }
}
```

### Step 4: Implement Container App Bridge

```swift
// IngestionBridge.swift
import Foundation
import os.log

class IngestionBridge: NSObject, NSFilePresenter {
    
    private let logger = Logger(subsystem: "com.yourapp.ingestion", category: "Bridge")
    private let appGroupID = "group.com.yourapp.ingestion"
    private let socketPath = "/tmp/clace-ingestion.sock"
    
    // NSFilePresenter requirements
    var presentedItemURL: URL?
    var presentedItemOperationQueue: OperationQueue = .main
    
    private var xpcConnection: NSXPCConnection?
    
    override init() {
        super.init()
        
        // Set up file presenter for App Group monitoring
        if let groupURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupID) {
            presentedItemURL = groupURL.appendingPathComponent("pending_payload.json")
        }
    }
    
    func start() {
        // Register as file presenter to receive change notifications
        NSFileCoordinator.addFilePresenter(self)
        logger.info("IngestionBridge started, monitoring: \(self.presentedItemURL?.path ?? "nil")")
        
        // Process any existing payload
        processPayloadIfExists()
    }
    
    func stop() {
        NSFileCoordinator.removeFilePresenter(self)
        logger.info("IngestionBridge stopped")
    }
    
    // Called when the monitored file changes
    func presentedItemDidChange() {
        logger.info("Payload file changed, processing...")
        processPayloadIfExists()
    }
    
    private func processPayloadIfExists() {
        guard let payloadURL = presentedItemURL else { return }
        
        let coordinator = NSFileCoordinator(filePresenter: self)
        var coordinatorError: NSError?
        
        coordinator.coordinate(readingItemAt: payloadURL, options: [], error: &coordinatorError) { url in
            do {
                let data = try Data(contentsOf: url)
                
                // Forward to ingestion service
                self.forwardToIngestionService(data: data)
                
                // Remove processed file
                try FileManager.default.removeItem(at: url)
                self.logger.info("Payload processed and removed")
                
            } catch {
                if (error as NSError).code != NSFileReadNoSuchFileError {
                    self.logger.error("Failed to read payload: \(error.localizedDescription)")
                }
            }
        }
    }
    
    private func forwardToIngestionService(data: Data) {
        // Option 1: Direct Unix socket connection
        forwardViaUnixSocket(data: data)
        
        // Option 2: Via XPC to launchd agent (if implemented)
        // forwardViaXPC(data: data)
    }
    
    private func forwardViaUnixSocket(data: Data) {
        let socket = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socket >= 0 else {
            logger.error("Failed to create socket")
            return
        }
        defer { close(socket) }
        
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        socketPath.withCString { ptr in
            withUnsafeMutablePointer(to: &addr.sun_path.0) { dest in
                _ = strcpy(dest, ptr)
            }
        }
        
        let connectResult = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPtr in
                Darwin.connect(socket, sockaddrPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        
        guard connectResult == 0 else {
            logger.error("Failed to connect to ingestion service: \(errno)")
            return
        }
        
        // Send payload with newline delimiter
        var payload = data
        payload.append(contentsOf: "\n".utf8)
        
        _ = payload.withUnsafeBytes { ptr in
            send(socket, ptr.baseAddress, ptr.count, 0)
        }
        
        // Read response
        var response = [UInt8](repeating: 0, count: 4096)
        let bytesRead = recv(socket, &response, response.count, 0)
        
        if bytesRead > 0 {
            let responseStr = String(bytes: response.prefix(bytesRead), encoding: .utf8) ?? ""
            logger.info("Ingestion service response: \(responseStr)")
        }
    }
}
```

### Step 5: Adapt Background Script

Convert the Chrome Service Worker to a Safari Background Page:

```javascript
// background.js (Safari Background Page)

const EXTENSION_ID = browser.runtime.id;

// Track processed tabs to avoid duplicates
let lastProcessedTabId = null;
let lastProcessedUrl = null;
let lastProcessedTime = 0;
const DEBOUNCE_MS = 1000;

/**
 * Send payload to native app via SafariWebExtensionHandler
 */
async function sendToNativeApp(payload) {
    try {
        const response = await browser.runtime.sendNativeMessage(EXTENSION_ID, {
            action: 'ingest',
            ...payload
        });
        
        console.log('[Background] Native app response:', response);
        return response;
    } catch (error) {
        console.error('[Background] Native messaging error:', error);
        throw error;
    }
}

/**
 * Handle tab activation
 */
async function handleTabActivated(activeInfo) {
    try {
        const tab = await browser.tabs.get(activeInfo.tabId);
        await processTab(tab, 'activated');
    } catch (error) {
        console.error('[Background] Tab activation error:', error);
    }
}

/**
 * Handle URL changes (SPA navigation)
 */
function handleTabUpdated(tabId, changeInfo, tab) {
    if (!changeInfo.url || !tab.active) return;
    
    console.log('[Background] URL changed:', changeInfo.url);
    processTab(tab, 'url-changed');
}

/**
 * Process a tab for content extraction
 */
async function processTab(tab, trigger) {
    // Skip non-http URLs
    if (!tab.url || !tab.url.startsWith('http')) {
        return;
    }
    
    const now = Date.now();
    
    // Debounce duplicate processing
    if (tab.id === lastProcessedTabId && tab.url === lastProcessedUrl) {
        if (now - lastProcessedTime < DEBOUNCE_MS) {
            console.log('[Background] Skipping (debounced):', tab.url);
            return;
        }
    }
    
    lastProcessedTabId = tab.id;
    lastProcessedUrl = tab.url;
    lastProcessedTime = now;
    
    console.log(`[Background] Processing tab (${trigger}):`, tab.url);
    
    try {
        // Request extraction from content script
        const response = await browser.tabs.sendMessage(tab.id, {
            type: 'extract',
            url: tab.url
        });
        
        if (response && response.success && response.data) {
            console.log('[Background] Extraction successful, sending to native app');
            await sendToNativeApp(response.data);
        } else {
            console.log('[Background] Extraction skipped:', response?.reason);
        }
    } catch (error) {
        console.error('[Background] Extraction error:', error);
    }
}

// Register event listeners
browser.tabs.onActivated.addListener(handleTabActivated);
browser.tabs.onUpdated.addListener(handleTabUpdated);

console.log('[Background] Safari extension initialized');
```

### Step 6: Update Manifest for Safari

```json
{
    "manifest_version": 2,
    "name": "Content Ingestion Pipeline",
    "version": "1.0.0",
    "description": "Captures and ingests content from browser tabs",
    
    "permissions": [
        "tabs",
        "activeTab",
        "nativeMessaging",
        "storage",
        "<all_urls>"
    ],
    
    "background": {
        "scripts": ["background.js"],
        "persistent": false
    },
    
    "content_scripts": [
        {
            "matches": ["<all_urls>"],
            "js": ["content/index.js"],
            "run_at": "document_idle",
            "all_frames": false
        }
    ],
    
    "browser_specific_settings": {
        "safari": {
            "strict_min_version": "14.0"
        }
    },
    
    "icons": {
        "48": "icons/icon-48.png",
        "96": "icons/icon-96.png",
        "128": "icons/icon-128.png"
    }
}
```

---

## Project Structure

```
safari-extension/
├── ContentIngestion/                    # Container App (macOS)
│   ├── ContentIngestion.xcodeproj
│   ├── ContentIngestion/
│   │   ├── AppDelegate.swift
│   │   ├── ViewController.swift         # Extension enable instructions
│   │   ├── IngestionBridge.swift         # App Group → Ingestion Service
│   │   ├── Assets.xcassets
│   │   └── Info.plist
│   │
│   ├── ContentIngestion Extension/       # Safari Web Extension
│   │   ├── SafariWebExtensionHandler.swift
│   │   ├── Info.plist
│   │   └── Resources/                    # Web Extension files
│   │       ├── manifest.json
│   │       ├── background.js
│   │       ├── content/
│   │       │   ├── index.js              # Main content script (from Chrome)
│   │       │   └── extractors/
│   │       │       ├── gdocs.js
│   │       │       └── jira.js
│   │       └── icons/
│   │
│   └── IngestionAgent/                   # Launchd Agent (optional)
│       ├── main.swift
│       ├── XPCService.swift
│       └── com.yourapp.ingestion.agent.plist
│
├── Shared/                               # Shared code
│   ├── PayloadTypes.swift
│   └── AppGroupConstants.swift
│
└── README.md
```

---

## Installation & Distribution

### Development Installation

1. **Build in Xcode**:
   ```bash
   cd safari-extension/ContentIngestion
   xcodebuild -scheme "ContentIngestion" -configuration Debug build
   ```

2. **Enable Developer Mode in Safari**:
   - Safari → Settings → Advanced → "Show Develop menu in menu bar"

3. **Allow Unsigned Extensions**:
   - Develop → Allow Unsigned Extensions (requires password, resets on Safari restart)

4. **Enable the Extension**:
   - Safari → Settings → Extensions → Enable "Content Ingestion"

### App Store Distribution

1. **Apple Developer Program**: Required ($99/year)

2. **Create App Store Connect Entry**:
   - Create App ID with App Groups capability
   - Create App Store listing

3. **Archive and Upload**:
   ```bash
   xcodebuild -scheme "ContentIngestion" -configuration Release archive \
     -archivePath ./build/ContentIngestion.xcarchive
   
   xcodebuild -exportArchive \
     -archivePath ./build/ContentIngestion.xcarchive \
     -exportPath ./build/export \
     -exportOptionsPlist ExportOptions.plist
   ```

4. **Submit for Review**:
   - Upload via Xcode or Transporter
   - Complete App Store Connect metadata
   - Submit for review

### Important App Store Guidelines

- **No donation links** unless registered nonprofit
- **No external payment** - must use Apple's payment system
- **No COVID-19 references** without official authorization
- **Extension name restrictions** - cannot use trademarked terms directly

---

## Development Workflow

### Making Changes to Content Scripts

Content scripts are shared between Chrome and Safari. Edit in `chrome-extension/content/` and copy to Safari:

```bash
# Sync content scripts from Chrome to Safari
cp -r chrome-extension/content/* \
  safari-extension/ContentIngestion/ContentIngestion\ Extension/Resources/content/
```

Or set up a symbolic link:
```bash
cd safari-extension/ContentIngestion/ContentIngestion\ Extension/Resources/
rm -rf content
ln -s ../../../../../chrome-extension/content content
```

### Testing the Extension

1. **Build and Run** in Xcode (Cmd+R)
2. **Open Safari** - it launches automatically
3. **Enable Extension** in Safari Settings
4. **Navigate** to a supported site
5. **Check Logs**:
   - Extension logs: Develop → Web Extension Background Pages
   - Native logs: Console.app → filter by "com.yourapp.ingestion"

### Debugging SafariWebExtensionHandler

```swift
// Add extensive logging
import os.log

let logger = Logger(subsystem: "com.yourapp.ingestion", category: "Handler")

func beginRequest(with context: NSExtensionContext) {
    logger.info("beginRequest called")
    logger.debug("Input items: \(context.inputItems)")
    // ...
}
```

View logs in Console.app or via:
```bash
log stream --predicate 'subsystem == "com.yourapp.ingestion"'
```

### Debugging Content Scripts

Same as Chrome - use Safari's Web Inspector:

1. Develop → Show Web Inspector
2. Select the page's context
3. Check Console for `[ContentScript]` logs

---

## Troubleshooting

### Extension Not Loading

**Symptom**: Extension doesn't appear in Safari Settings

**Solutions**:
1. Ensure "Allow Unsigned Extensions" is enabled (Develop menu)
2. Check that the extension target is properly signed
3. Verify App Groups are configured on both targets
4. Check Console.app for Safari extension loading errors

### Native Messaging Not Working

**Symptom**: `browser.runtime.sendNativeMessage()` returns undefined or errors

**Solutions**:
1. Verify SafariWebExtensionHandler is properly implemented
2. Check that the extension bundle ID matches the native app
3. Look for errors in Console.app
4. Ensure the response is sent via `context.completeRequest()`

### App Group Access Denied

**Symptom**: Cannot read/write to App Group container

**Solutions**:
1. Verify App Group ID is identical in both targets
2. Check entitlements file includes the App Group
3. Ensure both targets have App Groups capability enabled
4. Clean build folder and rebuild

### Ingestion Service Connection Failed

**Symptom**: Payloads not reaching ingestion service

**Solutions**:
1. Verify ingestion service is running: `lsof -i -P | grep clace`
2. Check Unix socket exists: `ls -la /tmp/clace-ingestion.sock`
3. Verify Container App has network entitlements
4. Check firewall settings

### Content Script Not Injecting

**Symptom**: Extraction not happening on page load

**Solutions**:
1. Check manifest.json `content_scripts` configuration
2. Verify extension has permission for the site
3. Check Safari's extension permissions for the site
4. Look for JavaScript errors in Web Inspector

---

## Integration with Existing Infrastructure

### Shared Ingestion Service

The Safari extension uses the same ingestion service as the Chrome extension:

```
Chrome Extension ──► Native Host ──► Unix Socket ──┐
                                                   │
Safari Extension ──► App Extension ──► App Group ──┼──► Ingestion Service
                          │                        │
                          └──► Container App ──────┘
```

### Payload Format Compatibility

Both extensions send identical `CapturePayload` format:

```typescript
interface CapturePayload {
    source: string;      // "slack" | "gmail" | "jira" | "gdocs" | etc.
    url: string;         // Canonical URL
    content: string;     // Extracted text
    title?: string;
    author?: string;
    channel?: string;
    timestamp?: number;  // Unix seconds
}
```

### Database Schema

Content from both browsers is stored in the same SQLite database with identical schema:

```sql
-- content_sources tracks origin regardless of browser
CREATE TABLE content_sources (
    id INTEGER PRIMARY KEY,
    source_type TEXT NOT NULL,      -- "slack", "gmail", etc.
    source_path TEXT NOT NULL,      -- Canonical URL
    content_hash TEXT NOT NULL,
    ehl_doc_id TEXT NOT NULL,
    chunk_count INTEGER NOT NULL,
    -- No browser field needed - content is browser-agnostic
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

---

## Summary

Implementing Safari support requires:

1. **Convert Chrome extension** using `xcrun safari-web-extension-converter`
2. **Implement SafariWebExtensionHandler** for native messaging
3. **Configure App Groups** for inter-process communication
4. **Implement IngestionBridge** in Container App to forward payloads
5. **Adapt background script** from Service Worker to Background Page

Your existing content scripts (extractors) transfer with zero changes. The main work is in the native Swift code for bridging the sandbox boundary.

The architecture is more complex than Chrome due to Apple's sandboxing, but provides the same end result: extracted content flows to the shared ingestion service for deduplication, chunking, and storage.
