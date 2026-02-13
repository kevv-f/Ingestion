/**
 * Google Docs Content Extractor
 * 
 * Handles extraction from Google Docs which uses complex rendering:
 * - Canvas-based rendering (newer, harder to extract)
 * - DOM-based rendering (older, easier to extract)
 * - Accessibility iframe (for screen readers)
 * 
 * Extraction Strategies (in order of reliability):
 * 1. Export URL (/export?format=txt) - Most reliable, gets full document
 * 2. DOM extraction (kix-page, kix-lineview) - Works with DOM rendering
 * 3. Iframe extraction (docs-texteventtarget-iframe) - Accessibility layer
 * 
 * Key Challenges:
 * - Google Docs switched to canvas-based rendering in 2021
 * - Canvas rendering doesn't expose text in DOM
 * - Visual line breaks vs semantic paragraph breaks
 * - Cross-origin iframe restrictions
 */

// ============================================
// CONFIGURATION
// ============================================

const GDOCS_CONFIG = {
  // Timeouts
  DOM_WAIT_TIMEOUT: 5000,
  EXPORT_TIMEOUT: 10000,
  
  // DOM Selectors
  selectors: {
    // Title selectors
    title: [
      '.docs-title-input',
      '.docs-title-input-label-inner',
      'input.docs-title-input',
      '[data-tooltip="Rename"]'
    ],
    
    // Page content selectors (DOM-based rendering)
    page: [
      '.kix-page',
      '.kix-paginateddocumentplugin'
    ],
    
    // Line view selectors
    lineView: [
      '.kix-lineview',
      '.kix-lineview-content'
    ],
    
    // Paragraph selectors
    paragraph: [
      '.kix-paragraphrenderer',
      '.kix-paragraph'
    ],
    
    // Editor container selectors
    editor: [
      '.kix-appview-editor',
      '.docs-editor-container',
      '.doc-content'
    ],
    
    // Accessibility iframe
    accessibilityIframe: [
      '.docs-texteventtarget-iframe',
      'iframe.docs-texteventtarget-iframe'
    ],
    
    // Accessibility textbox
    accessibilityTextbox: [
      '[role="textbox"][contenteditable="true"]',
      '[contenteditable="true"][aria-label]'
    ],
    
    // Owner/author info
    owner: [
      '[data-tooltip*="owner"]',
      '.docs-owner-name',
      '[aria-label*="Owner"]'
    ]
  }
};

// ============================================
// UTILITIES
// ============================================

/**
 * Extract document ID from Google Docs URL
 * Supports various URL formats:
 * - /document/d/DOC_ID/edit
 * - /document/d/DOC_ID/preview
 * - /document/d/DOC_ID
 */
function extractDocId(url) {
  try {
    const match = url.match(/\/document\/d\/([a-zA-Z0-9_-]+)/);
    return match ? match[1] : null;
  } catch (e) {
    return null;
  }
}

/**
 * Query element using multiple selector fallbacks
 */
function queryWithFallbacks(selectors) {
  const selectorList = Array.isArray(selectors) ? selectors : [selectors];
  for (const selector of selectorList) {
    const element = document.querySelector(selector);
    if (element) return element;
  }
  return null;
}

/**
 * Query all elements using multiple selector fallbacks
 */
function queryAllWithFallbacks(selectors) {
  const selectorList = Array.isArray(selectors) ? selectors : [selectors];
  for (const selector of selectorList) {
    const elements = document.querySelectorAll(selector);
    if (elements.length > 0) return elements;
  }
  return [];
}

/**
 * Wait for content to appear in DOM
 */
function waitForContent(selectors, timeout = GDOCS_CONFIG.DOM_WAIT_TIMEOUT) {
  const selectorList = Array.isArray(selectors) ? selectors : [selectors];
  
  return new Promise((resolve) => {
    for (const selector of selectorList) {
      const element = document.querySelector(selector);
      if (element) {
        resolve(element);
        return;
      }
    }
    
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of selectorList) {
        const element = document.querySelector(selector);
        if (element) {
          obs.disconnect();
          resolve(element);
          return;
        }
      }
    });
    
    observer.observe(document.body, {
      childList: true,
      subtree: true
    });
    
    setTimeout(() => {
      observer.disconnect();
      resolve(null);
    }, timeout);
  });
}

/**
 * Get current Unix timestamp in seconds
 */
function nowUnixSeconds() {
  return Math.floor(Date.now() / 1000);
}

// ============================================
// TITLE & METADATA EXTRACTION
// ============================================

/**
 * Get document title from various sources
 */
function getTitle() {
  const s = GDOCS_CONFIG.selectors;
  
  // Try title input field
  const titleInput = queryWithFallbacks(s.title);
  if (titleInput) {
    const title = titleInput.value || titleInput.textContent?.trim();
    if (title && title !== 'Untitled document') {
      return title;
    }
  }
  
  // Fallback to page title
  const pageTitle = document.title.replace(' - Google Docs', '').trim();
  return pageTitle || 'Untitled Document';
}

/**
 * Get document metadata
 */
function getMetadata() {
  const s = GDOCS_CONFIG.selectors;
  const metadata = {
    author: null,
    lastModified: null
  };
  
  const ownerEl = queryWithFallbacks(s.owner);
  if (ownerEl) {
    metadata.author = ownerEl.textContent?.trim();
  }
  
  return metadata;
}

