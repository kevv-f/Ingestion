//! Integration tests for the ax-extractor CLI.
//!
//! These tests verify the CLI interface works correctly by running the binary
//! and checking its output and exit codes.
//!
//! # Requirements Tested
//! - Requirement 11.1: CLI binary named "ax-extractor"
//! - Requirement 11.5: Output extracted content as JSON to stdout
//! - Requirement 11.6: Output errors to stderr with descriptive messages

use std::process::Command;

/// Get the path to the ax-extractor binary.
/// In tests, we use the debug build.
fn get_binary_path() -> String {
    // The binary is built in target/debug when running tests
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/ax-extractor", manifest_dir)
}

/// Build the binary before running tests.
/// This ensures the binary exists and is up to date.
fn ensure_binary_built() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "ax-extractor"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build binary");
    
    assert!(status.success(), "Failed to build ax-extractor binary");
}

/// Test that --check-permissions outputs valid JSON with "enabled" field.
/// 
/// **Validates: Requirements 11.1, 11.5**
#[test]
fn test_check_permissions_outputs_valid_json() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("--check-permissions")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse the output as JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    // Verify the JSON contains the "enabled" field
    assert!(
        json.get("enabled").is_some(),
        "JSON output should contain 'enabled' field. Got: {}",
        stdout
    );
    
    // Verify "enabled" is a boolean
    assert!(
        json["enabled"].is_boolean(),
        "'enabled' field should be a boolean. Got: {:?}",
        json["enabled"]
    );
    
    // Verify the JSON contains a "message" field
    assert!(
        json.get("message").is_some(),
        "JSON output should contain 'message' field. Got: {}",
        stdout
    );
}

/// Test that --check-permissions with short flag (-c) also works.
/// 
/// **Validates: Requirements 11.1, 11.5**
#[test]
fn test_check_permissions_short_flag_outputs_valid_json() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("-c")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse the output as JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    // Verify the JSON contains the "enabled" field
    assert!(
        json.get("enabled").is_some(),
        "JSON output should contain 'enabled' field. Got: {}",
        stdout
    );
}

/// Test that --help outputs usage information to stdout.
/// 
/// **Validates: Requirements 11.1**
#[test]
fn test_help_outputs_usage_information() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify help output contains expected content
    assert!(
        stdout.contains("ax-extractor"),
        "Help should mention the binary name. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("--check-permissions"),
        "Help should mention --check-permissions flag. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("--extract"),
        "Help should mention --extract flag. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("--selected"),
        "Help should mention --selected flag. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("--help"),
        "Help should mention --help flag. Got: {}",
        stdout
    );
    
    // Help should exit with code 0
    assert!(
        output.status.success(),
        "Help command should exit with code 0"
    );
}

/// Test that -h short flag also shows help.
/// 
/// **Validates: Requirements 11.1**
#[test]
fn test_help_short_flag() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("-h")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify help output contains expected content
    assert!(
        stdout.contains("ax-extractor"),
        "Help should mention the binary name"
    );
    
    // Help should exit with code 0
    assert!(
        output.status.success(),
        "Help command should exit with code 0"
    );
}

/// Test that unknown arguments output errors to stderr.
/// 
/// **Validates: Requirements 11.6**
#[test]
fn test_unknown_argument_outputs_error_to_stderr() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("--unknown-flag")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Verify error is written to stderr
    assert!(
        !stderr.is_empty(),
        "Unknown argument should produce error output to stderr"
    );
    
    // Verify error message mentions the unknown argument
    assert!(
        stderr.contains("unknown") || stderr.contains("Unknown"),
        "Error message should indicate unknown argument. Got: {}",
        stderr
    );
    
    // Verify exit code is non-zero (error)
    assert!(
        !output.status.success(),
        "Unknown argument should exit with non-zero code"
    );
    
    // Verify exit code is 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "Unknown argument should exit with code 1"
    );
}

/// Test that invalid arguments produce helpful error messages.
/// 
/// **Validates: Requirements 11.6**
#[test]
fn test_invalid_argument_suggests_help() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("--invalid")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Verify error suggests using --help
    assert!(
        stderr.contains("--help") || stderr.contains("help"),
        "Error message should suggest using --help. Got: {}",
        stderr
    );
}

/// Test that no arguments shows help (default behavior).
/// 
/// **Validates: Requirements 11.1**
#[test]
fn test_no_arguments_shows_help() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify help is shown when no arguments provided
    assert!(
        stdout.contains("ax-extractor") || stdout.contains("USAGE"),
        "No arguments should show help. Got: {}",
        stdout
    );
    
    // Should exit with code 0 (help is not an error)
    assert!(
        output.status.success(),
        "No arguments (showing help) should exit with code 0"
    );
}

/// Test that --check-permissions returns correct exit codes.
/// Exit code 0 if permissions granted, 1 if not.
/// 
/// **Validates: Requirements 11.1, 11.5**
#[test]
fn test_check_permissions_exit_code() {
    ensure_binary_built();
    
    let output = Command::new(get_binary_path())
        .arg("--check-permissions")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");
    
    let enabled = json["enabled"].as_bool().unwrap();
    let exit_code = output.status.code().unwrap();
    
    // Exit code should match the enabled status
    if enabled {
        assert_eq!(exit_code, 0, "Should exit with 0 when permissions are granted");
    } else {
        assert_eq!(exit_code, 1, "Should exit with 1 when permissions are not granted");
    }
}

/// Test that stdout and stderr are separate streams.
/// Errors should go to stderr, not stdout.
/// 
/// **Validates: Requirements 11.5, 11.6**
#[test]
fn test_stdout_stderr_separation() {
    ensure_binary_built();
    
    // Test with valid command - should have stdout, no stderr
    let valid_output = Command::new(get_binary_path())
        .arg("--check-permissions")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let valid_stdout = String::from_utf8_lossy(&valid_output.stdout);
    let _valid_stderr = String::from_utf8_lossy(&valid_output.stderr);
    
    assert!(
        !valid_stdout.is_empty(),
        "Valid command should produce stdout"
    );
    // Note: stderr might have debug logs if RUST_LOG is set, so we don't assert it's empty
    
    // Test with invalid command - should have stderr
    let invalid_output = Command::new(get_binary_path())
        .arg("--invalid-flag")
        .output()
        .expect("Failed to execute ax-extractor");
    
    let invalid_stderr = String::from_utf8_lossy(&invalid_output.stderr);
    
    assert!(
        !invalid_stderr.is_empty(),
        "Invalid command should produce stderr"
    );
}
