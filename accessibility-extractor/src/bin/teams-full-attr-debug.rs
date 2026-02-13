//! Full attribute debug tool for New Teams.
//! This tool dumps ALL attributes (regular and parameterized) on each element
//! to find any way to extract text content from Chromium-based web views.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;
use core_foundation::array::CFArray;

fn main() {
    env_logger::init();
    
    println!("New Teams Full Attribute Debug\n");
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
    println!("Exploring accessibility tree with full attribute dump...\n");
    
    // Find the web content group
    if let Some(web_group) = find_web_content_group(&window, 0) {
        println!("Found web content group!\n");
        
        // Dump all attributes on the web content group
        println!("=== Web Content Group Attributes ===");
        dump_all_attributes(&web_group);
        
        // Explore first few levels of children with full attribute dump
        println!("\n=== Exploring Children ===\n");
        explore_with_attributes(&web_group, 0, 5);
        
        // Try to find any element with non-empty attributes
        println!("\n=== Looking for elements with interesting attributes ===\n");
        find_interesting_elements(&web_group, 0);
    } else {
        println!("Could not find web content group");
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

fn dump_all_attributes(element: &AXUIElement) {
    // Get all regular attributes
    unsafe {
        use accessibility_sys::AXUIElementCopyAttributeNames;
        
        let mut names: core_foundation::array::CFArrayRef = std::ptr::null();
        let result = AXUIElementCopyAttributeNames(
            element.as_concrete_TypeRef(),
            &mut names
        );
        
        if result == 0 && !names.is_null() {
            let cf_array: CFArray<CFString> = CFArray::wrap_under_get_rule(names);
            println!("Regular attributes ({}):", cf_array.len());
            
            for i in 0..cf_array.len() {
                if let Some(attr_name) = cf_array.get(i) {
                    let attr_str = attr_name.to_string();
                    
                    // Try to get the value
                    let attr = AXAttribute::<CFType>::new(&CFString::new(&attr_str));
                    let value_str = if let Ok(value) = element.attribute(&attr) {
                        describe_cftype(&value)
                    } else {
                        "<error>".to_string()
                    };
                    
                    println!("  {} = {}", attr_str, value_str);
                }
            }
        }
    }
    
    // Get all parameterized attributes
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
                println!("\nParameterized attributes ({}):", cf_array.len());
                
                for i in 0..cf_array.len() {
                    if let Some(attr_name) = cf_array.get(i) {
                        println!("  {}", attr_name.to_string());
                    }
                }
            }
        }
    }
    
    // Get actions
    unsafe {
        use accessibility_sys::AXUIElementCopyActionNames;
        
        let mut names: core_foundation::array::CFArrayRef = std::ptr::null();
        let result = AXUIElementCopyActionNames(
            element.as_concrete_TypeRef(),
            &mut names
        );
        
        if result == 0 && !names.is_null() {
            let cf_array: CFArray<CFString> = CFArray::wrap_under_get_rule(names);
            if cf_array.len() > 0 {
                println!("\nActions ({}):", cf_array.len());
                
                for i in 0..cf_array.len() {
                    if let Some(action_name) = cf_array.get(i) {
                        println!("  {}", action_name.to_string());
                    }
                }
            }
        }
    }
}

fn describe_cftype(value: &CFType) -> String {
    let type_id = value.type_of();
    
    if type_id == CFString::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_string: CFString = unsafe {
            CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
        };
        let s = cf_string.to_string();
        if s.len() > 100 {
            format!("\"{}...\" (len={})", &s[..100], s.len())
        } else if s.is_empty() {
            "\"\" (empty)".to_string()
        } else {
            format!("\"{}\"", s)
        }
    } else if type_id == core_foundation::number::CFNumber::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_number: core_foundation::number::CFNumber = unsafe {
            core_foundation::number::CFNumber::wrap_under_get_rule(ptr as core_foundation::number::CFNumberRef)
        };
        if let Some(n) = cf_number.to_i64() {
            format!("{}", n)
        } else if let Some(f) = cf_number.to_f64() {
            format!("{}", f)
        } else {
            "<number>".to_string()
        }
    } else if type_id == core_foundation::boolean::CFBoolean::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_bool: core_foundation::boolean::CFBoolean = unsafe {
            core_foundation::boolean::CFBoolean::wrap_under_get_rule(ptr as core_foundation::boolean::CFBooleanRef)
        };
        format!("{}", cf_bool == core_foundation::boolean::CFBoolean::true_value())
    } else if type_id == CFArray::<CFType>::type_id() {
        let ptr = value.as_CFTypeRef();
        let cf_array: CFArray<CFType> = unsafe {
            CFArray::wrap_under_get_rule(ptr as core_foundation::array::CFArrayRef)
        };
        format!("[Array with {} items]", cf_array.len())
    } else if type_id == AXUIElement::type_id() {
        "<AXUIElement>".to_string()
    } else {
        format!("<CFType id={}>", type_id)
    }
}

