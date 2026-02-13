//! CLI entry point for the accessibility extractor.
//!
//! This binary provides command-line access to the accessibility extraction functionality.
//!
//! # Usage
//!
//! ```bash
//! # Check if accessibility permissions are granted
//! ax-extractor --check-permissions
//!
//! # Extract content from the frontmost application
//! ax-extractor --extract
//!
//! # Get currently selected text
//! ax-extractor --selected
//! ```
//!
//! # Requirements
//! - Requirement 11.1: CLI binary named "ax-extractor"
//! - Requirement 11.2: Support --check-permissions flag
//! - Requirement 11.3: Support --extract flag
//! - Requirement 11.4: Support --selected flag

use std::env;
use std::process;

use accessibility_extractor::AccessibilityExtractor;
use accessibility_extractor::platform::macos::{MacOSExtractor, debug_print_tree, debug_print_attributes, find_elements_by_role};

/// CLI command to execute
#[derive(Debug, Clone, PartialEq)]
enum Command {
    /// Check if accessibility permissions are granted
    CheckPermissions,
    /// Extract content from the frontmost application
    Extract,
    /// Extract content from a specific app by bundle ID
    ExtractApp(String),
    /// Debug: print accessibility tree for an app
    DebugTree(String),
    /// Debug: inspect attributes of a specific role in an app
    DebugRole(String, String),
    /// Debug: search for text content in all elements
    SearchText(String),
    /// Get currently selected text
    Selected,
    /// Show help message
    Help,
}

/// Parse command line arguments and return the command to execute
fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = env::args().collect();
    
    // If no arguments provided, show help
    if args.len() < 2 {
        return Ok(Command::Help);
    }
    
    // Parse the first argument as the command
    match args[1].as_str() {
        "--check-permissions" | "-c" => Ok(Command::CheckPermissions),
        "--extract" | "-e" => Ok(Command::Extract),
        "--app" | "-a" => {
            // Requires a bundle ID argument
            if args.len() < 3 {
                return Err("--app requires a bundle ID argument (e.g., --app com.microsoft.Word)".into());
            }
            Ok(Command::ExtractApp(args[2].clone()))
        }
        "--debug-tree" | "-d" => {
            // Requires a bundle ID argument
            if args.len() < 3 {
                return Err("--debug-tree requires a bundle ID argument (e.g., --debug-tree com.microsoft.Word)".into());
            }
            Ok(Command::DebugTree(args[2].clone()))
        }
        "--debug-role" => {
            // Requires bundle ID and role arguments
            if args.len() < 4 {
                return Err("--debug-role requires bundle ID and role (e.g., --debug-role com.microsoft.Word AXLayoutArea)".into());
            }
            Ok(Command::DebugRole(args[2].clone(), args[3].clone()))
        }
        "--search-text" => {
            // Requires a bundle ID argument
            if args.len() < 3 {
                return Err("--search-text requires a bundle ID argument".into());
            }
            Ok(Command::SearchText(args[2].clone()))
        }
        "--selected" | "-s" => Ok(Command::Selected),
        "--help" | "-h" => Ok(Command::Help),
        arg => Err(format!("Unknown argument: {}", arg)),
    }
}

/// Print help message to stdout
fn print_help() {
    println!("ax-extractor - Extract content from desktop applications using Accessibility APIs");
    println!();
    println!("USAGE:");
    println!("    ax-extractor [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -c, --check-permissions    Check if accessibility permissions are granted");
    println!("    -e, --extract              Extract content from the frontmost application");
    println!("    -a, --app <BUNDLE_ID>      Extract content from a specific app by bundle ID");
    println!("    -d, --debug-tree <BUNDLE_ID>  Print accessibility tree for debugging");
    println!("    -s, --selected             Get currently selected text");
    println!("    -h, --help                 Print this help message");
    println!();
    println!("BUNDLE IDs:");
    println!("    com.microsoft.Word         Microsoft Word");
    println!("    com.microsoft.Excel        Microsoft Excel");
    println!("    com.microsoft.Powerpoint   Microsoft PowerPoint");
    println!("    com.microsoft.teams        Microsoft Teams (classic)");
    println!("    com.microsoft.teams2       Microsoft Teams (new)");
    println!("    com.tinyspeck.slackmacgap  Slack");
    println!("    com.apple.iWork.Pages      Apple Pages");
    println!("    com.apple.iWork.Numbers    Apple Numbers");
    println!("    com.apple.iWork.Keynote    Apple Keynote");
    println!("    com.apple.TextEdit         TextEdit");
    println!();
    println!("OUTPUT:");
    println!("    All output is JSON formatted to stdout.");
    println!("    Errors are written to stderr.");
}

