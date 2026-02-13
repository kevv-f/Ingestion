# Accessibility API Content Extraction: Technical Implementation Guide

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Project Setup](#project-setup)
4. [macOS Implementation](#macos-implementation)
5. [Windows Implementation](#windows-implementation)
6. [Cross-Platform API](#cross-platform-api)
7. [Application-Specific Extraction](#application-specific-extraction)
8. [Event-Driven Architecture](#event-driven-architecture)
9. [Testing & Debugging](#testing--debugging)
10. [Deployment Considerations](#deployment-considerations)

---

## Overview

### What This Document Covers

This guide provides a complete implementation specification for extracting text content from desktop applications (Microsoft Word, Excel, PowerPoint, Outlook, etc.) using platform-native Accessibility APIs.

### Why Accessibility APIs?

| Approach | Admin Consent | Licensing Cost | Data Location | Works Offline |
|----------|--------------|----------------|---------------|---------------|
| **Accessibility API** | None | None | Local only | Yes |
| Microsoft Graph API | Required | E5 or per-message | Cloud | No |
| File parsing | None | None | Local only | Yes |

Accessibility APIs are designed for assistive technologies (screen readers) but can be used to read UI content from any application that properly implements accessibility.

### Supported Applications

| Application | Windows Support | macOS Support | Extraction Quality |
|-------------|-----------------|---------------|-------------------|
| Microsoft Word | ✅ Full | ✅ Full | Excellent |
| Microsoft Excel | ✅ Full | ⚠️ Partial | Good (cell-by-cell) |
| Microsoft PowerPoint | ✅ Full | ⚠️ Partial | Good (slide-by-slide) |
| Microsoft Outlook | ✅ Full | ✅ Full | Excellent |
| Apple Pages | N/A | ✅ Full | Excellent |
| LibreOffice Writer | ✅ Full | ✅ Full | Excellent |
| Notepad / TextEdit | ✅ Full | ✅ Full | Excellent |

### Permission Requirements

| Platform | Permission Type | User Action Required |
|----------|-----------------|---------------------|
| **macOS** | Accessibility Permission | One-time grant in System Preferences → Privacy & Security → Accessibility |
| **Windows** | None | Works out of the box |

---

## Architecture

### High-Level System Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Content Extraction System                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                    Accessibility Extractor (Rust)               │     │
│  │                                                                 │     │
│  │  ┌─────────────────────┐       ┌─────────────────────┐         │     │
│  │  │   macOS Module      │       │   Windows Module    │         │     │
│  │  │                     │       │                     │         │     │
│  │  │  ┌───────────────┐  │       │  ┌───────────────┐  │         │     │
│  │  │  │ accessibility │  │       │  │ uiautomation  │  │         │     │
│  │  │  │ crate (0.2)   │  │       │  │ crate (0.19)  │  │         │     │
│  │  │  └───────┬───────┘  │       │  └───────┬───────┘  │         │     │
│  │  │          │          │       │          │          │         │     │
│  │  │          ▼          │       │          ▼          │         │     │
│  │  │  ┌───────────────┐  │       │  ┌───────────────┐  │         │     │
│  │  │  │ AXUIElement   │  │       │  │ IUIAutomation │  │         │     │
│  │  │  │ (macOS API)   │  │       │  │ (Windows API) │  │         │     │
│  │  │  └───────────────┘  │       │  └───────────────┘  │         │     │
│  │  └─────────────────────┘       └─────────────────────┘         │     │
│  │                                                                 │     │
│  │  ┌─────────────────────────────────────────────────────────┐   │     │
│  │  │              Unified Cross-Platform API                  │   │     │
│  │  │  - extract_frontmost() -> ExtractedContent              │   │     │
│  │  │  - is_enabled() -> bool                                 │   │     │
│  │  │  - request_permissions()                                │   │     │
│  │  └─────────────────────────────────────────────────────────┘   │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                                    │                                     │
│                                    ▼                                     │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                      Ingestion Service                          │     │
│  │  - Receives ExtractedContent                                   │     │
│  │  - Chunks, deduplicates, stores                                │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```


### Data Flow

```
1. User opens Microsoft Word with a document
                    │
                    ▼
2. Accessibility Extractor detects app focus change (or polls)
                    │
                    ▼
3. Extractor queries accessibility tree:
   - Get frontmost application
   - Get focused window
   - Traverse element tree for text content
                    │
                    ▼
4. Extract text using:
   - macOS: AXUIElement.value() / children traversal
   - Windows: TextPattern.DocumentRange.GetText()
                    │
                    ▼
5. Package into ExtractedContent struct:
   {
     source: "word",
     title: "Document1.docx",
     content: "Full document text...",
     timestamp: 1707500000,
     extraction_method: "accessibility"
   }
                    │
                    ▼
6. Send to Ingestion Service via existing pipeline
```

---

## Project Setup

### Directory Structure

```
accessibility-extractor/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Library entry point
│   ├── main.rs                # CLI/service entry point
│   ├── extractor.rs           # Cross-platform unified API
│   ├── types.rs               # Shared types (ExtractedContent, etc.)
│   ├── platform/
│   │   ├── mod.rs             # Platform module exports
│   │   ├── macos/
│   │   │   ├── mod.rs         # macOS implementation
│   │   │   ├── element.rs     # AXUIElement helpers
│   │   │   └── permissions.rs # Permission handling
│   │   └── windows/
│   │       ├── mod.rs         # Windows implementation
│   │       ├── automation.rs  # UIAutomation wrapper
│   │       └── patterns.rs    # TextPattern helpers
│   └── apps/
│       ├── mod.rs             # App-specific extractors
│       ├── word.rs            # Microsoft Word specifics
│       ├── excel.rs           # Microsoft Excel specifics
│       └── powerpoint.rs      # Microsoft PowerPoint specifics
└── tests/
    ├── macos_tests.rs
    └── windows_tests.rs
```

### Cargo.toml (Complete)

```toml
[package]
name = "accessibility-extractor"
version = "0.1.0"
edition = "2021"
description = "Extract content from desktop applications using Accessibility APIs"

[lib]
name = "accessibility_extractor"
path = "src/lib.rs"

[[bin]]
name = "ax-extractor"
path = "src/main.rs"

[dependencies]
# Cross-platform dependencies
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"
tokio = { version = "1.0", features = ["full"], optional = true }

# Windows-specific dependencies
[target.'cfg(windows)'.dependencies]
uiautomation = { version = "0.19", features = ["pattern", "control", "event"] }
winapi = { version = "0.3", features = ["winuser", "processthreadsapi"] }
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_Accessibility",
    "Win32_System_Com",
]}

# macOS-specific dependencies
[target.'cfg(target_os = "macos")'.dependencies]
accessibility = "0.2"
accessibility-sys = "0.2"
core-foundation = "0.10"
macos-accessibility-client = "0.0.2"
cocoa = "0.26"
objc = "0.2"

[features]
default = []
async = ["tokio"]

[dev-dependencies]
pretty_assertions = "1.4"
```


---

## macOS Implementation

### Understanding the macOS Accessibility API

macOS provides the Accessibility API through the `ApplicationServices` framework. The API is built around `AXUIElement`, which represents any UI element in any application.

#### Key Concepts

1. **AXUIElement**: An opaque reference to a UI element (window, button, text field, etc.)
2. **Attributes**: Properties of an element (value, title, role, children, etc.)
3. **Actions**: Operations that can be performed on an element (press, increment, etc.)
4. **System-wide element**: A special element that provides access to the entire system

#### Element Hierarchy

```
System-Wide Element
    └── Application (e.g., Microsoft Word)
        └── Window (e.g., "Document1.docx - Word")
            └── Toolbar
            └── Document Area (AXScrollArea)
                └── Text Area (AXTextArea) ← Contains document text
                    └── Static Text elements
            └── Status Bar
```

### Complete macOS Implementation

#### src/platform/macos/mod.rs

```rust
//! macOS Accessibility API implementation
//! 
//! This module provides content extraction from macOS applications using the
//! native Accessibility API (AXUIElement).

mod element;
mod permissions;

pub use element::*;
pub use permissions::*;

use accessibility::{AXUIElement, AXUIElementAttributes, Error as AXError};
use accessibility::attribute::AXAttribute;
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType, CFTypeID};
use core_foundation::string::CFString;
use core_foundation::number::CFNumber;
use std::time::Duration;

use crate::types::{ExtractedContent, ExtractionError};

/// Main extractor for macOS applications
pub struct MacOSExtractor;

impl MacOSExtractor {
    /// Check if accessibility permissions are granted
    /// 
    /// # Returns
    /// `true` if the application has accessibility permissions
    /// 
    /// # Example
    /// ```rust
    /// if !MacOSExtractor::is_accessibility_enabled() {
    ///     MacOSExtractor::request_accessibility();
    /// }
    /// ```
    pub fn is_accessibility_enabled() -> bool {
        macos_accessibility_client::accessibility::application_is_trusted()
    }
    
    /// Request accessibility permissions from the user
    /// 
    /// This will display the system dialog asking the user to grant
    /// accessibility permissions. The user must:
    /// 1. Click "Open System Preferences"
    /// 2. Click the lock icon to make changes
    /// 3. Check the checkbox next to your application
    /// 4. Restart your application
    pub fn request_accessibility() {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    }
    
    /// Get the frontmost (active) application
    /// 
    /// # Returns
    /// The AXUIElement for the frontmost application, or None if unavailable
    pub fn get_frontmost_app() -> Option<AXUIElement> {
        let system_wide = AXUIElement::system_wide();
        
        // The focused application attribute gives us the frontmost app
        let focused_app_attr = AXAttribute::<AXUIElement>::new(
            &CFString::new("AXFocusedApplication")
        );
        
        system_wide.attribute(&focused_app_attr).ok()
    }
    
    /// Get the frontmost application by bundle identifier
    /// 
    /// # Arguments
    /// * `bundle_id` - The bundle identifier (e.g., "com.microsoft.Word")
    /// * `timeout` - How long to wait for the application
    /// 
    /// # Returns
    /// The AXUIElement for the application, or an error
    pub fn get_app_by_bundle_id(
        bundle_id: &str, 
        timeout: Duration
    ) -> Result<AXUIElement, ExtractionError> {
        AXUIElement::application_with_bundle_timeout(bundle_id, timeout)
            .map_err(|e| ExtractionError::AppNotFound(format!("{}: {:?}", bundle_id, e)))
    }
    
    /// Extract content from the frontmost application
    /// 
    /// This is the main entry point for content extraction. It will:
    /// 1. Get the frontmost application
    /// 2. Get the focused window
    /// 3. Traverse the element tree to find text content
    /// 4. Return the extracted content
    /// 
    /// # Returns
    /// `ExtractedContent` with the document text, or an error
    /// 
    /// # Example
    /// ```rust
    /// match MacOSExtractor::extract_frontmost() {
    ///     Ok(content) => println!("Extracted: {}", content.content),
    ///     Err(e) => eprintln!("Extraction failed: {:?}", e),
    /// }
    /// ```
    pub fn extract_frontmost() -> Result<ExtractedContent, ExtractionError> {
        // Check permissions first
        if !Self::is_accessibility_enabled() {
            return Err(ExtractionError::PermissionDenied(
                "Accessibility permission not granted. Please enable in System Preferences.".into()
            ));
        }
        
        // Get frontmost app
        let app = Self::get_frontmost_app()
            .ok_or_else(|| ExtractionError::AppNotFound("No frontmost application".into()))?;
        
        // Get app info for metadata
        let app_title = app.title()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let source = Self::detect_app_source(&app_title);
        
        // Get focused window
        let window = app.focused_window()
            .map_err(|e| ExtractionError::ElementNotFound(format!("No focused window: {:?}", e)))?;
        
        // Get window title (usually contains document name)
        let title = window.title()
            .map(|s| s.to_string())
            .ok();
        
        // Extract text content
        let content = Self::extract_text_from_element(&window)?;
        
        if content.trim().is_empty() {
            return Err(ExtractionError::NoContentFound(
                "Document appears to be empty".into()
            ));
        }
        
        Ok(ExtractedContent {
            source,
            title,
            content,
            app_name: app_title,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "accessibility".to_string(),
        })
    }
    
    /// Extract text from a specific application by bundle ID
    /// 
    /// # Arguments
    /// * `bundle_id` - The bundle identifier (e.g., "com.microsoft.Word")
    pub fn extract_from_app(bundle_id: &str) -> Result<ExtractedContent, ExtractionError> {
        let app = Self::get_app_by_bundle_id(bundle_id, Duration::from_secs(5))?;
        
        let window = app.focused_window()
            .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))?;
        
        let title = window.title().map(|s| s.to_string()).ok();
        let content = Self::extract_text_from_element(&window)?;
        
        Ok(ExtractedContent {
            source: Self::bundle_id_to_source(bundle_id),
            title,
            content,
            app_name: bundle_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "accessibility".to_string(),
        })
    }
    
    /// Recursively extract text from an element and all its children
    /// 
    /// This function traverses the accessibility tree depth-first,
    /// collecting text from elements that contain text content.
    /// 
    /// # Arguments
    /// * `element` - The root element to start extraction from
    /// 
    /// # Returns
    /// A string containing all extracted text
    fn extract_text_from_element(element: &AXUIElement) -> Result<String, ExtractionError> {
        let mut result = String::new();
        Self::extract_text_recursive(element, &mut result, 0)?;
        Ok(result)
    }
    
    /// Internal recursive text extraction
    fn extract_text_recursive(
        element: &AXUIElement, 
        result: &mut String,
        depth: usize
    ) -> Result<(), ExtractionError> {
        // Prevent infinite recursion
        if depth > 100 {
            log::warn!("Maximum recursion depth reached");
            return Ok(());
        }
        
        // Get the role of this element
        let role = element.role()
            .map(|s| s.to_string())
            .unwrap_or_default();
        
        // These roles typically contain text content
        let text_roles = [
            "AXTextArea",
            "AXTextField", 
            "AXStaticText",
            "AXDocument",
            "AXWebArea",
            "AXTextMarkerRange",
            "AXCell",  // For Excel cells
        ];
        
        if text_roles.contains(&role.as_str()) {
            // Try to get the value (text content)
            if let Ok(value) = element.value() {
                if let Some(text) = Self::cftype_to_string(&value) {
                    if !text.trim().is_empty() {
                        result.push_str(&text);
                        result.push('\n');
                    }
                }
            }
        }
        
        // Traverse children
        if let Ok(children) = element.children() {
            let count = children.len();
            for i in 0..count {
                if let Some(child) = children.get(i) {
                    Self::extract_text_recursive(&child, result, depth + 1)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Convert a Core Foundation type to a Rust String
    /// 
    /// CFType can be CFString, CFNumber, CFBoolean, etc.
    /// This function attempts to convert it to a string representation.
    fn cftype_to_string(value: &CFType) -> Option<String> {
        // Check if it's a CFString
        if value.type_of() == CFString::type_id() {
            unsafe {
                let cf_string = CFString::wrap_under_get_rule(
                    value.as_concrete_TypeRef() as *const _
                );
                return Some(cf_string.to_string());
            }
        }
        
        // Check if it's a CFNumber (convert to string)
        if value.type_of() == CFNumber::type_id() {
            unsafe {
                let cf_number = CFNumber::wrap_under_get_rule(
                    value.as_concrete_TypeRef() as *const _
                );
                if let Some(n) = cf_number.to_i64() {
                    return Some(n.to_string());
                }
                if let Some(n) = cf_number.to_f64() {
                    return Some(n.to_string());
                }
            }
        }
        
        None
    }
    
    /// Detect the application source from its title
    fn detect_app_source(title: &str) -> String {
        let title_lower = title.to_lowercase();
        
        if title_lower.contains("word") {
            "word".to_string()
        } else if title_lower.contains("excel") {
            "excel".to_string()
        } else if title_lower.contains("powerpoint") {
            "powerpoint".to_string()
        } else if title_lower.contains("outlook") {
            "outlook".to_string()
        } else if title_lower.contains("pages") {
            "pages".to_string()
        } else if title_lower.contains("numbers") {
            "numbers".to_string()
        } else if title_lower.contains("keynote") {
            "keynote".to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// Convert bundle ID to source name
    fn bundle_id_to_source(bundle_id: &str) -> String {
        match bundle_id {
            "com.microsoft.Word" => "word",
            "com.microsoft.Excel" => "excel",
            "com.microsoft.Powerpoint" => "powerpoint",
            "com.microsoft.Outlook" => "outlook",
            "com.apple.iWork.Pages" => "pages",
            "com.apple.iWork.Numbers" => "numbers",
            "com.apple.iWork.Keynote" => "keynote",
            _ => "unknown",
        }.to_string()
    }
    
    /// Get the currently selected text in any application
    /// 
    /// This is useful for extracting just the user's selection
    /// rather than the entire document.
    pub fn get_selected_text() -> Option<String> {
        let system_wide = AXUIElement::system_wide();
        
        // Get the focused UI element
        let focused_attr = AXAttribute::<AXUIElement>::new(
            &CFString::new("AXFocusedUIElement")
        );
        
        let focused = system_wide.attribute(&focused_attr).ok()?;
        
        // Get selected text from the focused element
        let selected_text_attr = AXAttribute::<CFString>::new(
            &CFString::new("AXSelectedText")
        );
        
        focused.attribute(&selected_text_attr)
            .ok()
            .map(|s| s.to_string())
    }
}
```


#### src/platform/macos/permissions.rs

```rust
//! macOS Accessibility Permission Handling
//!
//! This module handles checking and requesting accessibility permissions.

use std::process::Command;

/// Check if accessibility is enabled without prompting
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Check if accessibility is enabled, showing prompt if not
pub fn is_trusted_with_prompt() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Open System Preferences to the Accessibility pane
/// 
/// This is useful if you want to direct users to enable permissions
/// without showing the default system prompt.
pub fn open_accessibility_preferences() -> std::io::Result<()> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()?;
    Ok(())
}

/// Get instructions for enabling accessibility
pub fn get_permission_instructions() -> &'static str {
    r#"
To enable accessibility permissions:

1. Open System Preferences (or System Settings on macOS Ventura+)
2. Go to Privacy & Security → Accessibility
3. Click the lock icon to make changes
4. Find this application in the list and check the checkbox
5. If the application is not listed, click '+' and add it
6. Restart this application

Note: You may need to quit and reopen this application after 
granting permissions for them to take effect.
"#
}
```

#### src/platform/macos/element.rs

```rust
//! AXUIElement helper functions
//!
//! Provides utility functions for working with accessibility elements.

use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::string::CFString;
use core_foundation::array::CFArray;

/// Get all attribute names for an element (useful for debugging)
pub fn get_attribute_names(element: &AXUIElement) -> Vec<String> {
    element.attribute_names()
        .map(|names| {
            let mut result = Vec::new();
            for i in 0..names.len() {
                if let Some(name) = names.get(i) {
                    result.push(name.to_string());
                }
            }
            result
        })
        .unwrap_or_default()
}

/// Print the element tree for debugging
pub fn debug_print_tree(element: &AXUIElement, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    let role = element.role()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "?".to_string());
    
    let title = element.title()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "".to_string());
    
    println!("{}{} - {}", prefix, role, title);
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                debug_print_tree(&child, indent + 1);
            }
        }
    }
}

/// Find elements by role within a subtree
pub fn find_elements_by_role(
    root: &AXUIElement, 
    target_role: &str
) -> Vec<AXUIElement> {
    let mut results = Vec::new();
    find_elements_recursive(root, target_role, &mut results);
    results
}

fn find_elements_recursive(
    element: &AXUIElement,
    target_role: &str,
    results: &mut Vec<AXUIElement>
) {
    if let Ok(role) = element.role() {
        if role.to_string() == target_role {
            // Clone the element reference
            results.push(element.clone());
        }
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_elements_recursive(&child, target_role, results);
            }
        }
    }
}
```

---

## Windows Implementation

### Understanding Windows UI Automation

Windows UI Automation is a COM-based API that provides programmatic access to UI elements. It's more structured than the macOS API, with specific "patterns" for different types of interactions.

#### Key Concepts

1. **IUIAutomation**: The main entry point for UI Automation
2. **IUIAutomationElement**: Represents any UI element
3. **Control Patterns**: Interfaces for specific functionality:
   - **TextPattern**: For reading text content
   - **ValuePattern**: For getting/setting values
   - **ScrollPattern**: For scrolling
4. **Tree Walker**: For navigating the element tree
5. **Conditions**: For filtering elements when searching

#### Element Hierarchy (Word Example)

```
Desktop
└── Window (ClassName: "OpusApp", Name: "Document1 - Word")
    └── Document (ControlType: Document)
        └── Text (ControlType: Text) ← Supports TextPattern
            └── Contains full document text via DocumentRange
