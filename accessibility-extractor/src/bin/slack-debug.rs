//! Debug tool to explore Slack's accessibility tree and find message content.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;

fn main() {
    env_logger::init();
    
    println!("🔍 Slack Accessibility Tree Debug Tool\n");
    
    // Enable accessibility for Slack first
    if let Some(pid) = accessibility_extractor::platform::macos::get_pid_for_bundle_id("com.tinyspeck.slackmacgap") {
        println!("Found Slack with PID: {}", pid);
        let _ = accessibility_extractor::platform::macos::enable_electron_accessibility(pid);
        std::thread::sleep(Duration::from_millis(500));
    }
    
    // Get Slack app
    let app = match accessibility_extractor::platform::macos::MacOSExtractor::get_app_by_bundle_id(
        "com.tinyspeck.slackmacgap",
        Duration::from_secs(5)
    ) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to get Slack: {}", e);
            return;
        }
    };
    
    println!("\n=== Looking for DATE SEPARATORS in Slack ===\n");
    
    // Find all elements that might be date separators
    let mut date_elements: Vec<(String, String, String)> = Vec::new();
    find_date_elements(&app, &mut date_elements, 0);
    
    println!("Found {} potential date elements:\n", date_elements.len());
    for (i, (role, attr, value)) in date_elements.iter().enumerate() {
        println!("{}. [{}] {} = \"{}\"", i + 1, role, attr, value);
    }
    
    println!("\n=== Searching for AXGroup elements with AXTitle (message containers) ===\n");
    
    // Find AXGroup elements with titles (these contain message info)
    let mut groups: Vec<(String, String)> = Vec::new();
    find_groups_with_titles(&app, &mut groups, 0);
    
    println!("Found {} AXGroup elements with titles:\n", groups.len());
    
    // Show all messages, including long ones
    for (i, (role, title)) in groups.iter().enumerate() {
        let display_title = if title.len() > 200 { 
            format!("{}... [TRUNCATED, full length: {}]", &title[..200], title.len()) 
        } else { 
            title.clone() 
        };
        println!("{}. [{}] len={} AXTitle = \"{}\"", i + 1, role, title.len(),
            display_title.replace('\n', "\\n"));
    }
    
    println!("\n=== Testing timestamp extraction on all messages ===\n");
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for (i, (_, title)) in groups.iter().enumerate() {
        if let Some((content, time)) = extract_time_from_title(title) {
            success_count += 1;
            if i < 10 || title.len() > 100 {  // Show first 10 and any long messages
                println!("✅ #{} (len={}):", i + 1, title.len());
                println!("   Time: \"{}\"", time);
                println!("   Content: \"{}\"", if content.len() > 80 { format!("{}...", &content[..80]) } else { content.clone() });
            }
        } else {
            fail_count += 1;
            println!("❌ #{} (len={}) - NO TIME FOUND:", i + 1, title.len());
            println!("   Title: \"{}\"", if title.len() > 100 { format!("{}...", &title[..100]) } else { title.clone() });
        }
    }
    
    println!("\n=== Summary ===");
    println!("✅ Successfully parsed: {}", success_count);
    println!("❌ Failed to parse: {}", fail_count);
}

/// Find elements that look like date separators
fn find_date_elements(element: &AXUIElement, results: &mut Vec<(String, String, String)>, depth: usize) {
    if depth > 50 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check various attributes for date-like content
    // Slack shows dates like "Today", "Yesterday", "Monday, February 10th", etc.
    
    // Check AXTitle
    if let Ok(title) = element.title() {
        let title_str = title.to_string();
        if is_date_like(&title_str) {
            results.push((role.clone(), "AXTitle".to_string(), title_str));
        }
    }
    
    // Check AXValue
    if let Ok(value) = element.value() {
        if let Some(text) = cftype_to_string(&value) {
            if is_date_like(&text) {
                results.push((role.clone(), "AXValue".to_string(), text));
            }
        }
    }
    
    // Check AXDescription
    let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
    if let Ok(value) = element.attribute(&desc_attr) {
        if let Some(text) = cftype_to_string(&value) {
            if is_date_like(&text) {
                results.push((role.clone(), "AXDescription".to_string(), text));
            }
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_date_elements(&child, results, depth + 1);
            }
        }
    }
}

