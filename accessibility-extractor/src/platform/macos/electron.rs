//! Electron app support for macOS Accessibility API.
//!
//! Electron apps (Slack, Discord, VS Code, Teams, etc.) don't expose their DOM
//! through the standard macOS Accessibility API by default. This module provides
//! functionality to enable accessibility for Electron apps by setting the
//! `AXManualAccessibility` attribute.
//!
//! # Background
//!
//! Electron apps use Chromium to render web content inside a native window.
//! By default, this web content appears as a "black box" to accessibility tools.
//! Setting `AXManualAccessibility` to `true` on the application element enables
//! Chrome's accessibility tree, making the DOM content visible to screen readers
//! and accessibility APIs.
//!
//! # Slack-Specific Handling
//!
//! Slack requires special handling because:
//! 1. It takes time to load content, especially on first launch or after updates
//! 2. The accessibility tree may not be immediately populated after enabling
//! 3. Content readiness must be verified before extraction
//!
//! Use `prepare_slack()` for Slack-specific extraction with proper timeout handling.
//!
//! # References
//!
//! - [Electron Accessibility Documentation](https://www.electronjs.org/docs/latest/tutorial/accessibility)
//! - [Stack Overflow: macOS Accessibility Inspector does not work with Slack](https://stackoverflow.com/questions/73196518)

use accessibility_sys::{AXUIElementCreateApplication, AXUIElementSetAttributeValue, kAXErrorSuccess};
use core_foundation::base::{CFRelease, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::CFString;

use crate::types::ExtractionError;

/// Known Electron app bundle identifiers on macOS.
///
/// This list contains bundle IDs for popular Electron-based applications.
/// These apps require special handling to enable accessibility features.
///
/// # Adding New Apps
///
/// To add support for a new Electron app:
/// 1. Find the bundle ID using: `osascript -e 'id of app "App Name"'`
/// 2. Add the bundle ID to this list
/// 3. The app will automatically get accessibility enabled during extraction
const ELECTRON_BUNDLE_IDS: &[&str] = &[
    // Communication & Collaboration
    "com.tinyspeck.slackmacgap",      // Slack
    "com.microsoft.teams",            // Microsoft Teams (new)
    "com.microsoft.teams2",           // Microsoft Teams (newer version)
    "com.hnc.Discord",                // Discord
    "com.skype.skype",                // Skype (Electron version)
    
    // Development Tools
    "com.microsoft.VSCode",           // Visual Studio Code
    "com.microsoft.VSCodeInsiders",   // VS Code Insiders
    "com.github.atom",                // Atom (deprecated but still used)
    "com.visualstudio.code.oss",      // VS Code OSS
    "io.github.nickvision.cavalier",  // Cavalier
    
    // Productivity & Notes
    "notion.id",                      // Notion
    "com.notion.id",                  // Notion (alternative)
    "md.obsidian",                    // Obsidian
    "com.todoist.mac.Todoist",        // Todoist
    "com.linear",                     // Linear
    "com.clickup.desktop-app",        // ClickUp
    "com.asana.app",                  // Asana
    
    // Design Tools
    "com.figma.Desktop",              // Figma
    "com.framer.desktop",             // Framer
    
    // Media & Entertainment
    "com.spotify.client",             // Spotify
    "com.tidal.desktop",              // Tidal
    
    // Utilities
    "com.1password.1password",        // 1Password
    "com.bitwarden.desktop",          // Bitwarden
    "com.postmanlabs.mac",            // Postman
    "com.insomnia.app",               // Insomnia
    "com.mongodb.compass",            // MongoDB Compass
    "com.loom.desktop",               // Loom
    "com.krisp.krispMac",             // Krisp
    
    // Messaging
    "com.whatsapp.WhatsApp",          // WhatsApp Desktop
    "com.facebook.messenger",         // Facebook Messenger
    "org.nickvision.tubeconverter",   // Tube Converter
    "com.wire.Wire",                  // Wire
    "com.signal.Signal",              // Signal Desktop
    "org.nickvision.money",           // Money
    
    // Other Popular Electron Apps
    "com.twitch.twitch",              // Twitch
    "com.streamlabs.slobs",           // Streamlabs OBS
    "com.typora.typora",              // Typora
    "com.simplenote.Simplenote",      // Simplenote
    "com.trello.desktop",             // Trello
    "com.basecamp.basecamp3",         // Basecamp
    "com.airtable.desktop",           // Airtable
    "com.miro.desktop",               // Miro
];

/// Check if an application is an Electron app that requires special accessibility handling.
///
/// Electron apps don't expose their DOM through the standard macOS Accessibility API
/// by default. This function identifies known Electron apps by their bundle ID.
///
/// # Arguments
///
/// * `bundle_id` - The macOS bundle identifier (e.g., "com.tinyspeck.slackmacgap")
///
/// # Returns
///
/// `true` if the bundle ID matches a known Electron app, `false` otherwise.
///
/// # Examples
///
/// ```
/// use accessibility_extractor::platform::macos::electron::is_electron_app;
///
/// assert!(is_electron_app("com.tinyspeck.slackmacgap")); // Slack
/// assert!(is_electron_app("com.microsoft.VSCode"));      // VS Code
/// assert!(is_electron_app("com.hnc.Discord"));           // Discord
/// assert!(!is_electron_app("com.apple.Safari"));         // Not Electron
/// assert!(!is_electron_app("com.microsoft.Word"));       // Not Electron
/// ```
pub fn is_electron_app(bundle_id: &str) -> bool {
    ELECTRON_BUNDLE_IDS.contains(&bundle_id)
}

/// Enable accessibility for an Electron application.
///
/// This function sets the `AXManualAccessibility` attribute to `true` on the
/// specified application, which enables Chrome's accessibility tree and makes
/// the web content visible to accessibility APIs.
///
/// # Arguments
///
/// * `pid` - The process ID of the Electron application
///
/// # Returns
///
/// `Ok(())` if accessibility was successfully enabled, or an error if:
/// - The application reference could not be created
/// - Setting the attribute failed
///
/// # Safety
///
/// This function uses unsafe code to interact with the macOS Accessibility API.
/// It properly manages memory by releasing the application reference after use.
///
/// # Example
///
/// ```no_run
/// use accessibility_extractor::platform::macos::electron::enable_electron_accessibility;
///
/// // Enable accessibility for a process with PID 12345
/// match enable_electron_accessibility(12345) {
///     Ok(()) => println!("Accessibility enabled!"),
///     Err(e) => eprintln!("Failed to enable accessibility: {}", e),
/// }
/// ```
pub fn enable_electron_accessibility(pid: i32) -> Result<(), ExtractionError> {
    log::debug!("[AX-ELECTRON] Enabling accessibility for PID: {}", pid);
    
    unsafe {
        // Create an AXUIElement reference for the application
        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            log::error!("[AX-ELECTRON] Failed to create app reference for PID: {}", pid);
            return Err(ExtractionError::AppNotFound(
                format!("Could not create accessibility reference for PID {}", pid)
            ));
        }
        
        // Set AXManualAccessibility to true
        // This enables Chrome's accessibility tree in Electron apps
        let attr_name = CFString::new("AXManualAccessibility");
        let result = AXUIElementSetAttributeValue(
            app_ref,
            attr_name.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef()
        );
        
        // Release the app reference to avoid memory leaks
        CFRelease(app_ref as *const _);
        
        if result == kAXErrorSuccess {
            log::info!("[AX-ELECTRON] ✓ Accessibility enabled for PID: {}", pid);
            Ok(())
        } else {
            // Map common error codes to meaningful messages
            let error_msg = match result {
                -25200 => "Accessibility permission denied",
                -25201 => "Action not supported",
                -25202 => "Attribute not settable",
                -25203 => "Attribute not supported",
                -25204 => "Invalid UI element",
                -25205 => "Cannot complete action",
                -25211 => "Not implemented",
                _ => "Unknown error",
            };
            
            log::warn!(
                "[AX-ELECTRON] ⚠️ Failed to enable accessibility for PID {}: {} (error {})",
                pid, error_msg, result
            );
            
            // Don't fail hard - the app might still work without this
            // Some apps may not support this attribute but still expose content
            Err(ExtractionError::AccessibilityError(
                format!("Failed to set AXManualAccessibility: {} (error {})", error_msg, result)
            ))
        }
    }
}

