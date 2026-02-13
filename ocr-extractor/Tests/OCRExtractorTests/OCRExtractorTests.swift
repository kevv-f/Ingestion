// OCRExtractorTests.swift
// Tests for OCR Extractor components

import XCTest
@testable import OCRExtractorLib

final class OCRExtractorTests: XCTestCase {
    
    // MARK: - CaptureTypes Tests
    
    func testCaptureContextInitialization() {
        let context = CaptureContext(
            timestamp: Date(),
            applicationName: "Safari",
            applicationBundleID: "com.apple.Safari",
            windowTitle: "Test Page",
            idleTime: 5.0,
            displayCount: 2
        )
        
        XCTAssertEqual(context.applicationName, "Safari")
        XCTAssertEqual(context.applicationBundleID, "com.apple.Safari")
        XCTAssertEqual(context.windowTitle, "Test Page")
        XCTAssertEqual(context.idleTime, 5.0)
        XCTAssertEqual(context.displayCount, 2)
    }
    
    func testCapturePayloadEncoding() throws {
        let payload = CapturePayload(
            source: "ocr-capture",
            url: "ocr://safari/test-page",
            content: "Test content from screen",
            title: "Test Page",
            author: nil,
            channel: "Safari",
            timestamp: 1234567890
        )
        
        let encoder = JSONEncoder()
        let data = try encoder.encode(payload)
        
        let decoder = JSONDecoder()
        let decoded = try decoder.decode(CapturePayload.self, from: data)
        
        XCTAssertEqual(decoded.source, "ocr-capture")
        XCTAssertEqual(decoded.url, "ocr://safari/test-page")
        XCTAssertEqual(decoded.content, "Test content from screen")
        XCTAssertEqual(decoded.title, "Test Page")
        XCTAssertEqual(decoded.channel, "Safari")
        XCTAssertEqual(decoded.timestamp, 1234567890)
    }
    
    // MARK: - PrivacyFilter Tests
    
