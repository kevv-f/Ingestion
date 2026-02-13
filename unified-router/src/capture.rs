//! Window and display capture functionality.
//!
//! This module provides screenshot capture for specific windows or entire displays,
//! used for perceptual hash computation and OCR extraction.

use crate::types::{DisplayId, WindowBounds, WindowId};
use image::{DynamicImage, RgbaImage};
use std::collections::HashMap;
use tracing::{debug, trace, warn};

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use core_graphics::display::CGDisplayBounds;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::image::CGImage;
    use core_graphics::window::{
        kCGWindowImageBestResolution, kCGWindowImageBoundsIgnoreFraming,
        kCGWindowListOptionIncludingWindow, CGWindowListCreateImage,
    };
    use foreign_types_shared::ForeignType;

    /// Capture a specific window by ID
    pub fn capture_window(window_id: WindowId, bounds: &WindowBounds) -> Option<RgbaImage> {
        let rect = CGRect::new(
            &CGPoint::new(bounds.x as f64, bounds.y as f64),
            &CGSize::new(bounds.width as f64, bounds.height as f64),
        );

        let options = kCGWindowImageBoundsIgnoreFraming | kCGWindowImageBestResolution;

        let cg_image: CGImage = unsafe {
            let image_ref = CGWindowListCreateImage(
                rect,
                kCGWindowListOptionIncludingWindow,
                window_id as u32,
                options,
            );
            if image_ref.is_null() {
                return None;
            }
            CGImage::from_ptr(image_ref)
        };

        convert_cgimage_to_rgba(&cg_image)
    }

    /// Capture an entire display
    pub fn capture_display(display_id: DisplayId) -> Option<RgbaImage> {
        let bounds = unsafe { CGDisplayBounds(display_id) };

        let rect = CGRect::new(
            &CGPoint::new(bounds.origin.x, bounds.origin.y),
            &CGSize::new(bounds.size.width, bounds.size.height),
        );

        // Use 0 for window ID to capture the entire display
        let cg_image: CGImage = unsafe {
            let image_ref = CGWindowListCreateImage(
                rect,
                0, // kCGWindowListOptionAll
                0, // kCGNullWindowID
                kCGWindowImageBestResolution,
            );
            if image_ref.is_null() {
                return None;
            }
            CGImage::from_ptr(image_ref)
        };

        convert_cgimage_to_rgba(&cg_image)
    }

    /// Convert CGImage to image crate's RgbaImage
    fn convert_cgimage_to_rgba(cg_image: &CGImage) -> Option<RgbaImage> {
        let width = cg_image.width();
        let height = cg_image.height();
        let bytes_per_row = cg_image.bytes_per_row();
        let bits_per_pixel = cg_image.bits_per_pixel();

        // Get raw pixel data
        let data = cg_image.data();
        let bytes = data.bytes();

        if bytes.is_empty() {
            return None;
        }

        // CGImage is typically BGRA or RGBA depending on the display
        // We need to handle both cases
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            let row_start = y * bytes_per_row;
            for x in 0..width {
                let pixel_start = row_start + x * (bits_per_pixel / 8);
                if pixel_start + 3 < bytes.len() {
                    // Assume BGRA format (common on macOS)
                    let b = bytes[pixel_start];
                    let g = bytes[pixel_start + 1];
                    let r = bytes[pixel_start + 2];
                    let a = bytes[pixel_start + 3];
                    rgba_data.extend_from_slice(&[r, g, b, a]);
                }
            }
        }

        RgbaImage::from_raw(width as u32, height as u32, rgba_data)
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use super::*;

    pub fn capture_window(_window_id: WindowId, _bounds: &WindowBounds) -> Option<RgbaImage> {
        None
    }

    pub fn capture_display(_display_id: DisplayId) -> Option<RgbaImage> {
        None
    }
}

/// Window capture service
pub struct CaptureService {
    /// Cache of recent captures (for debugging/metrics)
    last_capture_times: HashMap<WindowId, std::time::Instant>,
    /// Windows that failed to capture (likely on different Space)
    uncapturable_windows: HashMap<WindowId, std::time::Instant>,
}

impl CaptureService {
    pub fn new() -> Self {
        Self {
            last_capture_times: HashMap::new(),
            uncapturable_windows: HashMap::new(),
        }
    }

    /// Check if a window is known to be uncapturable
    pub fn is_uncapturable(&self, window_id: WindowId) -> bool {
        if let Some(time) = self.uncapturable_windows.get(&window_id) {
            // Consider uncapturable for 30 seconds, then retry
            time.elapsed().as_secs() < 30
        } else {
            false
        }
    }

    /// Capture a specific window
    pub fn capture_window(&mut self, window_id: WindowId, bounds: &WindowBounds) -> Option<DynamicImage> {
        // Skip if recently failed
        if self.is_uncapturable(window_id) {
            trace!("Skipping uncapturable window {}", window_id);
            return None;
        }

        trace!("Capturing window {} at {:?}", window_id, bounds);

        let start = std::time::Instant::now();
        let result = macos::capture_window(window_id, bounds);
        let elapsed = start.elapsed();

        if result.is_some() {
            trace!("Window {} captured in {:?}", window_id, elapsed);
            self.last_capture_times.insert(window_id, std::time::Instant::now());
            // Remove from uncapturable if it was there
            self.uncapturable_windows.remove(&window_id);
        } else {
            // Mark as uncapturable (likely on different Space)
            debug!("Window {} not capturable (likely on different Space)", window_id);
            self.uncapturable_windows.insert(window_id, std::time::Instant::now());
        }

        result.map(DynamicImage::ImageRgba8)
    }

    /// Capture an entire display
    pub fn capture_display(&self, display_id: DisplayId) -> Option<DynamicImage> {
        trace!("Capturing display {}", display_id);

        let start = std::time::Instant::now();
        let result = macos::capture_display(display_id);
        let elapsed = start.elapsed();

        if result.is_some() {
            debug!("Display {} captured in {:?}", display_id, elapsed);
        } else {
            warn!("Failed to capture display {}", display_id);
        }

        result.map(DynamicImage::ImageRgba8)
    }

    /// Capture multiple windows in batch
    pub fn capture_windows(&mut self, windows: &[(WindowId, WindowBounds)]) -> HashMap<WindowId, DynamicImage> {
        let mut captures = HashMap::new();

        for (window_id, bounds) in windows {
            if let Some(image) = self.capture_window(*window_id, bounds) {
                captures.insert(*window_id, image);
            }
        }

        debug!("Captured {}/{} windows", captures.len(), windows.len());
        captures
    }

    /// Get time since last capture for a window
    pub fn time_since_capture(&self, window_id: WindowId) -> Option<std::time::Duration> {
        self.last_capture_times
            .get(&window_id)
            .map(|t| t.elapsed())
    }

    /// Clean up tracking for destroyed windows
    pub fn cleanup(&mut self, active_window_ids: &[WindowId]) {
        self.last_capture_times
            .retain(|id, _| active_window_ids.contains(id));
        self.uncapturable_windows
            .retain(|id, _| active_window_ids.contains(id));
    }
}

impl Default for CaptureService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_service_new() {
        let service = CaptureService::new();
        assert!(service.last_capture_times.is_empty());
    }

    #[test]
    fn test_time_since_capture_none() {
        let service = CaptureService::new();
        assert!(service.time_since_capture(12345).is_none());
    }
}
