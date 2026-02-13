//! Main routing logic for the unified extraction router.
//!
//! This module orchestrates content extraction across all windows and displays,
//! coordinating between different extractors based on application type.

use crate::capture::CaptureService;
use crate::change_detector::ChangeDetector;
use crate::config::Config;
use crate::extractors::{
    accessibility::AccessibilityExtractor,
    chrome::ChromeExtensionClient,
    ocr::OcrExtractor,
    ExtractorRegistry,
};
use crate::privacy::PrivacyFilter;
use crate::types::{
    CapturePayload, ExtractedContent, ExtractionError, ExtractionTrigger, ExtractorType,
    WindowId, WindowInfo, WindowState,
};
use crate::window_tracker::{WindowChanges, WindowTracker};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Main unified router
pub struct UnifiedRouter {
    /// Configuration
    config: Config,
    /// Window tracker
    window_tracker: WindowTracker,
    /// Change detector (perceptual hash)
    change_detector: ChangeDetector,
    /// Capture service
    capture_service: CaptureService,
    /// Privacy filter
    privacy_filter: PrivacyFilter,
    /// Extractor registry
    extractor_registry: ExtractorRegistry,
    /// Accessibility extractor
    accessibility_extractor: AccessibilityExtractor,
    /// OCR extractor
    ocr_extractor: OcrExtractor,
    /// Chrome extension client
    chrome_client: ChromeExtensionClient,
    /// Window states
    window_states: HashMap<WindowId, WindowState>,
    /// Channel for extracted content
    content_tx: mpsc::Sender<CapturePayload>,
    /// Whether the router is paused
    paused: bool,
    /// Last tick time
    last_tick: Instant,
}

impl UnifiedRouter {
    /// Create a new unified router
    pub fn new(config: Config, content_tx: mpsc::Sender<CapturePayload>) -> Self {
        let change_detector = ChangeDetector::new(config.change_detection.hash_sensitivity);
        let privacy_filter = PrivacyFilter::new(config.privacy.clone());

        Self {
            config,
            window_tracker: WindowTracker::new(),
            change_detector,
            capture_service: CaptureService::new(),
            privacy_filter,
            extractor_registry: ExtractorRegistry::new(),
            accessibility_extractor: AccessibilityExtractor::new(),
            ocr_extractor: OcrExtractor::new(),
            chrome_client: ChromeExtensionClient::new(),
            window_states: HashMap::new(),
            content_tx,
            paused: false,
            last_tick: Instant::now(),
        }
    }

    /// Initialize the router
    pub fn init(&mut self) {
        info!("Initializing unified router");

        // Check extractor availability
        if self.config.extractors.accessibility_enabled {
            if self.accessibility_extractor.is_available() {
                info!("✅ Accessibility extractor available");
            } else {
                warn!("⚠️  Accessibility extractor not found - run: cargo build --release -p accessibility-extractor");
            }
        }

        if self.config.extractors.ocr_enabled {
            if self.ocr_extractor.is_available() {
                info!("✅ OCR extractor available");
            } else {
                warn!("⚠️  OCR extractor not found - run: swift build -c release in ocr-extractor/");
            }
        }

        // Refresh displays and windows
        self.window_tracker.refresh_displays();
        let _changes = self.window_tracker.refresh_windows();

        // Collect windows first to avoid borrow issues
        let windows: Vec<_> = self.window_tracker.windows().cloned().collect();

        // Initialize window states
        for window in windows {
            self.init_window_state(window);
        }

        info!(
            "Initialized with {} displays, {} windows",
            self.window_tracker.displays().len(),
            self.window_states.len()
        );
    }

