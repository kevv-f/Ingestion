//! Debug tool to explore Microsoft Teams' accessibility tree and identify version.

use std::time::Duration;
use accessibility::{AXUIElement, AXUIElementAttributes};
use accessibility::attribute::AXAttribute;
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;

#[derive(Debug)]
struct TeamsVersionInfo {
    bundle_id: String,
    version_type: TeamsVersionType,
    pid: i32,
    app_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TeamsVersionType {
    Classic,
    New,
}

impl TeamsVersionType {
    fn as_str(&self) -> &'static str {
        match self {
            TeamsVersionType::Classic => "Classic (Electron)",
            TeamsVersionType::New => "New (Native/WebKit)",
        }
    }
}

fn main() {
    env_logger::init();
    
    let sep = "============================================================";
    
    println!("Microsoft Teams Accessibility Debug Tool\n");
    println!("{}", sep);
    
    println!("\nSTEP 1: Detecting installed Teams version...\n");
    
    let teams_info = detect_teams_version();
    
    match &teams_info {
        Some(info) => {
            println!("Found Microsoft Teams!");
            println!("   Bundle ID: {}", info.bundle_id);
            println!("   Version Type: {}", info.version_type.as_str());
            println!("   PID: {}", info.pid);
            println!("   App Name: {}", info.app_name);
        }
        None => {
            println!("Microsoft Teams is not running.");
            println!("\nPlease start Microsoft Teams and run this tool again.");
            return;
        }
    }

    let info = teams_info.unwrap();
    
    println!("\nSTEP 2: Enabling accessibility...\n");
    
    if info.version_type == TeamsVersionType::Classic {
        println!("   Classic Teams detected - enabling Electron accessibility...");
        let _ = accessibility_extractor::platform::macos::enable_electron_accessibility(info.pid);
        std::thread::sleep(Duration::from_millis(500));
        println!("   Accessibility enabled for Electron app");
    } else {
        println!("   New Teams detected - using native accessibility...");
        println!("   Note: New Teams may have limited accessibility tree exposure");
    }
    
    println!("\nSTEP 3: Getting app reference...\n");
    
    let app = match accessibility_extractor::platform::macos::MacOSExtractor::get_app_by_bundle_id(
        &info.bundle_id,
        Duration::from_secs(5)
    ) {
        Ok(app) => {
            println!("   Got app reference");
            app
        }
        Err(e) => {
            eprintln!("   Failed to get app reference: {}", e);
            return;
        }
    };
    
    println!("\nSTEP 4: Exploring accessibility tree...\n");
    
    if let Ok(window) = app.focused_window() {
        if let Ok(title) = window.title() {
            println!("   Window Title: \"{}\"", title);
        }
    }
    
    println!("\n   Element counts by role:");
    let mut role_counts = std::collections::HashMap::new();
    count_elements_by_role(&app, &mut role_counts, 0);
    
    let mut sorted_roles: Vec<_> = role_counts.iter().collect();
    sorted_roles.sort_by(|a, b| b.1.cmp(a.1));
    
    for (role, count) in sorted_roles.iter().take(20) {
        println!("      {}: {}", role, count);
    }
    
    println!("\nSTEP 5: Looking for AXWebArea elements (web content)...\n");
    
    let mut web_areas: Vec<AXUIElement> = Vec::new();
    find_web_areas(&app, &mut web_areas, 0);
    
    println!("   Found {} AXWebArea elements", web_areas.len());
    
    if web_areas.is_empty() {
        println!("\n   No AXWebArea found. This might indicate:");
        println!("      - The accessibility tree is not fully exposed");
        println!("      - Teams is showing a non-chat view");
        println!("      - New Teams has different accessibility structure");
    }
    
    println!("\nSTEP 6: Looking for message content...\n");
    
    let mut messages: Vec<(String, String, String)> = Vec::new();
    find_message_content(&app, &mut messages, 0);
    
    println!("   Found {} potential message elements:\n", messages.len());
    
    for (i, (role, attr, value)) in messages.iter().take(20).enumerate() {
        let display_value = if value.len() > 100 {
            format!("{}...", &value[..100])
        } else {
            value.clone()
        };
        println!("   {}. [{}] {} = \"{}\"", i + 1, role, attr, display_value.replace('\n', "\\n"));
    }
    
    if messages.len() > 20 {
        println!("\n   ... and {} more messages", messages.len() - 20);
    }

    println!("\nSTEP 7: Looking for AXGroup elements with AXTitle...\n");
    
    let mut groups: Vec<(String, String)> = Vec::new();
    find_groups_with_titles(&app, &mut groups, 0);
    
    println!("   Found {} AXGroup elements with titles:\n", groups.len());
    
    for (i, (role, title)) in groups.iter().take(15).enumerate() {
        let display_title = if title.len() > 150 {
            format!("{}... [len={}]", &title[..150], title.len())
        } else {
            title.clone()
        };
        println!("   {}. [{}] AXTitle = \"{}\"", i + 1, role, display_title.replace('\n', "\\n"));
    }
    
    println!("\nSTEP 8: Looking for AXStaticText elements...\n");
    
    let mut static_texts: Vec<String> = Vec::new();
    find_static_text(&app, &mut static_texts, 0);
    
    println!("   Found {} AXStaticText elements:\n", static_texts.len());
    
    let meaningful_texts: Vec<_> = static_texts.iter()
        .filter(|t| t.len() > 10 && !is_ui_text(t))
        .take(20)
        .collect();
    
    for (i, text) in meaningful_texts.iter().enumerate() {
        let display_text = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            (*text).clone()
        };
        println!("   {}. \"{}\"", i + 1, display_text.replace('\n', "\\n"));
    }
    
    println!("\n{}", sep);
    println!("SUMMARY");
    println!("{}", sep);
    println!("\nTeams Version: {} ({})", info.version_type.as_str(), info.bundle_id);
    println!("Total Elements: {}", role_counts.values().sum::<usize>());
    println!("AXWebArea Count: {}", web_areas.len());
    println!("Message-like Elements: {}", messages.len());
    println!("AXGroup with Title: {}", groups.len());
    println!("AXStaticText Elements: {}", static_texts.len());
    
    if info.version_type == TeamsVersionType::New && web_areas.is_empty() {
        println!("\nIMPORTANT: New Teams appears to have limited accessibility tree exposure.");
        println!("   The extraction may need to use alternative methods.");
    }
}

