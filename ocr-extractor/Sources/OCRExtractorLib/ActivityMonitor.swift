// ActivityMonitor.swift
// Tracks user activity to determine optimal capture timing

import Foundation
import Cocoa
import os.log

/// Monitors user activity to determine when to capture screen content
public class ActivityMonitor {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Seconds of inactivity before capture is allowed
        public var idleThreshold: TimeInterval = 2.0
        /// Wait time after scroll stops
        public var scrollDebounce: TimeInterval = 1.0
        /// Wait time after focus change
        public var focusDebounce: TimeInterval = 0.5
        /// Minimum time between captures
        public var minCaptureInterval: TimeInterval = 5.0
        /// Maximum time between captures (force capture)
        public var maxCaptureInterval: TimeInterval = 30.0
        /// Force capture on app switch (ignore idle)
        public var captureOnAppSwitch: Bool = true
        /// Force capture on window title change (tab switch)
        public var captureOnTitleChange: Bool = true
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "ActivityMonitor")
    private var config: Config
    
    private var lastCaptureTime: Date = .distantPast
    private var lastFocusChangeTime: Date = .distantPast
    private var lastScrollTime: Date = .distantPast
    private var lastAppBundleID: String?
    private var lastWindowTitle: String?
    private var pendingAppSwitchCapture: Bool = false
    
    private var focusObserver: Any?
    private var scrollMonitor: Any?
    private var titleCheckTimer: Timer?
    
    /// Callback when capture should be triggered
    public var onShouldCapture: ((CaptureContext) -> Void)?
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    deinit {
        stop()
    }
    
    // MARK: - Lifecycle
    
    /// Start monitoring user activity
    public func start() {
        logger.info("Starting activity monitor")
        setupObservers()
        startTitleMonitoring()
    }
    
    /// Stop monitoring
    public func stop() {
        logger.info("Stopping activity monitor")
        teardownObservers()
        titleCheckTimer?.invalidate()
        titleCheckTimer = nil
    }
    
    // MARK: - Setup
    
    private func setupObservers() {
        // Monitor app activation
        focusObserver = NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleAppActivation(notification)
        }
        
        // Monitor scroll events globally
        scrollMonitor = NSEvent.addGlobalMonitorForEvents(
            matching: .scrollWheel
        ) { [weak self] _ in
            self?.lastScrollTime = Date()
        }
    }
    
    private func teardownObservers() {
        if let observer = focusObserver {
            NSWorkspace.shared.notificationCenter.removeObserver(observer)
            focusObserver = nil
        }
        if let monitor = scrollMonitor {
            NSEvent.removeMonitor(monitor)
            scrollMonitor = nil
        }
    }
    
    /// Start monitoring window title changes (for tab switches)
    private func startTitleMonitoring() {
        // Check window title every 2 seconds
        titleCheckTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            self?.checkForTitleChange()
        }
    }
    
    private func checkForTitleChange() {
        guard config.captureOnTitleChange else { return }
        
        let currentTitle = getActiveWindowTitle()
        let currentApp = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        
        // Only check title change within the same app
        if currentApp == lastAppBundleID, let current = currentTitle, let last = lastWindowTitle {
            if current != last {
                logger.info("Window title changed: '\(last)' -> '\(current)'")
                lastWindowTitle = current
                
                // Force capture on title change (tab switch)
                if canCaptureNow() {
                    let context = getCaptureContext()
                    onShouldCapture?(context)
                }
            }
        }
        
        lastWindowTitle = currentTitle
    }
    
    // MARK: - Activity Detection
    
    /// Returns seconds since last user input (mouse/keyboard)
    public func getIdleTime() -> TimeInterval {
        // CGEventSource.secondsSinceLastEventType returns time since last event
        let mouseIdle = CGEventSource.secondsSinceLastEventType(
            .hidSystemState,
            eventType: .mouseMoved
        )
        let keyboardIdle = CGEventSource.secondsSinceLastEventType(
            .hidSystemState,
            eventType: .keyDown
        )
        let clickIdle = CGEventSource.secondsSinceLastEventType(
            .hidSystemState,
            eventType: .leftMouseDown
        )
        
        return min(mouseIdle, keyboardIdle, clickIdle)
    }
    
    /// Check if user is currently idle
    public func isUserIdle() -> Bool {
        return getIdleTime() >= config.idleThreshold
    }
    
    /// Check if scroll has settled
    public func isScrollSettled() -> Bool {
        return Date().timeIntervalSince(lastScrollTime) >= config.scrollDebounce
    }
    
    /// Check if enough time has passed since last capture
    public func canCaptureNow() -> Bool {
        let timeSinceLastCapture = Date().timeIntervalSince(lastCaptureTime)
        return timeSinceLastCapture >= config.minCaptureInterval
    }
    
    /// Check if we must capture (max interval exceeded)
    public func mustCaptureNow() -> Bool {
        let timeSinceLastCapture = Date().timeIntervalSince(lastCaptureTime)
        return timeSinceLastCapture >= config.maxCaptureInterval
    }
    
    /// Check if there's a pending app switch capture
    public func hasPendingAppSwitch() -> Bool {
        return pendingAppSwitchCapture
    }
    
    /// Clear pending app switch flag
    public func clearPendingAppSwitch() {
        pendingAppSwitchCapture = false
    }
    
    // MARK: - Capture Decision
    
    /// Determine if capture should happen now
    public func shouldCapture() -> Bool {
        // Always capture if there's a pending app switch
        if pendingAppSwitchCapture && canCaptureNow() {
            logger.debug("Pending app switch capture")
            return true
        }
        
        // Always capture if max interval exceeded
        if mustCaptureNow() {
            logger.debug("Max interval exceeded, forcing capture")
            return true
        }
        
        // Don't capture if min interval not met
        if !canCaptureNow() {
            return false
        }
        
        // Capture if user is idle and scroll has settled
        return isUserIdle() && isScrollSettled()
    }
    
    // MARK: - Context
    
    /// Get current capture context
    public func getCaptureContext() -> CaptureContext {
        let frontApp = NSWorkspace.shared.frontmostApplication
        
        return CaptureContext(
            timestamp: Date(),
            applicationName: frontApp?.localizedName,
            applicationBundleID: frontApp?.bundleIdentifier,
            windowTitle: getActiveWindowTitle(),
            idleTime: getIdleTime(),
            displayCount: getDisplayCount()
        )
    }
    
    private func getActiveWindowTitle() -> String? {
        guard let frontApp = NSWorkspace.shared.frontmostApplication else {
            return nil
        }
        
        let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        
        for window in windowList {
            if let ownerPID = window[kCGWindowOwnerPID as String] as? Int32,
               ownerPID == frontApp.processIdentifier,
               let title = window[kCGWindowName as String] as? String,
               !title.isEmpty {
                return title
            }
        }
        
        return nil
    }
    
    private func getDisplayCount() -> Int {
        var displayCount: UInt32 = 0
        CGGetActiveDisplayList(0, nil, &displayCount)
        return Int(displayCount)
    }
    
    // MARK: - Event Handlers
    
    private func handleAppActivation(_ notification: Notification) {
        lastFocusChangeTime = Date()
        
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else {
            return
        }
        
        let newBundleID = app.bundleIdentifier
        logger.info("App activated: \(app.localizedName ?? "unknown") (\(newBundleID ?? "nil"))")
        
        // Check if this is actually a different app
        let isNewApp = newBundleID != lastAppBundleID
        lastAppBundleID = newBundleID
        lastWindowTitle = getActiveWindowTitle()
        
        if isNewApp && config.captureOnAppSwitch {
            // Mark pending capture - will be picked up by capture loop
            pendingAppSwitchCapture = true
            
            // Also trigger immediate capture after debounce
            DispatchQueue.main.asyncAfter(deadline: .now() + config.focusDebounce) { [weak self] in
                guard let self = self, self.canCaptureNow() else { return }
                
                let context = self.getCaptureContext()
                self.logger.info("Triggering app switch capture for: \(context.applicationName ?? "unknown")")
                self.onShouldCapture?(context)
            }
        }
    }
    
    /// Mark that a capture was completed
    public func markCaptureCompleted() {
        lastCaptureTime = Date()
        pendingAppSwitchCapture = false
    }
}
