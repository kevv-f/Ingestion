/**
 * Content Script - DOM extraction for content ingestion
 * 
 * Outputs CapturePayload format:
 * {
 *   source: string,     // "slack" | "gmail" | "jira" | "browser" | etc.
 *   url: string,        // Location identifier
 *   content: string,    // The text to ingest
 *   title?: string,     // Optional title/subject
 *   author?: string,    // Optional author/sender
 *   channel?: string,   // Optional channel/project/workspace
 *   timestamp?: number  // Optional unix seconds
 * }
 */

// ============================================
// ROUTER
// ============================================

function getExtractor(url) {
  const hostname = new URL(url).hostname;
  const pathname = new URL(url).pathname;
  const searchParams = new URL(url).searchParams;

  if (hostname.includes('slack.com')) return extractSlack;
  if (hostname.includes('mail.google.com')) return extractGmail;
  if (hostname.includes('outlook.live.com') || hostname.includes('outlook.office.com')) return extractOutlook;
  if (hostname.includes('atlassian.net') || hostname.includes('jira')) return extractJira;
  // Google products must be checked in order: Sheets, Slides, then Docs (all on docs.google.com)
  if (hostname.includes('docs.google.com') && pathname.includes('/spreadsheets/')) return extractGoogleSheets;
  if (hostname.includes('docs.google.com') && pathname.includes('/presentation/')) return extractGoogleSlides;
  if (hostname.includes('docs.google.com')) return extractGoogleDocs;
  // Gemini (Google's AI assistant)
  if (hostname.includes('gemini.google.com')) return extractGemini;
  // Google Search AI Mode (udm=50 or AI overview results)
  if (hostname.includes('google.com') && (pathname === '/search' || pathname.startsWith('/search'))) {
    // Check for AI Mode indicator (udm=50) or AI overview presence
    if (searchParams.get('udm') === '50' || searchParams.has('ai')) {
      return extractGoogleAIMode;
    }
    // Also check if we're on a regular search but with AI overview
    return extractGoogleSearch;
  }
  if (hostname.includes('discord.com')) return extractDiscord;
  return extractGeneric;
}

// ============================================
// UTILITIES
// ============================================

function waitForContent(selector, timeout = 3000) {
  return new Promise((resolve) => {
    const element = document.querySelector(selector);
    if (element) { resolve(element); return; }

    const observer = new MutationObserver((mutations, obs) => {
      const el = document.querySelector(selector);
      if (el) { obs.disconnect(); resolve(el); }
    });

    observer.observe(document.body, { childList: true, subtree: true });
    setTimeout(() => { observer.disconnect(); resolve(document.querySelector(selector)); }, timeout);
  });
}

function formatTimestamp(date) {
  if (!date) return '';
  const d = new Date(date);
  return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
}

function nowUnixSeconds() {
  return Math.floor(Date.now() / 1000);
}

// ============================================
// EXTRACTORS
// ============================================

async function extractSlack() {
  await waitForContent('[data-qa="message_container"], [data-qa="virtual-list-item"]');

  const channelHeader = document.querySelector('[data-qa="channel_header_title"]');
  const channelName = channelHeader?.textContent?.trim() || 'unknown-channel';

  const messageContainers = document.querySelectorAll(
    '[data-qa="message_container"], [data-qa="virtual-list-item"]'
  );

  const messages = [];

  messageContainers.forEach((container) => {
    const authorEl = container.querySelector('[data-qa="message_sender_name"]');
    const author = authorEl?.textContent?.trim() || 'unknown';

    const timeEl = container.querySelector('[data-ts], time, [datetime]');
    const timestamp = timeEl?.getAttribute('datetime') || timeEl?.getAttribute('data-ts') || '';
    const timeStr = formatTimestamp(timestamp);

    const textEl = container.querySelector('[data-qa="message-text"], .c-message__body');
    const text = textEl?.textContent?.trim() || '';

    if (text) {
      messages.push(`[${author} ${timeStr}] ${text}`);
    }
  });

  return {
    source: 'slack',
    url: window.location.href,
    content: messages.join('\n'),
    title: channelName,
    channel: channelName,
    timestamp: nowUnixSeconds()
  };
}

async function extractGmail() {
  const emailBody = document.querySelector('.a3s.aiL');
  return emailBody ? extractGmailEmail() : extractGmailInbox();
}

async function extractGmailInbox() {
  await waitForContent('tr.zA');
  const emailRows = document.querySelectorAll('tr.zA');
  const emails = [];

  emailRows.forEach((row) => {
    const senderEl = row.querySelector('.yX.xY span[email], .yW span[email]');
    const sender = senderEl?.getAttribute('email') || senderEl?.textContent?.trim() || 'unknown';
    const subjectEl = row.querySelector('.y6 span, .bog');
    const subject = subjectEl?.textContent?.trim() || 'No Subject';
    const snippetEl = row.querySelector('.y2');
    const snippet = snippetEl?.textContent?.trim().replace(/^\s*-\s*/, '') || '';
    const dateEl = row.querySelector('.xW.xY span, td.xW');
    const date = dateEl?.textContent?.trim() || '';

    emails.push(`[${sender}] ${subject} - ${snippet} (${date})`);
  });

  return {
    source: 'gmail',
    url: window.location.href,
    content: emails.join('\n'),
    title: 'Gmail Inbox',
    timestamp: nowUnixSeconds()
  };
}

async function extractGmailEmail() {
  const subjectEl = document.querySelector('h2.hP');
  const subject = subjectEl?.textContent?.trim() || 'No Subject';
  const senderEl = document.querySelector('.gD');
  const sender = senderEl?.getAttribute('email') || senderEl?.textContent?.trim() || 'unknown';
  const bodyEl = document.querySelector('.a3s.aiL');
  const body = bodyEl?.textContent?.trim() || '';
  const dateEl = document.querySelector('.g3');
  const date = dateEl?.getAttribute('title') || dateEl?.textContent?.trim() || '';

  return {
    source: 'gmail',
    url: window.location.href,
    content: `Subject: ${subject}\nFrom: ${sender}\nDate: ${date}\n\n${body}`,
    title: subject,
    author: sender,
    timestamp: nowUnixSeconds()
  };
}

async function extractOutlook() {
  await waitForContent('[role="listbox"], [role="option"]');
  const readingPane = document.querySelector('[role="main"] [aria-label*="Message body"]');
  
  if (readingPane) {
    const subjectEl = document.querySelector('[role="heading"][aria-level="2"]');
    const senderEl = document.querySelector('[data-testid="SenderPersona"]');
    const body = readingPane?.textContent?.trim() || '';
    const subject = subjectEl?.textContent?.trim() || 'No Subject';

    return {
      source: 'outlook',
      url: window.location.href,
      content: `Subject: ${subject}\nFrom: ${senderEl?.textContent?.trim() || 'unknown'}\n\n${body}`,
      title: subject,
      author: senderEl?.textContent?.trim(),
      timestamp: nowUnixSeconds()
    };
  }

  // Inbox list view
  const emailItems = document.querySelectorAll('[role="option"]');
  const emails = [];

  emailItems.forEach((item) => {
    const sender = item.querySelector('[data-testid="SenderName"]')?.textContent?.trim() || 'unknown';
    const subject = item.querySelector('[data-testid="Subject"]')?.textContent?.trim() || 'No Subject';
    const preview = item.querySelector('[data-testid="Preview"]')?.textContent?.trim() || '';

    emails.push(`[${sender}] ${subject} - ${preview}`);
  });

  return {
    source: 'outlook',
    url: window.location.href,
    content: emails.join('\n'),
    title: 'Outlook Inbox',
    timestamp: nowUnixSeconds()
  };
}