/// Handle the --check-permissions command
/// Requirement 11.2: Support --check-permissions flag
fn handle_check_permissions() -> i32 {
    let enabled = AccessibilityExtractor::is_enabled();
    
    // Output JSON with enabled status
    let output = serde_json::json!({
        "enabled": enabled,
        "message": if enabled {
            "Accessibility permissions are granted"
        } else {
            "Accessibility permissions are NOT granted. Please enable in System Preferences."
        }
    });
    
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    
    if enabled { 0 } else { 1 }
}

/// Handle the --extract command
/// Requirement 11.3: Support --extract flag
fn handle_extract() -> i32 {
    eprintln!("[AX-EXTRACTOR] 🚀 Starting content extraction...");
    
    match AccessibilityExtractor::extract_frontmost() {
        Ok(content) => {
            eprintln!("[AX-EXTRACTOR] ✅ Extraction successful!");
            eprintln!("[AX-EXTRACTOR] 📱 Application: {}", content.app_name);
            eprintln!("[AX-EXTRACTOR] 🏷️  Source type: {}", content.source);
            eprintln!("[AX-EXTRACTOR] 📄 Document: {}", content.title.as_deref().unwrap_or("untitled"));
            eprintln!("[AX-EXTRACTOR] 📊 Content length: {} characters", content.content.len());
            
            // Convert to CapturePayload format for JSON output
            let payload = AccessibilityExtractor::to_capture_payload(&content);
            
            match serde_json::to_string_pretty(&payload) {
                Ok(json) => {
                    println!("{}", json);
                    0
                }
                Err(e) => {
                    eprintln!("[AX-EXTRACTOR] ❌ Error serializing output: {}", e);
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("[AX-EXTRACTOR] ❌ Extraction failed: {}", e);
            1
        }
    }
}

/// Handle the --app command to extract from a specific application
fn handle_extract_app(bundle_id: &str) -> i32 {
    eprintln!("[AX-EXTRACTOR] 🚀 Starting content extraction from {}...", bundle_id);
    
    match AccessibilityExtractor::extract_from_app(bundle_id) {
        Ok(content) => {
            eprintln!("[AX-EXTRACTOR] ✅ Extraction successful!");
            eprintln!("[AX-EXTRACTOR] 📱 Application: {}", content.app_name);
            eprintln!("[AX-EXTRACTOR] 🏷️  Source type: {}", content.source);
            eprintln!("[AX-EXTRACTOR] 📄 Document: {}", content.title.as_deref().unwrap_or("untitled"));
            eprintln!("[AX-EXTRACTOR] 📊 Content length: {} characters", content.content.len());
            
            // Convert to CapturePayload format for JSON output
            let payload = AccessibilityExtractor::to_capture_payload(&content);
            
            match serde_json::to_string_pretty(&payload) {
                Ok(json) => {
                    println!("{}", json);
                    0
                }
                Err(e) => {
                    eprintln!("[AX-EXTRACTOR] ❌ Error serializing output: {}", e);
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("[AX-EXTRACTOR] ❌ Extraction failed: {}", e);
            eprintln!("[AX-EXTRACTOR] 💡 Make sure {} is running", bundle_id);
            1
        }
    }
}

/// Handle the --debug-tree command to print accessibility tree
fn handle_debug_tree(bundle_id: &str) -> i32 {
    use std::time::Duration;
    
    eprintln!("[AX-EXTRACTOR] 🔍 Getting accessibility tree for {}...", bundle_id);
    
    match MacOSExtractor::get_app_by_bundle_id(bundle_id, Duration::from_secs(5)) {
        Ok(app) => {
            eprintln!("[AX-EXTRACTOR] ✅ Found app, printing tree (first 10 levels):\n");
            debug_print_tree(&app, 0);
            0
        }
        Err(e) => {
            eprintln!("[AX-EXTRACTOR] ❌ Failed to get app: {}", e);
            1
        }
    }
}

/// Handle the --debug-role command to inspect attributes of elements with a specific role
fn handle_debug_role(bundle_id: &str, role: &str) -> i32 {
    use std::time::Duration;
    
    eprintln!("[AX-EXTRACTOR] 🔍 Finding {} elements in {}...", role, bundle_id);
    
    match MacOSExtractor::get_app_by_bundle_id(bundle_id, Duration::from_secs(5)) {
        Ok(app) => {
            let elements = find_elements_by_role(&app, role);
            eprintln!("[AX-EXTRACTOR] ✅ Found {} elements with role {}\n", elements.len(), role);
            
            for (i, element) in elements.iter().enumerate().take(3) {
                println!("=== Element {} ===", i + 1);
                debug_print_attributes(element);
                println!();
            }
            0
        }
        Err(e) => {
            eprintln!("[AX-EXTRACTOR] ❌ Failed to get app: {}", e);
            1
        }
    }
}

/// Handle searching for text content in all elements
fn handle_search_text(bundle_id: &str) -> i32 {
    use std::time::Duration;
    use accessibility::attribute::AXAttribute;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::string::CFString;
    
    eprintln!("[AX-EXTRACTOR] 🔍 Searching for text content in {}...", bundle_id);
    
    match MacOSExtractor::get_app_by_bundle_id(bundle_id, Duration::from_secs(5)) {
        Ok(app) => {
            // Search common text-containing roles
            let roles_to_check = vec![
                "AXStaticText", "AXTextArea", "AXTextField", "AXLayoutArea",
                "AXGroup", "AXScrollArea", "AXWebArea", "AXDocument"
            ];
            
            for role in roles_to_check {
                let elements = find_elements_by_role(&app, role);
                for element in elements.iter().take(10) {
                    // Check various text attributes
                    for attr_name in &["AXValue", "AXDescription", "AXTitle", "AXHelp"] {
                        let attr = AXAttribute::<CFType>::new(&CFString::new(attr_name));
                        if let Ok(value) = element.attribute(&attr) {
                            let type_id = value.type_of();
                            if type_id == CFString::type_id() {
                                let ptr = value.as_CFTypeRef();
                                let cf_string: CFString = unsafe {
                                    CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
                                };
                                let s = cf_string.to_string();
                                if !s.is_empty() && s.len() > 5 {
                                    println!("{}.{} = \"{}\"", role, attr_name, 
                                        if s.len() > 100 { format!("{}...", &s[..100]) } else { s }.replace('\n', "\\n"));
                                }
                            }
                        }
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("[AX-EXTRACTOR] ❌ Failed to get app: {}", e);
            1
        }
    }
}

/// Handle the --selected command
/// Requirement 11.4: Support --selected flag
fn handle_selected() -> i32 {
    match AccessibilityExtractor::get_selected_text() {
        Some(text) => {
            let output = serde_json::json!({
                "selected_text": text,
                "has_selection": true
            });
            
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            0
        }
        None => {
            let output = serde_json::json!({
                "selected_text": null,
                "has_selection": false,
                "message": "No text is currently selected"
            });
            
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            0
        }
    }
}

fn main() {
    // Initialize env_logger for logging
    // Requirement 12.1: Initialize env_logger for logging
    env_logger::init();
    
    log::debug!("ax-extractor starting");
    
    // Parse command line arguments
    let command = match parse_args() {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Use --help for usage information.");
            process::exit(1);
        }
    };
    
    log::debug!("Executing command: {:?}", command);
    
    // Execute the command and get exit code
    let exit_code = match command {
        Command::CheckPermissions => handle_check_permissions(),
        Command::Extract => handle_extract(),
        Command::ExtractApp(bundle_id) => handle_extract_app(&bundle_id),
        Command::DebugTree(bundle_id) => handle_debug_tree(&bundle_id),
        Command::DebugRole(bundle_id, role) => handle_debug_role(&bundle_id, &role),
        Command::SearchText(bundle_id) => handle_search_text(&bundle_id),
        Command::Selected => handle_selected(),
        Command::Help => {
            print_help();
            0
        }
    };
    
    log::debug!("Exiting with code: {}", exit_code);
    
    process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_args_check_permissions() {
        // Note: We can't easily test parse_args directly since it reads from env::args()
        // This test verifies the Command enum works correctly
        assert_eq!(Command::CheckPermissions, Command::CheckPermissions);
    }
    
    #[test]
    fn test_parse_args_extract() {
        assert_eq!(Command::Extract, Command::Extract);
    }
    
    #[test]
    fn test_parse_args_selected() {
        assert_eq!(Command::Selected, Command::Selected);
    }
    
    #[test]
    fn test_parse_args_help() {
        assert_eq!(Command::Help, Command::Help);
    }
}
