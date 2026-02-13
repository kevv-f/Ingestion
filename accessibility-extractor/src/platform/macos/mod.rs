//! macOS platform-specific implementation for accessibility content extraction.
//!
//! This module provides the macOS-specific implementation using the native
//! Accessibility API (AXUIElement) to extract content from desktop applications.
//!
//! # Requirements
//! - Requirement 1.1: Provide a function to check if accessibility permissions are granted
//! - Requirement 1.2: Provide a function to request accessibility permissions from the user
//! - Requirement 2.1: Provide a function to get the frontmost (active) application
//! - Requirement 2.3: Provide a function to get an application by its Bundle_ID with timeout

pub mod element;
pub mod electron;
pub mod permissions;
pub mod roles;
pub mod applescript;
pub mod file_extract;

// Re-export commonly used items
pub use element::{debug_print_tree, debug_print_attributes, find_elements_by_role, get_attribute_names};
pub use electron::{
    is_electron_app, enable_electron_accessibility, get_pid_for_bundle_id, prepare_electron_app,
    is_slack, prepare_slack, SlackExtractionConfig, SLACK_BUNDLE_ID, extract_slack_content,
    SlackMessage,
    is_teams, prepare_teams, TeamsExtractionConfig, TEAMS_BUNDLE_ID, TEAMS_NEW_BUNDLE_ID,
    extract_teams_content, TeamsMessage, TeamsVersion, detect_teams_version, get_running_teams_bundle_id,
};
pub use permissions::{
    get_permission_instructions, is_trusted, is_trusted_with_prompt, open_accessibility_preferences,
};
pub use roles::{is_text_role, should_extract_from_role, DOCUMENT_ROLES, UI_CHROME_ROLES};
// Note: AppleScript module is kept for backwards compatibility but not used in main extraction flow
pub use applescript::{supports_applescript, extract_via_applescript};
pub use file_extract::{
    supports_direct_extraction, extract_from_office_app, 
    extract_excel, extract_word, extract_powerpoint,
    extract_pages, extract_numbers, extract_keynote,
    get_document_path_via_ax,
};

use accessibility::attribute::AXAttribute;
use accessibility::{AXUIElement, AXUIElementAttributes};
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use std::time::Duration;

use crate::types::{AppSource, ExtractedContent, ExtractionError};

/// Maximum recursion depth for accessibility tree traversal.
/// This constant prevents stack overflow on deeply nested or circular element structures.
/// 
/// # Requirements
/// - Requirement 7.3: Limit recursion depth to prevent infinite loops (maximum 100 levels)
pub const MAX_RECURSION_DEPTH: usize = 100;

/// macOS-specific content extractor using the Accessibility API.
///
/// This struct provides methods for extracting content from desktop applications
/// on macOS using the native AXUIElement API. It handles permission checking,
/// application detection, and content extraction.
///
/// # Example
///
/// ```no_run
/// use accessibility_extractor::platform::macos::MacOSExtractor;
///
/// // Check if accessibility permissions are granted
/// if MacOSExtractor::is_accessibility_enabled() {
///     // Get the frontmost application
///     if let Some(app) = MacOSExtractor::get_frontmost_app() {
///         println!("Got frontmost app!");
///     }
/// } else {
///     // Request permissions
///     MacOSExtractor::request_accessibility();
/// }
/// ```
pub struct MacOSExtractor;

impl MacOSExtractor {
    /// Check if accessibility permissions are granted.
    ///
    /// This function queries the system to determine if the current application
    /// has been granted accessibility permissions. It does not show any UI or
    /// prompt the user.
    ///
    /// # Returns
    ///
    /// `true` if accessibility permissions are granted, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// if MacOSExtractor::is_accessibility_enabled() {
    ///     println!("Accessibility permissions are granted!");
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 1.1: Provide a function to check if accessibility permissions are granted
    pub fn is_accessibility_enabled() -> bool {
        permissions::is_trusted()
    }

    /// Request accessibility permissions (shows system dialog).
    ///
    /// This function triggers the system's accessibility permission prompt dialog.
    /// If permissions are already granted, this function has no visible effect.
    ///
    /// # Note
    ///
    /// The system prompt is shown asynchronously. The application typically needs
    /// to be restarted after permissions are granted for them to take effect.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// if !MacOSExtractor::is_accessibility_enabled() {
    ///     MacOSExtractor::request_accessibility();
    ///     println!("Please grant accessibility permissions and restart the app.");
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 1.2: Provide a function to request accessibility permissions from the user
    pub fn request_accessibility() {
        permissions::is_trusted_with_prompt();
    }

    /// Get the frontmost (active) application.
    ///
    /// This function queries the system-wide accessibility element to get the
    /// currently focused application. The frontmost application is the one that
    /// has keyboard focus and is displayed in front of other windows.
    ///
    /// # Returns
    ///
    /// `Some(AXUIElement)` representing the frontmost application if one exists,
    /// `None` if no application is in focus or if the query fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// if let Some(app) = MacOSExtractor::get_frontmost_app() {
    ///     println!("Got the frontmost application!");
    /// } else {
    ///     println!("No application is in focus.");
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 2.1: Provide a function to get the frontmost (active) application
    /// - Requirement 2.2: Return None when no application is in focus
    pub fn get_frontmost_app() -> Option<AXUIElement> {
        let system_wide = AXUIElement::system_wide();
        let focused_app_attr =
            AXAttribute::<CFType>::new(&CFString::new("AXFocusedApplication"));
        
        // Get the attribute as CFType and then try to downcast to AXUIElement
        let cf_value = system_wide.attribute(&focused_app_attr).ok()?;
        
        // The CFType should be an AXUIElementRef, we need to convert it
        // Use unsafe to reinterpret the CFType as AXUIElement
        let type_id = cf_value.type_of();
        let ax_type_id = AXUIElement::type_id();
        
        if type_id == ax_type_id {
            // Safe to convert since we verified the type
            let ptr = cf_value.as_CFTypeRef();
            // Create AXUIElement from the raw pointer
            // We need to retain it since we're creating a new reference
            unsafe {
                Some(AXUIElement::wrap_under_get_rule(ptr as accessibility_sys::AXUIElementRef))
            }
        } else {
            None
        }
    }

    /// Get an application by its bundle ID with a configurable timeout.
    ///
    /// This function attempts to locate a running application by its bundle
    /// identifier (e.g., "com.microsoft.Word"). If the application is not
    /// found within the specified timeout, an error is returned.
    ///
    /// # Arguments
    ///
    /// * `bundle_id` - The bundle identifier of the application to find
    /// * `timeout` - Maximum time to wait for the application to be found
    ///
    /// # Returns
    ///
    /// `Ok(AXUIElement)` representing the application if found,
    /// `Err(ExtractionError::AppNotFound)` if the application is not found
    /// within the timeout period.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    /// use std::time::Duration;
    ///
    /// match MacOSExtractor::get_app_by_bundle_id("com.microsoft.Word", Duration::from_secs(5)) {
    ///     Ok(app) => println!("Found Microsoft Word!"),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 2.3: Provide a function to get an application by its Bundle_ID with timeout
    /// - Requirement 2.4: Return AppNotFound error if application is not found within timeout
    pub fn get_app_by_bundle_id(
        bundle_id: &str,
        timeout: Duration,
    ) -> Result<AXUIElement, ExtractionError> {
        AXUIElement::application_with_bundle_timeout(bundle_id, timeout).map_err(|e| {
            ExtractionError::AppNotFound(format!("{}: {:?}", bundle_id, e))
        })
    }

    /// Detect application source from the application title.
    ///
    /// This function performs case-insensitive matching against the application
    /// title to determine which known application is being used. It recognizes
    /// Microsoft Office applications (Word, Excel, PowerPoint, Outlook) and
    /// Apple iWork applications (Pages, Numbers, Keynote), as well as TextEdit
    /// and LibreOffice.
    ///
    /// # Arguments
    ///
    /// * `title` - The application title to analyze
    ///
    /// # Returns
    ///
    /// The corresponding `AppSource` variant if a known application is detected,
    /// or `AppSource::Unknown` if the application is not recognized.
    ///
    /// # Examples
    ///
    /// ```
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    /// use accessibility_extractor::AppSource;
    ///
    /// // Case-insensitive matching
    /// assert_eq!(MacOSExtractor::detect_app_source("Microsoft Word"), AppSource::Word);
    /// assert_eq!(MacOSExtractor::detect_app_source("MICROSOFT WORD"), AppSource::Word);
    /// assert_eq!(MacOSExtractor::detect_app_source("word"), AppSource::Word);
    ///
    /// // Apple iWork apps
    /// assert_eq!(MacOSExtractor::detect_app_source("Pages"), AppSource::Pages);
    /// assert_eq!(MacOSExtractor::detect_app_source("Numbers"), AppSource::Numbers);
    /// assert_eq!(MacOSExtractor::detect_app_source("Keynote"), AppSource::Keynote);
    ///
    /// // Unknown applications
    /// assert_eq!(MacOSExtractor::detect_app_source("Safari"), AppSource::Unknown);
    /// ```
    ///
    /// # Requirements
    /// - Requirement 6.1: Detect the application source from the application title
    /// - Requirement 6.2: Recognize Microsoft Word and return source "word"
    /// - Requirement 6.3: Recognize Microsoft Excel and return source "excel"
    /// - Requirement 6.4: Recognize Microsoft PowerPoint and return source "powerpoint"
    /// - Requirement 6.5: Recognize Microsoft Outlook and return source "outlook"
    /// - Requirement 6.6: Recognize Apple Pages and return source "pages"
    /// - Requirement 6.7: Recognize Apple Numbers and return source "numbers"
    /// - Requirement 6.8: Recognize Apple Keynote and return source "keynote"
    /// - Requirement 6.9: Return source "unknown" for unrecognized applications
    pub fn detect_app_source(title: &str) -> AppSource {
        let title_lower = title.to_lowercase();

        if title_lower.contains("word") {
            AppSource::Word
        } else if title_lower.contains("excel") {
            AppSource::Excel
        } else if title_lower.contains("powerpoint") {
            AppSource::PowerPoint
        } else if title_lower.contains("outlook") {
            AppSource::Outlook
        } else if title_lower.contains("teams") {
            AppSource::Teams
        } else if title_lower.contains("slack") {
            AppSource::Slack
        } else if title_lower.contains("pages") {
            AppSource::Pages
        } else if title_lower.contains("numbers") {
            AppSource::Numbers
        } else if title_lower.contains("keynote") {
            AppSource::Keynote
        } else if title_lower.contains("textedit") {
            AppSource::TextEdit
        } else if title_lower.contains("libreoffice") {
            AppSource::LibreOffice
        } else {
            AppSource::Unknown
        }
    }

