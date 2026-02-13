# Unified Router Implementation Context

This file tracks implemented components, their APIs, and dependencies for the unified router.

## Project Structure

```
unified-router/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs
│   ├── config.rs
│   ├── window_tracker.rs
│   ├── change_detector.rs
│   ├── capture.rs
│   ├── router.rs
│   ├── privacy.rs
│   ├── extractors/
│   │   ├── mod.rs
│   │   ├── accessibility.rs
│   │   ├── chrome.rs
│   │   └── ocr.rs
│   └── ipc/
│       ├── mod.rs
│       └── control_socket.rs
└── tests/
```

## Implemented Components

### 1. Types (`src/types.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub type WindowId = u64;
pub type DisplayId = u32;

pub enum ExtractorType { Accessibility, Chrome, Ocr }
pub struct WindowInfo { id, display_id, title, bundle_id, app_name, bounds, pid }
pub struct WindowBounds { x, y, width, height }
pub struct DisplayInfo { id, bounds, is_main, is_builtin }
pub struct WindowState { info, extractor_type, last_hash, last_content_hash, ... }
pub struct ExtractedContent { source, title, content, app_name, url, timestamp, ... }
pub struct CapturePayload { source, url, content, title, author, channel, timestamp }
pub enum ExtractionTrigger { AppActivated, TitleChanged, ContentChanged, TimerTick, ... }
pub enum ExtractionError { WindowNotFound, AppNotFound, ExtractionFailed, ... }
```

**Key Methods:**
- `WindowBounds::center() -> (i32, i32)` - Get center point
- `WindowBounds::contains(x, y) -> bool` - Point containment check
- `ExtractorType::as_str() -> &str` - String representation
- `From<ExtractedContent> for CapturePayload` - Conversion to ingestion format

---

### 2. Config (`src/config.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub struct Config {
    pub general: GeneralConfig,      // enabled, log_level
    pub timing: TimingConfig,        // intervals, debounce settings
    pub change_detection: ChangeDetectionConfig,  // hash_sensitivity, algorithm
    pub extractors: ExtractorsConfig,  // enable/disable each extractor
    pub privacy: PrivacyConfig,      // blocked_apps, redaction settings
    pub multi_display: MultiDisplayConfig,  // multi-monitor settings
}
```

**Key Methods:**
- `Config::load() -> Config` - Load from default path
- `Config::load_from_path(path) -> Config` - Load from specific path
- `Config::default_config_path() -> PathBuf` - Get default config location
- `Config::save() -> Result<()>` - Save to default path

**Default Values:**
- `base_interval_seconds: 5`
- `battery_interval_seconds: 15`
- `hash_sensitivity: 8`
- `blocked_apps: [1password, lastpass, bitwarden, banking apps, ...]`

---

### 3. Window Tracker (`src/window_tracker.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub struct WindowTracker {
    displays: Vec<DisplayInfo>,
    windows: HashMap<WindowId, WindowInfo>,
    active_per_display: HashMap<DisplayId, WindowId>,
    previous_titles: HashMap<WindowId, String>,
}

pub struct WindowChanges {
    pub created: Vec<WindowInfo>,
    pub destroyed: Vec<WindowId>,
    pub title_changed: Vec<(WindowId, String)>,
    pub focus_changed: Vec<(DisplayId, WindowId)>,
}
```

**Key Methods:**
- `WindowTracker::new() -> Self` - Create new tracker
- `refresh_displays() -> &[DisplayInfo]` - Refresh display list
- `refresh_windows() -> WindowChanges` - Refresh windows, detect changes
- `windows() -> impl Iterator<Item = &WindowInfo>` - Get all windows
- `get_window(id) -> Option<&WindowInfo>` - Get specific window
- `active_window_for_display(display_id) -> Option<&WindowInfo>` - Get active window per display
- `display_at_point(x, y) -> Option<&DisplayInfo>` - Find display at coordinates
- `display_for_window(window_id) -> Option<&DisplayInfo>` - Get display for window

**Platform Support:**
- macOS: Uses `CGWindowListCopyWindowInfo` and `CGGetActiveDisplayList`
- Other platforms: Returns empty (stub implementation)

---

### 4. Change Detector (`src/change_detector.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub type PerceptualHash = u64;

