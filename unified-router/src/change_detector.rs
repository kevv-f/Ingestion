//! Change detection using perceptual hashing.
//!
//! This module provides perceptual hash-based change detection for windows.
//! It uses the average hash (aHash) algorithm to detect visual changes
//! without expensive pixel-by-pixel comparison.

use crate::types::WindowId;
use image::DynamicImage;
use std::collections::HashMap;
use tracing::{debug, trace};

/// Hash size (8x8 = 64 bits)
const HASH_SIZE: u32 = 8;

/// Perceptual hash value (64-bit)
pub type PerceptualHash = u64;

/// Change detector that tracks visual changes per window
pub struct ChangeDetector {
    /// Hash per window
    window_hashes: HashMap<WindowId, PerceptualHash>,
    /// Hamming distance threshold for "changed"
    threshold: u32,
}

impl ChangeDetector {
    /// Create a new change detector with the given threshold
    pub fn new(threshold: u32) -> Self {
        Self {
            window_hashes: HashMap::new(),
            threshold,
        }
    }

    /// Check if a window's content has changed
    ///
    /// Returns `true` if:
    /// - This is the first time seeing this window
    /// - The visual hash differs by more than the threshold
    pub fn has_changed(&mut self, window_id: WindowId, image: &DynamicImage) -> bool {
        let current_hash = compute_ahash(image);

        let changed = match self.window_hashes.get(&window_id) {
            Some(&prev_hash) => {
                let distance = hamming_distance(current_hash, prev_hash);
                trace!(
                    "Window {} hash distance: {} (threshold: {})",
                    window_id,
                    distance,
                    self.threshold
                );
                distance >= self.threshold
            }
            None => {
                trace!("Window {} first seen, marking as changed", window_id);
                true
            }
        };

        if changed {
            self.window_hashes.insert(window_id, current_hash);
        }

        changed
    }