```

### Complete Windows Implementation

#### src/platform/windows/mod.rs

```rust
//! Windows UI Automation implementation
//!
//! This module provides content extraction from Windows applications using
//! the UI Automation API.

mod automation;
mod patterns;

pub use automation::*;
pub use patterns::*;

use uiautomation::{UIAutomation, UIElement, UITreeWalker};
use uiautomation::patterns::{TextPattern, ValuePattern};
use uiautomation::types::TreeScope;
use std::time::Duration;

use crate::types::{ExtractedContent, ExtractionError};

/// Window class names for Microsoft Office applications
pub mod window_classes {
    pub const WORD: &str = "OpusApp";
    pub const EXCEL: &str = "XLMAIN";
    pub const POWERPOINT: &str = "PPTFrameClass";
    pub const OUTLOOK: &str = "rctrl_renwnd32";
    pub const NOTEPAD: &str = "Notepad";
}

/// Main extractor for Windows applications
pub struct WindowsExtractor {
    automation: UIAutomation,
}

impl WindowsExtractor {
    /// Create a new Windows extractor
    /// 
    /// This initializes COM and creates the UIAutomation instance.
    /// 
    /// # Returns
    /// A new WindowsExtractor, or an error if initialization fails
    /// 
    /// # Example
    /// ```rust
    /// let extractor = WindowsExtractor::new()?;
    /// ```
    pub fn new() -> Result<Self, ExtractionError> {
        let automation = UIAutomation::new()
            .map_err(|e| ExtractionError::PlatformError(format!("Failed to initialize UI Automation: {:?}", e)))?;
        
        Ok(Self { automation })
    }
    