async function extractJira() {
  console.log('[ContentScript] Starting Jira extraction...');
  
  const url = window.location.href;
  const issueKey = extractIssueKeyFromUrl(url);
  
  // Strategy 1: If we have an issue key, try REST API first (most reliable)
  if (issueKey) {
    console.log('[ContentScript] Found issue key:', issueKey);
    const apiResult = await fetchJiraIssueViaApi(issueKey);
    if (apiResult && apiResult.content) {
      console.log('[ContentScript] Jira API extraction successful');
      return apiResult;
    }
  }
  
  // Strategy 2: Try DOM extraction for issue detail
  const domResult = await extractJiraFromDom();
  if (domResult && domResult.content) {
    console.log('[ContentScript] Jira DOM extraction successful');
    return domResult;
  }
  
  // Strategy 3: Check if this is a board view
  if (url.includes('/boards/') || url.includes('/board/')) {
    const boardResult = await extractJiraBoardView();
    if (boardResult && boardResult.content) {
      console.log('[ContentScript] Jira board extraction successful');
      return boardResult;
    }
  }
  
  // Strategy 4: Check if this is a backlog view
  if (url.includes('/backlog')) {
    const backlogResult = await extractJiraBacklogView();
    if (backlogResult && backlogResult.content) {
      console.log('[ContentScript] Jira backlog extraction successful');
      return backlogResult;
    }
  }
  
  console.log('[ContentScript] All Jira extraction strategies failed');
  return null;
}

// ============================================
// JIRA HELPER FUNCTIONS
// ============================================

/**
 * Extract issue key from URL (supports board view selectedIssue param and /browse/ paths)
 */
function extractIssueKeyFromUrl(url) {
  try {
    const urlObj = new URL(url);
    
    // Check for selectedIssue parameter (board view)
    const selectedIssue = urlObj.searchParams.get('selectedIssue');
    if (selectedIssue) return selectedIssue;
    
    // Check for /browse/PROJ-123 pattern
    const browseMatch = url.match(/\/browse\/([A-Z][A-Z0-9]+-\d+)/i);
    if (browseMatch) return browseMatch[1];
    
    // Check for issue key in path
    const pathMatch = url.match(/([A-Z][A-Z0-9]+-\d+)/i);
    if (pathMatch) return pathMatch[1];
  } catch (e) {
    console.log('[ContentScript] Error parsing URL:', e);
  }
  return null;
}

/**
 * Fetch issue details via REST API using session cookies
 */
async function fetchJiraIssueViaApi(issueKey) {
  const baseUrl = `${window.location.protocol}//${window.location.host}`;
  const apiUrl = `${baseUrl}/rest/api/3/issue/${issueKey}?expand=renderedFields,names`;
  
  try {
    const response = await fetch(apiUrl, {
      method: 'GET',
      credentials: 'include',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      }
    });
    
    if (!response.ok) {
      console.log(`[ContentScript] Jira API request failed: ${response.status}`);
      return null;
    }
    
    const data = await response.json();
    return parseJiraApiResponse(data);
  } catch (error) {
    console.log('[ContentScript] Jira API fetch error:', error.message);
    return null;
  }
}

/**
 * Parse Jira REST API response into payload format
 */
