/**
 * Service Worker - Handles tab focus detection, extraction, and native messaging
 * 
 * This is ephemeral - Chrome kills it after ~30s of inactivity.
 * Native messaging uses sendNativeMessage (one-shot) since connectNative
 * connections die when the service worker sleeps.
 * 
 * Triggers extraction on:
 * 1. Tab activation (user switches tabs)
 * 2. Window focus change (user switches windows)
 * 3. URL change within same tab (SPA navigation, e.g., clicking issues in Jira)
 * 4. Page load complete (initial page load)
 */

const NATIVE_HOST_NAME = 'com.yourapp.ingestion_host';

// Track the last processed tab+url to avoid duplicate extractions
let lastProcessedTabId = null;
let lastProcessedUrl = null;
let lastProcessedTime = 0;

// Track extracted content hashes to avoid re-processing unchanged content
const extractedContentCache = new Map(); // url -> { hash, timestamp }
const CONTENT_CACHE_TTL_MS = 300000; // 5 minutes - don't re-extract same URL within this window

// Debounce time in ms - prevents rapid-fire extractions during SPA navigation
const DEBOUNCE_MS = 1000;

// Longer debounce for Google products (they trigger many URL changes)
const GOOGLE_DEBOUNCE_MS = 5000;

// UNIFIED rate limiting for ALL Google exports (to avoid 429 errors)
// Google has a global rate limit, not per-product
let googleExportLastFetch = 0;
const GOOGLE_EXPORT_COOLDOWN_MS = 10000; // 10 seconds between ANY Google export
const GOOGLE_RATE_LIMIT_PENALTY_MS = 30000; // 30 second penalty after 429

/**
 * Generate a simple hash for URL-based caching
 */
function getUrlCacheKey(url) {
  // For Google Sheets, normalize to spreadsheet ID + gid
  if (url.includes('docs.google.com/spreadsheets')) {
    const match = url.match(/\/spreadsheets\/d\/([a-zA-Z0-9_-]+)/);
    const gidMatch = url.match(/gid=(\d+)/);
    if (match) {
      return `gsheets:${match[1]}:${gidMatch?.[1] || '0'}`;
    }
  }
  // For Google Docs, normalize to doc ID
  if (url.includes('docs.google.com/document')) {
    const match = url.match(/\/document\/d\/([a-zA-Z0-9_-]+)/);
    if (match) {
      return `gdocs:${match[1]}`;
    }
  }
  // For Google Slides, normalize to presentation ID
  if (url.includes('docs.google.com/presentation')) {
    const match = url.match(/\/presentation\/d\/([a-zA-Z0-9_-]+)/);
    if (match) {
      return `gslides:${match[1]}`;
    }
  }
  // For Gemini, normalize to conversation ID
  if (url.includes('gemini.google.com')) {
    const convMatch = url.match(/\/(?:app|c)\/([a-zA-Z0-9_-]+)/);
    if (convMatch) {
      return `gemini:${convMatch[1]}`;
    }
    // No conversation ID means new conversation - use shorter cache
    return `gemini:new:${Date.now()}`;
  }
  // For Google Search/AI Mode, normalize to search query
  if (url.includes('google.com/search')) {
    try {
      const urlObj = new URL(url);
      const query = urlObj.searchParams.get('q');
      const isAIMode = urlObj.searchParams.get('udm') === '50';
      if (query) {
        const prefix = isAIMode ? 'google-ai' : 'google-search';
        return `${prefix}:${query.toLowerCase().trim()}`;
      }
    } catch (e) {
      // Fall through to default
    }
  }
  return url;
}

/**
 * Check if we should skip extraction based on recent cache
 */
function shouldSkipExtraction(url) {
  const cacheKey = getUrlCacheKey(url);
  const cached = extractedContentCache.get(cacheKey);
  
  if (!cached) return false;
  
  // Use shorter TTL for conversational content (Gemini, AI Mode)
  // These change frequently as users interact
  const isConversational = cacheKey.startsWith('gemini:') || cacheKey.startsWith('google-ai:');
  const effectiveTTL = isConversational ? 30000 : CONTENT_CACHE_TTL_MS; // 30s for conversations, 5min for others
  
  if ((Date.now() - cached.timestamp) < effectiveTTL) {
    return true; // Recently extracted, skip
  }
  return false;
}