    /// Check if UI Automation is available
    /// 
    /// On Windows, UI Automation is always available (built into the OS).
    pub fn is_available() -> bool {
        true
    }
    
    /// Get the foreground (active) window element
    /// 
    /// # Returns
    /// The UIElement for the foreground window
    pub fn get_foreground_window(&self) -> Result<UIElement, ExtractionError> {
        // Get the foreground window handle using Win32 API
        let hwnd = unsafe { winapi::um::winuser::GetForegroundWindow() };
        
        if hwnd.is_null() {
            return Err(ExtractionError::ElementNotFound(
                "No foreground window".into()
            ));
        }
        
        self.automation.element_from_handle(hwnd as isize)
            .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))
    }
    
    /// Get the currently focused element
    pub fn get_focused_element(&self) -> Result<UIElement, ExtractionError> {
        self.automation.get_focused_element()
            .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))
    }
    
    /// Extract content from the foreground window
    /// 
    /// This is the main entry point for content extraction.
    /// 
    /// # Returns
    /// ExtractedContent with the document text
    pub fn extract_frontmost(&self) -> Result<ExtractedContent, ExtractionError> {
        let window = self.get_foreground_window()?;
        self.extract_from_element(&window)
    }
    
    /// Extract content from a specific element
    pub fn extract_from_element(&self, element: &UIElement) -> Result<ExtractedContent, ExtractionError> {
        // Get window info
        let name = element.get_name()
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let class_name = element.get_classname()
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let source = Self::class_to_source(&class_name);
        
        // Try TextPattern first (most reliable for documents)
        let content = self.extract_via_text_pattern(element)
            .or_else(|_| self.extract_via_tree_traversal(element))?;
        
        if content.trim().is_empty() {
            return Err(ExtractionError::NoContentFound(
                "Document appears to be empty".into()
            ));
        }
        
        Ok(ExtractedContent {
            source,
            title: Some(name.clone()),
            content,
            app_name: name,
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "accessibility".to_string(),
        })
    }
    
    /// Extract text using TextPattern (preferred method)
    /// 
    /// TextPattern provides direct access to document text and is
    /// the most efficient way to extract content from text-based controls.
    fn extract_via_text_pattern(&self, element: &UIElement) -> Result<String, ExtractionError> {
        // First, try to get TextPattern from the element itself
        if let Ok(text_pattern) = element.get_pattern::<TextPattern>() {
            return self.get_text_from_pattern(&text_pattern);
        }
        
        // If not available on the window, search for a Document element
        let document = self.find_document_element(element)?;
        
        let text_pattern = document.get_pattern::<TextPattern>()
            .map_err(|e| ExtractionError::PatternNotSupported(format!("{:?}", e)))?;
        
        self.get_text_from_pattern(&text_pattern)
    }
    
    /// Get text from a TextPattern
    fn get_text_from_pattern(&self, pattern: &TextPattern) -> Result<String, ExtractionError> {
        // Get the document range (entire document)
        let document_range = pattern.get_document_range()
            .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
        
        // Get all text (-1 means no limit)
        let text = document_range.get_text(-1)
            .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
        
        Ok(text)
    }
    
    /// Find the Document element within a window
    fn find_document_element(&self, window: &UIElement) -> Result<UIElement, ExtractionError> {
        // Create a condition to find Document control type
        let condition = self.automation.create_property_condition(
            uiautomation::types::UIProperty::ControlType,
            uiautomation::types::ControlType::Document.into(),
        ).map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
        
        // Search within the window
        window.find_first(TreeScope::Descendants, &condition)
            .map_err(|e| ExtractionError::ElementNotFound(
                format!("No Document element found: {:?}", e)
            ))
    }
    
    /// Extract text by traversing the element tree
    /// 
    /// This is a fallback method when TextPattern is not available.
    fn extract_via_tree_traversal(&self, element: &UIElement) -> Result<String, ExtractionError> {
        let walker = self.automation.get_control_view_walker()
            .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
        
        let mut result = String::new();
        self.traverse_and_extract(&walker, element, &mut result, 0)?;
        
        Ok(result)
    }
    
    /// Recursively traverse and extract text
    fn traverse_and_extract(
        &self,
        walker: &UITreeWalker,
        element: &UIElement,
        result: &mut String,
        depth: usize,
    ) -> Result<(), ExtractionError> {
        // Prevent infinite recursion
        if depth > 100 {
            return Ok(());
        }
        
        // Try to get text from this element
        
        // Method 1: Get Name property
        if let Ok(name) = element.get_name() {
            if !name.is_empty() && !result.contains(&name) {
                result.push_str(&name);
                result.push('\n');
            }
        }
        
        // Method 2: Try ValuePattern
        if let Ok(value_pattern) = element.get_pattern::<ValuePattern>() {
            if let Ok(value) = value_pattern.get_value() {
                if !value.is_empty() {
                    result.push_str(&value);
                    result.push('\n');
                }
            }
        }
        
        // Traverse children
        if let Ok(child) = walker.get_first_child(element) {
            self.traverse_and_extract(walker, &child, result, depth + 1)?;
            
            let mut current = child;
            while let Ok(sibling) = walker.get_next_sibling(&current) {
                self.traverse_and_extract(walker, &sibling, result, depth + 1)?;
                current = sibling;
            }
        }
        
        Ok(())
    }
    
    /// Find a window by class name
    /// 
    /// # Arguments
    /// * `class_name` - The window class name (e.g., "OpusApp" for Word)
    /// * `timeout` - How long to wait for the window
    pub fn find_window_by_class(
        &self, 
        class_name: &str,
        timeout: Duration,
    ) -> Result<UIElement, ExtractionError> {
        let root = self.automation.get_root_element()
            .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
        
        let matcher = self.automation.create_matcher()
            .from(root)
            .classname(class_name)
            .timeout(timeout.as_millis() as u64);
        
        matcher.find_first()
            .map_err(|e| ExtractionError::AppNotFound(format!("{}: {:?}", class_name, e)))
    }
    
    /// Find Microsoft Word window
    pub fn find_word(&self) -> Result<UIElement, ExtractionError> {
        self.find_window_by_class(window_classes::WORD, Duration::from_secs(2))
    }
    
    /// Find Microsoft Excel window
    pub fn find_excel(&self) -> Result<UIElement, ExtractionError> {
        self.find_window_by_class(window_classes::EXCEL, Duration::from_secs(2))
    }
    
    /// Find Microsoft PowerPoint window
    pub fn find_powerpoint(&self) -> Result<UIElement, ExtractionError> {
        self.find_window_by_class(window_classes::POWERPOINT, Duration::from_secs(2))
    }
    
    /// Get selected text from the focused element
    pub fn get_selected_text(&self) -> Result<Option<String>, ExtractionError> {
        let focused = self.get_focused_element()?;
        
        if let Ok(text_pattern) = focused.get_pattern::<TextPattern>() {
            let selections = text_pattern.get_selection()
                .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
            
            if !selections.is_empty() {
                let text = selections[0].get_text(-1)
                    .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
                return Ok(Some(text));
            }
        }
        
        Ok(None)
    }
    
    /// Convert window class name to source identifier
    fn class_to_source(class_name: &str) -> String {
        match class_name {
            "OpusApp" => "word",
            "XLMAIN" => "excel",
            "PPTFrameClass" => "powerpoint",
            "rctrl_renwnd32" => "outlook",
            "Notepad" => "notepad",
            _ => "unknown",
        }.to_string()
    }
}
```


#### src/platform/windows/patterns.rs

```rust
//! UI Automation Pattern helpers
//!
//! Provides utilities for working with UI Automation patterns.

