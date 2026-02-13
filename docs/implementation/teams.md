# Microsoft Teams Content Extraction

This document describes the implementation details for extracting content from Microsoft Teams on macOS using the Accessibility API.

## Overview

Microsoft Teams exists in two versions on macOS, each with different underlying technologies:

| Version | Bundle ID | Technology | Extraction Method |
|---------|-----------|------------|-------------------|
| Classic Teams | `com.microsoft.teams` | Electron (Chromium) | AXManualAccessibility + tree traversal |
| New Teams | `com.microsoft.teams2` | Native + WebView (Chromium) | Deep tree traversal only |

Both versions are supported by the accessibility extractor and produce the same output format.

## Architecture Differences

### Classic Teams (Electron-based)

Classic Teams is built on Electron, which uses Chromium to render web content inside a native window. By default, Electron apps don't expose their DOM through the macOS Accessibility API.

**Enabling Accessibility:**
```rust
// Set AXManualAccessibility to true on the application element
let attr_name = CFString::new("AXManualAccessibility");
AXUIElementSetAttributeValue(app_ref, attr_name, CFBoolean::true_value());
```

This enables Chrome's accessibility tree, making the DOM content visible to accessibility APIs.

### New Teams (WebView-based)

New Teams uses a native macOS application with embedded Chromium-based web views. Key differences:

- `AXManualAccessibility` setting fails with error -25205 ("Cannot complete action")
- `AXEnhancedUserInterface` setting fails with error -25208
- `AXNumberOfCharacters` returns 0 on the web content group
- `AXValue` is empty on top-level web content elements

**However**, text content IS accessible by traversing deep into the accessibility tree and collecting text from specific element types.

## Extraction Strategy

### New Teams Deep Tree Traversal

The extraction process for New Teams:

1. **Find the web content group** - Look for an `AXGroup` element with "Web content" in its title
2. **Collect all text elements** - Traverse the tree and collect text from:
   - `AXHeading` elements (contain message metadata like "Message by Author")
   - `AXStaticText` elements (contain actual message content)
   - `AXLink` elements (contain URLs and link descriptions)
3. **Parse into messages** - Process the collected text to extract author, timestamp, and content

### Accessibility Tree Structure

New Teams exposes content in this structure:

```
AXApplication "Microsoft Teams"
└── AXWindow "Chat | Channel Name | Microsoft Teams"
    └── AXGroup "Web content"
        └── AXGroup (chat container)
            ├── AXHeading "Link message-preview... by AuthorName"
            │   ├── AXStaticText "AuthorName"
            │   ├── AXStaticText "Wednesday 3:39 p.m."
            │   ├── AXLink "Link actual-content"
            │   └── AXStaticText "actual message content"
            ├── AXHeading "Meeting started at Wednesday 3:29 p.m."
            └── ...
```

### Message Parsing Logic

Messages are identified by `AXHeading` elements containing " by " pattern:

```rust
// Example heading: "Link registrar@mun.ca - Link Confirm Enro... by Abu,Arif"
if let Some(by_pos) = text.rfind(" by ") {
    let author = text[by_pos + 4..].trim();
    // Author is "Abu,Arif"
}
```

After the heading, the following elements contain:
1. Author name (AXStaticText)
2. Timestamp (AXStaticText) - e.g., "Wednesday 3:39 p.m."
3. Message content (AXStaticText/AXLink)

### System Events

Teams system events are also captured:

- **Meeting started**: `"Meeting started at Wednesday 3:29 p.m."`
- **Meeting ended**: `"Meeting ended: at Wednesday 3:48 p.m. after 21 minutes 7 seconds"`
- **User joined/left**: Filtered out (not included in output)

## Output Format

Extracted content follows the same format as Slack:

```
[Author] [Timestamp] Message content
```

Example output:
```
[System] [Wednesday 3:29 p.m.] Meeting started
[Abu,Arif] [Wednesday 3:39 p.m.] registrar@mun.ca Confirm Enrolment to Renew Study Permit
[Abu,Arif] [Wednesday 3:39 p.m.] ESU-124 - Post-Graduate Work Permit (MyCreds)
[Abu,Arif] [Wednesday 3:41 p.m.] English Test
[System] [Wednesday 3:48 p.m.] Meeting ended (duration: 21 minutes 7 seconds)
```

## Key Functions

### `electron.rs`

