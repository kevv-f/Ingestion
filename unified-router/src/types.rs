//! Core types used throughout the unified router.
//!
//! This module defines the fundamental data structures for window tracking,
//! extraction results, and extractor selection.

use serde::{Deserialize, Serialize};

/// Unique identifier for a window (platform-specific)
pub type WindowId = u64;

/// Unique identifier for a display
pub type DisplayId = u32;

/// Represents the type of extractor to use for content extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractorType {
    /// Use accessibility APIs (Word, Excel, etc.)
    Accessibility,
    /// Use Chrome extension (browsers with extension installed)
    Chrome,
    /// Use OCR (fallback for unsupported apps)
    Ocr,
}

impl ExtractorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtractorType::Accessibility => "accessibility",
            ExtractorType::Chrome => "chrome",
            ExtractorType::Ocr => "ocr",
        }
    }
}

/// Information about a window
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Unique window identifier
    pub id: WindowId,
    /// Display this window is on
    pub display_id: DisplayId,
    /// Window title
    pub title: String,
    /// Application bundle ID (macOS) or executable name
    pub bundle_id: String,
    /// Application name
    pub app_name: String,
    /// Window bounds (x, y, width, height)
    pub bounds: WindowBounds,
    /// Process ID of the owning application
    pub pid: u32,
    /// Whether the window is on the current Space and visible
    pub is_on_screen: bool,
}

/// Window position and size
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WindowBounds {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Get the center point of the window
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + (self.width as i32 / 2),
            self.y + (self.height as i32 / 2),
        )
    }

    /// Check if a point is inside this bounds
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }
}

/// Information about a display/monitor
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Unique display identifier
    pub id: DisplayId,
    /// Display bounds
    pub bounds: WindowBounds,
    /// Whether this is the main display
    pub is_main: bool,
    /// Whether this is a built-in display (laptop screen)
    pub is_builtin: bool,
}

/// State tracked for each window
#[derive(Debug, Clone)]
pub struct WindowState {
    /// Window information
    pub info: WindowInfo,
    /// Which extractor to use
    pub extractor_type: ExtractorType,
    /// Last visual hash (for change detection)
    pub last_hash: Option<u64>,
    /// Last content hash (SHA-256 of extracted text)
    pub last_content_hash: Option<String>,
    /// Timestamp of last extraction
    pub last_extraction: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of extractions performed
    pub extraction_count: u32,
    /// Whether this window is currently blocked (privacy)
    pub is_blocked: bool,
}

impl WindowState {
    pub fn new(info: WindowInfo, extractor_type: ExtractorType) -> Self {
        Self {
            info,
            extractor_type,
            last_hash: None,
            last_content_hash: None,
            last_extraction: None,
            extraction_count: 0,
            is_blocked: false,
        }
    }
}

/// Result of content extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    /// Source identifier (e.g., "word", "chrome", "ocr")
    pub source: String,
    /// Document/page title
    pub title: Option<String>,
    /// Extracted text content
    pub content: String,
    /// Application name (display name)
    pub app_name: String,
    /// Application bundle ID (e.g., "com.microsoft.Word")
    pub bundle_id: Option<String>,
    /// URL (for web content)
    pub url: Option<String>,
    /// Extraction timestamp
    pub timestamp: i64,
    /// Extraction method used
    pub extraction_method: String,
    /// Confidence score (0.0-1.0, mainly for OCR)
    pub confidence: Option<f32>,
}

/// Payload sent to ingestion service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturePayload {
    pub source: String,
    pub url: String,
    pub content: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub channel: Option<String>,
    pub timestamp: Option<i64>,
    /// Application name for display
    pub app_name: Option<String>,
    /// Application bundle ID for icon lookup
    pub bundle_id: Option<String>,
}

impl From<ExtractedContent> for CapturePayload {
    fn from(content: ExtractedContent) -> Self {
        // Normalize the source type based on bundle_id for better categorization
        let source_type = normalize_source_type(&content.bundle_id, &content.app_name);
        
        let url = content.url.unwrap_or_else(|| {
            // Generate URL for dedup purposes
            let app_identifier = content.bundle_id
                .as_ref()
                .map(|b| b.replace('.', "_"))
                .unwrap_or_else(|| content.app_name.replace(' ', "_"));
            
            let title = content.title.as_deref().unwrap_or("untitled");
            let encoded_title = url_encode(title);
            
            // For OCR extractions, include a content hash to differentiate
            // different content with the same window title (e.g., different chats in Claude/WhatsApp)
            if content.extraction_method == "ocr" {
                let content_hash = compute_short_hash(&content.content);
                format!(
                    "{}://{}/{}/{}",
                    content.extraction_method,
                    app_identifier,
                    encoded_title,
                    content_hash
                )
            } else {
                // For accessibility/chrome extractions, use title-based URL
                // These typically have more meaningful titles
                format!(
                    "{}://{}/{}",
                    content.extraction_method,
                    app_identifier,
                    encoded_title
                )
            }
        });

        Self {
            source: source_type,
            url,
            content: content.content,
            title: content.title,
            author: None,
            channel: None,
            timestamp: Some(content.timestamp),
            app_name: Some(content.app_name),
            bundle_id: content.bundle_id,
        }
    }
}

