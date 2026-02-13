// ScreenCaptureEngine.swift
// Handles efficient screen capture with differential detection

import Foundation
import Cocoa
import CoreGraphics
import os.log

/// Engine for capturing screen content efficiently
public class ScreenCaptureEngine {
    
    // MARK: - Configuration
    
    public struct Config {
        /// What to capture
        public var captureMode: CaptureMode = .allDisplays
        /// Image scale factor (1.0 = full resolution)
        public var imageScale: CGFloat = 1.0
        /// Bits difference threshold for "same" image
        public var hashSensitivity: Int = 8
        
        public init() {}
    }
    
    public enum CaptureMode {
        case activeWindow      // Only the frontmost window
        case activeDisplay     // Display containing active window
        case allDisplays       // All connected displays
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "ScreenCapture")
    private var config: Config
    private var lastImageHashes: [CGDirectDisplayID: UInt64] = [:]
    private var lastWindowTitle: String?  // Track title changes for tab detection
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    // MARK: - Permission Check
    
    /// Check if screen recording permission is granted
    public func hasScreenCapturePermission() -> Bool {
        // Attempting to capture will return nil if permission denied
        let testImage = CGWindowListCreateImage(
            CGRect(x: 0, y: 0, width: 1, height: 1),
            .optionOnScreenOnly,
            kCGNullWindowID,
            .bestResolution
        )
        return testImage != nil
    }
    
    /// Request screen capture permission (shows system dialog)
    public func requestPermission() {
        logger.info("Requesting screen capture permission")
        // This triggers the permission dialog
        _ = CGWindowListCreateImage(
            CGRect(x: 0, y: 0, width: 1, height: 1),
            .optionOnScreenOnly,
            kCGNullWindowID,
            .bestResolution
        )
    }
    
    // MARK: - Display Enumeration
    
    /// Get all active displays
    public func getActiveDisplays() -> [CGDirectDisplayID] {
        var displayCount: UInt32 = 0
        CGGetActiveDisplayList(0, nil, &displayCount)
        
        var displays = [CGDirectDisplayID](repeating: 0, count: Int(displayCount))
        CGGetActiveDisplayList(displayCount, &displays, &displayCount)
        
        return displays
    }
    
    /// Get detailed information about all displays
    public func getDisplayInfo() -> [DisplayInfo] {
        let displays = getActiveDisplays()
        
        return displays.map { displayID in
            let bounds = CGDisplayBounds(displayID)
            let isMain = CGDisplayIsMain(displayID) != 0
            let isBuiltin = CGDisplayIsBuiltin(displayID) != 0
            
            // Get scale factor from NSScreen
            let scaleFactor = NSScreen.screens.first {
                $0.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID == displayID
            }?.backingScaleFactor ?? 1.0
            
            return DisplayInfo(
                id: displayID,
                name: "Display \(displayID)",
                bounds: bounds,
                isMain: isMain,
                isBuiltin: isBuiltin,
                scaleFactor: scaleFactor
            )
        }
    }
    
    /// Get display containing the active window
    public func getActiveDisplay() -> CGDirectDisplayID? {
        guard let frontApp = NSWorkspace.shared.frontmostApplication else {
            return CGMainDisplayID()
        }
        
        let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return CGMainDisplayID()
        }
        
        for window in windowList {
            guard let ownerPID = window[kCGWindowOwnerPID as String] as? Int32,
                  ownerPID == frontApp.processIdentifier,
                  let boundsDict = window[kCGWindowBounds as String] as? [String: CGFloat] else {
                continue
            }
            
            let windowCenter = CGPoint(
                x: (boundsDict["X"] ?? 0) + (boundsDict["Width"] ?? 0) / 2,
                y: (boundsDict["Y"] ?? 0) + (boundsDict["Height"] ?? 0) / 2
            )
            
            // Find display containing this point
            for display in getActiveDisplays() {
                let displayBounds = CGDisplayBounds(display)
                if displayBounds.contains(windowCenter) {
                    return display
                }
            }
        }
        
