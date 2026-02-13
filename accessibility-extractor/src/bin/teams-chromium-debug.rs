//! Debug tool to explore the Chromium-based accessibility tree in New Teams.
//! This tool traverses deep into the web content group to find actual text elements.
//!
//! Key insight: New Teams uses Chromium under the hood (ChromeAXNodeId attribute present).
//! The text content is likely in child elements, not the top-level web content group.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_foundation::number::CFNumber;

fn main() {
    env_logger::init();
    
    println!("New Teams Chromium Accessibility Deep Dive\n");
    println!("============================================================");
    
    // Detect Teams
    let bundle_id = if accessibility_extractor::platform::macos::get_pid_for_bundle_id("com.microsoft.teams2").is_some() {
        "com.microsoft.teams2"
    } else if accessibility_extractor::platform::macos::get_pid_for_bundle_id("com.microsoft.teams").is_some() {
        "com.microsoft.teams"
    } else {
        println!("Microsoft Teams is not running.");
        return;
    };
    
    println!("Found Teams: {}\n", bundle_id);
    
    let app = match accessibility_extractor::platform::macos::MacOSExtractor::get_app_by_bundle_id(
        bundle_id,
        Duration::from_secs(5)
    ) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to get app reference: {}", e);
            return;
        }
    };
    
    // Get window
    let window = match app.focused_window() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("No focused window: {:?}", e);
            return;
        }
    };
    
    if let Ok(title) = window.title() {
        println!("Window: {}\n", title);
    }
    
    println!("============================================================");
    println!("STEP 1: Finding web content group and exploring children...\n");
    
    // Find the web content group and explore its children deeply
    if let Some(web_group) = find_web_content_group(&window, 0) {
        println!("Found web content group!");
        
        // Print info about the web content group
        print_element_info(&web_group, "Web Content Group", 0);
        
        println!("\n============================================================");
        println!("STEP 2: Exploring children of web content group...\n");
        
        // Explore children deeply
        explore_children_deeply(&web_group, 0, 50);
        
        println!("\n============================================================");
        println!("STEP 3: Looking for elements with actual text content...\n");
        
        // Find all elements with text
        let mut text_elements = Vec::new();
        find_elements_with_text(&web_group, &mut text_elements, 0);
        
        println!("Found {} elements with text content:\n", text_elements.len());
        for (i, (role, text)) in text_elements.iter().enumerate().take(30) {
            let display_text = if text.len() > 100 {
                format!("{}...", &text[..100])
            } else {
                text.clone()
            };
            println!("  {}. [{}] \"{}\"", i + 1, role, display_text);
        }
        
        if text_elements.len() > 30 {
            println!("  ... and {} more elements", text_elements.len() - 30);
        }
    } else {
        println!("Could not find web content group");
        
        // Try to find any AXGroup with interesting content
        println!("\nLooking for any groups with content...");
        find_any_content_groups(&window, 0);
    }
}

fn find_web_content_group(element: &AXUIElement, depth: usize) -> Option<AXUIElement> {
    if depth > 20 { return None; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check if this is the web content group
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if title_str.contains("Web content") {
                return Some(element.clone());
            }
        }
    }
    
    // Recurse into children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                if let Some(found) = find_web_content_group(&child, depth + 1) {
                    return Some(found);
                }
            }
        }
    }
    
    None
}

fn print_element_info(element: &AXUIElement, name: &str, indent: usize) {
    let prefix = "  ".repeat(indent);
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_else(|_| "Unknown".to_string());
    let title = element.title().map(|s| s.to_string()).ok();
    
    println!("{}[{}] Role: {}", prefix, name, role);
    if let Some(t) = title {
        if !t.is_empty() {
            let display = if t.len() > 80 { format!("{}...", &t[..80]) } else { t };
            println!("{}  Title: \"{}\"", prefix, display);
        }
    }
    
    // Check for AXValue
    if let Ok(value) = element.value() {
        if let Some(text) = cftype_to_string(&value) {
            if !text.is_empty() {
                let display = if text.len() > 80 { format!("{}...", &text[..80]) } else { text };
                println!("{}  Value: \"{}\"", prefix, display);
            }
        }
    }
    
    // Check for AXDescription
    let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
    if let Ok(desc) = element.attribute(&desc_attr) {
        if let Some(text) = cftype_to_string(&desc) {
            if !text.is_empty() {
                let display = if text.len() > 80 { format!("{}...", &text[..80]) } else { text };
                println!("{}  Description: \"{}\"", prefix, display);
            }
        }
    }
    
    // Check for AXNumberOfCharacters
    let num_chars_attr = AXAttribute::<CFType>::new(&CFString::new("AXNumberOfCharacters"));
    if let Ok(num_chars_value) = element.attribute(&num_chars_attr) {
        if let Some(num_chars) = cftype_to_i64(&num_chars_value) {
            println!("{}  NumberOfCharacters: {}", prefix, num_chars);
        }
    }
    
    // Check for ChromeAXNodeId (indicates Chromium-based element)
    let chrome_attr = AXAttribute::<CFType>::new(&CFString::new("ChromeAXNodeId"));
    if element.attribute(&chrome_attr).is_ok() {
        println!("{}  [Has ChromeAXNodeId - Chromium element]", prefix);
    }
    
    // Get children count
    if let Ok(children) = element.children() {
        println!("{}  Children: {}", prefix, children.len());
    }
}

