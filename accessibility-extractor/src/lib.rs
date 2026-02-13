//! Accessibility Extractor - Extract content from desktop applications using Accessibility APIs.
//!
//! This crate provides functionality to extract text content from macOS desktop applications
//! using the native Accessibility API (AXUIElement).
//!
//! # Overview
//!
//! The `accessibility-extractor` crate provides a cross-platform API for extracting text
//! content from desktop applications using accessibility APIs. Currently, macOS is the
//! only supported platform.
//!
//! # Quick Start
//!
//! ```no_run
//! use accessibility_extractor::AccessibilityExtractor;
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
//! # Modules
//!
//! - [`types`]: Core data types (ExtractedContent, ExtractionError, AppSource, CapturePayload)
//! - [`platform`]: Platform-specific implementations (macOS) - only available on macOS
//! - [`extractor`]: Cross-platform API wrapper (AccessibilityExtractor)
//! - [`storage_bridge`]: SQLite storage with deduplication for the daemon
//!
//! # Platform Support
//!
//! Currently, only macOS is supported. On unsupported platforms, the `AccessibilityExtractor`
//! methods will return appropriate errors or default values.
//!
//! # Requirements
//! - Requirement 9.1: Provide an AccessibilityExtractor struct with a unified API

// Declare modules
pub mod extractor;
pub mod types;
pub mod storage_bridge;

// Platform-specific modules are conditionally compiled
// Requirement 9.7: Return appropriate errors on unsupported platforms
#[cfg(target_os = "macos")]
pub mod platform;

// Re-export public types for convenience
// Requirement 9.1: Provide an AccessibilityExtractor struct with a unified API
pub use extractor::AccessibilityExtractor;

// Re-export core types (ExtractedContent, CapturePayload, ExtractionError, AppSource)
pub use types::{AppSource, CapturePayload, ChunkMeta, ExtractedContent, ExtractionError};

// Re-export database integration functions
pub use types::{generate_content_hash, generate_doc_id};

// Re-export storage bridge
pub use storage_bridge::{DaemonStorage, DedupResult, StorageError};
