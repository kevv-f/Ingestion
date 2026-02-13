//! Chrome extension integration via Native Messaging.
//!
//! This module handles communication with the Chrome extension using
//! the Chrome Native Messaging protocol.

use crate::types::{ExtractedContent, ExtractorType};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use tokio::sync::mpsc;
use tracing::{debug, error, trace, warn};

/// Message from Chrome extension
#[derive(Debug, Clone, Deserialize)]
pub struct ChromeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: Option<ChromePayload>,
}

/// Payload from Chrome extension
#[derive(Debug, Clone, Deserialize)]
pub struct ChromePayload {
    pub url: String,
    pub title: String,
    pub content: String,
    pub source: String,
}

/// Response to Chrome extension
#[derive(Debug, Clone, Serialize)]
pub struct ChromeResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub received: bool,
}

/// Chrome extension client for receiving pushed content
pub struct ChromeExtensionClient {
    /// Channel for received content
    content_rx: Option<mpsc::Receiver<ExtractedContent>>,
    /// Sender for content (used by the reader task)
    content_tx: mpsc::Sender<ExtractedContent>,
}

impl ChromeExtensionClient {
    /// Create a new Chrome extension client
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            content_rx: Some(rx),
            content_tx: tx,
        }
    }

    /// Take the content receiver (can only be called once)
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<ExtractedContent>> {
        self.content_rx.take()
    }

    /// Get a sender for pushing content (for the native messaging reader)
    pub fn get_sender(&self) -> mpsc::Sender<ExtractedContent> {
        self.content_tx.clone()
    }

    /// Read a message from stdin (Chrome Native Messaging protocol)
    pub fn read_message() -> std::io::Result<ChromeMessage> {
        let mut len_bytes = [0u8; 4];
        std::io::stdin().read_exact(&mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        if len > 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message too large",
            ));
        }

        let mut buffer = vec![0u8; len];
        std::io::stdin().read_exact(&mut buffer)?;

        let message: ChromeMessage = serde_json::from_slice(&buffer).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        Ok(message)
    }

    /// Write a response to stdout (Chrome Native Messaging protocol)
    pub fn write_response(response: &ChromeResponse) -> std::io::Result<()> {
        let json = serde_json::to_vec(response).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        let len = (json.len() as u32).to_le_bytes();

        std::io::stdout().write_all(&len)?;
        std::io::stdout().write_all(&json)?;
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Process a received message and convert to ExtractedContent
    pub fn process_message(message: ChromeMessage) -> Option<ExtractedContent> {
        if message.msg_type != "content" {
            trace!("Ignoring non-content message: {}", message.msg_type);
            return None;
        }

        let payload = message.payload?;

        Some(ExtractedContent {
            source: payload.source,
            title: Some(payload.title),
            content: payload.content,
            app_name: "Chrome".to_string(),
            bundle_id: Some("com.google.Chrome".to_string()),
            url: Some(payload.url),
            timestamp: chrono::Utc::now().timestamp(),
            extraction_method: "chrome_extension".to_string(),
            confidence: Some(1.0),
        })
    }

    /// Get the extractor type
    pub fn extractor_type(&self) -> ExtractorType {
        ExtractorType::Chrome
    }
}

impl Default for ChromeExtensionClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the native messaging reader loop (blocking, run in separate thread)
pub fn run_native_messaging_loop(sender: mpsc::Sender<ExtractedContent>) {
    debug!("Starting Chrome native messaging loop");

    loop {
        match ChromeExtensionClient::read_message() {
            Ok(message) => {
                trace!("Received Chrome message: {:?}", message.msg_type);

                // Send acknowledgment
                let response = ChromeResponse {
                    msg_type: "status".to_string(),
                    received: true,
                };
                if let Err(e) = ChromeExtensionClient::write_response(&response) {
                    warn!("Failed to send response: {}", e);
                }

                // Process and forward content
                if let Some(content) = ChromeExtensionClient::process_message(message) {
                    if sender.blocking_send(content).is_err() {
                        error!("Content channel closed, exiting native messaging loop");
                        break;
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    debug!("Chrome extension disconnected");
                    break;
                }
                error!("Error reading Chrome message: {}", e);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrome_client_new() {
        let mut client = ChromeExtensionClient::new();
        assert!(client.take_receiver().is_some());
        assert!(client.take_receiver().is_none()); // Can only take once
    }

    #[test]
    fn test_process_message_content() {
        let message = ChromeMessage {
            msg_type: "content".to_string(),
            payload: Some(ChromePayload {
                url: "https://example.com".to_string(),
                title: "Example".to_string(),
                content: "Hello world".to_string(),
                source: "chrome".to_string(),
            }),
        };

        let content = ChromeExtensionClient::process_message(message);
        assert!(content.is_some());

        let content = content.unwrap();
        assert_eq!(content.url, Some("https://example.com".to_string()));
        assert_eq!(content.title, Some("Example".to_string()));
    }

    #[test]
    fn test_process_message_non_content() {
        let message = ChromeMessage {
            msg_type: "ping".to_string(),
            payload: None,
        };

        let content = ChromeExtensionClient::process_message(message);
        assert!(content.is_none());
    }
}
