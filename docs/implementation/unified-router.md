# Unified Extraction Router

## Overview

The Unified Router is the central orchestrator for content extraction across all desktop applications. It monitors visible windows across all displays, determines the optimal extraction method for each application, and coordinates extraction timing using perceptual hash-based change detection.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           UNIFIED ROUTER                                 │
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Window    │  │  Extractor  │  │   Change    │  │   Event     │    │
│  │   Tracker   │  │   Router    │  │  Detector   │  │  Listener   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│         │                │                │                │            │
│         └────────────────┴────────────────┴────────────────┘            │
│                                   │                                      │
└───────────────────────────────────┼──────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
    ┌─────────────┐          ┌─────────────┐          ┌─────────────┐
    │Accessibility│          │   Chrome    │          │     OCR     │
    │  Extractor  │          │  Extension  │          │  Extractor  │
    │   (Rust)    │          │    (JS)     │          │  (Swift)    │
    └─────────────┘          └─────────────┘          └─────────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    ▼
                           ┌─────────────────┐
                           │    Ingestion    │
                           │    Service      │
                           └─────────────────┘
```

## Core Components

### 1. Window Tracker

Maintains real-time state of all visible windows across all displays.

**Responsibilities:**
- Monitor `NSWorkspace` notifications for app activation/deactivation
- Track window creation, destruction, and focus changes
- Maintain window-to-display mapping
- Detect window title changes (tab switches in browsers)

**State per window:**
```
WindowState {
    window_id: CGWindowID
    display_id: CGDirectDisplayID
    bundle_id: String
    window_title: String
    bounds: CGRect
    extractor_type: ExtractorType
    last_visual_hash: u64
    last_content_hash: String  // SHA-256 of extracted text
    last_extraction_time: Timestamp
    extraction_count: u32
}
```

### 2. Extractor Router

Determines which extraction method to use for each application.

**Decision logic:**
```
fn determine_extractor(bundle_id: &str, window_title: &str) -> ExtractorType {
    // Priority 1: Chrome extension (if browser with extension installed)
    if is_browser_with_extension(bundle_id) {
        return ExtractorType::Chrome
    }
    
    // Priority 2: Accessibility (if app has good AX support)
    if is_accessibility_supported(bundle_id) {
        return ExtractorType::Accessibility
    }
    
    // Priority 3: OCR (fallback for everything else)
    return ExtractorType::OCR
}
```

**Supported applications registry:**

| Bundle ID | Extractor | Notes |
|-----------|-----------|-------|
| `com.microsoft.Word` | Accessibility | Full document extraction |
| `com.microsoft.Excel` | Accessibility | Cell content extraction |
| `com.microsoft.Powerpoint` | Accessibility | Slide text extraction |
| `com.microsoft.Outlook` | Accessibility | Email content |
| `com.microsoft.teams2` | Accessibility | Chat messages |
| `com.tinyspeck.slackmacgap` | Accessibility | Channel messages |
| `com.apple.iWork.Pages` | Accessibility | Document text |
| `com.apple.iWork.Numbers` | Accessibility | Spreadsheet content |
| `com.apple.iWork.Keynote` | Accessibility | Presentation text |
| `com.apple.TextEdit` | Accessibility | Plain/rich text |
| `com.google.Chrome` | Chrome Extension | Web page content |
| `com.brave.Browser` | Chrome Extension | Web page content |
| `com.microsoft.edgemac` | Chrome Extension | Web page content |
| `*` (all others) | OCR | Fallback |

### 3. Change Detector

Determines when content has changed and extraction is needed.

**Perceptual hash algorithm (aHash):**
1. Capture window screenshot (5-10ms, GPU-accelerated)
2. Resize to 8x8 grayscale (64 pixels)
3. Calculate average brightness
4. Generate 64-bit hash: bit=1 if pixel > average, else 0
5. Compare with previous hash using Hamming distance
6. If distance >= threshold (default: 8), content has changed

**Change detection flow:**
```
┌─────────────────┐
│  Timer fires    │  (every 2-5 seconds)
│  or event       │
└────────┬────────┘
         ▼
┌─────────────────┐
│ For each window │
└────────┬────────┘
         ▼
┌─────────────────┐
│ Capture window  │  ~5ms (CGWindowListCreateImage)
└────────┬────────┘
         ▼
┌─────────────────┐
│ Compute hash    │  ~1ms (8x8 resize + average)
└────────┬────────┘
         ▼
┌─────────────────┐     distance < threshold
│ Compare hashes  │ ─────────────────────────► SKIP
└────────┬────────┘
         │ distance >= threshold
         ▼
