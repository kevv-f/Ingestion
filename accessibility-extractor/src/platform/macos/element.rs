//! Element helper utilities for macOS Accessibility API.
//!
//! This module provides debugging and utility functions for working with
//! AXUIElement instances, including tree visualization, attribute inspection,
//! and element searching.
//!
//! # Requirements
//! - 12.1: Get all attribute names for an AXUIElement
//! - 12.2: Print the accessibility tree for debugging
//! - 12.3: Find elements by role within a subtree

use accessibility::{AXUIElement, AXUIElementAttributes};

/// Get all attribute names for an element (debugging).
///
/// This function retrieves all available accessibility attribute names
/// for a given element, which is useful for debugging and understanding
/// what information is available from an element.
///
/// # Arguments
///
/// * `element` - The AXUIElement to inspect
///
/// # Returns
///
/// A vector of attribute name strings. Returns an empty vector if
/// attribute names cannot be retrieved.
///
/// # Example
///
/// ```ignore
/// let element = get_some_element();
/// let attrs = get_attribute_names(&element);
/// for attr in attrs {
///     println!("Attribute: {}", attr);
/// }
/// ```
///
/// # Requirements
/// - Validates: Requirement 12.1
pub fn get_attribute_names(element: &AXUIElement) -> Vec<String> {
    element
        .attribute_names()
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

/// Print element tree for debugging.
///
/// This function recursively prints the accessibility tree starting from
/// the given element, showing the role and title of each element with
/// indentation to indicate hierarchy.
///
/// # Arguments
///
/// * `element` - The root AXUIElement to start printing from
/// * `indent` - The current indentation level (use 0 for root)
///
/// # Output Format
///
/// Each element is printed as: `{indent}{role} - {title}`
/// where indent is two spaces per level.
///
/// # Example
///
/// ```ignore
/// let window = get_focused_window();
/// debug_print_tree(&window, 0);
/// // Output:
/// // AXWindow - Document1.docx
/// //   AXScrollArea -
/// //     AXTextArea - Document content...
/// ```
///
/// # Requirements
/// - Validates: Requirement 12.2
pub fn debug_print_tree(element: &AXUIElement, indent: usize) {
    // Limit depth to prevent excessive output
    if indent > 10 {
        return;
    }
    
    let prefix = "  ".repeat(indent);

    let role = element
        .role()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "?".to_string());

    let title = element
        .title()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| String::new());
    
    // For text-related roles, also try to get the value
    let value_preview = if matches!(role.as_str(), "AXLayoutArea" | "AXTextArea" | "AXTextField" | "AXStaticText" | "AXDocument" | "AXWebArea") {
        element.value()
            .ok()
            .and_then(|v| {
                // Try to convert to string
                use core_foundation::base::TCFType;
                use core_foundation::string::CFString;
                let type_id = v.type_of();
                if type_id == CFString::type_id() {
                    let ptr = v.as_CFTypeRef();
                    let cf_string: CFString = unsafe {
                        CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
                    };
                    let s = cf_string.to_string();
                    if !s.is_empty() {
                        let preview = if s.len() > 50 { format!("{}...", &s[..50]) } else { s };
                        Some(format!(" [value: \"{}\"]", preview.replace('\n', "\\n")))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    println!("{}{} - {}{}", prefix, role, title, value_preview);

    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                debug_print_tree(&child, indent + 1);
            }
        }
    }
}

/// Print all attributes of an element for debugging.
///
/// This function prints all available accessibility attributes and their
/// values for a given element, useful for understanding what data is available.
pub fn debug_print_attributes(element: &AXUIElement) {
    use accessibility::attribute::AXAttribute;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::string::CFString;
    use core_foundation::array::CFArray;
    
    let attrs = get_attribute_names(element);
    println!("Attributes ({}):", attrs.len());
    
    for attr_name in attrs {
        let attr = AXAttribute::<CFType>::new(&CFString::new(&attr_name));
        match element.attribute(&attr) {
            Ok(value) => {
                let type_id = value.type_of();
                let type_name = if type_id == CFString::type_id() {
                    "CFString"
                } else if type_id == AXUIElement::type_id() {
                    "AXUIElement"
                } else {
                    "Other"
                };
                
                // Try to get string value
                let str_value = if type_id == CFString::type_id() {
                    let ptr = value.as_CFTypeRef();
                    let cf_string: CFString = unsafe {
                        CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
                    };
                    let s = cf_string.to_string();
                    if s.len() > 200 {
                        format!("\"{}...\" ({} chars)", &s[..200].replace('\n', "\\n"), s.len())
                    } else {
                        format!("\"{}\"", s.replace('\n', "\\n"))
                    }
                } else {
                    format!("<{}>", type_name)
                };
                
                println!("  {} = {}", attr_name, str_value);
            }
            Err(_) => {
                println!("  {} = <error>", attr_name);
            }
        }
    }
    
    // Get parameterized attribute names using raw API
    unsafe {
        let mut names: core_foundation::base::CFTypeRef = std::ptr::null();
        let result = accessibility_sys::AXUIElementCopyParameterizedAttributeNames(
            element.as_concrete_TypeRef(),
            &mut names as *mut _ as *mut _
        );
        
        if result == accessibility_sys::kAXErrorSuccess && !names.is_null() {
            let cf_array: CFArray<CFString> = CFArray::wrap_under_create_rule(names as _);
            if cf_array.len() > 0 {
                println!("\nParameterized Attributes ({}):", cf_array.len());
                for i in 0..cf_array.len() {
                    if let Some(name) = cf_array.get(i) {
                        println!("  {}", name.to_string());
                    }
                }
            }
        }
    }
}

/// Find elements by role within a subtree.
///
/// This function recursively searches the accessibility tree starting from
/// the given root element and returns all elements that match the specified
/// role.
///
/// # Arguments
///
/// * `root` - The root AXUIElement to start searching from
/// * `target_role` - The accessibility role to search for (e.g., "AXTextArea", "AXButton")
///
/// # Returns
///
/// A vector of AXUIElement instances that match the target role.
/// Returns an empty vector if no matching elements are found.
///
/// # Example
///
/// ```ignore
/// let window = get_focused_window();
/// let text_areas = find_elements_by_role(&window, "AXTextArea");
/// println!("Found {} text areas", text_areas.len());
/// ```
///
/// # Requirements
/// - Validates: Requirement 12.3
pub fn find_elements_by_role(root: &AXUIElement, target_role: &str) -> Vec<AXUIElement> {
    let mut results = Vec::new();
    find_elements_recursive(root, target_role, &mut results);
    results
}

/// Recursive helper for find_elements_by_role.
///
/// This internal function performs the actual recursive traversal of the
/// accessibility tree, collecting elements that match the target role.
///
/// # Arguments
///
/// * `element` - The current element being examined
/// * `target_role` - The accessibility role to search for
/// * `results` - Mutable vector to collect matching elements
fn find_elements_recursive(
    element: &AXUIElement,
    target_role: &str,
    results: &mut Vec<AXUIElement>,
) {
    if let Ok(role) = element.role() {
        if role.to_string() == target_role {
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

#[cfg(test)]
mod tests {
    // Note: These tests require actual AXUIElement instances which can only
    // be obtained at runtime with accessibility permissions. The functions
    // are tested through integration tests that run with proper permissions.
    //
    // Unit tests here would require mocking the accessibility framework,
    // which is not practical for this low-level API.
}
