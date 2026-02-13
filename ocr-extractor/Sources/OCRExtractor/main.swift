// main.swift
// OCR Extraction Service Entry Point

import Foundation
import Cocoa
import OCRExtractorLib
import ScreenCaptureKit
import os.log

// MARK: - Main Application

@main
struct OCRExtractorApp {
    
    static let logger = Logger(subsystem: "com.clace.ocr", category: "Main")
    
    static func main() async {
        // Initialize NSApplication for GUI access (required for ScreenCaptureKit)
        let _ = NSApplication.shared
        
        logger.info("OCR Extractor starting...")
        
        // Parse command line arguments
        let arguments = CommandLine.arguments
        
        if arguments.contains("--help") || arguments.contains("-h") {
            printUsage()
            return
        }
        
        if arguments.contains("--version") || arguments.contains("-v") {
            print("OCR Extractor v1.0.0")
            return
        }
        
        // Check for single-image mode (for unified router integration)
        if let imageIndex = arguments.firstIndex(of: "--image"),
           imageIndex + 1 < arguments.count {
            let imagePath = arguments[imageIndex + 1]
            let jsonOutput = arguments.contains("--json")
            await processImage(path: imagePath, jsonOutput: jsonOutput)
            return
        }
        
        // Check for window-id mode (capture specific window and OCR)
        if let windowIdIndex = arguments.firstIndex(of: "--window-id"),
           windowIdIndex + 1 < arguments.count,
           let windowId = UInt32(arguments[windowIdIndex + 1]) {
            let jsonOutput = arguments.contains("--json")
            let captureOnly = arguments.contains("--capture-only")
            
            // Get output path for capture-only mode
            var outputPath: String? = nil
            if captureOnly, let outputIndex = arguments.firstIndex(of: "--output"),
               outputIndex + 1 < arguments.count {
                outputPath = arguments[outputIndex + 1]
            }
            
            await captureAndOCRWindow(windowId: windowId, jsonOutput: jsonOutput, captureOnly: captureOnly, outputPath: outputPath)
            return
        }
        
        // Check for required permissions
        let captureEngine = ScreenCaptureEngine()
        if !captureEngine.hasScreenCapturePermission() {
            print("⚠️  Screen recording permission required.")
            print("   Please grant permission in System Settings > Privacy & Security > Screen Recording")
            print("   Then restart the application.")
            captureEngine.requestPermission()
            
            // Wait a moment for the dialog
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            return
        }
        
        // Configure service
        var config = OCRExtractionService.Config()
        
        // Parse configuration from arguments
        if let modeIndex = arguments.firstIndex(of: "--mode"),
           modeIndex + 1 < arguments.count {
            switch arguments[modeIndex + 1] {
            case "window":
                config.captureMode = .activeWindow
            case "display":
                config.captureMode = .activeDisplay
            case "all":
                config.captureMode = .allDisplays
            default:
                break
            }
        } else {
            // Default to allDisplays for multi-monitor support
            config.captureMode = .allDisplays
        }
        
        if let intervalIndex = arguments.firstIndex(of: "--interval"),
           intervalIndex + 1 < arguments.count,
           let interval = Double(arguments[intervalIndex + 1]) {
            config.baseCaptureInterval = interval
        }
        
        if arguments.contains("--no-battery-throttle") {
            config.throttleOnBattery = false
        }
        
        // Create and configure service
        let service = OCRExtractionService(config: config)
        
        // Setup status handler
        service.onStatusUpdate = { status in
            switch status {
            case .running:
                logger.info("Service is running")
            case .stopped:
                logger.info("Service stopped")
            case .paused:
                logger.info("Service paused")
            case .permissionRequired:
                logger.warning("Screen recording permission required")
            case .error(let message):
                logger.error("Service error: \(message)")
            }
        }
        
        // Setup signal handlers for graceful shutdown
        setupSignalHandlers(service: service)
        
        // Start the service
        print("🚀 Starting OCR Extraction Service...")
        print("   Mode: \(config.captureMode)")
        print("   Interval: \(config.baseCaptureInterval)s")
        print("   Battery throttle: \(config.throttleOnBattery)")
        print("")
        print("Press Ctrl+C to stop.")
        print("")
        
        service.start()
        
        // Keep the application running
        // Use RunLoop to allow async tasks and NSWorkspace notifications to work
        while true {
            RunLoop.current.run(mode: .default, before: Date.distantFuture)
        }
    }
    