┌─────────────────┐
│ Trigger extract │
└─────────────────┘
```

### 4. Event Listener

Handles system events that trigger immediate extraction.

**Events monitored:**
- `NSWorkspace.didActivateApplicationNotification` — App switch
- `NSWorkspace.activeSpaceDidChangeNotification` — Space/desktop change
- Window title change (polled every 2s) — Tab switch
- Chrome extension messages — Web content updates

## Extraction Flows

### Flow 1: Accessibility-Supported App

```
User switches to Microsoft Word
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│ Event: didActivateApplicationNotification               │
│ Bundle ID: com.microsoft.Word                           │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Router: determine_extractor("com.microsoft.Word")       │
│ Result: ExtractorType::Accessibility                    │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Change Detector: capture_and_hash(window_id)            │
│ Hash changed? YES (new window)                          │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Accessibility Extractor: extract("com.microsoft.Word")  │
│ Returns: ExtractedContent { text, title, metadata }     │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Ingestion Service: ingest(content)                      │
│ Dedup, chunk, store                                     │
└─────────────────────────────────────────────────────────┘
```

### Flow 2: Chrome Extension

```
User switches tab in Chrome
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│ Chrome Extension: onActivated event                     │
│ Extracts DOM content using Readability                  │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Native Messaging: send to host                          │
│ Payload: { url, title, content, source }                │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Router: receive_chrome_content(payload)                 │
│ Updates window state, skips hash check                  │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Ingestion Service: ingest(content)                      │
└─────────────────────────────────────────────────────────┘
```

### Flow 3: OCR Fallback

```
User switches to Preview (PDF viewer)
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│ Event: didActivateApplicationNotification               │
│ Bundle ID: com.apple.Preview                            │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Router: determine_extractor("com.apple.Preview")        │
│ Result: ExtractorType::OCR (not in supported list)      │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Change Detector: capture_and_hash(window_id)            │
│ Hash changed? YES                                       │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ OCR Extractor: capture_and_ocr(window_id)               │
│ 1. Capture window screenshot                            │
│ 2. Run Vision framework OCR                             │
│ 3. Return extracted text                                │
└────────┬────────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│ Ingestion Service: ingest(content)                      │
└─────────────────────────────────────────────────────────┘
```

## Multi-Display Support

The router tracks windows across all connected displays independently.

**Per-display state:**
```
DisplayState {
    display_id: CGDirectDisplayID
    bounds: CGRect
    is_main: bool
    active_windows: Vec<WindowID>
}
```

**Multi-display extraction:**
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Display 1     │     │   Display 2     │     │   Display 3     │
│   (Main)        │     │   (External)    │     │   (External)    │
├─────────────────┤     ├─────────────────┤     ├─────────────────┤
│ VS Code         │     │ Slack           │     │ Chrome          │
│ → Accessibility │     │ → Accessibility │     │ → Extension     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 ▼
                    ┌─────────────────────────┐
                    │     Unified Router      │
                    │  (processes in parallel)│
                    └─────────────────────────┘
```

## Timing and Throttling

### Capture intervals

| Condition | Interval | Rationale |
|-----------|----------|-----------|
| Normal (AC power) | 5 seconds | Balance responsiveness and resources |
| Battery power | 15 seconds | Conserve battery |
| Thermal throttled | 30 seconds | Prevent overheating |
| User idle (>30s) | 60 seconds | User not actively working |
| App switch | Immediate | Capture new context |
| Tab switch | Immediate | Capture new content |

### Debouncing

- **Scroll debounce:** Wait 1 second after scroll stops
- **Typing debounce:** Wait 2 seconds after last keystroke
- **Focus debounce:** Wait 0.5 seconds after focus change

### Rate limiting

- **Min interval:** 3 seconds between extractions for same window
- **Max interval:** 60 seconds forced extraction (ensures periodic capture)
- **Burst limit:** Max 10 extractions per minute per window

## Privacy and Security

### Blocked applications

The router maintains a blocklist of sensitive applications:

```
BLOCKED_BUNDLE_IDS = [
    "com.1password.*",           // Password managers
    "com.agilebits.onepassword*",
    "com.lastpass.LastPass",
    "com.bitwarden.desktop",
    "com.dashlane.Dashlane",
    
    "com.apple.systempreferences", // System settings
    "com.apple.SecurityAgent",
    
    "com.apple.keychainaccess",   // Keychain
    
    "*banking*",                  // Banking apps (pattern match)
    "*bank*",
]
```

### Content redaction

Before sending to ingestion, content is scanned for:
- Credit card numbers (redacted to `[CARD]`)
- Social security numbers (redacted to `[SSN]`)
- API keys/tokens (redacted to `[API_KEY]`)
- Passwords in common formats (redacted to `[PASSWORD]`)

## IPC Protocol

### Router ↔ Accessibility Extractor

Communication via Unix domain socket or direct library call.

**Request:**
```json
{
    "action": "extract",
    "bundle_id": "com.microsoft.Word",
    "window_id": 12345
}
```

**Response:**
```json
{
    "success": true,
    "content": {
        "source": "word",
        "title": "Document.docx",
        "content": "...",
        "app_name": "Microsoft Word",
        "timestamp": 1707500000
    }
}
```

### Router ↔ Chrome Extension

Communication via Chrome Native Messaging protocol.

**From extension:**
```json
{
    "type": "content",
    "payload": {
        "url": "https://example.com",
        "title": "Page Title",
        "content": "...",
        "source": "chrome"
    }
}
```

**To extension:**
```json
{
    "type": "status",
    "received": true
}
```

### Router ↔ OCR Extractor

Direct library call (same process) or IPC.

**Request:**
```json
{
    "action": "capture_and_ocr",
    "window_id": 12345,
    "bounds": { "x": 0, "y": 0, "width": 1920, "height": 1080 }
}
```

**Response:**
```json
{
    "success": true,
    "content": {
        "text": "...",
        "confidence": 0.95,
        "processing_time_ms": 150
    }
}
```

## Configuration

### Router configuration file

Location: `~/.config/unified-router/config.toml`

```toml
[general]
enabled = true
log_level = "info"

[timing]
base_interval_seconds = 5
battery_interval_seconds = 15
idle_interval_seconds = 60
min_interval_seconds = 3
max_interval_seconds = 60

[change_detection]
hash_sensitivity = 8  # Hamming distance threshold (0-64)
title_change_triggers_extract = true

[extractors]
accessibility_enabled = true
chrome_extension_enabled = true
ocr_enabled = true
ocr_fallback_only = true  # Only use OCR when others unavailable

[privacy]
blocked_apps = ["com.1password.*", "com.lastpass.*"]
redact_credit_cards = true
redact_ssn = true
redact_api_keys = true

[multi_display]
enabled = true
capture_all_displays = true
```

### Runtime configuration

The router exposes a control socket for runtime configuration:

```bash
# Pause extraction
echo '{"action": "pause"}' | nc -U /tmp/unified-router.sock

# Resume extraction
echo '{"action": "resume"}' | nc -U /tmp/unified-router.sock

# Get status
echo '{"action": "status"}' | nc -U /tmp/unified-router.sock

# Add app to blocklist
echo '{"action": "block", "bundle_id": "com.example.app"}' | nc -U /tmp/unified-router.sock
```

