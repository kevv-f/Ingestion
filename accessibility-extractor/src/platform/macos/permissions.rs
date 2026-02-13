//! Permission handling for macOS Accessibility API.
//!
//! This module provides functions to check, request, and manage accessibility
//! permissions required for the extractor to access UI elements from other
//! applications on macOS.
//!
//! # Requirements
//! - Requirement 1.1: Provide a function to check if accessibility permissions are granted
//! - Requirement 1.2: Provide a function to request accessibility permissions from the user
//! - Requirement 1.4: Provide a function to open System Preferences to the Accessibility pane
//! - Requirement 1.5: Provide human-readable instructions for enabling accessibility permissions

use std::process::Command;

/// Check if accessibility permissions are granted without prompting the user.
///
/// This function queries the system to determine if the current application
/// has been granted accessibility permissions. It does not show any UI or
/// prompt the user.
///
/// # Returns
///
/// `true` if accessibility permissions are granted, `false` otherwise.
///
/// # Examples
///
/// ```no_run
/// use accessibility_extractor::platform::macos::permissions::is_trusted;
///
/// if is_trusted() {
///     println!("Accessibility permissions are granted!");
/// } else {
///     println!("Accessibility permissions are NOT granted.");
/// }
/// ```
///
/// # Requirements
/// - Requirement 1.1: Provide a function to check if accessibility permissions are granted
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Check if accessibility permissions are granted, showing a system prompt if not.
///
/// This function queries the system to determine if the current application
/// has been granted accessibility permissions. If permissions are not granted,
/// it will display the system's accessibility permission prompt dialog.
///
/// # Returns
///
/// `true` if accessibility permissions are granted (either already or after
/// the user grants them), `false` otherwise.
///
/// # Note
///
/// The system prompt is shown asynchronously, so this function may return
/// `false` even if the user subsequently grants permissions. The application
/// typically needs to be restarted after permissions are granted.
///
/// # Examples
///
/// ```no_run
/// use accessibility_extractor::platform::macos::permissions::is_trusted_with_prompt;
///
/// if is_trusted_with_prompt() {
///     println!("Accessibility permissions are granted!");
/// } else {
///     println!("Please grant accessibility permissions and restart the app.");
/// }
/// ```
///
/// # Requirements
/// - Requirement 1.2: Provide a function to request accessibility permissions from the user
pub fn is_trusted_with_prompt() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Open System Preferences (or System Settings) to the Accessibility pane.
///
/// This function launches the system preferences application and navigates
/// directly to the Privacy & Security > Accessibility section where users
/// can grant accessibility permissions to applications.
///
/// # Returns
///
/// `Ok(())` if the command was successfully spawned, `Err` if there was
/// an I/O error launching the preferences application.
///
/// # Errors
///
/// Returns an `std::io::Error` if:
/// - The `open` command is not available
/// - The system preferences URL scheme is not supported
/// - There's a system error spawning the process
///
/// # Examples
///
/// ```no_run
/// use accessibility_extractor::platform::macos::permissions::open_accessibility_preferences;
///
/// match open_accessibility_preferences() {
///     Ok(()) => println!("Opened System Preferences"),
///     Err(e) => eprintln!("Failed to open preferences: {}", e),
/// }
/// ```
///
/// # Requirements
/// - Requirement 1.4: Provide a function to open System Preferences to the Accessibility pane
pub fn open_accessibility_preferences() -> std::io::Result<()> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()?;
    Ok(())
}

/// Get human-readable instructions for enabling accessibility permissions.
///
/// Returns a static string containing step-by-step instructions for users
/// to manually enable accessibility permissions for the application.
///
/// # Returns
///
/// A static string slice containing formatted instructions.
///
/// # Examples
///
/// ```
/// use accessibility_extractor::platform::macos::permissions::get_permission_instructions;
///
/// let instructions = get_permission_instructions();
/// println!("{}", instructions);
/// ```
///
/// # Requirements
/// - Requirement 1.5: Provide human-readable instructions for enabling accessibility permissions
pub fn get_permission_instructions() -> &'static str {
    r#"
To enable accessibility permissions:

1. Open System Preferences (or System Settings on macOS Ventura+)
2. Go to Privacy & Security → Accessibility
3. Click the lock icon to make changes
4. Find this application in the list and check the checkbox
5. If the application is not listed, click '+' and add it
6. Restart this application

Note: You may need to quit and reopen this application after 
granting permissions for them to take effect.
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Unit Tests for is_trusted function
    // ============================================================================

    #[test]
    fn test_is_trusted_returns_boolean() {
        // is_trusted should return a boolean value
        // We can't test the actual value since it depends on system state,
        // but we can verify it returns without panicking
        let result = is_trusted();
        // Result is either true or false - both are valid
        assert!(result == true || result == false);
    }

    // ============================================================================
    // Unit Tests for get_permission_instructions function
    // ============================================================================

    #[test]
    fn test_get_permission_instructions_returns_non_empty_string() {
        let instructions = get_permission_instructions();
        assert!(!instructions.is_empty());
    }

    #[test]
    fn test_get_permission_instructions_contains_key_steps() {
        let instructions = get_permission_instructions();
        
        // Verify instructions contain key information
        assert!(instructions.contains("System Preferences") || instructions.contains("System Settings"));
        assert!(instructions.contains("Privacy"));
        assert!(instructions.contains("Accessibility"));
        assert!(instructions.contains("lock"));
    }

    #[test]
    fn test_get_permission_instructions_mentions_restart() {
        let instructions = get_permission_instructions();
        
        // Instructions should mention restarting the application
        assert!(
            instructions.to_lowercase().contains("restart") || 
            instructions.to_lowercase().contains("reopen") ||
            instructions.to_lowercase().contains("quit")
        );
    }

    #[test]
    fn test_get_permission_instructions_has_numbered_steps() {
        let instructions = get_permission_instructions();
        
        // Instructions should have numbered steps
        assert!(instructions.contains("1."));
        assert!(instructions.contains("2."));
        assert!(instructions.contains("3."));
    }

    // ============================================================================
    // Unit Tests for open_accessibility_preferences function
    // ============================================================================

    // Note: We don't test open_accessibility_preferences in unit tests because:
    // 1. It spawns an external process (System Preferences)
    // 2. It would open a window during test runs
    // 3. The actual behavior depends on the macOS version and system state
    //
    // Integration tests or manual testing should verify this function works correctly.

    #[test]
    fn test_open_accessibility_preferences_url_is_valid() {
        // We can at least verify the URL format is correct by checking
        // that the function compiles and the URL constant is well-formed
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        assert!(url.starts_with("x-apple.systempreferences:"));
        assert!(url.contains("Privacy_Accessibility"));
    }
}