function parseJiraApiResponse(data) {
  const fields = data.fields || {};
  const renderedFields = data.renderedFields || {};
  
  const parts = [];
  const summary = fields.summary || '';
  parts.push(`# ${data.key}: ${summary}`);
  parts.push('');
  
  // Metadata
  const metadata = [];
  if (fields.status?.name) metadata.push(`Status: ${fields.status.name}`);
  if (fields.priority?.name) metadata.push(`Priority: ${fields.priority.name}`);
  if (fields.issuetype?.name) metadata.push(`Type: ${fields.issuetype.name}`);
  if (fields.assignee?.displayName) metadata.push(`Assignee: ${fields.assignee.displayName}`);
  if (fields.reporter?.displayName) metadata.push(`Reporter: ${fields.reporter.displayName}`);
  if (fields.project?.name) metadata.push(`Project: ${fields.project.name}`);
  
  if (metadata.length > 0) {
    parts.push(metadata.join(' | '));
    parts.push('');
  }
  
  // Description
  let description = '';
  if (renderedFields.description) {
    const temp = document.createElement('div');
    temp.innerHTML = renderedFields.description;
    description = temp.textContent?.trim() || '';
  } else if (fields.description) {
    description = extractTextFromAdf(fields.description);
  }
  
  if (description) {
    parts.push('## Description');
    parts.push(description);
    parts.push('');
  }
  
  // Labels
  if (fields.labels?.length > 0) {
    parts.push(`Labels: ${fields.labels.join(', ')}`);
    parts.push('');
  }
  
  // Recent comments
  if (fields.comment?.comments?.length > 0) {
    parts.push('## Recent Comments');
    fields.comment.comments.slice(-3).forEach(comment => {
      const author = comment.author?.displayName || 'Unknown';
      const created = new Date(comment.created).toLocaleString();
      let body = '';
      if (comment.renderedBody) {
        const temp = document.createElement('div');
        temp.innerHTML = comment.renderedBody;
        body = temp.textContent?.trim() || '';
      } else if (comment.body) {
        body = extractTextFromAdf(comment.body);
      }
      parts.push(`[${author} - ${created}]`);
      parts.push(body);
      parts.push('');
    });
  }
  
  return {
    source: 'jira',
    url: window.location.href,
    content: parts.join('\n').trim(),
    title: `${data.key}: ${summary}`,
    author: fields.assignee?.displayName || fields.reporter?.displayName,
    channel: fields.project?.name,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Extract plain text from Atlassian Document Format (ADF)
 */
function extractTextFromAdf(adf) {
  if (!adf) return '';
  if (typeof adf === 'string') return adf;
  
  const extractFromNode = (node) => {
    if (!node) return '';
    if (node.type === 'text') return node.text || '';
    if (node.content && Array.isArray(node.content)) {
      return node.content.map(extractFromNode).join('');
    }
    return '';
  };
  
  return extractFromNode(adf).trim();
}

/**
 * Extract issue details from DOM (fallback when API fails)
 */
async function extractJiraFromDom() {
  // Multiple selector fallbacks for different Jira versions
  const issueDetailSelectors = [
    '[data-testid="issue.views.issue-details.issue-layout"]',
    '[data-testid="issue-detail"]',
    '[data-testid="issue.views.issue-base.foundation.summary.heading"]',
    '[role="dialog"][aria-label*="issue"]',
    '.issue-view'
  ];
  
  // Wait for issue detail with extended timeout for SPA
  const issueDetail = await waitForJiraContent(issueDetailSelectors, 5000);
  if (!issueDetail) {
    return null;
  }
  
  // Give React time to fully render
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Extract issue key with fallbacks
  let issueKey = '';
  const keySelectors = [
    '[data-testid="issue.views.issue-base.foundation.breadcrumbs.current-issue.item"]',
    '[data-testid="issue.views.issue-base.foundation.breadcrumbs.breadcrumb-current-issue-container"] a',
    'a[href*="/browse/"][data-testid]'
  ];
  for (const sel of keySelectors) {
    const el = document.querySelector(sel);
    if (el) {
      const text = el.textContent?.trim() || '';
      const match = text.match(/([A-Z][A-Z0-9]+-\d+)/i);
      if (match) {
        issueKey = match[1];
        break;
      }
    }
  }
  if (!issueKey) {
    issueKey = extractIssueKeyFromUrl(window.location.href) || 'UNKNOWN';
  }
  
  // Extract summary with fallbacks
  const summarySelectors = [
    '[data-testid="issue.views.issue-base.foundation.summary.heading"]',
    '[data-testid="issue.views.issue-base.foundation.summary"] h1',
    'h1[data-testid*="summary"]'
  ];
  let summary = '';
  for (const sel of summarySelectors) {
    const el = document.querySelector(sel);
    if (el) {
      summary = el.textContent?.trim() || '';
      if (summary) break;
    }
  }
  
  // Extract description with fallbacks
  const descSelectors = [
    '[data-testid="issue.views.field.rich-text.description"]',
    '[data-testid="issue.views.field.rich-text.description"] [data-renderer-start-pos]',
    '.user-content-block'
  ];
  let description = '';
  for (const sel of descSelectors) {
    const el = document.querySelector(sel);
    if (el) {
      description = el.textContent?.trim() || '';
      if (description) break;
    }
  }
  
  // Extract status
  const statusSelectors = [
    '[data-testid="issue.views.issue-base.foundation.status.status-field-wrapper"]',
    '[data-testid="issue.views.issue-base.foundation.status.status-field-wrapper"] button'
  ];
  let status = '';
  for (const sel of statusSelectors) {
    const el = document.querySelector(sel);
    if (el) {
      status = el.textContent?.trim() || '';
      if (status) break;
    }
  }
  
  // Extract assignee
  const assigneeSelectors = [
    '[data-testid="issue.views.field.user.assignee"]'
  ];
  let assignee = '';
  for (const sel of assigneeSelectors) {
    const el = document.querySelector(sel);
    if (el) {
      assignee = el.textContent?.trim().replace(/^Assignee\s*/i, '') || '';
      if (assignee === 'Unassigned') assignee = '';
      if (assignee) break;
    }
  }
  
  // Build content
  const parts = [];
  parts.push(`# ${issueKey}: ${summary}`);
  parts.push('');
  
  const metadata = [];
  if (status) metadata.push(`Status: ${status}`);
  if (assignee) metadata.push(`Assignee: ${assignee}`);
  if (metadata.length > 0) {
    parts.push(metadata.join(' | '));
    parts.push('');
  }
  
  if (description) {
    parts.push('## Description');
    parts.push(description);
  }
  
  const content = parts.join('\n').trim();
  if (!content || content === `# ${issueKey}: `) {
    return null;
  }
  
  return {
    source: 'jira',
    url: window.location.href,
    content: content,
    title: `${issueKey}: ${summary}`,
    author: assignee || undefined,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Wait for Jira content with MutationObserver (handles React SPA)
 */
function waitForJiraContent(selectors, timeout = 5000) {
  const selectorList = Array.isArray(selectors) ? selectors : [selectors];
  
  return new Promise((resolve) => {
    // Check if element already exists
    for (const selector of selectorList) {
      const element = document.querySelector(selector);
      if (element) {
        resolve(element);
        return;
      }
    }
    
    // Set up observer for dynamic content
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
      subtree: true,
      attributes: true,
      attributeFilter: ['data-testid', 'class', 'id']
    });
    
    setTimeout(() => {
      observer.disconnect();
      for (const selector of selectorList) {
        const element = document.querySelector(selector);
        if (element) {
          resolve(element);
          return;
        }
      }
      resolve(null);
    }, timeout);
  });
}

/**
 * Extract board view (list of issues on Scrum/Kanban board)
 */
async function extractJiraBoardView() {
  const cardSelectors = [
    '[data-testid="platform-board-kit.ui.card.card"]',
    '[data-testid="board-card"]',
    '[data-testid="software-board.board-container.board.card-container.card"]',
    '.ghx-issue'
  ];
  
  await waitForJiraContent(cardSelectors, 3000);
  await new Promise(resolve => setTimeout(resolve, 500));
  
  let cards = [];
  for (const sel of cardSelectors) {
    cards = document.querySelectorAll(sel);
    if (cards.length > 0) break;
  }
  
  if (cards.length === 0) {
    return null;
  }
  
  // Get board name
  const boardNameSelectors = [
    '[data-testid="board-header.ui.board-name"]',
    '[data-testid="software-board.header.title"]'
  ];
  let boardName = 'Jira Board';
  for (const sel of boardNameSelectors) {
    const el = document.querySelector(sel);
    if (el) {
      boardName = el.textContent?.trim() || boardName;
      break;
    }
  }
  
  // Extract issues from cards
  const issues = [];
  cards.forEach(card => {
    const keySelectors = [
      '[data-testid="platform-card.common.ui.key"]',
      '[data-testid="software-board.board-container.board.card-container.card.key"]'
    ];
    const summarySelectors = [
      '[data-testid="platform-card.common.ui.summary"]',
      '[data-testid="software-board.board-container.board.card-container.card.summary"]'
    ];
    
    let key = '';
    let summary = '';
    
    for (const sel of keySelectors) {
      const el = card.querySelector(sel);
      if (el) {
        key = el.textContent?.trim() || '';
        break;
      }
    }
    
    for (const sel of summarySelectors) {
      const el = card.querySelector(sel);
      if (el) {
        summary = el.textContent?.trim() || '';
        break;
      }
    }
    
    if (key) {
      issues.push(`${key}: ${summary}`);
    }
  });
  
  if (issues.length === 0) {
    return null;
  }
  
  return {
    source: 'jira',
    url: window.location.href,
    content: issues.join('\n'),
    title: boardName,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Extract backlog view
 */
async function extractJiraBacklogView() {
  const itemSelectors = [
    '[data-test-id="software-backlog.card-list.card.card-contents.accessible-card-key"]',
    '[data-testid="software-backlog.backlog-content.card"]'
  ];
  
  await waitForJiraContent(itemSelectors, 3000);
  await new Promise(resolve => setTimeout(resolve, 500));
  
  let items = [];
  for (const sel of itemSelectors) {
    items = document.querySelectorAll(sel);
    if (items.length > 0) break;
  }
  
  if (items.length === 0) {
    return null;
  }
  
  const issues = [];
  items.forEach(item => {
    const link = item.querySelector('a');
    if (link) {
      const key = link.textContent?.trim() || '';
      const summary = link.nextSibling?.textContent?.trim() || 
                      link.parentElement?.textContent?.replace(key, '').trim() || '';
      if (key) {
        issues.push(`${key}: ${summary}`);
      }
    }
  });
  
  if (issues.length === 0) {
    return null;
  }
  
  return {
    source: 'jira',
    url: window.location.href,
    content: issues.join('\n'),
    title: 'Jira Backlog',
    timestamp: nowUnixSeconds()
  };
}

async function extractGoogleDocs() {
  console.log('[ContentScript] Starting Google Docs extraction...');
  
  const url = window.location.href;
  const docId = extractDocIdFromUrl(url);
  
  // Skip if this is the Google Docs home/picker page (no document ID)
  if (!docId) {
    console.log('[ContentScript] No document ID found - this is likely the Docs home page, skipping');
    return null;
  }
  
  // Wait for the page to be fully loaded before attempting extraction
  await waitForGoogleDocsReady();
  
  console.log('[ContentScript] Found doc ID:', docId);
  
  // Get title first (from DOM)
  const title = getGoogleDocsTitle();
  const metadata = getGoogleDocsMetadata();
  
  // Strategy 1: Request export via service worker (avoids CORS issues)
  // Content scripts can't make cross-origin fetches, so we ask the service worker
  const exportResult = await requestExportViaServiceWorker(docId, title, metadata);
  if (exportResult && exportResult.content) {
    console.log('[ContentScript] Google Docs export via service worker successful');
    return exportResult;
  }
  
  // Strategy 2: Try DOM extraction with multiple methods
  const domResult = await extractGoogleDocsFromDom();
  if (domResult && domResult.content) {
    console.log('[ContentScript] Google Docs DOM extraction successful');
    return domResult;
  }
  
  // Strategy 3: Try iframe-based extraction
  const iframeResult = await extractGoogleDocsFromIframe();
  if (iframeResult && iframeResult.content) {
    console.log('[ContentScript] Google Docs iframe extraction successful');
    return iframeResult;
  }
  
  console.log('[ContentScript] All Google Docs extraction strategies failed');
  return null;
}

// ============================================
// GOOGLE DOCS HELPER FUNCTIONS
// ============================================

/**
 * Request document export via service worker to avoid CORS issues
 * Content scripts can't make cross-origin fetches since Chrome 85
 */
async function requestExportViaServiceWorker(docId, title, metadata) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(
      {
        type: 'fetchGoogleDoc',
        docId: docId,
        title: title,
        author: metadata.author
      },
      (response) => {
        if (chrome.runtime.lastError) {
          console.log('[ContentScript] Service worker fetch error:', chrome.runtime.lastError.message);
          resolve(null);
          return;
        }
        
        if (response && response.success && response.content) {
          resolve({
            source: 'gdocs',
            url: `https://docs.google.com/document/d/${docId}`, // Canonical URL without query params
            content: response.content,
            title: title,
            author: metadata.author,
            timestamp: nowUnixSeconds()
          });
        } else {
          console.log('[ContentScript] Service worker fetch failed:', response?.error);
          resolve(null);
        }
      }
    );
  });
}

/**
 * Wait for Google Docs to be fully loaded and ready
 * Google Docs is a heavy SPA that takes time to initialize
 */
async function waitForGoogleDocsReady(timeout = 8000) {
  const readyIndicators = [
    '.docs-title-input',           // Title input appears when doc is loaded
    '.kix-appview-editor',         // Editor container
    '.docs-editor',                // Alternative editor selector
    '[data-eventchip]',            // Event chips in the editor
    '.kix-page'                    // Page content
  ];
  
  return new Promise((resolve) => {
    // Check if already ready
    for (const selector of readyIndicators) {
      if (document.querySelector(selector)) {
        console.log('[ContentScript] Google Docs already ready');
        resolve(true);
        return;
      }
    }
    
    // Set up observer to wait for ready state
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of readyIndicators) {
        if (document.querySelector(selector)) {
          console.log('[ContentScript] Google Docs became ready');
          obs.disconnect();
          // Add a small delay to ensure everything is fully rendered
          setTimeout(() => resolve(true), 500);
          return;
        }
      }
    });
    
    observer.observe(document.body, {
      childList: true,
      subtree: true
    });
    
    // Timeout fallback
    setTimeout(() => {
      observer.disconnect();
      console.log('[ContentScript] Google Docs ready timeout - proceeding anyway');
      resolve(false);
    }, timeout);
  });
}

