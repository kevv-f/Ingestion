// PrivacyFilter.swift
// Filters sensitive content and blocks capture for privacy-sensitive apps

import Foundation
import os.log

/// Filters captures for privacy and redacts sensitive content
public class PrivacyFilter {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Enable privacy filtering
        public var enabled: Bool = true
        /// Redact sensitive patterns from text
        public var redactSensitivePatterns: Bool = true
        /// Custom blocked bundle IDs
        public var customBlockedBundleIDs: Set<String> = []
        /// Custom blocked title patterns
        public var customBlockedTitlePatterns: [String] = []
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "PrivacyFilter")
    private var config: Config
    
    // Apps to never capture
    private let blockedBundleIDs: Set<String> = [
        // System
        "com.apple.systempreferences",
        "com.apple.SystemPreferences",
        "com.apple.Passwords",
        
        // Password managers
        "com.apple.keychainaccess",
        "com.1password.1password",
        "com.agilebits.onepassword7",
        "com.lastpass.LastPass",
        "com.bitwarden.desktop",
        "com.dashlane.dashlanephonefinal",
        "com.keepersecurity.keeper",
        "com.nordpass.macos.nordpass",
        
        // Banking/Finance apps
        "com.intuit.quicken",
        "com.mint.internal",
        
        // Security tools
        "com.apple.security.pf",
        "com.apple.Console"
    ]
    
    // Window title patterns to skip
    private let blockedTitlePatterns: [String] = [
        "password",
        "sign in",
        "log in",
        "login",
        "signin",
        "credit card",
        "payment",
        "checkout",
        "banking",
        "bank of",
        "paypal",
        "venmo",
        "private browsing",
        "incognito",
        "inprivate",
        "keychain",
        "1password",
        "lastpass",
        "bitwarden",
        "two-factor",
        "2fa",
        "authenticator",
        "verification code",
        "security code",
        "cvv",
        "ssn",
        "social security"
    ]
    
    // URL patterns to skip (detected in window title)
    private let blockedURLPatterns: [String] = [
        "accounts.google.com",
        "login.",
        "signin.",
        "auth.",
        "auth0.",
        "secure.",
        "banking.",
        "pay.",
        "checkout.",
        "account.",
        "myaccount.",
        "password.",
        "identity."
    ]
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    // MARK: - Filtering
    
    /// Check if capture should be blocked for privacy
    public func shouldBlockCapture(bundleID: String?, windowTitle: String?) -> Bool {
        guard config.enabled else {
            return false
        }
        
        // Check bundle ID
        if let bundleID = bundleID {
            if blockedBundleIDs.contains(bundleID) {
                logger.info("Blocked capture for app: \(bundleID)")
                return true
            }
            if config.customBlockedBundleIDs.contains(bundleID) {
                logger.info("Blocked capture for custom blocked app: \(bundleID)")
                return true
            }
        }
        
        // Check window title
        if let title = windowTitle?.lowercased() {
            // Check title patterns
            for pattern in blockedTitlePatterns {
                if title.contains(pattern) {
                    logger.info("Blocked capture for title pattern: \(pattern)")
                    return true
                }
            }
            
            // Check custom patterns
            for pattern in config.customBlockedTitlePatterns {
                if title.contains(pattern.lowercased()) {
                    logger.info("Blocked capture for custom title pattern: \(pattern)")
                    return true
                }
            }
            
            // Check URL patterns
            for pattern in blockedURLPatterns {
                if title.contains(pattern) {
                    logger.info("Blocked capture for URL pattern: \(pattern)")
                    return true
                }
            }
        }
        
        return false
    }
    
    /// Check if capture should be blocked based on context
    public func shouldBlockCapture(context: CaptureContext) -> Bool {
        return shouldBlockCapture(
            bundleID: context.applicationBundleID,
            windowTitle: context.windowTitle
        )
    }
    
    // MARK: - Content Redaction
    
    /// Redact sensitive patterns from extracted text
    public func redactSensitiveContent(_ text: String) -> String {
        guard config.redactSensitivePatterns else {
            return text
        }
        
        var redacted = text
        
        // Credit card numbers (various formats)
        // Pattern: 4 groups of 4 digits, optionally separated by spaces or dashes
        let ccPatterns = [
            #"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b"#,
            #"\b\d{4}[\s-]?\d{6}[\s-]?\d{5}\b"#  // Amex format
        ]
        
        for pattern in ccPatterns {
            redacted = redacted.replacingOccurrences(
                of: pattern,
                with: "[REDACTED-CC]",
                options: .regularExpression
            )
        }
        
        // SSN patterns (XXX-XX-XXXX)
        let ssnPattern = #"\b\d{3}[\s-]?\d{2}[\s-]?\d{4}\b"#
        redacted = redacted.replacingOccurrences(
            of: ssnPattern,
            with: "[REDACTED-SSN]",
            options: .regularExpression
        )
        
        // Phone numbers (various formats)
        let phonePatterns = [
            #"\b\d{3}[\s.-]?\d{3}[\s.-]?\d{4}\b"#,
            #"\(\d{3}\)[\s.-]?\d{3}[\s.-]?\d{4}"#,
            #"\+1[\s.-]?\d{3}[\s.-]?\d{3}[\s.-]?\d{4}"#
        ]
        
        for pattern in phonePatterns {
            redacted = redacted.replacingOccurrences(
                of: pattern,
                with: "[REDACTED-PHONE]",
                options: .regularExpression
            )
        }
        
        // API keys / tokens (common patterns)
        let apiKeyPatterns = [
            #"(?i)(api[_-]?key|apikey|api[_-]?token|access[_-]?token|secret[_-]?key|auth[_-]?token)\s*[:=]\s*['\"]?[\w\-]{20,}['\"]?"#,
            #"(?i)bearer\s+[\w\-\.]{20,}"#,
            #"sk-[a-zA-Z0-9]{20,}"#,  // OpenAI keys
            #"ghp_[a-zA-Z0-9]{36}"#,   // GitHub tokens
            #"gho_[a-zA-Z0-9]{36}"#    // GitHub OAuth tokens
        ]
        
        for pattern in apiKeyPatterns {
            redacted = redacted.replacingOccurrences(
                of: pattern,
                with: "[REDACTED-KEY]",
                options: .regularExpression
            )
        }
        
        // Password fields (if visible in text)
        let passwordPatterns = [
            #"(?i)password\s*[:=]\s*\S+"#,
            #"(?i)passwd\s*[:=]\s*\S+"#,
            #"(?i)pwd\s*[:=]\s*\S+"#
        ]
        
        for pattern in passwordPatterns {
            redacted = redacted.replacingOccurrences(
                of: pattern,
                with: "[REDACTED-PASSWORD]",
                options: .regularExpression
            )
        }
        
        return redacted
    }
    
    // MARK: - Configuration
    
    /// Add a bundle ID to the block list
    public func addBlockedBundleID(_ bundleID: String) {
        config.customBlockedBundleIDs.insert(bundleID)
    }
    
    /// Remove a bundle ID from the custom block list
    public func removeBlockedBundleID(_ bundleID: String) {
        config.customBlockedBundleIDs.remove(bundleID)
    }
    
    /// Add a title pattern to the block list
    public func addBlockedTitlePattern(_ pattern: String) {
        config.customBlockedTitlePatterns.append(pattern)
    }
    
    /// Update configuration
    public func updateConfig(_ config: Config) {
        self.config = config
    }
}
