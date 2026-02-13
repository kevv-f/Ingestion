//! Unified Ingestion Service - Single command to run all extraction services
//!
//! This binary orchestrates all content extraction components:
//! - Ingestion Server (SQLite storage, dedup, chunking)
//! - Unified Router (window tracking, change detection, extractor routing)
//! - Native Host (Chrome extension relay) - optional, spawned on demand
//!
//! # Usage
//!
//! ```bash
//! # Start all services with defaults
//! ingestion
//!
//! # Start with custom config
//! ingestion --config /path/to/config.toml
//!
//! # Start specific components only
//! ingestion --no-ocr          # Disable OCR extraction
//! ingestion --no-accessibility # Disable accessibility extraction
//! ```

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Configuration for the unified ingestion service
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Enable accessibility extraction
    pub accessibility_enabled: bool,
    /// Enable OCR extraction
    pub ocr_enabled: bool,
    /// Enable Chrome extension support
    pub chrome_enabled: bool,
    /// Path to config file
    pub config_path: Option<PathBuf>,
    /// Socket path for ingestion service
    pub socket_path: PathBuf,
    /// Database path
    pub db_path: PathBuf,
    /// Base capture interval in seconds
    pub capture_interval: u64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clace-ingestion");

        Self {
            accessibility_enabled: true,
            ocr_enabled: true, // Enabled by default - uses per-image mode for unsupported apps
            chrome_enabled: true,
            config_path: None,
            socket_path: PathBuf::from("/tmp/clace-ingestion.sock"),
            db_path: data_dir.join("content.db"),
            capture_interval: 5,
        }
    }
}