/**
 * Extract document ID from Google Docs URL
 */
function extractDocIdFromUrl(url) {
  try {
    // Pattern: /document/d/DOC_ID/edit or /document/d/DOC_ID
    // Must have /d/ followed by the ID
    const match = url.match(/\/document\/d\/([a-zA-Z0-9_-]+)/);
    return match ? match[1] : null;
  } catch (e) {
    console.log('[ContentScript] Error extracting doc ID:', e);
    return null;
  }
}

/**
 * Get document title from various sources
 */
function getGoogleDocsTitle() {
  // Try multiple selectors for title
  const titleSelectors = [
    '.docs-title-input',
    '.docs-title-input-label-inner',
    'input.docs-title-input',
    '[data-tooltip="Rename"]'
  ];
  
  for (const selector of titleSelectors) {
    const el = document.querySelector(selector);
    if (el) {
      const title = el.value || el.textContent?.trim();
      if (title && title !== 'Untitled document') {
        return title;
      }
    }
  }
  
  // Fallback to document title
  const pageTitle = document.title.replace(' - Google Docs', '').trim();
  return pageTitle || 'Untitled Document';
}

/**
 * Get document metadata (owner, last modified, etc.)
 */
function getGoogleDocsMetadata() {
  const metadata = {
    author: null,
    lastModified: null
  };
  
  // Try to get owner from share dialog or document info
  const ownerSelectors = [
    '[data-tooltip*="owner"]',
    '.docs-owner-name',
    '[aria-label*="Owner"]'
  ];
  
  for (const selector of ownerSelectors) {
    const el = document.querySelector(selector);
    if (el) {
      metadata.author = el.textContent?.trim();
      break;
    }
  }
  
  return metadata;
}

/**
 * Extract content from Google Docs DOM
 * Handles both canvas-based and DOM-based rendering
 */