// ============================================
// CONTENT CLEANING
// ============================================

/**
 * Clean up Google Docs content
 * Handles visual vs semantic line breaks
 */
function cleanContent(content) {
  if (!content) return '';
  
  return content
    // Normalize line endings
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    // Preserve paragraph breaks (double newlines)
    .replace(/\n\s*\n/g, '{{PARA}}')
    // Remove single newlines (visual wrapping)
    .replace(/\n/g, ' ')
    // Restore paragraph breaks
    .replace(/{{PARA}}/g, '\n\n')
    // Clean up multiple spaces
    .replace(/\s+/g, ' ')
    // Clean up spaces around paragraph breaks
    .replace(/\s*\n\n\s*/g, '\n\n')
    .trim();
}

// ============================================
// EXTRACTION STRATEGIES
// ============================================

/**
 * Strategy 1: Export URL (Most Reliable)
 * Uses Google's export endpoint to get plain text
 */
async function extractViaExport(docId) {
  const exportUrl = `https://docs.google.com/document/d/${docId}/export?format=txt`;
  
  try {
    const response = await fetch(exportUrl, {
      method: 'GET',
      credentials: 'include'
    });
    
    if (!response.ok) {
      console.log(`[GDocsExtractor] Export failed: ${response.status}`);
      return null;
    }
    
    const content = await response.text();
    
    if (!content || content.trim().length === 0) {
      return null;
    }
    
    const title = getTitle();
    const metadata = getMetadata();
    
    return {
      source: 'gdocs',
      url: window.location.href,
      content: content.trim(),
      title: title,
      author: metadata.author,
      timestamp: nowUnixSeconds()
    };
  } catch (error) {
    console.log('[GDocsExtractor] Export error:', error.message);
    return null;
  }
}

/**
 * Strategy 2: DOM Extraction
 * Extracts from kix-page and kix-lineview elements
 */
async function extractFromDom() {
  const s = GDOCS_CONFIG.selectors;
  
  await waitForContent([...s.page, ...s.editor]);
  await new Promise(resolve => setTimeout(resolve, 500));
  
  const title = getTitle();
  let content = '';
  
  // Method 1: kix-page with kix-lineview
  const pages = queryAllWithFallbacks(s.page);
  if (pages.length > 0) {
    const textParts = [];
    pages.forEach(page => {
      const lines = page.querySelectorAll(s.lineView.join(', '));
      lines.forEach(line => {
        textParts.push(line.textContent || '');
      });
    });
    content = textParts.join('\n');
  }
  
  // Method 2: kix-paragraphrenderer
  if (!content) {
    const paragraphs = queryAllWithFallbacks(s.paragraph);
    if (paragraphs.length > 0) {
      const textParts = [];
      paragraphs.forEach(para => {
        textParts.push(para.textContent || '');
      });
      content = textParts.join('\n');
    }
  }
  
  // Method 3: Editor container
  if (!content) {
    const editor = queryWithFallbacks(s.editor);
    if (editor) {
      content = editor.textContent || '';
    }
  }
  
  content = cleanContent(content);
  
  if (!content) {
    return null;
  }
  
  return {
    source: 'gdocs',
    url: window.location.href,
    content: content,
    title: title,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Strategy 3: Iframe Extraction
 * Extracts from accessibility iframe
 */
async function extractFromIframe() {
  const s = GDOCS_CONFIG.selectors;
  const title = getTitle();
  let content = '';
  
  const iframe = queryWithFallbacks(s.accessibilityIframe);
  if (iframe) {
    try {
      const frameDoc = iframe.contentDocument || iframe.contentWindow?.document;
      if (frameDoc) {
        const textbox = frameDoc.querySelector(s.accessibilityTextbox.join(', '));
        if (textbox) {
          content = textbox.textContent || '';
        }
      }
    } catch (e) {
      console.log('[GDocsExtractor] Cannot access iframe:', e.message);
    }
  }
  
  content = cleanContent(content);
  
  if (!content) {
    return null;
  }
  
  return {
    source: 'gdocs',
    url: window.location.href,
    content: content,
    title: title,
    timestamp: nowUnixSeconds()
  };
}

// ============================================
// MAIN EXPORT
// ============================================

/**
 * Main Google Docs extraction function
 */
async function extractGoogleDocs() {
  console.log('[GDocsExtractor] Starting extraction...');
  
  const url = window.location.href;
  const docId = extractDocId(url);
  
  // Strategy 1: Export URL (most reliable)
  if (docId) {
    console.log('[GDocsExtractor] Found doc ID:', docId);
    const exportResult = await extractViaExport(docId);
    if (exportResult && exportResult.content) {
      console.log('[GDocsExtractor] Export extraction successful');
      return exportResult;
    }
  }
  
  // Strategy 2: DOM extraction
  const domResult = await extractFromDom();
  if (domResult && domResult.content) {
    console.log('[GDocsExtractor] DOM extraction successful');
    return domResult;
  }
  
  // Strategy 3: Iframe extraction
  const iframeResult = await extractFromIframe();
  if (iframeResult && iframeResult.content) {
    console.log('[GDocsExtractor] Iframe extraction successful');
    return iframeResult;
  }
  
  console.log('[GDocsExtractor] All extraction strategies failed');
  return null;
}

// Export for use in content script
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { extractGoogleDocs };
}