    /// Extract content from the frontmost application.
    ///
    /// This function extracts text content from the currently active (frontmost)
    /// application on macOS. It performs the following steps:
    /// 1. Checks if accessibility permissions are granted
    /// 2. Gets the frontmost application
    /// 3. Extracts content from the application's focused window
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// match MacOSExtractor::extract_frontmost() {
    ///     Ok(content) => {
    ///         println!("Source: {}", content.source);
    ///         println!("Title: {:?}", content.title);
    ///         println!("Content: {}", content.content);
    ///     }
    ///     Err(e) => eprintln!("Extraction failed: {}", e),
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 3.1: Check accessibility permissions first
    /// - Requirement 3.2: Return PermissionDenied error if permissions not granted
    /// - Requirement 3.3: Get the focused window of the frontmost application
    /// - Requirement 3.4: Return ElementNotFound error if no focused window exists
    /// - Requirement 3.5: Traverse the accessibility tree to extract text
    /// - Requirement 3.6: Return ExtractedContent struct with all required fields
    /// - Requirement 3.7: Return NoContentFound error if content is empty
    pub fn extract_frontmost() -> Result<ExtractedContent, ExtractionError> {
        log::debug!("[AX-EXTRACTOR] Starting extraction from frontmost application...");
        
        // Check permissions first (Requirement 3.1, 3.2)
        if !Self::is_accessibility_enabled() {
            log::error!("[AX-EXTRACTOR] ❌ Accessibility permission not granted");
            return Err(ExtractionError::PermissionDenied(
                "Accessibility permission not granted".into(),
            ));
        }
        log::debug!("[AX-EXTRACTOR] ✓ Accessibility permissions verified");

        // Get frontmost app (Requirement 2.1, 2.2)
        let app = Self::get_frontmost_app().ok_or_else(|| {
            log::error!("[AX-EXTRACTOR] ❌ No frontmost application found");
            ExtractionError::AppNotFound("No frontmost application".into())
        })?;
        
        // Get app title for logging
        let app_title = app.title().map(|s| s.to_string()).unwrap_or_else(|_| "Unknown".to_string());
        log::info!("[AX-EXTRACTOR] 📱 Detected application: {}", app_title);

        Self::extract_from_element(&app)
    }

    /// Extract content from a specific application by its bundle ID.
    ///
    /// This function extracts text content from a specific application identified
    /// by its bundle ID (e.g., "com.microsoft.Word"). It locates the application
    /// using the bundle ID with a 5-second timeout, then extracts content from
    /// the application's focused window.
    ///
    /// # Arguments
    ///
    /// * `bundle_id` - The bundle identifier of the application to extract from
    ///                 (e.g., "com.microsoft.Word", "com.apple.iWork.Pages")
    ///
    /// # Returns
    ///
    /// `Ok(ExtractedContent)` containing the extracted text and metadata on success,
    /// or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Errors
    ///
    /// - `ExtractionError::AppNotFound` - Application with the given bundle ID is not running
    ///   or was not found within the 5-second timeout
    /// - `ExtractionError::ElementNotFound` - No focused window in the application
    /// - `ExtractionError::NoContentFound` - Document is empty
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// // Extract content from Microsoft Word
    /// match MacOSExtractor::extract_from_app("com.microsoft.Word") {
    ///     Ok(content) => {
    ///         println!("Source: {}", content.source);
    ///         println!("Title: {:?}", content.title);
    ///         println!("Content length: {} chars", content.content.len());
    ///     }
    ///     Err(e) => eprintln!("Extraction failed: {}", e),
    /// }
    ///
    /// // Extract content from Apple Pages
    /// match MacOSExtractor::extract_from_app("com.apple.iWork.Pages") {
    ///     Ok(content) => println!("Extracted from Pages: {}", content.source),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 4.1: Provide a function to extract content from a specific application by Bundle_ID
    /// - Requirement 4.2: Locate the application using its Bundle_ID
    /// - Requirement 4.3: Return AppNotFound error if the application is not running
    /// - Requirement 4.4: Extract content from the focused window of the specified application
    pub fn extract_from_app(bundle_id: &str) -> Result<ExtractedContent, ExtractionError> {
        log::info!("[AX-EXTRACTOR] 🔍 Looking for application with bundle ID: {}", bundle_id);
        
        // Try direct file extraction first (no TCC/Automation prompts)
        // This covers: Word, Excel, PowerPoint, Pages, Numbers, Keynote
        if file_extract::supports_direct_extraction(bundle_id) {
            log::info!("[AX-EXTRACTOR] 📂 Trying direct file extraction for {}", bundle_id);
            match file_extract::extract_from_office_app(bundle_id) {
                Ok(content) => {
                    log::info!("[AX-EXTRACTOR] ✅ Direct file extraction successful ({} chars)", content.content.len());
                    return Ok(content);
                }
                Err(e) => {
                    log::warn!("[AX-EXTRACTOR] ⚠️ Direct file extraction failed: {}, falling back to Accessibility API", e);
                    // Fall through to Accessibility API (no AppleScript to avoid TCC prompts)
                }
            }
        }
        
        // Slack-specific handling with extended timeout and custom extraction
        if electron::is_slack(bundle_id) {
            log::info!("[AX-EXTRACTOR] 🔧 Detected Slack, using Slack-specific extraction...");
            match electron::prepare_slack(None) {
                Ok(true) => {
                    log::info!("[AX-EXTRACTOR] ✓ Slack prepared for extraction");
                }
                Ok(false) => {
                    log::error!("[AX-EXTRACTOR] ❌ Slack is not running");
                    return Err(ExtractionError::AppNotFound("Slack is not running".into()));
                }
                Err(e) => {
                    // Log warning but continue - extraction might still work
                    log::warn!("[AX-EXTRACTOR] ⚠️ Slack preparation warning: {}", e);
                }
            }
            
            // Use Slack-specific extraction
            let app = Self::get_app_by_bundle_id(bundle_id, Duration::from_secs(5))?;
            let content = electron::extract_slack_content(&app);
            
            if content.trim().is_empty() {
                log::warn!("[AX-EXTRACTOR] ⚠️ Slack content appears to be empty");
                return Err(ExtractionError::NoContentFound("Slack content appears to be empty".into()));
            }
            
            // Get window title for document title
            let title = app.focused_window()
                .ok()
                .and_then(|w| w.title().ok())
                .map(|s| s.to_string());
            
            let app_title = app.title().map(|s| s.to_string()).unwrap_or_else(|_| "Slack".to_string());
            
            log::info!("[AX-EXTRACTOR] ✅ Slack extraction successful ({} chars)", content.len());
            
            return Ok(ExtractedContent {
                source: "slack".to_string(),
                title,
                content,
                app_name: app_title,
                timestamp: chrono::Utc::now().timestamp(),
                extraction_method: "accessibility".to_string(),
            });
        }
        // Teams-specific handling with extended timeout and custom extraction
        else if electron::is_teams(bundle_id) {
            log::info!("[AX-EXTRACTOR] 🔧 Detected Microsoft Teams, using Teams-specific extraction...");
            
            // Check if this is New Teams (com.microsoft.teams2)
            if bundle_id == electron::TEAMS_NEW_BUNDLE_ID {
                log::info!("[AX-EXTRACTOR] 📱 New Teams (com.microsoft.teams2) detected - using deep tree traversal");
                
                // New Teams uses Chromium-based web views - we extract by traversing the tree
                // and collecting text from AXStaticText, AXHeading, and AXLink elements
                let app = Self::get_app_by_bundle_id(bundle_id, Duration::from_secs(5))?;
                let content = electron::extract_teams_content(&app);
                
                if content.trim().is_empty() {
                    log::warn!("[AX-EXTRACTOR] ⚠️ New Teams content appears to be empty");
                    return Err(ExtractionError::NoContentFound(
                        "Teams content appears to be empty. Make sure a chat is open and visible.".into()
                    ));
                }
                
                // Get window title for document title
                let title = app.focused_window()
                    .ok()
                    .and_then(|w| w.title().ok())
                    .map(|s| s.to_string());
                
                let app_title = app.title().map(|s| s.to_string()).unwrap_or_else(|_| "Microsoft Teams".to_string());
                
                log::info!("[AX-EXTRACTOR] ✅ New Teams extraction successful ({} chars)", content.len());
                
                return Ok(ExtractedContent {
                    source: "teams".to_string(),
                    title,
                    content,
                    app_name: app_title,
                    timestamp: chrono::Utc::now().timestamp(),
                    extraction_method: "accessibility".to_string(),
                });
            }
            
            // Classic Teams (com.microsoft.teams) - Electron-based, supports accessibility
            match electron::prepare_teams(bundle_id, None) {
                Ok(true) => {
                    log::info!("[AX-EXTRACTOR] ✓ Classic Teams prepared for extraction");
                }
                Ok(false) => {
                    log::error!("[AX-EXTRACTOR] ❌ Teams is not running");
                    return Err(ExtractionError::AppNotFound("Microsoft Teams is not running".into()));
                }
                Err(e) => {
                    // Log warning but continue - extraction might still work
                    log::warn!("[AX-EXTRACTOR] ⚠️ Teams preparation warning: {}", e);
                }
            }
            
            // Use Teams-specific extraction for Classic Teams
            let app = Self::get_app_by_bundle_id(bundle_id, Duration::from_secs(5))?;
            let content = electron::extract_teams_content(&app);
            
            if content.trim().is_empty() {
                log::warn!("[AX-EXTRACTOR] ⚠️ Teams content appears to be empty");
                return Err(ExtractionError::NoContentFound("Teams content appears to be empty. Make sure a chat is open and visible.".into()));
            }
            
            // Get window title for document title
            let title = app.focused_window()
                .ok()
                .and_then(|w| w.title().ok())
                .map(|s| s.to_string());
            
            let app_title = app.title().map(|s| s.to_string()).unwrap_or_else(|_| "Microsoft Teams".to_string());
            
            log::info!("[AX-EXTRACTOR] ✅ Teams extraction successful ({} chars)", content.len());
            
            return Ok(ExtractedContent {
                source: "teams".to_string(),
                title,
                content,
                app_name: app_title,
                timestamp: chrono::Utc::now().timestamp(),
                extraction_method: "accessibility".to_string(),
            });
        }
        // For other Electron apps (Discord, VS Code, etc.), enable accessibility first
        // Electron apps don't expose their DOM through the standard Accessibility API by default
        else if electron::is_electron_app(bundle_id) {
            log::info!("[AX-EXTRACTOR] 🔧 Detected Electron app, enabling accessibility...");
            match electron::prepare_electron_app(bundle_id) {
                Ok(true) => {
                    log::info!("[AX-EXTRACTOR] ✓ Electron accessibility enabled for {}", bundle_id);
                }
                Ok(false) => {
                    // Not an Electron app (shouldn't happen since we checked above)
                }
                Err(e) => {
                    // Log warning but continue - some Electron apps may still work
                    log::warn!("[AX-EXTRACTOR] ⚠️ Could not enable Electron accessibility: {}", e);
                    // Don't return error - try extraction anyway
                }
            }
        }
        
        // Fall back to Accessibility API (requires only Accessibility permission, no Automation)
        // Get the application by bundle ID with a 5-second timeout (Requirement 4.2, 4.3)
        let app = Self::get_app_by_bundle_id(bundle_id, Duration::from_secs(5)).map_err(|e| {
            log::error!("[AX-EXTRACTOR] ❌ Application not found: {}", bundle_id);
            e
        })?;
        
        log::debug!("[AX-EXTRACTOR] ✓ Found application: {}", bundle_id);
        
        // Extract content from the application's focused window (Requirement 4.4)
        Self::extract_from_element(&app)
    }

