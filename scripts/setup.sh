#!/bin/bash

# Content Ingestion Pipeline - Setup Script
# Run this after building the native host

set -e

echo "=== Content Ingestion Pipeline Setup ==="
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Darwin*)    PLATFORM="macos";;
    Linux*)     PLATFORM="linux";;
    MINGW*|MSYS*|CYGWIN*)    PLATFORM="windows";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

echo "Detected platform: $PLATFORM"
echo ""

# Get the directory where this script lives
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
NATIVE_HOST_DIR="$PROJECT_DIR/native-host"
MANIFEST_FILE="$NATIVE_HOST_DIR/com.clace.extension.json"

# Check if native host is built
if [ "$PLATFORM" = "windows" ]; then
    BINARY_PATH="$NATIVE_HOST_DIR/target/release/ingestion-host.exe"
else
    BINARY_PATH="$NATIVE_HOST_DIR/target/release/ingestion-host"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "Native host binary not found at: $BINARY_PATH"
    echo ""
    echo "Build it first with:"
    echo "  cd native-host"
    echo "  cargo build --release"
    echo ""
    exit 1
fi

echo "Found native host binary: $BINARY_PATH"
echo ""

# Prompt for extension ID
echo "You need your Chrome extension ID."
echo "To get it:"
echo "  1. Open Chrome and go to chrome://extensions"
echo "  2. Enable 'Developer mode' (toggle in top right)"
echo "  3. Click 'Load unpacked' and select: $PROJECT_DIR/chrome-extension"
echo "  4. Copy the ID shown under the extension name"
echo ""
read -p "Enter your extension ID: " EXTENSION_ID

if [ -z "$EXTENSION_ID" ]; then
    echo "Extension ID is required"
    exit 1
fi

# Create the manifest with correct values
MANIFEST_CONTENT=$(cat <<EOF
{
  "name": "com.clace.extension",
  "description": "Clace content ingestion native messaging host",
  "path": "$BINARY_PATH",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://$EXTENSION_ID/"
  ]
}
EOF
)

echo ""
echo "Generated manifest:"
echo "$MANIFEST_CONTENT"
echo ""

# Determine target directory based on platform
case "$PLATFORM" in
    macos)
        TARGET_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
        ;;
    linux)
        TARGET_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
        ;;
    windows)
        echo "Windows requires manual registry setup. See native-host/README.md"
        exit 1
        ;;
esac

# Create target directory if needed
mkdir -p "$TARGET_DIR"

# Write the manifest
TARGET_FILE="$TARGET_DIR/com.clace.extension.json"
echo "$MANIFEST_CONTENT" > "$TARGET_FILE"

echo "Manifest installed to: $TARGET_FILE"
echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "  1. Go to chrome://extensions"
echo "  2. Click the refresh icon on your extension (or reload it)"
echo "  3. Open any website and switch tabs"
echo "  4. Check the browser console (F12) for extraction logs"
echo "  5. Check ingested data at:"

case "$PLATFORM" in
    macos)
        echo "     ~/Library/Application Support/ingestion-pipeline/"
        ;;
    linux)
        echo "     ~/.local/share/ingestion-pipeline/"
        ;;
esac

echo ""