use uiautomation::{UIElement, UIAutomation};
use uiautomation::patterns::TextPattern;
use uiautomation::types::{TreeScope, ControlType};

/// Check if an element supports TextPattern
pub fn supports_text_pattern(element: &UIElement) -> bool {
    element.get_pattern::<TextPattern>().is_ok()
}

/// Get all text from an element that supports TextPattern
pub fn get_all_text(element: &UIElement) -> Option<String> {
    let pattern = element.get_pattern::<TextPattern>().ok()?;
    let range = pattern.get_document_range().ok()?;
    range.get_text(-1).ok()
}

/// Get visible text only (what's currently on screen)
pub fn get_visible_text(element: &UIElement) -> Option<String> {
    let pattern = element.get_pattern::<TextPattern>().ok()?;
    let ranges = pattern.get_visible_ranges().ok()?;
    
    let mut result = String::new();
    for range in ranges {
        if let Ok(text) = range.get_text(-1) {
            result.push_str(&text);
            result.push('\n');
        }
    }
    
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Debug: Print element info
pub fn debug_element(element: &UIElement) {
    println!("Element Info:");
    println!("  Name: {:?}", element.get_name());
    println!("  ClassName: {:?}", element.get_classname());
    println!("  ControlType: {:?}", element.get_control_type());
    println!("  Supports TextPattern: {}", supports_text_pattern(element));
}
```

---

## Cross-Platform API

### Unified Types

#### src/types.rs

```rust
//! Shared types for the accessibility extractor

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Extracted content from an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedContent {
    /// Source application identifier (e.g., "word", "excel")
    pub source: String,
    
    /// Document title (usually from window title)
    pub title: Option<String>,
    
    /// The extracted text content
    pub content: String,
    
    /// Full application name
    pub app_name: String,
    
    /// Unix timestamp of extraction
    pub timestamp: i64,
    
    /// Method used for extraction (always "accessibility" for this module)
    pub extraction_method: String,
}

/// Errors that can occur during extraction
#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Application not found: {0}")]
    AppNotFound(String),
    
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    
    #[error("No content found: {0}")]
    NoContentFound(String),
    
    #[error("Pattern not supported: {0}")]
    PatternNotSupported(String),
    
    #[error("Platform error: {0}")]
    PlatformError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Application identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSource {
    Word,
    Excel,
    PowerPoint,
    Outlook,
    Pages,
    Numbers,
    Keynote,
    Notepad,
    TextEdit,
    Unknown,
}

impl AppSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppSource::Word => "word",
            AppSource::Excel => "excel",
            AppSource::PowerPoint => "powerpoint",
            AppSource::Outlook => "outlook",
            AppSource::Pages => "pages",
            AppSource::Numbers => "numbers",
            AppSource::Keynote => "keynote",
            AppSource::Notepad => "notepad",
            AppSource::TextEdit => "textedit",
            AppSource::Unknown => "unknown",
        }
    }
}
```

### Unified Extractor

#### src/extractor.rs

```rust
//! Cross-platform unified accessibility extractor
//!
//! This module provides a single API that works on both macOS and Windows.

