//! Privacy filtering and PII redaction.
//!
//! This module provides functionality to:
//! - Block extraction from sensitive applications
//! - Redact personally identifiable information (PII) from extracted content

use crate::config::PrivacyConfig;
use lazy_static::lazy_static;
use regex::Regex;
use tracing::debug;

/// Applications that are always blacklisted and cannot be unblocked.
/// These are system apps that should never have content extracted.
/// Note: These are exact matches. For pattern matching, see ALWAYS_BLACKLISTED_PATTERNS.
pub const ALWAYS_BLACKLISTED_APPS: &[&str] = &[
    // Our own viewer app - avoid recursive extraction
    "com.ehl.viewer-app",
    // Tauri dev mode uses different identifiers
    "com.tauri.dev",
    // Kiro IDE - avoid extracting from the IDE itself
    "dev.kiro.app",
    "com.amazon.kiro",
    // DaVinci Resolve - video editing software with sensitive content
    "com.blackmagic-design.DaVinciResolve",
    "com.blackmagic-design.DaVinciResolveLite",
];

/// Patterns for always-blacklisted apps (supports glob wildcards).
/// These apps cannot be unblocked.
pub const ALWAYS_BLACKLISTED_PATTERNS: &[&str] = &[
    // Viewer app variations
    "*viewer-app*",
    "*viewer_app*",
    // Kiro IDE variations
    "*kiro*",
    // DaVinci Resolve variations
    "*DaVinciResolve*",
    "*davinciresolve*",
];

lazy_static! {
    // Credit card: 13-19 digits, optionally with spaces/dashes
    static ref CREDIT_CARD: Regex = Regex::new(
        r"\b(?:\d{4}[-\s]?){3,4}\d{1,4}\b"
    ).unwrap();

    // SSN: XXX-XX-XXXX format
    static ref SSN: Regex = Regex::new(
        r"\b\d{3}-\d{2}-\d{4}\b"
    ).unwrap();

    // API keys: common patterns
    static ref API_KEY: Regex = Regex::new(
        r#"(?i)(api[_-]?key|apikey|secret[_-]?key|access[_-]?token|auth[_-]?token)['"]?\s*[:=]\s*['"]?([a-zA-Z0-9_\-]{20,})"#
    ).unwrap();

    // AWS keys
    static ref AWS_KEY: Regex = Regex::new(
        r"(?i)(AKIA[0-9A-Z]{16})"
    ).unwrap();

    // Email addresses
    static ref EMAIL: Regex = Regex::new(
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"
    ).unwrap();

    // Phone numbers (various formats)
    static ref PHONE: Regex = Regex::new(
        r"\b(?:\+1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b"
    ).unwrap();

    // Password fields in common formats
    static ref PASSWORD_FIELD: Regex = Regex::new(
        r#"(?i)(password|passwd|pwd)['"]?\s*[:=]\s*['"]?([^\s'"]{4,})"#
    ).unwrap();
}

/// Privacy filter for content redaction and app blocking
pub struct PrivacyFilter {
    config: PrivacyConfig,
    /// Compiled patterns for blocked apps
    blocked_patterns: Vec<glob::Pattern>,
}

impl PrivacyFilter {
    /// Create a new privacy filter with the given configuration
    pub fn new(config: PrivacyConfig) -> Self {
        let blocked_patterns = config
            .blocked_apps
            .iter()
            .filter_map(|pattern| {
                glob::Pattern::new(pattern)
                    .map_err(|e| {
                        tracing::warn!("Invalid blocked app pattern '{}': {}", pattern, e);
                        e
                    })
                    .ok()
            })
            .collect();

        Self {
            config,
            blocked_patterns,
        }
    }

    /// Check if an application should be blocked from extraction.
    /// This checks both the static always-blacklisted apps and the configurable blocklist.
    pub fn is_blocked(&self, bundle_id: &str) -> bool {
        // First check the static always-blacklisted apps
        if Self::is_always_blacklisted(bundle_id) {
            debug!("App '{}' is always blacklisted", bundle_id);
            return true;
        }

        // Then check the configurable blocklist
        for pattern in &self.blocked_patterns {
            if pattern.matches(bundle_id) {
                debug!("App '{}' blocked by pattern '{}'", bundle_id, pattern);
                return true;
            }
        }
        false
    }