    func testPrivacyFilterBlocksPasswordManagers() {
        let filter = PrivacyFilter()
        
        // Should block 1Password
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: "com.1password.1password",
            windowTitle: nil
        ))
        
        // Should block LastPass
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: "com.lastpass.LastPass",
            windowTitle: nil
        ))
        
        // Should block Bitwarden
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: "com.bitwarden.desktop",
            windowTitle: nil
        ))
    }
    
    func testPrivacyFilterBlocksSensitiveTitles() {
        let filter = PrivacyFilter()
        
        // Should block login pages
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: nil,
            windowTitle: "Sign in to your account"
        ))
        
        // Should block password pages
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: nil,
            windowTitle: "Change Password"
        ))
        
        // Should block banking
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: nil,
            windowTitle: "Bank of America - Online Banking"
        ))
        
        // Should block private browsing
        XCTAssertTrue(filter.shouldBlockCapture(
            bundleID: nil,
            windowTitle: "Private Browsing - Safari"
        ))
    }
    
    func testPrivacyFilterAllowsNormalApps() {
        let filter = PrivacyFilter()
        
        // Should allow Safari with normal page
        XCTAssertFalse(filter.shouldBlockCapture(
            bundleID: "com.apple.Safari",
            windowTitle: "Apple - Wikipedia"
        ))
        
        // Should allow VS Code
        XCTAssertFalse(filter.shouldBlockCapture(
            bundleID: "com.microsoft.VSCode",
            windowTitle: "main.swift - MyProject"
        ))
        
        // Should allow Slack
        XCTAssertFalse(filter.shouldBlockCapture(
            bundleID: "com.tinyspeck.slackmacgap",
            windowTitle: "Slack - #general"
        ))
    }
    
    func testPrivacyFilterRedactsCreditCards() {
        let filter = PrivacyFilter()
        
        let text = "My card number is 4111-1111-1111-1111 and CVV is 123"
        let redacted = filter.redactSensitiveContent(text)
        
        XCTAssertTrue(redacted.contains("[REDACTED-CC]"))
        XCTAssertFalse(redacted.contains("4111-1111-1111-1111"))
    }
    
    func testPrivacyFilterRedactsSSN() {
        let filter = PrivacyFilter()
        
        let text = "SSN: 123-45-6789"
        let redacted = filter.redactSensitiveContent(text)
        
        XCTAssertTrue(redacted.contains("[REDACTED-SSN]"))
        XCTAssertFalse(redacted.contains("123-45-6789"))
    }
    
    func testPrivacyFilterRedactsAPIKeys() {
        let filter = PrivacyFilter()
        
        let text = "api_key: sk-1234567890abcdefghijklmnop"
        let redacted = filter.redactSensitiveContent(text)
        
        XCTAssertTrue(redacted.contains("[REDACTED-KEY]"))
    }
    
    // MARK: - ContentProcessor Tests
    
    func testContentProcessorRejectsShortText() {
        let processor = ContentProcessor()
        let context = CaptureContext()
        
        let shortResult = OCRResult(
            text: "Too short",
            blocks: [],
            confidence: 0.9,
            processingTime: 0.1,
            imageSize: CGSize(width: 100, height: 100)
        )
        
        let payload = processor.process(shortResult, context: context)
        XCTAssertNil(payload, "Should reject text shorter than minimum length")
    }
    
    func testContentProcessorAcceptsValidText() {
        let processor = ContentProcessor()
        let context = CaptureContext(
            applicationName: "Safari",
            applicationBundleID: "com.apple.Safari",
            windowTitle: "Test Page - Safari"
        )
        
        let validResult = OCRResult(
            text: String(repeating: "This is valid content. ", count: 10),
            blocks: [],
            confidence: 0.9,
            processingTime: 0.1,
            imageSize: CGSize(width: 1920, height: 1080)
        )
        
        let payload = processor.process(validResult, context: context)
        XCTAssertNotNil(payload, "Should accept text meeting minimum length")
        XCTAssertEqual(payload?.source, "safari")
    }
    
    func testContentProcessorDetectsSourceFromBundleID() {
        let processor = ContentProcessor()
        
        let testCases: [(bundleID: String, expectedSource: String)] = [
            ("com.apple.Safari", "safari"),
            ("com.google.Chrome", "chrome"),
            ("com.tinyspeck.slackmacgap", "slack"),
            ("com.microsoft.teams", "teams"),
            ("com.microsoft.VSCode", "vscode"),
            ("notion.id", "notion")
        ]
        
        for testCase in testCases {
            let context = CaptureContext(
                applicationBundleID: testCase.bundleID
            )
            
            let result = OCRResult(
                text: String(repeating: "Content ", count: 20),
                blocks: [],
                confidence: 0.9,
                processingTime: 0.1,
                imageSize: CGSize(width: 1920, height: 1080)
            )
            
            let payload = processor.process(result, context: context)
            XCTAssertEqual(payload?.source, testCase.expectedSource,
                          "Bundle ID \(testCase.bundleID) should map to \(testCase.expectedSource)")
        }
    }
    
    func testContentProcessorDeduplication() {
        let processor = ContentProcessor()
        let context = CaptureContext()
        
        let content = String(repeating: "Duplicate content test. ", count: 10)
        
        let result1 = OCRResult(
            text: content,
            blocks: [],
            confidence: 0.9,
            processingTime: 0.1,
            imageSize: CGSize(width: 1920, height: 1080)
        )
        
        // First submission should succeed
        let payload1 = processor.process(result1, context: context)
        XCTAssertNotNil(payload1, "First submission should succeed")
        
        // Identical content should be deduplicated
        let payload2 = processor.process(result1, context: context)
        XCTAssertNil(payload2, "Duplicate content should be rejected")
    }
    
    // MARK: - ActivityMonitor Tests
    
    func testActivityMonitorConfiguration() {
        var config = ActivityMonitor.Config()
        config.idleThreshold = 3.0
        config.minCaptureInterval = 10.0
        config.maxCaptureInterval = 60.0
        
        let monitor = ActivityMonitor(config: config)
        
        // Just verify it initializes without crashing
        XCTAssertNotNil(monitor)
    }
    
    // MARK: - OCREngine Tests
    
    func testOCREngineConfiguration() {
        var config = OCREngine.Config()
        config.recognitionLevel = .fast
        config.usesLanguageCorrection = false
        config.recognitionLanguages = ["en-US", "es-ES"]
        
        let engine = OCREngine(config: config)
        
        // Just verify it initializes without crashing
        XCTAssertNotNil(engine)
    }
    
    func testOCREngineSupportedLanguages() {
        let languages = OCREngine.supportedLanguages()
        
        // Should include at least English
        XCTAssertTrue(languages.contains("en-US") || languages.contains { $0.hasPrefix("en") },
                     "Should support English")
    }
    
    // MARK: - ScreenCaptureEngine Tests
    
    func testScreenCaptureEngineConfiguration() {
        var config = ScreenCaptureEngine.Config()
        config.captureMode = .activeDisplay
        config.hashSensitivity = 10
        
        let engine = ScreenCaptureEngine(config: config)
        
        // Just verify it initializes without crashing
        XCTAssertNotNil(engine)
    }
    
    func testScreenCaptureEngineDisplayEnumeration() {
        let engine = ScreenCaptureEngine()
        let displays = engine.getActiveDisplays()
        
        // Should have at least one display
        XCTAssertGreaterThanOrEqual(displays.count, 1, "Should detect at least one display")
    }
    
    func testScreenCaptureEngineDisplayInfo() {
        let engine = ScreenCaptureEngine()
        let displayInfo = engine.getDisplayInfo()
        
        // Should have info for at least one display
        XCTAssertGreaterThanOrEqual(displayInfo.count, 1)
        
        // Main display should exist
        let hasMain = displayInfo.contains { $0.isMain }
        XCTAssertTrue(hasMain, "Should have a main display")
    }
    
    // MARK: - IngestionClient Tests
    
    func testIngestionClientConfiguration() {
        var config = IngestionClient.Config()
        config.socketPath = "/tmp/test-socket.sock"
        config.timeout = 10.0
        config.retryCount = 5
        
        let client = IngestionClient(config: config)
        
        // Just verify it initializes without crashing
        XCTAssertNotNil(client)
    }
    
    func testIngestionClientServiceAvailability() {
        let client = IngestionClient()
        
        // Service may or may not be available depending on environment
        // Just verify the check doesn't crash
        _ = client.isServiceAvailable()
    }
    
    // MARK: - OCRExtractionService Tests
    
    func testServiceConfiguration() {
        var config = OCRExtractionService.Config()
        config.enabled = true
        config.captureMode = .activeWindow
        config.throttleOnBattery = true
        config.baseCaptureInterval = 15.0
        
        let service = OCRExtractionService(config: config)
        
        // Just verify it initializes without crashing
        XCTAssertNotNil(service)
    }
    
    func testServiceStatistics() {
        let service = OCRExtractionService()
        let stats = service.getStatistics()
        
        // Initial stats should be zero
        XCTAssertEqual(stats.captureCount, 0)
        XCTAssertEqual(stats.successCount, 0)
        XCTAssertEqual(stats.skipCount, 0)
        XCTAssertFalse(stats.isRunning)
    }
}