/// Get the process ID for a running application by its bundle ID.
///
/// This function searches through running applications to find one matching
/// the specified bundle identifier and returns its process ID.
///
/// # Arguments
///
/// * `bundle_id` - The macOS bundle identifier to search for
///
/// # Returns
///
/// `Some(pid)` if a running application with the bundle ID is found,
/// `None` if no matching application is running.
///
/// # Example
///
/// ```no_run
/// use accessibility_extractor::platform::macos::electron::get_pid_for_bundle_id;
///
/// if let Some(pid) = get_pid_for_bundle_id("com.tinyspeck.slackmacgap") {
///     println!("Slack is running with PID: {}", pid);
/// } else {
///     println!("Slack is not running");
/// }
/// ```
pub fn get_pid_for_bundle_id(bundle_id: &str) -> Option<i32> {
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::Object;
    
    unsafe {
        // Get the shared workspace: [NSWorkspace sharedWorkspace]
        let ns_workspace_class = class!(NSWorkspace);
        let workspace: *mut Object = msg_send![ns_workspace_class, sharedWorkspace];
        
        if workspace.is_null() {
            log::warn!("[AX-ELECTRON] Failed to get NSWorkspace");
            return None;
        }
        
        // Get all running applications: [workspace runningApplications]
        let running_apps: *mut Object = msg_send![workspace, runningApplications];
        
        if running_apps.is_null() {
            log::warn!("[AX-ELECTRON] Failed to get running applications");
            return None;
        }
        
        // Get the count of running applications
        let count: usize = msg_send![running_apps, count];
        
        for i in 0..count {
            // Get the app at index i: [runningApps objectAtIndex:i]
            let app: *mut Object = msg_send![running_apps, objectAtIndex: i];
            if app.is_null() {
                continue;
            }
            
            // Get the bundle identifier: [app bundleIdentifier]
            let app_bundle_id: *mut Object = msg_send![app, bundleIdentifier];
            if app_bundle_id.is_null() {
                continue;
            }
            
            // Convert NSString to C string: [bundleIdentifier UTF8String]
            let c_str: *const std::os::raw::c_char = msg_send![app_bundle_id, UTF8String];
            if c_str.is_null() {
                continue;
            }
            
            let rust_str = std::ffi::CStr::from_ptr(c_str).to_string_lossy();
            
            if rust_str == bundle_id {
                // Get the process identifier: [app processIdentifier]
                let pid: i32 = msg_send![app, processIdentifier];
                log::debug!("[AX-ELECTRON] Found {} with PID: {}", bundle_id, pid);
                return Some(pid);
            }
        }
        
        log::debug!("[AX-ELECTRON] No running app found for bundle ID: {}", bundle_id);
        None
    }
}

/// Prepare an Electron app for accessibility extraction.
///
/// This is a convenience function that combines `is_electron_app`, `get_pid_for_bundle_id`,
/// and `enable_electron_accessibility` into a single call. It's safe to call on any
/// bundle ID - non-Electron apps will be skipped.
///
/// # Arguments
///
/// * `bundle_id` - The macOS bundle identifier of the application
///
/// # Returns
///
/// `Ok(true)` if accessibility was enabled for an Electron app,
/// `Ok(false)` if the app is not an Electron app (no action needed),
/// `Err` if enabling accessibility failed.
///
/// # Example
///
/// ```no_run
/// use accessibility_extractor::platform::macos::electron::prepare_electron_app;
///
/// // Safe to call on any app
/// match prepare_electron_app("com.tinyspeck.slackmacgap") {
///     Ok(true) => println!("Electron accessibility enabled"),
///     Ok(false) => println!("Not an Electron app, no action needed"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn prepare_electron_app(bundle_id: &str) -> Result<bool, ExtractionError> {
    if !is_electron_app(bundle_id) {
        return Ok(false);
    }
    
    log::info!("[AX-ELECTRON] 🔧 Detected Electron app: {}", bundle_id);
    
    let pid = get_pid_for_bundle_id(bundle_id).ok_or_else(|| {
        ExtractionError::AppNotFound(format!("Electron app {} is not running", bundle_id))
    })?;
    
    enable_electron_accessibility(pid)?;
    
    // Give the accessibility tree time to populate
    // Electron apps need a moment after enabling accessibility
    std::thread::sleep(std::time::Duration::from_millis(150));
    
    Ok(true)
}

// ============================================================================
// Slack-Specific Extraction
// ============================================================================

/// Slack bundle identifier constant.
pub const SLACK_BUNDLE_ID: &str = "com.tinyspeck.slackmacgap";

/// Configuration for Slack content readiness polling.
#[derive(Debug, Clone)]
pub struct SlackExtractionConfig {
    /// Maximum time to wait for content to be ready (default: 10 seconds)
    pub max_wait_time: std::time::Duration,
    /// Initial delay before first content check (default: 500ms)
    pub initial_delay: std::time::Duration,
    /// Maximum delay between retries (default: 2 seconds)
    pub max_retry_delay: std::time::Duration,
    /// Minimum content length to consider extraction successful (default: 50 chars)
    pub min_content_length: usize,
}

impl Default for SlackExtractionConfig {
    fn default() -> Self {
        Self {
            max_wait_time: std::time::Duration::from_secs(10),
            initial_delay: std::time::Duration::from_millis(500),
            max_retry_delay: std::time::Duration::from_secs(2),
            min_content_length: 50,
        }
    }
}

/// Check if the given bundle ID is Slack.
pub fn is_slack(bundle_id: &str) -> bool {
    bundle_id == SLACK_BUNDLE_ID
}

/// Prepare Slack for accessibility extraction with extended timeout.
///
/// Slack requires special handling because:
/// 1. It's an Electron app that needs AXManualAccessibility enabled
/// 2. It takes time to load content, especially on first launch or after updates
/// 3. The accessibility tree may not be immediately populated
///
/// This function enables accessibility and waits for the content to be ready
/// using exponential backoff polling.
///
/// # Arguments
///
/// * `config` - Optional configuration for timeout and retry behavior
///
/// # Returns
///
/// `Ok(true)` if Slack is ready for extraction,
/// `Ok(false)` if Slack is not running,
/// `Err` if preparation failed after all retries.
pub fn prepare_slack(config: Option<SlackExtractionConfig>) -> Result<bool, ExtractionError> {
    let config = config.unwrap_or_default();
    
    log::info!("[AX-SLACK] 🔧 Preparing Slack for accessibility extraction...");
    
    // Get Slack's PID
    let pid = match get_pid_for_bundle_id(SLACK_BUNDLE_ID) {
        Some(pid) => pid,
        None => {
            log::debug!("[AX-SLACK] Slack is not running");
            return Ok(false);
        }
    };
    
    log::debug!("[AX-SLACK] Found Slack with PID: {}", pid);
    
    // Enable accessibility for Slack
    match enable_electron_accessibility(pid) {
        Ok(()) => {
            log::info!("[AX-SLACK] ✓ Accessibility enabled for Slack");
        }
        Err(e) => {
            // Log but continue - Slack might still work
            log::warn!("[AX-SLACK] ⚠️ Could not enable accessibility: {}", e);
        }
    }
    
    // Wait for content to be ready with exponential backoff
    wait_for_slack_content(pid, &config)
}

/// Wait for Slack's accessibility content to be ready.
///
/// This function polls the accessibility tree until content is available
/// or the timeout is reached. It uses exponential backoff to avoid
/// excessive CPU usage while still being responsive.
fn wait_for_slack_content(pid: i32, config: &SlackExtractionConfig) -> Result<bool, ExtractionError> {
    use std::time::Instant;
    
    let start_time = Instant::now();
    let mut current_delay = config.initial_delay;
    let mut attempt = 0;
    
    // Initial delay to let accessibility tree start building
    log::debug!("[AX-SLACK] Waiting {}ms for initial accessibility tree...", config.initial_delay.as_millis());
    std::thread::sleep(config.initial_delay);
    
    loop {
        attempt += 1;
        let elapsed = start_time.elapsed();
        
        if elapsed >= config.max_wait_time {
            log::warn!(
                "[AX-SLACK] ⚠️ Timeout waiting for Slack content after {:?} ({} attempts)",
                elapsed, attempt
            );
            // Return Ok(true) anyway - let the main extraction try
            // The content might be partially available
            return Ok(true);
        }
        
        log::debug!("[AX-SLACK] Checking content readiness (attempt {}, elapsed {:?})...", attempt, elapsed);
        
        // Check if Slack has meaningful content
        match check_slack_content_ready(pid, config.min_content_length) {
            ContentReadiness::Ready(content_len) => {
                log::info!(
                    "[AX-SLACK] ✓ Slack content ready ({} chars) after {:?} ({} attempts)",
                    content_len, elapsed, attempt
                );
                return Ok(true);
            }
            ContentReadiness::Loading(reason) => {
                log::debug!("[AX-SLACK] Content not ready: {}", reason);
            }
            ContentReadiness::Error(e) => {
                log::warn!("[AX-SLACK] Error checking content: {}", e);
            }
        }
        
        // Exponential backoff with cap
        std::thread::sleep(current_delay);
        current_delay = std::cmp::min(current_delay * 2, config.max_retry_delay);
    }
}

