/**
 * SourceLogo component - Renders appropriate logo based on source type
 * 
 * Requirements: 4.1-4.5
 * - 4.1: Slack logo for source_type "slack"
 * - 4.2: Gmail logo for source_type "gmail"
 * - 4.3: Jira logo for source_type "jira"
 * - 4.4: Browser/globe icon for source_type "browser"
 * - 4.5: Generic document icon for unrecognized source_type
 */

import React from 'react';
import { getSourceConfig, LogoType } from '../utils/sourceConfig';
import type { SourceType } from '../types';

export interface SourceLogoProps {
  sourceType: SourceType;
  size?: 'sm' | 'md' | 'lg';
}

/**
 * Size mappings for logo dimensions
 */
const SIZE_MAP = {
  sm: 24,
  md: 32,
  lg: 48,
} as const;

/**
 * Slack logo SVG component
 */
const SlackLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Slack"
  >
    <path
      d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zM8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zM18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zM15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"
      fill="#E01E5A"
    />
    <path
      d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313z"
      fill="#E01E5A"
    />
    <path
      d="M8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312z"
      fill="#36C5F0"
    />
    <path
      d="M18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312z"
      fill="#2EB67D"
    />
    <path
      d="M15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"
      fill="#ECB22E"
    />
  </svg>
);

/**
 * Gmail logo SVG component
 */
const GmailLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Gmail"
  >
    <path
      d="M24 5.457v13.909c0 .904-.732 1.636-1.636 1.636h-3.819V11.73L12 16.64l-6.545-4.91v9.273H1.636A1.636 1.636 0 0 1 0 19.366V5.457c0-2.023 2.309-3.178 3.927-1.964L5.455 4.64 12 9.548l6.545-4.91 1.528-1.145C21.69 2.28 24 3.434 24 5.457z"
      fill="#EA4335"
    />
  </svg>
);

/**
 * Jira logo SVG component
 */
const JiraLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Jira"
  >
    <path
      d="M11.571 11.513H0a5.218 5.218 0 0 0 5.232 5.215h2.13v2.057A5.215 5.215 0 0 0 12.575 24V12.518a1.005 1.005 0 0 0-1.005-1.005z"
      fill="#2684FF"
    />
    <path
      d="M11.571 11.513H0a5.218 5.218 0 0 0 5.232 5.215h2.13v2.057A5.215 5.215 0 0 0 12.575 24V12.518a1.005 1.005 0 0 0-1.005-1.005z"
      fill="url(#jira-gradient-1)"
    />
    <path
      d="M17.357 5.756H5.786a5.218 5.218 0 0 0 5.232 5.215h2.129v2.058a5.218 5.218 0 0 0 5.215 5.214V6.762a1.006 1.006 0 0 0-1.005-1.006z"
      fill="#2684FF"
    />
    <path
      d="M17.357 5.756H5.786a5.218 5.218 0 0 0 5.232 5.215h2.129v2.058a5.218 5.218 0 0 0 5.215 5.214V6.762a1.006 1.006 0 0 0-1.005-1.006z"
      fill="url(#jira-gradient-2)"
    />
    <path
      d="M23.143 0H11.571a5.218 5.218 0 0 0 5.232 5.215h2.13v2.057A5.215 5.215 0 0 0 24.146 12.5V1.005A1.005 1.005 0 0 0 23.143 0z"
      fill="#2684FF"
    />
    <defs>
      <linearGradient
        id="jira-gradient-1"
        x1="11.252"
        y1="11.647"
        x2="6.63"
        y2="16.312"
        gradientUnits="userSpaceOnUse"
      >
        <stop offset="0.18" stopColor="#0052CC" />
        <stop offset="1" stopColor="#2684FF" />
      </linearGradient>
      <linearGradient
        id="jira-gradient-2"
        x1="17.416"
        y1="5.828"
        x2="12.178"
        y2="11.066"
        gradientUnits="userSpaceOnUse"
      >
        <stop offset="0.18" stopColor="#0052CC" />
        <stop offset="1" stopColor="#2684FF" />
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Browser/Globe icon SVG component
 */
const BrowserLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Browser"
  >
    <circle cx="12" cy="12" r="10" stroke="#5F6368" strokeWidth="2" />
    <ellipse cx="12" cy="12" rx="4" ry="10" stroke="#5F6368" strokeWidth="2" />
    <path d="M2 12h20" stroke="#5F6368" strokeWidth="2" />
    <path d="M12 2v20" stroke="#5F6368" strokeWidth="2" />
  </svg>
);

/**
 * Generic document icon SVG component
 */
const DocumentLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Document"
  >
    <path
      d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6z"
      stroke="#6B7280"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <path
      d="M14 2v6h6"
      stroke="#6B7280"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <path
      d="M16 13H8M16 17H8M10 9H8"
      stroke="#6B7280"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

/**
 * Chrome logo using SVG image
 */
const ChromeLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/chrome-logo.svg"
    width={size}
    height={size}
    alt="Chrome"
    aria-label="Chrome"
  />
);

/**
 * Google Docs logo using SVG image
 */
const GDocsLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/docs.svg"
    width={size}
    height={size}
    alt="Google Docs"
    aria-label="Google Docs"
  />
);

/**
 * Google Sheets logo using SVG image
 */
const GSheetsLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/sheets.svg"
    width={size}
    height={size}
    alt="Google Sheets"
    aria-label="Google Sheets"
  />
);

/**
 * Google Slides logo using SVG image
 */
const GSlidesLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/slides-icon.svg"
    width={size}
    height={size}
    alt="Google Slides"
    aria-label="Google Slides"
  />
);

/**
 * Microsoft Word logo using SVG image
 */
const WordLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/word-logo.svg"
    width={size}
    height={size}
    alt="Microsoft Word"
    aria-label="Microsoft Word"
  />
);

/**
 * Microsoft Excel logo using SVG image
 */
const ExcelLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/excel.svg"
    width={size}
    height={size}
    alt="Microsoft Excel"
    aria-label="Microsoft Excel"
  />
);

/**
 * Microsoft PowerPoint logo SVG component
 */
const PowerPointLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="Microsoft PowerPoint"
  >
    <path
      d="M13.5 2H6C4.89543 2 4 2.89543 4 4V20C4 21.1046 4.89543 22 6 22H18C19.1046 22 20 21.1046 20 20V8.5L13.5 2Z"
      fill="#D24726"
    />
    <path
      d="M13.5 2V8.5H20L13.5 2Z"
      fill="#FF8F6B"
    />
    <path
      d="M9 11H12C13.1046 11 14 11.8954 14 13C14 14.1046 13.1046 15 12 15H10V17H9V11Z"
      fill="white"
    />
    <path
      d="M10 12V14H12C12.5523 14 13 13.5523 13 13C13 12.4477 12.5523 12 12 12H10Z"
      fill="#D24726"
    />
  </svg>
);

/**
 * Microsoft Outlook logo SVG component
 */
const OutlookLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Microsoft Outlook">
    <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z" fill="#0078D4"/>
    <path d="M12 6c-3.31 0-6 2.69-6 6s2.69 6 6 6 6-2.69 6-6-2.69-6-6-6zm0 10c-2.21 0-4-1.79-4-4s1.79-4 4-4 4 1.79 4 4-1.79 4-4 4z" fill="white"/>
  </svg>
);

/**
 * Microsoft OneNote logo SVG component
 */
const OneNoteLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Microsoft OneNote">
    <rect x="3" y="3" width="18" height="18" rx="2" fill="#7719AA"/>
    <path d="M8 7v10h2V11.5l3 5.5h2V7h-2v5.5L10 7H8z" fill="white"/>
  </svg>
);

/**
 * Microsoft Teams logo SVG component
 */
const TeamsLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Microsoft Teams">
    <path d="M20.5 7.5a2 2 0 100-4 2 2 0 000 4z" fill="#5059C9"/>
    <path d="M22 9h-5a1 1 0 00-1 1v6a3 3 0 006 0v-6a1 1 0 00-1-1h1z" fill="#5059C9"/>
    <path d="M14.5 6.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5z" fill="#7B83EB"/>
    <path d="M17 8H9a1 1 0 00-1 1v8a4 4 0 008 0V9a1 1 0 00-1-1h2z" fill="#7B83EB"/>
    <rect x="2" y="8" width="10" height="12" rx="1" fill="#4B53BC"/>
    <path d="M5 11h4v1H8v5H6v-5H5v-1z" fill="white"/>
  </svg>
);

/**
 * Discord logo SVG component
 */
const DiscordLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Discord">
    <path d="M20.317 4.37a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 00-5.487 0 12.64 12.64 0 00-.617-1.25.077.077 0 00-.079-.037A19.736 19.736 0 003.677 4.37a.07.07 0 00-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 00.031.057 19.9 19.9 0 005.993 3.03.078.078 0 00.084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 00-.041-.106 13.107 13.107 0 01-1.872-.892.077.077 0 01-.008-.128 10.2 10.2 0 00.372-.292.074.074 0 01.077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 01.078.01c.12.098.246.198.373.292a.077.077 0 01-.006.127 12.299 12.299 0 01-1.873.892.077.077 0 00-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 00.084.028 19.839 19.839 0 006.002-3.03.077.077 0 00.032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 00-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" fill="#5865F2"/>
  </svg>
);

/**
 * Zoom logo SVG component
 */
const ZoomLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Zoom">
    <rect x="2" y="4" width="20" height="16" rx="3" fill="#2D8CFF"/>
    <path d="M6 9h6v1.5H7.5v1H12v1.5H7.5v1H12V15H6V9z" fill="white"/>
    <path d="M14 9l4 3-4 3V9z" fill="white"/>
  </svg>
);

/**
 * Safari logo SVG component
 */
const SafariLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Safari">
    <circle cx="12" cy="12" r="10" fill="url(#safari-gradient)"/>
    <path d="M12 4l1 8-8 1 7-9z" fill="white"/>
    <path d="M12 20l-1-8 8-1-7 9z" fill="#FF3B30"/>
    <defs>
      <linearGradient id="safari-gradient" x1="12" y1="2" x2="12" y2="22" gradientUnits="userSpaceOnUse">
        <stop stopColor="#19D7FF"/>
        <stop offset="1" stopColor="#1E64F0"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Firefox logo SVG component
 */
const FirefoxLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Firefox">
    <circle cx="12" cy="12" r="10" fill="url(#firefox-gradient)"/>
    <path d="M12 4c-4.4 0-8 3.6-8 8s3.6 8 8 8 8-3.6 8-8c0-1.5-.4-2.9-1.1-4.1-.3.5-.8.9-1.4 1.1.3.9.5 1.9.5 3 0 3.3-2.7 6-6 6s-6-2.7-6-6 2.7-6 6-6c.5 0 1 .1 1.5.2.2-.6.6-1.1 1.1-1.4C13.5 4.3 12.8 4 12 4z" fill="#FF9500"/>
    <defs>
      <linearGradient id="firefox-gradient" x1="12" y1="2" x2="12" y2="22" gradientUnits="userSpaceOnUse">
        <stop stopColor="#FF7139"/>
        <stop offset="1" stopColor="#FF3647"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Edge logo SVG component
 */
const EdgeLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Microsoft Edge">
    <path d="M12 2C6.48 2 2 6.48 2 12c0 4.84 3.44 8.87 8 9.8V15c-2.76 0-5-2.24-5-5 0-2.21 1.79-4 4-4h6c2.21 0 4 1.79 4 4 0 1.86-1.28 3.41-3 3.86v4.94c4.56-.93 8-4.96 8-9.8 0-5.52-4.48-10-10-10z" fill="url(#edge-gradient)"/>
    <defs>
      <linearGradient id="edge-gradient" x1="2" y1="12" x2="22" y2="12" gradientUnits="userSpaceOnUse">
        <stop stopColor="#0078D7"/>
        <stop offset="1" stopColor="#00BCF2"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Brave logo SVG component
 */
const BraveLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Brave">
    <path d="M12 2L4 6v6c0 5.55 3.84 10.74 8 12 4.16-1.26 8-6.45 8-12V6l-8-4z" fill="#FB542B"/>
    <path d="M12 4L6 7v5c0 4.17 2.88 8.06 6 9 3.12-.94 6-4.83 6-9V7l-6-3z" fill="white"/>
    <path d="M12 6l-4 2v4c0 2.78 1.92 5.37 4 6 2.08-.63 4-3.22 4-6V8l-4-2z" fill="#FB542B"/>
  </svg>
);

/**
 * Arc logo SVG component
 */
const ArcLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Arc">
    <rect x="3" y="3" width="18" height="18" rx="4" fill="url(#arc-gradient)"/>
    <path d="M8 16V8h2l2 5 2-5h2v8h-2v-5l-2 5-2-5v5H8z" fill="white"/>
    <defs>
      <linearGradient id="arc-gradient" x1="3" y1="3" x2="21" y2="21" gradientUnits="userSpaceOnUse">
        <stop stopColor="#FF6B6B"/>
        <stop offset="0.5" stopColor="#A855F7"/>
        <stop offset="1" stopColor="#3B82F6"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Apple Notes logo SVG component
 */
const NotesLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Notes">
    <rect x="3" y="2" width="18" height="20" rx="2" fill="#FFCC00"/>
    <path d="M6 6h12M6 10h12M6 14h8" stroke="white" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Apple Reminders logo SVG component
 */
const RemindersLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Reminders">
    <rect x="3" y="3" width="18" height="18" rx="4" fill="white" stroke="#FF9500" strokeWidth="2"/>
    <circle cx="8" cy="9" r="2" fill="#FF9500"/>
    <circle cx="8" cy="15" r="2" fill="#4CD964"/>
    <path d="M12 9h6M12 15h6" stroke="#333" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Apple Mail logo SVG component
 */
const MailLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Mail">
    <rect x="2" y="4" width="20" height="16" rx="2" fill="#007AFF"/>
    <path d="M2 6l10 7 10-7" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

/**
 * Finder logo SVG component
 */
const FinderLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Finder">
    <rect x="3" y="2" width="18" height="20" rx="2" fill="url(#finder-gradient)"/>
    <circle cx="9" cy="10" r="2" fill="white"/>
    <circle cx="15" cy="10" r="2" fill="white"/>
    <path d="M8 15c0 2 2 3 4 3s4-1 4-3" stroke="white" strokeWidth="2" strokeLinecap="round"/>
    <defs>
      <linearGradient id="finder-gradient" x1="12" y1="2" x2="12" y2="22" gradientUnits="userSpaceOnUse">
        <stop stopColor="#6DD5FA"/>
        <stop offset="1" stopColor="#2980B9"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Preview logo SVG component
 */
const PreviewLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Preview">
    <rect x="3" y="3" width="18" height="18" rx="2" fill="#007AFF"/>
    <circle cx="12" cy="10" r="4" fill="white"/>
    <path d="M12 14v4M9 18h6" stroke="white" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Pages logo SVG component
 */
const PagesLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Pages">
    <rect x="3" y="2" width="18" height="20" rx="2" fill="#FF9500"/>
    <path d="M7 7h10M7 11h10M7 15h6" stroke="white" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Numbers logo SVG component
 */
const NumbersLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Numbers">
    <rect x="3" y="3" width="18" height="18" rx="2" fill="#34C759"/>
    <rect x="6" y="6" width="5" height="5" fill="white"/>
    <rect x="13" y="6" width="5" height="5" fill="white"/>
    <rect x="6" y="13" width="5" height="5" fill="white"/>
    <rect x="13" y="13" width="5" height="5" fill="white"/>
  </svg>
);

/**
 * Keynote logo SVG component
 */
const KeynoteLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Keynote">
    <rect x="2" y="4" width="20" height="14" rx="2" fill="#007AFF"/>
    <rect x="8" y="18" width="8" height="2" fill="#333"/>
    <path d="M12 8l3 4H9l3-4z" fill="white"/>
  </svg>
);