/**
 * Mark URL as recently extracted
 */
function markExtracted(url) {
  const cacheKey = getUrlCacheKey(url);
  extractedContentCache.set(cacheKey, { timestamp: Date.now() });
  
  // Clean old entries periodically
  if (extractedContentCache.size > 100) {
    const now = Date.now();
    for (const [key, value] of extractedContentCache) {
      if (now - value.timestamp > CONTENT_CACHE_TTL_MS * 2) {
        extractedContentCache.delete(key);
      }
    }
  }
}

/**
 * Send data to native host using one-shot messaging
 */
async function sendToNativeHost(data) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage(NATIVE_HOST_NAME, data, (response) => {
      if (chrome.runtime.lastError) {
        console.error('[ServiceWorker] Native messaging error:', chrome.runtime.lastError.message);
        reject(new Error(chrome.runtime.lastError.message));
      } else {
        console.log('[ServiceWorker] Native host response:', response);
        resolve(response);
      }
    });
  });
}

/**
 * Ensure content script is injected into the tab
 */
async function ensureContentScript(tabId) {
  try {
    // Try to ping the content script
    await chrome.tabs.sendMessage(tabId, { type: 'ping' });
    return true; // Content script is already there
  } catch (error) {
    // Content script not present, inject it
    console.log('[ServiceWorker] Injecting content script into tab', tabId);
    try {
      await chrome.scripting.executeScript({
        target: { tabId: tabId, allFrames: false },
        files: ['content/index.js']
      });
      // Give it a moment to initialize
      await new Promise(resolve => setTimeout(resolve, 100));
      return true;
    } catch (injectError) {
      console.error('[ServiceWorker] Failed to inject content script:', injectError.message);
      return false;
    }
  }
}

/**
 * Send message to content script with retry logic
 */
async function sendMessageWithRetry(tabId, message, maxRetries = 3) {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const response = await chrome.tabs.sendMessage(tabId, message);
      return response;
    } catch (error) {
      if (attempt === maxRetries) {
        throw error;
      }
      await new Promise(resolve => setTimeout(resolve, 100 * Math.pow(2, attempt - 1)));
      console.log(`[ServiceWorker] Retry ${attempt}/${maxRetries} for tab ${tabId}`);
    }
  }
}

/**
 * Handle tab activation - main entry point for extraction
 */
async function handleTabActivated(activeInfo) {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    await processTab(tab, 'activated');
  } catch (error) {
    console.error('[ServiceWorker] Error handling tab activation:', error.message);
  }
}

/**
 * Handle URL changes within the same tab (SPA navigation)
 * This catches when users click around in Jira, Gmail, Slack, etc.
 */
function handleTabUpdated(_tabId, changeInfo, tab) {
  // Only process URL changes, not other tab updates
  if (!changeInfo.url) {
    return;
  }
  
  // Only process if this is the active tab
  if (!tab.active) {
    return;
  }
  
  console.log('[ServiceWorker] URL changed in active tab:', changeInfo.url);
  processTab(tab, 'url-changed');
}

/**
 * Core tab processing logic - extracts content and sends to native host
 */