pub struct ChangeDetector {
    window_hashes: HashMap<WindowId, PerceptualHash>,
    threshold: u32,
}
```

**Key Methods:**
- `ChangeDetector::new(threshold: u32) -> Self` - Create with custom threshold
- `ChangeDetector::default() -> Self` - Create with default threshold (8)
- `has_changed(window_id, image) -> bool` - Check if window content changed
- `check_batch(windows) -> Vec<WindowId>` - Check multiple windows, return changed IDs
- `get_hash(window_id) -> Option<PerceptualHash>` - Get stored hash
- `remove(window_id)` - Remove hash for destroyed window
- `cleanup(active_window_ids)` - Remove hashes for non-existent windows
- `tracked_count() -> usize` - Number of tracked windows
- `set_threshold(threshold)` - Update sensitivity threshold

**Standalone Functions:**
- `compute_ahash(image) -> PerceptualHash` - Average hash (8x8, 64-bit)
- `compute_dhash(image) -> PerceptualHash` - Difference hash (9x8, 64-bit)
- `hamming_distance(a, b) -> u32` - Bit difference count (0-64)
- `hash_to_hex(hash) -> String` - Debug formatting

**Algorithm:**
1. Resize image to 8x8
2. Convert to grayscale
3. Calculate average brightness
4. Generate 64-bit hash: bit=1 if pixel > average
5. Compare using Hamming distance (threshold default: 8)

---

### 5. Capture (`src/capture.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub struct CaptureService {
    last_capture_times: HashMap<WindowId, Instant>,
}
```

**Key Methods:**
- `CaptureService::new() -> Self` - Create new capture service
- `capture_window(window_id, bounds) -> Option<DynamicImage>` - Capture specific window
- `capture_display(display_id) -> Option<DynamicImage>` - Capture entire display
- `capture_windows(windows) -> HashMap<WindowId, DynamicImage>` - Batch capture
- `time_since_capture(window_id) -> Option<Duration>` - Time since last capture
- `cleanup(active_window_ids)` - Remove tracking for destroyed windows

**Platform Support:**
- macOS: Uses `CGWindowListCreateImage` for window-specific capture
- Other platforms: Returns None (stub)

---

### 6. Privacy Filter (`src/privacy.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub struct PrivacyFilter {
    config: PrivacyConfig,
    blocked_patterns: Vec<glob::Pattern>,
}
```

**Key Methods:**
- `PrivacyFilter::new(config) -> Self` - Create with config
- `PrivacyFilter::default() -> Self` - Create with default blocklist
- `is_blocked(bundle_id) -> bool` - Check if app is blocked
- `redact(content) -> String` - Redact PII from content
- `block_app(bundle_id)` - Add app to blocklist at runtime
- `unblock_app(bundle_id)` - Remove app from blocklist
- `blocked_apps() -> &[String]` - Get current blocklist

**PII Patterns Detected:**
- Credit cards (with Luhn validation)
- Social Security Numbers
- API keys and tokens
- AWS access keys
- Email addresses
- Phone numbers
- Password fields

---

### 7. Extractors (`src/extractors/`)

**Status:** ✅ Implemented

**Module Structure:**
```rust
pub mod accessibility;  // AccessibilityExtractor
pub mod chrome;         // ChromeExtensionClient
pub mod ocr;            // OcrExtractor

pub struct ExtractorRegistry {
    accessibility_apps: HashSet<String>,
    chrome_browsers: HashSet<String>,
}
```

**ExtractorRegistry Methods:**
- `new() -> Self` - Create with default app mappings
- `get_extractor_type(bundle_id) -> ExtractorType` - Determine best extractor
- `supports_accessibility(bundle_id) -> bool` - Check accessibility support
- `is_chrome_browser(bundle_id) -> bool` - Check if browser
- `add_accessibility_app(bundle_id)` - Add custom app
- `add_chrome_browser(bundle_id)` - Add custom browser

**AccessibilityExtractor:**
- `new() -> Self` - Create with default binary path
- `with_path(path) -> Self` - Create with custom path
- `is_available() -> bool` - Check if binary exists
- `extract(window) -> Result<ExtractedContent>` - Extract via accessibility

**ChromeExtensionClient:**
- `new() -> Self` - Create client
- `take_receiver() -> Option<Receiver<ExtractedContent>>` - Get content channel
- `read_message() -> Result<ChromeMessage>` - Read from stdin (native messaging)
- `write_response(response)` - Write to stdout
- `process_message(message) -> Option<ExtractedContent>` - Convert message

**OcrExtractor:**
- `new() -> Self` - Create with default binary path
- `with_path(path) -> Self` - Create with custom path
- `is_available() -> bool` - Check if binary exists
- `extract(window, image) -> Result<ExtractedContent>` - Extract via OCR

---

### 8. Router (`src/router.rs`)

**Status:** ✅ Implemented

**Exports:**
```rust
pub struct UnifiedRouter {
    config: Config,
    window_tracker: WindowTracker,
    change_detector: ChangeDetector,
    capture_service: CaptureService,
    privacy_filter: PrivacyFilter,
    extractor_registry: ExtractorRegistry,
    accessibility_extractor: AccessibilityExtractor,
    ocr_extractor: OcrExtractor,
    chrome_client: ChromeExtensionClient,
    window_states: HashMap<WindowId, WindowState>,
    content_tx: mpsc::Sender<CapturePayload>,
    paused: bool,
    last_tick: Instant,
}