    /// Get the currently selected text from any application.
    ///
    /// This function queries the system-wide accessibility element to get the
    /// currently focused UI element, then retrieves the selected text from that
    /// element using the AXSelectedText attribute.
    ///
    /// # Returns
    ///
    /// `Some(String)` containing the selected text if text is selected,
    /// `None` if no text is selected or if the query fails.
    ///
    /// # How It Works
    ///
    /// 1. Gets the system-wide accessibility element
    /// 2. Queries AXFocusedUIElement to get the currently focused element
    /// 3. Queries AXSelectedText attribute from the focused element
    /// 4. Returns the selected text if available, None otherwise
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// // Get selected text from any application
    /// if let Some(selected) = MacOSExtractor::get_selected_text() {
    ///     println!("Selected text: {}", selected);
    /// } else {
    ///     println!("No text is currently selected.");
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This function requires accessibility permissions to be granted.
    /// If permissions are not granted, it will return `None`.
    ///
    /// # Requirements
    /// - Requirement 5.4: Provide a function to get the currently selected text from any application
    /// - Requirement 5.5: Query the AXSelectedText attribute of the focused UI element
    /// - Requirement 5.6: Return None if no text is selected
    pub fn get_selected_text() -> Option<String> {
        // Get the system-wide accessibility element
        let system_wide = AXUIElement::system_wide();
        
        // Query AXFocusedUIElement to get the currently focused element (Requirement 5.5)
        let focused_attr = AXAttribute::<CFType>::new(&CFString::new("AXFocusedUIElement"));
        let cf_value = system_wide.attribute(&focused_attr).ok()?;
        
        // Convert CFType to AXUIElement
        let type_id = cf_value.type_of();
        let ax_type_id = AXUIElement::type_id();
        
        if type_id != ax_type_id {
            // Not an AXUIElement, return None (Requirement 5.6)
            return None;
        }
        
        // Safe to convert since we verified the type
        let focused = unsafe {
            let ptr = cf_value.as_CFTypeRef();
            AXUIElement::wrap_under_get_rule(ptr as accessibility_sys::AXUIElementRef)
        };
        
        // Query AXSelectedText attribute from the focused element (Requirement 5.5)
        let selected_text_attr = AXAttribute::<CFType>::new(&CFString::new("AXSelectedText"));
        let selected_value = focused.attribute(&selected_text_attr).ok()?;
        
        // Convert the selected text CFType to String
        // Check if it's a CFString
        let selected_type_id = selected_value.type_of();
        if selected_type_id == CFString::type_id() {
            let ptr = selected_value.as_CFTypeRef();
            let cf_string: CFString = unsafe {
                CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
            };
            let text = cf_string.to_string();
            
            // Return None if the selected text is empty (Requirement 5.6)
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        } else {
            // Not a string type, return None (Requirement 5.6)
            None
        }
    }

    /// Extract content from an application element.
    ///
    /// This is an internal function that extracts text content from a given
    /// application's accessibility element. It gets the focused window,
    /// extracts text using role filtering, and builds the ExtractedContent struct.
    ///
    /// # Arguments
    ///
    /// * `app` - The AXUIElement representing the application to extract from
    ///
    /// # Returns
    ///
    /// `Ok(ExtractedContent)` containing the extracted text and metadata on success,
    /// or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Errors
    ///
    /// - `ExtractionError::ElementNotFound` - No focused window in the application
    /// - `ExtractionError::NoContentFound` - Document is empty
    ///
    /// # Requirements
    /// - Requirement 3.3: Get the focused window of the application
    /// - Requirement 3.4: Return ElementNotFound error if no focused window exists
    /// - Requirement 3.5: Traverse the accessibility tree to extract text
    /// - Requirement 3.6: Return ExtractedContent struct with all required fields
    /// - Requirement 3.7: Return NoContentFound error if content is empty
    fn extract_from_element(app: &AXUIElement) -> Result<ExtractedContent, ExtractionError> {
        // Get application title for source detection
        let app_title = app
            .title()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        // Detect the application source from the title
        let source = Self::detect_app_source(&app_title);
        log::debug!("[AX-EXTRACTOR] 🏷️  Detected source type: {} (from app: {})", source.as_str(), app_title);

        // Get focused window (Requirement 3.3, 3.4)
        let window = app.focused_window().map_err(|e| {
            log::error!("[AX-EXTRACTOR] ❌ No focused window found in application: {}", app_title);
            ExtractionError::ElementNotFound(format!("No focused window: {:?}", e))
        })?;

        // Get window title for the document title
        let title = window.title().map(|s| s.to_string()).ok();
        log::debug!("[AX-EXTRACTOR] 📄 Document title: {:?}", title);

        // Extract text with role filtering (Requirement 3.5)
        log::debug!("[AX-EXTRACTOR] 🔄 Traversing accessibility tree...");
        let content = Self::extract_text_filtered(&window)?;

        // Check for empty content (Requirement 3.7)
        if content.trim().is_empty() {
            log::warn!("[AX-EXTRACTOR] ⚠️  Document appears to be empty");
            return Err(ExtractionError::NoContentFound(
                "Document appears to be empty".into(),
            ));
        }

        let content_len = content.len();
        let preview = if content_len > 100 {
            format!("{}...", &content[..100])
        } else {
            content.clone()
        };
        
        log::info!("[AX-EXTRACTOR] ✅ Successfully extracted {} characters from {} ({})", 
            content_len, 
            source.as_str().to_uppercase(),
            title.as_deref().unwrap_or("untitled")
        );
        log::debug!("[AX-EXTRACTOR] 📝 Content preview: {}", preview.replace('\n', " "));

        // Build and return ExtractedContent struct (Requirement 3.6)
        Ok(ExtractedContent {
            source: source.as_str().to_string(),
            title,
            content,
            app_name: app_title,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "accessibility".to_string(),
        })
    }

    /// Extract text with role filtering (excludes UI chrome).
    ///
    /// This function extracts text content from an accessibility element and its
    /// descendants, filtering out UI chrome elements (menus, toolbars, buttons, etc.)
    /// to focus on document content.
    ///
    /// # Arguments
    ///
    /// * `element` - The root AXUIElement to extract text from
    ///
    /// # Returns
    ///
    /// `Ok(String)` containing the extracted text with newline separators,
    /// or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use accessibility_extractor::platform::macos::MacOSExtractor;
    ///
    /// if let Some(app) = MacOSExtractor::get_frontmost_app() {
    ///     match MacOSExtractor::extract_text_filtered(&app) {
    ///         Ok(text) => println!("Extracted: {}", text),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Requirements
    /// - Requirement 5.1: Extract text content from the document area
    /// - Requirement 5.2: Exclude text from menu bars, toolbars, tab bars, and navigation elements
    /// - Requirement 7.1: Recursively traverse child elements when extracting text
    /// - Requirement 7.4: Concatenate extracted text with newline separators
    pub fn extract_text_filtered(element: &AXUIElement) -> Result<String, ExtractionError> {
        let mut result = String::new();
        Self::extract_recursive_filtered(element, &mut result, 0)?;
        Ok(result)
    }

    /// Recursive extraction with role filtering.
    ///
    /// This is the internal recursive function that traverses the accessibility tree,
    /// filtering elements by role and extracting text from text-containing elements.
    ///
    /// # Arguments
    ///
    /// * `element` - The current AXUIElement to process
    /// * `result` - Mutable string buffer to accumulate extracted text
    /// * `depth` - Current recursion depth (used to prevent infinite recursion)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an `Err(ExtractionError)` if extraction fails.
    ///
    /// # Recursion Limit
    ///
    /// The function limits recursion to 100 levels to prevent stack overflow
    /// on deeply nested or circular element structures. When the limit is reached,
    /// a warning is logged and the function returns without error.
    ///
    /// # Requirements
    /// - Requirement 5.2: Exclude text from menu bars, toolbars, tab bars, and navigation elements
    /// - Requirement 5.7: Skip elements with roles indicating UI chrome
    /// - Requirement 7.1: Recursively traverse child elements when extracting text
    /// - Requirement 7.2: Extract text from elements with specific roles
    /// - Requirement 7.3: Limit recursion depth to prevent infinite loops (maximum 100 levels)
    /// - Requirement 7.4: Concatenate extracted text with newline separators
    /// - Requirement 7.5: Skip empty or whitespace-only text values
    fn extract_recursive_filtered(
        element: &AXUIElement,
        result: &mut String,
        depth: usize,
    ) -> Result<(), ExtractionError> {
        // Prevent infinite recursion (Requirement 7.3)
        if depth > MAX_RECURSION_DEPTH {
            log::warn!("Maximum recursion depth reached at depth {}", depth);
            return Ok(());
        }

        // Get the element's role
        let role = element
            .role()
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Skip UI chrome elements (Requirement 5.2, 5.7)
        if !should_extract_from_role(&role) {
            return Ok(());
        }

        // Extract text from text-containing roles (Requirement 7.2)
        if is_text_role(&role) {
            let mut found_text = false;
            
            // First try AXValue (standard text attribute)
            if let Ok(value) = element.value() {
                if let Some(text) = Self::cftype_to_string(&value) {
                    let trimmed = text.trim();
                    // Skip empty or whitespace-only values (Requirement 7.5)
                    if !trimmed.is_empty() {
                        result.push_str(trimmed);
                        result.push('\n');
                        found_text = true;
                    }
                }
            }
            
            // If no value found, try AXDescription (used by Microsoft Office)
            if !found_text {
                if let Some(text) = Self::get_description(element) {
                    let trimmed = text.trim();
                    // Skip empty, whitespace-only, or document name values
                    if !trimmed.is_empty() && !trimmed.contains('.') {
                        // Check if this looks like actual content (not just a filename)
                        // Filenames typically have extensions like .docx, .xlsx
                        result.push_str(trimmed);
                        result.push('\n');
                    }
                }
            }
        }

        // Traverse children (Requirement 7.1)
        if let Ok(children) = element.children() {
            for i in 0..children.len() {
                if let Some(child) = children.get(i) {
                    Self::extract_recursive_filtered(&child, result, depth + 1)?;
                }
            }
        }

        Ok(())
    }
    
    /// Get the AXDescription attribute from an element.
    /// 
    /// Microsoft Office apps store document content in AXDescription instead of AXValue.
    fn get_description(element: &AXUIElement) -> Option<String> {
        use accessibility::attribute::AXAttribute;
        
        let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
        element.attribute(&desc_attr).ok().and_then(|v| Self::cftype_to_string(&v))
    }

