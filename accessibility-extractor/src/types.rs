//! Core data types for the accessibility-extractor crate.
//!
//! This module defines the fundamental types used throughout the crate:
//! - `ExtractedContent`: The main output type for extracted content
//! - `CapturePayload`: BrowserCapturePayload-compatible output format
//! - `ExtractionError`: Error types that can occur during extraction
//! - `AppSource`: Known application source identifiers

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Extracted content from an application.
///
/// This struct represents the result of a successful content extraction
/// from a desktop application using the Accessibility API.
///
/// # Requirements
/// - Requirement 8.1: Defines ExtractedContent struct with all required fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedContent {
    /// Source application identifier (e.g., "word", "excel", "pages")
    pub source: String,

    /// Document title (usually from window title)
    pub title: Option<String>,

    /// The extracted text content
    pub content: String,

    /// Full application name
    pub app_name: String,

    /// Unix timestamp of extraction
    pub timestamp: i64,

    /// Method used for extraction (always "accessibility")
    pub extraction_method: String,
}

/// BrowserCapturePayload-compatible output format.
///
/// This struct provides compatibility with the existing ingestion pipeline
/// by conforming to the BrowserCapturePayload JSON schema.
///
/// # Requirements
/// - Requirement 8.3: Implements Serialize and Deserialize traits
/// - Requirement 11.7: Conforms to BrowserCapturePayload JSON schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapturePayload {
    /// Source identifier (word, excel, pages, etc.)
    pub source: String,

    /// URL-like identifier for the content
    pub url: String,

    /// The extracted text content
    pub content: String,

    /// Optional document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Optional channel/workspace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Unix timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Chunk metadata for database integration.
///
/// This struct represents the metadata stored in the `meta` JSON field of the
/// `chunks` table. It conforms to the existing database schema used by the
/// ingestion pipeline.
///
/// # Requirements
/// - Requirement 11.11: Populate the chunks table with extracted content and appropriate metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkMeta {
    /// Unique document identifier (UUID v4)
    pub id: String,

    /// Source application identifier (e.g., "word", "excel", "pages")
    pub source: String,

    /// Unix timestamp of extraction
    pub timestamp: i64,

    /// First 200 characters of content (preview/header)
    pub header: String,

    /// Index of this chunk within the document (0-based)
    pub chunk_idx: u32,

    /// Total number of chunks for this document
    pub total_chunks: u32,

    /// File path (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// File extension (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,

    /// File size in bytes (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,

    /// File modification timestamp (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_modified: Option<i64>,

    /// Application bundle ID (e.g., "com.microsoft.Word")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,

    /// Channel/workspace (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Author (None for accessibility-extracted content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// URL-like identifier for the content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl ChunkMeta {
    /// Creates a new ChunkMeta with the required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique document identifier (UUID v4)
    /// * `source` - Source application identifier
    /// * `timestamp` - Unix timestamp of extraction
    /// * `content` - Full content (used to generate header)
    /// * `chunk_idx` - Index of this chunk
    /// * `total_chunks` - Total number of chunks
    ///
    /// # Example
    ///
    /// ```
    /// use accessibility_extractor::types::ChunkMeta;
    ///
    /// let meta = ChunkMeta::new(
    ///     "550e8400-e29b-41d4-a716-446655440000".to_string(),
    ///     "word".to_string(),
    ///     1707500000,
    ///     "This is the document content...",
    ///     0,
    ///     1,
    /// );
    ///
    /// assert_eq!(meta.source, "word");
    /// assert_eq!(meta.chunk_idx, 0);
    /// ```
    pub fn new(
        id: String,
        source: String,
        timestamp: i64,
        content: &str,
        chunk_idx: u32,
        total_chunks: u32,
    ) -> Self {
        // Generate header from first 200 characters of content
        let header: String = content.chars().take(200).collect();

        ChunkMeta {
            id,
            source,
            timestamp,
            header,
            chunk_idx,
            total_chunks,
            path: None,
            ext: None,
            file_size: None,
            file_modified: None,
            app_id: None,
            channel: None,
            author: None,
            url: None,
            title: None,
        }
    }

    /// Sets the application bundle ID.
    pub fn with_app_id(mut self, app_id: String) -> Self {
        self.app_id = Some(app_id);
        self
    }

    /// Sets the URL.
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    /// Sets the title.
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
}