fn explore_children_deeply(element: &AXUIElement, depth: usize, max_depth: usize) {
    if depth > max_depth { return; }
    
    let prefix = "  ".repeat(depth);
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                let role = child.role().map(|s| s.to_string()).unwrap_or_else(|_| "Unknown".to_string());
                let title = child.title().map(|s| s.to_string()).ok();
                
                // Get value if it's a text element
                let value = if role == "AXStaticText" || role == "AXTextField" || role == "AXTextArea" {
                    child.value().ok().and_then(|v| cftype_to_string(&v))
                } else {
                    None
                };
                
                // Print element info
                print!("{}[{}] {}", prefix, i, role);
                
                if let Some(t) = &title {
                    if !t.is_empty() && t.len() < 50 {
                        print!(" title=\"{}\"", t);
                    }
                }
                
                if let Some(v) = &value {
                    if !v.is_empty() {
                        let display = if v.len() > 50 { format!("{}...", &v[..50]) } else { v.clone() };
                        print!(" value=\"{}\"", display);
                    }
                }
                
                // Check children count
                if let Ok(grandchildren) = child.children() {
                    if grandchildren.len() > 0 {
                        print!(" ({} children)", grandchildren.len());
                    }
                }
                
                println!();
                
                // Recurse into interesting elements (limit depth for non-text elements)
                let should_recurse = match role.as_str() {
                    "AXGroup" | "AXList" | "AXOutline" | "AXRow" | "AXCell" | 
                    "AXScrollArea" | "AXWebArea" | "AXSection" | "AXArticle" => true,
                    _ => depth < 10, // Limit depth for other elements
                };
                
                if should_recurse {
                    explore_children_deeply(&child, depth + 1, max_depth);
                }
            }
        }
    }
}

fn find_elements_with_text(element: &AXUIElement, results: &mut Vec<(String, String)>, depth: usize) {
    if depth > 100 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check for text content in AXValue
    if let Ok(value) = element.value() {
        if let Some(text) = cftype_to_string(&value) {
            let trimmed = text.trim();
            if !trimmed.is_empty() && trimmed.len() > 3 {
                results.push((role.clone(), trimmed.to_string()));
            }
        }
    }
    
    // Also check AXTitle for some elements
    if role == "AXButton" || role == "AXLink" || role == "AXStaticText" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            let trimmed = title_str.trim();
            if !trimmed.is_empty() && trimmed.len() > 3 {
                // Avoid duplicates
                let key = (role.clone(), trimmed.to_string());
                if !results.contains(&key) {
                    results.push(key);
                }
            }
        }
    }
    
    // Recurse into children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_elements_with_text(&child, results, depth + 1);
            }
        }
    }
}

fn find_any_content_groups(element: &AXUIElement, depth: usize) {
    if depth > 15 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXGroup" || role == "AXWebArea" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if !title_str.is_empty() {
                println!("Found {}: \"{}\"", role, title_str);
                
                // Check children count
                if let Ok(children) = element.children() {
                    println!("  Children: {}", children.len());
                }
            }
        }
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_any_content_groups(&child, depth + 1);
            }
        }
    }
}

fn cftype_to_string(value: &CFType) -> Option<String> {
    let type_id = value.type_of();
    if type_id == CFString::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_string: CFString = unsafe {
            CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
        };
        return Some(cf_string.to_string());
    }
    None
}

fn cftype_to_i64(value: &CFType) -> Option<i64> {
    let type_id = value.type_of();
    if type_id == CFNumber::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_number: CFNumber = unsafe {
            CFNumber::wrap_under_get_rule(ptr as core_foundation::number::CFNumberRef)
        };
        return cf_number.to_i64();
    }
    None
}