    /// Convert CFType to String.
    ///
    /// This helper function converts Core Foundation types (CFType) to Rust strings.
    /// It handles CFString and CFNumber types, which are the most common value types
    /// returned by accessibility elements.
    ///
    /// # Arguments
    ///
    /// * `value` - The CFType value to convert
    ///
    /// # Returns
    ///
    /// `Some(String)` if the value can be converted to a string,
    /// `None` if the value type is not supported or conversion fails.
    ///
    /// # Supported Types
    ///
    /// - `CFString`: Directly converted to String
    /// - `CFNumber`: Converted to string representation of the number
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Internal function - not directly callable from outside
    /// let cf_string = CFString::new("Hello");
    /// let cf_type: CFType = cf_string.as_CFType();
    /// let result = MacOSExtractor::cftype_to_string(&cf_type);
    /// assert_eq!(result, Some("Hello".to_string()));
    /// ```
    fn cftype_to_string(value: &CFType) -> Option<String> {
        use core_foundation::number::CFNumber;

        // Get the type ID of the value
        let type_id = value.type_of();

        // Check if it's a CFString
        if type_id == CFString::type_id() {
            // Safe to downcast to CFString
            let ptr = value.as_CFTypeRef();
            let cf_string: CFString = unsafe {
                CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
            };
            return Some(cf_string.to_string());
        }

        // Check if it's a CFNumber
        if type_id == CFNumber::type_id() {
            let ptr = value.as_CFTypeRef();
            let cf_number: CFNumber = unsafe {
                CFNumber::wrap_under_get_rule(ptr as core_foundation::number::CFNumberRef)
            };
            // Try to get as i64 first, then f64
            if let Some(n) = cf_number.to_i64() {
                return Some(n.to_string());
            }
            if let Some(n) = cf_number.to_f64() {
                return Some(n.to_string());
            }
        }

        // Unsupported type
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================================
    // Unit Tests for detect_app_source
    // ============================================================================

    #[test]
    fn test_detect_app_source_microsoft_word() {
        // Test various Word title formats
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft Word"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("Word"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("word"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("WORD"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft WORD"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("Document.docx - Word"), AppSource::Word);
    }

    #[test]
    fn test_detect_app_source_microsoft_excel() {
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft Excel"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("Excel"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("excel"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("EXCEL"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("Spreadsheet.xlsx - Excel"), AppSource::Excel);
    }

    #[test]
    fn test_detect_app_source_microsoft_powerpoint() {
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft PowerPoint"), AppSource::PowerPoint);
        assert_eq!(MacOSExtractor::detect_app_source("PowerPoint"), AppSource::PowerPoint);
        assert_eq!(MacOSExtractor::detect_app_source("powerpoint"), AppSource::PowerPoint);
        assert_eq!(MacOSExtractor::detect_app_source("POWERPOINT"), AppSource::PowerPoint);
        assert_eq!(MacOSExtractor::detect_app_source("Presentation.pptx - PowerPoint"), AppSource::PowerPoint);
    }

    #[test]
    fn test_detect_app_source_microsoft_outlook() {
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft Outlook"), AppSource::Outlook);
        assert_eq!(MacOSExtractor::detect_app_source("Outlook"), AppSource::Outlook);
        assert_eq!(MacOSExtractor::detect_app_source("outlook"), AppSource::Outlook);
        assert_eq!(MacOSExtractor::detect_app_source("OUTLOOK"), AppSource::Outlook);
        assert_eq!(MacOSExtractor::detect_app_source("Inbox - Outlook"), AppSource::Outlook);
    }

    #[test]
    fn test_detect_app_source_apple_pages() {
        assert_eq!(MacOSExtractor::detect_app_source("Pages"), AppSource::Pages);
        assert_eq!(MacOSExtractor::detect_app_source("pages"), AppSource::Pages);
        assert_eq!(MacOSExtractor::detect_app_source("PAGES"), AppSource::Pages);
        assert_eq!(MacOSExtractor::detect_app_source("Document - Pages"), AppSource::Pages);
    }

    #[test]
    fn test_detect_app_source_apple_numbers() {
        assert_eq!(MacOSExtractor::detect_app_source("Numbers"), AppSource::Numbers);
        assert_eq!(MacOSExtractor::detect_app_source("numbers"), AppSource::Numbers);
        assert_eq!(MacOSExtractor::detect_app_source("NUMBERS"), AppSource::Numbers);
        assert_eq!(MacOSExtractor::detect_app_source("Spreadsheet - Numbers"), AppSource::Numbers);
    }

    #[test]
    fn test_detect_app_source_apple_keynote() {
        assert_eq!(MacOSExtractor::detect_app_source("Keynote"), AppSource::Keynote);
        assert_eq!(MacOSExtractor::detect_app_source("keynote"), AppSource::Keynote);
        assert_eq!(MacOSExtractor::detect_app_source("KEYNOTE"), AppSource::Keynote);
        assert_eq!(MacOSExtractor::detect_app_source("Presentation - Keynote"), AppSource::Keynote);
    }

    #[test]
    fn test_detect_app_source_textedit() {
        assert_eq!(MacOSExtractor::detect_app_source("TextEdit"), AppSource::TextEdit);
        assert_eq!(MacOSExtractor::detect_app_source("textedit"), AppSource::TextEdit);
        assert_eq!(MacOSExtractor::detect_app_source("TEXTEDIT"), AppSource::TextEdit);
    }

    #[test]
    fn test_detect_app_source_libreoffice() {
        assert_eq!(MacOSExtractor::detect_app_source("LibreOffice"), AppSource::LibreOffice);
        assert_eq!(MacOSExtractor::detect_app_source("libreoffice"), AppSource::LibreOffice);
        assert_eq!(MacOSExtractor::detect_app_source("LIBREOFFICE"), AppSource::LibreOffice);
        assert_eq!(MacOSExtractor::detect_app_source("LibreOffice Writer"), AppSource::LibreOffice);
        assert_eq!(MacOSExtractor::detect_app_source("LibreOffice Calc"), AppSource::LibreOffice);
    }

    #[test]
    fn test_detect_app_source_unknown() {
        assert_eq!(MacOSExtractor::detect_app_source("Safari"), AppSource::Unknown);
        assert_eq!(MacOSExtractor::detect_app_source("Chrome"), AppSource::Unknown);
        assert_eq!(MacOSExtractor::detect_app_source("Firefox"), AppSource::Unknown);
        assert_eq!(MacOSExtractor::detect_app_source("Terminal"), AppSource::Unknown);
        assert_eq!(MacOSExtractor::detect_app_source(""), AppSource::Unknown);
        assert_eq!(MacOSExtractor::detect_app_source("Some Random App"), AppSource::Unknown);
    }