async function extractGoogleDocsFromDom() {
  // Wait for the editor to load
  await waitForGoogleDocsContent();
  
  // Give the page time to fully render
  await new Promise(resolve => setTimeout(resolve, 500));
  
  const title = getGoogleDocsTitle();
  let content = '';
  
  // Method 1: Try kix-page elements (older DOM-based rendering)
  const pages = document.querySelectorAll('.kix-page');
  if (pages.length > 0) {
    const textParts = [];
    pages.forEach(page => {
      // Get all line views within the page
      const lines = page.querySelectorAll('.kix-lineview');
      lines.forEach(line => {
        const lineText = line.textContent || '';
        textParts.push(lineText);
      });
    });
    content = textParts.join('\n');
  }
  
  // Method 2: Try kix-appview-editor (alternative structure)
  if (!content) {
    const editor = document.querySelector('.kix-appview-editor');
    if (editor) {
      const paragraphs = editor.querySelectorAll('.kix-paragraphrenderer');
      const textParts = [];
      paragraphs.forEach(para => {
        textParts.push(para.textContent || '');
      });
      content = textParts.join('\n');
    }
  }
  
  // Method 3: Try doc-content (mobile/basic view)
  if (!content) {
    const docContent = document.querySelector('.doc-content, .docs-editor-container');
    if (docContent) {
      content = docContent.textContent || '';
    }
  }
  
  // Method 4: Try to get from accessibility layer
  if (!content) {
    const accessibilityContent = document.querySelector('[role="textbox"][contenteditable="true"]');
    if (accessibilityContent) {
      content = accessibilityContent.textContent || '';
    }
  }
  
  // Clean up the content
  content = cleanGoogleDocsContent(content);
  
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
 * Extract content from Google Docs iframe (accessibility mode)
 */
async function extractGoogleDocsFromIframe() {
  const title = getGoogleDocsTitle();
  let content = '';
  
  // Try the accessibility iframe
  const accessibilityFrame = document.querySelector('.docs-texteventtarget-iframe');
  if (accessibilityFrame) {
    try {
      const frameDoc = accessibilityFrame.contentDocument || accessibilityFrame.contentWindow?.document;
      if (frameDoc) {
        const editableDiv = frameDoc.querySelector('[contenteditable="true"]');
        if (editableDiv) {
          content = editableDiv.textContent || '';
        }
      }
    } catch (e) {
      console.log('[ContentScript] Cannot access iframe content:', e.message);
    }
  }
  
  // Clean up the content
  content = cleanGoogleDocsContent(content);
  
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
 * Wait for Google Docs content to load
 */
function waitForGoogleDocsContent(timeout = 5000) {
  const selectors = [
    '.kix-page',
    '.kix-appview-editor',
    '.docs-texteventtarget-iframe',
    '.doc-content',
    '[role="textbox"][contenteditable="true"]'
  ];
  
  return new Promise((resolve) => {
    // Check if any element already exists
    for (const selector of selectors) {
      const element = document.querySelector(selector);
      if (element) {
        resolve(element);
        return;
      }
    }
    
    // Set up observer for dynamic content
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of selectors) {
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
 * Clean up Google Docs content
 * Removes visual line breaks and normalizes whitespace
 */
function cleanGoogleDocsContent(content) {
  if (!content) return '';
  
  // Google Docs inserts visual line breaks that aren't semantic
  // Pattern: newline followed by space and newline is a real paragraph break
  // Single newlines within text are often just visual wrapping
  
  let cleaned = content
    // Normalize line endings
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    // Preserve paragraph breaks (double newlines or newline-space-newline)
    .replace(/\n\s*\n/g, '{{PARA_BREAK}}')
    // Remove single newlines (visual wrapping)
    .replace(/\n/g, ' ')
    // Restore paragraph breaks
    .replace(/{{PARA_BREAK}}/g, '\n\n')
    // Clean up multiple spaces
    .replace(/\s+/g, ' ')
    // Clean up spaces around paragraph breaks
    .replace(/\s*\n\n\s*/g, '\n\n')
    .trim();
  
  return cleaned;
}

// ============================================
// GOOGLE SHEETS EXTRACTOR
// ============================================

async function extractGoogleSheets() {
  console.log('[ContentScript] Starting Google Sheets extraction...');
  
  const url = window.location.href;
  const spreadsheetId = extractSpreadsheetIdFromUrl(url);
  
  // Skip if this is the Google Sheets home/picker page (no spreadsheet ID)
  if (!spreadsheetId) {
    console.log('[ContentScript] No spreadsheet ID found - this is likely the Sheets home page, skipping');
    return null;
  }
  
  // Wait for the page to be fully loaded
  await waitForGoogleSheetsReady();
  
  console.log('[ContentScript] Found spreadsheet ID:', spreadsheetId);
  
  // Get title and metadata from DOM
  const title = getGoogleSheetsTitle();
  const sheetInfo = getActiveSheetInfo();
  
  // Strategy 1: Request CSV export via service worker (most reliable)
  const exportResult = await requestSheetExportViaServiceWorker(spreadsheetId, sheetInfo.gid, title);
  if (exportResult && exportResult.content) {
    console.log('[ContentScript] Google Sheets export via service worker successful');
    return exportResult;
  }
  
  // Strategy 2: Try DOM extraction (fallback)
  const domResult = await extractGoogleSheetsFromDom();
  if (domResult && domResult.content) {
    console.log('[ContentScript] Google Sheets DOM extraction successful');
    return domResult;
  }
  
  console.log('[ContentScript] All Google Sheets extraction strategies failed');
  return null;
}

// ============================================
// GOOGLE SHEETS HELPER FUNCTIONS
// ============================================

/**
 * Extract spreadsheet ID from Google Sheets URL
 * Pattern: /spreadsheets/d/SPREADSHEET_ID/...
 */
function extractSpreadsheetIdFromUrl(url) {
  try {
    const match = url.match(/\/spreadsheets\/d\/([a-zA-Z0-9_-]+)/);
    return match ? match[1] : null;
  } catch (e) {
    console.log('[ContentScript] Error extracting spreadsheet ID:', e);
    return null;
  }
}

/**
 * Get active sheet GID from URL or DOM
 * GID identifies which sheet tab is active
 */
function getActiveSheetInfo() {
  const url = window.location.href;
  let gid = '0'; // Default to first sheet
  
  try {
    const urlObj = new URL(url);
    const gidParam = urlObj.searchParams.get('gid') || urlObj.hash.match(/gid=(\d+)/)?.[1];
    if (gidParam) {
      gid = gidParam;
    }
  } catch (e) {
    console.log('[ContentScript] Error parsing GID from URL:', e);
  }
  
  // Try to get sheet name from DOM
  let sheetName = 'Sheet1';
  const activeTab = document.querySelector('.docs-sheet-tab.docs-sheet-active-tab .docs-sheet-tab-name');
  if (activeTab) {
    sheetName = activeTab.textContent?.trim() || sheetName;
  }
  
  return { gid, sheetName };
}

/**
 * Wait for Google Sheets to be fully loaded
 */
async function waitForGoogleSheetsReady(timeout = 8000) {
  const readyIndicators = [
    '#docs-editor-container',      // Main editor container
    '.docs-sheet-tab',             // Sheet tabs
    '.waffle-pane',                // Cell grid
    '.grid-container',             // Alternative grid selector
    '[data-sheet-id]'              // Sheet data attribute
  ];
  
  return new Promise((resolve) => {
    // Check if already ready
    for (const selector of readyIndicators) {
      if (document.querySelector(selector)) {
        console.log('[ContentScript] Google Sheets already ready');
        resolve(true);
        return;
      }
    }
    
    // Set up observer
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of readyIndicators) {
        if (document.querySelector(selector)) {
          console.log('[ContentScript] Google Sheets became ready');
          obs.disconnect();
          setTimeout(() => resolve(true), 500);
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
      console.log('[ContentScript] Google Sheets ready timeout - proceeding anyway');
      resolve(false);
    }, timeout);
  });
}

/**
 * Get spreadsheet title from DOM
 */
function getGoogleSheetsTitle() {
  const titleSelectors = [
    '.docs-title-input',
    'input.docs-title-input',
    '[data-tooltip="Rename"]'
  ];
  
  for (const selector of titleSelectors) {
    const el = document.querySelector(selector);
    if (el) {
      const title = el.value || el.textContent?.trim();
      if (title && title !== 'Untitled spreadsheet') {
        return title;
      }
    }
  }
  
  // Fallback to document title
  const pageTitle = document.title.replace(' - Google Sheets', '').trim();
  return pageTitle || 'Untitled Spreadsheet';
}

/**
 * Request spreadsheet export via service worker (avoids CORS)
 */
async function requestSheetExportViaServiceWorker(spreadsheetId, gid, title) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(
      {
        type: 'fetchGoogleSheet',
        spreadsheetId: spreadsheetId,
        gid: gid,
        title: title
      },
      (response) => {
        if (chrome.runtime.lastError) {
          console.log('[ContentScript] Service worker fetch error:', chrome.runtime.lastError.message);
          resolve(null);
          return;
        }
        
        if (response && response.success && response.content) {
          resolve({
            source: 'gsheets',
            url: `https://docs.google.com/spreadsheets/d/${spreadsheetId}`, // Canonical URL
            content: response.content,
            title: title,
            timestamp: nowUnixSeconds()
          });
        } else {
          console.log('[ContentScript] Service worker fetch failed:', response?.error);
          resolve(null);
        }
      }
    );
  });
}

/**
 * Extract content from Google Sheets DOM (fallback)
 * This is less reliable than export but works when export fails
 */
async function extractGoogleSheetsFromDom() {
  const title = getGoogleSheetsTitle();
  let content = '';
  
  // Try to get cell data from the grid
  const cells = document.querySelectorAll('.cell-input, [data-cell], .waffle-cell');
  if (cells.length > 0) {
    const rows = new Map();
    
    cells.forEach(cell => {
      // Try to determine row/col from attributes or position
      const row = cell.getAttribute('data-row') || cell.closest('[data-row]')?.getAttribute('data-row') || '0';
      const text = cell.textContent?.trim() || '';
      
      if (!rows.has(row)) {
        rows.set(row, []);
      }
      rows.get(row).push(text);
    });
    
    // Convert to CSV-like format
    const sortedRows = Array.from(rows.entries()).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
    content = sortedRows.map(([_, cells]) => cells.join('\t')).join('\n');
  }
  
  // Alternative: try to get from accessibility layer
  if (!content) {
    const accessibleGrid = document.querySelector('[role="grid"]');
    if (accessibleGrid) {
      const rowEls = accessibleGrid.querySelectorAll('[role="row"]');
      const rows = [];
      rowEls.forEach(row => {
        const cellEls = row.querySelectorAll('[role="gridcell"], [role="columnheader"]');
        const cellTexts = Array.from(cellEls).map(c => c.textContent?.trim() || '');
        if (cellTexts.some(t => t)) {
          rows.push(cellTexts.join('\t'));
        }
      });
      content = rows.join('\n');
    }
  }
  
  if (!content) {
    return null;
  }
  
  return {
    source: 'gsheets',
    url: window.location.href,
    content: content,
    title: title,
    timestamp: nowUnixSeconds()
  };
}

// ============================================
// GOOGLE SLIDES EXTRACTOR
// ============================================

async function extractGoogleSlides() {
  console.log('[ContentScript] Starting Google Slides extraction...');
  
  const url = window.location.href;
  const presentationId = extractPresentationIdFromUrl(url);
  
  // Skip if this is the Google Slides home/picker page (no presentation ID)
  if (!presentationId) {
    console.log('[ContentScript] No presentation ID found - this is likely the Slides home page, skipping');
    return null;
  }
  
  // Wait for the page to be fully loaded
  await waitForGoogleSlidesReady();
  
  console.log('[ContentScript] Found presentation ID:', presentationId);
  
  // Get title from DOM
  const title = getGoogleSlidesTitle();
  
  // Strategy 1: Request text export via service worker (most reliable)
  const exportResult = await requestSlidesExportViaServiceWorker(presentationId, title);
  if (exportResult && exportResult.content) {
    console.log('[ContentScript] Google Slides export via service worker successful');
    return exportResult;
  }
  
  // Strategy 2: Try DOM extraction (fallback)
  const domResult = await extractGoogleSlidesFromDom();
  if (domResult && domResult.content) {
    console.log('[ContentScript] Google Slides DOM extraction successful');
    return domResult;
  }
  
  console.log('[ContentScript] All Google Slides extraction strategies failed');
  return null;
}

// ============================================
// GOOGLE SLIDES HELPER FUNCTIONS
// ============================================

/**
 * Extract presentation ID from Google Slides URL
 * Pattern: /presentation/d/PRESENTATION_ID/...
 */
function extractPresentationIdFromUrl(url) {
  try {
    const match = url.match(/\/presentation\/d\/([a-zA-Z0-9_-]+)/);
    return match ? match[1] : null;
  } catch (e) {
    console.log('[ContentScript] Error extracting presentation ID:', e);
    return null;
  }
}

/**
 * Wait for Google Slides to be fully loaded
 */
async function waitForGoogleSlidesReady(timeout = 8000) {
  const readyIndicators = [
    '.punch-viewer-content',       // Main viewer content
    '.punch-filmstrip',            // Slide filmstrip/thumbnails
    '.docs-title-input',           // Title input
    '.punch-viewer-svgpage',       // SVG slide content
    '[data-slide-id]'              // Slide data attribute
  ];
  
  return new Promise((resolve) => {
    // Check if already ready
    for (const selector of readyIndicators) {
      if (document.querySelector(selector)) {
        console.log('[ContentScript] Google Slides already ready');
        resolve(true);
        return;
      }
    }
    
    // Set up observer
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of readyIndicators) {
        if (document.querySelector(selector)) {
          console.log('[ContentScript] Google Slides became ready');
          obs.disconnect();
          setTimeout(() => resolve(true), 500);
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
      console.log('[ContentScript] Google Slides ready timeout - proceeding anyway');
      resolve(false);
    }, timeout);
  });
}