        return CGMainDisplayID()
    }
    
    // MARK: - Capture Methods
    
    /// Capture based on configured mode
    public func capture() async -> [CaptureResult] {
        switch config.captureMode {
        case .activeWindow:
            if let result = await captureActiveWindow() {
                return [result]
            }
            return []
            
        case .activeDisplay:
            if let displayID = getActiveDisplay(),
               let result = await captureDisplay(displayID) {
                return [result]
            }
            return []
            
        case .allDisplays:
            return await captureAllDisplays()
        }
    }
    
    /// Capture only the active window
    public func captureActiveWindow() async -> CaptureResult? {
        guard let frontApp = NSWorkspace.shared.frontmostApplication else {
            logger.debug("No frontmost application")
            return nil
        }
        
        let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            logger.debug("Failed to get window list")
            return nil
        }
        
        // Find the frontmost window of the active app
        for window in windowList {
            guard let ownerPID = window[kCGWindowOwnerPID as String] as? Int32,
                  ownerPID == frontApp.processIdentifier,
                  let windowID = window[kCGWindowNumber as String] as? CGWindowID,
                  let boundsDict = window[kCGWindowBounds as String] as? [String: CGFloat],
                  let layer = window[kCGWindowLayer as String] as? Int,
                  layer == 0 else {  // Layer 0 = normal windows
                continue
            }
            
            let bounds = CGRect(
                x: boundsDict["X"] ?? 0,
                y: boundsDict["Y"] ?? 0,
                width: boundsDict["Width"] ?? 0,
                height: boundsDict["Height"] ?? 0
            )
            
            // Skip tiny windows
            guard bounds.width > 100 && bounds.height > 100 else {
                continue
            }
            
            // Capture this specific window
            guard let image = CGWindowListCreateImage(
                bounds,
                .optionIncludingWindow,
                windowID,
                [.boundsIgnoreFraming, .bestResolution]
            ) else {
                continue
            }
            
            let windowTitle = window[kCGWindowName as String] as? String
            
            logger.debug("Captured window: \(windowTitle ?? "untitled") (\(image.width)x\(image.height))")
            
            return CaptureResult(
                image: image,
                displayID: getActiveDisplay() ?? CGMainDisplayID(),
                bounds: bounds,
                windowTitle: windowTitle,
                applicationName: frontApp.localizedName,
                applicationBundleID: frontApp.bundleIdentifier,
                timestamp: Date()
            )
        }
        
        logger.debug("No suitable window found for \(frontApp.localizedName ?? "unknown")")
        return nil
    }
    
    /// Capture a specific display
    public func captureDisplay(_ displayID: CGDirectDisplayID) async -> CaptureResult? {
        guard let image = CGDisplayCreateImage(displayID) else {
            logger.warning("Failed to capture display \(displayID)")
            return nil
        }
        
        let bounds = CGDisplayBounds(displayID)
        
        logger.debug("Captured display \(displayID) (\(image.width)x\(image.height))")
        
        return CaptureResult(
            image: image,
            displayID: displayID,
            bounds: bounds,
            windowTitle: nil,
            applicationName: nil,
            timestamp: Date()
        )
    }
    
    /// Capture all displays in parallel
    public func captureAllDisplays() async -> [CaptureResult] {
        let displays = getActiveDisplays()
        
        return await withTaskGroup(of: CaptureResult?.self) { group in
            for displayID in displays {
                group.addTask {
                    await self.captureDisplay(displayID)
                }
            }
            
            var results: [CaptureResult] = []
            for await result in group {
                if let result = result {
                    results.append(result)
                }
            }
            return results
        }
    }
    
    // MARK: - Differential Capture
    
    /// Check if screen content has changed since last capture
    public func hasContentChanged(_ result: CaptureResult) -> Bool {
        // Check if window title changed (detects tab switches in browsers)
        if let currentTitle = result.windowTitle {
            if let lastTitle = lastWindowTitle, lastTitle != currentTitle {
                logger.info("Window title changed: '\(lastTitle)' -> '\(currentTitle)'")
                lastWindowTitle = currentTitle
                lastImageHashes[result.displayID] = computePerceptualHash(result.image)
                return true  // Title changed = content changed
            }
            lastWindowTitle = currentTitle
        }
        
        // Check perceptual hash for visual changes
        let currentHash = computePerceptualHash(result.image)
        
        if let lastHash = lastImageHashes[result.displayID] {
            let distance = hammingDistance(currentHash, lastHash)
            if distance < config.hashSensitivity {
                logger.debug("Content unchanged (distance: \(distance))")
                return false
            }
        }
        
        lastImageHashes[result.displayID] = currentHash
        return true
    }
    
    /// Capture only if content has changed
    public func captureIfChanged() async -> [CaptureResult] {
        let results = await capture()
        return results.filter { hasContentChanged($0) }
    }
    
    // MARK: - Perceptual Hashing
    
    /// Compute a perceptual hash for change detection
    /// Uses average hash algorithm - fast and good for detecting significant changes
    private func computePerceptualHash(_ image: CGImage) -> UInt64 {
        // Resize to 8x8 grayscale
        let size = 8
        let colorSpace = CGColorSpaceCreateDeviceGray()
        
        guard let context = CGContext(
            data: nil,
            width: size,
            height: size,
            bitsPerComponent: 8,
            bytesPerRow: size,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.none.rawValue
        ) else {
            return 0
        }
        
        context.interpolationQuality = .low
        context.draw(image, in: CGRect(x: 0, y: 0, width: size, height: size))
        
        guard let data = context.data else {
            return 0
        }
        
        let pixels = data.bindMemory(to: UInt8.self, capacity: size * size)
        
        // Calculate average
        var sum: Int = 0
        for i in 0..<(size * size) {
            sum += Int(pixels[i])
        }
        let average = UInt8(sum / (size * size))
        
        // Build hash
        var hash: UInt64 = 0
        for i in 0..<(size * size) {
            if pixels[i] > average {
                hash |= (1 << i)
            }
        }
        
        return hash
    }
    
    /// Calculate Hamming distance between two hashes
    private func hammingDistance(_ a: UInt64, _ b: UInt64) -> Int {
        return (a ^ b).nonzeroBitCount
    }
    
    /// Clear cached hashes (useful when resetting state)
    public func clearHashCache() {
        lastImageHashes.removeAll()
    }
}
