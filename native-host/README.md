# Native Messaging Host Setup

This directory contains the native messaging host manifest that tells Chrome how to communicate with your Tauri application.

## Installation

### macOS

Copy the manifest to Chrome's native messaging hosts directory:

```bash
# For current user only
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
cp com.clace.extension.json ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# For all users (requires sudo)
sudo mkdir -p /Library/Google/Chrome/NativeMessagingHosts/
sudo cp com.clace.extension.json /Library/Google/Chrome/NativeMessagingHosts/
```

### Linux

```bash
# For current user only
mkdir -p ~/.config/google-chrome/NativeMessagingHosts/
cp com.clace.extension.json ~/.config/google-chrome/NativeMessagingHosts/

# For all users (requires sudo)
sudo mkdir -p /etc/opt/chrome/native-messaging-hosts/
sudo cp com.clace.extension.json /etc/opt/chrome/native-messaging-hosts/
```

### Windows

1. Place the manifest JSON file somewhere permanent (e.g., `C:\Program Files\YourApp\`)
2. Add a registry key pointing to it:

```
HKEY_CURRENT_USER\Software\Google\Chrome\NativeMessagingHosts\com.clace.extension
```

Set the default value to the full path of the JSON file.

## Configuration

Before installing, update the manifest:

1. **path**: Set to the absolute path of your Tauri native host binary
2. **allowed_origins**: Replace `YOUR_EXTENSION_ID_HERE` with your actual extension ID

To find your extension ID:
1. Load the unpacked extension in Chrome (`chrome://extensions`)
2. Enable "Developer mode"
3. The ID will be shown under the extension name

## Testing

After installation, you can verify the setup by:
1. Loading the extension in Chrome
2. Opening the browser console (F12 on any page)
3. Checking for connection messages from the offscreen document
