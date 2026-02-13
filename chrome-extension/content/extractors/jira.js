/**
 * Jira Content Extractor
 * 
 * Handles extraction from Jira Cloud's React-based SPA.
 * Uses multiple strategies:
 * 1. REST API (most reliable) - uses existing session cookies
 * 2. DOM extraction with MutationObserver for dynamic content
 * 3. URL parsing for issue keys
 * 
 * Supports:
 * - Issue detail view (full page and modal/panel)
 * - Board view (Scrum/Kanban boards)
 * - Backlog view
 * - Issue navigator/search results
 */

// ============================================
// CONFIGURATION
// ============================================

const JIRA_CONFIG = {
  // Timeouts
  DOM_WAIT_TIMEOUT: 5000,
  API_TIMEOUT: 10000,
  
  // DOM Selectors - multiple fallbacks for different Jira versions/views
  selectors: {
    // Issue detail panel/modal selectors
    issueDetail: [
      '[data-testid="issue.views.issue-details.issue-layout"]',
      '[data-testid="issue-detail"]',
      '[data-testid="issue.views.issue-base.foundation.summary.heading"]',
      '[role="dialog"][aria-label*="issue"]',
      '.issue-view',
      '#jira-issue-header'
    ],
    
    // Issue key selectors
    issueKey: [
      '[data-testid="issue.views.issue-base.foundation.breadcrumbs.current-issue.item"]',
      '[data-testid="issue.views.issue-base.foundation.breadcrumbs.breadcrumb-current-issue-container"] a',
      '[data-testid="issue-key-link"]',
      'a[href*="/browse/"][data-testid]',
      '.issue-link',
      '[data-issue-key]'
    ],
    
    // Summary/title selectors
    summary: [
      '[data-testid="issue.views.issue-base.foundation.summary.heading"]',
      '[data-testid="issue.views.issue-base.foundation.summary"] h1',
      'h1[data-testid*="summary"]',
      '.issue-header-content h1',
      '#summary-val'
    ],
    
    // Description selectors
    description: [
      '[data-testid="issue.views.field.rich-text.description"]',
      '[data-testid="issue.views.field.rich-text.description"] [data-renderer-start-pos]',
      '[data-testid="issue-description"]',
      '.user-content-block',
      '#description-val'
    ],
    
    // Status selectors
    status: [
      '[data-testid="issue.views.issue-base.foundation.status.status-field-wrapper"]',
      '[data-testid="issue.views.issue-base.foundation.status.status-field-wrapper"] button',
      '[data-testid="issue-status"]',
      '.status-lozenge',
      '#status-val'
    ],
    
    // Assignee selectors
    assignee: [
      '[data-testid="issue.views.field.user.assignee"]',
      '[data-testid="issue.views.field.user.assignee"] span[role="img"]',
      '[data-testid="assignee-field"]',
      '.assignee-field',
      '#assignee-val'
    ],
    
    // Priority selectors
    priority: [
      '[data-testid="issue.views.field.priority"]',
      '[data-testid="issue-field-priority"]',
      '.priority-field',
      '#priority-val'
    ],
    
    // Issue type selectors
    issueType: [
      '[data-testid="issue.views.issue-base.foundation.breadcrumbs.issue-type"]',
      '[data-testid="issue-type-icon"]',
      '.issue-type-icon'
    ],
    
    // Comments selectors
    comments: [
      '[data-testid="issue-activity-feed.ui.activity-feed"] [data-testid*="comment"]',
      '[data-testid="issue.activity.comments-list"]',
      '.activity-comment'
    ],
    
    // Board card selectors
    boardCard: [
      '[data-testid="platform-board-kit.ui.card.card"]',
      '[data-testid="board-card"]',
      '[data-testid="software-board.board-container.board.card-container.card"]',
      '.ghx-issue'
    ],
    
    // Board card key
    boardCardKey: [
      '[data-testid="platform-card.common.ui.key"]',
      '[data-testid="software-board.board-container.board.card-container.card.key"]',
      '.ghx-key'
    ],
    
    // Board card summary
    boardCardSummary: [
      '[data-testid="platform-card.common.ui.summary"]',
      '[data-testid="software-board.board-container.board.card-container.card.summary"]',
      '.ghx-summary'
    ],
    
    // Backlog item selectors
    backlogItem: [
      '[data-test-id="software-backlog.card-list.card.card-contents.accessible-card-key"]',
      '[data-testid="software-backlog.backlog-content.card"]',
      '.js-issue'
    ],
    
    // Project key
    projectKey: [
      '[data-testid="navigation-apps.project-switcher-v2.project-switcher.menu-trigger"]',
      '[data-testid="project-name"]',
      '.project-title'
    ],
    
    // Board name
    boardName: [
      '[data-testid="board-header.ui.board-name"]',
      '[data-testid="software-board.header.title"]',
      '.ghx-board-name'
    ]
  }
};

