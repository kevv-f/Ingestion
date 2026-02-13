//! Platform-specific implementations for accessibility content extraction.
//!
//! This module provides platform-specific implementations for accessibility
//! content extraction. Currently only macOS is supported.
//!
//! # Platform Support
//!
//! - **macOS**: Full support via the native Accessibility API (AXUIElement)
//! - **Other platforms**: Not currently supported. The [`AccessibilityExtractor`](crate::AccessibilityExtractor)
//!   will return appropriate errors when running on unsupported platforms.
//!
//! # Conditional Compilation
//!
//! The macOS module is only compiled when targeting macOS (`target_os = "macos"`).
//! This ensures that platform-specific dependencies are not included on other platforms.
//!
//! # Requirements
//! - Requirement 9.7: WHEN running on unsupported platforms, THE AccessibilityExtractor
//!   SHALL return appropriate errors

/// macOS platform-specific implementation.
///
/// This module provides the macOS-specific implementation using the native
/// Accessibility API (AXUIElement) to extract content from desktop applications.
///
/// Only available when compiling for macOS (`target_os = "macos"`).
#[cfg(target_os = "macos")]
pub mod macos;
