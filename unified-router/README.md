# Unified Router & Ingestion Service

A single-command solution to run all content extraction services for the Ingestion system.

## Quick Start

```bash
# Build all components
./scripts/build-all.sh --release

# Start the unified service
./scripts/ingestion start

# Or run directly
./unified-router/target/release/ingestion
```

## Architecture

The unified ingestion service orchestrates:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        UNIFIED INGESTION SERVICE                         │
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Window    │  │  Extractor  │  │   Change    │  │   Privacy   │    │
│  │   Tracker   │  │   Router    │  │  Detector   │  │   Filter    │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│                                                                          │
└───────────────────────────────────┬──────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
    ┌─────────────┐          ┌─────────────┐          ┌─────────────┐
    │Accessibility│          │   Chrome    │          │     OCR     │
    │  Extractor  │          │  Extension  │          │  Extractor  │
    └─────────────┘          └─────────────┘          └─────────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    ▼
                           ┌─────────────────┐
                           │    Ingestion    │
                           │    Service      │
                           │  (SQLite + Dedup)│
                           └─────────────────┘
```

## Usage

### Direct Binary

```bash
# Start with defaults
./unified-router/target/release/ingestion

# Disable OCR (lower battery usage)
./unified-router/target/release/ingestion --no-ocr

# Custom capture interval
./unified-router/target/release/ingestion --interval 10

# Disable accessibility extraction
./unified-router/target/release/ingestion --no-accessibility
```

### Launcher Script

```bash
# Start service
./scripts/ingestion start

# Stop service
./scripts/ingestion stop

# Check status
./scripts/ingestion status

# Install as LaunchAgent (auto-start on login)
./scripts/ingestion install

# View logs
./scripts/ingestion logs

# Build all components
./scripts/ingestion build
```

## Configuration

Configuration file: `~/.config/unified-router/config.toml`

```toml
[general]
enabled = true
log_level = "info"

[timing]
base_interval_seconds = 5
battery_interval_seconds = 15
min_interval_seconds = 3

[change_detection]
hash_sensitivity = 8

[extractors]
accessibility_enabled = true
chrome_extension_enabled = true
ocr_enabled = true

[privacy]
blocked_apps = ["com.1password.*", "com.lastpass.*"]
redact_credit_cards = true
redact_ssn = true
redact_api_keys = true
```

## Permissions Required

### Accessibility
Required for extracting content from Office, iWork, Slack, Teams.

Grant in: **System Settings > Privacy & Security > Accessibility**

### Screen Recording
Required for OCR extraction and perceptual hash computation.

Grant in: **System Settings > Privacy & Security > Screen Recording**

## Supported Applications

### Accessibility Extraction
- Microsoft Word, Excel, PowerPoint, Outlook, Teams
- Apple Pages, Numbers, Keynote, TextEdit, Notes
- Slack, Discord

### Chrome Extension
- Google Chrome, Brave, Microsoft Edge, Vivaldi, Opera
- Extracts web page content via Readability

### OCR Fallback
- Any application not supported by accessibility
- PDF viewers, image editors, etc.

## Components

| Component | Description | Binary |
|-----------|-------------|--------|
| Unified Router | Window tracking, change detection, routing | `unified-router` |
| Ingestion Service | SQLite storage, dedup, chunking | `ingestion-server` |
| Accessibility Extractor | Office/iWork content extraction | `ax-extractor` |
| OCR Extractor | Screen capture + Vision OCR | `OCRExtractor` |
| Native Host | Chrome extension relay | `ingestion-host` |

## Binary Sizes

| Binary | Size |
|--------|------|
| `ingestion` | ~8.1 MB |
| `unified-router` | ~8.0 MB |
| `ingestion-server` | ~3.5 MB |
| `ax-extractor` | ~5.0 MB |
| `ax-daemon` | ~6.8 MB |
| `ingestion-host` | ~0.5 MB |
| `OCRExtractor` | ~0.5 MB |

## Development

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Check for issues
cargo clippy
```

## Troubleshooting

### Service won't start
1. Check permissions: `./scripts/ingestion status`
2. Grant Accessibility permission in System Settings
3. Grant Screen Recording permission in System Settings

### No content extracted
1. Ensure the target app is in the supported list
2. Check if the app is blocked by privacy filter
3. Try running with `RUST_LOG=debug` for verbose output

### High CPU usage
1. Increase capture interval: `--interval 15`
2. Disable OCR: `--no-ocr`
3. Check if many windows are open