/// Result of checking Slack content readiness.
#[derive(Debug)]
#[allow(dead_code)]
enum ContentReadiness {
    /// Content is ready with the given length
    Ready(usize),
    /// Content is still loading with reason
    Loading(String),
    /// Error occurred while checking (reserved for future use)
    Error(String),
}

/// Check if Slack's accessibility content is ready for extraction.
///
/// This function looks for specific indicators that Slack has loaded:
/// - AXWebArea elements (the main content area)
/// - Sufficient text content in the accessibility tree
fn check_slack_content_ready(pid: i32, min_content_length: usize) -> ContentReadiness {
    use accessibility::AXUIElement;
    use accessibility::AXUIElementAttributes;
    
    // Create application reference
    let app = AXUIElement::application(pid);
    
    // Try to get the focused window
    let window = match app.focused_window() {
        Ok(w) => w,
        Err(e) => {
            return ContentReadiness::Loading(format!("No focused window: {:?}", e));
        }
    };
    
    // Look for AXWebArea elements (Slack's main content)
    let web_areas = find_web_areas(&window, 0);
    
    if web_areas.is_empty() {
        return ContentReadiness::Loading("No AXWebArea elements found".to_string());
    }
    
    // Check if any web area has meaningful content
    let mut total_content_len = 0;
    for web_area in &web_areas {
        let content_len = estimate_content_length(web_area, 0);
        total_content_len += content_len;
    }
    
    if total_content_len >= min_content_length {
        ContentReadiness::Ready(total_content_len)
    } else {
        ContentReadiness::Loading(format!(
            "Content too short: {} chars (need {})",
            total_content_len, min_content_length
        ))
    }
}

/// Find all AXWebArea elements in the accessibility tree.
fn find_web_areas(element: &accessibility::AXUIElement, depth: usize) -> Vec<accessibility::AXUIElement> {
    use accessibility::AXUIElementAttributes;
    
    // Limit depth to prevent excessive traversal
    if depth > 20 {
        return Vec::new();
    }
    
    let mut results = Vec::new();
    
    // Check if this element is an AXWebArea
    if let Ok(role) = element.role() {
        if role.to_string() == "AXWebArea" {
            results.push(element.clone());
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                results.extend(find_web_areas(&child, depth + 1));
            }
        }
    }
    
    results
}

/// Estimate the content length in an accessibility element tree.
fn estimate_content_length(element: &accessibility::AXUIElement, depth: usize) -> usize {
    use accessibility::AXUIElementAttributes;
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    
    // Limit depth
    if depth > 30 {
        return 0;
    }
    
    let mut total = 0;
    
    // Check for text value
    if let Ok(value) = element.value() {
        let type_id = value.type_of();
        if type_id == CFString::type_id() {
            let ptr = value.as_CFTypeRef();
            let cf_string: CFString = unsafe {
                CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
            };
            let text = cf_string.to_string();
            let trimmed_len = text.trim().len();
            if trimmed_len > 0 {
                total += trimmed_len;
            }
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                total += estimate_content_length(&child, depth + 1);
            }
        }
    }
    
    total
}

/// A parsed Slack message with author, date, and content.
#[derive(Debug, Clone)]
pub struct SlackMessage {
    pub author: String,
    pub date: String,
    pub content: String,
}

impl SlackMessage {
    /// Format the message as [Author] [Date] Message
    pub fn format(&self) -> String {
        format!("[{}] [{}] {}", self.author, self.date, self.content)
    }
}

/// Extract text content from Slack's accessibility tree.
///
/// Slack uses a different structure than typical document apps:
/// - Messages are in AXStaticText elements inside AXList/AXGroup/AXRow
/// - We need to traverse through AXList and AXOutline (normally filtered as UI chrome)
/// - Focus on the AXWebArea which contains the main content
///
/// Messages are formatted as: [Author] [Date] Message
///
/// # Arguments
///
/// * `app` - The Slack application AXUIElement
///
/// # Returns
///
/// Extracted text content from Slack messages, formatted with author and date.
pub fn extract_slack_content(app: &accessibility::AXUIElement) -> String {
    use accessibility::AXUIElementAttributes;
    
    // Get the focused window
    let window = match app.focused_window() {
        Ok(w) => w,
        Err(_) => return String::new(),
    };
    
    // Find the AXWebArea (main content area)
    let web_areas = find_web_areas(&window, 0);
    
    let mut messages: Vec<SlackMessage> = Vec::new();
    
    for web_area in web_areas {
        extract_slack_messages_recursive(&web_area, &mut messages, 0);
    }
    
    // Deduplicate messages by content (Slack may have duplicate elements)
    let mut seen = std::collections::HashSet::new();
    let unique_messages: Vec<&SlackMessage> = messages
        .iter()
        .filter(|m| {
            let key = format!("{}:{}:{}", m.author, m.date, m.content);
            if seen.contains(&key) {
                false
            } else {
                seen.insert(key);
                true
            }
        })
        .collect();
    
    // Format all messages
    unique_messages
        .iter()
        .map(|m| m.format())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Recursively extract messages from Slack's accessibility tree.
///
/// This function parses the AXGroup.AXTitle attribute which contains structured message info:
/// Format: "Author: message text. timestamp" or similar patterns
///
/// It also looks for:
/// - AXButton with author name
/// - AXLink with date/timestamp
/// - AXStaticText with message content
fn extract_slack_messages_recursive(
    element: &accessibility::AXUIElement,
    messages: &mut Vec<SlackMessage>,
    depth: usize,
) {
    use accessibility::AXUIElementAttributes;
    
    // Limit depth
    if depth > 100 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Skip menu-related UI chrome
    if matches!(role.as_str(), 
        "AXMenuBar" | "AXMenu" | "AXMenuItem" | "AXToolbar" | 
        "AXTabGroup" | "AXTab" | "AXSlider" | "AXSplitter"
    ) {
        return;
    }
    
    // Check AXGroup elements - they often contain the full message info in AXTitle
    // Format: "Author Name: message content. timestamp"
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if let Some(msg) = parse_slack_message_from_title(&title_str) {
                messages.push(msg);
                // Don't return - continue traversing for other messages
            }
        }
    }
    
    // Also check AXCell elements which may contain message info
    if role == "AXCell" || role == "AXRow" {
        // Try to extract message from this cell/row by looking at children
        if let Some(msg) = extract_message_from_container(element) {
            messages.push(msg);
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                extract_slack_messages_recursive(&child, messages, depth + 1);
            }
        }
    }
}

/// Parse a Slack message from an AXGroup's AXTitle attribute.
///
/// The title format is typically: "Author Name: message content. Image: No alt text. timestamp"
/// or "Author Name: message content. timestamp"
fn parse_slack_message_from_title(title: &str) -> Option<SlackMessage> {
    let title = title.trim();
    
    // Skip if too short or looks like UI text
    if title.len() < 10 || is_slack_ui_text(title) {
        return None;
    }
    
    // Look for the pattern "Author: message. timestamp"
    // The colon after the author name is the key separator
    let colon_pos = title.find(':')?;
    
    // Author is before the first colon
    let author = title[..colon_pos].trim();
    
    // Skip if author looks like a UI label
    if author.is_empty() || is_slack_ui_text(author) {
        return None;
    }
    
    // The rest contains message and timestamp
    let rest = title[colon_pos + 1..].trim();
    
    // Try to find timestamp at the end (formats like "9:41 AM", "Jan 31st at 9:41:50 AM", etc.)
    let (content, date) = extract_timestamp_from_end(rest);
    
    // Skip if content is empty or too short
    let content = content.trim();
    if content.is_empty() || content.len() < 3 {
        return None;
    }
    
    // Clean up content - remove "Image: No alt text." and similar artifacts
    let content = clean_message_content(content);
    
    if content.is_empty() {
        return None;
    }
    
    Some(SlackMessage {
        author: author.to_string(),
        date: date.unwrap_or_else(|| "Unknown".to_string()),
        content,
    })
}

