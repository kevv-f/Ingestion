//! Tool to try enabling accessibility for New Teams' Chromium web view.
//! 
//! This tool attempts various methods to enable/populate accessibility content:
//! 1. AXManualAccessibility (works for Electron)
//! 2. AXEnhancedUserInterface (used by some apps)
//! 3. Performing AXPress action to trigger accessibility
//! 4. Setting focus to trigger content population

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use accessibility_sys::{AXUIElementSetAttributeValue, kAXErrorSuccess};
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;

fn main() {
    env_logger::init();
    
    println!("New Teams Accessibility Enabler\n");
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
    
    let pid = accessibility_extractor::platform::macos::get_pid_for_bundle_id(bundle_id).unwrap();
    println!("PID: {}\n", pid);
    
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
    
    println!("============================================================");
    println!("STEP 1: Trying AXManualAccessibility...\n");
    
    try_set_attribute(&app, "AXManualAccessibility", true);
    
    println!("\n============================================================");
    println!("STEP 2: Trying AXEnhancedUserInterface...\n");
    
    try_set_attribute(&app, "AXEnhancedUserInterface", true);
    
    println!("\n============================================================");
    println!("STEP 3: Waiting for accessibility tree to populate...\n");
    
    std::thread::sleep(Duration::from_secs(2));
    
    // Get window and check content
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
    println!("STEP 4: Checking if content is now available...\n");
    
    // Find web content group and check AXNumberOfCharacters
    if let Some(web_group) = find_web_content_group(&window, 0) {
        let num_chars_attr = AXAttribute::<CFType>::new(&CFString::new("AXNumberOfCharacters"));
        if let Ok(num_chars_value) = web_group.attribute(&num_chars_attr) {
            if let Some(num_chars) = cftype_to_i64(&num_chars_value) {
                println!("AXNumberOfCharacters: {}", num_chars);
                
                if num_chars > 0 {
                    println!("\n✅ SUCCESS! Content is now available!");
                    println!("Attempting to extract text...\n");
                    
                    // Try to extract text
                    if let Some(text) = get_string_for_range(&web_group, 0, num_chars as usize) {
                        println!("Extracted {} characters:", text.len());
                        let display = if text.len() > 500 { &text[..500] } else { &text };
                        println!("{}", display);
                    }
                } else {
                    println!("\n❌ Content still not available (0 characters)");
                }
            }
        }
        
        // Also check AXValue
        if let Ok(value) = web_group.value() {
            if let Some(text) = cftype_to_string(&value) {
                if !text.is_empty() {
                    println!("\nAXValue: {}", text);
                }
            }
        }
    }
    
    println!("\n============================================================");
    println!("STEP 5: Trying to focus the web content area...\n");
    
    if let Some(web_group) = find_web_content_group(&window, 0) {
        // Try to set focus
        unsafe {
            let attr_name = CFString::new("AXFocused");
            let result = AXUIElementSetAttributeValue(
                web_group.as_concrete_TypeRef(),
                attr_name.as_concrete_TypeRef(),
                CFBoolean::true_value().as_CFTypeRef()
            );
            
            if result == kAXErrorSuccess {
                println!("✓ Set AXFocused to true");
            } else {
                println!("✗ Failed to set AXFocused (error {})", result);
            }
        }
        
        // Wait and check again
        std::thread::sleep(Duration::from_millis(500));
        
        let num_chars_attr = AXAttribute::<CFType>::new(&CFString::new("AXNumberOfCharacters"));
        if let Ok(num_chars_value) = web_group.attribute(&num_chars_attr) {
            if let Some(num_chars) = cftype_to_i64(&num_chars_value) {
                println!("AXNumberOfCharacters after focus: {}", num_chars);
            }
        }
    }
    
    println!("\n============================================================");
    println!("STEP 6: Exploring deeper into the tree for text elements...\n");
    
    if let Some(web_group) = find_web_content_group(&window, 0) {
        find_text_elements_deep(&web_group, 0);
    }
}