    /// Process a single image file and output OCR results
    static func processImage(path: String, jsonOutput: Bool) async {
        // Load image from file
        guard let image = NSImage(contentsOfFile: path),
              let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            if jsonOutput {
                print("{\"error\": \"Failed to load image from path: \(path)\"}")
            } else {
                print("Error: Failed to load image from path: \(path)")
            }
            return
        }
        
        // Perform OCR
        let engine = OCREngine()
        do {
            let result = try await engine.extractText(from: cgImage)
            
            if jsonOutput {
                // Output as JSON for unified router
                let output: [String: Any] = [
                    "text": result.text,
                    "confidence": result.confidence,
                    "processingTime": result.processingTime,
                    "blockCount": result.blocks.count
                ]
                if let jsonData = try? JSONSerialization.data(withJSONObject: output),
                   let jsonString = String(data: jsonData, encoding: .utf8) {
                    print(jsonString)
                }
            } else {
                // Human-readable output
                print("OCR Result:")
                print("  Confidence: \(String(format: "%.1f", result.confidence * 100))%")
                print("  Processing time: \(String(format: "%.2f", result.processingTime))s")
                print("  Text blocks: \(result.blocks.count)")
                print("")
                print("--- Extracted Text ---")
                print(result.text)
            }
        } catch {
            if jsonOutput {
                print("{\"error\": \"\(error.localizedDescription)\"}")
            } else {
                print("Error: \(error.localizedDescription)")
            }
        }
    }
    
    /// Capture a specific window by ID and optionally perform OCR
    static func captureAndOCRWindow(windowId: UInt32, jsonOutput: Bool, captureOnly: Bool = false, outputPath: String? = nil) async {
        // Use ScreenCaptureKit to capture the window (macOS 14.0+)
        if #available(macOS 14.0, *) {
            await captureWindowWithScreenCaptureKit(windowId: windowId, jsonOutput: jsonOutput, captureOnly: captureOnly, outputPath: outputPath)
        } else {
            // Fallback to CGWindowListCreateImage for older macOS
            await captureWindowWithCGWindowList(windowId: windowId, jsonOutput: jsonOutput, captureOnly: captureOnly, outputPath: outputPath)
        }
    }
    
    @available(macOS 14.0, *)
    static func captureWindowWithScreenCaptureKit(windowId: UInt32, jsonOutput: Bool, captureOnly: Bool = false, outputPath: String? = nil) async {
        do {
            // Import ScreenCaptureKit dynamically
            let scContent = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            // Find the window with matching ID
            guard let window = scContent.windows.first(where: { $0.windowID == windowId }) else {
                if jsonOutput {
                    print("{\"error\": \"Window not found with ID: \(windowId)\"}")
                } else {
                    print("Error: Window not found with ID: \(windowId)")
                }
                return
            }
            
            // Create a filter for just this window
            let filter = SCContentFilter(desktopIndependentWindow: window)
            
            // Configure capture
            let config = SCStreamConfiguration()
            config.width = Int(window.frame.width) * 2  // Retina
            config.height = Int(window.frame.height) * 2
            config.showsCursor = false
            config.capturesAudio = false
            
            // Capture the window
            let image = try await SCScreenshotManager.captureImage(
                contentFilter: filter,
                configuration: config
            )
            
            // If capture-only mode, save image and return
            if captureOnly {
                let savePath = outputPath ?? "/tmp/ocr_capture_\(windowId).png"
                let nsImage = NSImage(cgImage: image, size: NSSize(width: image.width, height: image.height))
                
                guard let tiffData = nsImage.tiffRepresentation,
                      let bitmapRep = NSBitmapImageRep(data: tiffData),
                      let pngData = bitmapRep.representation(using: .png, properties: [:]) else {
                    if jsonOutput {
                        print("{\"error\": \"Failed to convert image to PNG\"}")
                    } else {
                        print("Error: Failed to convert image to PNG")
                    }
                    return
                }
                
                do {
                    try pngData.write(to: URL(fileURLWithPath: savePath))
                    if jsonOutput {
                        let output: [String: Any] = [
                            "captured": true,
                            "path": savePath,
                            "width": image.width,
                            "height": image.height,
                            "windowId": windowId,
                            "windowTitle": window.title ?? ""
                        ]
                        if let jsonData = try? JSONSerialization.data(withJSONObject: output),
                           let jsonString = String(data: jsonData, encoding: .utf8) {
                            print(jsonString)
                        }
                    } else {
                        print("Captured window \(windowId) to \(savePath)")
                    }
                } catch {
                    if jsonOutput {
                        print("{\"error\": \"Failed to save image: \(error.localizedDescription)\"}")
                    } else {
                        print("Error: Failed to save image: \(error.localizedDescription)")
                    }
                }
                return
            }
            
            // Perform OCR
            let engine = OCREngine()
            let result = try await engine.extractText(from: image)
            
            if jsonOutput {
                let output: [String: Any] = [
                    "text": result.text,
                    "confidence": result.confidence,
                    "processingTime": result.processingTime,
                    "blockCount": result.blocks.count,
                    "windowId": windowId,
                    "windowTitle": window.title ?? ""
                ]
                if let jsonData = try? JSONSerialization.data(withJSONObject: output),
                   let jsonString = String(data: jsonData, encoding: .utf8) {
                    print(jsonString)
                }
            } else {
                print("OCR Result for window \(windowId) (\(window.title ?? "untitled")):")
                print("  Confidence: \(String(format: "%.1f", result.confidence * 100))%")
                print("  Processing time: \(String(format: "%.2f", result.processingTime))s")
                print("  Text blocks: \(result.blocks.count)")
                print("")
                print("--- Extracted Text ---")
                print(result.text)
            }
        } catch {
            if jsonOutput {
                print("{\"error\": \"\(error.localizedDescription)\"}")
            } else {
                print("Error: \(error.localizedDescription)")
            }
        }
    }
    
    static func captureWindowWithCGWindowList(windowId: UInt32, jsonOutput: Bool, captureOnly: Bool = false, outputPath: String? = nil) async {
        // Get window bounds
        let options: CGWindowListOption = [.optionIncludingWindow]
        guard let windowList = CGWindowListCopyWindowInfo(options, CGWindowID(windowId)) as? [[String: Any]],
              let windowInfo = windowList.first,
              let boundsDict = windowInfo[kCGWindowBounds as String] as? [String: CGFloat] else {
            if jsonOutput {
                print("{\"error\": \"Failed to get window info for ID: \(windowId)\"}")
            } else {
                print("Error: Failed to get window info for ID: \(windowId)")
            }
            return
        }
        
        let bounds = CGRect(
            x: boundsDict["X"] ?? 0,
            y: boundsDict["Y"] ?? 0,
            width: boundsDict["Width"] ?? 0,
            height: boundsDict["Height"] ?? 0
        )
        
        // Capture the window
        guard let image = CGWindowListCreateImage(
            bounds,
            .optionIncludingWindow,
            CGWindowID(windowId),
            [.boundsIgnoreFraming, .bestResolution]
        ) else {
            if jsonOutput {
                print("{\"error\": \"Failed to capture window ID: \(windowId)\"}")
            } else {
                print("Error: Failed to capture window ID: \(windowId)")
            }
            return
        }
        
        // If capture-only mode, save image and return
        if captureOnly {
            let savePath = outputPath ?? "/tmp/ocr_capture_\(windowId).png"
            let nsImage = NSImage(cgImage: image, size: NSSize(width: image.width, height: image.height))
            
            guard let tiffData = nsImage.tiffRepresentation,
                  let bitmapRep = NSBitmapImageRep(data: tiffData),
                  let pngData = bitmapRep.representation(using: .png, properties: [:]) else {
                if jsonOutput {
                    print("{\"error\": \"Failed to convert image to PNG\"}")
                } else {
                    print("Error: Failed to convert image to PNG")
                }
                return
            }
            
            do {
                try pngData.write(to: URL(fileURLWithPath: savePath))
                if jsonOutput {
                    let output: [String: Any] = [
                        "captured": true,
                        "path": savePath,
                        "width": image.width,
                        "height": image.height,
                        "windowId": windowId
                    ]
                    if let jsonData = try? JSONSerialization.data(withJSONObject: output),
                       let jsonString = String(data: jsonData, encoding: .utf8) {
                        print(jsonString)
                    }
                } else {
                    print("Captured window \(windowId) to \(savePath)")
                }
            } catch {
                if jsonOutput {
                    print("{\"error\": \"Failed to save image: \(error.localizedDescription)\"}")
                } else {
                    print("Error: Failed to save image: \(error.localizedDescription)")
                }
            }
            return
        }
        
        // Perform OCR
        let engine = OCREngine()
        do {
            let result = try await engine.extractText(from: image)
            
            if jsonOutput {
                let output: [String: Any] = [
                    "text": result.text,
                    "confidence": result.confidence,
                    "processingTime": result.processingTime,
                    "blockCount": result.blocks.count,
                    "windowId": windowId
                ]
                if let jsonData = try? JSONSerialization.data(withJSONObject: output),
                   let jsonString = String(data: jsonData, encoding: .utf8) {
                    print(jsonString)
                }
            } else {
                print("OCR Result for window \(windowId):")
                print("  Confidence: \(String(format: "%.1f", result.confidence * 100))%")
                print("  Processing time: \(String(format: "%.2f", result.processingTime))s")
                print("  Text blocks: \(result.blocks.count)")
                print("")
                print("--- Extracted Text ---")
                print(result.text)
            }
        } catch {
            if jsonOutput {
                print("{\"error\": \"\(error.localizedDescription)\"}")
            } else {
                print("Error: \(error.localizedDescription)")
            }
        }
    }
    
    static func printUsage() {
        print("""
        OCR Extractor - Screen content extraction service
        
        USAGE:
            ocr-extractor [OPTIONS]
        
        OPTIONS:
            --mode <MODE>       Capture mode: window, display, all (default: all)
            --interval <SECS>   Base capture interval in seconds (default: 10)
            --no-battery-throttle   Don't slow down on battery power
            --image <PATH>      Process a single image file (one-shot mode)
            --window-id <ID>    Capture and OCR a specific window by ID
            --capture-only      Only capture window, don't run OCR (use with --window-id)
            --output <PATH>     Output path for captured image (use with --capture-only)
            --json              Output results as JSON (for --image/--window-id mode)
            --help, -h          Show this help message
            --version, -v       Show version
        
        EXAMPLES:
            ocr-extractor                           # Start continuous service
            ocr-extractor --mode display            # Capture active display
            ocr-extractor --interval 5              # Capture every 5 seconds
            ocr-extractor --mode all --interval 15  # All displays, 15s interval
            ocr-extractor --image screenshot.png    # OCR a single image
            ocr-extractor --image shot.png --json   # OCR with JSON output
            ocr-extractor --window-id 12345 --json  # Capture window and OCR
            ocr-extractor --window-id 12345 --capture-only --json  # Capture only
        
        PERMISSIONS:
            This application requires Screen Recording permission for continuous mode.
            Grant access in System Settings > Privacy & Security > Screen Recording
        
        NOTES:
            - Continuous mode sends captures to the ingestion service via Unix socket
            - Single image mode (--image) outputs directly to stdout
            - Window ID mode (--window-id) captures a specific window using ScreenCaptureKit
            - Capture-only mode (--capture-only) saves image without OCR for change detection
            - Sensitive apps (password managers, banking) are automatically skipped
            - Content is deduplicated to avoid redundant processing
        """)
    }
    
    static func setupSignalHandlers(service: OCRExtractionService) {
        // Handle SIGINT (Ctrl+C)
        signal(SIGINT) { _ in
            print("\n🛑 Shutting down...")
            exit(0)
        }
        
        // Handle SIGTERM
        signal(SIGTERM) { _ in
            print("\n🛑 Received SIGTERM, shutting down...")
            exit(0)
        }
    }
}