// ============================================
// UTILITIES
// ============================================

/**
 * Wait for an element to appear in the DOM using MutationObserver
 */
function waitForElement(selectors, timeout = JIRA_CONFIG.DOM_WAIT_TIMEOUT) {
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
    
    // Timeout fallback
    setTimeout(() => {
      observer.disconnect();
      // Final check before giving up
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
 * Extract text content safely
 */
function getText(element) {
  if (!element) return '';
  return element.textContent?.trim() || element.innerText?.trim() || '';
}

/**
 * Extract issue key from URL
 */
function extractIssueKeyFromUrl(url) {
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
  
  return null;
}

/**
 * Get base URL for API calls
 */
function getBaseUrl() {
  const url = new URL(window.location.href);
  return `${url.protocol}//${url.host}`;
}

/**
 * Get current Unix timestamp in seconds
 */
function nowUnixSeconds() {
  return Math.floor(Date.now() / 1000);
}

// ============================================
// REST API EXTRACTION (Primary Strategy)
// ============================================

/**
 * Fetch issue details via REST API using session cookies
 * This is the most reliable method as it doesn't depend on DOM structure
 */
async function fetchIssueViaApi(issueKey) {
  const baseUrl = getBaseUrl();
  const apiUrl = `${baseUrl}/rest/api/3/issue/${issueKey}?expand=renderedFields,names`;
  
  try {
    const response = await fetch(apiUrl, {
      method: 'GET',
      credentials: 'include', // Include session cookies
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      }
    });
    
    if (!response.ok) {
      console.log(`[JiraExtractor] API request failed: ${response.status}`);
      return null;
    }
    
    const data = await response.json();
    return parseApiResponse(data);
  } catch (error) {
    console.log('[JiraExtractor] API fetch error:', error.message);
    return null;
  }
}

/**
 * Parse Jira REST API response into our payload format
 */
function parseApiResponse(data) {
  const fields = data.fields || {};
  const renderedFields = data.renderedFields || {};
  
  // Build content string
  const parts = [];
  
  // Summary
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
  
  // Description - prefer rendered HTML converted to text, fallback to plain
  let description = '';
  if (renderedFields.description) {
    // Create temp element to extract text from HTML
    const temp = document.createElement('div');
    temp.innerHTML = renderedFields.description;
    description = temp.textContent?.trim() || '';
  } else if (fields.description) {
    // Handle ADF format
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
  
  // Comments (if available)
  if (fields.comment?.comments?.length > 0) {
    parts.push('## Comments');
    fields.comment.comments.slice(-5).forEach(comment => {
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
    
    if (node.type === 'text') {
      return node.text || '';
    }
    
    if (node.content && Array.isArray(node.content)) {
      return node.content.map(extractFromNode).join('');
    }
    
    return '';
  };
  
  const text = extractFromNode(adf);
  return text.trim();
}

// ============================================
// DOM EXTRACTION (Fallback Strategy)
// ============================================

/**
 * Extract issue details from DOM
 */
async function extractIssueFromDom() {
  const s = JIRA_CONFIG.selectors;
  
  // Wait for issue detail to load
  const issueDetail = await waitForElement(s.issueDetail);
  if (!issueDetail) {
    console.log('[JiraExtractor] Issue detail not found in DOM');
    return null;
  }
  
  // Give React a moment to fully render
  await new Promise(resolve => setTimeout(resolve, 500));
  
  // Extract issue key
  let issueKey = '';
  const keyEl = queryWithFallbacks(s.issueKey);
  if (keyEl) {
    issueKey = getText(keyEl);
    // Clean up - sometimes includes extra text
    const keyMatch = issueKey.match(/([A-Z][A-Z0-9]+-\d+)/i);
    if (keyMatch) issueKey = keyMatch[1];
  }
  
  // Fallback to URL
  if (!issueKey) {
    issueKey = extractIssueKeyFromUrl(window.location.href) || 'UNKNOWN';
  }
  
  // Extract summary
  const summaryEl = queryWithFallbacks(s.summary);
  const summary = getText(summaryEl);
  
  // Extract description
  const descEl = queryWithFallbacks(s.description);
  const description = getText(descEl);
  
  // Extract status
  const statusEl = queryWithFallbacks(s.status);
  const status = getText(statusEl);
  
  // Extract assignee
  const assigneeEl = queryWithFallbacks(s.assignee);
  let assignee = getText(assigneeEl);
  // Clean up assignee text (often includes "Assignee" label)
  assignee = assignee.replace(/^Assignee\s*/i, '').trim();
  if (assignee === 'Unassigned') assignee = '';
  
  // Extract priority
  const priorityEl = queryWithFallbacks(s.priority);
  const priority = getText(priorityEl);
  
  // Extract issue type
  const typeEl = queryWithFallbacks(s.issueType);
  const issueType = getText(typeEl);
  
  // Extract project
  const projectEl = queryWithFallbacks(s.projectKey);
  const project = getText(projectEl);
  
  // Build content
  const parts = [];
  parts.push(`# ${issueKey}: ${summary}`);
  parts.push('');
  
  const metadata = [];
  if (status) metadata.push(`Status: ${status}`);
  if (priority) metadata.push(`Priority: ${priority}`);
  if (issueType) metadata.push(`Type: ${issueType}`);
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
    channel: project || undefined,
    timestamp: nowUnixSeconds()
  };
}

// ============================================
// BOARD VIEW EXTRACTION
// ============================================

/**
 * Extract board view (list of issues)
 */
async function extractBoardView() {
  const s = JIRA_CONFIG.selectors;
  
  // Wait for board cards to load
  await waitForElement(s.boardCard);
  await new Promise(resolve => setTimeout(resolve, 500));
  
  const cards = queryAllWithFallbacks(s.boardCard);
  if (cards.length === 0) {
    console.log('[JiraExtractor] No board cards found');
    return null;
  }
  
  // Get board name
  const boardNameEl = queryWithFallbacks(s.boardName);
  const boardName = getText(boardNameEl) || 'Jira Board';
  
  // Extract issues from cards
  const issues = [];
  cards.forEach(card => {
    const keyEl = card.querySelector(s.boardCardKey.join(', '));
    const summaryEl = card.querySelector(s.boardCardSummary.join(', '));
    
    const key = getText(keyEl);
    const summary = getText(summaryEl);
    
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

// ============================================
// BACKLOG VIEW EXTRACTION
// ============================================

/**
 * Extract backlog view
 */
async function extractBacklogView() {
  const s = JIRA_CONFIG.selectors;
  
  await waitForElement(s.backlogItem);
  await new Promise(resolve => setTimeout(resolve, 500));
  
  const items = queryAllWithFallbacks(s.backlogItem);
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

// ============================================
// MAIN EXPORT
// ============================================

/**
 * Main Jira extraction function
 * Tries multiple strategies in order of reliability
 */
async function extractJira() {
  console.log('[JiraExtractor] Starting extraction...');
  
  const url = window.location.href;
  const issueKey = extractIssueKeyFromUrl(url);
  
  // Strategy 1: If we have an issue key, try REST API first (most reliable)
  if (issueKey) {
    console.log('[JiraExtractor] Found issue key:', issueKey);
    const apiResult = await fetchIssueViaApi(issueKey);
    if (apiResult && apiResult.content) {
      console.log('[JiraExtractor] API extraction successful');
      return apiResult;
    }
  }
  
  // Strategy 2: Try DOM extraction for issue detail
  const domResult = await extractIssueFromDom();
  if (domResult && domResult.content) {
    console.log('[JiraExtractor] DOM extraction successful');
    return domResult;
  }
  
  // Strategy 3: Check if this is a board view
  if (url.includes('/boards/') || url.includes('/board/')) {
    const boardResult = await extractBoardView();
    if (boardResult && boardResult.content) {
      console.log('[JiraExtractor] Board extraction successful');
      return boardResult;
    }
  }
  
  // Strategy 4: Check if this is a backlog view
  if (url.includes('/backlog')) {
    const backlogResult = await extractBacklogView();
    if (backlogResult && backlogResult.content) {
      console.log('[JiraExtractor] Backlog extraction successful');
      return backlogResult;
    }
  }
  
  console.log('[JiraExtractor] All extraction strategies failed');
  return null;
}

// Export for use in content script
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { extractJira };
}