## Metrics and Monitoring

### Exposed metrics

```
unified_router_extractions_total{extractor="accessibility|chrome|ocr", status="success|error"}
unified_router_extraction_duration_seconds{extractor="accessibility|chrome|ocr"}
unified_router_hash_comparisons_total{result="changed|unchanged"}
unified_router_windows_tracked
unified_router_displays_tracked
```

### Health check endpoint

```bash
curl http://localhost:9090/health
# {"status": "healthy", "uptime_seconds": 3600, "extractions_total": 1234}
```

## Error Handling

### Extractor failures

```
Extraction attempt
         │
         ▼
    ┌────┴────┐
    │ Success │──────────────────────► Send to ingestion
    └────┬────┘
         │ Failure
         ▼
    ┌─────────────────┐
    │ Retry (max 2x)  │
    └────────┬────────┘
             │ Still failing
             ▼
    ┌─────────────────┐
    │ Fallback to OCR │  (if not already OCR)
    └────────┬────────┘
             │ OCR also fails
             ▼
    ┌─────────────────┐
    │ Log error, skip │
    │ Mark window as  │
    │ "problematic"   │
    └─────────────────┘
```

### Graceful degradation

| Failure | Fallback |
|---------|----------|
| Accessibility extractor unavailable | Use OCR |
| Chrome extension not responding | Use OCR for browser windows |
| OCR extractor unavailable | Skip extraction, log warning |
| Ingestion service unavailable | Queue locally, retry later |

## Implementation Language

The Unified Router will be implemented in **Rust** for:
- Performance (low latency event handling)
- Memory safety (long-running daemon)
- Easy integration with existing accessibility extractor
- Cross-platform potential (future Windows/Linux support)

### Crate dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }  # Async runtime
core-foundation = "0.9"      # macOS APIs
core-graphics = "0.23"       # Screen capture, window list
cocoa = "0.25"               # NSWorkspace notifications
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"                 # Config parsing
tracing = "0.1"              # Logging
sha2 = "0.10"                # Content hashing
```

## File Structure

```
unified-router/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, daemon setup
│   ├── lib.rs               # Library exports
│   ├── router.rs            # Core routing logic
│   ├── window_tracker.rs    # Window state management
│   ├── change_detector.rs   # Perceptual hash implementation
│   ├── event_listener.rs    # System event handling
│   ├── extractors/
│   │   ├── mod.rs
│   │   ├── accessibility.rs # AX extractor integration
│   │   ├── chrome.rs        # Native messaging client
│   │   └── ocr.rs           # OCR extractor integration
│   ├── config.rs            # Configuration management
│   ├── privacy.rs           # Blocklist and redaction
│   ├── metrics.rs           # Prometheus metrics
│   └── ipc.rs               # Control socket
└── tests/
    ├── router_tests.rs
    ├── change_detector_tests.rs
    └── integration_tests.rs
```

## Deployment

### As a LaunchAgent (recommended)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.yourapp.unified-router</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/unified-router</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/unified-router.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/unified-router.err</string>
</dict>
</plist>
```

### Required permissions

- **Accessibility:** Required for window tracking and accessibility extraction
- **Screen Recording:** Required for OCR capture and hash computation
- **Automation (optional):** Only if using AppleScript fallbacks

## Future Enhancements

1. **Windows support:** Abstract platform-specific code behind traits
2. **Linux support:** Use AT-SPI for accessibility, X11/Wayland for window tracking
3. **ML-based change detection:** Replace perceptual hash with learned embeddings
4. **Semantic deduplication:** Detect similar content across different windows
5. **User activity prediction:** Pre-fetch content for likely next windows

---

## Implementation Details by Feature

This section provides concrete Rust crates, code snippets, and implementation guidance for each feature of the Unified Router.

### Feature 1: Window Tracking and Enumeration