/// Generate a unique document identifier (ehl_doc_id) using UUID v4.
///
/// This function generates a unique identifier for each extracted document.
/// The identifier is a UUID v4 (random) string that can be used to uniquely
/// identify documents in the database and ingestion pipeline.
///
/// # Returns
///
/// A `String` containing a UUID v4 in the standard hyphenated format
/// (e.g., "550e8400-e29b-41d4-a716-446655440000").
///
/// # Example
///
/// ```
/// use accessibility_extractor::types::generate_doc_id;
///
/// let doc_id = generate_doc_id();
/// println!("Generated document ID: {}", doc_id);
///
/// // Each call generates a unique ID
/// let another_id = generate_doc_id();
/// assert_ne!(doc_id, another_id);
/// ```
///
/// # Requirements
/// - Requirement 11.8: Generate a unique document identifier (ehl_doc_id) for each extracted document
pub fn generate_doc_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a content hash using SHA-256.
///
/// This function generates a SHA-256 hash of the provided content. The hash
/// is used for deduplication in the ingestion pipeline to detect duplicate
/// content.
///
/// # Arguments
///
/// * `content` - The content to hash
///
/// # Returns
///
/// A `String` containing the SHA-256 hash in lowercase hexadecimal format
/// (64 characters).
///
/// # Example
///
/// ```
/// use accessibility_extractor::types::generate_content_hash;
///
/// let hash = generate_content_hash("Hello, world!");
/// assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
///
/// // Same content produces same hash
/// let hash2 = generate_content_hash("Hello, world!");
/// assert_eq!(hash, hash2);
///
/// // Different content produces different hash
/// let hash3 = generate_content_hash("Different content");
/// assert_ne!(hash, hash3);
/// ```
///
/// # Requirements
/// - Requirement 11.10: Check for duplicate content using the existing dedup mechanism
pub fn generate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Errors that can occur during extraction.
///
/// This enum represents all possible error conditions that can occur
/// during the content extraction process.
///
/// # Requirements
/// - Requirement 8.2: Defines ExtractionError enum with all variants
/// - Requirement 8.4: Implements the Error trait with descriptive messages
/// - Requirement 8.5: Uses the thiserror crate for error definitions
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// Accessibility permission not granted
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Application not found or not running
    #[error("Application not found: {0}")]
    AppNotFound(String),

    /// UI element not found (e.g., no focused window)
    #[error("Element not found: {0}")]
    ElementNotFound(String),

    /// No content found in the document
    #[error("No content found: {0}")]
    NoContentFound(String),

    /// Accessibility pattern not supported by the application
    #[error("Pattern not supported: {0}")]
    PatternNotSupported(String),

    /// Platform-specific error (macOS API error)
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Operation timed out
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Accessibility API error (e.g., failed to enable Electron accessibility)
    #[error("Accessibility error: {0}")]
    AccessibilityError(String),
}

/// Known application sources.
///
/// This enum represents the known desktop applications that the extractor
/// can identify and extract content from.
///
/// # Requirements
/// - Requirement 6.2-6.9: Recognizes Microsoft Office and Apple iWork apps
/// - Requirement 6.10: Provides from_bundle_id conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSource {
    /// Microsoft Word
    Word,
    /// Microsoft Excel
    Excel,
    /// Microsoft PowerPoint
    PowerPoint,
    /// Microsoft Outlook
    Outlook,
    /// Microsoft Teams
    Teams,
    /// Apple Pages
    Pages,
    /// Apple Numbers
    Numbers,
    /// Apple Keynote
    Keynote,
    /// Apple TextEdit
    TextEdit,
    /// LibreOffice (any variant)
    LibreOffice,
    /// Slack
    Slack,
    /// Unknown application
    Unknown,
}