#[cfg(target_os = "macos")]
use crate::platform::macos::MacOSExtractor;

#[cfg(target_os = "windows")]
use crate::platform::windows::WindowsExtractor;

use crate::types::{ExtractedContent, ExtractionError};

/// Cross-platform accessibility extractor
/// 
/// This struct provides a unified API for extracting content from
/// desktop applications on both macOS and Windows.
/// 
/// # Example
/// 
/// ```rust
/// use accessibility_extractor::AccessibilityExtractor;
/// 
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Check permissions (macOS only)
///     if !AccessibilityExtractor::is_enabled() {
///         AccessibilityExtractor::request_permissions();
///         println!("Please grant accessibility permissions and restart");
///         return Ok(());
///     }
///     
///     // Extract content from frontmost app
///     match AccessibilityExtractor::extract_frontmost() {
///         Ok(content) => {
///             println!("Source: {}", content.source);
///             println!("Title: {:?}", content.title);
///             println!("Content length: {} chars", content.content.len());
///         }
///         Err(e) => eprintln!("Extraction failed: {}", e),
///     }
///     
///     Ok(())
/// }
/// ```
pub struct AccessibilityExtractor;

impl AccessibilityExtractor {
    /// Check if accessibility features are enabled/available
    /// 
    /// - On macOS: Returns true if accessibility permission is granted
    /// - On Windows: Always returns true (no permission needed)
    /// - On other platforms: Returns false
    pub fn is_enabled() -> bool {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::is_accessibility_enabled()
        }
        