**Purpose:** Track all visible windows across all displays, get window metadata (title, bounds, owner app).

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `x-win` | Cross-platform window info (macOS, Windows, Linux) | [lib.rs/crates/x-win](https://lib.rs/crates/x-win) |
| `core-graphics` | Low-level macOS window APIs | [docs.rs/core-graphics](https://docs.rs/core-graphics) |

**Code Example using `x-win`:**

```rust
use x_win::{get_active_window, get_open_windows, WindowInfo};

fn get_all_windows() -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
    let windows = get_open_windows()?;
    Ok(windows)
}

fn get_focused_window() -> Result<WindowInfo, Box<dyn std::error::Error>> {
    let active = get_active_window()?;
    println!("Active window: {} ({})", active.title, active.info.name);
    Ok(active)
}
```

**Low-level macOS approach using `CGWindowListCopyWindowInfo`:**

```rust
use core_graphics::display::{
    CGWindowListCopyWindowInfo, 
    kCGWindowListOptionOnScreenOnly,
    kCGWindowListExcludeDesktopElements,
    kCGNullWindowID
};

fn list_windows() {
    let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let window_list = unsafe { 
        CGWindowListCopyWindowInfo(options, kCGNullWindowID) 
    };
    // Parse CFArray of CFDictionary entries
    // Keys: kCGWindowOwnerPID, kCGWindowNumber, kCGWindowName, kCGWindowBounds
}
```


### Feature 2: Application Focus Change Detection

**Purpose:** Detect when user switches between applications to trigger immediate extraction.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `frontmost` | macOS app focus detection via NSWorkspace | [lib.rs/crates/frontmost](https://lib.rs/crates/frontmost) |
| `objc2` + `objc2_app_kit` | Low-level Objective-C bindings | [docs.rs/objc2](https://docs.rs/objc2) |

**Code Example using `frontmost`:**

```rust
use frontmost::app::FrontmostApp;
use frontmost::{Detector, start_nsrunloop};

struct AppMonitor {
    current_app: String,
}

impl FrontmostApp for AppMonitor {
    fn set_frontmost(&mut self, new_value: &str) {
        self.current_app = new_value.to_string();
    }
    
    fn update(&self) {
        println!("App switched to: {}", self.current_app);
        // Trigger extraction here
    }
}

fn main() {
    let monitor = AppMonitor { current_app: String::new() };
    Detector::init(Box::new(monitor));
    start_nsrunloop!();  // Blocks, runs event loop
}
```

**NSWorkspace notification (low-level):**

The `NSWorkspaceDidActivateApplicationNotification` fires when the frontmost app changes. This is the macOS-native way to detect app switches without polling.


### Feature 3: Perceptual Hash (Change Detection)

**Purpose:** Detect visual changes in windows without expensive pixel-by-pixel comparison.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `image_hasher` | Perceptual hashing (aHash, dHash, pHash) | [lib.rs/crates/image_hasher](https://lib.rs/crates/image_hasher) |
| `blockhash` | Blockhash algorithm for image similarity | [lib.rs/crates/blockhash](https://lib.rs/crates/blockhash) |

**Code Example using `image_hasher`:**

```rust
use image_hasher::{HasherConfig, HashAlg};

fn compute_hash_and_compare() {
    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::Mean)  // aHash - fastest
        .hash_size(8, 8)          // 64-bit hash
        .to_hasher();

    let image1 = image::open("screenshot1.png").unwrap();
    let image2 = image::open("screenshot2.png").unwrap();

    let hash1 = hasher.hash_image(&image1);
    let hash2 = hasher.hash_image(&image2);

    let distance = hash1.dist(&hash2);  // Hamming distance
    
    if distance < 8 {
        println!("Images are similar (no significant change)");
    } else {
        println!("Images differ significantly, trigger extraction");
    }
}
```

**Manual aHash implementation (minimal dependencies):**

```rust
fn compute_ahash(pixels: &[u8], width: usize, height: usize) -> u64 {
    // 1. Resize to 8x8 (use image crate or manual downsampling)
    // 2. Convert to grayscale
    // 3. Calculate average brightness
    let avg: u8 = pixels.iter().map(|&p| p as u32).sum::<u32>() as u8 / 64;
    
    // 4. Build hash: bit=1 if pixel > average
    let mut hash: u64 = 0;
    for (i, &pixel) in pixels.iter().enumerate().take(64) {
        if pixel > avg {
            hash |= 1 << i;
        }
    }
    hash
}

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}
```


### Feature 4: Screen/Window Capture

**Purpose:** Capture screenshots of specific windows for hash computation and OCR.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `screenshots` | Cross-platform screen capture | [lib.rs/crates/screenshots](https://lib.rs/crates/screenshots) |
| `crabgrab` | Window/display capture for macOS/Windows | [lib.rs/crates/crabgrab](https://lib.rs/crates/crabgrab) |

**Code Example using `screenshots`:**

```rust
use screenshots::Screen;

fn capture_all_displays() {
    let screens = Screen::all().unwrap();
    
    for screen in screens {
        println!("Display: {:?}", screen.display_info);
        
        // Capture full display
        let image = screen.capture().unwrap();
        image.save(format!("display_{}.png", screen.display_info.id)).unwrap();
        
        // Capture specific region
        let region = screen.capture_area(100, 100, 800, 600).unwrap();
    }
}

fn capture_display_at_point(x: i32, y: i32) {
    let screen = Screen::from_point(x, y).unwrap();
    let image = screen.capture().unwrap();
}
```

**macOS-specific window capture:**

```rust
// Using CGWindowListCreateImage for specific window capture
use core_graphics::display::CGWindowListCreateImage;
use core_graphics::geometry::CGRect;

fn capture_window(window_id: u32, bounds: CGRect) -> Option<CGImage> {
    unsafe {
        CGWindowListCreateImage(
            bounds,
            kCGWindowListOptionIncludingWindow,
            window_id,
            kCGWindowImageBoundsIgnoreFraming | kCGWindowImageBestResolution
        )
    }
}
```


### Feature 5: Chrome Native Messaging

**Purpose:** Communicate with Chrome extension to receive web page content.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `chrome-native-messaging` | Native messaging protocol | [github.com/neon64/chrome-native-messaging](https://github.com/neon64/chrome-native-messaging) |
| `native_messaging` | Cross-browser native messaging | [crates.io/crates/native_messaging](https://crates.io/crates/native_messaging) |

**Protocol:** Chrome native messaging uses stdin/stdout with length-prefixed JSON:
- First 4 bytes: message length (little-endian u32)
- Remaining bytes: UTF-8 JSON message

**Code Example:**

```rust
use std::io::{self, Read, Write};

fn read_message() -> io::Result<serde_json::Value> {
    let mut len_bytes = [0u8; 4];
    io::stdin().read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    let mut buffer = vec![0u8; len];
    io::stdin().read_exact(&mut buffer)?;
    
    let message: serde_json::Value = serde_json::from_slice(&buffer)?;
    Ok(message)
}

fn write_message(msg: &serde_json::Value) -> io::Result<()> {
    let json = serde_json::to_vec(msg)?;
    let len = (json.len() as u32).to_le_bytes();
    
    io::stdout().write_all(&len)?;
    io::stdout().write_all(&json)?;
    io::stdout().flush()?;
    Ok(())
}

fn main() {
    loop {
        match read_message() {
            Ok(msg) => {
                // Process message from Chrome extension
                let response = serde_json::json!({"status": "received"});
                write_message(&response).unwrap();
            }
            Err(_) => break,
        }
    }
}
```

**Manifest file** (`com.yourapp.native_host.json`):

```json
{
    "name": "com.yourapp.native_host",
    "description": "Unified Router Native Host",
    "path": "/usr/local/bin/unified-router",
    "type": "stdio",
    "allowed_origins": ["chrome-extension://YOUR_EXTENSION_ID/"]
}
```


### Feature 6: Unix Domain Socket IPC

**Purpose:** Control socket for runtime configuration and inter-process communication.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `parity-tokio-ipc` | Cross-platform IPC (Unix sockets / named pipes) | [lib.rs/crates/parity-tokio-ipc](https://lib.rs/crates/parity-tokio-ipc) |
| `tokio` | Async runtime with Unix socket support | [docs.rs/tokio](https://docs.rs/tokio) |

**Code Example using `tokio`:**

```rust
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn start_control_socket() -> std::io::Result<()> {
    let socket_path = "/tmp/unified-router.sock";
    
    // Remove existing socket
    let _ = std::fs::remove_file(socket_path);
    
    let listener = UnixListener::bind(socket_path)?;
    
    loop {
        let (mut stream, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            
            let request: serde_json::Value = 
                serde_json::from_slice(&buf[..n]).unwrap();
            
            let response = match request["action"].as_str() {
                Some("status") => serde_json::json!({"status": "running"}),
                Some("pause") => serde_json::json!({"status": "paused"}),
                Some("resume") => serde_json::json!({"status": "resumed"}),
                _ => serde_json::json!({"error": "unknown action"}),
            };
            
            stream.write_all(response.to_string().as_bytes()).await.unwrap();
        });
    }
}
```

**Code Example using `parity-tokio-ipc`:**

```rust
use parity_tokio_ipc::Endpoint;
use futures::stream::StreamExt;

async fn start_ipc_server() {
    let endpoint = Endpoint::new("/tmp/unified-router.sock".into());
    
    let mut incoming = endpoint.incoming().expect("Failed to bind");
    
    while let Some(conn) = incoming.next().await {
        match conn {
            Ok(stream) => {
                tokio::spawn(handle_connection(stream));
            }
            Err(e) => eprintln!("Connection error: {:?}", e),
        }
    }
}
```


### Feature 7: Configuration File Parsing

**Purpose:** Load and parse TOML configuration files.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `toml` | TOML parser | [docs.rs/toml](https://docs.rs/toml) |
| `serde` | Serialization framework | [docs.rs/serde](https://docs.rs/serde) |
| `config` | Layered configuration | [docs.rs/config](https://docs.rs/config) |

**Code Example:**

```rust
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    general: GeneralConfig,
    timing: TimingConfig,
    change_detection: ChangeDetectionConfig,
    privacy: PrivacyConfig,
}

#[derive(Debug, Deserialize)]
struct GeneralConfig {
    enabled: bool,
    log_level: String,
}

#[derive(Debug, Deserialize)]
struct TimingConfig {
    base_interval_seconds: u64,
    battery_interval_seconds: u64,
    min_interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct ChangeDetectionConfig {
    hash_sensitivity: u32,
    title_change_triggers_extract: bool,
}

#[derive(Debug, Deserialize)]
struct PrivacyConfig {
    blocked_apps: Vec<String>,
    redact_credit_cards: bool,
    redact_ssn: bool,
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("unified-router/config.toml");
    
    let contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
```


### Feature 8: Privacy Filter (PII Redaction)

**Purpose:** Detect and redact sensitive information before sending to ingestion.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `regex` | Regular expressions | [docs.rs/regex](https://docs.rs/regex) |
| `lazy_static` | Compile-time regex initialization | [docs.rs/lazy_static](https://docs.rs/lazy_static) |

**PII Detection Patterns:**

```rust
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Credit card: 16 digits, optionally with spaces/dashes
    static ref CREDIT_CARD: Regex = Regex::new(
        r"\b(?:\d{4}[-\s]?){3}\d{4}\b"
    ).unwrap();
    
    // SSN: XXX-XX-XXXX format
    static ref SSN: Regex = Regex::new(
        r"\b\d{3}-\d{2}-\d{4}\b"
    ).unwrap();
    
    // API keys: common patterns (AWS, GitHub, etc.)
    static ref API_KEY: Regex = Regex::new(
        r"(?i)(api[_-]?key|apikey|secret[_-]?key|access[_-]?token)['\"]?\s*[:=]\s*['\"]?([a-zA-Z0-9_\-]{20,})"
    ).unwrap();
    
    // Email addresses
    static ref EMAIL: Regex = Regex::new(
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"
    ).unwrap();
    
    // Phone numbers (US format)
    static ref PHONE: Regex = Regex::new(
        r"\b(?:\+1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b"
    ).unwrap();
}

fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();
    
    result = CREDIT_CARD.replace_all(&result, "[REDACTED_CARD]").to_string();
    result = SSN.replace_all(&result, "[REDACTED_SSN]").to_string();
    result = API_KEY.replace_all(&result, "$1=[REDACTED_KEY]").to_string();
    result = EMAIL.replace_all(&result, "[REDACTED_EMAIL]").to_string();
    result = PHONE.replace_all(&result, "[REDACTED_PHONE]").to_string();
    
    result
}

// Luhn algorithm for credit card validation (reduce false positives)
fn is_valid_credit_card(number: &str) -> bool {
    let digits: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    
    let sum: u32 = digits.iter().rev().enumerate().map(|(i, &d)| {
        if i % 2 == 1 {
            let doubled = d * 2;
            if doubled > 9 { doubled - 9 } else { doubled }
        } else {
            d
        }
    }).sum();
    
    sum % 10 == 0
}
```


### Feature 9: Prometheus Metrics

**Purpose:** Expose metrics for monitoring extraction performance and health.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `prometheus` | Prometheus client library | [docs.rs/prometheus](https://docs.rs/prometheus) |
| `metrics` | Metrics facade | [docs.rs/metrics](https://docs.rs/metrics) |
| `metrics-exporter-prometheus` | Prometheus exporter | [docs.rs/metrics-exporter-prometheus](https://docs.rs/metrics-exporter-prometheus) |

**Code Example:**

```rust
use prometheus::{
    IntCounter, IntCounterVec, IntGauge, Histogram, HistogramVec,
    Opts, Registry, labels
};
use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    static ref EXTRACTIONS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("unified_router_extractions_total", "Total extractions"),
        &["extractor", "status"]
    ).unwrap();
    
    static ref EXTRACTION_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "unified_router_extraction_duration_seconds",
            "Extraction duration in seconds"
        ).buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["extractor"]
    ).unwrap();
    
    static ref WINDOWS_TRACKED: IntGauge = IntGauge::new(
        "unified_router_windows_tracked",
        "Number of windows being tracked"
    ).unwrap();
    
    static ref HASH_COMPARISONS: IntCounterVec = IntCounterVec::new(
        Opts::new("unified_router_hash_comparisons_total", "Hash comparisons"),
        &["result"]  // "changed" or "unchanged"
    ).unwrap();
}

fn init_metrics() {
    REGISTRY.register(Box::new(EXTRACTIONS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(EXTRACTION_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(WINDOWS_TRACKED.clone())).unwrap();
    REGISTRY.register(Box::new(HASH_COMPARISONS.clone())).unwrap();
}

fn record_extraction(extractor: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    EXTRACTIONS_TOTAL.with_label_values(&[extractor, status]).inc();
    EXTRACTION_DURATION.with_label_values(&[extractor]).observe(duration_secs);
}

// HTTP endpoint for Prometheus scraping
async fn metrics_handler() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```


### Feature 10: LaunchAgent Daemon Setup

**Purpose:** Run the router as a background service that starts on login.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `lunchctl` | LaunchAgent management | [lib.rs/crates/lunchctl](https://lib.rs/crates/lunchctl) |

**Code Example using `lunchctl`:**

```rust
use lunchctl::LaunchAgent;

fn install_launch_agent() -> Result<(), Box<dyn std::error::Error>> {
    let agent = LaunchAgent::new("com.yourapp.unified-router")
        .program_arguments(vec!["/usr/local/bin/unified-router".to_string()])
        .run_at_load(true)
        .keep_alive(true);
    
    agent.write()?;      // Write plist to ~/Library/LaunchAgents/
    agent.bootstrap()?;  // Load and start the agent
    
    Ok(())
}

fn uninstall_launch_agent() -> Result<(), Box<dyn std::error::Error>> {
    let agent = LaunchAgent::from_file("com.yourapp.unified-router")?;
    agent.boot_out()?;   // Stop the agent
    agent.remove()?;     // Delete the plist
    Ok(())
}

fn check_agent_status() -> bool {
    if let Ok(agent) = LaunchAgent::from_file("com.yourapp.unified-router") {
        agent.is_running().unwrap_or(false)
    } else {
        false
    }
}
```

**Manual plist creation:**

```rust
use std::fs;
use std::path::PathBuf;

fn create_plist() -> std::io::Result<()> {
    let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.yourapp.unified-router</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/unified-router</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/unified-router.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/unified-router.err</string>
</dict>
</plist>"#;

    let home = dirs::home_dir().unwrap();
    let path = home.join("Library/LaunchAgents/com.yourapp.unified-router.plist");
    fs::write(path, plist)?;
    Ok(())
}
```


### Feature 11: Multi-Display Enumeration

**Purpose:** Detect and enumerate all connected displays.

**Recommended Crates:**

| Crate | Description | Link |
|-------|-------------|------|
| `display-info` | Cross-platform display info | [github.com/nashaofu/display-info](https://github.com/nashaofu/display-info) |
| `core-graphics` | macOS display APIs | [docs.rs/core-graphics](https://docs.rs/core-graphics) |

**Code Example using Core Graphics:**

```rust
use core_graphics::display::{
    CGGetActiveDisplayList, CGDisplayBounds, CGMainDisplayID,
    CGDisplayIsMain, CGDisplayIsBuiltin
};

#[derive(Debug)]
struct DisplayInfo {
    id: u32,
    bounds: (f64, f64, f64, f64),  // x, y, width, height
    is_main: bool,
    is_builtin: bool,
}

fn get_all_displays() -> Vec<DisplayInfo> {
    let mut display_count: u32 = 0;
    
    // Get count first
    unsafe {
        CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut display_count);
    }
    
    let mut displays = vec![0u32; display_count as usize];
    
    unsafe {
        CGGetActiveDisplayList(display_count, displays.as_mut_ptr(), &mut display_count);
    }
    
    displays.iter().map(|&id| {
        let bounds = unsafe { CGDisplayBounds(id) };
        DisplayInfo {
            id,
            bounds: (bounds.origin.x, bounds.origin.y, bounds.size.width, bounds.size.height),
            is_main: unsafe { CGDisplayIsMain(id) } != 0,
            is_builtin: unsafe { CGDisplayIsBuiltin(id) } != 0,
        }
    }).collect()
}

fn get_display_containing_point(x: f64, y: f64) -> Option<u32> {
    let displays = get_all_displays();
    
    for display in displays {
        let (dx, dy, dw, dh) = display.bounds;
        if x >= dx && x < dx + dw && y >= dy && y < dy + dh {
            return Some(display.id);
        }
    }
    None
}
```


### Complete Cargo.toml

```toml
[package]
name = "unified-router"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full", "net", "sync", "time", "fs"] }

# macOS APIs
core-foundation = "0.9"
core-graphics = "0.23"
cocoa = "0.25"
objc2 = "0.5"
objc2-app-kit = "0.2"
objc2-foundation = "0.2"

# Window tracking (cross-platform option)
x-win = "5.5"

# App focus detection
frontmost = "1.1"

# Image processing and hashing
image = "0.25"
image_hasher = "3.1"

# Screen capture
screenshots = "0.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# IPC
parity-tokio-ipc = "0.9"

# Metrics
prometheus = "0.13"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
lazy_static = "1.4"
regex = "1"
sha2 = "0.10"
dirs = "5"
chrono = "0.4"

# LaunchAgent management
lunchctl = "0.2"

[target.'cfg(target_os = "macos")'.dependencies]
accessibility = "0.1"
accessibility-sys = "0.1"
```

### Summary of Features and Crates

| Feature | Primary Crate | Fallback |
|---------|---------------|----------|
| Window tracking | `x-win` | `core-graphics` |
| Focus detection | `frontmost` | `objc2-app-kit` |
| Perceptual hash | `image_hasher` | Manual implementation |
| Screen capture | `screenshots` | `core-graphics` |
| Chrome messaging | Manual stdin/stdout | `chrome-native-messaging` |
| IPC socket | `tokio` UnixListener | `parity-tokio-ipc` |
| Config parsing | `toml` + `serde` | - |
| PII redaction | `regex` | - |
| Metrics | `prometheus` | `metrics` |
| LaunchAgent | `lunchctl` | Manual plist |
| Display enum | `core-graphics` | `display-info` |



---

## Multi-Display Considerations

The following sections expand on how each feature handles multiple displays.

### Focus Detection with Multiple Displays

**The Problem:** `NSWorkspaceDidActivateApplicationNotification` only fires when the *frontmost application* changes — not when the user clicks on a different window of the same app on another display.

**Solution:** Combine app activation events with window-level tracking.

```rust
use std::collections::HashMap;
use x_win::{get_open_windows, WindowInfo};

struct MultiDisplayTracker {
    // Track the "active" window per display
    active_window_per_display: HashMap<u32, u64>,  // display_id -> window_id
    // Track last known window positions
    window_positions: HashMap<u64, u32>,  // window_id -> display_id
}

impl MultiDisplayTracker {
    /// Poll all windows and detect which display each is on
    fn update_window_positions(&mut self) -> Vec<WindowChange> {
        let mut changes = Vec::new();
        let windows = get_open_windows().unwrap_or_default();
        
        for window in windows {
            let display_id = self.get_display_for_window(&window);
            let window_id = window.id;
            
            // Check if this window moved to a different display
            if let Some(&old_display) = self.window_positions.get(&window_id) {
                if old_display != display_id {
                    changes.push(WindowChange::MovedDisplay {
                        window_id,
                        from: old_display,
                        to: display_id,
                    });
                }
            }
            
            self.window_positions.insert(window_id, display_id);
        }
        
        changes
    }
    
    /// Determine which display a window is on based on its center point
    fn get_display_for_window(&self, window: &WindowInfo) -> u32 {
        let center_x = window.position.x + window.position.width / 2;
        let center_y = window.position.y + window.position.height / 2;
        
        // Use display enumeration to find containing display
        get_display_containing_point(center_x as f64, center_y as f64)
            .unwrap_or(0)  // Default to main display
    }
    
    /// Track focus changes per display (not just global frontmost)
    fn update_focus_per_display(&mut self, windows: &[WindowInfo]) {
        // Group windows by display
        let mut windows_by_display: HashMap<u32, Vec<&WindowInfo>> = HashMap::new();
        
        for window in windows {
            let display = self.get_display_for_window(window);
            windows_by_display.entry(display).or_default().push(window);
        }
        
        // For each display, find the topmost window (lowest layer/z-order)
        for (display_id, display_windows) in windows_by_display {
            // Windows are typically returned in z-order (front to back)
            if let Some(topmost) = display_windows.first() {
                let prev = self.active_window_per_display.get(&display_id);
                if prev != Some(&topmost.id) {
                    // Focus changed on this display
                    self.active_window_per_display.insert(display_id, topmost.id);
                    // Trigger extraction for this window
                }
            }
        }
    }
}

enum WindowChange {
    MovedDisplay { window_id: u64, from: u32, to: u32 },
    FocusChanged { display_id: u32, window_id: u64 },
    Created { window_id: u64, display_id: u32 },
    Destroyed { window_id: u64 },
}
```

### Perceptual Hash with Multiple Displays

**The Problem:** Need to track hash state independently per window, not globally.

**Solution:** Store hashes in a per-window map, keyed by window ID.

```rust
use std::collections::HashMap;
use image_hasher::{HasherConfig, ImageHash};

struct PerWindowHashTracker {
    hasher: image_hasher::Hasher,
    // Store hash per window, not per display
    window_hashes: HashMap<u64, ImageHash>,
    // Sensitivity threshold
    threshold: u32,
}

impl PerWindowHashTracker {
    fn new(threshold: u32) -> Self {
        let hasher = HasherConfig::new()
            .hash_size(8, 8)
            .to_hasher();
        
        Self {
            hasher,
            window_hashes: HashMap::new(),
            threshold,
        }
    }
    
    /// Check if a specific window's content has changed
    fn has_window_changed(&mut self, window_id: u64, screenshot: &image::DynamicImage) -> bool {
        let current_hash = self.hasher.hash_image(screenshot);
        
        let changed = match self.window_hashes.get(&window_id) {
            Some(prev_hash) => {
                let distance = prev_hash.dist(&current_hash);
                distance >= self.threshold
            }
            None => true,  // First time seeing this window
        };
        
        if changed {
            self.window_hashes.insert(window_id, current_hash);
        }
        
        changed
    }
    
    /// Clean up hashes for windows that no longer exist
    fn cleanup_stale_windows(&mut self, active_window_ids: &[u64]) {
        self.window_hashes.retain(|id, _| active_window_ids.contains(id));
    }
    
    /// Process all windows across all displays
    fn check_all_windows(&mut self, windows: &[(u64, image::DynamicImage)]) -> Vec<u64> {
        let mut changed_windows = Vec::new();
        
        for (window_id, screenshot) in windows {
            if self.has_window_changed(*window_id, screenshot) {
                changed_windows.push(*window_id);
            }
        }
        
        changed_windows
    }
}
```

### Screen Capture with Multiple Displays

**The Problem:** Need to capture specific windows regardless of which display they're on.

**Solution:** Use window-based capture (by window ID), not display-based capture.

```rust
use screenshots::Screen;
use std::collections::HashMap;

struct MultiDisplayCapture {
    displays: Vec<Screen>,
}

impl MultiDisplayCapture {
    fn new() -> Self {
        let displays = Screen::all().unwrap_or_default();
        Self { displays }
    }
    
    /// Capture a specific window by ID (works across any display)
    fn capture_window(&self, window_id: u32, bounds: (i32, i32, u32, u32)) -> Option<image::RgbaImage> {
        // CGWindowListCreateImage captures by window ID, not display
        // This works regardless of which display the window is on
        
        #[cfg(target_os = "macos")]
        {
            use core_graphics::display::CGWindowListCreateImage;
            use core_graphics::geometry::{CGRect, CGPoint, CGSize};
            use core_graphics::window::{
                kCGWindowListOptionIncludingWindow,
                kCGWindowImageBoundsIgnoreFraming,
            };
            
            let (x, y, w, h) = bounds;
            let rect = CGRect::new(
                &CGPoint::new(x as f64, y as f64),
                &CGSize::new(w as f64, h as f64),
            );
            
            let image = unsafe {
                CGWindowListCreateImage(
                    rect,
                    kCGWindowListOptionIncludingWindow,
                    window_id,
                    kCGWindowImageBoundsIgnoreFraming,
                )
            };
            
            // Convert CGImage to image::RgbaImage
            image.map(|cg_image| convert_cgimage_to_rgba(&cg_image))
        }
        
        #[cfg(not(target_os = "macos"))]
        None
    }
    
    /// Capture all visible windows across all displays
    fn capture_all_windows(&self, windows: &[WindowInfo]) -> HashMap<u64, image::RgbaImage> {
        let mut captures = HashMap::new();
        
        for window in windows {
            let bounds = (
                window.position.x,
                window.position.y,
                window.position.width as u32,
                window.position.height as u32,
            );
            
            if let Some(image) = self.capture_window(window.id as u32, bounds) {
                captures.insert(window.id, image);
            }
        }
        
        captures
    }
    
    /// Capture by display (for full-display OCR fallback)
    fn capture_display(&self, display_id: u32) -> Option<image::RgbaImage> {
        self.displays
            .iter()
            .find(|d| d.display_info.id == display_id)
            .and_then(|screen| screen.capture().ok())
    }
    
    /// Capture all displays in parallel
    fn capture_all_displays(&self) -> HashMap<u32, image::RgbaImage> {
        use rayon::prelude::*;
        
        self.displays
            .par_iter()
            .filter_map(|screen| {
                screen.capture().ok().map(|img| (screen.display_info.id, img))
            })
            .collect()
    }
}

// Helper to convert CGImage to image crate format
#[cfg(target_os = "macos")]
fn convert_cgimage_to_rgba(cg_image: &core_graphics::image::CGImage) -> image::RgbaImage {
    let width = cg_image.width();
    let height = cg_image.height();
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();
    
    // Create RgbaImage from raw bytes
    // Note: CGImage may be BGRA, need to swizzle channels
    image::RgbaImage::from_raw(width as u32, height as u32, data.to_vec())
        .unwrap_or_else(|| image::RgbaImage::new(width as u32, height as u32))
}
```

### Integrated Multi-Display Extraction Loop

```rust
use tokio::time::{interval, Duration};

struct UnifiedRouter {
    window_tracker: MultiDisplayTracker,
    hash_tracker: PerWindowHashTracker,
    capture: MultiDisplayCapture,
    extractor_router: ExtractorRouter,
}

impl UnifiedRouter {
    async fn run(&mut self) {
        let mut tick = interval(Duration::from_secs(5));
        
        loop {
            tick.tick().await;
            
            // 1. Get all visible windows across all displays
            let windows = get_open_windows().unwrap_or_default();
            
            // 2. Update window positions and detect display changes
            let changes = self.window_tracker.update_window_positions();
            
            // 3. Capture all windows
            let captures = self.capture.capture_all_windows(&windows);
            
            // 4. Check which windows have changed (per-window hash)
            let screenshots: Vec<_> = captures
                .iter()
                .map(|(id, img)| (*id, image::DynamicImage::ImageRgba8(img.clone())))
                .collect();
            
            let changed_windows = self.hash_tracker.check_all_windows(&screenshots);
            
            // 5. Extract content from changed windows
            for window_id in changed_windows {
                if let Some(window) = windows.iter().find(|w| w.id == window_id) {
                    let extractor = self.extractor_router.get_extractor(&window.info.name);
                    
                    match extractor {
                        ExtractorType::Accessibility => {
                            // Call accessibility extractor
                        }
                        ExtractorType::Chrome => {
                            // Chrome pushes to us, skip
                        }
                        ExtractorType::OCR => {
                            // Run OCR on the captured image
                            if let Some(screenshot) = captures.get(&window_id) {
                                // ocr_extract(screenshot)
                            }
                        }
                    }
                }
            }
            
            // 6. Cleanup stale window hashes
            let active_ids: Vec<_> = windows.iter().map(|w| w.id).collect();
            self.hash_tracker.cleanup_stale_windows(&active_ids);
        }
    }
}
```

### Key Multi-Display Design Principles

| Aspect | Single Display | Multi-Display |
|--------|----------------|---------------|
| Focus tracking | Global frontmost app | Per-display topmost window |
| Hash storage | Single hash | HashMap<window_id, hash> |
| Capture target | Display screenshot | Window-specific capture |
| Change detection | One comparison | N comparisons (one per window) |
| Event handling | App activation only | App activation + window position polling |

