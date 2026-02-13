//! Unified Router - Main entry point
//!
//! This binary runs the unified content extraction router as a daemon.

use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use unified_router::{Config, UnifiedRouter, CapturePayload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Unified Router");

    // Load configuration
    let config = Config::load();
    info!("Configuration loaded from {:?}", Config::default_config_path());

    if !config.general.enabled {
        info!("Router is disabled in configuration, exiting");
        return Ok(());
    }

    // Create content channel
    let (content_tx, mut content_rx) = mpsc::channel::<CapturePayload>(100);

    // Create router
    let mut router = UnifiedRouter::new(config.clone(), content_tx);
    router.init();

    // Take Chrome receiver for handling extension messages
    let chrome_rx = router.take_chrome_receiver();

    // Spawn content handler task
    let _content_handle = tokio::spawn(async move {
        while let Some(payload) = content_rx.recv().await {
            // In production, send to ingestion service
            info!(
                "Extracted content from {}: {} chars",
                payload.source,
                payload.content.len()
            );

            // TODO: Send to ingestion service
            // ingestion_client.send(payload).await;
        }
    });

    // Spawn Chrome extension handler if available
    if let Some(mut chrome_rx) = chrome_rx {
        tokio::spawn(async move {
            while let Some(content) = chrome_rx.recv().await {
                info!(
                    "Received Chrome content: {}",
                    content.title.as_deref().unwrap_or("untitled")
                );
                // Content is already sent through the main channel
            }
        });
    }

    // Calculate tick interval based on config
    let interval = Duration::from_secs(config.timing.base_interval_seconds);

    info!("Router running with {}s interval", interval.as_secs());

    // Main loop
    let mut tick_interval = tokio::time::interval(interval);

    loop {
        tick_interval.tick().await;

        if let Err(e) = router.tick().await {
            error!("Router tick error: {}", e);
        }
    }
}
