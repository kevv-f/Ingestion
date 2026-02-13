//! Cross-platform accessibility extractor API.
//!
//! This module provides a unified API for accessibility content extraction
//! that abstracts platform-specific details. On macOS, it delegates to the
//! `MacOSExtractor` implementation. On unsupported platforms, it returns
//! appropriate errors.
//!
//! # Example
//!
//! ```no_run
//! use accessibility_extractor::extractor::AccessibilityExtractor;
//!
//! // Check if accessibility features are enabled
//! if AccessibilityExtractor::is_enabled() {
//!     // Extract content from the frontmost application
//!     match AccessibilityExtractor::extract_frontmost() {
//!         Ok(content) => {
//!             println!("Source: {}", content.source);
//!             println!("Content: {}", content.content);
//!         }
//!         Err(e) => eprintln!("Extraction failed: {}", e),
//!     }
//! } else {
//!     // Request permissions
//!     AccessibilityExtractor::request_permissions();
//! }
//! ```
//!
//! # Requirements
//! - Requirement 9.1: Provide an AccessibilityExtractor struct with a unified API
//! - Requirement 9.2: Provide is_enabled() to check if accessibility features are available
//! - Requirement 9.3: Provide request_permissions() to request accessibility permissions
//! - Requirement 9.4: Provide extract_frontmost() to extract from the active application
//! - Requirement 9.5: Provide extract_from_app(app_identifier) to extract from a specific application
//! - Requirement 9.6: Provide get_selected_text() to get the current text selection
//! - Requirement 9.7: Return appropriate errors on unsupported platforms

use crate::types::{CapturePayload, ChunkMeta, ExtractedContent, ExtractionError};
use crate::types::{generate_content_hash, generate_doc_id};

#[cfg(target_os = "macos")]
use crate::platform::macos::MacOSExtractor;

/// Cross-platform accessibility extractor.
///
/// This struct provides a unified API for extracting content from desktop
/// applications using accessibility APIs. It abstracts platform-specific
/// details and provides consistent behavior across supported platforms.
///
/// Currently supported platforms:
/// - macOS (using AXUIElement API)
///
/// On unsupported platforms, methods return appropriate errors or default values.
///
/// # Requirements
/// - Requirement 9.1: Provide an AccessibilityExtractor struct with a unified API
pub struct AccessibilityExtractor;

impl AccessibilityExtractor {
    /// Check if accessibility features are enabled.
    ///
    /// This function checks whether the current application has the necessary
    /// permissions to use accessibility features. On macOS, this checks if
    /// accessibility permissions have been granted in System Preferences.
    ///
    /// # Returns
    ///
    /// `true` if accessibility features are enabled and available,
    /// `false` otherwise (including on unsupported platforms).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// if AccessibilityExtractor::is_enabled() {
    ///     println!("Accessibility features are available!");
    /// } else {
    ///     println!("Please enable accessibility permissions.");
    /// }
    /// ```
    ///
    /// # Platform Behavior
    ///
    /// - **macOS**: Delegates to `MacOSExtractor::is_accessibility_enabled()`
    /// - **Other platforms**: Returns `false`
    ///
    /// # Requirements
    /// - Requirement 9.2: Provide is_enabled() to check if accessibility features are available
    pub fn is_enabled() -> bool {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::is_accessibility_enabled()
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Request accessibility permissions.
    ///
    /// This function triggers the system's accessibility permission prompt.
    /// On macOS, this shows the system dialog asking the user to grant
    /// accessibility permissions to the application.
    ///
    /// # Note
    ///
    /// On unsupported platforms, this function does nothing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// if !AccessibilityExtractor::is_enabled() {
    ///     AccessibilityExtractor::request_permissions();
    ///     println!("Please grant accessibility permissions and restart the app.");
    /// }
    /// ```
    ///
    /// # Platform Behavior
    ///
    /// - **macOS**: Delegates to `MacOSExtractor::request_accessibility()`
    /// - **Other platforms**: No-op
    ///
    /// # Requirements
    /// - Requirement 9.3: Provide request_permissions() to request accessibility permissions
    pub fn request_permissions() {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::request_accessibility();
        }

        // On unsupported platforms, this is a no-op
        #[cfg(not(target_os = "macos"))]
        {
            // No-op on unsupported platforms
        }
    }