async function processTab(tab, trigger) {
  try {
    // Skip chrome:// urls, extension pages, etc.
    if (!tab.url || !tab.url.startsWith('http')) {
      console.log('[ServiceWorker] Skipping non-http tab:', tab.url);
      return;
    }

    // Skip Google Docs home/picker pages (no document ID)
    if (tab.url.includes('docs.google.com/document') && !tab.url.includes('/document/d/')) {
      console.log('[ServiceWorker] Skipping Google Docs home page:', tab.url);
      return;
    }

    // Skip Google Sheets home/picker pages (no spreadsheet ID)
    if (tab.url.includes('docs.google.com/spreadsheets') && !tab.url.includes('/spreadsheets/d/')) {
      console.log('[ServiceWorker] Skipping Google Sheets home page:', tab.url);
      return;
    }

    // Skip Google Slides home/picker pages (no presentation ID)
    if (tab.url.includes('docs.google.com/presentation') && !tab.url.includes('/presentation/d/')) {
      console.log('[ServiceWorker] Skipping Google Slides home page:', tab.url);
      return;
    }

    const now = Date.now();
    const isGoogleProduct = tab.url.includes('docs.google.com');
    const effectiveDebounce = isGoogleProduct ? GOOGLE_DEBOUNCE_MS : DEBOUNCE_MS;
    
    // Skip if we just processed this exact tab+url combo (with debounce)
    if (tab.id === lastProcessedTabId && tab.url === lastProcessedUrl) {
      if (now - lastProcessedTime < effectiveDebounce) {
        console.log('[ServiceWorker] Skipping duplicate (debounced):', tab.url);
        return;
      }
    }
    
    // Skip if we recently extracted this URL (content-based cache)
    if (shouldSkipExtraction(tab.url)) {
      console.log('[ServiceWorker] Skipping (recently extracted):', getUrlCacheKey(tab.url));
      return;
    }

    lastProcessedTabId = tab.id;
    lastProcessedUrl = tab.url;
    lastProcessedTime = now;

    console.log(`[ServiceWorker] Processing tab (${trigger}):`, tab.url);

    // Ensure content script is injected
    const scriptReady = await ensureContentScript(tab.id);
    if (!scriptReady) {
      console.log('[ServiceWorker] Could not inject content script, skipping');
      return;
    }

    // For URL changes in SPAs, give the page time to update its content
    // Google Docs/Sheets needs extra time as they're heavy SPAs
    if (trigger === 'url-changed') {
      const isGoogleDocs = tab.url.includes('docs.google.com/document');
      const isGoogleSheets = tab.url.includes('docs.google.com/spreadsheets');
      const delay = (isGoogleDocs || isGoogleSheets) ? 2000 : 500;
      await new Promise(resolve => setTimeout(resolve, delay));
    }

    // Request extraction from content script
    const response = await sendMessageWithRetry(tab.id, {
      type: 'extract',
      url: tab.url,
      tabId: tab.id
    });

    if (response && response.success) {
      console.log('[ServiceWorker] Extraction successful, sending to native host');
      
      // Mark as extracted to prevent rapid re-extraction
      markExtracted(tab.url);
      
      try {
        const nativeResponse = await sendToNativeHost(response.data);
        console.log('[ServiceWorker] Native host processed:', nativeResponse?.action || 'unknown');
      } catch (nativeError) {
        console.error('[ServiceWorker] Failed to send to native host:', nativeError.message);
      }
    } else {
      console.log('[ServiceWorker] Extraction skipped or failed:', response?.reason);
    }

  } catch (error) {
    console.error('[ServiceWorker] Error processing tab:', error.message);
  }
}

/**
 * Handle window focus changes - user might switch between windows
 */
async function handleWindowFocusChanged(windowId) {
  if (windowId === chrome.windows.WINDOW_ID_NONE) {
    return; // Chrome lost focus entirely
  }

  try {
    const [activeTab] = await chrome.tabs.query({ active: true, windowId });
    if (activeTab) {
      await processTab(activeTab, 'window-focus');
    }
  } catch (error) {
    console.error('[ServiceWorker] Error handling window focus:', error.message);
  }
}

/**
 * Handle messages from content scripts
 * This is needed for cross-origin fetches that content scripts can't do
 */
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === 'fetchGoogleDoc') {
    // Fetch Google Doc content from service worker (avoids CORS issues)
    fetchGoogleDocExport(message.docId)
      .then(content => {
        sendResponse({ success: true, content: content });
      })
      .catch(error => {
        console.error('[ServiceWorker] Google Doc fetch error:', error.message);
        sendResponse({ success: false, error: error.message });
      });
    return true; // Will respond asynchronously
  }
  
  if (message.type === 'fetchGoogleSheet') {
    // Fetch Google Sheet content from service worker (avoids CORS issues)
    fetchGoogleSheetExport(message.spreadsheetId, message.gid)
      .then(content => {
        sendResponse({ success: true, content: content });
      })
      .catch(error => {
        console.error('[ServiceWorker] Google Sheet fetch error:', error.message);
        sendResponse({ success: false, error: error.message });
      });
    return true; // Will respond asynchronously
  }
  
  if (message.type === 'fetchGoogleSlides') {
    // Fetch Google Slides content from service worker (avoids CORS issues)
    fetchGoogleSlidesExport(message.presentationId)
      .then(content => {
        sendResponse({ success: true, content: content });
      })
      .catch(error => {
        console.error('[ServiceWorker] Google Slides fetch error:', error.message);
        sendResponse({ success: false, error: error.message });
      });
    return true; // Will respond asynchronously
  }
});