pub struct RouterStatus {
    pub paused: bool,
    pub displays: usize,
    pub windows: usize,
    pub extractions_total: u32,
}
```

**Key Methods:**
- `UnifiedRouter::new(config, content_tx) -> Self` - Create router
- `init()` - Initialize displays and windows
- `tick() -> Result<()>` - Run one extraction cycle
- `handle_chrome_content(content)` - Process Chrome extension content
- `pause()` / `resume()` - Control extraction
- `is_paused() -> bool` - Check pause state
- `status() -> RouterStatus` - Get current status
- `block_app(bundle_id)` / `unblock_app(bundle_id)` - Runtime blocklist
- `take_chrome_receiver() -> Option<Receiver<ExtractedContent>>` - Get Chrome channel

**Extraction Flow:**
1. Refresh window list, detect changes
2. Handle new/destroyed/changed windows
3. For each non-blocked, non-Chrome window:
   - Check time since last extraction
   - Capture window screenshot
   - Check perceptual hash for changes
   - If changed, trigger extraction
4. Apply privacy redaction
5. Send to content channel

---

## Dependencies Between Components

```
types.rs ─────────────────────────────────────────────┐
    │                                                 │
    ▼                                                 │
config.rs                                             │
    │                                                 │
    ▼                                                 │
window_tracker.rs ◄── capture.rs ◄── change_detector.rs
    │                     │                │
    └─────────────────────┼────────────────┘
                          ▼
                     router.rs ◄── privacy.rs
                          │
                          ▼
                   extractors/mod.rs
                    ▲    ▲    ▲
                    │    │    │
        accessibility  chrome  ocr
```

## Build Order

1. `types.rs` - Core types used everywhere
2. `config.rs` - Configuration (depends on types)
3. `window_tracker.rs` - Window enumeration (depends on types)
4. `capture.rs` - Screenshot capture (depends on types, window_tracker)
5. `change_detector.rs` - Perceptual hash (depends on types, capture)
6. `privacy.rs` - PII redaction (standalone)
7. `extractors/` - Extractor integrations (depends on types)
8. `router.rs` - Main orchestration (depends on all above)
9. `main.rs` - Entry point

## Current Task

✅ Implementation complete! All core modules implemented and compiling.

**Implemented:**
1. `types.rs` - Core types
2. `config.rs` - Configuration management
3. `window_tracker.rs` - Window enumeration
4. `change_detector.rs` - Perceptual hash
5. `capture.rs` - Window screenshot capture
6. `privacy.rs` - PII redaction
7. `extractors/` - Extractor integrations
8. `router.rs` - Main orchestration
9. `main.rs` - Entry point
10. `bin/ingestion.rs` - Unified service launcher

**Binaries:**
- `unified-router` - Core router daemon
- `ingestion` - Unified service (single command to run everything)

---

## Unified Ingestion Service

The `ingestion` binary provides a single command to run all extraction services.

**Usage:**
```bash
# Start all services with defaults (accessibility + Chrome, no OCR)
./unified-router/target/release/ingestion

# Start with OCR enabled (runs as background service)
./unified-router/target/release/ingestion --ocr

# Start with options
./unified-router/target/release/ingestion --no-accessibility # Only Chrome extension
./unified-router/target/release/ingestion --interval 10      # 10 second interval