| Function | Purpose |
|----------|---------|
| `is_teams(bundle_id)` | Check if bundle ID is Teams (classic or new) |
| `detect_teams_version()` | Detect which Teams version is running |
| `prepare_teams(bundle_id, config)` | Prepare Teams for extraction (Classic only) |
| `extract_teams_content(app)` | Main extraction entry point |
| `extract_new_teams_content(window)` | New Teams specific extraction |
| `collect_all_text_elements(element, results, depth)` | Collect text from tree |
| `parse_teams_text_into_messages(elements)` | Parse text into messages |
| `looks_like_teams_timestamp(text)` | Detect timestamp patterns |
| `is_teams_ui_text(text)` | Filter UI chrome text |

### `mod.rs`

The `extract_from_app()` function handles Teams specially:

```rust
if electron::is_teams(bundle_id) {
    if bundle_id == electron::TEAMS_NEW_BUNDLE_ID {
        // New Teams - use deep tree traversal
        let content = electron::extract_teams_content(&app);
    } else {
        // Classic Teams - enable accessibility first
        electron::prepare_teams(bundle_id, None)?;
        let content = electron::extract_teams_content(&app);
    }
}
```

## Timestamp Detection

Teams uses various timestamp formats that need to be detected:

```rust
fn looks_like_teams_timestamp(text: &str) -> bool {
    text.contains("AM") || text.contains("PM") ||
    text.contains("a.m.") || text.contains("p.m.") ||  // Teams uses lowercase
    text.contains("ago") ||
    text.starts_with("Monday") || text.starts_with("Tuesday") || // etc.
    text.contains("Yesterday") || text.contains("Today") ||
    text.contains("Jan") || text.contains("Feb") || // etc.
}
```

## UI Text Filtering

Common Teams UI elements are filtered out:

```rust
const UI_LABELS: &[&str] = &[
    "Chat", "Teams", "Calendar", "Calls", "Files", "Activity",
    "More", "Search", "Settings", "Help", "New chat", "New meeting",
    "Meet", "Join", "Leave", "Mute", "Unmute", "Share", "React",
    "Reply", "Forward", "Copy", "Delete", "Edit", "Pin", "Save",
    "Mentions", "Favourites", "Chats", "Shared", "Recap", "Q&A",
    "OneDrive", "Apps", "Type a message",
];
```

## Deduplication

Content deduplication handles:

1. **Exact duplicates** - Same text appearing multiple times
2. **Substring duplicates** - URL previews that repeat the URL text
3. **Heading duplicates** - Heading text repeated in child AXStaticText

```rust
let mut seen: HashSet<String> = HashSet::new();
let unique_content: Vec<String> = content_parts
    .into_iter()
    .filter(|p| {
        if seen.contains(p) { return false; }
        for existing in &seen {
            if existing.contains(p.as_str()) || p.contains(existing.as_str()) {
                return false;
            }
        }
        seen.insert(p.clone());
        true
    })
    .collect();
```

## Integration with ax-daemon

Both Teams versions are registered in `SUPPORTED_APPS`:

```rust
const SUPPORTED_APPS: &[(&str, &str)] = &[
    // ... other apps ...
    ("com.microsoft.teams", "teams"),   // Classic Teams
    ("com.microsoft.teams2", "teams"),  // New Teams
];
```

The daemon automatically extracts content when users switch away from Teams.

## Limitations

1. **No Graph API** - This implementation uses only the Accessibility API, not Microsoft Graph
2. **Visible content only** - Only messages visible in the current view are extracted
3. **No TCC prompts** - The implementation avoids triggering Automation permission prompts
4. **Read-only** - Cannot send messages or interact with Teams

## Debugging

Debug tools are available in `src/bin/`:

- `teams-enable-ax.rs` - Shows the full accessibility tree with all text elements
- `teams-chromium-debug.rs` - Explores Chromium-specific attributes
- `teams-extract-test.rs` - Tests extraction with detailed output

Run with:
```bash
cargo run --bin teams-enable-ax
```

## Error Handling

Common errors and their meanings:

| Error Code | Meaning | Resolution |
|------------|---------|------------|
| -25205 | Cannot complete action | Expected for New Teams - use tree traversal |
| -25208 | Unknown error | AXEnhancedUserInterface not supported |
| -25200 | Permission denied | Grant Accessibility permission |

## Future Improvements

Potential enhancements:

1. **Channel detection** - Extract channel/chat name separately
2. **Reaction support** - Parse reaction counts from messages
3. **File attachments** - Better handling of shared files
4. **Thread support** - Detect and format threaded replies
5. **Date grouping** - Use `is_date_header()` to group messages by date
