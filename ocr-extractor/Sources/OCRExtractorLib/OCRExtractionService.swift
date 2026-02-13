// OCRExtractionService.swift
// Main coordinator that orchestrates all OCR extraction components

import Foundation
import Cocoa
import IOKit.ps
import os.log

/// Main service that coordinates screen capture, OCR, and ingestion
public class OCRExtractionService {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Enable/disable the service
        public var enabled: Bool = true
        /// Capture mode (activeWindow, activeDisplay, allDisplays)
        public var captureMode: ScreenCaptureEngine.CaptureMode = .allDisplays
        /// Slow down on battery power
        public var throttleOnBattery: Bool = true
        /// Multiplier for capture interval on battery
        public var batteryThrottleMultiplier: Double = 3.0
        /// Slow down when thermal pressure is high
        public var thermalThrottleEnabled: Bool = true
        /// Base capture interval in seconds
        public var baseCaptureInterval: TimeInterval = 10.0
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "Service")
    
    private var config: Config
    private let activityMonitor: ActivityMonitor
    private let captureEngine: ScreenCaptureEngine
    private let ocrEngine: OCREngine
    private let contentProcessor: ContentProcessor
    private let privacyFilter: PrivacyFilter
    private let ingestionClient: IngestionClient
    
    private var isRunning = false
    private var captureTask: Task<Void, Never>?
    private var powerObserver: Any?
    
    // Statistics
    private var captureCount: Int = 0
    private var successCount: Int = 0
    private var skipCount: Int = 0
    
    /// Callback for processed payloads (optional, for custom handling)
    public var onPayloadReady: ((CapturePayload) -> Void)?
    
    /// Callback for status updates
    public var onStatusUpdate: ((ServiceStatus) -> Void)?
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
        
        var captureConfig = ScreenCaptureEngine.Config()
        captureConfig.captureMode = config.captureMode
        
        self.activityMonitor = ActivityMonitor()
        self.captureEngine = ScreenCaptureEngine(config: captureConfig)
        self.ocrEngine = OCREngine()
        self.contentProcessor = ContentProcessor()
        self.privacyFilter = PrivacyFilter()
        self.ingestionClient = IngestionClient()
        
        setupActivityMonitor()
    }
    
    // MARK: - Setup
    
    private func setupActivityMonitor() {
        activityMonitor.onShouldCapture = { [weak self] context in
            Task {
                await self?.performCapture(context: context)
            }
        }
    }
    
    // MARK: - Lifecycle
    
    /// Start the OCR extraction service
    public func start() {
        guard !isRunning else {
            logger.warning("Service already running")
            return
        }
        
        // Check screen capture permission
        guard captureEngine.hasScreenCapturePermission() else {
            logger.warning("Screen capture permission not granted")
            captureEngine.requestPermission()
            onStatusUpdate?(.permissionRequired)
            return
        }
        
        // Check if ingestion service is available
        if !ingestionClient.isServiceAvailable() {
            logger.warning("Ingestion service not available at socket path")
        }
        
        isRunning = true
        logger.info("OCR Extraction Service started")
        
        // Start activity monitoring
        activityMonitor.start()
        
        // Start periodic capture loop
        startCaptureLoop()
        
        // Monitor power state changes
        setupPowerMonitoring()
        
        onStatusUpdate?(.running)
    }
    
    /// Stop the OCR extraction service
    public func stop() {
        guard isRunning else { return }
        
        isRunning = false
        captureTask?.cancel()
        captureTask = nil
        activityMonitor.stop()
        
        if let observer = powerObserver {
            NotificationCenter.default.removeObserver(observer)
            powerObserver = nil
        }
        
        logger.info("OCR Extraction Service stopped")
        onStatusUpdate?(.stopped)
    }
    
    /// Pause capture temporarily
    public func pause() {
        config.enabled = false
        logger.info("OCR capture paused")
        onStatusUpdate?(.paused)
    }
    
    /// Resume capture
    public func resume() {
        config.enabled = true
        logger.info("OCR capture resumed")
        onStatusUpdate?(.running)
    }

    
    // MARK: - Capture Loop
    
    private func startCaptureLoop() {
        logger.info("Starting capture loop with interval: \(self.config.baseCaptureInterval)s")
        
        captureTask = Task { [weak self] in
            while let self = self, self.isRunning {
                let interval = self.calculateCaptureInterval()
                
                // Wait for interval
                try? await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
                
                guard self.isRunning && self.config.enabled else { 
                    self.logger.debug("Capture loop: service disabled, skipping")
                    continue 
                }
                
                // Always log what app is active
                let context = self.activityMonitor.getCaptureContext()
                let shouldCapture = self.activityMonitor.shouldCapture()
                let hasPending = self.activityMonitor.hasPendingAppSwitch()
                
                self.logger.info("Capture check - App: \(context.applicationName ?? "nil"), Idle: \(String(format: "%.1f", context.idleTime))s, ShouldCapture: \(shouldCapture), PendingSwitch: \(hasPending)")
                
                // Check if we should capture
                if shouldCapture {
                    await self.performCapture(context: context)
                }
            }
            self?.logger.info("Capture loop ended")
        }
    }
    
    private func calculateCaptureInterval() -> TimeInterval {
        var interval = config.baseCaptureInterval
        
        // Throttle on battery
        if config.throttleOnBattery && isOnBattery() {
            interval *= config.batteryThrottleMultiplier
            logger.debug("On battery, interval: \(interval)s")
        }
        
        // Throttle on thermal pressure
        if config.thermalThrottleEnabled && isThermalThrottled() {
            interval *= 2.0
            logger.debug("Thermal throttled, interval: \(interval)s")
        }
        
        return interval
    }
    
    // MARK: - Capture Pipeline
    
    private func performCapture(context: CaptureContext) async {
        guard isRunning && config.enabled else { return }
        
        captureCount += 1
        let startTime = Date()
        
        // Log context for debugging
        logger.info("Capture context - App: \(context.applicationName ?? "nil"), BundleID: \(context.applicationBundleID ?? "nil"), Window: \(context.windowTitle ?? "nil")")
        
        // Privacy check - block sensitive apps
        if privacyFilter.shouldBlockCapture(context: context) {
            logger.debug("Capture blocked for privacy: \(context.applicationName ?? "unknown")")
            skipCount += 1
            return
        }
        
        logger.debug("Starting capture #\(self.captureCount) for: \(context.applicationName ?? "unknown")")
        
        // Step 1: Capture screen(s) with change detection
        let captures = await captureEngine.captureIfChanged()
        
        guard !captures.isEmpty else {
            logger.debug("No screen changes detected, skipping OCR")
            skipCount += 1
            return
        }
        
        logger.info("Captured \(captures.count) screen(s)")
        
        // Step 2: Run OCR on captures
        let ocrResults = await ocrEngine.extractTextBatch(from: captures)
        
        // Step 3: Process results into payloads
        for result in ocrResults {
            // Apply privacy redaction
            var processedResult = result
            processedResult.text = privacyFilter.redactSensitiveContent(result.text)
            
            // Convert to payload
            guard let payload = contentProcessor.process(processedResult, context: context) else {
                continue
            }
            
            logger.info("Extracted \(payload.content.count) chars from \(payload.source)")
            
            // Call custom handler if set
            onPayloadReady?(payload)
            
            // Send to ingestion service
            await sendToIngestion(payload)
        }
        
        // Mark capture completed for timing
        activityMonitor.markCaptureCompleted()
        successCount += 1
        
        let elapsed = Date().timeIntervalSince(startTime)
        logger.debug("Capture pipeline completed in \(String(format: "%.2f", elapsed))s")
    }
    
    private func sendToIngestion(_ payload: CapturePayload) async {
        guard ingestionClient.isServiceAvailable() else {
            logger.warning("Ingestion service unavailable, payload not sent")
            return
        }
        
        do {
            let response = try await ingestionClient.send(payload)
            logger.info("Ingested: \(response.action) - \(response.ehl_doc_id ?? "unknown")")
        } catch {
            logger.error("Ingestion failed: \(error.localizedDescription)")
        }
    }
    
    // MARK: - Power Management
    
    private func setupPowerMonitoring() {
        powerObserver = NotificationCenter.default.addObserver(
            forName: NSNotification.Name("NSProcessInfoPowerStateDidChangeNotification"),
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handlePowerStateChange()
        }
    }
    
    private func handlePowerStateChange() {
        let interval = calculateCaptureInterval()
        logger.info("Power state changed, capture interval: \(interval)s")
    }
    
    private func isOnBattery() -> Bool {
        guard let snapshot = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
              let sources = IOPSCopyPowerSourcesList(snapshot)?.takeRetainedValue() as? [CFTypeRef],
              let source = sources.first,
              let info = IOPSGetPowerSourceDescription(snapshot, source)?.takeUnretainedValue() as? [String: Any],
              let powerSource = info[kIOPSPowerSourceStateKey as String] as? String else {
            return false
        }
        
        return powerSource == kIOPSBatteryPowerValue as String
    }
    
    private func isThermalThrottled() -> Bool {
        let state = ProcessInfo.processInfo.thermalState
        return state == .serious || state == .critical
    }
    
    // MARK: - Statistics
    
    /// Get current service statistics
    public func getStatistics() -> ServiceStatistics {
        return ServiceStatistics(
            captureCount: captureCount,
            successCount: successCount,
            skipCount: skipCount,
            isRunning: isRunning,
            isEnabled: config.enabled,
            currentInterval: calculateCaptureInterval(),
            isOnBattery: isOnBattery(),
            isThermalThrottled: isThermalThrottled()
        )
    }
    
    // MARK: - Configuration
    
    /// Update service configuration
    public func updateConfig(_ newConfig: Config) {
        self.config = newConfig
        logger.info("Configuration updated")
    }
}

// MARK: - Supporting Types

public enum ServiceStatus {
    case stopped
    case running
    case paused
    case permissionRequired
    case error(String)
}

public struct ServiceStatistics {
    public let captureCount: Int
    public let successCount: Int
    public let skipCount: Int
    public let isRunning: Bool
    public let isEnabled: Bool
    public let currentInterval: TimeInterval
    public let isOnBattery: Bool
    public let isThermalThrottled: Bool
    
    public var successRate: Double {
        guard captureCount > 0 else { return 0 }
        return Double(successCount) / Double(captureCount)
    }
}