/// Extract timestamp from the end of a message string.
///
/// Returns (content_without_timestamp, Some(timestamp)) or (original_content, None)
fn extract_timestamp_from_end(text: &str) -> (&str, Option<String>) {
    // Common timestamp patterns at the end of Slack messages:
    // "message content. 10:59 AM."
    // "message content. 4:10 PM. 1 link."
    // "message content. 7:28 PM. 1 reaction."
    // "message content 10:59 AM" (no trailing period)
    // Long messages might have the time without a preceding period
    
    // Pattern 1: Time with period before and after, possibly with reaction/link counts
    let pattern1 = regex_lite::Regex::new(
        r"[.\s]+(\d{1,2}:\d{2}\s*(?:AM|PM))\.(?:\s*\d+\s*(?:reaction|link|file|reply|replies|attachment)s?\.?)*\s*$"
    );
    
    if let Ok(pattern) = pattern1 {
        if let Some(caps) = pattern.captures(text) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                if let Some(full_match) = caps.get(0) {
                    let content = text[..full_match.start()].trim();
                    return (content, Some(time));
                }
            }
        }
    }
    
    // Pattern 2: Time at end with just a period before (simpler)
    let pattern2 = regex_lite::Regex::new(r"\.\s*(\d{1,2}:\d{2}\s*(?:AM|PM))\.\s*$");
    if let Ok(pattern) = pattern2 {
        if let Some(caps) = pattern.captures(text) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                if let Some(full_match) = caps.get(0) {
                    let content = text[..full_match.start()].trim();
                    return (content, Some(time));
                }
            }
        }
    }
    
    // Pattern 3: Time at end without trailing period (for longer messages)
    let pattern3 = regex_lite::Regex::new(r"[.\s]+(\d{1,2}:\d{2}\s*(?:AM|PM))\s*$");
    if let Ok(pattern) = pattern3 {
        if let Some(caps) = pattern.captures(text) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                if let Some(full_match) = caps.get(0) {
                    let content = text[..full_match.start()].trim();
                    return (content, Some(time));
                }
            }
        }
    }
    
    // Pattern 4: Just look for any time pattern near the end (last 30 chars)
    if text.len() > 30 {
        let end_portion = &text[text.len() - 30..];
        let pattern4 = regex_lite::Regex::new(r"(\d{1,2}:\d{2}\s*(?:AM|PM))");
        if let Ok(pattern) = pattern4 {
            if let Some(caps) = pattern.captures(end_portion) {
                if let Some(time_match) = caps.get(1) {
                    let time = time_match.as_str().trim().to_string();
                    // Find this time in the original text and split there
                    if let Some(pos) = text.rfind(&time) {
                        // Find the start of the time section (look for period or space before)
                        let mut start = pos;
                        while start > 0 && (text.as_bytes()[start - 1] == b'.' || text.as_bytes()[start - 1] == b' ') {
                            start -= 1;
                        }
                        let content = text[..start].trim();
                        if !content.is_empty() {
                            return (content, Some(time));
                        }
                    }
                }
            }
        }
    }
    
    (text, None)
}

/// Clean up message content by removing artifacts.
fn clean_message_content(content: &str) -> String {
    let mut result = content.to_string();
    
    // Remove "Image: No alt text." and similar
    let artifacts = [
        "Image: No alt text.",
        "Image:",
        "Attachment:",
        "File:",
    ];
    
    for artifact in artifacts {
        result = result.replace(artifact, "");
    }
    
    // Clean up multiple spaces and trim
    let parts: Vec<&str> = result.split_whitespace().collect();
    parts.join(" ")
}

/// Try to extract a message from a container element (AXCell, AXRow) by examining children.
fn extract_message_from_container(element: &accessibility::AXUIElement) -> Option<SlackMessage> {
    let mut author: Option<String> = None;
    let mut date: Option<String> = None;
    let mut content_parts: Vec<String> = Vec::new();
    
    // Recursively collect info from children
    collect_message_parts(element, &mut author, &mut date, &mut content_parts, 0);
    
    // Need at least author and some content
    let author = author?;
    if content_parts.is_empty() {
        return None;
    }
    
    let content = content_parts.join(" ");
    if content.trim().is_empty() || content.len() < 3 {
        return None;
    }
    
    Some(SlackMessage {
        author,
        date: date.unwrap_or_else(|| "Unknown".to_string()),
        content,
    })
}

/// Collect message parts (author, date, content) from element children.
fn collect_message_parts(
    element: &accessibility::AXUIElement,
    author: &mut Option<String>,
    date: &mut Option<String>,
    content_parts: &mut Vec<String>,
    depth: usize,
) {
    use accessibility::AXUIElementAttributes;
    use accessibility::attribute::AXAttribute;
    use core_foundation::base::CFType;
    use core_foundation::string::CFString;
    
    if depth > 10 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // AXButton often contains author name
    if role == "AXButton" && author.is_none() {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if !title_str.is_empty() && !is_slack_ui_text(&title_str) && title_str.len() > 2 {
                *author = Some(title_str);
            }
        }
    }
    
    // AXLink often contains date/timestamp
    if role == "AXLink" && date.is_none() {
        // Check AXDescription for date
        let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
        if let Ok(value) = element.attribute(&desc_attr) {
            if let Some(text) = cftype_to_string_local(&value) {
                if looks_like_timestamp(&text) {
                    *date = Some(text);
                }
            }
        }
    }
    
    // AXStaticText contains message content
    if role == "AXStaticText" {
        if let Ok(value) = element.value() {
            if let Some(text) = cftype_to_string_local(&value) {
                let trimmed = text.trim();
                if !trimmed.is_empty() && trimmed.len() > 2 && !is_slack_ui_text(trimmed) {
                    content_parts.push(trimmed.to_string());
                }
            }
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                collect_message_parts(&child, author, date, content_parts, depth + 1);
            }
        }
    }
}

/// Check if text looks like a timestamp.
fn looks_like_timestamp(text: &str) -> bool {
    let text = text.trim();
    
    // Check for common timestamp patterns
    text.contains("AM") || text.contains("PM") ||
    text.contains("ago") ||
    text.contains("Yesterday") || text.contains("Today") ||
    // Month names
    text.contains("Jan") || text.contains("Feb") || text.contains("Mar") ||
    text.contains("Apr") || text.contains("May") || text.contains("Jun") ||
    text.contains("Jul") || text.contains("Aug") || text.contains("Sep") ||
    text.contains("Oct") || text.contains("Nov") || text.contains("Dec")
}

/// Helper to convert CFType to String.
fn cftype_to_string_local(value: &core_foundation::base::CFType) -> Option<String> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    
    let type_id = value.type_of();
    if type_id == CFString::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_string: CFString = unsafe {
            CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
        };
        return Some(cf_string.to_string());
    }
    None
}

/// Check if text is common Slack UI text that should be filtered out.
fn is_slack_ui_text(text: &str) -> bool {
    // Common Slack UI labels to filter out
    const UI_LABELS: &[&str] = &[
        "Home", "Activity", "Files", "More", "Threads", "Huddles", 
        "Drafts & sent", "Directories", "Starred", "Channels",
        "Direct Messages", "Apps", "Messages", "Add canvas",
        "Canvas", "List", "Folder", "Channel",
        // Time indicators (short ones)
        "AM", "PM",
    ];
    
    // Check exact matches
    if UI_LABELS.contains(&text) {
        return true;
    }
    
    // Check if it's just a time like "9:41" or "8:43"
    if text.len() <= 8 && text.contains(':') && text.chars().all(|c| c.is_ascii_digit() || c == ':' || c == ' ') {
        return true;
    }
    
    false
}

// ============================================================================
// Microsoft Teams-Specific Extraction
// ============================================================================

/// Microsoft Teams bundle identifier constant (classic/old version).
pub const TEAMS_BUNDLE_ID: &str = "com.microsoft.teams";

/// Microsoft Teams bundle identifier constant (new version).
pub const TEAMS_NEW_BUNDLE_ID: &str = "com.microsoft.teams2";

/// Configuration for Teams content readiness polling.
#[derive(Debug, Clone)]
pub struct TeamsExtractionConfig {
    /// Maximum time to wait for content to be ready (default: 15 seconds)
    pub max_wait_time: std::time::Duration,
    /// Initial delay before first content check (default: 500ms)
    pub initial_delay: std::time::Duration,
    /// Maximum delay between retries (default: 2 seconds)
    pub max_retry_delay: std::time::Duration,
    /// Minimum content length to consider extraction successful (default: 50 chars)
    pub min_content_length: usize,
}

impl Default for TeamsExtractionConfig {
    fn default() -> Self {
        Self {
            max_wait_time: std::time::Duration::from_secs(15),
            initial_delay: std::time::Duration::from_millis(500),
            max_retry_delay: std::time::Duration::from_secs(2),
            min_content_length: 50,
        }
    }
}

/// Check if the given bundle ID is Microsoft Teams (any version).
pub fn is_teams(bundle_id: &str) -> bool {
    bundle_id == TEAMS_BUNDLE_ID || bundle_id == TEAMS_NEW_BUNDLE_ID
}

