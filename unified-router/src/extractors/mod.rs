//! Extractor integrations for different content sources.
//!
//! This module provides interfaces to the three extraction methods:
//! - Accessibility: For apps with good accessibility API support
//! - Chrome: For web browsers with the extension installed
//! - OCR: Fallback for apps without accessibility support

pub mod accessibility;
pub mod chrome;
pub mod ocr;

use crate::types::{ExtractedContent, ExtractionError, ExtractorType, WindowInfo};
use std::collections::HashSet;

/// Registry of supported applications and their preferred extractors
pub struct ExtractorRegistry {
    /// Apps that support accessibility extraction
    accessibility_apps: HashSet<String>,
    /// Browser bundle IDs that may have the Chrome extension
    chrome_browsers: HashSet<String>,
}

impl ExtractorRegistry {
    pub fn new() -> Self {
        let mut accessibility_apps = HashSet::new();
        let mut chrome_browsers = HashSet::new();

        // Microsoft Office
        accessibility_apps.insert("com.microsoft.Word".to_string());
        accessibility_apps.insert("com.microsoft.Excel".to_string());
        accessibility_apps.insert("com.microsoft.Powerpoint".to_string());
        accessibility_apps.insert("com.microsoft.Outlook".to_string());
        accessibility_apps.insert("com.microsoft.teams2".to_string());
        accessibility_apps.insert("com.microsoft.onenote.mac".to_string());

        // Apple iWork
        accessibility_apps.insert("com.apple.iWork.Pages".to_string());
        accessibility_apps.insert("com.apple.iWork.Numbers".to_string());
        accessibility_apps.insert("com.apple.iWork.Keynote".to_string());

        // Apple apps with good accessibility
        accessibility_apps.insert("com.apple.TextEdit".to_string());
        accessibility_apps.insert("com.apple.Notes".to_string());
        accessibility_apps.insert("com.apple.mail".to_string());
        accessibility_apps.insert("com.apple.finder".to_string());
        accessibility_apps.insert("com.apple.Preview".to_string());
        accessibility_apps.insert("com.apple.reminders".to_string());

        // Communication apps
        accessibility_apps.insert("com.tinyspeck.slackmacgap".to_string());

        // Chromium-based browsers (use Chrome extension)
        chrome_browsers.insert("com.google.Chrome".to_string());
        chrome_browsers.insert("com.google.Chrome.canary".to_string());
        chrome_browsers.insert("com.brave.Browser".to_string());
        chrome_browsers.insert("com.microsoft.edgemac".to_string());
        chrome_browsers.insert("com.vivaldi.Vivaldi".to_string());
        chrome_browsers.insert("com.operasoftware.Opera".to_string());
        chrome_browsers.insert("com.arc.browser".to_string());

        // NOTE: Safari is NOT included here because it uses Safari Web Extensions,
        // not Chrome Native Messaging. Without a Safari extension installed,
        // Safari windows would be skipped entirely (no extraction).
        // Safari will fall back to OCR extraction instead.

        Self {
            accessibility_apps,
            chrome_browsers,
        }
    }

    /// Determine the best extractor for an application
    pub fn get_extractor_type(&self, bundle_id: &str) -> ExtractorType {
        // Priority 1: Chrome extension for supported browsers
        if self.chrome_browsers.contains(bundle_id) {
            return ExtractorType::Chrome;
        }

        // Priority 2: Accessibility for supported apps
        if self.accessibility_apps.contains(bundle_id) {
            return ExtractorType::Accessibility;
        }

        // Priority 3: OCR fallback
        ExtractorType::Ocr
    }

    /// Check if an app supports accessibility extraction
    pub fn supports_accessibility(&self, bundle_id: &str) -> bool {
        self.accessibility_apps.contains(bundle_id)
    }

    /// Check if an app is a supported browser
    pub fn is_chrome_browser(&self, bundle_id: &str) -> bool {
        self.chrome_browsers.contains(bundle_id)
    }

    /// Add a custom app to accessibility support list
    pub fn add_accessibility_app(&mut self, bundle_id: &str) {
        self.accessibility_apps.insert(bundle_id.to_string());
    }

    /// Add a custom browser to Chrome extension list
    pub fn add_chrome_browser(&mut self, bundle_id: &str) {
        self.chrome_browsers.insert(bundle_id.to_string());
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for content extractors
#[async_trait::async_trait]
pub trait Extractor: Send + Sync {
    /// Extract content from a window
    async fn extract(&self, window: &WindowInfo) -> Result<ExtractedContent, ExtractionError>;

    /// Check if this extractor can handle the given app
    fn can_handle(&self, bundle_id: &str) -> bool;

    /// Get the extractor type
    fn extractor_type(&self) -> ExtractorType;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_microsoft_apps() {
        let registry = ExtractorRegistry::new();

        assert_eq!(
            registry.get_extractor_type("com.microsoft.Word"),
            ExtractorType::Accessibility
        );
        assert_eq!(
            registry.get_extractor_type("com.microsoft.Excel"),
            ExtractorType::Accessibility
        );
    }

    #[test]
    fn test_registry_browsers() {
        let registry = ExtractorRegistry::new();

        assert_eq!(
            registry.get_extractor_type("com.google.Chrome"),
            ExtractorType::Chrome
        );
        assert_eq!(
            registry.get_extractor_type("com.brave.Browser"),
            ExtractorType::Chrome
        );
    }

    #[test]
    fn test_registry_fallback() {
        let registry = ExtractorRegistry::new();

        assert_eq!(
            registry.get_extractor_type("com.unknown.app"),
            ExtractorType::Ocr
        );
    }
}