/**
 * Unified rate limiting for Google exports
 * Google has a global rate limit across all their products
 */
async function waitForGoogleRateLimit() {
  const now = Date.now();
  const timeSinceLastFetch = now - googleExportLastFetch;
  
  if (timeSinceLastFetch < GOOGLE_EXPORT_COOLDOWN_MS) {
    const waitTime = GOOGLE_EXPORT_COOLDOWN_MS - timeSinceLastFetch;
    console.log(`[ServiceWorker] Rate limiting Google export, waiting ${waitTime}ms`);
    await new Promise(resolve => setTimeout(resolve, waitTime));
  }
  
  googleExportLastFetch = Date.now();
}

/**
 * Handle 429 rate limit response from Google
 */
function handleGoogleRateLimit() {
  googleExportLastFetch = Date.now() + GOOGLE_RATE_LIMIT_PENALTY_MS;
  console.log(`[ServiceWorker] Google rate limited, backing off for ${GOOGLE_RATE_LIMIT_PENALTY_MS}ms`);
}

/**
 * Fetch Google Doc content via export URL
 * Service workers can make cross-origin requests with proper permissions
 */
async function fetchGoogleDocExport(docId) {
  await waitForGoogleRateLimit();
  
  const exportUrl = `https://docs.google.com/document/d/${docId}/export?format=txt`;
  console.log('[ServiceWorker] Fetching Google Doc:', exportUrl);
  
  const response = await fetch(exportUrl, {
    method: 'GET',
    credentials: 'include'
  });
  
  if (response.status === 429) {
    handleGoogleRateLimit();
    throw new Error('Rate limited by Google (429) - try again later');
  }
  
  if (!response.ok) {
    throw new Error(`Export failed with status ${response.status}`);
  }
  
  const content = await response.text();
  
  if (!content || content.trim().length === 0) {
    throw new Error('Export returned empty content');
  }
  
  console.log('[ServiceWorker] Google Doc fetched, length:', content.length);
  return content.trim();
}

/**
 * Fetch Google Sheet content via export URL
 * Exports as CSV format for easy parsing
 */
async function fetchGoogleSheetExport(spreadsheetId, gid = '0') {
  await waitForGoogleRateLimit();
  
  const exportUrl = `https://docs.google.com/spreadsheets/d/${spreadsheetId}/export?format=csv&gid=${gid}`;
  console.log('[ServiceWorker] Fetching Google Sheet:', exportUrl);
  
  const response = await fetch(exportUrl, {
    method: 'GET',
    credentials: 'include'
  });
  
  if (response.status === 429) {
    handleGoogleRateLimit();
    throw new Error('Rate limited by Google (429) - try again later');
  }
  
  if (!response.ok) {
    throw new Error(`Export failed with status ${response.status}`);
  }
  
  const content = await response.text();
  
  if (!content || content.trim().length === 0) {
    throw new Error('Export returned empty content');
  }
  
  console.log('[ServiceWorker] Google Sheet fetched, length:', content.length);
  return content.trim();
}

/**
 * Fetch Google Slides content via export URL
 * Exports as plain text format
 */
async function fetchGoogleSlidesExport(presentationId) {
  await waitForGoogleRateLimit();
  
  const exportUrl = `https://docs.google.com/presentation/d/${presentationId}/export?format=txt`;
  console.log('[ServiceWorker] Fetching Google Slides:', exportUrl);
  
  const response = await fetch(exportUrl, {
    method: 'GET',
    credentials: 'include'
  });
  
  if (response.status === 429) {
    handleGoogleRateLimit();
    throw new Error('Rate limited by Google (429) - try again later');
  }
  
  if (!response.ok) {
    throw new Error(`Export failed with status ${response.status}`);
  }
  
  const content = await response.text();
  
  if (!content || content.trim().length === 0) {
    throw new Error('Export returned empty content');
  }
  
  console.log('[ServiceWorker] Google Slides fetched, length:', content.length);
  return content.trim();
}

// Register event listeners
chrome.tabs.onActivated.addListener(handleTabActivated);
chrome.tabs.onUpdated.addListener(handleTabUpdated);  // Catch URL changes in SPAs
chrome.windows.onFocusChanged.addListener(handleWindowFocusChanged);

console.log('[ServiceWorker] Initialized - listening for tab activation, URL changes, and window focus');