/// Teams version type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TeamsVersion {
    /// Classic Teams (com.microsoft.teams) - Electron-based, supports accessibility extraction
    Classic,
    /// New Teams (com.microsoft.teams2) - Native/WebKit-based, limited accessibility support
    New,
    /// Teams is not running
    NotRunning,
}

impl TeamsVersion {
    /// Get a human-readable description of the Teams version
    pub fn description(&self) -> &'static str {
        match self {
            TeamsVersion::Classic => "Classic Teams (Electron-based) - supports accessibility extraction",
            TeamsVersion::New => "New Teams (Native/WebKit) - limited accessibility support, use Graph API",
            TeamsVersion::NotRunning => "Microsoft Teams is not running",
        }
    }
    
    /// Check if this version supports accessibility extraction
    pub fn supports_accessibility_extraction(&self) -> bool {
        matches!(self, TeamsVersion::Classic)
    }
}

/// Detect which version of Microsoft Teams is currently running.
///
/// Returns the Teams version type:
/// - `TeamsVersion::New` if New Teams (com.microsoft.teams2) is running
/// - `TeamsVersion::Classic` if Classic Teams (com.microsoft.teams) is running
/// - `TeamsVersion::NotRunning` if neither version is running
///
/// Note: New Teams is checked first since it's the default version as of 2024.
pub fn detect_teams_version() -> TeamsVersion {
    // Check for new Teams first (com.microsoft.teams2) - this is the default now
    if get_pid_for_bundle_id(TEAMS_NEW_BUNDLE_ID).is_some() {
        return TeamsVersion::New;
    }
    
    // Check for classic Teams (com.microsoft.teams)
    if get_pid_for_bundle_id(TEAMS_BUNDLE_ID).is_some() {
        return TeamsVersion::Classic;
    }
    
    TeamsVersion::NotRunning
}

/// Get the bundle ID of the currently running Teams version.
///
/// Returns `Some(bundle_id)` if Teams is running, `None` otherwise.
pub fn get_running_teams_bundle_id() -> Option<&'static str> {
    match detect_teams_version() {
        TeamsVersion::New => Some(TEAMS_NEW_BUNDLE_ID),
        TeamsVersion::Classic => Some(TEAMS_BUNDLE_ID),
        TeamsVersion::NotRunning => None,
    }
}

/// Prepare Microsoft Teams for accessibility extraction.
///
/// Teams requires special handling because:
/// 1. The classic version is Electron-based and needs AXManualAccessibility enabled
/// 2. The new version (Teams 2.0) may have different accessibility behavior
/// 3. Content takes time to load, especially chat history
///
/// This function enables accessibility and waits for the content to be ready.
///
/// # Arguments
///
/// * `bundle_id` - The Teams bundle ID (classic or new)
/// * `config` - Optional configuration for timeout and retry behavior
///
/// # Returns
///
/// `Ok(true)` if Teams is ready for extraction,
/// `Ok(false)` if Teams is not running,
/// `Err` if preparation failed after all retries.
pub fn prepare_teams(bundle_id: &str, config: Option<TeamsExtractionConfig>) -> Result<bool, ExtractionError> {
    let config = config.unwrap_or_default();
    
    log::info!("[AX-TEAMS] 🔧 Preparing Microsoft Teams for accessibility extraction...");
    
    // Get Teams' PID
    let pid = match get_pid_for_bundle_id(bundle_id) {
        Some(pid) => pid,
        None => {
            log::debug!("[AX-TEAMS] Teams is not running");
            return Ok(false);
        }
    };
    
    log::debug!("[AX-TEAMS] Found Teams with PID: {}", pid);
    
    // Enable accessibility for Teams (works for classic Electron version)
    match enable_electron_accessibility(pid) {
        Ok(()) => {
            log::info!("[AX-TEAMS] ✓ Accessibility enabled for Teams");
        }
        Err(e) => {
            // Log but continue - new Teams might still work without this
            log::warn!("[AX-TEAMS] ⚠️ Could not enable accessibility: {}", e);
        }
    }
    
    // Wait for content to be ready with exponential backoff
    wait_for_teams_content(pid, &config)
}

/// Wait for Teams' accessibility content to be ready.
fn wait_for_teams_content(pid: i32, config: &TeamsExtractionConfig) -> Result<bool, ExtractionError> {
    use std::time::Instant;
    
    let start_time = Instant::now();
    let mut current_delay = config.initial_delay;
    let mut attempt = 0;
    
    // Initial delay to let accessibility tree start building
    log::debug!("[AX-TEAMS] Waiting {}ms for initial accessibility tree...", config.initial_delay.as_millis());
    std::thread::sleep(config.initial_delay);
    
    loop {
        attempt += 1;
        let elapsed = start_time.elapsed();
        
        if elapsed >= config.max_wait_time {
            log::warn!(
                "[AX-TEAMS] ⚠️ Timeout waiting for Teams content after {:?} ({} attempts)",
                elapsed, attempt
            );
            // Return Ok(true) anyway - let the main extraction try
            return Ok(true);
        }
        
        log::debug!("[AX-TEAMS] Checking content readiness (attempt {}, elapsed {:?})...", attempt, elapsed);
        
        // Check if Teams has meaningful content
        match check_teams_content_ready(pid, config.min_content_length) {
            TeamsContentReadiness::Ready(content_len) => {
                log::info!(
                    "[AX-TEAMS] ✓ Teams content ready ({} chars) after {:?} ({} attempts)",
                    content_len, elapsed, attempt
                );
                return Ok(true);
            }
            TeamsContentReadiness::Loading(reason) => {
                log::debug!("[AX-TEAMS] Content not ready: {}", reason);
            }
            TeamsContentReadiness::Error(e) => {
                log::warn!("[AX-TEAMS] Error checking content: {}", e);
            }
        }
        
        // Exponential backoff with cap
        std::thread::sleep(current_delay);
        current_delay = std::cmp::min(current_delay * 2, config.max_retry_delay);
    }
}

/// Result of checking Teams content readiness.
#[derive(Debug)]
#[allow(dead_code)]
enum TeamsContentReadiness {
    /// Content is ready with the given length
    Ready(usize),
    /// Content is still loading with reason
    Loading(String),
    /// Error occurred while checking (reserved for future use)
    Error(String),
}

/// Check if Teams' accessibility content is ready for extraction.
fn check_teams_content_ready(pid: i32, min_content_length: usize) -> TeamsContentReadiness {
    use accessibility::AXUIElement;
    use accessibility::AXUIElementAttributes;
    
    // Create application reference
    let app = AXUIElement::application(pid);
    
    // Try to get the focused window
    let window = match app.focused_window() {
        Ok(w) => w,
        Err(e) => {
            return TeamsContentReadiness::Loading(format!("No focused window: {:?}", e));
        }
    };
    
    // Look for AXWebArea elements (Teams' main content)
    let web_areas = find_web_areas(&window, 0);
    
    if web_areas.is_empty() {
        return TeamsContentReadiness::Loading("No AXWebArea elements found".to_string());
    }
    
    // Check if any web area has meaningful content
    let mut total_content_len = 0;
    for web_area in &web_areas {
        let content_len = estimate_content_length(web_area, 0);
        total_content_len += content_len;
    }
    
    if total_content_len >= min_content_length {
        TeamsContentReadiness::Ready(total_content_len)
    } else {
        TeamsContentReadiness::Loading(format!(
            "Content too short: {} chars (need {})",
            total_content_len, min_content_length
        ))
    }
}

/// A parsed Teams message with author, date, and content.
#[derive(Debug, Clone)]
pub struct TeamsMessage {
    pub author: String,
    pub date: String,
    pub content: String,
}

impl TeamsMessage {
    /// Format the message as [Author] [Date] Message
    pub fn format(&self) -> String {
        format!("[{}] [{}] {}", self.author, self.date, self.content)
    }
}

