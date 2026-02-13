/**
 * Offscreen Document - Relay for native messaging
 * 
 * Note: chrome.runtime.connectNative is NOT available in offscreen documents.
 * This document now just relays messages to the service worker, which handles
 * the native messaging connection.
 */

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'send-to-native') {
    // Relay back to service worker to handle native messaging
    // This is a no-op now - the service worker handles native messaging directly
    console.log('[Offscreen] Received data, but native messaging must be done in service worker');
    sendResponse({ success: false, reason: 'Native messaging not available in offscreen document' });
    return false;
  }

  return false;
});

console.log('[Offscreen] Initialized (relay mode)');
