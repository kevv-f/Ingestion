// ContentProcessor.swift
// Transforms OCR results into ingestion payloads

import Foundation
import os.log

/// Processes OCR results into payloads for the ingestion service
public class ContentProcessor {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Minimum characters to consider valid content
        public var minTextLength: Int = 50
        /// Maximum characters to process
        public var maxTextLength: Int = 100_000
        /// Time window for deduplication
        public var deduplicationWindow: TimeInterval = 300  // 5 minutes
        /// Text similarity threshold for dedup (0-1)
        public var similarityThreshold: Double = 0.9
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "ContentProcessor")
    private var config: Config
    private var recentContent: [(text: String, timestamp: Date)] = []
    
    // App bundle ID to source type mapping
    private let sourceMapping: [String: String] = [
        // Browsers
        "com.apple.Safari": "safari",
        "com.google.Chrome": "chrome",
        "com.microsoft.edgemac": "edge",
        "org.mozilla.firefox": "firefox",
        "com.brave.Browser": "brave",
        "com.operasoftware.Opera": "opera",
        "com.arc.browser": "arc",
        
        // Communication
        "com.tinyspeck.slackmacgap": "slack",
        "com.microsoft.teams": "teams",
        "com.microsoft.teams2": "teams",
        "com.microsoft.Outlook": "outlook",
        "com.apple.mail": "apple-mail",
        "com.hnc.Discord": "discord",
        "ru.keepcoder.Telegram": "telegram",
        "com.facebook.archon.developerID": "messenger",
        "com.zoom.us": "zoom",
        
        // Productivity
        "com.microsoft.Word": "word",
        "com.microsoft.Excel": "excel",
        "com.microsoft.Powerpoint": "powerpoint",
        "com.apple.iWork.Pages": "pages",
        "com.apple.iWork.Numbers": "numbers",
        "com.apple.iWork.Keynote": "keynote",
        "com.apple.Notes": "apple-notes",
        "notion.id": "notion",
        "md.obsidian": "obsidian",
        "com.apple.Preview": "preview",
        "com.apple.finder": "finder",
        
        // Development
        "com.microsoft.VSCode": "vscode",
        "com.microsoft.VSCodeInsiders": "vscode",
        "com.todesktop.230313mzl4w4u92": "cursor",  // Cursor editor
        "dev.kiro.app": "kiro",  // Kiro IDE
        "com.amazon.kiro": "kiro",  // Kiro IDE (alternate)
        "com.jetbrains.intellij": "intellij",
        "com.jetbrains.intellij.ce": "intellij",
        "com.jetbrains.WebStorm": "webstorm",
        "com.jetbrains.pycharm": "pycharm",
        "com.jetbrains.goland": "goland",
        "com.apple.dt.Xcode": "xcode",
        "com.sublimetext.4": "sublime",
        "com.sublimetext.3": "sublime",
        "com.github.atom": "atom",
        "com.googlecode.iterm2": "terminal",
        "com.apple.Terminal": "terminal",
        
        // Other
        "com.figma.Desktop": "figma",
        "com.linear": "linear",
        "com.readdle.PDFExpert-Mac": "pdf",
        "com.spotify.client": "spotify",
        "com.apple.Music": "music"
    ]
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    // MARK: - Processing
    
    /// Process OCR result into ingestion payload
    public func process(_ ocrResult: OCRResult, context: CaptureContext) -> CapturePayload? {
        // Validate text length
        var text = ocrResult.text.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Clean up OCR artifacts and browser UI noise
        text = cleanOCRText(text)
        
        guard text.count >= config.minTextLength else {
            logger.debug("Text too short (\(text.count) chars), skipping")
            return nil
        }
        
        // Truncate if too long
        let processedText = String(text.prefix(config.maxTextLength))
        
        // Check for duplicates
        if isDuplicate(processedText) {
            logger.debug("Duplicate content detected, skipping")
            return nil
        }
        
        // Determine source type
        let source = determineSource(
            bundleID: context.applicationBundleID ?? ocrResult.applicationBundleID,
            windowTitle: ocrResult.windowTitle ?? context.windowTitle
        )
        
        // Build canonical URL
        let url = buildCanonicalURL(
            source: source,
            bundleID: context.applicationBundleID ?? ocrResult.applicationBundleID,
            windowTitle: ocrResult.windowTitle ?? context.windowTitle
        )
        
        // Extract title
        let title = extractTitle(
            windowTitle: ocrResult.windowTitle ?? context.windowTitle,
            text: processedText
        )
        
        // Record for deduplication
        recordContent(processedText)
        
        logger.info("Processed content: \(source) - \(title ?? "untitled") (\(processedText.count) chars)")
        
        return CapturePayload(
            source: source,
            url: url,
            content: processedText,
            title: title,
            author: nil,
            channel: context.applicationName ?? ocrResult.applicationName,
            timestamp: Int(context.timestamp.timeIntervalSince1970)
        )
    }
    
    // MARK: - Text Cleaning
    
    /// Clean up OCR text by removing browser UI artifacts and noise
    private func cleanOCRText(_ text: String) -> String {
        var lines = text.components(separatedBy: .newlines)
        
        // Patterns that indicate browser UI or noise (not actual content)
        let noisePatterns: [String] = [
            // Browser UI elements
            "File Edit View History Bookmarks",
            "File Edit View Go Window Help",
            "Safari File Edit View",
            "Chrome File Edit View",
            "• Personal",
            "Personal v",
            // Tab indicators
            "...",
            "• • •",
            // Common UI noise
            "Search or enter",
            "Address bar",
            "Reload this page",
            // macOS menu bar
            "Fri ", "Sat ", "Sun ", "Mon ", "Tue ", "Wed ", "Thu ",
            "AM", "PM",
            // Short fragments that are likely UI
        ]
        
        // Filter out lines that are likely UI noise
        lines = lines.filter { line in
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            
            // Skip empty or very short lines
            if trimmed.count < 3 {
                return false
            }
            
            // Skip lines that are mostly special characters (likely UI)
            let alphanumericCount = trimmed.filter { $0.isLetter || $0.isNumber }.count
            let ratio = Double(alphanumericCount) / Double(trimmed.count)
            if ratio < 0.5 && trimmed.count < 20 {
                return false
            }
            
            // Skip known noise patterns
            for pattern in noisePatterns {
                if trimmed.contains(pattern) {
                    return false
                }
            }
            
            // Skip lines that look like tab titles (short with ellipsis or special chars)
            if trimmed.count < 30 && (trimmed.contains("...") || trimmed.contains("•") || trimmed.contains("฿") || trimmed.contains("%")) {
                return false
            }
            
            return true
        }
        
        // Join and clean up multiple spaces/newlines
        var result = lines.joined(separator: "\n")
        
        // Clean up multiple consecutive newlines
        while result.contains("\n\n\n") {
            result = result.replacingOccurrences(of: "\n\n\n", with: "\n\n")
        }
        
        // Clean up multiple spaces
        while result.contains("  ") {
            result = result.replacingOccurrences(of: "  ", with: " ")
        }
        
        return result.trimmingCharacters(in: .whitespacesAndNewlines)
    }
    
    // MARK: - Source Detection
    
    private func determineSource(bundleID: String?, windowTitle: String?) -> String {
        // Check bundle ID mapping
        if let bundleID = bundleID,
           let source = sourceMapping[bundleID] {
            logger.debug("Source from bundleID '\(bundleID)': \(source)")
            return source
        }
        
        // Log if bundle ID not found in mapping
        if let bundleID = bundleID {
            logger.debug("Bundle ID '\(bundleID)' not in source mapping")
        }
        
        // Infer from window title
        if let title = windowTitle?.lowercased() {
            // Browsers with specific sites
            if title.contains("slack") { return "slack" }
            if title.contains("teams") { return "teams" }
            if title.contains("jira") { return "jira" }
            if title.contains("confluence") { return "confluence" }
            if title.contains("notion") { return "notion" }
            if title.contains("google docs") || title.contains("- google docs") { return "gdocs" }
            if title.contains("google sheets") || title.contains("- google sheets") { return "gsheets" }
            if title.contains("google slides") || title.contains("- google slides") { return "gslides" }
            if title.contains("gmail") { return "gmail" }
            if title.contains("github") { return "github" }
            if title.contains("gitlab") { return "gitlab" }
            if title.contains("linear") { return "linear" }
            if title.contains("figma") { return "figma" }
        }
        
        logger.debug("Using fallback source 'ocr-capture'")
        return "ocr-capture"
    }
    
    // MARK: - URL Building
    
    private func buildCanonicalURL(source: String, bundleID: String?, windowTitle: String?) -> String {
        // Create a stable canonical URL for metadata-based deduplication
        // This allows the ingestion service to find existing entries and append new content
        
        let cleanedTitle = cleanWindowTitle(windowTitle ?? "unknown")
        
        // Sanitize for URL: lowercase, replace spaces/special chars with dashes
        let sanitizedTitle = cleanedTitle
            .lowercased()
            .replacingOccurrences(of: " ", with: "-")
            .replacingOccurrences(of: "/", with: "-")
            .replacingOccurrences(of: "\\", with: "-")
            .replacingOccurrences(of: ":", with: "-")
            .replacingOccurrences(of: ".", with: "-")
            .replacingOccurrences(of: ",", with: "")
            .replacingOccurrences(of: "'", with: "")
            .replacingOccurrences(of: "\"", with: "")
            .replacingOccurrences(of: "--", with: "-")  // Clean up double dashes
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        
        // Truncate to reasonable length
        let truncatedTitle = String(sanitizedTitle.prefix(80))
        
        // NO timestamp - this creates a stable URL for the same document
        return "ocr://\(source)/\(truncatedTitle)"
    }
    
    // MARK: - Title Extraction
    
    private func extractTitle(windowTitle: String?, text: String) -> String? {
        // Use window title if available
        if let title = windowTitle, !title.isEmpty {
            // Clean up common browser suffixes
            return cleanWindowTitle(title)
        }
        
        // Extract first meaningful line as title
        let lines = text.components(separatedBy: .newlines)
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.count >= 10 && trimmed.count <= 200 {
                return String(trimmed.prefix(100))
            }
        }
        
        return nil
    }
    
    private func cleanWindowTitle(_ title: String) -> String {
        var cleaned = title
        
        // Remove common browser suffixes
        let suffixes = [
            " - Google Chrome",
            " - Safari",
            " - Microsoft Edge",
            " — Mozilla Firefox",
            " - Firefox",
            " - Brave",
            " - Opera"
        ]
        
        for suffix in suffixes {
            if cleaned.hasSuffix(suffix) {
                cleaned = String(cleaned.dropLast(suffix.count))
            }
        }
        
        return cleaned.trimmingCharacters(in: .whitespacesAndNewlines)
    }
    
    // MARK: - Deduplication
    
    private func isDuplicate(_ text: String) -> Bool {
        let now = Date()
        
        // Clean old entries
        recentContent.removeAll { now.timeIntervalSince($0.timestamp) > config.deduplicationWindow }
        
        // Check similarity with recent content
        for recent in recentContent {
            let similarity = calculateSimilarity(text, recent.text)
            if similarity >= config.similarityThreshold {
                return true
            }
        }
        
        return false
    }
    
    private func recordContent(_ text: String) {
        recentContent.append((text: text, timestamp: Date()))
        
        // Keep only last 100 entries
        if recentContent.count > 100 {
            recentContent.removeFirst(recentContent.count - 100)
        }
    }
    
    /// Jaccard similarity for text comparison
    private func calculateSimilarity(_ a: String, _ b: String) -> Double {
        let wordsA = Set(a.lowercased().components(separatedBy: .whitespacesAndNewlines).filter { !$0.isEmpty })
        let wordsB = Set(b.lowercased().components(separatedBy: .whitespacesAndNewlines).filter { !$0.isEmpty })
        
        guard !wordsA.isEmpty || !wordsB.isEmpty else {
            return 1.0  // Both empty = same
        }
        
        let intersection = wordsA.intersection(wordsB).count
        let union = wordsA.union(wordsB).count
        
        return union > 0 ? Double(intersection) / Double(union) : 0
    }
    
    // MARK: - State Management
    
    /// Clear deduplication cache
    public func clearCache() {
        recentContent.removeAll()
    }
}
