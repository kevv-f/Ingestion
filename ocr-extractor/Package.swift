// swift-tools-version: 5.9
// OCR Extraction Service for macOS

import PackageDescription

let package = Package(
    name: "OCRExtractor",
    platforms: [
        .macOS(.v13)  // Requires macOS 13+ for modern Vision APIs
    ],
    products: [
        .executable(name: "ocr-extractor", targets: ["OCRExtractor"]),
        .library(name: "OCRExtractorLib", targets: ["OCRExtractorLib"])
    ],
    dependencies: [],
    targets: [
        .executableTarget(
            name: "OCRExtractor",
            dependencies: ["OCRExtractorLib"],
            path: "Sources/OCRExtractor"
        ),
        .target(
            name: "OCRExtractorLib",
            dependencies: [],
            path: "Sources/OCRExtractorLib"
        ),
        .testTarget(
            name: "OCRExtractorTests",
            dependencies: ["OCRExtractorLib"],
            path: "Tests/OCRExtractorTests"
        )
    ]
)