    /// Perform initial extraction for the active window only (called after init)
    pub async fn initial_extraction(&mut self) {
        info!("Performing initial extraction for active window...");
        
        // Get the single active/focused window
        let active_window = self.window_tracker.get_active_window().cloned();
        
        if let Some(window) = active_window {
            if let Some(state) = self.window_states.get(&window.id) {
                // Skip blocked windows
                if state.is_blocked {
                    info!("Active window is blocked, skipping");
                    return;
                }
                
                // Skip Chrome windows (they push content to us)
                if state.extractor_type == ExtractorType::Chrome {
                    info!("Active window is Chrome, skipping (content pushed via extension)");
                    return;
                }
                
                self.trigger_extraction(window.id, ExtractionTrigger::AppActivated {
                    bundle_id: window.bundle_id.clone(),
                }).await;
                
                info!("Initial extraction complete: {} ({})", window.title, window.app_name);
            }
        } else {
            info!("No active window found");
        }
    }

    /// Initialize state for a new window
    fn init_window_state(&mut self, window: WindowInfo) {
        let extractor_type = self.extractor_registry.get_extractor_type(&window.bundle_id);
        let is_blocked = self.privacy_filter.is_blocked(&window.bundle_id);

        let state = WindowState {
            info: window.clone(),
            extractor_type,
            last_hash: None,
            last_content_hash: None,
            last_extraction: None,
            extraction_count: 0,
            is_blocked,
        };

        info!(
            "Tracking window: {} | bundle_id: {} | extractor: {:?} | on_screen: {} | blocked: {}",
            window.title,
            window.bundle_id,
            extractor_type,
            window.is_on_screen,
            is_blocked
        );

        self.window_states.insert(window.id, state);
    }

