//! Background daemon for accessibility content extraction.
//!
//! This daemon monitors application activations/deactivations and extracts
//! content from supported apps when the user switches away from them.
//! Content is stored in SQLite with deduplication.

use accessibility_extractor::extractor::AccessibilityExtractor;
use accessibility_extractor::types::ExtractionError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSAutoreleasePool, NSString};
#[cfg(target_os = "macos")]
use objc::declare::ClassDecl;
#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object, Sel};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

use accessibility_extractor::{DaemonStorage, DedupResult};

/// Supported applications for extraction
const SUPPORTED_APPS: &[(&str, &str)] = &[
    ("com.microsoft.Word", "word"),
    ("com.microsoft.Excel", "excel"),
    ("com.microsoft.Powerpoint", "powerpoint"),
    ("com.apple.iWork.Pages", "pages"),
    ("com.apple.iWork.Numbers", "numbers"),
    ("com.apple.iWork.Keynote", "keynote"),
    ("com.tinyspeck.slackmacgap", "slack"),
    ("com.hnc.Discord", "discord"),
    ("com.microsoft.teams", "teams"),      // Classic Teams (Electron-based)
    ("com.microsoft.teams2", "teams"),     // New Teams (WebView/Chromium-based)
];

/// Debounce configuration
const DEBOUNCE_DELAY_MS: u64 = 2000; // 2 seconds

/// State shared between the observer callback and main thread
struct DaemonState {
    /// Last extraction time per app bundle ID
    last_extraction: HashMap<String, Instant>,
    /// Pending extractions (app deactivated, waiting for debounce)
    pending_extractions: HashMap<String, Instant>,
    /// Storage for persisting content
    storage: Option<DaemonStorage>,
    /// Currently active app bundle ID
    current_app: Option<String>,
}

impl DaemonState {
    fn new() -> Self {
        Self {
            last_extraction: HashMap::new(),
            pending_extractions: HashMap::new(),
            storage: None,
            current_app: None,
        }
    }

    fn with_storage(storage: DaemonStorage) -> Self {
        Self {
            last_extraction: HashMap::new(),
            pending_extractions: HashMap::new(),
            storage: Some(storage),
            current_app: None,
        }
    }
}

/// Global state (needed for Objective-C callback)
static mut DAEMON_STATE: Option<Arc<Mutex<DaemonState>>> = None;

fn get_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("clace-ingestion")
        .join("content.db")
}

fn is_supported_app(bundle_id: &str) -> bool {
    SUPPORTED_APPS.iter().any(|(id, _)| *id == bundle_id)
}

fn get_source_type(bundle_id: &str) -> Option<&'static str> {
    SUPPORTED_APPS
        .iter()
        .find(|(id, _)| *id == bundle_id)
        .map(|(_, source)| *source)
}

/// Extract content from an app and store it
fn extract_and_store(bundle_id: &str, state: &mut DaemonState) {
    let source_type = match get_source_type(bundle_id) {
        Some(s) => s,
        None => return,
    };

    log::info!("[AX-DAEMON] 📥 Extracting content from {} ({})", bundle_id, source_type);

    // Extract content
    let content = match AccessibilityExtractor::extract_from_app(bundle_id) {
        Ok(c) => c,
        Err(e) => {
            match &e {
                ExtractionError::NoContentFound(_) => {
                    log::debug!("[AX-DAEMON] ⏭️  No content to extract from {}", bundle_id);
                }
                _ => {
                    log::warn!("[AX-DAEMON] ⚠️  Extraction failed for {}: {}", bundle_id, e);
                }
            }
            return;
        }
    };

    // Check for duplicates and store
    if let Some(ref mut storage) = state.storage {
        match storage.store_content(&content) {
            Ok(result) => match result {
                DedupResult::New(doc_id) => {
                    log::info!(
                        "[AX-DAEMON] ✅ Stored new content: {} ({} chars) [{}]",
                        content.title.as_deref().unwrap_or("untitled"),
                        content.content.len(),
                        doc_id
                    );
                }
                DedupResult::Updated(doc_id) => {
                    log::info!(
                        "[AX-DAEMON] 🔄 Updated content: {} ({} chars) [{}]",
                        content.title.as_deref().unwrap_or("untitled"),
                        content.content.len(),
                        doc_id
                    );
                }
                DedupResult::Duplicate => {
                    log::debug!(
                        "[AX-DAEMON] ⏭️  Content unchanged: {}",
                        content.title.as_deref().unwrap_or("untitled")
                    );
                }
            },
            Err(e) => {
                log::error!("[AX-DAEMON] ❌ Failed to store content: {}", e);
            }
        }
    } else {
        // No storage - just log
        log::info!(
            "[AX-DAEMON] 📄 Extracted: {} - {} ({} chars)",
            source_type,
            content.title.as_deref().unwrap_or("untitled"),
            content.content.len()
        );
    }

    state.last_extraction.insert(bundle_id.to_string(), Instant::now());
}

/// Process pending extractions (called from run loop)
fn process_pending_extractions(state: &mut DaemonState) {
    let now = Instant::now();
    let debounce_duration = Duration::from_millis(DEBOUNCE_DELAY_MS);

    // Collect apps ready for extraction
    let ready: Vec<String> = state
        .pending_extractions
        .iter()
        .filter(|(_, scheduled_time)| now.duration_since(**scheduled_time) >= debounce_duration)
        .map(|(bundle_id, _)| bundle_id.clone())
        .collect();

    // Extract from ready apps
    for bundle_id in ready {
        state.pending_extractions.remove(&bundle_id);
        extract_and_store(&bundle_id, state);
    }
}