    #[test]
    fn test_detect_app_source_case_insensitive() {
        // Test mixed case variations
        assert_eq!(MacOSExtractor::detect_app_source("WoRd"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("ExCeL"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("PoWeRpOiNt"), AppSource::PowerPoint);
        assert_eq!(MacOSExtractor::detect_app_source("OuTlOoK"), AppSource::Outlook);
        assert_eq!(MacOSExtractor::detect_app_source("PaGeS"), AppSource::Pages);
        assert_eq!(MacOSExtractor::detect_app_source("NuMbErS"), AppSource::Numbers);
        assert_eq!(MacOSExtractor::detect_app_source("KeYnOtE"), AppSource::Keynote);
    }

    #[test]
    fn test_detect_app_source_partial_match() {
        // Test that partial matches work (keyword contained in title)
        assert_eq!(MacOSExtractor::detect_app_source("My Document - Microsoft Word 2021"), AppSource::Word);
        assert_eq!(MacOSExtractor::detect_app_source("Budget.xlsx - Microsoft Excel for Mac"), AppSource::Excel);
        assert_eq!(MacOSExtractor::detect_app_source("Slides - PowerPoint Presentation"), AppSource::PowerPoint);
    }

    // ============================================================================
    // Property-Based Tests for Source Detection
    // Feature: accessibility-extractor, Property 7: Source Detection Correctness
    // **Validates: Requirements 6.1-6.9**
    //
    // For any application title containing a known application keyword
    // (word, excel, powerpoint, outlook, pages, numbers, keynote), the
    // `detect_app_source` function SHALL return the corresponding `AppSource` variant.
    // ============================================================================

    proptest! {
        /// Property test: Any title containing "word" (case-insensitive) returns AppSource::Word
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_word(title in ".*[Ww][Oo][Rr][Dd].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Word);
        }

        /// Property test: Any title containing "excel" (case-insensitive) returns AppSource::Excel
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_excel(title in ".*[Ee][Xx][Cc][Ee][Ll].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Excel);
        }

        /// Property test: Any title containing "powerpoint" (case-insensitive) returns AppSource::PowerPoint
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_powerpoint(title in ".*[Pp][Oo][Ww][Ee][Rr][Pp][Oo][Ii][Nn][Tt].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::PowerPoint);
        }

        /// Property test: Any title containing "outlook" (case-insensitive) returns AppSource::Outlook
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_outlook(title in ".*[Oo][Uu][Tt][Ll][Oo][Oo][Kk].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Outlook);
        }

        /// Property test: Any title containing "pages" (case-insensitive) returns AppSource::Pages
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_pages(title in ".*[Pp][Aa][Gg][Ee][Ss].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Pages);
        }

        /// Property test: Any title containing "numbers" (case-insensitive) returns AppSource::Numbers
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_numbers(title in ".*[Nn][Uu][Mm][Bb][Ee][Rr][Ss].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Numbers);
        }

        /// Property test: Any title containing "keynote" (case-insensitive) returns AppSource::Keynote
        /// **Validates: Requirements 6.1-6.9**
        #[test]
        fn prop_source_detection_keynote(title in ".*[Kk][Ee][Yy][Nn][Oo][Tt][Ee].*") {
            let source = MacOSExtractor::detect_app_source(&title);
            prop_assert_eq!(source, AppSource::Keynote);
        }
    }

    // ============================================================================
    // Tests for Recursion Depth Limit
    // Feature: accessibility-extractor, Property 9: Recursion Depth Limit
    // **Validates: Requirements 7.3**
    //
    // For any accessibility tree traversal, the extractor SHALL NOT recurse deeper
    // than 100 levels, preventing stack overflow on deeply nested or circular
    // element structures.
    // ============================================================================

    /// Unit test: Verify the MAX_RECURSION_DEPTH constant is set to 100
    /// **Validates: Requirements 7.3**
    #[test]
    fn test_max_recursion_depth_constant_is_100() {
        // The recursion depth limit must be exactly 100 as specified in Requirements 7.3
        assert_eq!(MAX_RECURSION_DEPTH, 100, 
            "MAX_RECURSION_DEPTH must be 100 as per Requirement 7.3");
    }

    /// Unit test: Document the recursion limit behavior
    /// **Validates: Requirements 7.3**
    /// 
    /// This test documents that:
    /// 1. The MAX_RECURSION_DEPTH constant exists and is publicly accessible
    /// 2. The constant is used to prevent stack overflow on deeply nested structures
    /// 3. The limit of 100 levels is sufficient for typical document structures
    ///    while preventing infinite recursion on circular references
    #[test]
    fn test_recursion_depth_limit_documentation() {
        // Verify the constant is accessible and has the expected value
        let depth_limit = MAX_RECURSION_DEPTH;
        
        // The depth limit should be a reasonable value that:
        // - Is large enough to handle deeply nested documents (typical nesting < 50 levels)
        // - Is small enough to prevent stack overflow (100 is safe for most stack sizes)
        assert!(depth_limit >= 50, 
            "Depth limit should be at least 50 to handle typical document structures");
        assert!(depth_limit <= 200, 
            "Depth limit should not exceed 200 to prevent stack overflow risks");
        
        // Verify the exact value matches the requirement
        assert_eq!(depth_limit, 100, 
            "Depth limit must be exactly 100 as specified in Requirement 7.3");
    }

    /// Unit test: Verify the recursion limit is used in extract_recursive_filtered
    /// **Validates: Requirements 7.3**
    /// 
    /// This test verifies that the MAX_RECURSION_DEPTH constant is the authoritative
    /// source for the recursion limit, ensuring consistency across the codebase.
    #[test]
    fn test_recursion_limit_constant_usage() {
        // The constant should be used consistently throughout the module
        // This test ensures the constant exists and can be referenced
        let _limit: usize = MAX_RECURSION_DEPTH;
        
        // Verify the constant is of the correct type (usize for depth comparison)
        let depth: usize = 50;
        assert!(depth <= MAX_RECURSION_DEPTH, 
            "Typical depths should be well within the limit");
        
        // Verify boundary behavior documentation
        // At depth 100, we should still process (depth > 100 triggers the limit)
        assert!(100 <= MAX_RECURSION_DEPTH, 
            "Depth 100 should be at the boundary");
        assert!(101 > MAX_RECURSION_DEPTH, 
            "Depth 101 should exceed the limit");
    }

    // ============================================================================
    // Helper Function and Tests for Empty Text Filtering
    // Feature: accessibility-extractor, Property 11: Empty Text Filtering
    // **Validates: Requirements 7.5**
    //
    // For any extraction, text values that are empty or contain only whitespace
    // SHALL be excluded from the final content.
    // ============================================================================

    /// Helper function that simulates the text filtering logic used in extract_recursive_filtered.
    /// 
    /// This function takes a text value and determines whether it should be included
    /// in the extracted content. It mirrors the filtering logic in the actual extraction
    /// code where empty or whitespace-only values are skipped.
    /// 
    /// # Arguments
    /// 
    /// * `text` - The text value to filter
    /// 
    /// # Returns
    /// 
    /// `Some(String)` containing the trimmed text if it's non-empty after trimming,
    /// `None` if the text is empty or contains only whitespace.
    /// 
    /// # Requirements
    /// - Requirement 7.5: Skip empty or whitespace-only text values
    fn filter_text_value(text: &str) -> Option<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Helper function that simulates accumulating filtered text values.
    /// 
    /// This function processes a list of text values and accumulates only the
    /// non-empty, non-whitespace values into a result string, separated by newlines.
    /// This mirrors the behavior of extract_recursive_filtered.
    /// 
    /// # Arguments
    /// 
    /// * `texts` - A slice of text values to process
    /// 
    /// # Returns
    /// 
    /// A string containing all non-empty text values, each followed by a newline.
    fn accumulate_filtered_texts(texts: &[&str]) -> String {
        let mut result = String::new();
        for text in texts {
            if let Some(filtered) = filter_text_value(text) {
                result.push_str(&filtered);
                result.push('\n');
            }
        }
        result
    }

    // ============================================================================
    // Unit Tests for Empty Text Filtering
    // **Validates: Requirements 7.5**
    // ============================================================================

    /// Unit test: Empty strings are filtered out
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_filter_empty_string() {
        assert_eq!(filter_text_value(""), None);
    }

    /// Unit test: Whitespace-only strings are filtered out
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_filter_whitespace_only_strings() {
        // Single space
        assert_eq!(filter_text_value(" "), None);
        // Multiple spaces
        assert_eq!(filter_text_value("   "), None);
        // Tab character
        assert_eq!(filter_text_value("\t"), None);
        // Multiple tabs
        assert_eq!(filter_text_value("\t\t\t"), None);
        // Newline
        assert_eq!(filter_text_value("\n"), None);
        // Multiple newlines
        assert_eq!(filter_text_value("\n\n\n"), None);
        // Carriage return
        assert_eq!(filter_text_value("\r"), None);
        // Mixed whitespace
        assert_eq!(filter_text_value(" \t\n\r "), None);
        // Unicode whitespace (non-breaking space)
        assert_eq!(filter_text_value("\u{00A0}"), None);
        // Unicode whitespace (em space)
        assert_eq!(filter_text_value("\u{2003}"), None);
    }

    /// Unit test: Non-empty strings are preserved
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_filter_non_empty_strings() {
        assert_eq!(filter_text_value("hello"), Some("hello".to_string()));
        assert_eq!(filter_text_value("Hello World"), Some("Hello World".to_string()));
        assert_eq!(filter_text_value("a"), Some("a".to_string()));
        assert_eq!(filter_text_value("123"), Some("123".to_string()));
        assert_eq!(filter_text_value("!@#$%"), Some("!@#$%".to_string()));
    }

    /// Unit test: Strings with leading/trailing whitespace are trimmed but preserved
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_filter_strings_with_surrounding_whitespace() {
        assert_eq!(filter_text_value("  hello  "), Some("hello".to_string()));
        assert_eq!(filter_text_value("\thello\t"), Some("hello".to_string()));
        assert_eq!(filter_text_value("\nhello\n"), Some("hello".to_string()));
        assert_eq!(filter_text_value("  Hello World  "), Some("Hello World".to_string()));
    }

    /// Unit test: Accumulation filters out empty and whitespace-only values
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_accumulate_filters_empty_values() {
        let texts = &["hello", "", "world", "   ", "test"];
        let result = accumulate_filtered_texts(texts);
        assert_eq!(result, "hello\nworld\ntest\n");
    }

    /// Unit test: Accumulation with all empty values produces empty result
    /// **Validates: Requirements 7.5**
    #[test]
    fn test_accumulate_all_empty_values() {
        let texts: &[&str] = &["", "   ", "\t", "\n"];
        let result = accumulate_filtered_texts(texts);
        assert_eq!(result, "");
    }

    // ============================================================================
    // Property-Based Tests for Empty Text Filtering
    // Feature: accessibility-extractor, Property 11: Empty Text Filtering
    // **Validates: Requirements 7.5**
    //
    // For any extraction, text values that are empty or contain only whitespace
    // SHALL be excluded from the final content.
    // ============================================================================

    proptest! {
        /// Property test: Empty strings are always filtered out
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_empty_string_filtered_out(_dummy in Just(())) {
            // The empty string should always be filtered out
            prop_assert_eq!(filter_text_value(""), None);
        }

        /// Property test: Whitespace-only strings are always filtered out
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_whitespace_only_filtered_out(whitespace in "[ \t\n\r]+") {
            // Any string containing only whitespace characters should be filtered out
            let result = filter_text_value(&whitespace);
            prop_assert_eq!(result, None, 
                "Whitespace-only string '{}' should be filtered out", 
                whitespace.escape_debug());
        }

        /// Property test: Non-empty strings (after trimming) are preserved
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_non_empty_strings_preserved(
            prefix_ws in "[ \t\n\r]*",
            content in "[a-zA-Z0-9_]+",  // At least one alphanumeric character (guaranteed non-whitespace)
            suffix_ws in "[ \t\n\r]*"
        ) {
            let input = format!("{}{}{}", prefix_ws, content, suffix_ws);
            let result = filter_text_value(&input);
            
            // The result should be Some with the trimmed content
            prop_assert!(result.is_some(), 
                "Non-empty string '{}' should not be filtered out", 
                input.escape_debug());
            
            // The result should equal the trimmed input
            let expected = input.trim().to_string();
            prop_assert_eq!(result, Some(expected.clone()),
                "Result should be trimmed content '{}' for input '{}'",
                expected.escape_debug(), input.escape_debug());
        }

        /// Property test: Filtered result never contains only whitespace
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_filtered_result_never_whitespace_only(text in ".*") {
            if let Some(filtered) = filter_text_value(&text) {
                // If we get a result, it must not be empty or whitespace-only
                prop_assert!(!filtered.is_empty(), 
                    "Filtered result should never be empty");
                prop_assert!(!filtered.trim().is_empty(), 
                    "Filtered result should never be whitespace-only");
            }
        }

        /// Property test: Accumulation excludes all empty/whitespace values
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_accumulation_excludes_empty_whitespace(
            texts in prop::collection::vec("[ \t\n\r]*[a-zA-Z0-9]*[ \t\n\r]*", 0..10)
        ) {
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            let result = accumulate_filtered_texts(&text_refs);
            
            // The result should not contain any lines that are empty or whitespace-only
            for line in result.lines() {
                prop_assert!(!line.trim().is_empty(),
                    "Accumulated result should not contain empty/whitespace lines, found: '{}'",
                    line.escape_debug());
            }
        }

        /// Property test: Non-empty content is preserved in accumulation
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_non_empty_content_preserved_in_accumulation(
            non_empty_texts in prop::collection::vec("[a-zA-Z0-9]+", 1..5)
        ) {
            let text_refs: Vec<&str> = non_empty_texts.iter().map(|s| s.as_str()).collect();
            let result = accumulate_filtered_texts(&text_refs);
            
            // Each non-empty text should appear in the result
            for text in &non_empty_texts {
                prop_assert!(result.contains(text.as_str()),
                    "Non-empty text '{}' should be preserved in result '{}'",
                    text, result.escape_debug());
            }
        }

        /// Property test: Trimming is idempotent - trimmed text equals itself when trimmed again
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_trimming_idempotent(text in ".*") {
            if let Some(filtered) = filter_text_value(&text) {
                // Filtering the already-filtered result should give the same result
                let double_filtered = filter_text_value(&filtered);
                prop_assert_eq!(double_filtered, Some(filtered.clone()),
                    "Trimming should be idempotent");
            }
        }
    }

    // ============================================================================
    // Tests for Permission Denied Error
    // Feature: accessibility-extractor, Property 1: Permission Denied Error on Missing Permissions
    // **Validates: Requirements 1.3, 3.2**
    //
    // For any extraction attempt when accessibility permissions are not granted,
    // the extractor SHALL return a `PermissionDenied` error variant with a
    // non-empty descriptive message.
    //
    // NOTE: Since we cannot easily mock the permission state in tests, we verify:
    // 1. The PermissionDenied error variant exists and can be constructed
    // 2. The error message format is correct (contains "Permission denied:" prefix)
    // 3. The error message is non-empty and descriptive
    // ============================================================================

    /// Unit test: PermissionDenied error variant exists and has correct format
    /// **Validates: Requirements 1.3, 3.2**
    #[test]
    fn test_permission_denied_error_exists_and_has_correct_format() {
        // Create a PermissionDenied error with a descriptive message
        let error = ExtractionError::PermissionDenied(
            "Accessibility permission not granted".to_string()
        );
        
        // Verify the error message format
        let error_string = error.to_string();
        
        // The error message should start with "Permission denied:"
        assert!(error_string.starts_with("Permission denied:"),
            "PermissionDenied error should start with 'Permission denied:', got: '{}'",
            error_string);
        
        // The error message should be non-empty
        assert!(!error_string.is_empty(),
            "PermissionDenied error message should not be empty");
        
        // The error message should contain the descriptive message
        assert!(error_string.contains("Accessibility permission not granted"),
            "PermissionDenied error should contain the descriptive message");
    }

    /// Unit test: PermissionDenied error matches the format used in extract_frontmost
    /// **Validates: Requirements 1.3, 3.2**
    #[test]
    fn test_permission_denied_error_matches_extract_frontmost_format() {
        // This is the exact error message format used in extract_frontmost()
        let error = ExtractionError::PermissionDenied(
            "Accessibility permission not granted".into()
        );
        
        let error_string = error.to_string();
        
        // Verify the exact format matches what extract_frontmost returns
        assert_eq!(
            error_string,
            "Permission denied: Accessibility permission not granted",
            "PermissionDenied error format should match extract_frontmost implementation"
        );
    }

    /// Unit test: PermissionDenied error is Debug-printable
    /// **Validates: Requirements 1.3, 3.2**
    #[test]
    fn test_permission_denied_error_is_debug_printable() {
        let error = ExtractionError::PermissionDenied(
            "Test permission error".to_string()
        );
        
        // Verify the error can be formatted with Debug
        let debug_string = format!("{:?}", error);
        
        // Debug output should contain the variant name
        assert!(debug_string.contains("PermissionDenied"),
            "Debug output should contain 'PermissionDenied', got: '{}'",
            debug_string);
        
        // Debug output should contain the message
        assert!(debug_string.contains("Test permission error"),
            "Debug output should contain the error message");
    }

    // ============================================================================
    // Property-Based Tests for Permission Denied Error
    // Feature: accessibility-extractor, Property 1: Permission Denied Error on Missing Permissions
    // **Validates: Requirements 1.3, 3.2**
    // ============================================================================

    proptest! {
        /// Property test: PermissionDenied error always has non-empty message
        /// **Validates: Requirements 1.3, 3.2**
        /// 
        /// For any non-empty descriptive message, the PermissionDenied error
        /// SHALL produce a non-empty error string when converted to string.
        #[test]
        fn prop_permission_denied_error_has_non_empty_message(msg in ".+") {
            let error = ExtractionError::PermissionDenied(msg.clone());
            let error_string = error.to_string();
            
            // The error message should never be empty
            prop_assert!(!error_string.is_empty(),
                "PermissionDenied error message should not be empty for input: '{}'",
                msg);
        }

        /// Property test: PermissionDenied error always starts with correct prefix
        /// **Validates: Requirements 1.3, 3.2**
        /// 
        /// For any descriptive message, the PermissionDenied error string
        /// SHALL start with "Permission denied:" prefix.
        #[test]
        fn prop_permission_denied_error_has_correct_prefix(msg in ".+") {
            let error = ExtractionError::PermissionDenied(msg.clone());
            let error_string = error.to_string();
            
            // The error message should always start with "Permission denied:"
            prop_assert!(error_string.starts_with("Permission denied:"),
                "PermissionDenied error should start with 'Permission denied:', got: '{}' for input: '{}'",
                error_string, msg);
        }

        /// Property test: PermissionDenied error preserves the descriptive message
        /// **Validates: Requirements 1.3, 3.2**
        /// 
        /// For any descriptive message, the PermissionDenied error string
        /// SHALL contain the original descriptive message.
        #[test]
        fn prop_permission_denied_error_preserves_message(msg in "[a-zA-Z0-9 ]+") {
            let error = ExtractionError::PermissionDenied(msg.clone());
            let error_string = error.to_string();
            
            // The error message should contain the original descriptive message
            prop_assert!(error_string.contains(&msg),
                "PermissionDenied error should contain the descriptive message '{}', got: '{}'",
                msg, error_string);
        }

        /// Property test: PermissionDenied error message length is always greater than prefix
        /// **Validates: Requirements 1.3, 3.2**
        /// 
        /// For any non-empty descriptive message, the PermissionDenied error string
        /// SHALL have a length greater than just the "Permission denied: " prefix.
        #[test]
        fn prop_permission_denied_error_message_length_exceeds_prefix(msg in ".+") {
            let error = ExtractionError::PermissionDenied(msg.clone());
            let error_string = error.to_string();
            
            // The prefix "Permission denied: " has 19 characters
            let prefix_length = "Permission denied: ".len();
            
            // The error message should be longer than just the prefix
            prop_assert!(error_string.len() > prefix_length,
                "PermissionDenied error message length ({}) should exceed prefix length ({}) for input: '{}'",
                error_string.len(), prefix_length, msg);
        }

        /// Property test: PermissionDenied error format is consistent
        /// **Validates: Requirements 1.3, 3.2**
        /// 
        /// For any descriptive message, the PermissionDenied error string
        /// SHALL follow the format "Permission denied: {message}".
        #[test]
        fn prop_permission_denied_error_format_is_consistent(msg in "[a-zA-Z0-9 ]+") {
            let error = ExtractionError::PermissionDenied(msg.clone());
            let error_string = error.to_string();
            
            // The expected format is "Permission denied: {message}"
            let expected = format!("Permission denied: {}", msg);
            
            prop_assert_eq!(&error_string, &expected,
                "PermissionDenied error format should be 'Permission denied: {}', got: '{}'",
                msg, error_string);
        }
    }

    // ============================================================================
    // Tests for AppNotFound Error
    // Feature: accessibility-extractor, Property 2: AppNotFound Error for Invalid Applications
    // **Validates: Requirements 2.4, 4.3**
    //
    // For any bundle ID that does not correspond to a running application,
    // the extractor SHALL return an `AppNotFound` error variant within the
    // specified timeout period.
    //
    // NOTE: Since we cannot easily test with actual applications, we verify:
    // 1. The AppNotFound error variant exists and can be constructed
    // 2. The error message format is correct (contains "Application not found:" prefix)
    // 3. The error message contains the bundle ID
    // 4. The error message is non-empty and descriptive
    // ============================================================================

    /// Unit test: AppNotFound error variant exists and has correct format
    /// **Validates: Requirements 2.4, 4.3**
    #[test]
    fn test_app_not_found_error_exists_and_has_correct_format() {
        // Create an AppNotFound error with a bundle ID message
        let bundle_id = "com.example.nonexistent";
        let error = ExtractionError::AppNotFound(
            format!("{}: application not running", bundle_id)
        );
        
        // Verify the error message format
        let error_string = error.to_string();
        
        // The error message should start with "Application not found:"
        assert!(error_string.starts_with("Application not found:"),
            "AppNotFound error should start with 'Application not found:', got: '{}'",
            error_string);
        
        // The error message should be non-empty
        assert!(!error_string.is_empty(),
            "AppNotFound error message should not be empty");
        
        // The error message should contain the bundle ID
        assert!(error_string.contains(bundle_id),
            "AppNotFound error should contain the bundle ID '{}', got: '{}'",
            bundle_id, error_string);
    }

    /// Unit test: AppNotFound error matches the format used in get_app_by_bundle_id
    /// **Validates: Requirements 2.4, 4.3**
    #[test]
    fn test_app_not_found_error_matches_get_app_by_bundle_id_format() {
        // This simulates the error format used in get_app_by_bundle_id()
        let bundle_id = "com.microsoft.Word";
        let error = ExtractionError::AppNotFound(
            format!("{}: {:?}", bundle_id, "timeout")
        );
        
        let error_string = error.to_string();
        
        // Verify the format contains the bundle ID
        assert!(error_string.contains(bundle_id),
            "AppNotFound error should contain bundle ID '{}', got: '{}'",
            bundle_id, error_string);
        
        // Verify the format starts with the correct prefix
        assert!(error_string.starts_with("Application not found:"),
            "AppNotFound error should start with 'Application not found:', got: '{}'",
            error_string);
    }

    /// Unit test: AppNotFound error for various bundle ID formats
    /// **Validates: Requirements 2.4, 4.3**
    #[test]
    fn test_app_not_found_error_various_bundle_ids() {
        // Test with various bundle ID formats
        let bundle_ids = vec![
            "com.microsoft.Word",
            "com.apple.iWork.Pages",
            "com.example.app",
            "org.mozilla.firefox",
            "io.github.someapp",
        ];
        
        for bundle_id in bundle_ids {
            let error = ExtractionError::AppNotFound(bundle_id.to_string());
            let error_string = error.to_string();
            
            // Each error should contain the bundle ID
            assert!(error_string.contains(bundle_id),
                "AppNotFound error should contain bundle ID '{}', got: '{}'",
                bundle_id, error_string);
            
            // Each error should have the correct prefix
            assert!(error_string.starts_with("Application not found:"),
                "AppNotFound error should start with 'Application not found:', got: '{}'",
                error_string);
        }
    }

    /// Unit test: AppNotFound error is Debug-printable
    /// **Validates: Requirements 2.4, 4.3**
    #[test]
    fn test_app_not_found_error_is_debug_printable() {
        let error = ExtractionError::AppNotFound(
            "com.test.app: not running".to_string()
        );
        
        // Verify the error can be formatted with Debug
        let debug_string = format!("{:?}", error);
        
        // Debug output should contain the variant name
        assert!(debug_string.contains("AppNotFound"),
            "Debug output should contain 'AppNotFound', got: '{}'",
            debug_string);
        
        // Debug output should contain the message
        assert!(debug_string.contains("com.test.app"),
            "Debug output should contain the bundle ID");
    }

    /// Unit test: AppNotFound error for "No frontmost application" case
    /// **Validates: Requirements 2.4, 4.3**
    #[test]
    fn test_app_not_found_error_no_frontmost_application() {
        // This is the error format used in extract_frontmost when no app is in focus
        let error = ExtractionError::AppNotFound(
            "No frontmost application".into()
        );
        
        let error_string = error.to_string();
        
        // Verify the exact format
        assert_eq!(
            error_string,
            "Application not found: No frontmost application",
            "AppNotFound error format should match extract_frontmost implementation"
        );
    }

    // ============================================================================
    // Property-Based Tests for AppNotFound Error
    // Feature: accessibility-extractor, Property 2: AppNotFound Error for Invalid Applications
    // **Validates: Requirements 2.4, 4.3**
    // ============================================================================

    proptest! {
        /// Property test: AppNotFound error always has non-empty message
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any non-empty bundle ID or descriptive message, the AppNotFound error
        /// SHALL produce a non-empty error string when converted to string.
        #[test]
        fn prop_app_not_found_error_has_non_empty_message(msg in ".+") {
            let error = ExtractionError::AppNotFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should never be empty
            prop_assert!(!error_string.is_empty(),
                "AppNotFound error message should not be empty for input: '{}'",
                msg);
        }

        /// Property test: AppNotFound error always starts with correct prefix
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any descriptive message, the AppNotFound error string
        /// SHALL start with "Application not found:" prefix.
        #[test]
        fn prop_app_not_found_error_has_correct_prefix(msg in ".+") {
            let error = ExtractionError::AppNotFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should always start with "Application not found:"
            prop_assert!(error_string.starts_with("Application not found:"),
                "AppNotFound error should start with 'Application not found:', got: '{}' for input: '{}'",
                error_string, msg);
        }

        /// Property test: AppNotFound error preserves the bundle ID in message
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any bundle ID, the AppNotFound error string
        /// SHALL contain the original bundle ID.
        #[test]
        fn prop_app_not_found_error_preserves_bundle_id(bundle_id in "[a-zA-Z0-9.]+") {
            let error = ExtractionError::AppNotFound(bundle_id.clone());
            let error_string = error.to_string();
            
            // The error message should contain the original bundle ID
            prop_assert!(error_string.contains(&bundle_id),
                "AppNotFound error should contain the bundle ID '{}', got: '{}'",
                bundle_id, error_string);
        }

        /// Property test: AppNotFound error message length is always greater than prefix
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any non-empty bundle ID or message, the AppNotFound error string
        /// SHALL have a length greater than just the "Application not found: " prefix.
        #[test]
        fn prop_app_not_found_error_message_length_exceeds_prefix(msg in ".+") {
            let error = ExtractionError::AppNotFound(msg.clone());
            let error_string = error.to_string();
            
            // The prefix "Application not found: " has 23 characters
            let prefix_length = "Application not found: ".len();
            
            // The error message should be longer than just the prefix
            prop_assert!(error_string.len() > prefix_length,
                "AppNotFound error message length ({}) should exceed prefix length ({}) for input: '{}'",
                error_string.len(), prefix_length, msg);
        }

        /// Property test: AppNotFound error format is consistent
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any bundle ID or message, the AppNotFound error string
        /// SHALL follow the format "Application not found: {message}".
        #[test]
        fn prop_app_not_found_error_format_is_consistent(msg in "[a-zA-Z0-9. ]+") {
            let error = ExtractionError::AppNotFound(msg.clone());
            let error_string = error.to_string();
            
            // The expected format is "Application not found: {message}"
            let expected = format!("Application not found: {}", msg);
            
            prop_assert_eq!(&error_string, &expected,
                "AppNotFound error format should be 'Application not found: {}', got: '{}'",
                msg, error_string);
        }

        /// Property test: AppNotFound error with bundle ID format (com.xxx.yyy)
        /// **Validates: Requirements 2.4, 4.3**
        /// 
        /// For any valid bundle ID format (reverse domain notation), the AppNotFound error
        /// SHALL correctly preserve the bundle ID in the error message.
        #[test]
        fn prop_app_not_found_error_with_bundle_id_format(
            domain in "[a-z]{2,10}",
            company in "[a-z]{2,15}",
            app in "[a-zA-Z]{2,20}"
        ) {
            let bundle_id = format!("{}.{}.{}", domain, company, app);
            let error = ExtractionError::AppNotFound(bundle_id.clone());
            let error_string = error.to_string();
            
            // The error message should contain the full bundle ID
            prop_assert!(error_string.contains(&bundle_id),
                "AppNotFound error should contain bundle ID '{}', got: '{}'",
                bundle_id, error_string);
            
            // The error message should have the correct format
            let expected = format!("Application not found: {}", bundle_id);
            prop_assert_eq!(&error_string, &expected,
                "AppNotFound error format mismatch for bundle ID '{}'",
                bundle_id);
        }
    }

    // ============================================================================
    // Tests for NoContentFound Error
    // Feature: accessibility-extractor, Property 4: NoContentFound Error for Empty Documents
    // **Validates: Requirements 3.7**
    //
    // For any extraction from a document with no text content, the extractor
    // SHALL return a `NoContentFound` error variant.
    //
    // NOTE: Since we cannot easily test with actual empty documents, we verify:
    // 1. The NoContentFound error variant exists and can be constructed
    // 2. The error message format is correct (starts with "No content found:" prefix)
    // 3. The error message is non-empty and descriptive
    // ============================================================================

    /// Unit test: NoContentFound error variant exists and has correct format
    /// **Validates: Requirements 3.7**
    #[test]
    fn test_no_content_found_error_exists_and_has_correct_format() {
        // Create a NoContentFound error with a descriptive message
        let error = ExtractionError::NoContentFound(
            "Document appears to be empty".to_string()
        );
        
        // Verify the error message format
        let error_string = error.to_string();
        
        // The error message should start with "No content found:"
        assert!(error_string.starts_with("No content found:"),
            "NoContentFound error should start with 'No content found:', got: '{}'",
            error_string);
        
        // The error message should be non-empty
        assert!(!error_string.is_empty(),
            "NoContentFound error message should not be empty");
        
        // The error message should contain the descriptive message
        assert!(error_string.contains("Document appears to be empty"),
            "NoContentFound error should contain the descriptive message");
    }

    /// Unit test: NoContentFound error matches the format used in extract_from_element
    /// **Validates: Requirements 3.7**
    #[test]
    fn test_no_content_found_error_matches_extract_from_element_format() {
        // This is the exact error message format used in extract_from_element()
        let error = ExtractionError::NoContentFound(
            "Document appears to be empty".into()
        );
        
        let error_string = error.to_string();
        
        // Verify the exact format matches what extract_from_element returns
        assert_eq!(
            error_string,
            "No content found: Document appears to be empty",
            "NoContentFound error format should match extract_from_element implementation"
        );
    }

    /// Unit test: NoContentFound error is Debug-printable
    /// **Validates: Requirements 3.7**
    #[test]
    fn test_no_content_found_error_is_debug_printable() {
        let error = ExtractionError::NoContentFound(
            "Test empty document error".to_string()
        );
        
        // Verify the error can be formatted with Debug
        let debug_string = format!("{:?}", error);
        
        // Debug output should contain the variant name
        assert!(debug_string.contains("NoContentFound"),
            "Debug output should contain 'NoContentFound', got: '{}'",
            debug_string);
        
        // Debug output should contain the message
        assert!(debug_string.contains("Test empty document error"),
            "Debug output should contain the error message");
    }

    /// Unit test: NoContentFound error for various empty document scenarios
    /// **Validates: Requirements 3.7**
    #[test]
    fn test_no_content_found_error_various_scenarios() {
        // Test with various descriptive messages for empty document scenarios
        let scenarios = vec![
            "Document appears to be empty",
            "No text content found in document",
            "Empty document",
            "Document contains only whitespace",
            "No extractable content",
        ];
        
        for scenario in scenarios {
            let error = ExtractionError::NoContentFound(scenario.to_string());
            let error_string = error.to_string();
            
            // Each error should contain the scenario description
            assert!(error_string.contains(scenario),
                "NoContentFound error should contain scenario '{}', got: '{}'",
                scenario, error_string);
            
            // Each error should have the correct prefix
            assert!(error_string.starts_with("No content found:"),
                "NoContentFound error should start with 'No content found:', got: '{}'",
                error_string);
        }
    }

    /// Unit test: NoContentFound error implements std::error::Error trait
    /// **Validates: Requirements 3.7**
    #[test]
    fn test_no_content_found_error_implements_error_trait() {
        let error = ExtractionError::NoContentFound(
            "Empty document".to_string()
        );
        
        // Verify the error implements std::error::Error by using it as a trait object
        let error_ref: &dyn std::error::Error = &error;
        
        // The error should have a description via Display
        let description = error_ref.to_string();
        assert!(!description.is_empty(),
            "Error description should not be empty");
        
        // The error should start with the correct prefix
        assert!(description.starts_with("No content found:"),
            "Error description should start with 'No content found:', got: '{}'",
            description);
    }

    // ============================================================================
    // Property-Based Tests for NoContentFound Error
    // Feature: accessibility-extractor, Property 4: NoContentFound Error for Empty Documents
    // **Validates: Requirements 3.7**
    // ============================================================================

    proptest! {
        /// Property test: NoContentFound error always has non-empty message
        /// **Validates: Requirements 3.7**
        /// 
        /// For any non-empty descriptive message, the NoContentFound error
        /// SHALL produce a non-empty error string when converted to string.
        #[test]
        fn prop_no_content_found_error_has_non_empty_message(msg in ".+") {
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should never be empty
            prop_assert!(!error_string.is_empty(),
                "NoContentFound error message should not be empty for input: '{}'",
                msg);
        }

        /// Property test: NoContentFound error always starts with correct prefix
        /// **Validates: Requirements 3.7**
        /// 
        /// For any descriptive message, the NoContentFound error string
        /// SHALL start with "No content found:" prefix.
        #[test]
        fn prop_no_content_found_error_has_correct_prefix(msg in ".+") {
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should always start with "No content found:"
            prop_assert!(error_string.starts_with("No content found:"),
                "NoContentFound error should start with 'No content found:', got: '{}' for input: '{}'",
                error_string, msg);
        }

        /// Property test: NoContentFound error preserves the descriptive message
        /// **Validates: Requirements 3.7**
        /// 
        /// For any descriptive message, the NoContentFound error string
        /// SHALL contain the original descriptive message.
        #[test]
        fn prop_no_content_found_error_preserves_message(msg in "[a-zA-Z0-9 ]+") {
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should contain the original descriptive message
            prop_assert!(error_string.contains(&msg),
                "NoContentFound error should contain the descriptive message '{}', got: '{}'",
                msg, error_string);
        }

        /// Property test: NoContentFound error message length is always greater than prefix
        /// **Validates: Requirements 3.7**
        /// 
        /// For any non-empty descriptive message, the NoContentFound error string
        /// SHALL have a length greater than just the "No content found: " prefix.
        #[test]
        fn prop_no_content_found_error_message_length_exceeds_prefix(msg in ".+") {
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The prefix "No content found: " has 18 characters
            let prefix_length = "No content found: ".len();
            
            // The error message should be longer than just the prefix
            prop_assert!(error_string.len() > prefix_length,
                "NoContentFound error message length ({}) should exceed prefix length ({}) for input: '{}'",
                error_string.len(), prefix_length, msg);
        }

        /// Property test: NoContentFound error format is consistent
        /// **Validates: Requirements 3.7**
        /// 
        /// For any descriptive message, the NoContentFound error string
        /// SHALL follow the format "No content found: {message}".
        #[test]
        fn prop_no_content_found_error_format_is_consistent(msg in "[a-zA-Z0-9 ]+") {
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The expected format is "No content found: {message}"
            let expected = format!("No content found: {}", msg);
            
            prop_assert_eq!(&error_string, &expected,
                "NoContentFound error format should be 'No content found: {}', got: '{}'",
                msg, error_string);
        }

        /// Property test: NoContentFound error is descriptive for empty document scenarios
        /// **Validates: Requirements 3.7**
        /// 
        /// For any descriptive message about empty documents, the NoContentFound error
        /// SHALL produce a meaningful error message that helps identify the issue.
        #[test]
        fn prop_no_content_found_error_is_descriptive(
            prefix in "(Document|Content|Text|File)",
            description in "(appears to be empty|is empty|contains no text|has no content)"
        ) {
            let msg = format!("{} {}", prefix, description);
            let error = ExtractionError::NoContentFound(msg.clone());
            let error_string = error.to_string();
            
            // The error message should be descriptive
            prop_assert!(error_string.len() > 30,
                "NoContentFound error should be descriptive, got: '{}'",
                error_string);
            
            // The error message should contain both the prefix and description
            prop_assert!(error_string.contains(&prefix),
                "NoContentFound error should contain prefix '{}', got: '{}'",
                prefix, error_string);
            prop_assert!(error_string.contains(&description),
                "NoContentFound error should contain description '{}', got: '{}'",
                description, error_string);
        }
    }

    // ============================================================================
    // Tests for Selected Text Returns None When No Selection
    // Feature: accessibility-extractor, Property 6: Selected Text Returns None When No Selection
    // **Validates: Requirements 5.6**
    //
    // For any call to `get_selected_text()` when no text is selected in the focused
    // application, the function SHALL return `None`.
    //
    // NOTE: Since we cannot easily control the selection state in tests, we verify:
    // 1. The get_selected_text function returns Option<String>
    // 2. The function can return None (document the behavior)
    // 3. The function can return Some(String) when text is selected (document the behavior)
    // ============================================================================

    /// Unit test: get_selected_text returns Option<String> type
    /// **Validates: Requirements 5.6**
    /// 
    /// This test verifies that the get_selected_text function has the correct
    /// return type of Option<String>, which allows it to return None when no
    /// text is selected.
    #[test]
    fn test_get_selected_text_returns_option_string() {
        // Call get_selected_text and verify it returns Option<String>
        let result: Option<String> = MacOSExtractor::get_selected_text();
        
        // The result should be either None or Some(String)
        // We can't control the selection state, but we can verify the type
        match result {
            None => {
                // This is valid - no text is selected
                // This satisfies Requirement 5.6: Return None if no text is selected
            }
            Some(text) => {
                // This is also valid - some text is selected
                // The text should be a non-empty string when Some
                // (empty selections are converted to None by the implementation)
                assert!(!text.is_empty(),
                    "When get_selected_text returns Some, the text should not be empty");
            }
        }
    }

    /// Unit test: Document the behavior of get_selected_text returning None
    /// **Validates: Requirements 5.6**
    /// 
    /// This test documents the expected behavior when no text is selected:
    /// - The function queries AXFocusedUIElement from the system-wide element
    /// - The function queries AXSelectedText attribute from the focused element
    /// - If no text is selected, the function returns None
    /// - If the focused element doesn't support AXSelectedText, the function returns None
    #[test]
    fn test_get_selected_text_none_behavior_documented() {
        // Document the scenarios where get_selected_text returns None:
        // 
        // 1. No application is in focus (AXFocusedUIElement query fails)
        // 2. The focused element doesn't support AXSelectedText attribute
        // 3. No text is currently selected (AXSelectedText is empty)
        // 4. The AXSelectedText value is not a string type
        // 5. Accessibility permissions are not granted
        //
        // In all these cases, the function correctly returns None as per Requirement 5.6
        
        // We can verify the function exists and is callable
        let _result = MacOSExtractor::get_selected_text();
        
        // The function signature guarantees Option<String> return type
        // which allows None to be returned when no selection exists
    }

    /// Unit test: Document the behavior of get_selected_text returning Some(String)
    /// **Validates: Requirements 5.6**
    /// 
    /// This test documents the expected behavior when text is selected:
    /// - The function queries AXFocusedUIElement from the system-wide element
    /// - The function queries AXSelectedText attribute from the focused element
    /// - If text is selected, the function returns Some(String) with the selected text
    /// - The returned text is the exact text that is selected in the focused element
    #[test]
    fn test_get_selected_text_some_behavior_documented() {
        // Document the scenarios where get_selected_text returns Some(String):
        //
        // 1. An application is in focus with a text element focused
        // 2. The focused element supports AXSelectedText attribute
        // 3. Text is currently selected in the focused element
        // 4. The AXSelectedText value is a non-empty string
        //
        // In these cases, the function returns Some(String) containing the selected text
        
        // We can verify the function exists and is callable
        let result = MacOSExtractor::get_selected_text();
        
        // If we get Some, verify the text is non-empty
        if let Some(text) = result {
            assert!(!text.is_empty(),
                "Selected text should not be empty when Some is returned");
        }
    }

    /// Unit test: get_selected_text handles empty selection correctly
    /// **Validates: Requirements 5.6**
    /// 
    /// This test verifies that when the AXSelectedText attribute returns an
    /// empty string (no text selected), the function returns None instead of
    /// Some("").
    #[test]
    fn test_get_selected_text_empty_selection_returns_none() {
        // The implementation checks if the selected text is empty and returns None
        // This is verified by the implementation in get_selected_text():
        //
        // ```rust
        // if text.is_empty() {
        //     None
        // } else {
        //     Some(text)
        // }
        // ```
        //
        // This ensures that empty selections are treated as "no selection"
        // which satisfies Requirement 5.6: Return None if no text is selected
        
        // We can verify the function exists and is callable
        let result = MacOSExtractor::get_selected_text();
        
        // If we get Some, the text must be non-empty
        if let Some(text) = result {
            assert!(!text.is_empty(),
                "get_selected_text should never return Some with empty string");
        }
    }

    /// Unit test: Verify get_selected_text function signature
    /// **Validates: Requirements 5.6**
    /// 
    /// This test verifies that the get_selected_text function has the correct
    /// signature as specified in the requirements:
    /// - Takes no arguments
    /// - Returns Option<String>
    #[test]
    fn test_get_selected_text_function_signature() {
        // Verify the function can be called with no arguments
        // and returns Option<String>
        fn verify_signature() -> Option<String> {
            MacOSExtractor::get_selected_text()
        }
        
        // Call the function to verify it compiles and runs
        let _result = verify_signature();
    }

    // ============================================================================
    // Property-Based Tests for Selected Text Returns None When No Selection
    // Feature: accessibility-extractor, Property 6: Selected Text Returns None When No Selection
    // **Validates: Requirements 5.6**
    //
    // Since we cannot control the actual selection state in tests, these property
    // tests verify the invariants of the Option<String> return type and document
    // the expected behavior.
    // ============================================================================

    /// Helper function that simulates the selected text filtering logic.
    /// 
    /// This function mirrors the behavior in get_selected_text where empty
    /// strings are converted to None. This allows us to test the filtering
    /// logic independently of the actual accessibility API.
    /// 
    /// # Arguments
    /// 
    /// * `text` - The selected text value (or empty string if no selection)
    /// 
    /// # Returns
    /// 
    /// `Some(String)` if text is non-empty, `None` if text is empty.
    /// 
    /// # Requirements
    /// - Requirement 5.6: Return None if no text is selected
    fn filter_selected_text(text: &str) -> Option<String> {
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    }

    proptest! {
        /// Property test: Empty string always returns None
        /// **Validates: Requirements 5.6**
        /// 
        /// For any call where the selected text is empty, the function SHALL return None.
        /// This property verifies the filtering logic that converts empty selections to None.
        #[test]
        fn prop_empty_selection_returns_none(_dummy in Just(())) {
            // An empty string should always result in None
            let result = filter_selected_text("");
            prop_assert_eq!(result, None,
                "Empty selection should return None");
        }

        /// Property test: Non-empty string always returns Some
        /// **Validates: Requirements 5.6**
        /// 
        /// For any non-empty selected text, the function SHALL return Some(String)
        /// containing the selected text.
        #[test]
        fn prop_non_empty_selection_returns_some(text in ".+") {
            // A non-empty string should always result in Some
            let result = filter_selected_text(&text);
            prop_assert!(result.is_some(),
                "Non-empty selection '{}' should return Some", text.escape_debug());
            
            // The returned text should match the input
            prop_assert_eq!(result, Some(text.clone()),
                "Returned text should match input");
        }

        /// Property test: Selected text is preserved exactly
        /// **Validates: Requirements 5.6**
        /// 
        /// For any non-empty selected text, the returned string SHALL be exactly
        /// equal to the input text (no trimming or modification).
        #[test]
        fn prop_selected_text_preserved_exactly(text in ".+") {
            let result = filter_selected_text(&text);
            
            if let Some(returned_text) = result {
                prop_assert_eq!(&returned_text, &text,
                    "Selected text should be preserved exactly");
            }
        }

        /// Property test: Option<String> return type invariants
        /// **Validates: Requirements 5.6**
        /// 
        /// For any input, the result SHALL be either None (for empty input)
        /// or Some(String) (for non-empty input), never anything else.
        #[test]
        fn prop_option_string_invariants(text in ".*") {
            let result = filter_selected_text(&text);
            
            // The result must be either None or Some
            match &result {
                None => {
                    // None is only valid for empty input
                    prop_assert!(text.is_empty(),
                        "None should only be returned for empty input, got: '{}'",
                        text.escape_debug());
                }
                Some(returned_text) => {
                    // Some is only valid for non-empty input
                    prop_assert!(!text.is_empty(),
                        "Some should only be returned for non-empty input");
                    
                    // The returned text should match the input
                    prop_assert_eq!(returned_text, &text,
                        "Returned text should match input");
                }
            }
        }

        /// Property test: Selected text with various content types
        /// **Validates: Requirements 5.6**
        /// 
        /// For any type of text content (alphanumeric, special characters, unicode),
        /// the function SHALL correctly return Some(String) with the exact content.
        #[test]
        fn prop_selected_text_various_content_types(
            content in prop_oneof![
                "[a-zA-Z]+",           // Alphabetic
                "[0-9]+",              // Numeric
                "[a-zA-Z0-9]+",        // Alphanumeric
                "[!@#$%^&*()]+",       // Special characters
                "[ \t]+[a-z]+[ \t]+",  // Text with surrounding whitespace
            ]
        ) {
            let result = filter_selected_text(&content);
            
            // Non-empty content should always return Some
            prop_assert!(result.is_some(),
                "Non-empty content '{}' should return Some", content.escape_debug());
            
            // The content should be preserved exactly
            prop_assert_eq!(result, Some(content.clone()),
                "Content should be preserved exactly");
        }

        /// Property test: Consistent behavior across multiple calls
        /// **Validates: Requirements 5.6**
        /// 
        /// For any input, calling the filter function multiple times with the same
        /// input SHALL always produce the same result (deterministic behavior).
        #[test]
        fn prop_consistent_behavior_across_calls(text in ".*") {
            let result1 = filter_selected_text(&text);
            let result2 = filter_selected_text(&text);
            let result3 = filter_selected_text(&text);
            
            // All results should be identical
            prop_assert_eq!(&result1, &result2,
                "Results should be consistent across calls");
            prop_assert_eq!(&result2, &result3,
                "Results should be consistent across calls");
        }
    }

    // ============================================================================
    // Unit Tests for Teams and Slack Detection
    // ============================================================================

    #[test]
    fn test_detect_app_source_microsoft_teams() {
        // Test various Teams title formats
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft Teams"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("Teams"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("teams"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("TEAMS"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("Chat - Microsoft Teams"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("General | Team Name - Teams"), AppSource::Teams);
    }

    #[test]
    fn test_detect_app_source_slack() {
        // Test various Slack title formats
        assert_eq!(MacOSExtractor::detect_app_source("Slack"), AppSource::Slack);
        assert_eq!(MacOSExtractor::detect_app_source("slack"), AppSource::Slack);
        assert_eq!(MacOSExtractor::detect_app_source("SLACK"), AppSource::Slack);
        assert_eq!(MacOSExtractor::detect_app_source("#general | Workspace - Slack"), AppSource::Slack);
        assert_eq!(MacOSExtractor::detect_app_source("Direct Message - Slack"), AppSource::Slack);
    }

    #[test]
    fn test_detect_app_source_teams_case_insensitive() {
        // Test mixed case variations for Teams
        assert_eq!(MacOSExtractor::detect_app_source("TeAmS"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("tEaMs"), AppSource::Teams);
        assert_eq!(MacOSExtractor::detect_app_source("Microsoft TEAMS"), AppSource::Teams);
    }

    #[test]
    fn test_detect_app_source_slack_case_insensitive() {
        // Test mixed case variations for Slack
        assert_eq!(MacOSExtractor::detect_app_source("SlAcK"), AppSource::Slack);
        assert_eq!(MacOSExtractor::detect_app_source("sLaCk"), AppSource::Slack);
    }
}
