#!/bin/bash
# Build all Ingestion components
# Usage: ./scripts/build-all.sh [--release]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

BUILD_TYPE="debug"
CARGO_FLAGS=""

if [[ "$1" == "--release" ]]; then
    BUILD_TYPE="release"
    CARGO_FLAGS="--release"
fi

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║           Building Ingestion Components ($BUILD_TYPE)              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Build Rust components
echo "🔨 Building Unified Router..."
cd "$PROJECT_ROOT/unified-router"
cargo build $CARGO_FLAGS
echo "   ✅ unified-router built"
echo "   ✅ ingestion built"

echo ""
echo "🔨 Building Ingestion Service..."
cd "$PROJECT_ROOT/ingestion-service"
cargo build $CARGO_FLAGS
echo "   ✅ ingestion-server built"

echo ""
echo "🔨 Building Accessibility Extractor..."
cd "$PROJECT_ROOT/accessibility-extractor"
cargo build $CARGO_FLAGS
echo "   ✅ ax-extractor built"
echo "   ✅ ax-daemon built"

echo ""
echo "🔨 Building Native Host..."
cd "$PROJECT_ROOT/native-host"
cargo build $CARGO_FLAGS
echo "   ✅ ingestion-host built"

# Build Swift OCR extractor
echo ""
echo "🔨 Building OCR Extractor..."
cd "$PROJECT_ROOT/ocr-extractor"
if [[ "$BUILD_TYPE" == "release" ]]; then
    swift build -c release
else
    swift build
fi
echo "   ✅ OCRExtractor built"

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                    Build Complete!                           ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Binaries location:"
echo "  unified-router/target/$BUILD_TYPE/ingestion"
echo "  unified-router/target/$BUILD_TYPE/unified-router"
echo "  ingestion-service/target/$BUILD_TYPE/ingestion-server"
echo "  accessibility-extractor/target/$BUILD_TYPE/ax-extractor"
echo "  accessibility-extractor/target/$BUILD_TYPE/ax-daemon"
echo "  native-host/target/$BUILD_TYPE/ingestion-host"
echo "  ocr-extractor/.build/$BUILD_TYPE/OCRExtractor"
echo ""
echo "To run the unified ingestion service:"
echo "  ./unified-router/target/$BUILD_TYPE/ingestion"
echo ""