/// Extract text content from Teams' accessibility tree.
///
/// Teams uses a similar structure to Slack:
/// - Messages are in AXStaticText elements inside AXList/AXGroup/AXRow
/// - We need to traverse through AXList and AXOutline (normally filtered as UI chrome)
/// - Focus on the AXWebArea which contains the main content
///
/// Messages are formatted as: [Author] [Date] Message
///
/// # Arguments
///
/// * `app` - The Teams application AXUIElement
///
/// # Returns
///
/// Extracted text content from Teams messages, formatted with author and date.
pub fn extract_teams_content(app: &accessibility::AXUIElement) -> String {
    use accessibility::AXUIElementAttributes;
    
    // Get the focused window
    let window = match app.focused_window() {
        Ok(w) => w,
        Err(_) => return String::new(),
    };
    
    // First, try the new extraction method for New Teams (com.microsoft.teams2)
    // This traverses the accessibility tree looking for AXStaticText and AXHeading elements
    let new_teams_content = extract_new_teams_content(&window);
    if !new_teams_content.is_empty() {
        return new_teams_content;
    }
    
    // Fall back to the old method for Classic Teams
    // Find the AXWebArea (main content area)
    let web_areas = find_web_areas(&window, 0);
    
    let mut messages: Vec<TeamsMessage> = Vec::new();
    
    for web_area in web_areas {
        extract_teams_messages_recursive(&web_area, &mut messages, 0);
    }
    
    // Deduplicate messages by content (Teams may have duplicate elements)
    let mut seen = std::collections::HashSet::new();
    let unique_messages: Vec<&TeamsMessage> = messages
        .iter()
        .filter(|m| {
            let key = format!("{}:{}:{}", m.author, m.date, m.content);
            if seen.contains(&key) {
                false
            } else {
                seen.insert(key);
                true
            }
        })
        .collect();
    
    // Format all messages
    unique_messages
        .iter()
        .map(|m| m.format())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract content from New Teams (com.microsoft.teams2) using deep tree traversal.
///
/// New Teams uses Chromium-based web views that don't expose text through AXNumberOfCharacters
/// or AXValue on the top-level web content group. Instead, we need to traverse deep into
/// the tree and collect text from AXStaticText, AXHeading, and AXLink elements.
fn extract_new_teams_content(window: &accessibility::AXUIElement) -> String {
    // Find the web content group (contains "Web content" in title)
    let web_group = match find_web_content_group_for_teams(window, 0) {
        Some(g) => g,
        None => return String::new(),
    };
    
    // Collect all text elements from the tree in order
    let mut all_text: Vec<(String, String)> = Vec::new(); // (role, text)
    collect_all_text_elements(&web_group, &mut all_text, 0);
    
    if all_text.is_empty() {
        return String::new();
    }
    
    // Parse the collected text into messages
    let messages = parse_teams_text_into_messages(&all_text);
    
    if messages.is_empty() {
        return String::new();
    }
    
    // Deduplicate and format
    let mut seen = std::collections::HashSet::new();
    let unique_messages: Vec<&TeamsMessage> = messages
        .iter()
        .filter(|m| {
            // Skip UI elements
            if is_teams_ui_text(&m.content) {
                return false;
            }
            let key = format!("{}:{}", m.author, m.content);
            if seen.contains(&key) {
                false
            } else {
                seen.insert(key);
                true
            }
        })
        .collect();
    
    unique_messages
        .iter()
        .map(|m| m.format())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collect all text elements from the tree in order.
fn collect_all_text_elements(
    element: &accessibility::AXUIElement,
    results: &mut Vec<(String, String)>,
    depth: usize,
) {
    use accessibility::AXUIElementAttributes;
    use accessibility::attribute::AXAttribute;
    use core_foundation::base::CFType;
    use core_foundation::string::CFString;
    
    if depth > 150 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Skip menu-related UI chrome
    if matches!(role.as_str(), 
        "AXMenuBar" | "AXMenu" | "AXMenuItem" | "AXToolbar"
    ) {
        return;
    }
    
    // Collect text from relevant elements
    match role.as_str() {
        "AXHeading" => {
            if let Ok(title) = element.title() {
                let text = title.to_string();
                if !text.is_empty() {
                    results.push(("AXHeading".to_string(), text));
                }
            }
        }
        "AXStaticText" => {
            if let Ok(value) = element.value() {
                if let Some(text) = cftype_to_string_local(&value) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && trimmed.len() > 1 {
                        results.push(("AXStaticText".to_string(), trimmed.to_string()));
                    }
                }
            }
        }
        "AXLink" => {
            let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
            if let Ok(desc) = element.attribute(&desc_attr) {
                if let Some(text) = cftype_to_string_local(&desc) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && trimmed.len() > 3 {
                        results.push(("AXLink".to_string(), trimmed.to_string()));
                    }
                }
            }
        }
        _ => {}
    }
    
    // Recurse into children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                collect_all_text_elements(&child, results, depth + 1);
            }
        }
    }
}

/// Parse collected text elements into messages.
fn parse_teams_text_into_messages(elements: &[(String, String)]) -> Vec<TeamsMessage> {
    let mut messages: Vec<TeamsMessage> = Vec::new();
    let mut i = 0;
    
    while i < elements.len() {
        let (role, text) = &elements[i];
        
        // Look for headings that indicate messages
        if role == "AXHeading" {
            // Check for " by Author" pattern (message from a user)
            if let Some(by_pos) = text.rfind(" by ") {
                let author = text[by_pos + 4..].trim().to_string();
                
                if !author.is_empty() && !is_teams_ui_text(&author) {
                    // Look ahead for the actual content and timestamp
                    let mut content_parts: Vec<String> = Vec::new();
                    let mut timestamp: Option<String> = None;
                    let mut j = i + 1;
                    let mut found_author = false;
                    
                    // Collect content until we hit another heading with " by " or a system event
                    while j < elements.len() {
                        let (next_role, next_text) = &elements[j];
                        
                        // Stop at next message heading or system event
                        if next_role == "AXHeading" {
                            if next_text.contains(" by ") || 
                               next_text.starts_with("Meeting ") ||
                               next_text.contains(" joined ") ||
                               next_text.contains(" left ") ||
                               next_text.contains(" was invited ") ||
                               next_text.contains(" named the meeting ") {
                                break;
                            }
                        }
                        
                        // Skip the heading title duplicate in static text
                        if next_text == text {
                            j += 1;
                            continue;
                        }
                        
                        // Skip author name (appears right after heading)
                        if next_text == &author && !found_author {
                            found_author = true;
                            j += 1;
                            continue;
                        }
                        
                        // Skip UI text
                        if is_teams_ui_text(next_text) {
                            j += 1;
                            continue;
                        }
                        
                        // Skip "More message options" button descriptions
                        if next_text == "More message options" {
                            j += 1;
                            continue;
                        }
                        
                        // Check for timestamp (appears after author name)
                        if looks_like_teams_timestamp(next_text) && timestamp.is_none() && found_author {
                            timestamp = Some(next_text.clone());
                            j += 1;
                            continue;
                        }
                        
                        // Collect content from static text and links
                        if next_role == "AXStaticText" || next_role == "AXLink" {
                            let clean = next_text.strip_prefix("Link ").unwrap_or(next_text);
                            let clean = clean.strip_prefix("Url Preview for ").unwrap_or(clean);
                            
                            // Skip very short text, ellipsis, and separators
                            if clean.len() > 2 && 
                               !clean.ends_with("...") && 
                               clean != " - " &&
                               clean != " " {
                                content_parts.push(clean.to_string());
                            }
                        }
                        
                        j += 1;
                    }
                    
                    // Deduplicate content while preserving order
                    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                    let unique_content: Vec<String> = content_parts
                        .into_iter()
                        .filter(|p| {
                            // Skip if we've seen this exact text
                            if seen.contains(p) {
                                return false;
                            }
                            // Skip if this is a substring of something we've already seen
                            // (handles URL preview duplicates)
                            for existing in &seen {
                                if existing.contains(p.as_str()) || p.contains(existing.as_str()) {
                                    return false;
                                }
                            }
                            seen.insert(p.clone());
                            true
                        })
                        .collect();
                    
                    if !unique_content.is_empty() {
                        messages.push(TeamsMessage {
                            author,
                            date: timestamp.unwrap_or_else(|| "Unknown".to_string()),
                            content: unique_content.join(" "),
                        });
                    }
                    
                    i = j;
                    continue;
                }
            }
            
            // Check for meeting events
            if text.starts_with("Meeting started") {
                let timestamp = if let Some(at_pos) = text.find(" at ") {
                    // Extract timestamp, stopping at "after" if present
                    let rest = &text[at_pos + 4..];
                    if let Some(after_pos) = rest.find(" after ") {
                        rest[..after_pos].trim().to_string()
                    } else {
                        rest.trim().to_string()
                    }
                } else {
                    "Unknown".to_string()
                };
                
                messages.push(TeamsMessage {
                    author: "System".to_string(),
                    date: timestamp,
                    content: "Meeting started".to_string(),
                });
                
                i += 1;
                continue;
            }
            
            if text.starts_with("Meeting ended") {
                let timestamp = if let Some(at_pos) = text.find(" at ") {
                    // Extract timestamp, stopping at "after" if present
                    let rest = &text[at_pos + 4..];
                    if let Some(after_pos) = rest.find(" after ") {
                        rest[..after_pos].trim().to_string()
                    } else {
                        rest.trim().to_string()
                    }
                } else {
                    "Unknown".to_string()
                };
                
                // Extract duration if present
                let duration = if let Some(after_pos) = text.find(" after ") {
                    Some(text[after_pos + 7..].trim().to_string())
                } else {
                    None
                };
                
                let content = if let Some(dur) = duration {
                    format!("Meeting ended (duration: {})", dur)
                } else {
                    "Meeting ended".to_string()
                };
                
                messages.push(TeamsMessage {
                    author: "System".to_string(),
                    date: timestamp,
                    content,
                });
                
                i += 1;
                continue;
            }
            
            // Skip other system events (joined, left, invited, etc.)
            if text.contains(" joined ") || 
               text.contains(" left ") || 
               text.contains(" was invited ") ||
               text.contains(" named the meeting ") {
                i += 1;
                continue;
            }
        }
        
        i += 1;
    }
    
    messages
}

