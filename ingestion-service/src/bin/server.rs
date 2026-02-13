//! Standalone ingestion server binary
//!
//! Run this to start the ingestion service as a standalone process.
//! In production, this would be integrated into the Tauri app.

use ingestion_service::IngestionServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("Starting Clace Ingestion Service...");

    let server = IngestionServer::with_defaults()?;

    println!("Socket: {:?}", server.socket_path());
    println!("Press Ctrl+C to stop");

    // Handle shutdown gracefully
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down...");
        }
    }

    // Clean up socket file
    if server.socket_path().exists() {
        std::fs::remove_file(server.socket_path())?;
    }

    Ok(())
}