fn explore_with_attributes(element: &AXUIElement, depth: usize, max_depth: usize) {
    if depth > max_depth { return; }
    
    let prefix = "  ".repeat(depth);
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                let role = child.role().map(|s| s.to_string()).unwrap_or_else(|_| "Unknown".to_string());
                
                println!("{}[{}] {}", prefix, i, role);
                
                // Get key attributes
                let attrs_to_check = [
                    "AXValue", "AXTitle", "AXDescription", "AXHelp",
                    "AXRoleDescription", "AXNumberOfCharacters",
                    "ChromeAXNodeId", "AXDOMIdentifier", "AXDOMClassList",
                ];
                
                for attr_name in attrs_to_check {
                    let attr = AXAttribute::<CFType>::new(&CFString::new(attr_name));
                    if let Ok(value) = child.attribute(&attr) {
                        let value_str = describe_cftype(&value);
                        if !value_str.contains("empty") && !value_str.contains("<error>") && value_str != "0" {
                            println!("{}  {} = {}", prefix, attr_name, value_str);
                        }
                    }
                }
                
                // Check parameterized attributes
                unsafe {
                    use accessibility_sys::AXUIElementCopyParameterizedAttributeNames;
                    
                    let mut names: core_foundation::array::CFArrayRef = std::ptr::null();
                    let result = AXUIElementCopyParameterizedAttributeNames(
                        child.as_concrete_TypeRef(),
                        &mut names
                    );
                    
                    if result == 0 && !names.is_null() {
                        let cf_array: CFArray<CFString> = CFArray::wrap_under_get_rule(names);
                        if cf_array.len() > 0 {
                            let param_attrs: Vec<String> = (0..cf_array.len())
                                .filter_map(|i| cf_array.get(i).map(|s| s.to_string()))
                                .collect();
                            println!("{}  [Parameterized: {}]", prefix, param_attrs.join(", "));
                        }
                    }
                }
                
                // Recurse
                explore_with_attributes(&child, depth + 1, max_depth);
            }
        }
    }
}

fn find_interesting_elements(element: &AXUIElement, depth: usize) {
    if depth > 50 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check for elements with actual content
    let interesting_attrs = [
        ("AXValue", true),
        ("AXTitle", true),
        ("AXDescription", true),
        ("AXDOMIdentifier", false),
        ("AXDOMClassList", false),
    ];
    
    let mut found_interesting = false;
    let mut interesting_info = Vec::new();
    
    for (attr_name, check_content) in interesting_attrs {
        let attr = AXAttribute::<CFType>::new(&CFString::new(attr_name));
        if let Ok(value) = element.attribute(&attr) {
            let value_str = describe_cftype(&value);
            if check_content {
                // Only report if has actual content
                if !value_str.contains("empty") && !value_str.contains("<error>") && value_str != "0" && value_str.len() > 5 {
                    found_interesting = true;
                    interesting_info.push(format!("{} = {}", attr_name, value_str));
                }
            } else {
                // Report DOM-related attributes regardless
                if !value_str.contains("<error>") {
                    found_interesting = true;
                    interesting_info.push(format!("{} = {}", attr_name, value_str));
                }
            }
        }
    }
    
    if found_interesting {
        println!("[{}] {}", role, interesting_info.join(", "));
    }
    
    // Recurse
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_interesting_elements(&child, depth + 1);
            }
        }
    }
}
