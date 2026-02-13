//! Test tool to extract content from New Teams using AXStringForRange.
//! This uses the parameterized attribute approach for Chromium-based web views.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_foundation::number::CFNumber;

fn main() {
    env_logger::init();
    
    println!("Microsoft Teams Content Extraction Test\n");
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
    println!("Looking for web content group...\n");
    
    // Find the web content group
    if let Some(web_group) = find_web_content_group(&window, 0) {
        println!("Found web content group!");
        
        // Try to get AXNumberOfCharacters
        let num_chars_attr = AXAttribute::<CFType>::new(&CFString::new("AXNumberOfCharacters"));
        if let Ok(num_chars_value) = web_group.attribute(&num_chars_attr) {
            if let Some(num_chars) = cftype_to_i64(&num_chars_value) {
                println!("Number of characters: {}", num_chars);
                
                if num_chars > 0 {
                    // Try to extract text using AXStringForRange
                    println!("\nAttempting to extract text using AXStringForRange...\n");
                    
                    if let Some(text) = get_string_for_range(&web_group, 0, num_chars as usize) {
                        println!("============================================================");
                        println!("EXTRACTED CONTENT ({} chars):", text.len());
                        println!("============================================================\n");
                        
                        // Print first 2000 chars
                        let display_text = if text.len() > 2000 {
                            format!("{}...\n\n[truncated, {} more chars]", &text[..2000], text.len() - 2000)
                        } else {
                            text.clone()
                        };
                        println!("{}", display_text);
                    } else {
                        println!("Failed to extract text using AXStringForRange");
                        
                        // Try AXValue as fallback
                        println!("\nTrying AXValue as fallback...");
                        if let Ok(value) = web_group.value() {
                            if let Some(text) = cftype_to_string(&value) {
                                println!("AXValue: {}", text);
                            }
                        }
                    }
                }
            }
        } else {
            println!("Could not get AXNumberOfCharacters");
        }
        
        // Also try to get AXValue directly
        println!("\n============================================================");
        println!("Checking AXValue attribute...\n");
        
        if let Ok(value) = web_group.value() {
            if let Some(text) = cftype_to_string(&value) {
                if !text.is_empty() {
                    println!("AXValue content ({} chars):", text.len());
                    let display = if text.len() > 500 { &text[..500] } else { &text };
                    println!("{}", display);
                } else {
                    println!("AXValue is empty");
                }
            } else {
                println!("AXValue is not a string type");
            }
        } else {
            println!("Could not get AXValue");
        }
    } else {
        println!("Could not find web content group");
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

fn get_string_for_range(element: &AXUIElement, start: usize, length: usize) -> Option<String> {
    unsafe {
        use accessibility_sys::AXUIElementCopyParameterizedAttributeValue;
        use core_foundation::base::CFRelease;
        
        // Create a CFRange for the text range
        let range = core_foundation::base::CFRange {
            location: start as isize,
            length: length as isize,
        };
        
        // Create an AXValue containing the range
        // kAXValueCFRangeType = 4
        let range_value = accessibility_sys::AXValueCreate(
            4, // kAXValueCFRangeType
            &range as *const _ as *const std::ffi::c_void
        );
        
        if range_value.is_null() {
            println!("Failed to create AXValue for range");
            return None;
        }
        
        // Call AXStringForRange
        let attr_name = CFString::new("AXStringForRange");
        let mut result: core_foundation::base::CFTypeRef = std::ptr::null();
        
        let error = AXUIElementCopyParameterizedAttributeValue(
            element.as_concrete_TypeRef(),
            attr_name.as_concrete_TypeRef(),
            range_value as core_foundation::base::CFTypeRef,
            &mut result
        );
        
        CFRelease(range_value as *const _);
        
        if error != 0 {
            println!("AXStringForRange failed with error: {}", error);
            return None;
        }
        
        if result.is_null() {
            println!("AXStringForRange returned null");
            return None;
        }
        
        // Convert result to string
        let cf_type = CFType::wrap_under_create_rule(result);
        let text = cftype_to_string(&cf_type);
        
        text
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