/**
 * Get presentation title from DOM
 */
function getGoogleSlidesTitle() {
  const titleSelectors = [
    '.docs-title-input',
    'input.docs-title-input',
    '[data-tooltip="Rename"]'
  ];
  
  for (const selector of titleSelectors) {
    const el = document.querySelector(selector);
    if (el) {
      const title = el.value || el.textContent?.trim();
      if (title && title !== 'Untitled presentation') {
        return title;
      }
    }
  }
  
  // Fallback to document title
  const pageTitle = document.title.replace(' - Google Slides', '').trim();
  return pageTitle || 'Untitled Presentation';
}

/**
 * Request presentation export via service worker (avoids CORS)
 */
async function requestSlidesExportViaServiceWorker(presentationId, title) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(
      {
        type: 'fetchGoogleSlides',
        presentationId: presentationId,
        title: title
      },
      (response) => {
        if (chrome.runtime.lastError) {
          console.log('[ContentScript] Service worker fetch error:', chrome.runtime.lastError.message);
          resolve(null);
          return;
        }
        
        if (response && response.success && response.content) {
          resolve({
            source: 'gslides',
            url: `https://docs.google.com/presentation/d/${presentationId}`, // Canonical URL
            content: response.content,
            title: title,
            timestamp: nowUnixSeconds()
          });
        } else {
          console.log('[ContentScript] Service worker fetch failed:', response?.error);
          resolve(null);
        }
      }
    );
  });
}

/**
 * Extract content from Google Slides DOM (fallback)
 * Google Slides uses SVG rendering, so DOM extraction is limited
 */
async function extractGoogleSlidesFromDom() {
  const title = getGoogleSlidesTitle();
  let content = '';
  
  // Try to get text from SVG text elements
  const svgTexts = document.querySelectorAll('.punch-viewer-svgpage text, .punch-viewer-content text');
  if (svgTexts.length > 0) {
    const textParts = [];
    svgTexts.forEach(textEl => {
      const text = textEl.textContent?.trim();
      if (text) {
        textParts.push(text);
      }
    });
    content = textParts.join('\n');
  }
  
  // Try filmstrip slide titles/content
  if (!content) {
    const filmstripSlides = document.querySelectorAll('.punch-filmstrip-thumbnail');
    if (filmstripSlides.length > 0) {
      const slideParts = [];
      filmstripSlides.forEach((slide, index) => {
        const slideText = slide.textContent?.trim();
        if (slideText) {
          slideParts.push(`Slide ${index + 1}: ${slideText}`);
        }
      });
      content = slideParts.join('\n');
    }
  }
  
  // Try speaker notes
  if (!content) {
    const speakerNotes = document.querySelector('.punch-viewer-speakernotes-text, [data-speakernotes]');
    if (speakerNotes) {
      content = speakerNotes.textContent?.trim() || '';
    }
  }
  
  if (!content) {
    return null;
  }
  
  return {
    source: 'gslides',
    url: window.location.href,
    content: content,
    title: title,
    timestamp: nowUnixSeconds()
  };
}

// ============================================
// GOOGLE GEMINI EXTRACTOR
// ============================================