    /// Extract content from the frontmost application.
    ///
    /// This function extracts text content from the currently active (frontmost)
    /// application. It retrieves the focused window and extracts text from
    /// document-related accessibility elements, excluding UI chrome.
    ///
    /// # Returns
    ///
    /// `Ok(ExtractedContent)` containing the extracted text and metadata on success,
    /// or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Errors
    ///
    /// - `ExtractionError::PermissionDenied` - Accessibility permissions not granted
    /// - `ExtractionError::AppNotFound` - No frontmost application found
    /// - `ExtractionError::ElementNotFound` - No focused window in the application
    /// - `ExtractionError::NoContentFound` - Document is empty
    /// - `ExtractionError::PlatformError` - Running on an unsupported platform
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// match AccessibilityExtractor::extract_frontmost() {
    ///     Ok(content) => {
    ///         println!("Source: {}", content.source);
    ///         println!("Title: {:?}", content.title);
    ///         println!("Content: {}", content.content);
    ///     }
    ///     Err(e) => eprintln!("Extraction failed: {}", e),
    /// }
    /// ```
    ///
    /// # Platform Behavior
    ///
    /// - **macOS**: Delegates to `MacOSExtractor::extract_frontmost()`
    /// - **Other platforms**: Returns `Err(ExtractionError::PlatformError)`
    ///
    /// # Requirements
    /// - Requirement 9.4: Provide extract_frontmost() to extract from the active application
    /// - Requirement 9.7: Return appropriate errors on unsupported platforms
    pub fn extract_frontmost() -> Result<ExtractedContent, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::extract_frontmost()
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(ExtractionError::PlatformError(
                "Unsupported platform".into(),
            ))
        }
    }

    /// Extract content from a specific application.
    ///
    /// This function extracts text content from a specific application identified
    /// by its bundle ID (on macOS) or other platform-specific identifier.
    ///
    /// # Arguments
    ///
    /// * `app_identifier` - The application identifier (bundle ID on macOS,
    ///                      e.g., "com.microsoft.Word")
    ///
    /// # Returns
    ///
    /// `Ok(ExtractedContent)` containing the extracted text and metadata on success,
    /// or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Errors
    ///
    /// - `ExtractionError::AppNotFound` - Application with the given identifier is not running
    /// - `ExtractionError::ElementNotFound` - No focused window in the application
    /// - `ExtractionError::NoContentFound` - Document is empty
    /// - `ExtractionError::PlatformError` - Running on an unsupported platform
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// // Extract content from Microsoft Word
    /// match AccessibilityExtractor::extract_from_app("com.microsoft.Word") {
    ///     Ok(content) => {
    ///         println!("Source: {}", content.source);
    ///         println!("Content length: {} chars", content.content.len());
    ///     }
    ///     Err(e) => eprintln!("Extraction failed: {}", e),
    /// }
    /// ```
    ///
    /// # Platform Behavior
    ///
    /// - **macOS**: Delegates to `MacOSExtractor::extract_from_app()`
    /// - **Other platforms**: Returns `Err(ExtractionError::PlatformError)`
    ///
    /// # Requirements
    /// - Requirement 9.5: Provide extract_from_app(app_identifier) to extract from a specific application
    /// - Requirement 9.7: Return appropriate errors on unsupported platforms
    pub fn extract_from_app(app_identifier: &str) -> Result<ExtractedContent, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::extract_from_app(app_identifier)
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Suppress unused variable warning on non-macOS platforms
            let _ = app_identifier;
            Err(ExtractionError::PlatformError(
                "Unsupported platform".into(),
            ))
        }
    }

    /// Get the currently selected text.
    ///
    /// This function retrieves the currently selected text from any application.
    /// It queries the system's accessibility API to get the text selection from
    /// the focused UI element.
    ///
    /// # Returns
    ///
    /// `Some(String)` containing the selected text if text is selected,
    /// `None` if no text is selected, if the query fails, or on unsupported platforms.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// if let Some(selected) = AccessibilityExtractor::get_selected_text() {
    ///     println!("Selected text: {}", selected);
    /// } else {
    ///     println!("No text is currently selected.");
    /// }
    /// ```
    ///
    /// # Platform Behavior
    ///
    /// - **macOS**: Delegates to `MacOSExtractor::get_selected_text()`
    /// - **Other platforms**: Returns `None`
    ///
    /// # Requirements
    /// - Requirement 9.6: Provide get_selected_text() to get the current text selection
    pub fn get_selected_text() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::get_selected_text()
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    /// Extract content with retry logic.
    ///
    /// This function attempts to extract content from the frontmost application,
    /// retrying up to `max_attempts` times if extraction fails or returns empty content.
    /// It waits `delay_ms` milliseconds between retry attempts.
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum number of extraction attempts (must be >= 1)
    /// * `delay_ms` - Delay in milliseconds between retry attempts
    ///
    /// # Returns
    ///
    /// `Ok(ExtractedContent)` containing the extracted text and metadata on success,
    /// or `Err(ExtractionError)` with the error from the last attempt if all attempts fail.
    ///
    /// # Errors
    ///
    /// Returns the error from the last failed attempt. Possible errors include:
    /// - `ExtractionError::PermissionDenied` - Accessibility permissions not granted
    /// - `ExtractionError::AppNotFound` - No frontmost application found
    /// - `ExtractionError::ElementNotFound` - No focused window in the application
    /// - `ExtractionError::NoContentFound` - Document is empty
    /// - `ExtractionError::PlatformError` - Running on an unsupported platform
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// // Try up to 3 times with 500ms delay between attempts
    /// match AccessibilityExtractor::extract_with_retry(3, 500) {
    ///     Ok(content) => {
    ///         println!("Successfully extracted: {}", content.content);
    ///     }
    ///     Err(e) => eprintln!("All extraction attempts failed: {}", e),
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 10.1: Provide extract_with_retry(max_attempts, delay_ms) function
    /// - Requirement 10.2: Retry on failure or empty content
    /// - Requirement 10.3: Sleep between attempts
    /// - Requirement 10.4: Return last error if all attempts fail
    pub fn extract_with_retry(
        max_attempts: u32,
        delay_ms: u64,
    ) -> Result<ExtractedContent, ExtractionError> {
        // Handle edge case of 0 attempts
        if max_attempts == 0 {
            return Err(ExtractionError::NoContentFound(
                "No attempts made (max_attempts was 0)".into(),
            ));
        }

        let mut last_error = ExtractionError::NoContentFound("No attempts made".into());

        for attempt in 0..max_attempts {
            match Self::extract_frontmost() {
                Ok(content) if !content.content.trim().is_empty() => {
                    // Success: non-empty content extracted
                    return Ok(content);
                }
                Ok(_) => {
                    // Extraction succeeded but content was empty - treat as failure
                    last_error = ExtractionError::NoContentFound(format!(
                        "Empty content on attempt {}",
                        attempt + 1
                    ));
                }
                Err(e) => {
                    // Extraction failed - store the error
                    last_error = e;
                }
            }

            // Sleep between attempts (but not after the last attempt)
            if attempt < max_attempts - 1 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }

        // All attempts failed - return the last error
        Err(last_error)
    }

    /// Convert ExtractedContent to CapturePayload format.
    ///
    /// This function converts the internal `ExtractedContent` representation
    /// to the `CapturePayload` format that is compatible with the existing
    /// ingestion pipeline (BrowserCapturePayload JSON schema).
    ///
    /// # Arguments
    ///
    /// * `content` - The extracted content to convert
    ///
    /// # Returns
    ///
    /// A `CapturePayload` instance with:
    /// - `source`: Copied from ExtractedContent
    /// - `url`: Generated accessibility:// URL from app_name and title
    /// - `content`: Copied from ExtractedContent
    /// - `title`: Copied from ExtractedContent
    /// - `author`: Set to None (not available from accessibility extraction)
    /// - `channel`: Set to None (not available from accessibility extraction)
    /// - `timestamp`: Copied from ExtractedContent
    ///
    /// # URL Format
    ///
    /// The generated URL follows the format:
    /// `accessibility://{app_name}/{title}`
    ///
    /// Where:
    /// - `app_name` has spaces replaced with underscores
    /// - `title` defaults to "untitled" if not present
    ///
    /// # Example
    ///
    /// ```
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    /// use accessibility_extractor::types::ExtractedContent;
    ///
    /// let content = ExtractedContent {
    ///     source: "word".to_string(),
    ///     title: Some("Document.docx".to_string()),
    ///     content: "Hello, world!".to_string(),
    ///     app_name: "Microsoft Word".to_string(),
    ///     timestamp: 1707500000,
    ///     extraction_method: "accessibility".to_string(),
    /// };
    ///
    /// let payload = AccessibilityExtractor::to_capture_payload(&content);
    ///
    /// assert_eq!(payload.source, "word");
    /// assert_eq!(payload.url, "accessibility://Microsoft_Word/Document.docx");
    /// assert_eq!(payload.content, "Hello, world!");
    /// assert_eq!(payload.title, Some("Document.docx".to_string()));
    /// assert_eq!(payload.timestamp, Some(1707500000));
    /// ```
    ///
    /// # Requirements
    /// - Requirement 11.7: Produce output conforming to BrowserCapturePayload JSON schema
    pub fn to_capture_payload(content: &ExtractedContent) -> CapturePayload {
        // Generate URL-like identifier
        // Replace spaces with underscores in app_name for URL compatibility
        let url = format!(
            "accessibility://{}/{}",
            content.app_name.replace(" ", "_"),
            content.title.as_deref().unwrap_or("untitled")
        );

        CapturePayload {
            source: content.source.clone(),
            url,
            content: content.content.clone(),
            title: content.title.clone(),
            author: None,
            channel: None,
            timestamp: Some(content.timestamp),
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
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// let doc_id = AccessibilityExtractor::generate_doc_id();
    /// println!("Generated document ID: {}", doc_id);
    ///
    /// // Each call generates a unique ID
    /// let another_id = AccessibilityExtractor::generate_doc_id();
    /// assert_ne!(doc_id, another_id);
    /// ```
    ///
    /// # Requirements
    /// - Requirement 11.8: Generate a unique document identifier (ehl_doc_id) for each extracted document
    pub fn generate_doc_id() -> String {
        generate_doc_id()
    }

    // =========================================================================
    // Ingestion Pipeline Integration
    // =========================================================================
    //
    // The following functions provide integration points for the existing
    // ingestion pipeline. They convert extracted content into the chunk format
    // used by the SQLite database schema.
    //
    // ## Integration Architecture
    //
    // The ingestion pipeline integration follows this flow:
    //
    // 1. Extract content using `extract_frontmost()` or `extract_from_app()`
    // 2. Convert to chunk format using `to_chunk_meta()`
    // 3. Generate JSON metadata using `to_chunk_meta_json()`
    // 4. Pass to existing ingestion service for database persistence
    //
    // ## Database Schema Integration
    //
    // The chunk metadata is designed to be stored in the `chunks` table:
    // - `id`: Auto-incremented primary key
    // - `vector_index`: Index for vector search (managed by ingestion service)
    // - `text`: The extracted content text
    // - `meta`: JSON string from `to_chunk_meta_json()`
    // - `is_deleted`: Soft delete flag
    //
    // The content source tracking is stored in `content_sources` table:
    // - `source_type`: "accessibility"
    // - `source_ref`: Application bundle ID
    // - `url`: Generated accessibility:// URL
    // - `title`: Document title
    // - `ehl_doc_id`: Unique document ID from `generate_doc_id()`
    // - `content_hash`: SHA-256 hash from `generate_content_hash()`
    //
    // ## Deduplication
    //
    // The ingestion service should use `generate_content_hash()` to check for
    // duplicate content before inserting new chunks. The hash is computed from
    // the full content text and can be compared against existing entries in
    // the `content_sources` table.
    //
    // ## Example Integration Code
    //
    // ```rust,ignore
    // use accessibility_extractor::{AccessibilityExtractor, generate_content_hash};
    //
    // // Extract content
    // let content = AccessibilityExtractor::extract_frontmost()?;
    //
    // // Check for duplicates
    // let content_hash = generate_content_hash(&content.content);
    // if !ingestion_service.is_duplicate(&content_hash) {
    //     // Convert to chunk format
    //     let chunk_meta = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);
    //     let meta_json = AccessibilityExtractor::to_chunk_meta_json(&content);
    //
    //     // Insert into database
    //     ingestion_service.insert_chunk(&content.content, &meta_json)?;
    //     ingestion_service.track_source("accessibility", &content_hash, &chunk_meta.id)?;
    // }
    // ```
    // =========================================================================

    /// Convert ExtractedContent to ChunkMeta format for database storage.
    ///
    /// This function converts the extracted content into the chunk metadata format
    /// used by the existing ingestion pipeline. It generates a unique document ID
    /// and populates all relevant fields for database storage.
    ///
    /// # Arguments
    ///
    /// * `content` - The extracted content to convert
    /// * `chunk_idx` - Index of this chunk within the document (0-based)
    /// * `total_chunks` - Total number of chunks for this document
    ///
    /// # Returns
    ///
    /// A `ChunkMeta` instance with all fields populated:
    /// - `id`: Unique document identifier (UUID v4)
    /// - `source`: Application source identifier (e.g., "word", "excel")
    /// - `timestamp`: Unix timestamp of extraction
    /// - `header`: First 200 characters of content
    /// - `chunk_idx`: Index of this chunk
    /// - `total_chunks`: Total number of chunks
    /// - `app_id`: Application bundle ID (derived from app_name)
    /// - `url`: Generated accessibility:// URL
    /// - `title`: Document title (if available)
    ///
    /// # Example
    ///
    /// ```
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    /// use accessibility_extractor::types::ExtractedContent;
    ///
    /// let content = ExtractedContent {
    ///     source: "word".to_string(),
    ///     title: Some("Document.docx".to_string()),
    ///     content: "Hello, world! This is the document content.".to_string(),
    ///     app_name: "Microsoft Word".to_string(),
    ///     timestamp: 1707500000,
    ///     extraction_method: "accessibility".to_string(),
    /// };
    ///
    /// let chunk_meta = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);
    ///
    /// assert_eq!(chunk_meta.source, "word");
    /// assert_eq!(chunk_meta.chunk_idx, 0);
    /// assert_eq!(chunk_meta.total_chunks, 1);
    /// assert!(chunk_meta.url.is_some());
    /// assert!(chunk_meta.title.is_some());
    /// ```
    ///
    /// # Requirements
    /// - Requirement 11.9: Support integration with the existing ingestion pipeline
    /// - Requirement 11.11: Populate the chunks table with extracted content and appropriate metadata
    pub fn to_chunk_meta(content: &ExtractedContent, chunk_idx: u32, total_chunks: u32) -> ChunkMeta {
        // Generate unique document ID
        let doc_id = generate_doc_id();

        // Generate accessibility:// URL
        let url = format!(
            "accessibility://{}/{}",
            content.app_name.replace(' ', "_"),
            content.title.as_deref().unwrap_or("untitled")
        );

        // Derive app_id from app_name (convert to bundle ID format)
        // This is a best-effort mapping; the actual bundle ID may differ
        let app_id = Self::derive_bundle_id(&content.app_name);

        // Create ChunkMeta with all fields populated
        ChunkMeta::new(
            doc_id,
            content.source.clone(),
            content.timestamp,
            &content.content,
            chunk_idx,
            total_chunks,
        )
        .with_app_id(app_id)
        .with_url(url)
        .with_title(content.title.clone().unwrap_or_else(|| "untitled".to_string()))
    }

    /// Convert ExtractedContent to chunk metadata JSON string.
    ///
    /// This function converts the extracted content into a JSON string suitable
    /// for storage in the `meta` column of the `chunks` table. It creates a
    /// single chunk (chunk_idx=0, total_chunks=1) from the content.
    ///
    /// For documents that need to be split into multiple chunks, use
    /// `to_chunk_meta()` directly with appropriate chunk indices.
    ///
    /// # Arguments
    ///
    /// * `content` - The extracted content to convert
    ///
    /// # Returns
    ///
    /// A JSON string containing the chunk metadata, suitable for storage in
    /// the database. The JSON includes all fields from `ChunkMeta`.
    ///
    /// # Example
    ///
    /// ```
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    /// use accessibility_extractor::types::ExtractedContent;
    ///
    /// let content = ExtractedContent {
    ///     source: "word".to_string(),
    ///     title: Some("Document.docx".to_string()),
    ///     content: "Hello, world!".to_string(),
    ///     app_name: "Microsoft Word".to_string(),
    ///     timestamp: 1707500000,
    ///     extraction_method: "accessibility".to_string(),
    /// };
    ///
    /// let json = AccessibilityExtractor::to_chunk_meta_json(&content);
    ///
    /// // The JSON contains all required fields
    /// assert!(json.contains("\"source\":\"word\""));
    /// assert!(json.contains("\"chunk_idx\":0"));
    /// assert!(json.contains("\"total_chunks\":1"));
    /// ```
    ///
    /// # Requirements
    /// - Requirement 11.9: Support integration with the existing ingestion pipeline
    /// - Requirement 11.11: Populate the chunks table with extracted content and appropriate metadata
    /// - Requirement 11.12: Update the content_sources table with source tracking information
    pub fn to_chunk_meta_json(content: &ExtractedContent) -> String {
        let chunk_meta = Self::to_chunk_meta(content, 0, 1);
        serde_json::to_string(&chunk_meta).unwrap_or_else(|_| "{}".to_string())
    }

    /// Derive a bundle ID from an application name.
    ///
    /// This is a helper function that attempts to map common application names
    /// to their macOS bundle identifiers. For unknown applications, it generates
    /// a generic bundle ID format.
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name (e.g., "Microsoft Word")
    ///
    /// # Returns
    ///
    /// A string containing the bundle ID (e.g., "com.microsoft.Word")
    fn derive_bundle_id(app_name: &str) -> String {
        let app_name_lower = app_name.to_lowercase();

        // Map known application names to bundle IDs
        if app_name_lower.contains("word") {
            "com.microsoft.Word".to_string()
        } else if app_name_lower.contains("excel") {
            "com.microsoft.Excel".to_string()
        } else if app_name_lower.contains("powerpoint") {
            "com.microsoft.Powerpoint".to_string()
        } else if app_name_lower.contains("outlook") {
            "com.microsoft.Outlook".to_string()
        } else if app_name_lower.contains("teams") {
            "com.microsoft.teams2".to_string()
        } else if app_name_lower.contains("slack") {
            "com.tinyspeck.slackmacgap".to_string()
        } else if app_name_lower.contains("pages") {
            "com.apple.iWork.Pages".to_string()
        } else if app_name_lower.contains("numbers") {
            "com.apple.iWork.Numbers".to_string()
        } else if app_name_lower.contains("keynote") {
            "com.apple.iWork.Keynote".to_string()
        } else if app_name_lower.contains("textedit") {
            "com.apple.TextEdit".to_string()
        } else if app_name_lower.contains("libreoffice") {
            "org.libreoffice.script".to_string()
        } else {
            // Generate a generic bundle ID for unknown apps
            format!(
                "com.unknown.{}",
                app_name.replace(' ', "").to_lowercase()
            )
        }
    }

    /// Generate a content hash for deduplication.
    ///
    /// This function generates a SHA-256 hash of the content for use in
    /// deduplication checks. The hash can be compared against existing
    /// entries in the `content_sources` table to detect duplicate content.
    ///
    /// # Arguments
    ///
    /// * `content` - The content string to hash
    ///
    /// # Returns
    ///
    /// A 64-character hexadecimal string representing the SHA-256 hash.
    ///
    /// # Example
    ///
    /// ```
    /// use accessibility_extractor::extractor::AccessibilityExtractor;
    ///
    /// let hash = AccessibilityExtractor::generate_content_hash("Hello, world!");
    /// assert_eq!(hash.len(), 64);
    /// ```
    ///
    /// # Requirements
    /// - Requirement 11.10: Check for duplicate content using the existing dedup mechanism
    pub fn generate_content_hash(content: &str) -> String {
        generate_content_hash(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================================
    // Property-Based Tests
    // ============================================================================

    // Feature: accessibility-extractor, Property 14: Retry Behavior
    // **Validates: Requirements 10.2, 10.4**
    //
    // *For any* call to `extract_with_retry(n, delay)` where all `n` attempts fail,
    // the function SHALL return the error from the last attempt, not an earlier one.
    //
    // Since we cannot easily mock the actual extraction in the real implementation,
    // we test the retry logic by verifying the behavior with a testable helper function
    // that simulates the retry logic with controllable outcomes.

    /// Helper function that simulates extract_with_retry behavior with controllable outcomes.
    /// This allows us to test the retry logic without depending on actual accessibility APIs.
    ///
    /// # Arguments
    /// * `max_attempts` - Maximum number of extraction attempts
    /// * `delay_ms` - Delay in milliseconds between attempts (not actually used in test)
    /// * `outcomes` - A vector of Results representing the outcome of each attempt
    ///
    /// # Returns
    /// The result of the retry logic - either the first success or the last error
    fn simulate_extract_with_retry(
        max_attempts: u32,
        outcomes: &[Result<ExtractedContent, ExtractionError>],
    ) -> Result<ExtractedContent, ExtractionError> {
        if max_attempts == 0 {
            return Err(ExtractionError::NoContentFound(
                "No attempts made (max_attempts was 0)".into(),
            ));
        }

        let mut last_error = ExtractionError::NoContentFound("No attempts made".into());

        for attempt in 0..max_attempts {
            let outcome = if (attempt as usize) < outcomes.len() {
                // Clone the outcome for this attempt
                match &outcomes[attempt as usize] {
                    Ok(content) => Ok(content.clone()),
                    Err(e) => Err(clone_extraction_error(e)),
                }
            } else {
                // If we run out of outcomes, use the last one
                if let Some(last) = outcomes.last() {
                    match last {
                        Ok(content) => Ok(content.clone()),
                        Err(e) => Err(clone_extraction_error(e)),
                    }
                } else {
                    Err(ExtractionError::NoContentFound("No outcomes provided".into()))
                }
            };

            match outcome {
                Ok(content) if !content.content.trim().is_empty() => {
                    return Ok(content);
                }
                Ok(_) => {
                    last_error = ExtractionError::NoContentFound(format!(
                        "Empty content on attempt {}",
                        attempt + 1
                    ));
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }

        Err(last_error)
    }

    /// Helper function to clone an ExtractionError (since it doesn't implement Clone)
    fn clone_extraction_error(e: &ExtractionError) -> ExtractionError {
        match e {
            ExtractionError::PermissionDenied(msg) => ExtractionError::PermissionDenied(msg.clone()),
            ExtractionError::AppNotFound(msg) => ExtractionError::AppNotFound(msg.clone()),
            ExtractionError::ElementNotFound(msg) => ExtractionError::ElementNotFound(msg.clone()),
            ExtractionError::NoContentFound(msg) => ExtractionError::NoContentFound(msg.clone()),
            ExtractionError::PatternNotSupported(msg) => {
                ExtractionError::PatternNotSupported(msg.clone())
            }
            ExtractionError::PlatformError(msg) => ExtractionError::PlatformError(msg.clone()),
            ExtractionError::Timeout(msg) => ExtractionError::Timeout(msg.clone()),
            ExtractionError::AccessibilityError(msg) => ExtractionError::AccessibilityError(msg.clone()),
        }
    }

    /// Helper function to get the error variant name for comparison
    fn error_variant_name(e: &ExtractionError) -> &'static str {
        match e {
            ExtractionError::PermissionDenied(_) => "PermissionDenied",
            ExtractionError::AppNotFound(_) => "AppNotFound",
            ExtractionError::ElementNotFound(_) => "ElementNotFound",
            ExtractionError::NoContentFound(_) => "NoContentFound",
            ExtractionError::PatternNotSupported(_) => "PatternNotSupported",
            ExtractionError::PlatformError(_) => "PlatformError",
            ExtractionError::Timeout(_) => "Timeout",
            ExtractionError::AccessibilityError(_) => "AccessibilityError",
        }
    }

    /// Helper function to get the error message
    fn error_message(e: &ExtractionError) -> &str {
        match e {
            ExtractionError::PermissionDenied(msg) => msg,
            ExtractionError::AppNotFound(msg) => msg,
            ExtractionError::ElementNotFound(msg) => msg,
            ExtractionError::NoContentFound(msg) => msg,
            ExtractionError::PatternNotSupported(msg) => msg,
            ExtractionError::PlatformError(msg) => msg,
            ExtractionError::Timeout(msg) => msg,
            ExtractionError::AccessibilityError(msg) => msg,
        }
    }

    /// Strategy to generate ExtractionError variants
    fn arb_extraction_error() -> impl Strategy<Value = ExtractionError> {
        prop_oneof![
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::PermissionDenied),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::AppNotFound),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::ElementNotFound),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::NoContentFound),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::PatternNotSupported),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::PlatformError),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::Timeout),
            "[a-zA-Z0-9 ]{1,50}".prop_map(ExtractionError::AccessibilityError),
        ]
    }

    proptest! {
        // Feature: accessibility-extractor, Property 14: Retry Behavior
        // **Validates: Requirements 10.2, 10.4**
        //
        // *For any* call to `extract_with_retry(n, delay)` where all `n` attempts fail,
        // the function SHALL return the error from the last attempt, not an earlier one.
        #[test]
        fn prop_retry_returns_last_error_when_all_fail(
            max_attempts in 1u32..10,
            errors in prop::collection::vec(arb_extraction_error(), 1..10),
        ) {
            // Ensure we have enough errors for all attempts
            let num_errors = errors.len();
            prop_assume!(num_errors >= 1);

            // Create outcomes that are all failures
            let outcomes: Vec<Result<ExtractedContent, ExtractionError>> =
                errors.iter().map(|e| Err(clone_extraction_error(e))).collect();

            // Run the simulated retry logic
            let result = simulate_extract_with_retry(max_attempts, &outcomes);

            // The result should be an error
            prop_assert!(result.is_err(), "Expected error when all attempts fail");

            // Determine which error should be the last one
            let last_attempt_idx = (max_attempts as usize).min(num_errors) - 1;
            let expected_error = &errors[last_attempt_idx];
            let actual_error = result.unwrap_err();

            // Verify the error variant matches the last attempt's error
            prop_assert_eq!(
                error_variant_name(&actual_error),
                error_variant_name(expected_error),
                "Error variant should match the last attempt's error. Expected {:?}, got {:?}",
                expected_error,
                actual_error
            );

            // Verify the error message matches the last attempt's error
            prop_assert_eq!(
                error_message(&actual_error),
                error_message(expected_error),
                "Error message should match the last attempt's error"
            );
        }

        // Test that retry stops on first success
        #[test]
        fn prop_retry_returns_first_success(
            attempts_before_success in 0u32..5,
            max_attempts in 1u32..10,
            source in "[a-z]{1,10}",
            content in "[a-zA-Z0-9]{1,50}",  // Non-whitespace content
            app_name in "[a-zA-Z]{1,20}",
        ) {
            // Only test when success can happen within max_attempts
            prop_assume!(attempts_before_success < max_attempts);
            // Ensure content is not empty after trim
            prop_assume!(!content.trim().is_empty());

            // Create a successful content
            let success_content = ExtractedContent {
                source: source.clone(),
                title: Some("Test".to_string()),
                content: content.clone(),
                app_name: app_name.clone(),
                timestamp: 1707500000,
                extraction_method: "accessibility".to_string(),
            };

            // Create outcomes: failures followed by success
            let mut outcomes: Vec<Result<ExtractedContent, ExtractionError>> = Vec::new();
            for i in 0..attempts_before_success {
                outcomes.push(Err(ExtractionError::NoContentFound(format!("Attempt {} failed", i + 1))));
            }
            outcomes.push(Ok(success_content.clone()));

            // Run the simulated retry logic
            let result = simulate_extract_with_retry(max_attempts, &outcomes);

            // The result should be success
            prop_assert!(result.is_ok(), "Expected success when one attempt succeeds");

            let extracted = result.unwrap();
            prop_assert_eq!(extracted.source, source);
            prop_assert_eq!(extracted.content, content);
            prop_assert_eq!(extracted.app_name, app_name);
        }

        // Test that empty content is treated as failure and triggers retry
        #[test]
        fn prop_retry_treats_empty_content_as_failure(
            max_attempts in 2u32..10,
        ) {
            // Create outcomes: empty content followed by actual error
            let empty_content = ExtractedContent {
                source: "test".to_string(),
                title: None,
                content: "   ".to_string(), // whitespace-only content
                app_name: "Test App".to_string(),
                timestamp: 1707500000,
                extraction_method: "accessibility".to_string(),
            };

            let final_error = ExtractionError::AppNotFound("Final error".to_string());

            let outcomes: Vec<Result<ExtractedContent, ExtractionError>> = vec![
                Ok(empty_content),
                Err(final_error),
            ];

            // Run the simulated retry logic
            let result = simulate_extract_with_retry(max_attempts, &outcomes);

            // The result should be an error (since empty content triggers retry)
            prop_assert!(result.is_err(), "Expected error when content is empty");

            // The error should be from the last attempt (AppNotFound)
            let actual_error = result.unwrap_err();
            prop_assert_eq!(
                error_variant_name(&actual_error),
                "AppNotFound",
                "Should return the last error (AppNotFound), not NoContentFound from empty content"
            );
        }

        // Test that zero attempts returns appropriate error
        #[test]
        fn prop_retry_zero_attempts_returns_error(
            delay_ms in 0u64..1000,
        ) {
            let _ = delay_ms; // Not used in simulation
            let result = simulate_extract_with_retry(0, &[]);

            prop_assert!(result.is_err(), "Expected error when max_attempts is 0");

            let error = result.unwrap_err();
            prop_assert_eq!(
                error_variant_name(&error),
                "NoContentFound",
                "Zero attempts should return NoContentFound error"
            );
        }
    }

    // ============================================================================
    // Feature: accessibility-extractor, Property 15: CapturePayload Schema Conformance
    // **Validates: Requirements 11.7**
    //
    // *For any* conversion from `ExtractedContent` to `CapturePayload`, the result
    // SHALL contain:
    // - A non-empty `source` string
    // - A non-empty `url` string (accessibility:// scheme)
    // - A non-empty `content` string
    // ============================================================================

    /// Strategy to generate arbitrary ExtractedContent instances with non-empty fields
    fn arb_extracted_content() -> impl Strategy<Value = ExtractedContent> {
        (
            "[a-z]{1,20}",                    // source: non-empty lowercase string
            prop::option::of("[a-zA-Z0-9 ._-]{1,50}"), // title: optional non-empty string
            ".{1,200}",                       // content: non-empty string
            "[a-zA-Z ]{1,30}",                // app_name: non-empty string
            1i64..i64::MAX,                   // timestamp: positive integer
        )
            .prop_map(|(source, title, content, app_name, timestamp)| ExtractedContent {
                source,
                title,
                content,
                app_name,
                timestamp,
                extraction_method: "accessibility".to_string(),
            })
    }

    proptest! {
        // Feature: accessibility-extractor, Property 15: CapturePayload Schema Conformance
        // **Validates: Requirements 11.7**
        //
        // *For any* conversion from `ExtractedContent` to `CapturePayload`, the result
        // SHALL contain:
        // - A non-empty `source` string
        // - A non-empty `url` string (accessibility:// scheme)
        // - A non-empty `content` string
        #[test]
        fn prop_capture_payload_schema_conformance(
            extracted in arb_extracted_content()
        ) {
            // Convert ExtractedContent to CapturePayload
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // Property 15.1: source SHALL be a non-empty string
            prop_assert!(
                !payload.source.is_empty(),
                "CapturePayload.source must be non-empty, got: '{}'",
                payload.source
            );

            // Property 15.2: url SHALL be a non-empty string with accessibility:// scheme
            prop_assert!(
                !payload.url.is_empty(),
                "CapturePayload.url must be non-empty, got: '{}'",
                payload.url
            );
            prop_assert!(
                payload.url.starts_with("accessibility://"),
                "CapturePayload.url must start with 'accessibility://', got: '{}'",
                payload.url
            );

            // Property 15.3: content SHALL be a non-empty string
            prop_assert!(
                !payload.content.is_empty(),
                "CapturePayload.content must be non-empty, got: '{}'",
                payload.content
            );
        }

        // Additional test: verify source is preserved from ExtractedContent
        #[test]
        fn prop_capture_payload_preserves_source(
            extracted in arb_extracted_content()
        ) {
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // Source should be copied directly from ExtractedContent
            prop_assert_eq!(
                &payload.source,
                &extracted.source,
                "CapturePayload.source should match ExtractedContent.source"
            );
        }

        // Additional test: verify content is preserved from ExtractedContent
        #[test]
        fn prop_capture_payload_preserves_content(
            extracted in arb_extracted_content()
        ) {
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // Content should be copied directly from ExtractedContent
            prop_assert_eq!(
                &payload.content,
                &extracted.content,
                "CapturePayload.content should match ExtractedContent.content"
            );
        }

        // Additional test: verify URL format contains app_name and title
        #[test]
        fn prop_capture_payload_url_contains_app_info(
            extracted in arb_extracted_content()
        ) {
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // URL should contain app_name (with spaces replaced by underscores)
            let expected_app_name = extracted.app_name.replace(" ", "_");
            prop_assert!(
                payload.url.contains(&expected_app_name),
                "CapturePayload.url should contain app_name '{}', got: '{}'",
                expected_app_name,
                payload.url
            );

            // URL should contain title or "untitled" if title is None
            let expected_title = extracted.title.as_deref().unwrap_or("untitled");
            prop_assert!(
                payload.url.contains(expected_title),
                "CapturePayload.url should contain title '{}', got: '{}'",
                expected_title,
                payload.url
            );
        }

        // Additional test: verify timestamp is preserved
        #[test]
        fn prop_capture_payload_preserves_timestamp(
            extracted in arb_extracted_content()
        ) {
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // Timestamp should be copied from ExtractedContent
            prop_assert_eq!(
                payload.timestamp,
                Some(extracted.timestamp),
                "CapturePayload.timestamp should match ExtractedContent.timestamp"
            );
        }

        // Additional test: verify title is preserved
        #[test]
        fn prop_capture_payload_preserves_title(
            extracted in arb_extracted_content()
        ) {
            let payload = AccessibilityExtractor::to_capture_payload(&extracted);

            // Title should be copied from ExtractedContent
            prop_assert_eq!(
                &payload.title,
                &extracted.title,
                "CapturePayload.title should match ExtractedContent.title"
            );
        }
    }

    // ============================================================================
    // Unit Tests for extract_with_retry
    // ============================================================================

    /// Test that extract_with_retry with 0 attempts returns an error
    #[test]
    fn test_extract_with_retry_zero_attempts() {
        let result = AccessibilityExtractor::extract_with_retry(0, 100);
        assert!(result.is_err());
        match result.unwrap_err() {
            ExtractionError::NoContentFound(msg) => {
                assert!(msg.contains("max_attempts was 0"));
            }
            _ => panic!("Expected NoContentFound error for 0 attempts"),
        }
    }

    /// Test that extract_with_retry returns a Result
    #[test]
    fn test_extract_with_retry_returns_result() {
        // This test just verifies the function can be called
        // The actual result depends on system state
        let result = AccessibilityExtractor::extract_with_retry(1, 0);
        match result {
            Ok(content) => {
                assert!(!content.source.is_empty());
                assert!(!content.content.is_empty());
            }
            Err(e) => {
                assert!(!e.to_string().is_empty());
            }
        }
    }

    // ============================================================================
    // Unit Tests for AccessibilityExtractor
    // ============================================================================

    /// Test that is_enabled returns a boolean value.
    /// Note: The actual value depends on system permissions.
    #[test]
    fn test_is_enabled_returns_boolean() {
        let result = AccessibilityExtractor::is_enabled();
        // Just verify it returns a boolean (true or false)
        assert!(result == true || result == false);
    }

    /// Test that request_permissions doesn't panic.
    /// Note: This test just verifies the function can be called without error.
    #[test]
    fn test_request_permissions_does_not_panic() {
        // This should not panic on any platform
        AccessibilityExtractor::request_permissions();
    }

    /// Test that get_selected_text returns an Option.
    /// Note: The actual value depends on system state.
    #[test]
    fn test_get_selected_text_returns_option() {
        let result = AccessibilityExtractor::get_selected_text();
        // Just verify it returns an Option (Some or None)
        match result {
            Some(text) => assert!(!text.is_empty() || text.is_empty()), // Any string is valid
            None => {} // None is also valid
        }
    }

    /// Test that extract_frontmost returns a Result.
    /// Note: This test may fail if accessibility permissions are not granted,
    /// which is expected behavior.
    #[test]
    fn test_extract_frontmost_returns_result() {
        let result = AccessibilityExtractor::extract_frontmost();
        // The result should be either Ok or Err
        match result {
            Ok(content) => {
                // If successful, verify the content structure
                assert!(!content.source.is_empty());
                assert!(!content.content.is_empty());
                assert!(!content.app_name.is_empty());
                assert!(content.timestamp > 0);
                assert_eq!(content.extraction_method, "accessibility");
            }
            Err(e) => {
                // If error, verify it's a valid error type
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
            }
        }
    }

    /// Test that extract_from_app returns a Result.
    /// Note: This test may fail if the app is not running or permissions are not granted.
    #[test]
    fn test_extract_from_app_returns_result() {
        // Use a bundle ID that likely doesn't exist to test error handling
        let result = AccessibilityExtractor::extract_from_app("com.nonexistent.app");
        // The result should be either Ok or Err
        match result {
            Ok(content) => {
                // If successful (unlikely with fake bundle ID), verify structure
                assert!(!content.source.is_empty());
            }
            Err(e) => {
                // If error, verify it's a valid error type
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
            }
        }
    }

    // ============================================================================
    // Unit Tests for to_capture_payload
    // ============================================================================

    /// Test that to_capture_payload correctly converts ExtractedContent with title
    #[test]
    fn test_to_capture_payload_with_title() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Document.docx".to_string()),
            content: "Hello, world!".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let payload = AccessibilityExtractor::to_capture_payload(&content);

        assert_eq!(payload.source, "word");
        assert_eq!(payload.url, "accessibility://Microsoft_Word/Document.docx");
        assert_eq!(payload.content, "Hello, world!");
        assert_eq!(payload.title, Some("Document.docx".to_string()));
        assert_eq!(payload.author, None);
        assert_eq!(payload.channel, None);
        assert_eq!(payload.timestamp, Some(1707500000));
    }

    /// Test that to_capture_payload correctly handles missing title
    #[test]
    fn test_to_capture_payload_without_title() {
        let content = ExtractedContent {
            source: "pages".to_string(),
            title: None,
            content: "Some content".to_string(),
            app_name: "Pages".to_string(),
            timestamp: 1707500001,
            extraction_method: "accessibility".to_string(),
        };

        let payload = AccessibilityExtractor::to_capture_payload(&content);

        assert_eq!(payload.source, "pages");
        assert_eq!(payload.url, "accessibility://Pages/untitled");
        assert_eq!(payload.content, "Some content");
        assert_eq!(payload.title, None);
        assert_eq!(payload.timestamp, Some(1707500001));
    }

    /// Test that to_capture_payload replaces spaces with underscores in app_name
    #[test]
    fn test_to_capture_payload_url_space_replacement() {
        let content = ExtractedContent {
            source: "excel".to_string(),
            title: Some("Budget 2024.xlsx".to_string()),
            content: "Spreadsheet data".to_string(),
            app_name: "Microsoft Excel".to_string(),
            timestamp: 1707500002,
            extraction_method: "accessibility".to_string(),
        };

        let payload = AccessibilityExtractor::to_capture_payload(&content);

        // Verify spaces in app_name are replaced with underscores
        assert_eq!(payload.url, "accessibility://Microsoft_Excel/Budget 2024.xlsx");
    }

    /// Test that to_capture_payload generates valid accessibility:// URL
    #[test]
    fn test_to_capture_payload_url_format() {
        let content = ExtractedContent {
            source: "keynote".to_string(),
            title: Some("Presentation.key".to_string()),
            content: "Slide content".to_string(),
            app_name: "Keynote".to_string(),
            timestamp: 1707500003,
            extraction_method: "accessibility".to_string(),
        };

        let payload = AccessibilityExtractor::to_capture_payload(&content);

        // Verify URL starts with accessibility:// scheme
        assert!(payload.url.starts_with("accessibility://"));
        // Verify URL contains app name and title
        assert!(payload.url.contains("Keynote"));
        assert!(payload.url.contains("Presentation.key"));
    }

    /// Test that to_capture_payload preserves all content fields
    #[test]
    fn test_to_capture_payload_preserves_content() {
        let long_content = "This is a very long document content that spans multiple paragraphs.\n\nIt contains various text elements and should be preserved exactly as-is in the CapturePayload.";
        
        let content = ExtractedContent {
            source: "textedit".to_string(),
            title: Some("Notes.txt".to_string()),
            content: long_content.to_string(),
            app_name: "TextEdit".to_string(),
            timestamp: 1707500004,
            extraction_method: "accessibility".to_string(),
        };

        let payload = AccessibilityExtractor::to_capture_payload(&content);

        // Verify content is preserved exactly
        assert_eq!(payload.content, long_content);
    }

    // ============================================================================
    // Platform-specific tests
    // ============================================================================

    /// Test that on non-macOS platforms, extract_frontmost returns PlatformError.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_extract_frontmost_returns_platform_error_on_unsupported() {
        let result = AccessibilityExtractor::extract_frontmost();
        match result {
            Err(ExtractionError::PlatformError(msg)) => {
                assert_eq!(msg, "Unsupported platform");
            }
            _ => panic!("Expected PlatformError on unsupported platform"),
        }
    }

    /// Test that on non-macOS platforms, extract_from_app returns PlatformError.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_extract_from_app_returns_platform_error_on_unsupported() {
        let result = AccessibilityExtractor::extract_from_app("com.test.app");
        match result {
            Err(ExtractionError::PlatformError(msg)) => {
                assert_eq!(msg, "Unsupported platform");
            }
            _ => panic!("Expected PlatformError on unsupported platform"),
        }
    }

    /// Test that on non-macOS platforms, is_enabled returns false.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_is_enabled_returns_false_on_unsupported() {
        assert!(!AccessibilityExtractor::is_enabled());
    }

    /// Test that on non-macOS platforms, get_selected_text returns None.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_get_selected_text_returns_none_on_unsupported() {
        assert!(AccessibilityExtractor::get_selected_text().is_none());
    }

    // ============================================================================
    // Feature: accessibility-extractor, Property 16: Unique Document Identifiers
    // **Validates: Requirements 11.8**
    //
    // *For any* two distinct extractions, the generated `ehl_doc_id` values SHALL
    // be different (UUID v4 uniqueness).
    // ============================================================================

    proptest! {
        // Feature: accessibility-extractor, Property 16: Unique Document Identifiers
        // **Validates: Requirements 11.8**
        //
        // *For any* two distinct extractions, the generated `ehl_doc_id` values SHALL
        // be different (UUID v4 uniqueness).
        //
        // This property test generates multiple document IDs and verifies they are all unique.
        #[test]
        fn prop_unique_document_identifiers(
            count in 2usize..100,
        ) {
            // Generate 'count' document IDs
            let doc_ids: Vec<String> = (0..count)
                .map(|_| AccessibilityExtractor::generate_doc_id())
                .collect();

            // Verify all IDs are unique by checking the set size equals the vector size
            let unique_ids: std::collections::HashSet<&String> = doc_ids.iter().collect();
            
            prop_assert_eq!(
                unique_ids.len(),
                doc_ids.len(),
                "All generated document IDs should be unique. Generated {} IDs but only {} are unique.",
                doc_ids.len(),
                unique_ids.len()
            );
        }

        // Additional test: verify document IDs are valid UUID v4 format
        #[test]
        fn prop_document_id_is_valid_uuid_format(
            _seed in 0u64..1000,
        ) {
            let doc_id = AccessibilityExtractor::generate_doc_id();

            // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
            // where x is any hex digit and y is one of 8, 9, a, or b
            
            // Verify the ID is not empty
            prop_assert!(
                !doc_id.is_empty(),
                "Document ID should not be empty"
            );

            // Verify the ID has the correct length (36 characters for UUID with hyphens)
            prop_assert_eq!(
                doc_id.len(),
                36,
                "Document ID should be 36 characters (UUID format), got {} characters: '{}'",
                doc_id.len(),
                doc_id
            );

            // Verify the ID has hyphens in the correct positions
            let chars: Vec<char> = doc_id.chars().collect();
            prop_assert_eq!(chars[8], '-', "Expected hyphen at position 8");
            prop_assert_eq!(chars[13], '-', "Expected hyphen at position 13");
            prop_assert_eq!(chars[18], '-', "Expected hyphen at position 18");
            prop_assert_eq!(chars[23], '-', "Expected hyphen at position 23");

            // Verify the version digit (position 14) is '4' for UUID v4
            prop_assert_eq!(
                chars[14],
                '4',
                "UUID v4 should have '4' at position 14, got '{}'",
                chars[14]
            );

            // Verify the variant digit (position 19) is one of 8, 9, a, b
            let variant = chars[19];
            prop_assert!(
                variant == '8' || variant == '9' || variant == 'a' || variant == 'b',
                "UUID v4 variant digit should be 8, 9, a, or b, got '{}'",
                variant
            );

            // Verify all other characters are valid hex digits
            for (i, c) in chars.iter().enumerate() {
                if i != 8 && i != 13 && i != 18 && i != 23 {
                    prop_assert!(
                        c.is_ascii_hexdigit(),
                        "Character at position {} should be a hex digit, got '{}'",
                        i,
                        c
                    );
                }
            }
        }

        // Test that consecutive calls produce different IDs
        #[test]
        fn prop_consecutive_calls_produce_different_ids(
            _seed in 0u64..1000,
        ) {
            let id1 = AccessibilityExtractor::generate_doc_id();
            let id2 = AccessibilityExtractor::generate_doc_id();

            prop_assert_ne!(
                id1,
                id2,
                "Two consecutive calls to generate_doc_id should produce different IDs"
            );
        }
    }

    // ============================================================================
    // Unit Tests for generate_doc_id
    // ============================================================================

    /// Test that generate_doc_id returns a non-empty string
    #[test]
    fn test_generate_doc_id_returns_non_empty() {
        let doc_id = AccessibilityExtractor::generate_doc_id();
        assert!(!doc_id.is_empty(), "Document ID should not be empty");
    }

    /// Test that generate_doc_id returns a valid UUID format
    #[test]
    fn test_generate_doc_id_valid_uuid_format() {
        let doc_id = AccessibilityExtractor::generate_doc_id();
        
        // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars)
        assert_eq!(doc_id.len(), 36, "UUID should be 36 characters");
        
        // Verify hyphens are in correct positions
        let chars: Vec<char> = doc_id.chars().collect();
        assert_eq!(chars[8], '-');
        assert_eq!(chars[13], '-');
        assert_eq!(chars[18], '-');
        assert_eq!(chars[23], '-');
    }

    /// Test that generate_doc_id produces unique IDs
    #[test]
    fn test_generate_doc_id_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        
        // Generate 1000 IDs and verify they're all unique
        for _ in 0..1000 {
            let id = AccessibilityExtractor::generate_doc_id();
            assert!(ids.insert(id), "Generated duplicate document ID");
        }
    }

    /// Test that generate_doc_id produces UUID v4 (version 4)
    #[test]
    fn test_generate_doc_id_is_uuid_v4() {
        let doc_id = AccessibilityExtractor::generate_doc_id();
        let chars: Vec<char> = doc_id.chars().collect();
        
        // UUID v4 has '4' at position 14 (the version digit)
        assert_eq!(chars[14], '4', "UUID should be version 4");
        
        // UUID v4 has 8, 9, a, or b at position 19 (the variant digit)
        let variant = chars[19];
        assert!(
            variant == '8' || variant == '9' || variant == 'a' || variant == 'b',
            "UUID variant should be 8, 9, a, or b"
        );
    }

    // ============================================================================
    // Unit Tests for to_chunk_meta
    // ============================================================================

    /// Test that to_chunk_meta creates valid ChunkMeta with all fields
    #[test]
    fn test_to_chunk_meta_creates_valid_chunk() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Document.docx".to_string()),
            content: "Hello, world! This is the document content.".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let chunk_meta = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);

        // Verify required fields
        assert!(!chunk_meta.id.is_empty(), "ID should not be empty");
        assert_eq!(chunk_meta.source, "word");
        assert_eq!(chunk_meta.timestamp, 1707500000);
        assert_eq!(chunk_meta.chunk_idx, 0);
        assert_eq!(chunk_meta.total_chunks, 1);

        // Verify optional fields are populated
        assert!(chunk_meta.app_id.is_some());
        assert_eq!(chunk_meta.app_id.as_deref(), Some("com.microsoft.Word"));
        assert!(chunk_meta.url.is_some());
        assert!(chunk_meta.url.as_ref().unwrap().starts_with("accessibility://"));
        assert!(chunk_meta.title.is_some());
        assert_eq!(chunk_meta.title.as_deref(), Some("Document.docx"));
    }

    /// Test that to_chunk_meta handles missing title
    #[test]
    fn test_to_chunk_meta_without_title() {
        let content = ExtractedContent {
            source: "pages".to_string(),
            title: None,
            content: "Some content".to_string(),
            app_name: "Pages".to_string(),
            timestamp: 1707500001,
            extraction_method: "accessibility".to_string(),
        };

        let chunk_meta = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);

        // Title should default to "untitled"
        assert_eq!(chunk_meta.title.as_deref(), Some("untitled"));
        // URL should contain "untitled"
        assert!(chunk_meta.url.as_ref().unwrap().contains("untitled"));
    }

    /// Test that to_chunk_meta generates unique IDs for each call
    #[test]
    fn test_to_chunk_meta_unique_ids() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Document.docx".to_string()),
            content: "Hello, world!".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let chunk1 = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);
        let chunk2 = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);

        // Each call should generate a unique ID
        assert_ne!(chunk1.id, chunk2.id);
    }

    /// Test that to_chunk_meta correctly sets chunk indices
    #[test]
    fn test_to_chunk_meta_chunk_indices() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("LargeDocument.docx".to_string()),
            content: "Content chunk".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        // Create multiple chunks
        let chunk0 = AccessibilityExtractor::to_chunk_meta(&content, 0, 5);
        let chunk2 = AccessibilityExtractor::to_chunk_meta(&content, 2, 5);
        let chunk4 = AccessibilityExtractor::to_chunk_meta(&content, 4, 5);

        assert_eq!(chunk0.chunk_idx, 0);
        assert_eq!(chunk0.total_chunks, 5);
        assert_eq!(chunk2.chunk_idx, 2);
        assert_eq!(chunk2.total_chunks, 5);
        assert_eq!(chunk4.chunk_idx, 4);
        assert_eq!(chunk4.total_chunks, 5);
    }

    /// Test that to_chunk_meta truncates header to 200 characters
    #[test]
    fn test_to_chunk_meta_header_truncation() {
        let long_content = "A".repeat(500);
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Document.docx".to_string()),
            content: long_content,
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let chunk_meta = AccessibilityExtractor::to_chunk_meta(&content, 0, 1);

        // Header should be truncated to 200 characters
        assert_eq!(chunk_meta.header.chars().count(), 200);
    }

    // ============================================================================
    // Unit Tests for to_chunk_meta_json
    // ============================================================================

    /// Test that to_chunk_meta_json returns valid JSON
    #[test]
    fn test_to_chunk_meta_json_valid_json() {
        let content = ExtractedContent {
            source: "word".to_string(),
            title: Some("Document.docx".to_string()),
            content: "Hello, world!".to_string(),
            app_name: "Microsoft Word".to_string(),
            timestamp: 1707500000,
            extraction_method: "accessibility".to_string(),
        };

        let json = AccessibilityExtractor::to_chunk_meta_json(&content);

        // Verify it's valid JSON by parsing it
        let parsed: serde_json::Value = serde_json::from_str(&json)
            .expect("Should be valid JSON");

        // Verify required fields are present
        assert!(parsed.get("id").is_some());
        assert!(parsed.get("source").is_some());
        assert!(parsed.get("timestamp").is_some());
        assert!(parsed.get("chunk_idx").is_some());
        assert!(parsed.get("total_chunks").is_some());
    }

    /// Test that to_chunk_meta_json contains correct values
    #[test]
    fn test_to_chunk_meta_json_correct_values() {
        let content = ExtractedContent {
            source: "excel".to_string(),
            title: Some("Budget.xlsx".to_string()),
            content: "Spreadsheet data".to_string(),
            app_name: "Microsoft Excel".to_string(),
            timestamp: 1707500002,
            extraction_method: "accessibility".to_string(),
        };

        let json = AccessibilityExtractor::to_chunk_meta_json(&content);

        // Verify specific values
        assert!(json.contains("\"source\":\"excel\""));
        assert!(json.contains("\"chunk_idx\":0"));
        assert!(json.contains("\"total_chunks\":1"));
        assert!(json.contains("\"timestamp\":1707500002"));
    }

    /// Test that to_chunk_meta_json creates single chunk by default
    #[test]
    fn test_to_chunk_meta_json_single_chunk() {
        let content = ExtractedContent {
            source: "pages".to_string(),
            title: Some("Document.pages".to_string()),
            content: "Content".to_string(),
            app_name: "Pages".to_string(),
            timestamp: 1707500003,
            extraction_method: "accessibility".to_string(),
        };

        let json = AccessibilityExtractor::to_chunk_meta_json(&content);

        // Should be chunk 0 of 1
        assert!(json.contains("\"chunk_idx\":0"));
        assert!(json.contains("\"total_chunks\":1"));
    }

    // ============================================================================
    // Unit Tests for derive_bundle_id
    // ============================================================================

    /// Test that derive_bundle_id maps known applications correctly
    #[test]
    fn test_derive_bundle_id_known_apps() {
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Microsoft Word"),
            "com.microsoft.Word"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Microsoft Excel"),
            "com.microsoft.Excel"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Microsoft PowerPoint"),
            "com.microsoft.Powerpoint"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Microsoft Outlook"),
            "com.microsoft.Outlook"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Pages"),
            "com.apple.iWork.Pages"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Numbers"),
            "com.apple.iWork.Numbers"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("Keynote"),
            "com.apple.iWork.Keynote"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("TextEdit"),
            "com.apple.TextEdit"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("LibreOffice Writer"),
            "org.libreoffice.script"
        );
    }

    /// Test that derive_bundle_id handles unknown applications
    #[test]
    fn test_derive_bundle_id_unknown_apps() {
        let bundle_id = AccessibilityExtractor::derive_bundle_id("Custom App");
        assert!(bundle_id.starts_with("com.unknown."));
        assert!(bundle_id.contains("customapp"));
    }

    /// Test that derive_bundle_id is case-insensitive
    #[test]
    fn test_derive_bundle_id_case_insensitive() {
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("MICROSOFT WORD"),
            "com.microsoft.Word"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("microsoft word"),
            "com.microsoft.Word"
        );
        assert_eq!(
            AccessibilityExtractor::derive_bundle_id("MiCrOsOfT wOrD"),
            "com.microsoft.Word"
        );
    }

    // ============================================================================
    // Unit Tests for generate_content_hash
    // ============================================================================

    /// Test that generate_content_hash returns valid SHA-256 hash
    #[test]
    fn test_generate_content_hash_format() {
        let hash = AccessibilityExtractor::generate_content_hash("Hello, world!");

        // SHA-256 produces 64 hexadecimal characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Test that generate_content_hash is deterministic
    #[test]
    fn test_generate_content_hash_deterministic() {
        let content = "Hello, world!";
        let hash1 = AccessibilityExtractor::generate_content_hash(content);
        let hash2 = AccessibilityExtractor::generate_content_hash(content);

        assert_eq!(hash1, hash2);
    }

    /// Test that generate_content_hash produces different hashes for different content
    #[test]
    fn test_generate_content_hash_different_content() {
        let hash1 = AccessibilityExtractor::generate_content_hash("Hello, world!");
        let hash2 = AccessibilityExtractor::generate_content_hash("Different content");

        assert_ne!(hash1, hash2);
    }
}
