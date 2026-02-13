//! OCR extractor integration.
//!
//! This module provides integration with the OCR extractor (Swift/Vision)
//! for extracting text from window screenshots.

use crate::types::{ExtractedContent, ExtractionError, ExtractorType, WindowInfo};
use image::DynamicImage;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info};

/// OCR extractor client
pub struct OcrExtractor {
    /// Path to the OCR extractor binary
    binary_path: PathBuf,
    /// Timeout for extraction in seconds
    timeout_secs: u64,
}

impl OcrExtractor {
    /// Create a new OCR extractor with default binary path
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

        let paths = [
            // Same directory as the running binary
            exe_dir.join("ocr-extractor"),
            // Relative to workspace root (Swift build output)
            PathBuf::from("ocr-extractor/.build/release/ocr-extractor"),
            PathBuf::from("ocr-extractor/.build/debug/ocr-extractor"),
            PathBuf::from("../ocr-extractor/.build/release/ocr-extractor"),
            PathBuf::from("../ocr-extractor/.build/debug/ocr-extractor"),
            // System paths
            PathBuf::from("/usr/local/bin/ocr-extractor"),
        ];

        for path in paths {
            if path.exists() {
                return path;
            }
        }

        // Default fallback - will fail gracefully
        PathBuf::from("ocr-extractor")
    }

    /// Check if the binary is available
    pub fn is_available(&self) -> bool {
        let exists = self.binary_path.exists();
        if !exists {
            tracing::debug!(
                "OCR extractor binary not found at: {}",
                self.binary_path.display()
            );
        }
        exists
    }

    /// Extract text from a window using OCR (captures window using ScreenCaptureKit)
    pub async fn extract_window(&self, window: &WindowInfo) -> Result<ExtractedContent, ExtractionError> {
        info!(
            "🔍 OCR: Extracting from {} ({}) via window-id {}",
            window.app_name, window.bundle_id, window.id
        );

        if !self.is_available() {
            return Err(ExtractionError::ExtractionFailed(
                "OCR extractor binary not found. Please build the ocr-extractor.".to_string(),
            ));
        }

        // Use --window-id mode which uses ScreenCaptureKit internally
        let output = Command::new(&self.binary_path)
            .arg("--window-id")
            .arg(window.id.to_string())
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ExtractionError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("OCR extraction failed: {}", stderr);
            return Err(ExtractionError::ExtractionFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("OCR output: {}", stdout);

        // Parse JSON output
        let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to parse output: {} - raw: {}", e, stdout))
        })?;

        // Check for error in response
        if let Some(error) = result["error"].as_str() {
            return Err(ExtractionError::ExtractionFailed(error.to_string()));
        }

        let text = result["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if text.is_empty() {
            return Err(ExtractionError::NoContent);
        }

        let confidence = result["confidence"].as_f64().unwrap_or(0.9) as f32;

        info!("🔍 OCR: Extracted {} chars from {} (confidence: {:.1}%)", 
              text.len(), window.title, confidence * 100.0);

        Ok(ExtractedContent {
            source: "ocr".to_string(),
            title: Some(window.title.clone()),
            content: text,
            app_name: window.app_name.clone(),
            bundle_id: Some(window.bundle_id.clone()),
            url: None,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "ocr".to_string(),
            confidence: Some(confidence),
        })
    }

    /// Extract text from a window using OCR (legacy method with pre-captured image)
    pub async fn extract(&self, window: &WindowInfo, image: &DynamicImage) -> Result<ExtractedContent, ExtractionError> {
        debug!(
            "🔍 OCR: Extracting from {} ({}) - image size: {}x{}",
            window.app_name, window.bundle_id, image.width(), image.height()
        );

        // Save image to temp file
        let temp_path = std::env::temp_dir().join(format!("ocr_capture_{}.png", window.id));
        debug!("🔍 OCR: Saving temp image to {}", temp_path.display());
        image.save(&temp_path).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to save temp image: {}", e))
        })?;

        // Run OCR on the image
        debug!("🔍 OCR: Running OCR extractor on {}", temp_path.display());
        let result = self.extract_from_file(&temp_path).await;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        match &result {
            Ok(content) => debug!("🔍 OCR: Extracted {} chars from {}", content.len(), window.title),
            Err(e) => debug!("🔍 OCR: Extraction failed for {}: {}", window.title, e),
        }

        let content = result?;

        Ok(ExtractedContent {
            source: "ocr".to_string(),
            title: Some(window.title.clone()),
            content,
            app_name: window.app_name.clone(),
            bundle_id: Some(window.bundle_id.clone()),
            url: None,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "ocr".to_string(),
            confidence: Some(0.9), // OCR confidence varies
        })
    }

    /// Extract text from an image file
    pub async fn extract_from_file(&self, path: &PathBuf) -> Result<String, ExtractionError> {
        if !self.is_available() {
            return Err(ExtractionError::ExtractionFailed(
                "OCR extractor binary not found. Please build the ocr-extractor.".to_string(),
            ));
        }

        let output = Command::new(&self.binary_path)
            .arg("--image")
            .arg(path)
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ExtractionError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("OCR extraction failed: {}", stderr);
            return Err(ExtractionError::ExtractionFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to parse output: {}", e))
        })?;

        let text = result["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if text.is_empty() {
            return Err(ExtractionError::NoContent);
        }

        Ok(text)
    }

    /// Set the extraction timeout
    pub fn set_timeout(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }

    /// Capture a window screenshot (for perceptual hash change detection)
    /// Returns the path to the captured image file
    pub async fn capture_window_image(&self, window: &WindowInfo) -> Result<PathBuf, ExtractionError> {
        if !self.is_available() {
            return Err(ExtractionError::ExtractionFailed(
                "OCR extractor binary not found. Please build the ocr-extractor.".to_string(),
            ));
        }

        let output_path = std::env::temp_dir().join(format!("ocr_capture_{}.png", window.id));

        // Use --capture-only mode which captures without running OCR
        let output = Command::new(&self.binary_path)
            .arg("--window-id")
            .arg(window.id.to_string())
            .arg("--capture-only")
            .arg("--output")
            .arg(output_path.to_str().unwrap_or("/tmp/capture.png"))
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ExtractionError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExtractionError::ExtractionFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to parse output: {}", e))
        })?;

        if let Some(error) = result["error"].as_str() {
            return Err(ExtractionError::ExtractionFailed(error.to_string()));
        }

        if result["captured"].as_bool() != Some(true) {
            return Err(ExtractionError::ExtractionFailed("Capture failed".to_string()));
        }

        Ok(output_path)
    }

    /// Load an image from a file path
    pub fn load_image(&self, path: &PathBuf) -> Result<DynamicImage, ExtractionError> {
        image::open(path).map_err(|e| {
            ExtractionError::ExtractionFailed(format!("Failed to load image: {}", e))
        })
    }

    /// Get the extractor type
    pub fn extractor_type(&self) -> ExtractorType {
        ExtractorType::Ocr
    }
}

impl Default for OcrExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_binary_path() {
        let extractor = OcrExtractor::new();
        // Just verify it doesn't panic
        let _ = extractor.binary_path;
    }
}