fn try_set_attribute(app: &AXUIElement, attr_name: &str, value: bool) {
    unsafe {
        let attr = CFString::new(attr_name);
        let cf_value = if value {
            CFBoolean::true_value()
        } else {
            CFBoolean::false_value()
        };
        
        let result = AXUIElementSetAttributeValue(
            app.as_concrete_TypeRef(),
            attr.as_concrete_TypeRef(),
            cf_value.as_CFTypeRef()
        );
        
        if result == kAXErrorSuccess {
            println!("✓ Set {} to {}", attr_name, value);
        } else {
            let error_msg = match result {
                -25200 => "Permission denied",
                -25201 => "Action not supported",
                -25202 => "Attribute not settable",
                -25203 => "Attribute not supported",
                -25204 => "Invalid UI element",
                -25205 => "Cannot complete action",
                -25211 => "Not implemented",
                _ => "Unknown error",
            };
            println!("✗ Failed to set {}: {} (error {})", attr_name, error_msg, result);
        }
    }
}

fn find_web_content_group(element: &AXUIElement, depth: usize) -> Option<AXUIElement> {
    if depth > 20 { return None; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if title_str.contains("Web content") {
                return Some(element.clone());
            }
        }
    }
    
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

fn find_text_elements_deep(element: &AXUIElement, depth: usize) {
    if depth > 100 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check for text-related roles
    if matches!(role.as_str(), "AXStaticText" | "AXTextField" | "AXTextArea" | "AXHeading" | "AXLink" | "AXButton") {
        // Check AXValue
        if let Ok(value) = element.value() {
            if let Some(text) = cftype_to_string(&value) {
                if !text.is_empty() {
                    println!("[{}] Value: \"{}\"", role, text);
                }
            }
        }
        
        // Check AXTitle
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if !title_str.is_empty() {
                println!("[{}] Title: \"{}\"", role, title_str);
            }
        }
        
        // Check AXDescription
        let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
        if let Ok(desc) = element.attribute(&desc_attr) {
            if let Some(text) = cftype_to_string(&desc) {
                if !text.is_empty() {
                    println!("[{}] Description: \"{}\"", role, text);
                }
            }
        }
    }
    
    // Also check AXNumberOfCharacters on any element
    let num_chars_attr = AXAttribute::<CFType>::new(&CFString::new("AXNumberOfCharacters"));
    if let Ok(num_chars_value) = element.attribute(&num_chars_attr) {
        if let Some(num_chars) = cftype_to_i64(&num_chars_value) {
            if num_chars > 0 {
                println!("[{}] Has {} characters!", role, num_chars);
            }
        }
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_text_elements_deep(&child, depth + 1);
            }
        }
    }
}

fn get_string_for_range(element: &AXUIElement, start: usize, length: usize) -> Option<String> {
    unsafe {
        use accessibility_sys::AXUIElementCopyParameterizedAttributeValue;
        use core_foundation::base::CFRelease;
        
        let range = core_foundation::base::CFRange {
            location: start as isize,
            length: length as isize,
        };
        
        let range_value = accessibility_sys::AXValueCreate(
            4, // kAXValueCFRangeType
            &range as *const _ as *const std::ffi::c_void
        );
        
        if range_value.is_null() {
            return None;
        }
        
        let attr_name = CFString::new("AXStringForRange");
        let mut result: core_foundation::base::CFTypeRef = std::ptr::null();
        
        let error = AXUIElementCopyParameterizedAttributeValue(
            element.as_concrete_TypeRef(),
            attr_name.as_concrete_TypeRef(),
            range_value as core_foundation::base::CFTypeRef,
            &mut result
        );
        
        CFRelease(range_value as *const _);
        
        if error != 0 || result.is_null() {
            return None;
        }
        
        let cf_type = CFType::wrap_under_create_rule(result);
        cftype_to_string(&cf_type)
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