async function extractGemini() {
  console.log('[ContentScript] Starting Gemini extraction...');
  
  // Wait for conversation content to load
  await waitForGeminiContent();
  
  // Give the page time to fully render
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Extract conversation ID from URL for dedup
  const conversationId = extractGeminiConversationId(window.location.href);
  
  // Extract the conversation
  const conversation = extractGeminiConversation();
  
  if (!conversation || !conversation.content) {
    console.log('[ContentScript] No Gemini conversation content found');
    return null;
  }
  
  // Create a canonical URL for dedup - use conversation ID if available
  const canonicalUrl = conversationId 
    ? `gemini://conversation/${conversationId}`
    : `gemini://conversation/${generateConversationFingerprint(conversation.content)}`;
  
  return {
    source: 'gemini',
    url: canonicalUrl,
    content: conversation.content,
    title: conversation.title || 'Gemini Conversation',
    timestamp: nowUnixSeconds()
  };
}

/**
 * Wait for Gemini conversation content to load
 */
function waitForGeminiContent(timeout = 5000) {
  const selectors = [
    'message-content',              // Gemini message content element
    '[data-message-id]',            // Message with ID
    '.conversation-container',      // Conversation container
    '.model-response',              // Model response
    '.user-query',                  // User query
    'model-response',               // Custom element for model response
    'user-query'                    // Custom element for user query
  ];
  
  return new Promise((resolve) => {
    // Check if any element already exists
    for (const selector of selectors) {
      const element = document.querySelector(selector);
      if (element) {
        resolve(element);
        return;
      }
    }
    
    // Set up observer for dynamic content
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of selectors) {
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
 * Extract conversation ID from Gemini URL
 * Pattern: gemini.google.com/app/CONVERSATION_ID or /c/CONVERSATION_ID
 */
function extractGeminiConversationId(url) {
  try {
    const patterns = [
      /\/app\/([a-zA-Z0-9_-]+)/,
      /\/c\/([a-zA-Z0-9_-]+)/,
      /[?&]c=([a-zA-Z0-9_-]+)/
    ];
    
    for (const pattern of patterns) {
      const match = url.match(pattern);
      if (match) return match[1];
    }
  } catch (e) {
    console.log('[ContentScript] Error extracting Gemini conversation ID:', e);
  }
  return null;
}

/**
 * Generate a fingerprint for conversation content (for dedup when no ID available)
 * Uses first user query to create stable identifier
 */
function generateConversationFingerprint(content) {
  // Extract first user message for fingerprint
  const firstUserMatch = content.match(/\[User\]\s*(.{0,100})/);
  if (firstUserMatch) {
    // Simple hash of first user query
    let hash = 0;
    const str = firstUserMatch[1].trim().toLowerCase();
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return Math.abs(hash).toString(36);
  }
  return Date.now().toString(36);
}

/**
 * Extract conversation content from Gemini DOM
 */
function extractGeminiConversation() {
  const parts = [];
  let title = '';
  
  // Try multiple selector strategies for Gemini's DOM structure
  
  // Strategy 1: Look for message-content custom elements
  const messageContents = document.querySelectorAll('message-content');
  if (messageContents.length > 0) {
    messageContents.forEach((msg, index) => {
      const isUser = msg.closest('user-query') || msg.closest('[data-user-message]');
      const role = isUser ? 'User' : 'Gemini';
      const text = msg.textContent?.trim();
      if (text) {
        parts.push(`[${role}]\n${text}`);
        // Use first user query as title
        if (!title && isUser) {
          title = text.substring(0, 100) + (text.length > 100 ? '...' : '');
        }
      }
    });
  }
  
  // Strategy 2: Look for model-response and user-query elements
  if (parts.length === 0) {
    const userQueries = document.querySelectorAll('user-query, [data-user-message], .user-query');
    const modelResponses = document.querySelectorAll('model-response, [data-model-response], .model-response');
    
    // Interleave user queries and model responses
    const allMessages = [];
    userQueries.forEach(q => allMessages.push({ type: 'user', el: q }));
    modelResponses.forEach(r => allMessages.push({ type: 'model', el: r }));
    
    // Sort by DOM position
    allMessages.sort((a, b) => {
      const pos = a.el.compareDocumentPosition(b.el);
      return pos & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1;
    });
    
    allMessages.forEach(msg => {
      const text = msg.el.textContent?.trim();
      if (text) {
        const role = msg.type === 'user' ? 'User' : 'Gemini';
        parts.push(`[${role}]\n${text}`);
        if (!title && msg.type === 'user') {
          title = text.substring(0, 100) + (text.length > 100 ? '...' : '');
        }
      }
    });
  }
  
  // Strategy 3: Look for generic conversation structure
  if (parts.length === 0) {
    const conversationTurns = document.querySelectorAll('[data-message-id], .conversation-turn, .chat-message');
    conversationTurns.forEach(turn => {
      const isUser = turn.classList.contains('user') || 
                     turn.getAttribute('data-role') === 'user' ||
                     turn.querySelector('.user-avatar, [data-user]');
      const role = isUser ? 'User' : 'Gemini';
      const text = turn.textContent?.trim();
      if (text) {
        parts.push(`[${role}]\n${text}`);
        if (!title && isUser) {
          title = text.substring(0, 100) + (text.length > 100 ? '...' : '');
        }
      }
    });
  }
  
  if (parts.length === 0) {
    return null;
  }
  
  return {
    content: parts.join('\n\n'),
    title: title || 'Gemini Conversation'
  };
}

// ============================================
// GOOGLE AI MODE EXTRACTOR
// ============================================

async function extractGoogleAIMode() {
  console.log('[ContentScript] Starting Google AI Mode extraction...');
  
  // Wait for AI content to load
  await waitForGoogleAIContent();
  
  // Give the page time to fully render
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Extract the AI conversation/overview
  const aiContent = extractGoogleAIContent();
  
  if (!aiContent || !aiContent.content) {
    console.log('[ContentScript] No Google AI Mode content found, falling back to generic');
    return extractGoogleSearch();
  }
  
  // Create a canonical URL based on the search query for dedup
  const searchQuery = new URL(window.location.href).searchParams.get('q') || '';
  const canonicalUrl = `google-ai://search/${encodeURIComponent(searchQuery.toLowerCase().trim())}`;
  
  return {
    source: 'google-ai',
    url: canonicalUrl,
    content: aiContent.content,
    title: aiContent.title || `AI Mode: ${searchQuery}`,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Extract content from regular Google Search (with potential AI overview)
 */
async function extractGoogleSearch() {
  console.log('[ContentScript] Starting Google Search extraction...');
  
  const searchQuery = new URL(window.location.href).searchParams.get('q') || '';
  
  // Check for AI overview first
  const aiOverview = extractGoogleAIOverview();
  
  // Extract regular search results
  const searchResults = extractGoogleSearchResults();
  
  let content = '';
  if (aiOverview) {
    content = `## AI Overview\n${aiOverview}\n\n`;
  }
  if (searchResults) {
    content += `## Search Results\n${searchResults}`;
  }
  
  if (!content.trim()) {
    return extractGeneric();
  }
  
  // Use query as canonical URL for dedup
  const canonicalUrl = `google://search/${encodeURIComponent(searchQuery.toLowerCase().trim())}`;
  
  return {
    source: 'google-search',
    url: canonicalUrl,
    content: content.trim(),
    title: `Search: ${searchQuery}`,
    timestamp: nowUnixSeconds()
  };
}

/**
 * Wait for Google AI Mode content to load
 */
function waitForGoogleAIContent(timeout = 5000) {
  const selectors = [
    '[data-attrid="AIOverview"]',   // AI Overview container
    '.AI-overview',                  // AI overview class
    '[jsname="N760b"]',             // AI mode container
    '.wDYxhc',                      // Featured snippet / AI content
    '[data-async-context*="ai"]',   // Async AI content
    '.kp-wholepage',                // Knowledge panel (often contains AI)
    '.aimode-response',             // AI mode response
    '[data-hveid]'                  // Search results loaded indicator
  ];
  
  return new Promise((resolve) => {
    // Check if any element already exists
    for (const selector of selectors) {
      const element = document.querySelector(selector);
      if (element) {
        resolve(element);
        return;
      }
    }
    
    // Set up observer for dynamic content
    const observer = new MutationObserver((mutations, obs) => {
      for (const selector of selectors) {
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
 * Extract Google AI Mode conversation content
 */
function extractGoogleAIContent() {
  const parts = [];
  let title = '';
  
  // Get the search query as title
  const searchQuery = new URL(window.location.href).searchParams.get('q') || '';
  title = `AI Mode: ${searchQuery}`;
  
  // Strategy 1: Look for AI Mode specific containers
  const aiModeContainers = document.querySelectorAll(
    '[data-attrid="AIOverview"], .AI-overview, [jsname="N760b"], .aimode-response'
  );
  
  if (aiModeContainers.length > 0) {
    aiModeContainers.forEach(container => {
      // Look for user queries and AI responses within the container
      const userQueries = container.querySelectorAll('[data-user-query], .user-query-text');
      const aiResponses = container.querySelectorAll('[data-ai-response], .ai-response-text, .wDYxhc');
      
      // If we find structured Q&A
      if (userQueries.length > 0 || aiResponses.length > 0) {
        userQueries.forEach(q => {
          const text = q.textContent?.trim();
          if (text) parts.push(`[User]\n${text}`);
        });
        aiResponses.forEach(r => {
          const text = r.textContent?.trim();
          if (text) parts.push(`[AI]\n${text}`);
        });
      } else {
        // Just get all text content
        const text = container.textContent?.trim();
        if (text) parts.push(text);
      }
    });
  }
  
  // Strategy 2: Look for the main AI response area
  if (parts.length === 0) {
    // Google AI Mode often uses these containers
    const responseContainers = document.querySelectorAll(
      '.kp-wholepage .wDYxhc, [data-md] .wDYxhc, .ifM9O .wDYxhc'
    );
    
    responseContainers.forEach(container => {
      const text = container.textContent?.trim();
      if (text && text.length > 50) { // Filter out small snippets
        parts.push(text);
      }
    });
  }
  
  // Strategy 3: Look for conversational turns in AI mode
  if (parts.length === 0) {
    const turns = document.querySelectorAll('[data-turn], .conversation-turn');
    turns.forEach(turn => {
      const isUser = turn.getAttribute('data-role') === 'user' || 
                     turn.classList.contains('user-turn');
      const role = isUser ? 'User' : 'AI';
      const text = turn.textContent?.trim();
      if (text) {
        parts.push(`[${role}]\n${text}`);
      }
    });
  }
  
  if (parts.length === 0) {
    return null;
  }
  
  return {
    content: parts.join('\n\n'),
    title: title
  };
}

/**
 * Extract AI Overview from regular Google Search
 */
function extractGoogleAIOverview() {
  const overviewSelectors = [
    '[data-attrid="AIOverview"]',
    '.AI-overview',
    '[data-async-context*="ai_overview"]',
    '.kp-wholepage .wDYxhc'
  ];
  
  for (const selector of overviewSelectors) {
    const element = document.querySelector(selector);
    if (element) {
      const text = element.textContent?.trim();
      if (text && text.length > 100) {
        return text;
      }
    }
  }
  
  return null;
}

/**
 * Extract regular Google Search results
 */
function extractGoogleSearchResults() {
  const results = [];
  
  // Get organic search results
  const searchResults = document.querySelectorAll('.g, [data-hveid] .tF2Cxc');
  
  searchResults.forEach((result, index) => {
    if (index >= 10) return; // Limit to top 10 results
    
    const titleEl = result.querySelector('h3');
    const snippetEl = result.querySelector('.VwiC3b, .IsZvec');
    const linkEl = result.querySelector('a[href]');
    
    const title = titleEl?.textContent?.trim();
    const snippet = snippetEl?.textContent?.trim();
    const link = linkEl?.href;
    
    if (title && snippet) {
      results.push(`${index + 1}. ${title}\n   ${snippet}\n   ${link || ''}`);
    }
  });
  
  return results.join('\n\n');
}

async function extractDiscord() {
  await waitForContent('[class*="messagesWrapper"], [class*="message-"]');

  const channelHeader = document.querySelector('[class*="title-"][class*="container-"]');
  const channelName = channelHeader?.textContent?.trim() || 'unknown-channel';

  const messageEls = document.querySelectorAll('[id^="chat-messages-"] > [class*="message-"]');
  const messages = [];

  messageEls.forEach((msg) => {
    const authorEl = msg.querySelector('[class*="username-"]');
    const contentEl = msg.querySelector('[class*="messageContent-"]');
    const timestampEl = msg.querySelector('time');

    const author = authorEl?.textContent?.trim() || 'unknown';
    const text = contentEl?.textContent?.trim() || '';
    const timestamp = timestampEl?.getAttribute('datetime') || '';
    const timeStr = formatTimestamp(timestamp);

    if (text) {
      messages.push(`[${author} ${timeStr}] ${text}`);
    }
  });

  return {
    source: 'discord',
    url: window.location.href,
    content: messages.join('\n'),
    title: channelName,
    channel: channelName,
    timestamp: nowUnixSeconds()
  };
}

async function extractGeneric() {
  const title = document.title || 'Untitled Page';

  const mainContent = document.querySelector(
    'main, article, [role="main"], .content, .post, .article, #content, #main'
  ) || document.body;

  const clone = mainContent.cloneNode(true);
  const unwanted = clone.querySelectorAll(
    'script, style, nav, header, footer, aside, [role="navigation"], ' +
    '[role="banner"], [role="contentinfo"], .sidebar, .nav, .menu, ' +
    '.advertisement, .ad, .social-share, .comments, iframe'
  );
  unwanted.forEach((el) => el.remove());

  const content = clone.textContent?.replace(/\s+/g, ' ').trim().substring(0, 50000) || '';
  const metaDesc = document.querySelector('meta[name="description"]');
  const description = metaDesc?.getAttribute('content') || '';

  // Try to find author
  const authorEl = document.querySelector('[rel="author"], .author, .byline, [itemprop="author"]');
  const author = authorEl?.textContent?.trim();

  return {
    source: 'browser',
    url: window.location.href,
    content: description ? `${description}\n\n${content}` : content,
    title: title,
    author: author,
    timestamp: nowUnixSeconds()
  };
}

// ============================================
// MESSAGE HANDLER
// ============================================

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'ping') {
    sendResponse({ pong: true });
    return false;
  }

  if (message.type !== 'extract') return false;

  console.log('[ContentScript] Extraction requested for:', message.url);

  (async () => {
    try {
      const extractor = getExtractor(message.url);
      const payload = await extractor();
      
      if (!payload || !payload.content) {
        sendResponse({ success: false, reason: 'No content extracted' });
        return;
      }

      console.log('[ContentScript] Extracted:', payload.source, '-', payload.title || payload.url);
      sendResponse({ success: true, data: payload });
    } catch (error) {
      console.error('[ContentScript] Extraction error:', error);
      sendResponse({ success: false, reason: error.message });
    }
  })();

  return true; // Async response
});

console.log('[ContentScript] Loaded and ready');