/// Check if text looks like a date separator
fn is_date_like(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.len() > 50 {
        return false;
    }
    
    // Common date patterns in Slack
    let date_keywords = [
        "Today", "Yesterday", "Monday", "Tuesday", "Wednesday", "Thursday", 
        "Friday", "Saturday", "Sunday", "January", "February", "March", 
        "April", "May", "June", "July", "August", "September", "October", 
        "November", "December"
    ];
    
    for keyword in date_keywords {
        if text.contains(keyword) {
            return true;
        }
    }
    
    // Check for date patterns like "Feb 10th", "February 10", etc.
    let date_pattern = regex_lite::Regex::new(
        r"(?i)(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\s+\d{1,2}"
    );
    if let Ok(pattern) = date_pattern {
        if pattern.is_match(text) {
            return true;
        }
    }
    
    false
}

fn find_groups_with_titles(element: &AXUIElement, results: &mut Vec<(String, String)>, depth: usize) {
    if depth > 50 {
        return;
    }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    // Check AXGroup elements for AXTitle
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            // Include messages of any length, but filter out very short UI elements
            if !title_str.trim().is_empty() && title_str.len() > 10 && title_str.contains(':') {
                results.push((role.clone(), title_str));
            }
        }
    }
    
    // Traverse children
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_groups_with_titles(&child, results, depth + 1);
            }
        }
    }
}

/// Extract time from the end of a Slack message title
fn extract_time_from_title(title: &str) -> Option<(String, String)> {
    let title = title.trim();
    
    // Pattern 1: Time with period before and after
    let pattern1 = regex_lite::Regex::new(
        r"[.\s]+(\d{1,2}:\d{2}\s*(?:AM|PM))\.(?:\s*\d+\s*(?:reaction|link|file|reply|replies|attachment)s?\.?)*\s*$"
    ).ok()?;
    
    if let Some(caps) = pattern1.captures(title) {
        if let (Some(time_match), Some(full_match)) = (caps.get(1), caps.get(0)) {
            let time = time_match.as_str().trim().to_string();
            let content = title[..full_match.start()].trim();
            if !content.is_empty() {
                return Some((content.to_string(), time));
            }
        }
    }
    
    // Pattern 2: Simpler - just period before time
    let pattern2 = regex_lite::Regex::new(r"\.\s*(\d{1,2}:\d{2}\s*(?:AM|PM))\.\s*$").ok()?;
    if let Some(caps) = pattern2.captures(title) {
        if let (Some(time_match), Some(full_match)) = (caps.get(1), caps.get(0)) {
            let time = time_match.as_str().trim().to_string();
            let content = title[..full_match.start()].trim();
            if !content.is_empty() {
                return Some((content.to_string(), time));
            }
        }
    }
    
    // Pattern 3: Time at end without trailing period
    let pattern3 = regex_lite::Regex::new(r"[.\s]+(\d{1,2}:\d{2}\s*(?:AM|PM))\s*$").ok()?;
    if let Some(caps) = pattern3.captures(title) {
        if let (Some(time_match), Some(full_match)) = (caps.get(1), caps.get(0)) {
            let time = time_match.as_str().trim().to_string();
            let content = title[..full_match.start()].trim();
            if !content.is_empty() {
                return Some((content.to_string(), time));
            }
        }
    }
    
    // Pattern 4: Look for time anywhere near the end
    if title.len() > 30 {
        let pattern4 = regex_lite::Regex::new(r"(\d{1,2}:\d{2}\s*(?:AM|PM))").ok()?;
        if let Some(caps) = pattern4.captures(&title[title.len().saturating_sub(50)..]) {
            if let Some(time_match) = caps.get(1) {
                let time = time_match.as_str().trim().to_string();
                // Find this time in the original and split
                if let Some(pos) = title.rfind(&time) {
                    let mut start = pos;
                    while start > 0 {
                        let c = title.as_bytes()[start - 1];
                        if c == b'.' || c == b' ' {
                            start -= 1;
                        } else {
                            break;
                        }
                    }
                    let content = title[..start].trim();
                    if !content.is_empty() {
                        return Some((content.to_string(), time));
                    }
                }
            }
        }
    }
    
    None
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