    /// Run one tick of the router
    pub async fn tick(&mut self) -> Result<(), ExtractionError> {
        if self.paused {
            return Ok(());
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        self.last_tick = now;

        trace!("Router tick, elapsed: {:?}", elapsed);

        // Refresh window list
        let changes = self.window_tracker.refresh_windows();

        // Handle window changes
        self.handle_window_changes(&changes).await;

        // Process windows that need extraction
        self.process_windows().await?;

        // Cleanup stale data
        self.cleanup();

        Ok(())
    }

    /// Handle window creation, destruction, and changes
    async fn handle_window_changes(&mut self, changes: &WindowChanges) {
        // Handle new windows - just track them, don't extract yet
        // Extraction will happen when they become active
        for window in &changes.created {
            self.init_window_state(window.clone());
        }

        // Handle destroyed windows
        for window_id in &changes.destroyed {
            self.window_states.remove(window_id);
            self.change_detector.remove(*window_id);
        }

        // Handle title changes (tab switches) - only for the active window
        if self.config.change_detection.title_change_triggers_extract {
            let active_id = self.window_tracker.get_active_window().map(|w| w.id);
            for (window_id, new_title) in &changes.title_changed {
                // Only extract if this is the active window
                if active_id == Some(*window_id) {
                    self.trigger_extraction(*window_id, ExtractionTrigger::TitleChanged {
                        window_id: *window_id,
                        new_title: new_title.clone(),
                    }).await;
                }
            }
        }

        // Handle focus changes - extract the newly focused window
        for (_display_id, window_id) in &changes.focus_changed {
            if let Some(window) = self.window_tracker.get_window(*window_id) {
                info!("Focus changed to: {} ({})", window.title, window.app_name);
                self.trigger_extraction(*window_id, ExtractionTrigger::AppActivated {
                    bundle_id: window.bundle_id.clone(),
                }).await;
            }
        }
    }

    /// Process the active window for potential extraction
    /// Only processes the single currently active/focused window
    async fn process_windows(&mut self) -> Result<(), ExtractionError> {
        // Get the single active/focused window
        let active_window = self.window_tracker.get_active_window().cloned();
        
        let window = match active_window {
            Some(w) => w,
            None => return Ok(()), // No active window
        };
        
        let window_id = window.id;
        
        let state = match self.window_states.get(&window_id) {
            Some(s) => s,
            None => return Ok(()), // Window not tracked
        };
        
        // Skip blocked windows
        if state.is_blocked {
            return Ok(());
        }

        // Skip Chrome windows (they push content to us)
        if state.extractor_type == ExtractorType::Chrome {
            return Ok(());
        }

        // Skip windows not on screen (minimized, different Space)
        if !window.is_on_screen {
            trace!("Skipping off-screen window {}", window_id);
            return Ok(());
        }

        // Check if enough time has passed since last extraction
        if let Some(last) = state.last_extraction {
            let elapsed = chrono::Utc::now().signed_duration_since(last);
            if elapsed.num_seconds() < self.config.timing.min_interval_seconds as i64 {
                return Ok(());
            }
        }

        // For OCR windows, capture and check perceptual hash
        if state.extractor_type == ExtractorType::Ocr {
            debug!("Processing OCR window: {} ({})", window.title, window_id);
            match self.ocr_extractor.capture_window_image(&window).await {
                Ok(image_path) => {
                    debug!("Captured window {} to {:?}", window_id, image_path);
                    // Load image and check perceptual hash
                    match self.ocr_extractor.load_image(&image_path) {
                        Ok(image) => {
                            if self.change_detector.has_changed(window_id, &image) {
                                info!("OCR window {} content changed, triggering extraction", window_id);
                                self.trigger_extraction(window_id, ExtractionTrigger::ContentChanged {
                                    window_id,
                                }).await;
                            } else {
                                debug!("OCR window {} unchanged (perceptual hash)", window_id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to load captured image for {}: {}", window_id, e);
                            self.trigger_extraction(window_id, ExtractionTrigger::TimerTick).await;
                        }
                    }
                    // Clean up temp file
                    let _ = std::fs::remove_file(&image_path);
                }
                Err(e) => {
                    // If capture fails, fall back to always extracting
                    warn!("OCR capture failed for {}: {}, falling back to extraction", window_id, e);
                    self.trigger_extraction(window_id, ExtractionTrigger::TimerTick).await;
                }
            }
            return Ok(());
        }

        // For accessibility windows, capture and check perceptual hash
        // Note: This uses CGWindowListCreateImage which is deprecated on macOS 15+
        // If capture fails, we rely on focus/title change events instead
        let bounds = window.bounds;
        if let Some(image) = self.capture_service.capture_window(window_id, &bounds) {
            if self.change_detector.has_changed(window_id, &image) {
                self.trigger_extraction(window_id, ExtractionTrigger::ContentChanged {
                    window_id,
                }).await;
            }
        }

        Ok(())
    }

    /// Trigger extraction for a window
    async fn trigger_extraction(&mut self, window_id: WindowId, trigger: ExtractionTrigger) {
        let state = match self.window_states.get(&window_id) {
            Some(s) => s,
            None => return,
        };

        if state.is_blocked {
            debug!("Skipping blocked window {}", window_id);
            return;
        }

        let window = match self.window_tracker.get_window(window_id) {
            Some(w) => w.clone(),
            None => return,
        };

        // Check if the required extractor is available
        let extractor_available = match state.extractor_type {
            ExtractorType::Accessibility => {
                self.config.extractors.accessibility_enabled && self.accessibility_extractor.is_available()
            }
            ExtractorType::Ocr => {
                self.config.extractors.ocr_enabled && self.ocr_extractor.is_available()
            }
            ExtractorType::Chrome => true, // Chrome pushes to us
        };

        if !extractor_available {
            trace!("Skipping {} - {:?} extractor not available", window.title, state.extractor_type);
            return;
        }

        debug!(
            "Triggering {:?} extraction for {} ({:?})",
            state.extractor_type, window.title, trigger
        );

        let result = match state.extractor_type {
            ExtractorType::Accessibility => {
                self.accessibility_extractor.extract(&window).await
            }
            ExtractorType::Ocr => {
                // Use the OCR extractor's window-id mode which uses ScreenCaptureKit
                // This handles capture internally, avoiding the deprecated CGWindowListCreateImage
                info!("🔍 Attempting OCR extraction for {} ({})", window.title, window.bundle_id);
                self.ocr_extractor.extract_window(&window).await
            }
            ExtractorType::Chrome => {
                // Chrome pushes to us, shouldn't reach here
                return;
            }
        };

        match result {
            Ok(content) => {
                self.handle_extracted_content(window_id, content).await;
            }
            Err(e) => {
                warn!("Extraction failed for {}: {}", window.title, e);
            }
        }
    }

    /// Handle successfully extracted content
    async fn handle_extracted_content(&mut self, window_id: WindowId, mut content: ExtractedContent) {
        // Apply privacy redaction
        content.content = self.privacy_filter.redact(&content.content);

        // Update window state
        if let Some(state) = self.window_states.get_mut(&window_id) {
            state.last_extraction = Some(chrono::Utc::now());
            state.extraction_count += 1;

            // Compute content hash for dedup
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(content.content.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            // Skip if content hasn't changed
            if state.last_content_hash.as_ref() == Some(&hash) {
                debug!("Content unchanged for {}, skipping", window_id);
                return;
            }
            state.last_content_hash = Some(hash);
        }

        info!(
            "📥 Extracted: {} - {} ({} chars)",
            content.source,
            content.title.as_deref().unwrap_or("untitled"),
            content.content.len()
        );

        // Convert to capture payload and send
        let payload: CapturePayload = content.into();

        if let Err(e) = self.content_tx.send(payload).await {
            error!("Failed to send content: {}", e);
        }
    }

    /// Handle content pushed from Chrome extension
    pub async fn handle_chrome_content(&mut self, content: ExtractedContent) {
        // Apply privacy redaction
        let mut content = content;
        content.content = self.privacy_filter.redact(&content.content);

        // Convert and send
        let payload: CapturePayload = content.into();

        if let Err(e) = self.content_tx.send(payload).await {
            error!("Failed to send Chrome content: {}", e);
        }
    }

    /// Cleanup stale data
    fn cleanup(&mut self) {
        let active_ids: Vec<_> = self.window_states.keys().copied().collect();
        self.change_detector.cleanup(&active_ids);
        self.capture_service.cleanup(&active_ids);
    }

    /// Pause the router
    pub fn pause(&mut self) {
        info!("Router paused");
        self.paused = true;
    }

    /// Resume the router
    pub fn resume(&mut self) {
        info!("Router resumed");
        self.paused = false;
    }

    /// Check if router is paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Get router status
    pub fn status(&self) -> RouterStatus {
        RouterStatus {
            paused: self.paused,
            displays: self.window_tracker.displays().len(),
            windows: self.window_states.len(),
            extractions_total: self.window_states.values().map(|s| s.extraction_count).sum(),
        }
    }

    /// Block an app at runtime
    pub fn block_app(&mut self, bundle_id: &str) {
        self.privacy_filter.block_app(bundle_id);

        // Update existing window states
        for state in self.window_states.values_mut() {
            if state.info.bundle_id == bundle_id {
                state.is_blocked = true;
            }
        }
    }

    /// Unblock an app at runtime
    pub fn unblock_app(&mut self, bundle_id: &str) {
        self.privacy_filter.unblock_app(bundle_id);

        // Update existing window states
        for state in self.window_states.values_mut() {
            if state.info.bundle_id == bundle_id {
                state.is_blocked = false;
            }
        }
    }

    /// Get the Chrome content receiver
    pub fn take_chrome_receiver(&mut self) -> Option<mpsc::Receiver<ExtractedContent>> {
        self.chrome_client.take_receiver()
    }
}

/// Router status information
#[derive(Debug, Clone)]
pub struct RouterStatus {
    pub paused: bool,
    pub displays: usize,
    pub windows: usize,
    pub extractions_total: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_status() {
        let status = RouterStatus {
            paused: false,
            displays: 2,
            windows: 5,
            extractions_total: 100,
        };

        assert!(!status.paused);
        assert_eq!(status.displays, 2);
    }
}