fn detect_teams_version() -> Option<TeamsVersionInfo> {
    use accessibility_extractor::platform::macos::get_pid_for_bundle_id;
    
    if let Some(pid) = get_pid_for_bundle_id("com.microsoft.teams2") {
        return Some(TeamsVersionInfo {
            bundle_id: "com.microsoft.teams2".to_string(),
            version_type: TeamsVersionType::New,
            pid,
            app_name: get_app_name(pid).unwrap_or_else(|| "Microsoft Teams".to_string()),
        });
    }
    
    if let Some(pid) = get_pid_for_bundle_id("com.microsoft.teams") {
        return Some(TeamsVersionInfo {
            bundle_id: "com.microsoft.teams".to_string(),
            version_type: TeamsVersionType::Classic,
            pid,
            app_name: get_app_name(pid).unwrap_or_else(|| "Microsoft Teams classic".to_string()),
        });
    }
    
    None
}

fn get_app_name(pid: i32) -> Option<String> {
    let app = AXUIElement::application(pid);
    app.title().ok().map(|s| s.to_string())
}

fn count_elements_by_role(element: &AXUIElement, counts: &mut std::collections::HashMap<String, usize>, depth: usize) {
    if depth > 50 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_else(|_| "Unknown".to_string());
    *counts.entry(role).or_insert(0) += 1;
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                count_elements_by_role(&child, counts, depth + 1);
            }
        }
    }
}

fn find_web_areas(element: &AXUIElement, results: &mut Vec<AXUIElement>, depth: usize) {
    if depth > 30 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXWebArea" {
        results.push(element.clone());
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_web_areas(&child, results, depth + 1);
            }
        }
    }
}

fn find_message_content(element: &AXUIElement, results: &mut Vec<(String, String, String)>, depth: usize) {
    if depth > 50 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if let Ok(title) = element.title() {
        let title_str = title.to_string();
        if looks_like_message(&title_str) {
            results.push((role.clone(), "AXTitle".to_string(), title_str));
        }
    }
    
    if let Ok(value) = element.value() {
        if let Some(text) = cftype_to_string(&value) {
            if looks_like_message(&text) {
                results.push((role.clone(), "AXValue".to_string(), text));
            }
        }
    }
    
    let desc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDescription"));
    if let Ok(value) = element.attribute(&desc_attr) {
        if let Some(text) = cftype_to_string(&value) {
            if looks_like_message(&text) {
                results.push((role.clone(), "AXDescription".to_string(), text));
            }
        }
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_message_content(&child, results, depth + 1);
            }
        }
    }
}

fn looks_like_message(text: &str) -> bool {
    let text = text.trim();
    text.len() > 20 && (text.contains(':') || text.contains("AM") || text.contains("PM"))
}

fn find_groups_with_titles(element: &AXUIElement, results: &mut Vec<(String, String)>, depth: usize) {
    if depth > 50 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXGroup" {
        if let Ok(title) = element.title() {
            let title_str = title.to_string();
            if !title_str.trim().is_empty() && title_str.len() > 5 {
                results.push((role.clone(), title_str));
            }
        }
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_groups_with_titles(&child, results, depth + 1);
            }
        }
    }
}

fn find_static_text(element: &AXUIElement, results: &mut Vec<String>, depth: usize) {
    if depth > 50 { return; }
    
    let role = element.role().map(|s| s.to_string()).unwrap_or_default();
    
    if role == "AXStaticText" {
        if let Ok(value) = element.value() {
            if let Some(text) = cftype_to_string(&value) {
                if !text.trim().is_empty() {
                    results.push(text);
                }
            }
        }
    }
    
    if let Ok(children) = element.children() {
        for i in 0..children.len() {
            if let Some(child) = children.get(i) {
                find_static_text(&child, results, depth + 1);
            }
        }
    }
}

fn is_ui_text(text: &str) -> bool {
    let ui_labels = [
        "Chat", "Teams", "Calendar", "Calls", "Files", "Activity",
        "More", "Search", "Settings", "Help", "New chat", "New meeting",
        "Meet", "Join", "Leave", "Mute", "Unmute", "Share", "React",
        "Reply", "Forward", "Copy", "Delete", "Edit", "Pin", "Save",
    ];
    ui_labels.contains(&text.trim())
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
