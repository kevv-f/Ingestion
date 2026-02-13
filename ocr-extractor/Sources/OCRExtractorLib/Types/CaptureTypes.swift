// CaptureTypes.swift
// Core types for screen capture and OCR processing

import Foundation
import CoreGraphics

// MARK: - Capture Context

/// Context information about the current capture environment
public struct CaptureContext {
    public let timestamp: Date
    public let applicationName: String?
    public let applicationBundleID: String?
    public let windowTitle: String?
    public let idleTime: TimeInterval
    public let displayCount: Int
    
    public init(
        timestamp: Date = Date(),
        applicationName: String? = nil,
        applicationBundleID: String? = nil,
        windowTitle: String? = nil,
        idleTime: TimeInterval = 0,
        displayCount: Int = 1
    ) {
        self.timestamp = timestamp
        self.applicationName = applicationName
        self.applicationBundleID = applicationBundleID
        self.windowTitle = windowTitle
        self.idleTime = idleTime
        self.displayCount = displayCount
    }
}

// MARK: - Capture Result

/// Result of a screen capture operation
public struct CaptureResult {
    public let image: CGImage
    public let displayID: CGDirectDisplayID
    public let bounds: CGRect
    public let windowTitle: String?
    public let applicationName: String?
    public let applicationBundleID: String?
    public let timestamp: Date
    
    public var width: Int { image.width }
    public var height: Int { image.height }
    
    public init(
        image: CGImage,
        displayID: CGDirectDisplayID,
        bounds: CGRect,
        windowTitle: String? = nil,
        applicationName: String? = nil,
        applicationBundleID: String? = nil,
        timestamp: Date = Date()
    ) {
        self.image = image
        self.displayID = displayID
        self.bounds = bounds
        self.windowTitle = windowTitle
        self.applicationName = applicationName
        self.applicationBundleID = applicationBundleID
        self.timestamp = timestamp
    }
}

// MARK: - Display Info

/// Information about a connected display
public struct DisplayInfo {
    public let id: CGDirectDisplayID
    public let name: String
    public let bounds: CGRect
    public let isMain: Bool
    public let isBuiltin: Bool
    public let scaleFactor: CGFloat
    
    public var resolution: String {
        "\(Int(bounds.width * scaleFactor))x\(Int(bounds.height * scaleFactor))"
    }
    
    public init(
        id: CGDirectDisplayID,
        name: String,
        bounds: CGRect,
        isMain: Bool,
        isBuiltin: Bool,
        scaleFactor: CGFloat
    ) {
        self.id = id
        self.name = name
        self.bounds = bounds
        self.isMain = isMain
        self.isBuiltin = isBuiltin
        self.scaleFactor = scaleFactor
    }
}

// MARK: - OCR Result

/// Result of OCR text extraction
public struct OCRResult {
    public var text: String
    public var blocks: [TextBlock]
    public var confidence: Float
    public var processingTime: TimeInterval
    public var imageSize: CGSize
    
    // Metadata from capture
    public var windowTitle: String?
    public var applicationName: String?
    public var applicationBundleID: String?
    public var displayID: CGDirectDisplayID?
    
    public init(
        text: String,
        blocks: [TextBlock] = [],
        confidence: Float = 0,
        processingTime: TimeInterval = 0,
        imageSize: CGSize = .zero,
        windowTitle: String? = nil,
        applicationName: String? = nil,
        applicationBundleID: String? = nil,
        displayID: CGDirectDisplayID? = nil
    ) {
        self.text = text
        self.blocks = blocks
        self.confidence = confidence
        self.processingTime = processingTime
        self.imageSize = imageSize
        self.windowTitle = windowTitle
        self.applicationName = applicationName
        self.applicationBundleID = applicationBundleID
        self.displayID = displayID
    }
}

/// A block of text with position information
public struct TextBlock {
    public let text: String
    public let confidence: Float
    public let boundingBox: CGRect  // Normalized (0-1) coordinates
    
    public init(text: String, confidence: Float, boundingBox: CGRect) {
        self.text = text
        self.confidence = confidence
        self.boundingBox = boundingBox
    }
}

// MARK: - Capture Payload (for ingestion service)

/// Payload format compatible with the ingestion service
public struct CapturePayload: Codable {
    public let source: String
    public let url: String
    public let content: String
    public let title: String?
    public let author: String?
    public let channel: String?
    public let timestamp: Int?
    
    public init(
        source: String,
        url: String,
        content: String,
        title: String? = nil,
        author: String? = nil,
        channel: String? = nil,
        timestamp: Int? = nil
    ) {
        self.source = source
        self.url = url
        self.content = content
        self.title = title
        self.author = author
        self.channel = channel
        self.timestamp = timestamp
    }
}

// MARK: - Ingestion Response

/// Response from the ingestion service
public struct IngestionResponse: Codable {
    public let status: String
    public let action: String
    public let ehl_doc_id: String?
    public let chunk_count: Int?
    public let message: String?
    
    public var isSuccess: Bool {
        status == "ok"
    }
}