        #[cfg(target_os = "windows")]
        {
            WindowsExtractor::is_available()
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }
    
    /// Request accessibility permissions from the user
    /// 
    /// - On macOS: Shows the system permission dialog
    /// - On Windows: No-op (permissions not required)
    pub fn request_permissions() {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::request_accessibility();
        }
        
        #[cfg(target_os = "windows")]
        {
            // No permissions needed on Windows
        }
    }
    
    /// Extract content from the frontmost (active) application
    /// 
    /// This is the main entry point for content extraction. It will:
    /// 1. Get the frontmost application
    /// 2. Find the document/text content
    /// 3. Extract and return the text
    /// 
    /// # Returns
    /// - `Ok(ExtractedContent)` with the extracted text and metadata
    /// - `Err(ExtractionError)` if extraction fails
    pub fn extract_frontmost() -> Result<ExtractedContent, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::extract_frontmost()
        }
        
        #[cfg(target_os = "windows")]
        {
            let extractor = WindowsExtractor::new()?;
            extractor.extract_frontmost()
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(ExtractionError::PlatformError(
                "Unsupported platform".into()
            ))
        }
    }
    
    /// Extract content from a specific application
    /// 
    /// # Arguments
    /// * `app_identifier` - On macOS: bundle ID (e.g., "com.microsoft.Word")
    ///                      On Windows: window class (e.g., "OpusApp")
    pub fn extract_from_app(app_identifier: &str) -> Result<ExtractedContent, ExtractionError> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::extract_from_app(app_identifier)
        }
        
        #[cfg(target_os = "windows")]
        {
            let extractor = WindowsExtractor::new()?;
            let window = extractor.find_window_by_class(
                app_identifier, 
                std::time::Duration::from_secs(5)
            )?;
            extractor.extract_from_element(&window)
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(ExtractionError::PlatformError(
                "Unsupported platform".into()
            ))
        }
    }
    
    /// Get the currently selected text in any application
    /// 
    /// # Returns
    /// - `Some(String)` with the selected text
    /// - `None` if no text is selected or selection cannot be read
    pub fn get_selected_text() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            MacOSExtractor::get_selected_text()
        }
        
        #[cfg(target_os = "windows")]
        {
            let extractor = WindowsExtractor::new().ok()?;
            extractor.get_selected_text().ok().flatten()
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }
    
    /// Extract with retry logic
    /// 
    /// Sometimes extraction fails on the first attempt (e.g., if the
    /// application is still loading). This method retries extraction
    /// with a delay between attempts.
    /// 
    /// # Arguments
    /// * `max_attempts` - Maximum number of extraction attempts
    /// * `delay_ms` - Delay between attempts in milliseconds
    pub fn extract_with_retry(
        max_attempts: u32,
        delay_ms: u64,
    ) -> Result<ExtractedContent, ExtractionError> {
        let mut last_error = ExtractionError::NoContentFound("No attempts made".into());
        
        for attempt in 0..max_attempts {
            match Self::extract_frontmost() {
                Ok(content) if !content.content.trim().is_empty() => {
                    return Ok(content);
                }
                Ok(_) => {
                    // Content was empty, retry
                    last_error = ExtractionError::NoContentFound(
                        format!("Empty content on attempt {}", attempt + 1)
                    );
                }
                Err(e) => {
                    last_error = e;
                }
            }
            
            if attempt < max_attempts - 1 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
        
        Err(last_error)
    }
}
```


---

## Application-Specific Extraction

### Microsoft Word

Word has excellent accessibility support on both platforms.

#### macOS Word Extraction

```rust
/// Extract content specifically from Microsoft Word on macOS
pub fn extract_word_macos() -> Result<ExtractedContent, ExtractionError> {
    let app = MacOSExtractor::get_app_by_bundle_id(
        "com.microsoft.Word",
        Duration::from_secs(5)
    )?;
    
    // Word's document content is in the focused window
    let window = app.focused_window()
        .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))?;
    
    // The document title is in the window title (e.g., "Document1.docx - Word")
    let title = window.title()
        .map(|s| {
            let t = s.to_string();
            // Remove " - Word" suffix
            t.split(" - Word").next().unwrap_or(&t).to_string()
        })
        .ok();
    
    // Extract text from the document area
    let content = MacOSExtractor::extract_text_from_element(&window)?;
    
    Ok(ExtractedContent {
        source: "word".to_string(),
        title,
        content,
        app_name: "Microsoft Word".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "accessibility".to_string(),
    })
}
```

#### Windows Word Extraction

```rust
/// Extract content specifically from Microsoft Word on Windows
pub fn extract_word_windows(extractor: &WindowsExtractor) -> Result<ExtractedContent, ExtractionError> {
    // Find Word window by class name
    let word_window = extractor.find_word()?;
    
    // Word exposes a Document element with TextPattern
    let document = extractor.find_document_element(&word_window)?;
    
    // Use TextPattern for efficient extraction
    let text_pattern = document.get_pattern::<TextPattern>()
        .map_err(|e| ExtractionError::PatternNotSupported(format!("{:?}", e)))?;
    
    let document_range = text_pattern.get_document_range()
        .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
    
    let content = document_range.get_text(-1)
        .map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
    
    let title = word_window.get_name().ok();
    
    Ok(ExtractedContent {
        source: "word".to_string(),
        title,
        content,
        app_name: "Microsoft Word".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "accessibility".to_string(),
    })
}
```

### Microsoft Excel

Excel requires cell-by-cell extraction since content is organized in a grid.

#### Windows Excel Extraction

```rust
/// Extract content from Microsoft Excel on Windows
/// 
/// Excel organizes content in cells, so we need to traverse the grid.
pub fn extract_excel_windows(extractor: &WindowsExtractor) -> Result<ExtractedContent, ExtractionError> {
    let excel_window = extractor.find_excel()?;
    
    // Find the worksheet/grid element
    let condition = extractor.automation.create_property_condition(
        uiautomation::types::UIProperty::ControlType,
        uiautomation::types::ControlType::DataGrid.into(),
    ).map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
    
    let grid = excel_window.find_first(TreeScope::Descendants, &condition)
        .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))?;
    
    // Get all DataItem (cell) elements
    let cell_condition = extractor.automation.create_property_condition(
        uiautomation::types::UIProperty::ControlType,
        uiautomation::types::ControlType::DataItem.into(),
    ).map_err(|e| ExtractionError::PlatformError(format!("{:?}", e)))?;
    
    let cells = grid.find_all(TreeScope::Descendants, &cell_condition)
        .map_err(|e| ExtractionError::ElementNotFound(format!("{:?}", e)))?;
    
    let mut content = String::new();
    let mut current_row = 0;
    
    for cell in cells {
        // Get cell value
        if let Ok(value_pattern) = cell.get_pattern::<ValuePattern>() {
            if let Ok(value) = value_pattern.get_value() {
                if !value.is_empty() {
                    content.push_str(&value);
                    content.push('\t');  // Tab-separated
                }
            }
        }
        
        // Check for row change (simplified - real implementation needs row tracking)
        // This is a simplified example; actual implementation would track row/column
    }
    
    Ok(ExtractedContent {
        source: "excel".to_string(),
        title: excel_window.get_name().ok(),
        content,
        app_name: "Microsoft Excel".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "accessibility".to_string(),
    })
}
```

---

## Event-Driven Architecture

Instead of polling, you can listen for application focus changes.

### Windows Focus Change Handler

```rust
use uiautomation::events::{UIFocusChangedEventHandler, CustomFocusChangedEventHandler};
use std::sync::mpsc;

