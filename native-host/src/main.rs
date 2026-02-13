//! Native Messaging Host - Thin relay to Ingestion Service
//!
//! This binary receives messages from the Chrome extension via stdin/stdout
//! and forwards them to the Tauri ingestion service via Unix socket.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const SOCKET_PATH: &str = "/tmp/clace-ingestion.sock";
const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);

/// Read a native messaging message from stdin
fn read_message() -> io::Result<Option<Vec<u8>>> {
    let mut length_bytes = [0u8; 4];

    match io::stdin().read_exact(&mut length_bytes) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let length = u32::from_ne_bytes(length_bytes) as usize;
    if length == 0 {
        return Ok(None);
    }

    let mut message = vec![0u8; length];
    io::stdin().read_exact(&mut message)?;

    Ok(Some(message))
}

/// Write a native messaging message to stdout
fn write_message(message: &[u8]) -> io::Result<()> {
    let length = message.len() as u32;
    let length_bytes = length.to_ne_bytes();

    let mut stdout = io::stdout().lock();
    stdout.write_all(&length_bytes)?;
    stdout.write_all(message)?;
    stdout.flush()?;

    Ok(())
}

/// Forward message to ingestion service via Unix socket
fn forward_to_service(message: &[u8]) -> io::Result<Vec<u8>> {
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    stream.set_read_timeout(Some(SOCKET_TIMEOUT))?;
    stream.set_write_timeout(Some(SOCKET_TIMEOUT))?;

    // Send message with newline delimiter
    stream.write_all(message)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    // Read response (newline-delimited JSON)
    let mut response = Vec::new();
    let mut buf = [0u8; 1];

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if buf[0] == b'\n' {
                    break;
                }
                response.push(buf[0]);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }

    Ok(response)
}

/// Create an error response JSON
fn error_response(message: &str) -> Vec<u8> {
    format!(
        r#"{{"status":"error","action":"failed","message":"{}"}}"#,
        message.replace('"', "\\\"")
    )
    .into_bytes()
}

fn main() {
    eprintln!("Native host started, connecting to {}", SOCKET_PATH);

    // Main message loop
    loop {
        match read_message() {
            Ok(Some(message)) => {
                eprintln!("Received {} bytes from extension", message.len());

                let response = match forward_to_service(&message) {
                    Ok(resp) => {
                        eprintln!("Service response: {} bytes", resp.len());
                        resp
                    }
                    Err(e) => {
                        eprintln!("Service error: {}", e);
                        error_response(&format!("Service unavailable: {}", e))
                    }
                };

                if let Err(e) = write_message(&response) {
                    eprintln!("Failed to write response: {}", e);
                    break;
                }
            }
            Ok(None) => {
                eprintln!("Connection closed");
                break;
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}