/// Find the web content group in New Teams.
fn find_web_content_group_for_teams(element: &accessibility::AXUIElement, depth: usize) -> Option<accessibility::AXUIElement> {
    use accessibility::AXUIElementAttributes;
    
    if depth > 20 { return None; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if title_str.contains("Web content") {
                return Some(element.clone());
            }
        }
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                if let Some(found) = find_web_content_group_for_teams(&child, depth + 1) {
                    return Some(found);
                }
            }
        }
    }
    
    None
}

/// Recursively extract messages from Teams' accessibility tree.
fn extract_teams_messages_recursive(
    element: &accessibility::AXUIElement,
    messages: &mut Vec<TeamsMessage>,
    depth: usize,
) {
    use accessibility::AXUIElementAttributes;
    
    // Limit depth
    if depth > 100 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Skip menu-related UI chrome
    if matches!(role.as_str(), 
        "AXMenuBar" | "AXMenu" | "AXMenuItem" | "AXToolbar" | 
        "AXTabGroup" | "AXTab" | "AXSlider" | "AXSplitter"
    ) {
        return;
    }
    
    // Check AXGroup elements - they often contain the full message info in AXTitle
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if let Some(msg) = parse_teams_message_from_title(&title_str) {
                messages.push(msg);
            }
        }
    }
    
    // Also check AXCell elements which may contain message info
    if role == "AXCell" || role == "AXRow" {
        if let Some(msg) = extract_teams_message_from_container(element) {
            messages.push(msg);
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                extract_teams_messages_recursive(&child, messages, depth + 1);
            }
        }
    }
}

/// Parse a Teams message from an AXGroup's AXTitle attribute.
fn parse_teams_message_from_title(title: &str) -> Option<TeamsMessage> {
    let title = title.trim();
    
    // Skip if too short or looks like UI text
    if title.len() < 10 || is_teams_ui_text(title) {
        return None;
    }
    
    // Teams message format varies, but often follows patterns like:
    // "Author Name: message content. timestamp"
    // "Author Name, timestamp: message content"
    
    // Look for the pattern "Author: message. timestamp"
    let colon_pos = title.find(':')?;
    
    // Author is before the first colon
    let author = title[..colon_pos].trim();
    
    // Skip if author looks like a UI label
    if author.is_empty() || is_teams_ui_text(author) {
        return None;
    }
    
    // The rest contains message and timestamp
    let rest = title[colon_pos + 1..].trim();
    
    // Try to find timestamp at the end
    let (content, date) = extract_teams_timestamp_from_end(rest);
    
    // Skip if content is empty or too short
    let content = content.trim();
    if content.is_empty() || content.len() < 3 {
        return None;
    }
    
    // Clean up content
    let content = clean_teams_message_content(content);
    
    if content.is_empty() {
        return None;
    }
    
    Some(TeamsMessage {
        author: author.to_string(),
        date: date.unwrap_or_else(|| "Unknown".to_string()),
        content,
    })
}

/// Extract timestamp from the end of a Teams message string.
fn extract_teams_timestamp_from_end(text: &str) -> (&str, Option<String>) {
    // Common timestamp patterns in Teams:
    // "message content. 10:59 AM"
    // "message content, Today at 4:10 PM"
    // "message content, Yesterday at 7:28 PM"
    
    // Pattern 1: Time with AM/PM at end
    let pattern1 = regex_lite::Regex::new(
        r"[,.\s]+(\d{1,2}:\d{2}\s*(?:AM|PM))\s*$"
    );
    
    if let Ok(pattern) = pattern1 {
        if let Some(caps) = pattern.captures(text) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                if let Some(full_match) = caps.get(0) {
                    let content = text[..full_match.start()].trim();
                    return (content, Some(time));
                }
            }
        }
    }
    
    // Pattern 2: "Today at" or "Yesterday at" with time
    let pattern2 = regex_lite::Regex::new(
        r"[,.\s]+((?:Today|Yesterday)\s+at\s+\d{1,2}:\d{2}\s*(?:AM|PM))\s*$"
    );
    
    if let Ok(pattern) = pattern2 {
        if let Some(caps) = pattern.captures(text) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                if let Some(full_match) = caps.get(0) {
                    let content = text[..full_match.start()].trim();
                    return (content, Some(time));
                }
            }
        }
    }
    
    (text, None)
}

/// Clean up Teams message content by removing artifacts.
fn clean_teams_message_content(content: &str) -> String {
    let mut result = content.to_string();
    
    // Remove common artifacts
    let artifacts = [
        "Image",
        "Attachment",
        "File",
        "GIF",
        "Sticker",
        "Edited",
    ];
    
    for artifact in artifacts {
        result = result.replace(artifact, "");
    }
    
    // Clean up multiple spaces and trim
    let parts: Vec<&str> = result.split_whitespace().collect();
    parts.join(" ")
}

/// Try to extract a message from a container element (AXCell, AXRow).
fn extract_teams_message_from_container(element: &accessibility::AXUIElement) -> Option<TeamsMessage> {
    let mut author: Option<String> = None;
    let mut date: Option<String> = None;
    let mut content_parts: Vec<String> = Vec::new();
    
    // Recursively collect info from children
    collect_teams_message_parts(element, &mut author, &mut date, &mut content_parts, 0);
    
    // Need at least author and some content
    let author = author?;
    if content_parts.is_empty() {
        return None;
    }
    
    let content = content_parts.join(" ");
    if content.trim().is_empty() || content.len() < 3 {
        return None;
    }
    
    Some(TeamsMessage {
        author,
        date: date.unwrap_or_else(|| "Unknown".to_string()),
        content,
    })
}

/// Collect message parts (author, date, content) from element children.
fn collect_teams_message_parts(
    element: &accessibility::AXUIElement,
    author: &mut Option<String>,
    date: &mut Option<String>,
    content_parts: &mut Vec<String>,
    depth: usize,
) {
    use accessibility::AXUIElementAttributes;
    use accessibility::attribute::AXAttribute;
    use core_foundation::base::CFType;
    use core_foundation::string::CFString;
    
    if depth > 10 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // AXButton often contains author name
    if role == "AXButton" && author.is_none() {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if !title_str.is_empty() && !is_teams_ui_text(&title_str) && title_str.len() > 2 {
                *author = Some(title_str);
            }
        }
    }
    
    // AXLink often contains date/timestamp
    if role == "AXLink" && date.is_none() {
        let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
        if let Ok(value) = element.attribute(&desc_attr) {
            if let Some(text) = cftype_to_string_local(&value) {
                if looks_like_teams_timestamp(&text) {
                    *date = Some(text);
                }
            }
        }
    }
    
    // AXStaticText contains message content
    if role == "AXStaticText" {
        if let Ok(value) = element.value() {
            if let Some(text) = cftype_to_string_local(&value) {
                let trimmed = text.trim();
                if !trimmed.is_empty() && trimmed.len() > 2 && !is_teams_ui_text(trimmed) {
                    content_parts.push(trimmed.to_string());
                }
            }
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                collect_teams_message_parts(&child, author, date, content_parts, depth + 1);
            }
        }
    }
}

/// Check if text looks like a Teams timestamp.
fn looks_like_teams_timestamp(text: &str) -> bool {
    let text = text.trim();
    
    // Check for time patterns (AM/PM, a.m./p.m.)
    text.contains("AM") || text.contains("PM") ||
    text.contains("a.m.") || text.contains("p.m.") ||
    text.contains("ago") ||
    text.contains("Yesterday") || text.contains("Today") ||
    // Day names
    text.starts_with("Monday") || text.starts_with("Tuesday") || 
    text.starts_with("Wednesday") || text.starts_with("Thursday") ||
    text.starts_with("Friday") || text.starts_with("Saturday") || 
    text.starts_with("Sunday") ||
    // Month names
    text.contains("Jan") || text.contains("Feb") || text.contains("Mar") ||
    text.contains("Apr") || text.contains("May") || text.contains("Jun") ||
    text.contains("Jul") || text.contains("Aug") || text.contains("Sep") ||
    text.contains("Oct") || text.contains("Nov") || text.contains("Dec")
}

