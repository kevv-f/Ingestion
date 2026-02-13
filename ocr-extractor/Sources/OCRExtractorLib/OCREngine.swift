// OCREngine.swift
// Processes captured images through Apple's Vision framework

import Foundation
import Vision
import CoreGraphics
import os.log

/// Engine for extracting text from images using Apple Vision
public class OCREngine {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Recognition accuracy level
        public var recognitionLevel: VNRequestTextRecognitionLevel = .accurate
        /// Enable language correction
        public var usesLanguageCorrection: Bool = true
        /// Languages to recognize
        public var recognitionLanguages: [String] = ["en-US"]
        /// Minimum text height (0 = no minimum)
        public var minimumTextHeight: Float = 0.0
        /// Custom vocabulary for domain-specific terms
        public var customWords: [String] = []
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "OCREngine")
    private var config: Config
    
    // Processing queue with low priority
    private let processingQueue = DispatchQueue(
        label: "com.clace.ocr.processing",
        qos: .background
    )
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    // MARK: - Configuration
    
    /// Update OCR configuration
    public func updateConfig(_ config: Config) {
        self.config = config
    }
    
    /// Get supported languages
    public static func supportedLanguages() -> [String] {
        // Vision framework supported languages
        return (try? VNRecognizeTextRequest.supportedRecognitionLanguages(
            for: .accurate,
            revision: VNRecognizeTextRequestRevision3
        )) ?? ["en-US"]
    }
    
    // MARK: - OCR Processing
    
    /// Extract text from an image
    public func extractText(from image: CGImage) async throws -> OCRResult {
        let startTime = Date()
        
        return try await withCheckedThrowingContinuation { continuation in
            processingQueue.async {
                do {
                    let result = try self.performOCR(on: image, startTime: startTime)
                    continuation.resume(returning: result)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
    
    /// Extract text from a capture result
    public func extractText(from capture: CaptureResult) async throws -> OCRResult {
        var result = try await extractText(from: capture.image)
        result.windowTitle = capture.windowTitle
        result.applicationName = capture.applicationName
        result.applicationBundleID = capture.applicationBundleID
        result.displayID = capture.displayID
        return result
    }
    
    // MARK: - Private Implementation
    
    private func performOCR(on image: CGImage, startTime: Date) throws -> OCRResult {
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = config.recognitionLevel
        request.usesLanguageCorrection = config.usesLanguageCorrection
        request.recognitionLanguages = config.recognitionLanguages
        request.minimumTextHeight = config.minimumTextHeight
        
        if !config.customWords.isEmpty {
            request.customWords = config.customWords
        }
        
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        try handler.perform([request])
        
        guard let observations = request.results else {
            return OCRResult(
                text: "",
                blocks: [],
                confidence: 0,
                processingTime: Date().timeIntervalSince(startTime),
                imageSize: CGSize(width: image.width, height: image.height)
            )
        }
        
        // Extract text blocks with position information
        var blocks: [TextBlock] = []
        var fullText: [String] = []
        var totalConfidence: Float = 0
        
        for observation in observations {
            guard let topCandidate = observation.topCandidates(1).first else {
                continue
            }
            
            let block = TextBlock(
                text: topCandidate.string,
                confidence: topCandidate.confidence,
                boundingBox: observation.boundingBox
            )
            
            blocks.append(block)
            fullText.append(topCandidate.string)
            totalConfidence += topCandidate.confidence
        }
        
        let averageConfidence = blocks.isEmpty ? 0 : totalConfidence / Float(blocks.count)
        let processingTime = Date().timeIntervalSince(startTime)
        
        logger.debug("OCR completed: \(blocks.count) blocks, \(String(format: "%.1f", averageConfidence * 100))% confidence, \(String(format: "%.2f", processingTime))s")
        
        return OCRResult(
            text: fullText.joined(separator: "\n"),
            blocks: blocks,
            confidence: averageConfidence,
            processingTime: processingTime,
            imageSize: CGSize(width: image.width, height: image.height)
        )
    }
    
    // MARK: - Batch Processing
    
    /// Process multiple captures in parallel
    public func extractTextBatch(from captures: [CaptureResult]) async -> [OCRResult] {
        await withTaskGroup(of: OCRResult?.self) { group in
            for capture in captures {
                group.addTask {
                    try? await self.extractText(from: capture)
                }
            }
            
            var results: [OCRResult] = []
            for await result in group {
                if let result = result {
                    results.append(result)
                }
            }
            return results
        }
    }
    
    // MARK: - Region-Based OCR
    
    /// Detect text regions before full OCR (faster for filtering)
    public func detectTextRegions(in image: CGImage) async throws -> [CGRect] {
        return try await withCheckedThrowingContinuation { continuation in
            processingQueue.async {
                do {
                    let request = VNDetectTextRectanglesRequest()
                    request.reportCharacterBoxes = false
                    
                    let handler = VNImageRequestHandler(cgImage: image, options: [:])
                    try handler.perform([request])
                    
                    let regions = request.results?.map { $0.boundingBox } ?? []
                    continuation.resume(returning: regions)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
    
    /// OCR only specific regions of interest
    public func extractTextFromRegions(
        image: CGImage,
        regions: [CGRect]
    ) async throws -> [OCRResult] {
        var results: [OCRResult] = []
        
        for region in regions {
            // Convert normalized coordinates to pixel coordinates
            let pixelRect = CGRect(
                x: region.origin.x * CGFloat(image.width),
                y: region.origin.y * CGFloat(image.height),
                width: region.width * CGFloat(image.width),
                height: region.height * CGFloat(image.height)
            )
            
            guard let croppedImage = image.cropping(to: pixelRect) else {
                continue
            }
            
            let result = try await extractText(from: croppedImage)
            results.append(result)
        }
        
        return results
    }
}

// MARK: - OCR Errors

public enum OCRError: Error, LocalizedError {
    case imageProcessingFailed
    case noTextFound
    case recognitionFailed(String)
    
    public var errorDescription: String? {
        switch self {
        case .imageProcessingFailed:
            return "Failed to process image for OCR"
        case .noTextFound:
            return "No text found in image"
        case .recognitionFailed(let message):
            return "OCR recognition failed: \(message)"
        }
    }
}