/**
 * TextEdit logo SVG component
 */
const TextEditLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="TextEdit">
    <rect x="4" y="2" width="16" height="20" rx="1" fill="white" stroke="#8E8E93" strokeWidth="2"/>
    <path d="M7 6h10M7 10h10M7 14h6" stroke="#8E8E93" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Calculator logo SVG component
 */
const CalculatorLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Calculator">
    <rect x="4" y="2" width="16" height="20" rx="2" fill="#1C1C1E"/>
    <rect x="6" y="4" width="12" height="4" rx="1" fill="#FF9500"/>
    <circle cx="8" cy="12" r="1.5" fill="#636366"/>
    <circle cx="12" cy="12" r="1.5" fill="#636366"/>
    <circle cx="16" cy="12" r="1.5" fill="#FF9500"/>
    <circle cx="8" cy="16" r="1.5" fill="#636366"/>
    <circle cx="12" cy="16" r="1.5" fill="#636366"/>
    <circle cx="16" cy="16" r="1.5" fill="#FF9500"/>
    <circle cx="8" cy="20" r="1.5" fill="#636366"/>
    <circle cx="12" cy="20" r="1.5" fill="#636366"/>
  </svg>
);

/**
 * Terminal logo SVG component
 */
const TerminalLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Terminal">
    <rect x="2" y="3" width="20" height="18" rx="2" fill="#1C1C1E"/>
    <path d="M6 8l4 4-4 4" stroke="#34C759" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M12 16h6" stroke="#34C759" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * VS Code logo SVG component
 */
const VSCodeLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="VS Code">
    <path d="M17 2l5 4v12l-5 4-10-8 10-8V2z" fill="#007ACC"/>
    <path d="M17 6L7 14l-5-4v4l5 4 10-8V6z" fill="#1F9CF0"/>
    <path d="M2 10l5-4v12l-5-4V10z" fill="#0065A9"/>
  </svg>
);

/**
 * Kiro logo SVG component
 */
const KiroLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Kiro">
    <rect x="3" y="3" width="18" height="18" rx="4" fill="#FF6B35"/>
    <path d="M8 7v10M8 12l4-5M8 12l4 5M14 7v10" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

/**
 * Xcode logo SVG component
 */
const XcodeLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Xcode">
    <rect x="3" y="3" width="18" height="18" rx="4" fill="#147EFB"/>
    <path d="M8 8l8 8M16 8l-8 8" stroke="white" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * JetBrains logo SVG component
 */
const JetBrainsLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="JetBrains">
    <rect x="3" y="3" width="18" height="18" fill="#000"/>
    <path d="M6 17h6v1H6v-1z" fill="white"/>
    <path d="M6 6h2v6H6V6z" fill="url(#jb-gradient)"/>
    <defs>
      <linearGradient id="jb-gradient" x1="6" y1="6" x2="8" y2="12" gradientUnits="userSpaceOnUse">
        <stop stopColor="#FF0087"/>
        <stop offset="1" stopColor="#FF7700"/>
      </linearGradient>
    </defs>
  </svg>
);

/**
 * Notion logo SVG component
 */
const NotionLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Notion">
    <rect x="4" y="2" width="16" height="20" rx="2" fill="white" stroke="#000" strokeWidth="2"/>
    <path d="M8 6h8M8 10h8M8 14h4" stroke="#000" strokeWidth="2" strokeLinecap="round"/>
  </svg>
);

/**
 * Figma logo SVG component
 */
const FigmaLogo: React.FC<{ size: number }> = ({ size }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Figma">
    <path d="M8 24c2.2 0 4-1.8 4-4v-4H8c-2.2 0-4 1.8-4 4s1.8 4 4 4z" fill="#0ACF83"/>
    <path d="M4 12c0-2.2 1.8-4 4-4h4v8H8c-2.2 0-4-1.8-4-4z" fill="#A259FF"/>
    <path d="M4 4c0-2.2 1.8-4 4-4h4v8H8C5.8 8 4 6.2 4 4z" fill="#F24E1E"/>
    <path d="M12 0h4c2.2 0 4 1.8 4 4s-1.8 4-4 4h-4V0z" fill="#FF7262"/>
    <circle cx="16" cy="12" r="4" fill="#1ABCFE"/>
  </svg>
);