/// Parse command line arguments
fn parse_args() -> IngestionConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut config = IngestionConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-v" => {
                println!("Unified Ingestion Service v0.1.0");
                std::process::exit(0);
            }
            "--no-accessibility" => {
                config.accessibility_enabled = false;
            }
            "--no-ocr" => {
                config.ocr_enabled = false;
            }
            "--no-chrome" => {
                config.chrome_enabled = false;
            }
            "--config" | "-c" => {
                i += 1;
                if i < args.len() {
                    config.config_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--socket" => {
                i += 1;
                if i < args.len() {
                    config.socket_path = PathBuf::from(&args[i]);
                }
            }
            "--db" => {
                i += 1;
                if i < args.len() {
                    config.db_path = PathBuf::from(&args[i]);
                }
            }
            "--interval" => {
                i += 1;
                if i < args.len() {
                    if let Ok(interval) = args[i].parse() {
                        config.capture_interval = interval;
                    }
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information.");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    config
}

fn print_help() {
    println!(
        r#"Unified Ingestion Service - Content extraction orchestrator

USAGE:
    ingestion [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -v, --version           Show version
    -c, --config <PATH>     Path to configuration file
    --socket <PATH>         Unix socket path (default: /tmp/clace-ingestion.sock)
    --db <PATH>             Database path (default: ~/Library/Application Support/clace-ingestion/content.db)
    --interval <SECS>       Base capture interval in seconds (default: 5)
    --no-accessibility      Disable accessibility extraction
    --no-ocr                Disable OCR extraction
    --no-chrome             Disable Chrome extension support

COMPONENTS:
    This service orchestrates:
    - Ingestion Server: SQLite storage with dedup and chunking
    - Unified Router: Window tracking and extractor routing
    - Accessibility Extractor: Content from Office, iWork, Slack, Teams
    - OCR Extractor: Screen capture and text recognition for unsupported apps
    - Chrome Extension: Web page content via native messaging

EXTRACTION METHODS:
    - Accessibility: Used for apps with good accessibility support (Word, Excel, Slack, etc.)
    - Chrome Extension: Web browsers push content via native messaging
    - OCR: Fallback for apps without accessibility support (uses Vision framework)

PERMISSIONS REQUIRED:
    - Accessibility: System Settings > Privacy & Security > Accessibility
    - Screen Recording: System Settings > Privacy & Security > Screen Recording

EXAMPLES:
    ingestion                           # Start all extractors
    ingestion --no-ocr                  # Disable OCR fallback
    ingestion --no-accessibility        # Only Chrome extension + OCR
    ingestion --interval 10             # 10 second capture interval
    ingestion --config ~/my-config.toml # Custom configuration
"#
    );
}

/// Check if required permissions are granted
fn check_permissions() -> (bool, bool) {
    // Check accessibility permission
    let ax_enabled = {
        #[cfg(target_os = "macos")]
        {
            extern "C" {
                fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            }

            unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    };

    // Check screen recording permission (we can't directly check, but we can try to capture)
    let screen_enabled = {
        #[cfg(target_os = "macos")]
        {
            // Try to get display list - if we can, we likely have permission
            use core_graphics::display::CGGetActiveDisplayList;
            let mut count: u32 = 0;
            unsafe {
                CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count);
            }
            count > 0
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    };

    (ax_enabled, screen_enabled)
}

/// Managed child process (for spawning external binaries)
#[allow(dead_code)]
struct ManagedProcess {
    name: String,
    child: Option<Child>,
}

impl ManagedProcess {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            child: None,
        }
    }

    fn start(&mut self, command: &str, args: &[&str]) -> std::io::Result<()> {
        info!("Starting {}: {} {:?}", self.name, command, args);

        let child = Command::new(command)
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        self.child = Some(child);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            info!("Stopping {}...", self.name);
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }

    fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Find the ingestion server binary
fn find_ingestion_server_binary() -> Option<PathBuf> {
    // Get the directory of the current executable
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let paths = [
        // Same directory as the running binary
        exe_dir.join("ingestion-server"),
        // Relative to workspace root (Cargo build output)
        PathBuf::from("ingestion-service/target/release/ingestion-server"),
        PathBuf::from("ingestion-service/target/debug/ingestion-server"),
        PathBuf::from("../ingestion-service/target/release/ingestion-server"),
        PathBuf::from("../ingestion-service/target/debug/ingestion-server"),
        // System paths
        PathBuf::from("/usr/local/bin/ingestion-server"),
    ];

    for path in paths {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Send content to the ingestion server via Unix socket
async fn send_to_ingestion_server(
    socket_path: &PathBuf,
    payload: &unified_router::CapturePayload,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    // Connect to the socket
    let stream = UnixStream::connect(socket_path).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Serialize payload to JSON (one line)
    let json = serde_json::to_string(payload)?;

    // Write JSON payload followed by newline
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response (one line)
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;

    if response["status"] == "ok" || response["status"] == "created" || response["status"] == "updated" || response["status"] == "skipped" {
        Ok(())
    } else {
        Err(format!("Ingestion server error: {:?}", response).into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Parse arguments
    let config = parse_args();

    // Print banner
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Unified Ingestion Service v0.1.0                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Content extraction from desktop apps and web browsers       ║");
    println!("║  Press Ctrl+C to stop all services                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check permissions
    let (ax_enabled, screen_enabled) = check_permissions();

    println!("📋 Permission Status:");
    println!(
        "   Accessibility: {}",
        if ax_enabled { "✅ Granted" } else { "❌ Not granted" }
    );
    println!(
        "   Screen Recording: {}",
        if screen_enabled {
            "✅ Granted"
        } else {
            "⚠️  May not be granted"
        }
    );
    println!();

    if !ax_enabled && config.accessibility_enabled {
        warn!("Accessibility permission not granted. Accessibility extraction will be disabled.");
        warn!("Grant permission in: System Settings > Privacy & Security > Accessibility");
    }

    if !screen_enabled && config.ocr_enabled {
        warn!("Screen recording permission may not be granted. OCR extraction may fail.");
        warn!("Grant permission in: System Settings > Privacy & Security > Screen Recording");
    }

    println!("🔧 Configuration:");
    println!("   Socket: {}", config.socket_path.display());
    println!("   Database: {}", config.db_path.display());
    println!("   Capture interval: {}s", config.capture_interval);
    println!(
        "   Accessibility: {}",
        if config.accessibility_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "   OCR: {}",
        if config.ocr_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "   Chrome extension: {}",
        if config.chrome_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    // Ensure data directory exists
    if let Some(parent) = config.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Setup shutdown signal
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\n🛑 Shutting down...");
        r.store(false, Ordering::SeqCst);
    })?;

    // Create content channel
    let (content_tx, mut content_rx) = mpsc::channel::<unified_router::CapturePayload>(100);

    // Start ingestion server as background process
    let mut ingestion_server_process: Option<ManagedProcess> = None;
    if let Some(server_binary) = find_ingestion_server_binary() {
        let mut process = ManagedProcess::new("Ingestion Server");
        
        if let Err(e) = process.start(
            server_binary.to_str().unwrap_or("ingestion-server"),
            &[],  // Server uses defaults, no args needed
        ) {
            warn!("Failed to start ingestion server: {}", e);
            warn!("Chrome extension content will not be stored. Run: cargo build --release -p ingestion-service");
        } else {
            info!("✅ Ingestion Server started (socket: {})", config.socket_path.display());
            ingestion_server_process = Some(process);
            // Give the server time to start and create the socket
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    } else {
        warn!("Ingestion server binary not found. Chrome extension content will not be stored.");
        warn!("Run: cargo build --release -p ingestion-service");
    }

    // Load router config
    let mut router_config = unified_router::Config::load();
    router_config.timing.base_interval_seconds = config.capture_interval;
    router_config.extractors.accessibility_enabled = config.accessibility_enabled && ax_enabled;
    router_config.extractors.ocr_enabled = config.ocr_enabled;
    router_config.extractors.chrome_extension_enabled = config.chrome_enabled;

    // Create and initialize router
    let mut router = unified_router::UnifiedRouter::new(router_config, content_tx);
    router.init();

    info!("✅ Unified Router initialized");

    // Perform initial extraction for all windows
    router.initial_extraction().await;

    // Spawn content handler that sends to ingestion server
    let content_running = running.clone();
    let socket_path = config.socket_path.clone();
    let _content_handle = tokio::spawn(async move {
        while content_running.load(Ordering::SeqCst) {
            tokio::select! {
                Some(payload) = content_rx.recv() => {
                    info!(
                        "📥 Received content: {} - {} ({} chars)",
                        payload.source,
                        payload.title.as_deref().unwrap_or("untitled"),
                        payload.content.len()
                    );

                    // Send to ingestion service via Unix socket
                    match send_to_ingestion_server(&socket_path, &payload).await {
                        Ok(_) => {
                            info!("✅ Stored: {} - {}", payload.source, payload.title.as_deref().unwrap_or("untitled"));
                        }
                        Err(e) => {
                            error!("Failed to store content: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }
    });

    // Main loop
    let interval = Duration::from_secs(config.capture_interval);
    let mut tick_interval = tokio::time::interval(interval);

    info!("🎯 Starting extraction loop ({}s interval)...", config.capture_interval);
    println!();

    while running.load(Ordering::SeqCst) {
        tick_interval.tick().await;

        if let Err(e) = router.tick().await {
            error!("Router tick error: {}", e);
        }
    }

    // Clean up ingestion server process
    if let Some(mut process) = ingestion_server_process {
        info!("Stopping Ingestion Server...");
        process.stop();
    }

    info!("👋 Shutdown complete");
    Ok(())
}