#[cfg(target_os = "macos")]
fn get_bundle_id_from_notification(notification: id) -> Option<String> {
    unsafe {
        let user_info: id = msg_send![notification, userInfo];
        if user_info == nil {
            return None;
        }

        let app_key = NSString::alloc(nil).init_str("NSWorkspaceApplicationKey");
        let app: id = msg_send![user_info, objectForKey: app_key];
        if app == nil {
            return None;
        }

        let bundle_id: id = msg_send![app, bundleIdentifier];
        if bundle_id == nil {
            return None;
        }

        let len: usize = msg_send![bundle_id, length];
        if len == 0 {
            return None;
        }

        let c_str: *const i8 = msg_send![bundle_id, UTF8String];
        if c_str.is_null() {
            return None;
        }

        Some(std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned())
    }
}

#[cfg(target_os = "macos")]
extern "C" fn app_activated(_this: &Object, _sel: Sel, notification: id) {
    let bundle_id = match get_bundle_id_from_notification(notification) {
        Some(id) => id,
        None => return,
    };

    unsafe {
        if let Some(ref state_arc) = DAEMON_STATE {
            if let Ok(mut state) = state_arc.lock() {
                log::debug!("[AX-DAEMON] 🔵 App activated: {}", bundle_id);
                state.current_app = Some(bundle_id.clone());

                // If this is a supported app, schedule extraction after debounce
                if is_supported_app(&bundle_id) {
                    state.pending_extractions.insert(bundle_id, Instant::now());
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn app_deactivated(_this: &Object, _sel: Sel, notification: id) {
    let bundle_id = match get_bundle_id_from_notification(notification) {
        Some(id) => id,
        None => return,
    };

    unsafe {
        if let Some(ref state_arc) = DAEMON_STATE {
            if let Ok(mut state) = state_arc.lock() {
                log::debug!("[AX-DAEMON] 🔴 App deactivated: {}", bundle_id);

                // If this was a supported app, extract content now
                if is_supported_app(&bundle_id) {
                    // Remove from pending (we're extracting now)
                    state.pending_extractions.remove(&bundle_id);
                    extract_and_store(&bundle_id, &mut state);
                }

                if state.current_app.as_deref() == Some(&bundle_id) {
                    state.current_app = None;
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn register_observer_class() -> *const Class {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("AXDaemonObserver", superclass).unwrap();

    unsafe {
        decl.add_method(
            sel!(appActivated:),
            app_activated as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(appDeactivated:),
            app_deactivated as extern "C" fn(&Object, Sel, id),
        );
    }

    decl.register()
}

#[cfg(target_os = "macos")]
fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Initialize storage
        let db_path = get_db_path();
        log::info!("[AX-DAEMON] 📂 Database path: {}", db_path.display());

        // Create directory if needed
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let storage = DaemonStorage::open(&db_path)?;
        let state = Arc::new(Mutex::new(DaemonState::with_storage(storage)));
        DAEMON_STATE = Some(state.clone());

        // Register observer class
        let observer_class = register_observer_class();
        let observer: id = msg_send![observer_class, new];

        // Get workspace notification center
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let notification_center: id = msg_send![workspace, notificationCenter];

        // Register for app activation notifications
        let activate_name = NSString::alloc(nil).init_str("NSWorkspaceDidActivateApplicationNotification");
        let deactivate_name = NSString::alloc(nil).init_str("NSWorkspaceDidDeactivateApplicationNotification");

        let _: () = msg_send![notification_center,
            addObserver: observer
            selector: sel!(appActivated:)
            name: activate_name
            object: nil
        ];

        let _: () = msg_send![notification_center,
            addObserver: observer
            selector: sel!(appDeactivated:)
            name: deactivate_name
            object: nil
        ];

        log::info!("[AX-DAEMON] 👀 Monitoring app activations...");
        log::info!("[AX-DAEMON] 📋 Supported apps:");
        for (bundle_id, source) in SUPPORTED_APPS {
            log::info!("[AX-DAEMON]    - {} ({})", source, bundle_id);
        }

        // Run the main loop
        let run_loop: id = msg_send![class!(NSRunLoop), currentRunLoop];
        
        loop {
            // Process run loop for a short interval
            let date: id = msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: 0.5f64];
            let _: () = msg_send![run_loop, runUntilDate: date];

            // Process pending extractions
            if let Ok(mut state) = state.lock() {
                process_pending_extractions(&mut state);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    Err("This daemon only runs on macOS".into())
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Accessibility Extractor Daemon v0.1.0              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Monitors app switches and extracts content automatically   ║");
    println!("║  Press Ctrl+C to stop                                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check accessibility permissions
    if !AccessibilityExtractor::is_enabled() {
        log::error!("[AX-DAEMON] ❌ Accessibility permissions not granted!");
        log::error!("[AX-DAEMON] 💡 Please grant accessibility permissions in:");
        log::error!("[AX-DAEMON]    System Preferences → Security & Privacy → Privacy → Accessibility");
        AccessibilityExtractor::request_permissions();
        std::process::exit(1);
    }

    log::info!("[AX-DAEMON] ✅ Accessibility permissions verified");

    if let Err(e) = run_daemon() {
        log::error!("[AX-DAEMON] ❌ Daemon error: {}", e);
        std::process::exit(1);
    }
}