/**
 * Claude logo SVG component
 */
const ClaudeLogo: React.FC<{ size: number }> = ({ size }) => (
  <img
    src="/logo/claude-color.svg"
    width={size}
    height={size}
    alt="Claude"
    aria-label="Claude"
  />
);

/**
 * Logo component mapping
 */
const LOGO_COMPONENTS: Record<string, React.FC<{ size: number }>> = {
  // Communication
  [LogoType.SLACK]: SlackLogo,
  [LogoType.GMAIL]: GmailLogo,
  [LogoType.TEAMS]: TeamsLogo,
  [LogoType.DISCORD]: DiscordLogo,
  [LogoType.ZOOM]: ZoomLogo,
  [LogoType.MAIL]: MailLogo,
  
  // Browsers
  [LogoType.BROWSER]: BrowserLogo,
  [LogoType.CHROME]: ChromeLogo,
  [LogoType.SAFARI]: SafariLogo,
  [LogoType.FIREFOX]: FirefoxLogo,
  [LogoType.EDGE]: EdgeLogo,
  [LogoType.BRAVE]: BraveLogo,
  [LogoType.ARC]: ArcLogo,
  
  // Google Workspace
  [LogoType.GDOCS]: GDocsLogo,
  [LogoType.GSHEETS]: GSheetsLogo,
  [LogoType.GSLIDES]: GSlidesLogo,
  
  // Microsoft Office
  [LogoType.WORD]: WordLogo,
  [LogoType.EXCEL]: ExcelLogo,
  [LogoType.POWERPOINT]: PowerPointLogo,
  [LogoType.OUTLOOK]: OutlookLogo,
  [LogoType.ONENOTE]: OneNoteLogo,
  
  // Apple Apps
  [LogoType.NOTES]: NotesLogo,
  [LogoType.REMINDERS]: RemindersLogo,
  [LogoType.FINDER]: FinderLogo,
  [LogoType.PREVIEW]: PreviewLogo,
  [LogoType.PAGES]: PagesLogo,
  [LogoType.NUMBERS]: NumbersLogo,
  [LogoType.KEYNOTE]: KeynoteLogo,
  [LogoType.TEXTEDIT]: TextEditLogo,
  [LogoType.CALCULATOR]: CalculatorLogo,
  [LogoType.TERMINAL]: TerminalLogo,
  
  // Development
  [LogoType.VSCODE]: VSCodeLogo,
  [LogoType.KIRO]: KiroLogo,
  [LogoType.XCODE]: XcodeLogo,
  [LogoType.JETBRAINS]: JetBrainsLogo,
  
  // Other
  [LogoType.JIRA]: JiraLogo,
  [LogoType.NOTION]: NotionLogo,
  [LogoType.FIGMA]: FigmaLogo,
  [LogoType.CLAUDE]: ClaudeLogo,
  [LogoType.DOCUMENT]: DocumentLogo,
};

/**
 * SourceLogo component
 * Renders the appropriate logo based on source type with configurable size
 * 
 * @param sourceType - The source type (slack, gmail, jira, browser, or other)
 * @param size - Size variant: 'sm' (24px), 'md' (32px), or 'lg' (48px)
 */
export const SourceLogo: React.FC<SourceLogoProps> = ({
  sourceType,
  size = 'md',
}) => {
  const config = getSourceConfig(sourceType);
  const LogoComponent = LOGO_COMPONENTS[config.logo] || DocumentLogo;
  const pixelSize = SIZE_MAP[size];

  return (
    <div
      className="source-logo"
      data-source-type={sourceType}
      data-size={size}
      data-testid="source-logo"
    >
      <LogoComponent size={pixelSize} />
    </div>
  );
};

export default SourceLogo;