impl AppSource {
    /// Returns the string identifier for this application source.
    ///
    /// # Examples
    ///
    /// ```
    /// use accessibility_extractor::AppSource;
    ///
    /// assert_eq!(AppSource::Word.as_str(), "word");
    /// assert_eq!(AppSource::Pages.as_str(), "pages");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            AppSource::Word => "word",
            AppSource::Excel => "excel",
            AppSource::PowerPoint => "powerpoint",
            AppSource::Outlook => "outlook",
            AppSource::Teams => "teams",
            AppSource::Pages => "pages",
            AppSource::Numbers => "numbers",
            AppSource::Keynote => "keynote",
            AppSource::TextEdit => "textedit",
            AppSource::LibreOffice => "libreoffice",
            AppSource::Slack => "slack",
            AppSource::Unknown => "unknown",
        }
    }

    /// Creates an AppSource from a macOS bundle identifier.
    ///
    /// # Arguments
    ///
    /// * `bundle_id` - The bundle identifier (e.g., "com.microsoft.Word")
    ///
    /// # Examples
    ///
    /// ```
    /// use accessibility_extractor::AppSource;
    ///
    /// assert_eq!(AppSource::from_bundle_id("com.microsoft.Word"), AppSource::Word);
    /// assert_eq!(AppSource::from_bundle_id("com.apple.iWork.Pages"), AppSource::Pages);
    /// assert_eq!(AppSource::from_bundle_id("com.unknown.app"), AppSource::Unknown);
    /// ```
    ///
    /// # Requirements
    /// - Requirement 6.10: Converts Bundle_ID to source name
    pub fn from_bundle_id(bundle_id: &str) -> Self {
        match bundle_id {
            "com.microsoft.Word" => AppSource::Word,
            "com.microsoft.Excel" => AppSource::Excel,
            "com.microsoft.Powerpoint" => AppSource::PowerPoint,
            "com.microsoft.Outlook" => AppSource::Outlook,
            "com.microsoft.teams" | "com.microsoft.teams2" => AppSource::Teams,
            "com.apple.iWork.Pages" => AppSource::Pages,
            "com.apple.iWork.Numbers" => AppSource::Numbers,
            "com.apple.iWork.Keynote" => AppSource::Keynote,
            "com.apple.TextEdit" => AppSource::TextEdit,
            "com.tinyspeck.slackmacgap" => AppSource::Slack,
            _ if bundle_id.to_lowercase().contains("libreoffice") => AppSource::LibreOffice,
            _ => AppSource::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================================
    // Property-Based Tests
    // ============================================================================

    // Feature: accessibility-extractor, Property 12: ExtractedContent Serialization Round-Trip
    // **Validates: Requirements 8.3**
    //
    // *For any* valid `ExtractedContent` instance, serializing to JSON and
    // deserializing back SHALL produce an equivalent instance.
    proptest! {
        #[test]
        fn prop_extracted_content_roundtrip_with_title(
            source in "[a-z]{1,20}",
            title in "[a-zA-Z0-9 ]{1,50}",
            content in ".{1,100}",
            app_name in "[a-zA-Z ]{1,30}",
            timestamp in 0i64..i64::MAX,
        ) {
            let original = ExtractedContent {
                source,
                title: Some(title),
                content,
                app_name,
                timestamp,
                extraction_method: "accessibility".to_string(),
            };

            let json = serde_json::to_string(&original).unwrap();
            let restored: ExtractedContent = serde_json::from_str(&json).unwrap();

            // Compare all fields to verify round-trip equivalence
            prop_assert_eq!(&original, &restored);
        }

        #[test]
        fn prop_extracted_content_roundtrip_without_title(
            source in "[a-z]{1,20}",
            content in ".{1,100}",
            app_name in "[a-zA-Z ]{1,30}",
            timestamp in 0i64..i64::MAX,
        ) {
            let original = ExtractedContent {
                source,
                title: None,
                content,
                app_name,
                timestamp,
                extraction_method: "accessibility".to_string(),
            };

            let json = serde_json::to_string(&original).unwrap();
            let restored: ExtractedContent = serde_json::from_str(&json).unwrap();

            // Compare all fields to verify round-trip equivalence
            prop_assert_eq!(&original, &restored);
        }
    }

    // Feature: accessibility-extractor, Property 8: Bundle ID to Source Mapping
    // **Validates: Requirements 6.10**
    //
    // *For any* known bundle ID (com.microsoft.Word, com.microsoft.Excel,
    // com.apple.iWork.Pages, etc.), the `AppSource::from_bundle_id` function
    // SHALL return the correct `AppSource` variant.
    proptest! {
        #[test]
        fn prop_bundle_id_to_source_mapping(
            bundle_id_idx in 0usize..9,
        ) {
            // Define the known bundle IDs and their expected AppSource mappings
            let known_mappings: Vec<(&str, AppSource)> = vec![
                ("com.microsoft.Word", AppSource::Word),
                ("com.microsoft.Excel", AppSource::Excel),
                ("com.microsoft.Powerpoint", AppSource::PowerPoint),
                ("com.microsoft.Outlook", AppSource::Outlook),
                ("com.apple.iWork.Pages", AppSource::Pages),
                ("com.apple.iWork.Numbers", AppSource::Numbers),
                ("com.apple.iWork.Keynote", AppSource::Keynote),
                ("com.apple.TextEdit", AppSource::TextEdit),
                ("org.libreoffice.script", AppSource::LibreOffice),
            ];

            let (bundle_id, expected_source) = known_mappings[bundle_id_idx];
            let actual_source = AppSource::from_bundle_id(bundle_id);

            prop_assert_eq!(
                actual_source, expected_source,
                "Bundle ID '{}' should map to {:?}, but got {:?}",
                bundle_id, expected_source, actual_source
            );
        }

        #[test]
        fn prop_bundle_id_libreoffice_variants(
            prefix in "(org|com)\\.",
            middle in "[a-zA-Z]*[Ll][Ii][Bb][Rr][Ee][Oo][Ff][Ff][Ii][Cc][Ee][a-zA-Z]*",
            suffix in "(\\.[a-zA-Z]+)?",
        ) {
            // Any bundle ID containing "libreoffice" (case-insensitive) should map to LibreOffice
            let bundle_id = format!("{}{}{}", prefix, middle, suffix);
            let source = AppSource::from_bundle_id(&bundle_id);

            prop_assert_eq!(
                source, AppSource::LibreOffice,
                "Bundle ID '{}' containing 'libreoffice' should map to LibreOffice, but got {:?}",
                bundle_id, source
            );
        }

        #[test]
        fn prop_unknown_bundle_id_returns_unknown(
            prefix in "[a-z]{2,10}\\.",
            middle in "[a-z]{2,10}\\.",
            suffix in "[a-z]{2,10}",
        ) {
            // Generate bundle IDs that don't match any known patterns
            let bundle_id = format!("{}{}{}", prefix, middle, suffix);

            // Skip if it accidentally matches a known bundle ID or contains "libreoffice"
            let is_known = bundle_id == "com.microsoft.Word"
                || bundle_id == "com.microsoft.Excel"
                || bundle_id == "com.microsoft.Powerpoint"
                || bundle_id == "com.microsoft.Outlook"
                || bundle_id == "com.apple.iWork.Pages"
                || bundle_id == "com.apple.iWork.Numbers"
                || bundle_id == "com.apple.iWork.Keynote"
                || bundle_id == "com.apple.TextEdit"
                || bundle_id.to_lowercase().contains("libreoffice");

            prop_assume!(!is_known);

            let source = AppSource::from_bundle_id(&bundle_id);

            prop_assert_eq!(
                source, AppSource::Unknown,
                "Unknown bundle ID '{}' should map to Unknown, but got {:?}",
                bundle_id, source
            );
        }
    }

    // Feature: accessibility-extractor, Property 13: Error Messages Non-Empty
    // **Validates: Requirements 8.4**
    //
    // *For any* `ExtractionError` variant, calling the `Error` trait's `to_string()`
    // method SHALL return a non-empty descriptive message.
    proptest! {
        #[test]
        fn prop_error_messages_non_empty(msg in ".+") {
            // Test all ExtractionError variants with generated message
            let errors = vec![
                ExtractionError::PermissionDenied(msg.clone()),
                ExtractionError::AppNotFound(msg.clone()),
                ExtractionError::ElementNotFound(msg.clone()),
                ExtractionError::NoContentFound(msg.clone()),
                ExtractionError::PatternNotSupported(msg.clone()),
                ExtractionError::PlatformError(msg.clone()),
                ExtractionError::Timeout(msg.clone()),
                ExtractionError::AccessibilityError(msg),
            ];

            for error in errors {
                // Verify that to_string() returns a non-empty message
                prop_assert!(!error.to_string().is_empty(),
                    "Error message should not be empty for {:?}", error);
            }
        }
    }

    // Feature: accessibility-extractor, Property 3: ExtractedContent Structure Completeness
    // **Validates: Requirements 3.6**
    //
    // *For any* successful extraction, the returned `ExtractedContent` SHALL contain:
    // - A non-empty `source` string
    // - A non-empty `content` string
    // - A non-empty `app_name` string
    // - A valid Unix `timestamp` (positive integer)
    // - An `extraction_method` equal to "accessibility"
    proptest! {
        #[test]
        fn prop_extracted_content_structure_completeness(
            source in "[a-z]{1,20}",
            content in ".{1,200}",
            app_name in "[a-zA-Z ]{1,30}",
            timestamp in 1i64..i64::MAX,  // Positive integer (valid Unix timestamp)
        ) {
            // Create an ExtractedContent instance simulating a successful extraction
            let extracted = ExtractedContent {
                source: source.clone(),
                title: None,  // title is optional
                content: content.clone(),
                app_name: app_name.clone(),
                timestamp,
                extraction_method: "accessibility".to_string(),
            };

            // Property 3.1: source SHALL be a non-empty string
            prop_assert!(
                !extracted.source.is_empty(),
                "ExtractedContent.source must be non-empty, got: '{}'",
                extracted.source
            );

            // Property 3.2: content SHALL be a non-empty string
            prop_assert!(
                !extracted.content.is_empty(),
                "ExtractedContent.content must be non-empty, got: '{}'",
                extracted.content
            );

            // Property 3.3: app_name SHALL be a non-empty string
            prop_assert!(
                !extracted.app_name.is_empty(),
                "ExtractedContent.app_name must be non-empty, got: '{}'",
                extracted.app_name
            );

            // Property 3.4: timestamp SHALL be a valid Unix timestamp (positive integer)
            prop_assert!(
                extracted.timestamp > 0,
                "ExtractedContent.timestamp must be a positive integer, got: {}",
                extracted.timestamp
            );

            // Property 3.5: extraction_method SHALL equal "accessibility"
            prop_assert_eq!(
                &extracted.extraction_method,
                "accessibility",
                "ExtractedContent.extraction_method must be 'accessibility', got: '{}'",
                extracted.extraction_method
            );
        }

        #[test]
        fn prop_extracted_content_structure_completeness_with_title(
            source in "[a-z]{1,20}",
            title in "[a-zA-Z0-9 ]{1,50}",
            content in ".{1,200}",
            app_name in "[a-zA-Z ]{1,30}",
            timestamp in 1i64..i64::MAX,  // Positive integer (valid Unix timestamp)
        ) {
            // Create an ExtractedContent instance with optional title present
            let extracted = ExtractedContent {
                source: source.clone(),
                title: Some(title),
                content: content.clone(),
                app_name: app_name.clone(),
                timestamp,
                extraction_method: "accessibility".to_string(),
            };

            // Property 3.1: source SHALL be a non-empty string
            prop_assert!(
                !extracted.source.is_empty(),
                "ExtractedContent.source must be non-empty, got: '{}'",
                extracted.source
            );

            // Property 3.2: content SHALL be a non-empty string
            prop_assert!(
                !extracted.content.is_empty(),
                "ExtractedContent.content must be non-empty, got: '{}'",
                extracted.content
            );

            // Property 3.3: app_name SHALL be a non-empty string
            prop_assert!(
                !extracted.app_name.is_empty(),
                "ExtractedContent.app_name must be non-empty, got: '{}'",
                extracted.app_name
            );

            // Property 3.4: timestamp SHALL be a valid Unix timestamp (positive integer)
            prop_assert!(
                extracted.timestamp > 0,
                "ExtractedContent.timestamp must be a positive integer, got: {}",
                extracted.timestamp
            );

            // Property 3.5: extraction_method SHALL equal "accessibility"
            prop_assert_eq!(
                &extracted.extraction_method,
                "accessibility",
                "ExtractedContent.extraction_method must be 'accessibility', got: '{}'",
                extracted.extraction_method
            );
        }
    }

    // Additional property test to verify that the structure validation function
    // correctly identifies valid ExtractedContent instances
    proptest! {
        #[test]
        fn prop_extracted_content_is_valid_structure(
            source in "[a-z]{1,20}",
            content in ".{1,200}",
            app_name in "[a-zA-Z ]{1,30}",
            timestamp in 1i64..i64::MAX,
        ) {
            let extracted = ExtractedContent {
                source,
                title: None,
                content,
                app_name,
                timestamp,
                extraction_method: "accessibility".to_string(),
            };

            // Verify the structure is valid using the is_valid_structure helper
            prop_assert!(
                is_valid_extracted_content(&extracted),
                "ExtractedContent should be valid: {:?}",
                extracted
            );
        }
    }

    /// Helper function to validate ExtractedContent structure completeness.
    /// This function checks all the requirements from Property 3.
    fn is_valid_extracted_content(content: &ExtractedContent) -> bool {
        !content.source.is_empty()
            && !content.content.is_empty()
            && !content.app_name.is_empty()
            && content.timestamp > 0
            && content.extraction_method == "accessibility"
    }

    // ============================================================================
    // Unit Tests
    // ============================================================================

    // Unit tests for AppSource::as_str
    #[test]
    fn test_app_source_as_str() {
        assert_eq!(AppSource::Word.as_str(), "word");
        assert_eq!(AppSource::Excel.as_str(), "excel");
        assert_eq!(AppSource::PowerPoint.as_str(), "powerpoint");
        assert_eq!(AppSource::Outlook.as_str(), "outlook");
        assert_eq!(AppSource::Teams.as_str(), "teams");
        assert_eq!(AppSource::Pages.as_str(), "pages");
        assert_eq!(AppSource::Numbers.as_str(), "numbers");
        assert_eq!(AppSource::Keynote.as_str(), "keynote");
        assert_eq!(AppSource::TextEdit.as_str(), "textedit");
        assert_eq!(AppSource::LibreOffice.as_str(), "libreoffice");
        assert_eq!(AppSource::Slack.as_str(), "slack");
        assert_eq!(AppSource::Unknown.as_str(), "unknown");
    }

    // Unit tests for AppSource::from_bundle_id
    #[test]
    fn test_app_source_from_bundle_id_microsoft() {
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.Word"),
            AppSource::Word
        );
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.Excel"),
            AppSource::Excel
        );
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.Powerpoint"),
            AppSource::PowerPoint
        );
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.Outlook"),
            AppSource::Outlook
        );
        // Teams - both classic and new versions
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.teams"),
            AppSource::Teams
        );
        assert_eq!(
            AppSource::from_bundle_id("com.microsoft.teams2"),
            AppSource::Teams
        );
    }

    #[test]
    fn test_app_source_from_bundle_id_apple() {
        assert_eq!(
            AppSource::from_bundle_id("com.apple.iWork.Pages"),
            AppSource::Pages
        );
        assert_eq!(
            AppSource::from_bundle_id("com.apple.iWork.Numbers"),
            AppSource::Numbers
        );
        assert_eq!(
            AppSource::from_bundle_id("com.apple.iWork.Keynote"),
            AppSource::Keynote
        );
        assert_eq!(
            AppSource::from_bundle_id("com.apple.TextEdit"),
            AppSource::TextEdit
        );
    }

    #[test]
    fn test_app_source_from_bundle_id_libreoffice() {
        assert_eq!(
            AppSource::from_bundle_id("org.libreoffice.script"),
            AppSource::LibreOffice
        );
        assert_eq!(
            AppSource::from_bundle_id("org.LibreOffice.Writer"),
            AppSource::LibreOffice
        );
    }

    #[test]
    fn test_app_source_from_bundle_id_unknown() {
        assert_eq!(
            AppSource::from_bundle_id("com.unknown.app"),
            AppSource::Unknown
        );
        assert_eq!(AppSource::from_bundle_id(""), AppSource::Unknown);
    }

    // Unit tests for ExtractedContent serialization
    #[test]
    fn test_extracted_content_serialization() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Test Document.docx".to_string()),
            content: "Hello, world!".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        let deserialized: ExtractedContent =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(content, deserialized);
    }

    #[test]
    fn test_extracted_content_with_none_title() {
        let content = ExtractedContent {
            source: "pages".to_string(),
            title: None,
            content: "Some content".to_string(),
            app_name: "Pages".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let json = serde_json::to_string(&content).expect("Serialization should succeed");
        assert!(json.contains("\"title\":null"));

        let deserialized: ExtractedContent =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(content, deserialized);
    }

    // Unit tests for CapturePayload serialization
    #[test]
    fn test_capture_payload_serialization() {
        let payload = CapturePayload {
            source: "word".to_string(),
            url: "accessibility://Microsoft_Word/Document.docx".to_string(),
            content: "Document content".to_string(),
            title: Some("Document.docx".to_string()),
            author: None,
            channel: None,
            timestamp: Some(1707500000),
        };

        let json = serde_json::to_string(&payload).expect("Serialization should succeed");
        let deserialized: CapturePayload =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_capture_payload_skips_none_fields() {
        let payload = CapturePayload {
            source: "excel".to_string(),
            url: "accessibility://Microsoft_Excel/Sheet.xlsx".to_string(),
            content: "Spreadsheet content".to_string(),
            title: None,
            author: None,
            channel: None,
            timestamp: None,
        };

        let json = serde_json::to_string(&payload).expect("Serialization should succeed");

        // None fields should be skipped in serialization
        assert!(!json.contains("\"title\""));
        assert!(!json.contains("\"author\""));
        assert!(!json.contains("\"channel\""));
        assert!(!json.contains("\"timestamp\""));
    }

    // Unit tests for ExtractionError
    #[test]
    fn test_extraction_error_display() {
        let errors = vec![
            (
                ExtractionError::PermissionDenied("test message".to_string()),
                "Permission denied: test message",
            ),
            (
                ExtractionError::AppNotFound("com.test.app".to_string()),
                "Application not found: com.test.app",
            ),
            (
                ExtractionError::ElementNotFound("no window".to_string()),
                "Element not found: no window",
            ),
            (
                ExtractionError::NoContentFound("empty doc".to_string()),
                "No content found: empty doc",
            ),
            (
                ExtractionError::PatternNotSupported("custom app".to_string()),
                "Pattern not supported: custom app",
            ),
            (
                ExtractionError::PlatformError("API error".to_string()),
                "Platform error: API error",
            ),
            (
                ExtractionError::Timeout("5 seconds".to_string()),
                "Timeout: 5 seconds",
            ),
        ];

        for (error, expected_message) in errors {
            assert_eq!(error.to_string(), expected_message);
        }
    }

    #[test]
    fn test_extraction_error_messages_non_empty() {
        let errors = vec![
            ExtractionError::PermissionDenied("test".to_string()),
            ExtractionError::AppNotFound("test".to_string()),
            ExtractionError::ElementNotFound("test".to_string()),
            ExtractionError::NoContentFound("test".to_string()),
            ExtractionError::PatternNotSupported("test".to_string()),
            ExtractionError::PlatformError("test".to_string()),
            ExtractionError::Timeout("test".to_string()),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }

    // ============================================================================
    // Unit Tests for ChunkMeta
    // ============================================================================

    #[test]
    fn test_chunk_meta_new() {
        let meta = ChunkMeta::new(
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            "word".to_string(),
            1707500000,
            "This is the document content.",
            0,
            1,
        );

        assert_eq!(meta.id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(meta.source, "word");
        assert_eq!(meta.timestamp, 1707500000);
        assert_eq!(meta.header, "This is the document content.");
        assert_eq!(meta.chunk_idx, 0);
        assert_eq!(meta.total_chunks, 1);
        assert!(meta.path.is_none());
        assert!(meta.app_id.is_none());
        assert!(meta.url.is_none());
        assert!(meta.title.is_none());
    }

    #[test]
    fn test_chunk_meta_header_truncation() {
        // Create content longer than 200 characters
        let long_content = "A".repeat(300);
        let meta = ChunkMeta::new(
            "test-id".to_string(),
            "word".to_string(),
            1707500000,
            &long_content,
            0,
            1,
        );

        // Header should be truncated to 200 characters
        assert_eq!(meta.header.chars().count(), 200);
        assert_eq!(meta.header, "A".repeat(200));
    }

    #[test]
    fn test_chunk_meta_builder_methods() {
        let meta = ChunkMeta::new(
            "test-id".to_string(),
            "word".to_string(),
            1707500000,
            "Content",
            0,
            1,
        )
        .with_app_id("com.microsoft.Word".to_string())
        .with_url("accessibility://Microsoft_Word/Document.docx".to_string())
        .with_title("Document.docx".to_string());

        assert_eq!(meta.app_id, Some("com.microsoft.Word".to_string()));
        assert_eq!(
            meta.url,
            Some("accessibility://Microsoft_Word/Document.docx".to_string())
        );
        assert_eq!(meta.title, Some("Document.docx".to_string()));
    }

    #[test]
    fn test_chunk_meta_serialization() {
        let meta = ChunkMeta::new(
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            "word".to_string(),
            1707500000,
            "Document content",
            0,
            1,
        )
        .with_app_id("com.microsoft.Word".to_string())
        .with_title("Document.docx".to_string());

        let json = serde_json::to_string(&meta).expect("Serialization should succeed");
        let deserialized: ChunkMeta =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(meta, deserialized);
    }

    #[test]
    fn test_chunk_meta_skips_none_fields() {
        let meta = ChunkMeta::new(
            "test-id".to_string(),
            "word".to_string(),
            1707500000,
            "Content",
            0,
            1,
        );

        let json = serde_json::to_string(&meta).expect("Serialization should succeed");

        // None fields should be skipped in serialization
        assert!(!json.contains("\"path\""));
        assert!(!json.contains("\"ext\""));
        assert!(!json.contains("\"file_size\""));
        assert!(!json.contains("\"file_modified\""));
        assert!(!json.contains("\"app_id\""));
        assert!(!json.contains("\"channel\""));
        assert!(!json.contains("\"author\""));
        assert!(!json.contains("\"url\""));
        assert!(!json.contains("\"title\""));
    }

    // ============================================================================
    // Unit Tests for generate_doc_id
    // ============================================================================

    #[test]
    fn test_generate_doc_id_format() {
        let doc_id = generate_doc_id();

        // UUID v4 format: 8-4-4-4-12 (36 characters with hyphens)
        assert_eq!(doc_id.len(), 36);
        assert_eq!(doc_id.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn test_generate_doc_id_uniqueness() {
        let id1 = generate_doc_id();
        let id2 = generate_doc_id();
        let id3 = generate_doc_id();

        // Each call should generate a unique ID
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    // ============================================================================
    // Unit Tests for generate_content_hash
    // ============================================================================

    #[test]
    fn test_generate_content_hash_format() {
        let hash = generate_content_hash("Hello, world!");

        // SHA-256 produces 64 hexadecimal characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_content_hash_deterministic() {
        let content = "Hello, world!";
        let hash1 = generate_content_hash(content);
        let hash2 = generate_content_hash(content);

        // Same content should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_content_hash_different_content() {
        let hash1 = generate_content_hash("Hello, world!");
        let hash2 = generate_content_hash("Different content");

        // Different content should produce different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_content_hash_known_value() {
        // Known SHA-256 hash for "Hello, world!"
        let hash = generate_content_hash("Hello, world!");
        assert_eq!(
            hash,
            "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3"
        );
    }

    #[test]
    fn test_generate_content_hash_empty_string() {
        // SHA-256 hash of empty string
        let hash = generate_content_hash("");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // ============================================================================
    // Property-Based Tests for Database Integration Types
    // ============================================================================

    // Feature: accessibility-extractor, Property 16: Unique Document Identifiers
    // **Validates: Requirements 11.8**
    //
    // *For any* two distinct extractions, the generated `ehl_doc_id` values
    // SHALL be different (UUID v4 uniqueness).
    proptest! {
        #[test]
        fn prop_generate_doc_id_uniqueness(
            _seed in 0u64..1000000,  // Just to run multiple iterations
        ) {
            let id1 = generate_doc_id();
            let id2 = generate_doc_id();

            // Verify UUID format first (36 characters with 4 hyphens)
            prop_assert_eq!(id1.len(), 36, "UUID should be 36 characters");
            prop_assert_eq!(id2.len(), 36, "UUID should be 36 characters");

            // Each call should generate a unique ID
            prop_assert_ne!(
                id1, id2,
                "Two consecutive calls to generate_doc_id should produce different IDs"
            );
        }

        // Property test for content hash determinism
        #[test]
        fn prop_content_hash_deterministic(
            content in ".{1,500}",
        ) {
            let hash1 = generate_content_hash(&content);
            let hash2 = generate_content_hash(&content);

            // Hash should be 64 hex characters
            prop_assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 characters");
            prop_assert!(
                hash1.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash should only contain hex characters"
            );

            // Same content should always produce same hash
            prop_assert_eq!(
                hash1, hash2,
                "Same content should produce same hash"
            );
        }

        // Property test for content hash uniqueness (different content -> different hash)
        #[test]
        fn prop_content_hash_different_for_different_content(
            content1 in ".{1,100}",
            content2 in ".{1,100}",
        ) {
            // Skip if contents are the same
            prop_assume!(content1 != content2);

            let hash1 = generate_content_hash(&content1);
            let hash2 = generate_content_hash(&content2);

            // Different content should produce different hash (with very high probability)
            prop_assert_ne!(
                hash1, hash2,
                "Different content should produce different hash"
            );
        }

        // Property test for ChunkMeta header truncation
        #[test]
        fn prop_chunk_meta_header_max_200_chars(
            content in ".{1,500}",
        ) {
            let meta = ChunkMeta::new(
                "test-id".to_string(),
                "word".to_string(),
                1707500000,
                &content,
                0,
                1,
            );

            // Header should never exceed 200 characters (not bytes)
            let header_char_count = meta.header.chars().count();
            prop_assert!(
                header_char_count <= 200,
                "Header should be at most 200 characters, got {}",
                header_char_count
            );

            // If content is <= 200 chars, header should equal content
            let content_char_count = content.chars().count();
            if content_char_count <= 200 {
                prop_assert_eq!(
                    meta.header, content,
                    "Header should equal content when content <= 200 chars"
                );
            }
        }

        // Property test for ChunkMeta serialization round-trip
        #[test]
        fn prop_chunk_meta_roundtrip(
            id in "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
            source in "[a-z]{1,20}",
            timestamp in 0i64..i64::MAX,
            content in ".{1,200}",
            chunk_idx in 0u32..100,
            total_chunks in 1u32..100,
        ) {
            let original = ChunkMeta::new(
                id,
                source,
                timestamp,
                &content,
                chunk_idx,
                total_chunks,
            );

            let json = serde_json::to_string(&original).unwrap();
            let restored: ChunkMeta = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(&original, &restored);
        }
    }
}