/// Check if text is common Teams UI text that should be filtered out.
fn is_teams_ui_text(text: &str) -> bool {
    const UI_LABELS: &[&str] = &[
        "Chat", "Teams", "Calendar", "Calls", "Files", "Activity",
        "More", "Search", "Settings", "Help", "New chat", "New meeting",
        "Meet", "Join", "Leave", "Mute", "Unmute", "Share", "React",
        "Reply", "Forward", "Copy", "Delete", "Edit", "Pin", "Save",
        "Mark as unread", "Turn on notifications", "Hide",
        // Navigation items
        "Mentions", "Favourites", "Chats", "Shared", "Recap", "Q&A",
        "OneDrive", "Apps", "Type a message",
        // Time indicators
        "AM", "PM",
    ];
    
    if UI_LABELS.contains(&text) {
        return true;
    }
    
    // Check if it's just a time like "9:41" or "8:43"
    if text.len() <= 8 && text.contains(':') && text.chars().all(|c| c.is_ascii_digit() || c == ':' || c == ' ') {
        return true;
    }
    
    false
}

/// Check if text is a date header (like "February 3", "Wednesday", etc.)
/// This function is kept for potential future use in message grouping.
#[allow(dead_code)]
fn is_date_header(text: &str) -> bool {
    let text = text.trim();
    
    // Day names
    const DAYS: &[&str] = &[
        "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
        "Today", "Yesterday",
    ];
    
    // Month names
    const MONTHS: &[&str] = &[
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    
    // Check if it's just a day name
    if DAYS.contains(&text) {
        return true;
    }
    
    // Check if it starts with a month name followed by a number (e.g., "February 3")
    for month in MONTHS {
        if text.starts_with(month) {
            // Check if the rest is just a number
            let rest = text[month.len()..].trim();
            if rest.chars().all(|c| c.is_ascii_digit() || c == ',' || c == ' ') {
                return true;
            }
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Unit Tests for is_electron_app
    // ============================================================================

    #[test]
    fn test_is_electron_app_slack() {
        assert!(is_electron_app("com.tinyspeck.slackmacgap"));
    }

    #[test]
    fn test_is_electron_app_discord() {
        assert!(is_electron_app("com.hnc.Discord"));
    }

    #[test]
    fn test_is_electron_app_vscode() {
        assert!(is_electron_app("com.microsoft.VSCode"));
        assert!(is_electron_app("com.microsoft.VSCodeInsiders"));
    }

    #[test]
    fn test_is_electron_app_teams() {
        assert!(is_electron_app("com.microsoft.teams"));
        assert!(is_electron_app("com.microsoft.teams2"));
    }

    #[test]
    fn test_is_electron_app_notion() {
        assert!(is_electron_app("notion.id"));
    }

    #[test]
    fn test_is_electron_app_figma() {
        assert!(is_electron_app("com.figma.Desktop"));
    }

    #[test]
    fn test_is_electron_app_spotify() {
        assert!(is_electron_app("com.spotify.client"));
    }

    #[test]
    fn test_is_electron_app_obsidian() {
        assert!(is_electron_app("md.obsidian"));
    }

    #[test]
    fn test_is_electron_app_1password() {
        assert!(is_electron_app("com.1password.1password"));
    }

    #[test]
    fn test_is_electron_app_whatsapp() {
        assert!(is_electron_app("com.whatsapp.WhatsApp"));
    }

    #[test]
    fn test_is_not_electron_app_native_apps() {
        // Native macOS apps should return false
        assert!(!is_electron_app("com.apple.Safari"));
        assert!(!is_electron_app("com.apple.finder"));
        assert!(!is_electron_app("com.apple.TextEdit"));
        assert!(!is_electron_app("com.apple.mail"));
    }

    #[test]
    fn test_is_not_electron_app_microsoft_office() {
        // Microsoft Office apps are native, not Electron
        assert!(!is_electron_app("com.microsoft.Word"));
        assert!(!is_electron_app("com.microsoft.Excel"));
        assert!(!is_electron_app("com.microsoft.Powerpoint"));
        assert!(!is_electron_app("com.microsoft.Outlook"));
    }

    #[test]
    fn test_is_not_electron_app_apple_iwork() {
        // Apple iWork apps are native
        assert!(!is_electron_app("com.apple.iWork.Pages"));
        assert!(!is_electron_app("com.apple.iWork.Numbers"));
        assert!(!is_electron_app("com.apple.iWork.Keynote"));
    }

    #[test]
    fn test_is_not_electron_app_empty_string() {
        assert!(!is_electron_app(""));
    }

    #[test]
    fn test_is_not_electron_app_unknown() {
        assert!(!is_electron_app("com.unknown.app"));
        assert!(!is_electron_app("some.random.bundle.id"));
    }

    #[test]
    fn test_is_electron_app_case_sensitive() {
        // Bundle IDs are case-sensitive
        assert!(!is_electron_app("com.tinyspeck.SlackMacGap")); // Wrong case
        assert!(!is_electron_app("COM.TINYSPECK.SLACKMACGAP")); // All caps
        assert!(is_electron_app("com.tinyspeck.slackmacgap"));  // Correct
    }

    // ============================================================================
    // Unit Tests for Slack-specific functions
    // ============================================================================

    #[test]
    fn test_slack_bundle_id_constant() {
        assert_eq!(SLACK_BUNDLE_ID, "com.tinyspeck.slackmacgap");
    }

    #[test]
    fn test_is_slack_true() {
        assert!(is_slack("com.tinyspeck.slackmacgap"));
    }

    #[test]
    fn test_is_slack_false() {
        assert!(!is_slack("com.hnc.Discord"));
        assert!(!is_slack("com.microsoft.VSCode"));
        assert!(!is_slack("com.apple.Safari"));
        assert!(!is_slack(""));
    }

    #[test]
    fn test_is_slack_case_sensitive() {
        assert!(!is_slack("com.tinyspeck.SlackMacGap"));
        assert!(!is_slack("COM.TINYSPECK.SLACKMACGAP"));
    }

    #[test]
    fn test_slack_extraction_config_default() {
        let config = SlackExtractionConfig::default();
        assert_eq!(config.max_wait_time, std::time::Duration::from_secs(10));
        assert_eq!(config.initial_delay, std::time::Duration::from_millis(500));
        assert_eq!(config.max_retry_delay, std::time::Duration::from_secs(2));
        assert_eq!(config.min_content_length, 50);
    }

    #[test]
    fn test_slack_extraction_config_custom() {
        let config = SlackExtractionConfig {
            max_wait_time: std::time::Duration::from_secs(30),
            initial_delay: std::time::Duration::from_millis(1000),
            max_retry_delay: std::time::Duration::from_secs(5),
            min_content_length: 100,
        };
        assert_eq!(config.max_wait_time, std::time::Duration::from_secs(30));
        assert_eq!(config.initial_delay, std::time::Duration::from_millis(1000));
        assert_eq!(config.max_retry_delay, std::time::Duration::from_secs(5));
        assert_eq!(config.min_content_length, 100);
    }

    #[test]
    fn test_slack_is_also_electron_app() {
        // Slack should be detected as both Slack and an Electron app
        assert!(is_slack(SLACK_BUNDLE_ID));
        assert!(is_electron_app(SLACK_BUNDLE_ID));
    }

    // ============================================================================
    // Unit Tests for Teams-specific functions
    // ============================================================================

    #[test]
    fn test_teams_bundle_ids() {
        assert_eq!(TEAMS_BUNDLE_ID, "com.microsoft.teams");
        assert_eq!(TEAMS_NEW_BUNDLE_ID, "com.microsoft.teams2");
    }

    #[test]
    fn test_is_teams_true() {
        assert!(is_teams("com.microsoft.teams"));
        assert!(is_teams("com.microsoft.teams2"));
    }

    #[test]
    fn test_is_teams_false() {
        assert!(!is_teams("com.tinyspeck.slackmacgap"));
        assert!(!is_teams("com.microsoft.VSCode"));
        assert!(!is_teams("com.apple.Safari"));
        assert!(!is_teams(""));
    }

    #[test]
    fn test_teams_is_electron_app() {
        // Both Teams versions should be detected as Electron apps
        assert!(is_electron_app(TEAMS_BUNDLE_ID));
        assert!(is_electron_app(TEAMS_NEW_BUNDLE_ID));
    }
}