    /// Check multiple windows and return IDs of changed ones
    pub fn check_batch(&mut self, windows: &[(WindowId, DynamicImage)]) -> Vec<WindowId> {
        windows
            .iter()
            .filter_map(|(id, image)| {
                if self.has_changed(*id, image) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the current hash for a window (if known)
    pub fn get_hash(&self, window_id: WindowId) -> Option<PerceptualHash> {
        self.window_hashes.get(&window_id).copied()
    }

    /// Remove hash for a window (e.g., when window is destroyed)
    pub fn remove(&mut self, window_id: WindowId) {
        self.window_hashes.remove(&window_id);
    }

    /// Clean up hashes for windows that no longer exist
    pub fn cleanup(&mut self, active_window_ids: &[WindowId]) {
        let to_remove: Vec<_> = self
            .window_hashes
            .keys()
            .filter(|id| !active_window_ids.contains(id))
            .copied()
            .collect();

        for id in to_remove {
            debug!("Cleaning up hash for destroyed window {}", id);
            self.window_hashes.remove(&id);
        }
    }

    /// Get the number of tracked windows
    pub fn tracked_count(&self) -> usize {
        self.window_hashes.len()
    }

    /// Update the threshold
    pub fn set_threshold(&mut self, threshold: u32) {
        self.threshold = threshold;
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new(8) // Default threshold
    }
}

/// Compute average hash (aHash) for an image
///
/// Algorithm:
/// 1. Resize to 8x8
/// 2. Convert to grayscale
/// 3. Calculate average brightness
/// 4. Generate 64-bit hash: bit=1 if pixel > average, else 0
pub fn compute_ahash(image: &DynamicImage) -> PerceptualHash {
    // Resize to 8x8
    let resized = image.resize_exact(HASH_SIZE, HASH_SIZE, image::imageops::FilterType::Nearest);

    // Convert to grayscale
    let gray = resized.to_luma8();

    // Calculate average brightness
    let sum: u32 = gray.pixels().map(|p| p.0[0] as u32).sum();
    let avg = (sum / (HASH_SIZE * HASH_SIZE)) as u8;

    // Build hash
    let mut hash: PerceptualHash = 0;
    for (i, pixel) in gray.pixels().enumerate() {
        if pixel.0[0] > avg {
            hash |= 1 << i;
        }
    }

    hash
}

/// Compute difference hash (dHash) for an image
///
/// Algorithm:
/// 1. Resize to 9x8 (one extra column)
/// 2. Convert to grayscale
/// 3. Compare adjacent pixels horizontally
/// 4. Generate 64-bit hash: bit=1 if left > right
#[allow(dead_code)]
pub fn compute_dhash(image: &DynamicImage) -> PerceptualHash {
    // Resize to 9x8 (need extra column for comparison)
    let resized = image.resize_exact(HASH_SIZE + 1, HASH_SIZE, image::imageops::FilterType::Nearest);

    // Convert to grayscale
    let gray = resized.to_luma8();

    // Build hash by comparing adjacent pixels
    let mut hash: PerceptualHash = 0;
    let mut bit = 0;

    for y in 0..HASH_SIZE {
        for x in 0..HASH_SIZE {
            let left = gray.get_pixel(x, y).0[0];
            let right = gray.get_pixel(x + 1, y).0[0];

            if left > right {
                hash |= 1 << bit;
            }
            bit += 1;
        }
    }

    hash
}

/// Calculate Hamming distance between two hashes
///
/// Returns the number of bits that differ (0-64)
pub fn hamming_distance(a: PerceptualHash, b: PerceptualHash) -> u32 {
    (a ^ b).count_ones()
}

/// Convert hash to hex string for debugging
pub fn hash_to_hex(hash: PerceptualHash) -> String {
    format!("{:016x}", hash)
}

/// Convert hash to binary string for debugging
#[allow(dead_code)]
pub fn hash_to_binary(hash: PerceptualHash) -> String {
    format!("{:064b}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn create_test_image(brightness: u8) -> DynamicImage {
        let mut img = RgbImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([brightness, brightness, brightness]);
        }
        DynamicImage::ImageRgb8(img)
    }

    fn create_half_image() -> DynamicImage {
        let mut img = RgbImage::new(100, 100);
        for (x, _, pixel) in img.enumerate_pixels_mut() {
            let brightness = if x < 50 { 0 } else { 255 };
            *pixel = Rgb([brightness, brightness, brightness]);
        }
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(0, 1), 1);
        assert_eq!(hamming_distance(0, 0xFF), 8);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
    }

    #[test]
    fn test_identical_images_same_hash() {
        let img1 = create_test_image(128);
        let img2 = create_test_image(128);

        let hash1 = compute_ahash(&img1);
        let hash2 = compute_ahash(&img2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_images_different_hash() {
        let img1 = create_test_image(0);   // Black
        let img2 = create_test_image(255); // White

        let hash1 = compute_ahash(&img1);
        let hash2 = compute_ahash(&img2);

        // Very different images should have high distance
        let distance = hamming_distance(hash1, hash2);
        assert!(distance > 30, "Distance was {}", distance);
    }

    #[test]
    fn test_change_detector_first_seen() {
        let mut detector = ChangeDetector::new(8);
        let img = create_test_image(128);

        // First time should always be "changed"
        assert!(detector.has_changed(1, &img));
    }

    #[test]
    fn test_change_detector_same_image() {
        let mut detector = ChangeDetector::new(8);
        let img = create_test_image(128);

        // First time
        assert!(detector.has_changed(1, &img));

        // Same image again - should not be changed
        assert!(!detector.has_changed(1, &img));
    }

    #[test]
    fn test_change_detector_different_image() {
        let mut detector = ChangeDetector::new(8);
        let img1 = create_test_image(0);
        let img2 = create_half_image();

        // First time
        assert!(detector.has_changed(1, &img1));

        // Different image - should be changed
        assert!(detector.has_changed(1, &img2));
    }

    #[test]
    fn test_change_detector_cleanup() {
        let mut detector = ChangeDetector::new(8);
        let img = create_test_image(128);

        detector.has_changed(1, &img);
        detector.has_changed(2, &img);
        detector.has_changed(3, &img);

        assert_eq!(detector.tracked_count(), 3);

        // Only window 2 still exists
        detector.cleanup(&[2]);

        assert_eq!(detector.tracked_count(), 1);
        assert!(detector.get_hash(2).is_some());
        assert!(detector.get_hash(1).is_none());
    }

    #[test]
    fn test_hash_to_hex() {
        assert_eq!(hash_to_hex(0), "0000000000000000");
        assert_eq!(hash_to_hex(255), "00000000000000ff");
        assert_eq!(hash_to_hex(u64::MAX), "ffffffffffffffff");
    }
}