/// Compute a short hash of content for URL differentiation
/// Uses first 12 chars of SHA-256 hex (48 bits of entropy)
fn compute_short_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    // Take first 6 bytes (12 hex chars) for a reasonably short but unique identifier
    format!("{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", 
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5])
}

/// Simple URL encoding for path segments
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            ' ' => {
                result.push_str("%20");
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

/// Normalize source type based on bundle ID for consistent categorization
fn normalize_source_type(bundle_id: &Option<String>, app_name: &str) -> String {
    if let Some(bid) = bundle_id {
        let bid_lower = bid.to_lowercase();
        
        // Microsoft Office
        if bid_lower.contains("microsoft.word") {
            return "word".to_string();
        }
        if bid_lower.contains("microsoft.excel") {
            return "excel".to_string();
        }
        if bid_lower.contains("microsoft.powerpoint") {
            return "powerpoint".to_string();
        }
        if bid_lower.contains("microsoft.outlook") {
            return "outlook".to_string();
        }
        if bid_lower.contains("microsoft.onenote") {
            return "onenote".to_string();
        }
        if bid_lower.contains("microsoft.teams") {
            return "teams".to_string();
        }
        
        // Apple Apps
        if bid_lower.contains("apple.notes") {
            return "notes".to_string();
        }
        if bid_lower.contains("apple.reminders") {
            return "reminders".to_string();
        }
        if bid_lower.contains("apple.mail") {
            return "mail".to_string();
        }
        if bid_lower.contains("apple.finder") {
            return "finder".to_string();
        }
        if bid_lower.contains("apple.preview") {
            return "preview".to_string();
        }
        if bid_lower.contains("apple.pages") {
            return "pages".to_string();
        }
        if bid_lower.contains("apple.numbers") {
            return "numbers".to_string();
        }
        if bid_lower.contains("apple.keynote") {
            return "keynote".to_string();
        }
        if bid_lower.contains("apple.textedit") {
            return "textedit".to_string();
        }
        if bid_lower.contains("apple.safari") {
            return "safari".to_string();
        }
        if bid_lower.contains("apple.calculator") {
            return "calculator".to_string();
        }
        if bid_lower.contains("apple.terminal") {
            return "terminal".to_string();
        }
        
        // Browsers
        if bid_lower.contains("google.chrome") {
            return "chrome".to_string();
        }
        if bid_lower.contains("brave") {
            return "brave".to_string();
        }
        if bid_lower.contains("microsoft.edge") {
            return "edge".to_string();
        }
        if bid_lower.contains("mozilla.firefox") {
            return "firefox".to_string();
        }
        if bid_lower.contains("opera") {
            return "opera".to_string();
        }
        if bid_lower.contains("arc") {
            return "arc".to_string();
        }
        
        // Communication
        if bid_lower.contains("slack") {
            return "slack".to_string();
        }
        if bid_lower.contains("discord") {
            return "discord".to_string();
        }
        if bid_lower.contains("zoom") {
            return "zoom".to_string();
        }
        
        // Development
        if bid_lower.contains("vscode") || bid_lower.contains("visual-studio-code") {
            return "vscode".to_string();
        }
        if bid_lower.contains("kiro") {
            return "kiro".to_string();
        }
        if bid_lower.contains("xcode") {
            return "xcode".to_string();
        }
        if bid_lower.contains("jetbrains") || bid_lower.contains("intellij") {
            return "jetbrains".to_string();
        }
        
        // Other known apps
        if bid_lower.contains("anthropic.claude") {
            return "claude".to_string();
        }
        if bid_lower.contains("notion") {
            return "notion".to_string();
        }
        if bid_lower.contains("figma") {
            return "figma".to_string();
        }
        if bid_lower.contains("spotify") {
            return "spotify".to_string();
        }
    }
    
    // Fallback to app name (lowercase, no spaces)
    app_name.to_lowercase().replace(' ', "_")
}

/// Events that can trigger extraction
#[derive(Debug, Clone)]
pub enum ExtractionTrigger {
    /// Application was activated (came to foreground)
    AppActivated { bundle_id: String },
    /// Window title changed (e.g., tab switch)
    TitleChanged { window_id: WindowId, new_title: String },
    /// Visual content changed (hash mismatch)
    ContentChanged { window_id: WindowId },
    /// Periodic timer fired
    TimerTick,
    /// Chrome extension pushed content
    ChromePush { url: String },
    /// Manual extraction request
    Manual { window_id: Option<WindowId> },
}

/// Errors that can occur during extraction
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("Window not found: {0}")]
    WindowNotFound(WindowId),

    #[error("Application not found: {0}")]
    AppNotFound(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Content blocked by privacy filter")]
    Blocked,

    #[error("No content extracted")]
    NoContent,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_bounds_center() {
        let bounds = WindowBounds::new(100, 200, 800, 600);
        assert_eq!(bounds.center(), (500, 500));
    }

    #[test]
    fn test_window_bounds_contains() {
        let bounds = WindowBounds::new(0, 0, 100, 100);
        assert!(bounds.contains(50, 50));
        assert!(bounds.contains(0, 0));
        assert!(!bounds.contains(100, 100));
        assert!(!bounds.contains(-1, 50));
    }

    #[test]
    fn test_extractor_type_as_str() {
        assert_eq!(ExtractorType::Accessibility.as_str(), "accessibility");
        assert_eq!(ExtractorType::Chrome.as_str(), "chrome");
        assert_eq!(ExtractorType::Ocr.as_str(), "ocr");
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("test/path"), "test%2Fpath");
        assert_eq!(url_encode("Claude — Chat"), "Claude%20%E2%80%94%20Chat");
    }

    #[test]
    fn test_short_hash() {
        let hash1 = compute_short_hash("hello world");
        let hash2 = compute_short_hash("hello world");
        let hash3 = compute_short_hash("different content");
        
        // Same content = same hash
        assert_eq!(hash1, hash2);
        // Different content = different hash
        assert_ne!(hash1, hash3);
        // Hash is 12 hex chars
        assert_eq!(hash1.len(), 12);
    }

    #[test]
    fn test_capture_payload_ocr_url_includes_hash() {
        let content = ExtractedContent {
            source: "ocr".to_string(),
            title: Some("Claude".to_string()),
            content: "Chat A content here".to_string(),
            app_name: "Claude".to_string(),
            bundle_id: Some("com.anthropic.claude".to_string()),
            url: None,
            timestamp: 12345,
            extraction_method: "ocr".to_string(),
            confidence: Some(0.9),
        };

        let payload: CapturePayload = content.into();
        // OCR URLs should include content hash: ocr://bundle/title/hash
        assert!(payload.url.starts_with("ocr://com_anthropic_claude/Claude/"));
        // URL format: ocr://bundle/title/hash - should have 4 path segments after scheme
        let path = payload.url.strip_prefix("ocr://").unwrap();
        assert_eq!(path.matches('/').count(), 2); // bundle/title/hash = 2 slashes
    }

    #[test]
    fn test_capture_payload_ocr_different_content_different_url() {
        let content_a = ExtractedContent {
            source: "ocr".to_string(),
            title: Some("Claude".to_string()),
            content: "Chat A: Hello Alice".to_string(),
            app_name: "Claude".to_string(),
            bundle_id: Some("com.anthropic.claude".to_string()),
            url: None,
            timestamp: 12345,
            extraction_method: "ocr".to_string(),
            confidence: Some(0.9),
        };

        let content_b = ExtractedContent {
            source: "ocr".to_string(),
            title: Some("Claude".to_string()), // Same title!
            content: "Chat B: Hello Bob".to_string(), // Different content
            app_name: "Claude".to_string(),
            bundle_id: Some("com.anthropic.claude".to_string()),
            url: None,
            timestamp: 12346,
            extraction_method: "ocr".to_string(),
            confidence: Some(0.9),
        };

        let payload_a: CapturePayload = content_a.into();
        let payload_b: CapturePayload = content_b.into();
        
        // Different content should produce different URLs even with same title
        assert_ne!(payload_a.url, payload_b.url);
    }

    #[test]
    fn test_capture_payload_accessibility_url_no_hash() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("My Document.docx".to_string()),
            content: "Document content".to_string(),
            app_name: "Microsoft Word".to_string(),
            bundle_id: Some("com.microsoft.Word".to_string()),
            url: None,
            timestamp: 12345,
            extraction_method: "accessibility".to_string(),
            confidence: None,
        };

        let payload: CapturePayload = content.into();
        // Accessibility URLs should NOT include hash (title-based dedup works)
        assert_eq!(payload.url, "accessibility://com_microsoft_Word/My%20Document.docx");
        // URL format: accessibility://bundle/title - should have 1 slash in path
        let path = payload.url.strip_prefix("accessibility://").unwrap();
        assert_eq!(path.matches('/').count(), 1); // bundle/title = 1 slash
    }
}