    /// Check if an application is in the static always-blacklisted list.
    /// These apps cannot be unblocked.
    /// Checks both exact matches and glob patterns.
    pub fn is_always_blacklisted(bundle_id: &str) -> bool {
        // Check exact matches first
        if ALWAYS_BLACKLISTED_APPS.iter().any(|&blocked| blocked == bundle_id) {
            return true;
        }
        
        // Check patterns
        for pattern_str in ALWAYS_BLACKLISTED_PATTERNS {
            if let Ok(pattern) = glob::Pattern::new(pattern_str) {
                if pattern.matches(bundle_id) {
                    return true;
                }
            }
        }
        
        // Also check app name patterns (for cases where bundle_id might be the app name)
        let bundle_lower = bundle_id.to_lowercase();
        if bundle_lower.contains("viewer-app") || bundle_lower.contains("viewer_app") 
            || bundle_lower.contains("davinciresolve") {
            return true;
        }
        
        false
    }

    /// Redact PII from content based on configuration
    pub fn redact(&self, content: &str) -> String {
        let mut result = content.to_string();

        if self.config.redact_credit_cards {
            result = self.redact_credit_cards(&result);
        }

        if self.config.redact_ssn {
            result = SSN.replace_all(&result, "[REDACTED_SSN]").to_string();
        }

        if self.config.redact_api_keys {
            result = self.redact_api_keys(&result);
        }

        if self.config.redact_emails {
            result = EMAIL.replace_all(&result, "[REDACTED_EMAIL]").to_string();
        }

        if self.config.redact_phone_numbers {
            result = PHONE.replace_all(&result, "[REDACTED_PHONE]").to_string();
        }

        result
    }

    /// Redact credit card numbers with Luhn validation
    fn redact_credit_cards(&self, content: &str) -> String {
        CREDIT_CARD
            .replace_all(content, |caps: &regex::Captures| {
                let matched = &caps[0];
                // Extract digits only
                let digits: String = matched.chars().filter(|c| c.is_ascii_digit()).collect();

                // Validate with Luhn algorithm to reduce false positives
                if is_valid_luhn(&digits) {
                    "[REDACTED_CARD]".to_string()
                } else {
                    matched.to_string()
                }
            })
            .to_string()
    }

    /// Redact API keys and tokens
    fn redact_api_keys(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Generic API key patterns
        result = API_KEY
            .replace_all(&result, "$1=[REDACTED_KEY]")
            .to_string();

        // AWS access keys
        result = AWS_KEY
            .replace_all(&result, "[REDACTED_AWS_KEY]")
            .to_string();

        // Password fields
        result = PASSWORD_FIELD
            .replace_all(&result, "$1=[REDACTED_PASSWORD]")
            .to_string();

        result
    }

    /// Add an app to the blocklist at runtime
    pub fn block_app(&mut self, bundle_id: &str) {
        if let Ok(pattern) = glob::Pattern::new(bundle_id) {
            self.blocked_patterns.push(pattern);
            self.config.blocked_apps.push(bundle_id.to_string());
            debug!("Added '{}' to blocklist", bundle_id);
        }
    }

    /// Remove an app from the blocklist.
    /// Note: Always-blacklisted apps cannot be unblocked.
    pub fn unblock_app(&mut self, bundle_id: &str) {
        // Prevent unblocking always-blacklisted apps
        if Self::is_always_blacklisted(bundle_id) {
            debug!("Cannot unblock '{}' - it is always blacklisted", bundle_id);
            return;
        }

        self.config.blocked_apps.retain(|s| s != bundle_id);
        self.blocked_patterns
            .retain(|p| p.as_str() != bundle_id);
        debug!("Removed '{}' from blocklist", bundle_id);
    }

    /// Get the current blocklist
    pub fn blocked_apps(&self) -> &[String] {
        &self.config.blocked_apps
    }
}

impl Default for PrivacyFilter {
    fn default() -> Self {
        Self::new(PrivacyConfig::default())
    }
}

