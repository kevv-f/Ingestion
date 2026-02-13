//! Unified Router - Content extraction orchestrator
//!
//! This crate provides a unified interface for extracting content from desktop
//! applications using multiple extraction methods:
//!
//! - **Accessibility**: For apps with good accessibility support (Office, iWork)
//! - **Chrome Extension**: For web browsers with the extension installed
//! - **OCR**: Fallback for apps without accessibility support
//!
//! # Architecture
//!
//! The router monitors all visible windows across all displays, determines the
//! optimal extraction method for each application, and coordinates extraction
//! timing using perceptual hash-based change detection.

pub mod capture;
pub mod change_detector;
pub mod config;
pub mod extractors;
pub mod privacy;
pub mod router;
pub mod types;
pub mod window_tracker;

// Re-export commonly used types
pub use capture::CaptureService;
pub use change_detector::{compute_ahash, hamming_distance, ChangeDetector, PerceptualHash};
pub use config::Config;
pub use extractors::ExtractorRegistry;
pub use privacy::PrivacyFilter;
pub use privacy::ALWAYS_BLACKLISTED_APPS;
pub use privacy::ALWAYS_BLACKLISTED_PATTERNS;
pub use router::{RouterStatus, UnifiedRouter};
pub use types::{
    CapturePayload, DisplayId, DisplayInfo, ExtractedContent, ExtractionError,
    ExtractionTrigger, ExtractorType, WindowBounds, WindowId, WindowInfo, WindowState,
};
pub use window_tracker::{WindowChanges, WindowTracker};