/// Handler for focus change events
struct FocusChangeHandler {
    sender: mpsc::Sender<ExtractedContent>,
    extractor: WindowsExtractor,
}

impl CustomFocusChangedEventHandler for FocusChangeHandler {
    fn handle(&self, element: &UIElement) -> uiautomation::Result<()> {
        // Check if this is an Office application
        if let Ok(class_name) = element.get_classname() {
            if Self::is_office_app(&class_name) {
                // Extract content
                if let Ok(content) = self.extractor.extract_from_element(element) {
                    let _ = self.sender.send(content);
                }
            }
        }
        Ok(())
    }
}

impl FocusChangeHandler {
    fn is_office_app(class_name: &str) -> bool {
        matches!(class_name, "OpusApp" | "XLMAIN" | "PPTFrameClass" | "rctrl_renwnd32")
    }
}

/// Start listening for focus changes
pub fn start_focus_listener() -> mpsc::Receiver<ExtractedContent> {
    let (sender, receiver) = mpsc::channel();
    
    std::thread::spawn(move || {
        let automation = UIAutomation::new().expect("Failed to create UIAutomation");
        let extractor = WindowsExtractor::new().expect("Failed to create extractor");
        
        let handler = FocusChangeHandler { sender, extractor };
        let event_handler = UIFocusChangedEventHandler::from(handler);
        
        automation.add_focus_changed_event_handler(None, &event_handler)
            .expect("Failed to add event handler");
        
        // Keep thread alive
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
    
    receiver
}
```

### macOS App Activation Observer

```rust
use cocoa::appkit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
use cocoa::base::nil;
use objc::runtime::Object;

/// Start observing application activations on macOS
/// 
/// Note: This requires running on the main thread with a run loop.
pub fn start_app_observer<F>(callback: F) 
where
    F: Fn(&str) + Send + 'static,
{
    unsafe {
        let workspace = NSWorkspace::sharedWorkspace(nil);
        let notification_center = workspace.notificationCenter();
        
        // Create observer block
        // Note: This is simplified; actual implementation needs proper block handling
        
        // In practice, you'd use the `block` crate or similar to create
        // an Objective-C block that calls your Rust callback
    }
}
```

---

## Testing & Debugging

### Manual Testing Steps

1. **macOS Permission Test**:
   ```bash
   # Build and run
   cargo build
   ./target/debug/ax-extractor --check-permissions
   ```

2. **Extraction Test**:
   ```bash
   # Open Microsoft Word with a document
   # Then run:
   ./target/debug/ax-extractor --extract
   ```

### Debug Utilities

```rust
/// Print the accessibility tree for debugging
pub fn debug_print_accessibility_tree() {
    #[cfg(target_os = "macos")]
    {
        if let Some(app) = MacOSExtractor::get_frontmost_app() {
            if let Ok(window) = app.focused_window() {
                crate::platform::macos::element::debug_print_tree(&window, 0);
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Ok(extractor) = WindowsExtractor::new() {
            if let Ok(window) = extractor.get_foreground_window() {
                crate::platform::windows::patterns::debug_element(&window);
            }
        }
    }
}
```

### Common Issues and Solutions

| Issue | Platform | Solution |
|-------|----------|----------|
| "Permission denied" | macOS | Grant accessibility permission in System Preferences |
| Empty content returned | Both | Check if app supports accessibility; try tree traversal fallback |
| "Element not found" | Both | App may not have focus; ensure window is active |
| Slow extraction | Both | Large documents take time; consider extracting visible text only |
| Garbled text | Both | App may use custom rendering; try different extraction method |

---

## Deployment Considerations

### macOS Code Signing

For accessibility to work in production, your app must be:
1. Code signed with a valid Developer ID
2. Notarized by Apple (for distribution outside App Store)

```bash
# Sign the binary
codesign --sign "Developer ID Application: Your Name" \
         --options runtime \
         --entitlements entitlements.plist \
         target/release/ax-extractor

# Notarize
xcrun notarytool submit target/release/ax-extractor.zip \
      --apple-id "your@email.com" \
      --team-id "TEAMID" \
      --password "@keychain:AC_PASSWORD"
```

### Entitlements (macOS)

Create `entitlements.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.automation.apple-events</key>
    <true/>
</dict>
</plist>
```

### Windows Deployment

No special signing required for accessibility. However, for enterprise deployment:
1. Consider code signing with an EV certificate
2. Test on Windows 10 and Windows 11
3. Ensure Visual C++ Redistributable is installed (if using MSVC toolchain)

---

## Complete Cargo.toml Reference

```toml
[package]
name = "accessibility-extractor"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your@email.com>"]
description = "Extract content from desktop applications using Accessibility APIs"
license = "MIT"
repository = "https://github.com/yourorg/accessibility-extractor"

[lib]
name = "accessibility_extractor"
path = "src/lib.rs"

[[bin]]
name = "ax-extractor"
path = "src/main.rs"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"

[target.'cfg(windows)'.dependencies]
uiautomation = { version = "0.19", features = ["pattern", "control", "event"] }
winapi = { version = "0.3", features = ["winuser", "processthreadsapi"] }

[target.'cfg(target_os = "macos")'.dependencies]
accessibility = "0.2"
accessibility-sys = "0.2"
core-foundation = "0.10"
macos-accessibility-client = "0.0.2"
cocoa = "0.26"
objc = "0.2"

[dev-dependencies]
pretty_assertions = "1.4"

[profile.release]
opt-level = 3
lto = true
```

---

## Summary

This implementation provides:

1. **Pure Rust** implementation for both macOS and Windows
2. **No external dependencies** like Swift or C++ bridges
3. **Unified API** that works identically on both platforms
4. **Comprehensive error handling** with typed errors
5. **Application-specific extractors** for Office apps
6. **Event-driven architecture** support for real-time extraction
7. **Debug utilities** for troubleshooting

The key crates used are:
- `accessibility` (0.2) for macOS
- `uiautomation` (0.19) for Windows

Both are pure Rust and well-maintained.
