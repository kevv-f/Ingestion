//! Configuration management for the unified router.
//!
//! Loads configuration from TOML files and provides runtime defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub timing: TimingConfig,

    #[serde(default)]
    pub change_detection: ChangeDetectionConfig,

    #[serde(default)]
    pub extractors: ExtractorsConfig,

    #[serde(default)]
    pub privacy: PrivacyConfig,

    #[serde(default)]
    pub multi_display: MultiDisplayConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            timing: TimingConfig::default(),
            change_detection: ChangeDetectionConfig::default(),
            extractors: ExtractorsConfig::default(),
            privacy: PrivacyConfig::default(),
            multi_display: MultiDisplayConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Whether the router is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    /// Base capture interval in seconds (AC power)
    #[serde(default = "default_base_interval")]
    pub base_interval_seconds: u64,

    /// Capture interval on battery power
    #[serde(default = "default_battery_interval")]
    pub battery_interval_seconds: u64,

    /// Capture interval when user is idle
    #[serde(default = "default_idle_interval")]
    pub idle_interval_seconds: u64,

    /// Minimum time between extractions for same window
    #[serde(default = "default_min_interval")]
    pub min_interval_seconds: u64,

    /// Maximum time between extractions (forced)
    #[serde(default = "default_max_interval")]
    pub max_interval_seconds: u64,

    /// Seconds of inactivity before considered idle
    #[serde(default = "default_idle_threshold")]
    pub idle_threshold_seconds: u64,

    /// Debounce time after scroll stops
    #[serde(default = "default_scroll_debounce")]
    pub scroll_debounce_ms: u64,

    /// Debounce time after focus change
    #[serde(default = "default_focus_debounce")]
    pub focus_debounce_ms: u64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            base_interval_seconds: 5,
            battery_interval_seconds: 15,
            idle_interval_seconds: 60,
            min_interval_seconds: 3,
            max_interval_seconds: 60,
            idle_threshold_seconds: 30,
            scroll_debounce_ms: 1000,
            focus_debounce_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectionConfig {
    /// Hamming distance threshold for hash comparison (0-64)
    #[serde(default = "default_hash_sensitivity")]
    pub hash_sensitivity: u32,

    /// Whether title changes trigger extraction
    #[serde(default = "default_true")]
    pub title_change_triggers_extract: bool,

    /// Hash algorithm to use (mean, gradient, dct)
    #[serde(default = "default_hash_algorithm")]
    pub hash_algorithm: String,
}

impl Default for ChangeDetectionConfig {
    fn default() -> Self {
        Self {
            hash_sensitivity: 8,
            title_change_triggers_extract: true,
            hash_algorithm: "mean".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorsConfig {
    /// Enable accessibility extractor
    #[serde(default = "default_true")]
    pub accessibility_enabled: bool,

    /// Enable Chrome extension integration
    #[serde(default = "default_true")]
    pub chrome_extension_enabled: bool,

    /// Enable OCR extractor
    #[serde(default = "default_true")]
    pub ocr_enabled: bool,

    /// Only use OCR as fallback (not primary)
    #[serde(default = "default_true")]
    pub ocr_fallback_only: bool,

    /// Path to accessibility extractor binary
    #[serde(default)]
    pub accessibility_binary_path: Option<String>,

    /// Path to OCR extractor binary
    #[serde(default)]
    pub ocr_binary_path: Option<String>,
}

impl Default for ExtractorsConfig {
    fn default() -> Self {
        Self {
            accessibility_enabled: true,
            chrome_extension_enabled: true,
            ocr_enabled: true,
            ocr_fallback_only: true,
            accessibility_binary_path: None,
            ocr_binary_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Bundle IDs to block (supports wildcards)
    #[serde(default = "default_blocked_apps")]
    pub blocked_apps: Vec<String>,

    /// Redact credit card numbers
    #[serde(default = "default_true")]
    pub redact_credit_cards: bool,

    /// Redact social security numbers
    #[serde(default = "default_true")]
    pub redact_ssn: bool,

    /// Redact API keys and tokens
    #[serde(default = "default_true")]
    pub redact_api_keys: bool,

    /// Redact email addresses
    #[serde(default)]
    pub redact_emails: bool,

    /// Redact phone numbers
    #[serde(default)]
    pub redact_phone_numbers: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            blocked_apps: default_blocked_apps(),
            redact_credit_cards: true,
            redact_ssn: true,
            redact_api_keys: true,
            redact_emails: false,
            redact_phone_numbers: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDisplayConfig {
    /// Enable multi-display support
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Track windows on all displays
    #[serde(default = "default_true")]
    pub capture_all_displays: bool,
}

impl Default for MultiDisplayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            capture_all_displays: true,
        }
    }
}

// Default value functions for serde
fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_base_interval() -> u64 {
    5
}

fn default_battery_interval() -> u64 {
    15
}

fn default_idle_interval() -> u64 {
    60
}

fn default_min_interval() -> u64 {
    3
}

fn default_max_interval() -> u64 {
    60
}

fn default_idle_threshold() -> u64 {
    30
}

fn default_scroll_debounce() -> u64 {
    1000
}

fn default_focus_debounce() -> u64 {
    500
}

fn default_hash_sensitivity() -> u32 {
    8
}

fn default_hash_algorithm() -> String {
    "mean".to_string()
}

fn default_blocked_apps() -> Vec<String> {
    vec![
        // Password managers
        "com.1password.*".to_string(),
        "com.agilebits.onepassword*".to_string(),
        "com.lastpass.LastPass".to_string(),
        "com.bitwarden.desktop".to_string(),
        "com.dashlane.Dashlane".to_string(),
        // System
        "com.apple.systempreferences".to_string(),
        "com.apple.SecurityAgent".to_string(),
        "com.apple.keychainaccess".to_string(),
        // Banking (pattern)
        "*banking*".to_string(),
        "*bank*".to_string(),
    ]
}

impl Config {
    /// Load configuration from the default path
    pub fn load() -> Self {
        Self::load_from_path(Self::default_config_path())
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: PathBuf) -> Self {
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    info!("Loaded configuration from {:?}", path);
                    config
                }
                Err(e) => {
                    warn!("Failed to parse config file: {}, using defaults", e);
                    Self::default()
                }
            },
            Err(_) => {
                info!("No config file found at {:?}, using defaults", path);
                Self::default()
            }
        }
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("unified-router")
            .join("config.toml")
    }

    /// Save configuration to the default path
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to_path(Self::default_config_path())
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        std::fs::write(&path, contents)?;
        info!("Saved configuration to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.general.enabled);
        assert_eq!(config.timing.base_interval_seconds, 5);
        assert_eq!(config.change_detection.hash_sensitivity, 8);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[general]
enabled = true
log_level = "debug"

[timing]
base_interval_seconds = 10

[change_detection]
hash_sensitivity = 12
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.log_level, "debug");
        assert_eq!(config.timing.base_interval_seconds, 10);
        assert_eq!(config.change_detection.hash_sensitivity, 12);
    }

    #[test]
    fn test_blocked_apps_default() {
        let config = Config::default();
        assert!(config.privacy.blocked_apps.iter().any(|s| s.contains("1password")));
    }
}
