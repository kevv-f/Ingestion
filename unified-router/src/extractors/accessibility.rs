//! Accessibility extractor integration.
//!
//! This module provides integration with the accessibility-extractor binary
//! for extracting content from apps with good accessibility API support.

use crate::types::{ExtractedContent, ExtractionError, ExtractorType, WindowInfo};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error};

/// Accessibility extractor client
pub struct AccessibilityExtractor {
    /// Path to the accessibility extractor binary
    binary_path: PathBuf,
    /// Timeout for extraction in seconds
    timeout_secs: u64,
}

impl AccessibilityExtractor {
    /// Create a new accessibility extractor with default binary path
    pub fn new() -> Self {
        Self {
            binary_path: Self::default_binary_path(),
            timeout_secs: 30,
        }
    }

    /// Create with a custom binary path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            binary_path: path,
            timeout_secs: 30,
        }
    }

    /// Get the default binary path
    fn default_binary_path() -> PathBuf {
        // Get the directory of the current executable
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        // Check common locations
        let paths = [
            // Same directory as the running binary
            exe_dir.join("ax-extractor"),
            // Relative to workspace root
            PathBuf::from("accessibility-extractor/target/release/ax-extractor"),
            PathBuf::from("accessibility-extractor/target/debug/ax-extractor"),
            PathBuf::from("../accessibility-extractor/target/release/ax-extractor"),
            PathBuf::from("../accessibility-extractor/target/debug/ax-extractor"),
            // System paths
            PathBuf::from("/usr/local/bin/ax-extractor"),
        ];

        for path in paths {
            if path.exists() {
                return path;
            }
        }

        // Default fallback - will fail gracefully
        PathBuf::from("ax-extractor")
    }

    /// Check if the binary is available
    pub fn is_available(&self) -> bool {
        let exists = self.binary_path.exists();
        if !exists {
            tracing::debug!(
                "Accessibility extractor binary not found at: {}",
                self.binary_path.display()
            );
        }
        exists
    }

    /// Extract content from a window using the accessibility extractor
    pub async fn extract(&self, window: &WindowInfo) -> Result<ExtractedContent, ExtractionError> {
        if !self.is_available() {
            return Err(ExtractionError::ExtractionFailed(format!(
                "Accessibility extractor binary not found at: {}. Run: cargo build --release -p accessibility-extractor",
                self.binary_path.display()
            )));
        }

        debug!(
            "Extracting from {} ({}) via accessibility",
            window.app_name, window.bundle_id
        );

        // Call the accessibility extractor with the bundle ID
        let output = Command::new(&self.binary_path)
            .arg("--app")
            .arg(&window.bundle_id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ExtractionError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Accessibility extraction failed: {}", stderr);
            return Err(ExtractionError::ExtractionFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to parse output: {}", e))
        })?;

        // Extract content from JSON
        let content = result["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(ExtractionError::NoContent);
        }

        Ok(ExtractedContent {
            source: "accessibility".to_string(),
            title: Some(window.title.clone()),
            content,
            app_name: window.app_name.clone(),
            bundle_id: Some(window.bundle_id.clone()),
            url: None,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "accessibility".to_string(),
            confidence: Some(1.0),
        })
    }

    /// Set the extraction timeout
    pub fn set_timeout(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }

    /// Get the extractor type
    pub fn extractor_type(&self) -> ExtractorType {
        ExtractorType::Accessibility
    }
}

impl Default for AccessibilityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_binary_path() {
        let extractor = AccessibilityExtractor::new();
        // Just verify it doesn't panic
        let _ = extractor.binary_path;
    }
}