/// Luhn algorithm for credit card validation
fn is_valid_luhn(number: &str) -> bool {
    let digits: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();

    // Credit cards are typically 13-19 digits
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                d
            }
        })
        .sum();

    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luhn_valid() {
        // Test Visa card number (passes Luhn)
        assert!(is_valid_luhn("4532015112830366"));
        // Test Mastercard
        assert!(is_valid_luhn("5425233430109903"));
    }

    #[test]
    fn test_luhn_invalid() {
        assert!(!is_valid_luhn("1234567890123456"));
        assert!(!is_valid_luhn("0000000000000000"));
    }

    #[test]
    fn test_is_blocked() {
        let filter = PrivacyFilter::default();

        assert!(filter.is_blocked("com.1password.app"));
        assert!(filter.is_blocked("com.agilebits.onepassword7"));
        assert!(filter.is_blocked("com.example.banking"));
        assert!(!filter.is_blocked("com.microsoft.Word"));
    }

    #[test]
    fn test_always_blacklisted_apps() {
        // Test that always-blacklisted apps are blocked (exact matches)
        assert!(PrivacyFilter::is_always_blacklisted("com.ehl.viewer-app"));
        assert!(PrivacyFilter::is_always_blacklisted("com.blackmagic-design.DaVinciResolve"));
        assert!(PrivacyFilter::is_always_blacklisted("com.blackmagic-design.DaVinciResolveLite"));
        assert!(PrivacyFilter::is_always_blacklisted("dev.kiro.app"));
        assert!(PrivacyFilter::is_always_blacklisted("com.amazon.kiro"));
        
        // Test pattern matches
        assert!(PrivacyFilter::is_always_blacklisted("com.example.viewer-app.test"));
        assert!(PrivacyFilter::is_always_blacklisted("viewer-app"));
        assert!(PrivacyFilter::is_always_blacklisted("Viewer-App"));
        assert!(PrivacyFilter::is_always_blacklisted("com.example.kiro.test"));
        
        // Test that other apps are not always-blacklisted
        assert!(!PrivacyFilter::is_always_blacklisted("com.microsoft.Word"));
        assert!(!PrivacyFilter::is_always_blacklisted("com.apple.Safari"));
    }

    #[test]
    fn test_is_blocked_includes_always_blacklisted() {
        let filter = PrivacyFilter::default();
        
        // Always-blacklisted apps should be blocked
        assert!(filter.is_blocked("com.ehl.viewer-app"));
        assert!(filter.is_blocked("com.blackmagic-design.DaVinciResolve"));
        assert!(filter.is_blocked("dev.kiro.app"));
    }

    #[test]
    fn test_cannot_unblock_always_blacklisted() {
        let mut filter = PrivacyFilter::default();
        
        // Try to unblock an always-blacklisted app
        filter.unblock_app("com.ehl.viewer-app");
        
        // It should still be blocked
        assert!(filter.is_blocked("com.ehl.viewer-app"));
    }

    #[test]
    fn test_redact_ssn() {
        let config = PrivacyConfig {
            redact_ssn: true,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(config);

        let input = "My SSN is 123-45-6789 and yours is 987-65-4321";
        let output = filter.redact(input);

        assert!(output.contains("[REDACTED_SSN]"));
        assert!(!output.contains("123-45-6789"));
        assert!(!output.contains("987-65-4321"));
    }

    #[test]
    fn test_redact_credit_card() {
        let config = PrivacyConfig {
            redact_credit_cards: true,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(config);

        // Valid Visa number
        let input = "Card: 4532015112830366";
        let output = filter.redact(input);

        assert!(output.contains("[REDACTED_CARD]"));
        assert!(!output.contains("4532015112830366"));
    }

    #[test]
    fn test_redact_api_key() {
        let config = PrivacyConfig {
            redact_api_keys: true,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(config);

        let input = "api_key=sk_test_FAKE_KEY_FOR_TESTING_1234";
        let output = filter.redact(input);

        assert!(output.contains("[REDACTED_KEY]"));
        assert!(!output.contains("sk_test_FAKE_KEY_FOR_TESTING_1234"));
    }

    #[test]
    fn test_redact_email() {
        let config = PrivacyConfig {
            redact_emails: true,
            ..Default::default()
        };
        let filter = PrivacyFilter::new(config);

        let input = "Contact: user@example.com";
        let output = filter.redact(input);

        assert!(output.contains("[REDACTED_EMAIL]"));
        assert!(!output.contains("user@example.com"));
    }

    #[test]
    fn test_block_unblock_app() {
        let mut filter = PrivacyFilter::default();

        assert!(!filter.is_blocked("com.custom.app"));

        filter.block_app("com.custom.app");
        assert!(filter.is_blocked("com.custom.app"));

        filter.unblock_app("com.custom.app");
        assert!(!filter.is_blocked("com.custom.app"));
    }
}
