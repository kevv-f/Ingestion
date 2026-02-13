//! Deep debug tool to explore ALL accessibility attributes of Microsoft Teams.
//! This tool checks for AXTextMarker and other WebKit-specific attributes.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_foundation::array::CFArray;

fn main() {
    env_logger::init();
    
    println!("Microsoft Teams Deep Accessibility Debug Tool\n");
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
    println!("STEP 1: Listing ALL attributes on the window...\n");
    
    list_all_attributes(&window, "Window");
    
    println!("\n============================================================");
    println!("STEP 2: Looking for AXGroup with web content...\n");
    
    find_and_inspect_groups(&window, 0);
    
    println!("\n============================================================");
    println!("STEP 3: Looking for ANY element with text markers...\n");
    
    find_text_marker_elements(&window, 0);
    
    println!("\n============================================================");
    println!("STEP 4: Checking for AXValue on all elements...\n");
    
    find_elements_with_value(&window, 0);
}

fn list_all_attributes(element: &AXUIElement, name: &str) {
    // Try using the raw API to get all attribute names
    unsafe {
        use accessibility_sys::AXUIElementCopyAttributeNames;
        
        let mut names: core_foundation::array::CFArrayRef = std::ptr::null();
        let result = AXUIElementCopyAttributeNames(
            element.as_concrete_TypeRef(),
            &mut names
        );
        
        if result == 0 && !names.is_null() {
            let cf_array: CFArray<CFString> = CFArray::wrap_under_get_rule(names);
            println!("{} has {} attributes:", name, cf_array.len());
            
            for i in 0..cf_array.len() {
                if let Some(attr_name) = cf_array.get(i) {
                    let attr_str = attr_name.to_string();
                    print!("  - {}", attr_str);
                    
                    // Check for interesting attributes
                    if attr_str.contains("TextMarker") || 
                       attr_str.contains("Text") ||
                       attr_str.contains("Value") ||
                       attr_str.contains("String") {
                        print!(" <-- INTERESTING");
                    }
                    println!();
                }
            }
        } else {
            println!("{}: Could not get attribute names (error {})", name, result);
        }
    }
    
    // Also check parameterized attributes
    unsafe {
        use accessibility_sys::AXUIElementCopyParameterizedAttributeNames;
        
        let mut names: core_foundation::array::CFArrayRef = std::ptr::null();
        let result = AXUIElementCopyParameterizedAttributeNames(
            element.as_concrete_TypeRef(),
            &mut names
        );
        
        if result == 0 && !names.is_null() {
            let cf_array: CFArray<CFString> = CFArray::wrap_under_get_rule(names);
            if cf_array.len() > 0 {
                println!("\n{} has {} parameterized attributes:", name, cf_array.len());
                
                for i in 0..cf_array.len() {
                    if let Some(attr_name) = cf_array.get(i) {
                        let attr_str = attr_name.to_string();
                        print!("  - {}", attr_str);
                        
                        if attr_str.contains("TextMarker") || 
                           attr_str.contains("Text") ||
                           attr_str.contains("String") {
                            print!(" <-- INTERESTING");
                        }
                        println!();
                    }
                }
            }
        }
    }
}

fn find_and_inspect_groups(element: &AXUIElement, depth: usize) {
    if depth > 20 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check AXGroup elements
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if title_str.contains("Web content") || title_str.contains("Teams") {
                println!("\nFound interesting AXGroup: \"{}\"", title_str);
                list_all_attributes(element, "AXGroup");
                
                // Check children
                if let Ok(children) = element.children() {
                    println!("\n  Children count: {}", children.len());
                    for i in 0..std::cmp::min(children.len(), 5) {
                        if let Some(child) = children.get(i) {
                            let child_role = child.role().map(|s| s.to_string()).unwrap_or_default();
                            println!("    Child {}: {}", i, child_role);
                            
                            // List attributes of first few children
                            if i < 2 {
                                list_all_attributes(&child, &format!("    Child {} ({})", i, child_role));
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_and_inspect_groups(&child, depth + 1);
            }
        }
    }
}

fn find_text_marker_elements(element: &AXUIElement, depth: usize) {
    if depth > 30 { return; }
    
    // Check for text marker attributes
    let text_marker_attrs = [
        "AXStartTextMarker",
        "AXEndTextMarker", 
        "AXSelectedTextMarkerRange",
        "AXTextMarkerRangeForUIElement",
    ];
    
    for attr_name in &text_marker_attrs {
        let attr = AXAttribute::<CFType>::new(&CFString::new(attr_name));
        if element.attribute(&attr).is_ok() {
            let role = element.role().map(|s| s.to_string()).unwrap_or_default();
            println!("Found {} on element with role: {}", attr_name, role);
            list_all_attributes(element, &format!("Element with {}", attr_name));
        }
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_text_marker_elements(&child, depth + 1);
            }
        }
    }
}

fn find_elements_with_value(element: &AXUIElement, depth: usize) {
    if depth > 30 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check AXValue
    if let Ok(value) = element.value() {
        if let Some(text) = cftype_to_string(&value) {
            if text.len() > 20 {
                println!("Found AXValue on {}: \"{}...\" (len={})", 
                    role, 
                    &text[..std::cmp::min(50, text.len())],
                    text.len()
                );
            }
        }
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_elements_with_value(&child, depth + 1);
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
