//! Ingestion Service Library
//!
//! Provides content ingestion with deduplication and chunking.
//! Designed to be embedded in a Tauri application.

pub mod chunker;
pub mod dedup;
pub mod payload;
pub mod server;
pub mod storage;

pub use payload::CapturePayload;
pub use server::IngestionServer;
pub use storage::Storage;