# Using the launcher script
./scripts/ingestion start                    # Start service
./scripts/ingestion stop                     # Stop service
./scripts/ingestion status                   # Check status
./scripts/ingestion install                  # Install as LaunchAgent
./scripts/ingestion logs                     # View logs
```

**Components Orchestrated:**
- Ingestion Server (SQLite storage, dedup, chunking)
- Unified Router (window tracking, change detection)
- Accessibility Extractor (Office, iWork, Slack, Teams, Finder)
- OCR Extractor (optional, runs as background service)
- Chrome Extension (web page content via native messaging)

**Extraction Methods:**
- Accessibility: Used for apps with good accessibility support (Word, Excel, Slack, etc.)
- Chrome Extension: Web browsers push content via native messaging (skipped by router)
- OCR: Fallback for apps without accessibility support (disabled by default, enable with --ocr)

**Binary Sizes:**
- `ingestion`: ~8.1MB
- `unified-router`: ~8.0MB

**Key Fixes Applied:**
1. Fixed bundle ID detection using `osascript` instead of broken `ps` fallback
2. OCR enabled by default - uses ScreenCaptureKit for window capture (macOS 14.0+)
3. Added `--window-id` mode to OCR extractor for per-window capture and OCR
4. Chrome windows properly skipped (they push content via extension)
5. Ingestion server now spawned as background process (enables Chrome extension → native host → socket communication)
6. Fixed `CGWindowListCreateImage` deprecation on macOS 15+ by using ScreenCaptureKit in OCR extractor
7. Added `is_on_screen` field to WindowInfo for visibility tracking

---

## OCR Extractor Updates (macOS 15+ Compatibility)

The `CGWindowListCreateImage` API was deprecated in macOS 15.0 and replaced with ScreenCaptureKit.

**Changes Made:**
1. Added `--window-id <ID>` mode to OCR extractor that:
   - Uses ScreenCaptureKit to capture specific windows (macOS 14.0+)
   - Falls back to CGWindowListCreateImage on older macOS
   - Performs OCR on the captured image
   - Returns JSON output with text, confidence, and metadata

2. Updated unified router's OCR extractor integration:
   - Uses `extract_window()` method instead of `extract()` with pre-captured image
   - Passes window ID to OCR extractor which handles capture internally
   - No longer relies on deprecated Rust capture service for OCR windows

**OCR Extractor Usage:**
```bash
# Capture and OCR a specific window
./ocr-extractor/.build/release/ocr-extractor --window-id 12345 --json

# Output:
{"text":"...", "confidence":0.95, "windowId":12345, "windowTitle":"..."}
```

---

## Debounce and Dedup Implementation

**Active Window Detection:**
- There is only ONE active window system-wide (the one with keyboard focus)
- Uses `CGWindowListCopyWindowInfo` to get the frontmost window (first in z-order with layer 0)
- Focus changes are detected by comparing the frontmost window ID between ticks

**Debounce Logic:**
1. `min_interval_seconds` (default: 3s) - Minimum time between extractions for the same window
2. Only the single active/focused window is processed during tick loop
3. `is_on_screen` check - Skip windows that are minimized or on different Space
4. Perceptual hash change detection - Only extract if visual content changed

**Content Dedup:**
1. Router-level: SHA256 hash of content, skips if unchanged (`handle_extracted_content()`)
2. Ingestion server-level: Database dedup with content hash comparison

**Current Behavior:**
- Initial extraction: Only the active window extracted on startup
- Tick loop (every 5s):
  - Only processes the single active/focused window
  - Perceptual hash check before extraction (capture → hash → compare)
  - OCR/Accessibility only triggered if visual content changed
- Event-driven: Focus changes trigger immediate extraction of newly focused window

**Architecture:**
```
unified-router (orchestrator)
    │
    ├── tick() every 5s
    │       │
    │       └── process_windows() - single active window only
    │               │
    │               ├── capture_window_image() - ScreenCaptureKit
    │               │
    │               ├── perceptual hash check
    │               │
    │               └── if changed → trigger_extraction()
    │                       │
    │                       ├── OCR: spawns ocr-extractor --window-id
    │                       │
    │                       └── Accessibility: spawns ax-extractor --app
    │
    └── handle_window_changes()
            │
            └── focus_changed → trigger_extraction() for new active window
```

---

## API Reference

(Will be populated as components are implemented)
