/**
 * Source type configuration for visual styling
 * Maps source types to their logos, colors, and gradients
 * 
 * Requirements: 3.5, 4.1-4.5
 */

import { SourceConfig } from '../types';

/**
 * Logo identifiers for each source type
 * These will be used to render the appropriate logo component
 */
export const LogoType = {
  // Communication
  SLACK: 'slack',
  GMAIL: 'gmail',
  TEAMS: 'teams',
  DISCORD: 'discord',
  ZOOM: 'zoom',
  MAIL: 'mail',
  
  // Browsers
  BROWSER: 'browser',
  CHROME: 'chrome',
  SAFARI: 'safari',
  FIREFOX: 'firefox',
  EDGE: 'edge',
  BRAVE: 'brave',
  ARC: 'arc',
  
  // Google Workspace
  GDOCS: 'gdocs',
  GSHEETS: 'gsheets',
  GSLIDES: 'gslides',
  
  // Microsoft Office
  WORD: 'word',
  EXCEL: 'excel',
  POWERPOINT: 'powerpoint',
  OUTLOOK: 'outlook',
  ONENOTE: 'onenote',
  
  // Apple Apps
  NOTES: 'notes',
  REMINDERS: 'reminders',
  FINDER: 'finder',
  PREVIEW: 'preview',
  PAGES: 'pages',
  NUMBERS: 'numbers',
  KEYNOTE: 'keynote',
  TEXTEDIT: 'textedit',
  CALCULATOR: 'calculator',
  TERMINAL: 'terminal',
  
  // Development
  VSCODE: 'vscode',
  KIRO: 'kiro',
  XCODE: 'xcode',
  JETBRAINS: 'jetbrains',
  
  // Other
  JIRA: 'jira',
  NOTION: 'notion',
  FIGMA: 'figma',
  CLAUDE: 'claude',
  DOCUMENT: 'document',
} as const;

/**
 * Known source types that have specific configurations
 */
export const KNOWN_SOURCE_TYPES = [
  // Communication
  'slack', 'gmail', 'teams', 'discord', 'zoom', 'mail',
  // Browsers
  'browser', 'chrome', 'safari', 'firefox', 'edge', 'brave', 'arc',
  // Google Workspace
  'gdocs', 'gsheets', 'gslides',
  // Microsoft Office
  'word', 'excel', 'powerpoint', 'outlook', 'onenote',
  // Apple Apps
  'notes', 'reminders', 'finder', 'preview', 'pages', 'numbers', 'keynote', 'textedit', 'calculator', 'terminal',
  // Development
  'vscode', 'kiro', 'xcode', 'jetbrains',
  // Other
  'jira', 'notion', 'figma', 'claude', 'spotify',
] as const;
export type KnownSourceType = typeof KNOWN_SOURCE_TYPES[number];

/**
 * Source configuration mapping
 * Each source type has a logo, primary color, and gradient colors for glass morphism
 * 
 * Requirements:
 * - 3.5: Tint glass gradient to complement source logo colors
 * - 4.1: Slack logo for source_type "slack"
 * - 4.2: Gmail logo for source_type "gmail"
 * - 4.3: Jira logo for source_type "jira"
 * - 4.4: Browser/globe icon for source_type "browser"
 * - 4.5: Generic document icon for unrecognized source_type
 */
export const SOURCE_CONFIG: Record<string, SourceConfig> = {
  // Communication
  slack: {
    logo: LogoType.SLACK,
    primaryColor: '#4A154B',
    gradientColors: ['rgba(74, 21, 75, 0.15)', 'rgba(54, 197, 240, 0.1)'],
  },
  gmail: {
    logo: LogoType.GMAIL,
    primaryColor: '#EA4335',
    gradientColors: ['rgba(234, 67, 53, 0.15)', 'rgba(251, 188, 4, 0.1)'],
  },
  teams: {
    logo: LogoType.TEAMS,
    primaryColor: '#6264A7',
    gradientColors: ['rgba(98, 100, 167, 0.15)', 'rgba(138, 140, 191, 0.1)'],
  },
  discord: {
    logo: LogoType.DISCORD,
    primaryColor: '#5865F2',
    gradientColors: ['rgba(88, 101, 242, 0.15)', 'rgba(114, 137, 218, 0.1)'],
  },
  zoom: {
    logo: LogoType.ZOOM,
    primaryColor: '#2D8CFF',
    gradientColors: ['rgba(45, 140, 255, 0.15)', 'rgba(77, 161, 255, 0.1)'],
  },
  mail: {
    logo: LogoType.MAIL,
    primaryColor: '#007AFF',
    gradientColors: ['rgba(0, 122, 255, 0.15)', 'rgba(64, 156, 255, 0.1)'],
  },

  // Browsers
  browser: {
    logo: LogoType.CHROME,
    primaryColor: '#4285F4',
    gradientColors: ['rgba(66, 133, 244, 0.15)', 'rgba(234, 67, 53, 0.1)'],
  },
  chrome: {
    logo: LogoType.CHROME,
    primaryColor: '#4285F4',
    gradientColors: ['rgba(66, 133, 244, 0.15)', 'rgba(234, 67, 53, 0.1)'],
  },
  safari: {
    logo: LogoType.SAFARI,
    primaryColor: '#006CFF',
    gradientColors: ['rgba(0, 108, 255, 0.15)', 'rgba(0, 200, 255, 0.1)'],
  },
  firefox: {
    logo: LogoType.FIREFOX,
    primaryColor: '#FF7139',
    gradientColors: ['rgba(255, 113, 57, 0.15)', 'rgba(255, 189, 79, 0.1)'],
  },
  edge: {
    logo: LogoType.EDGE,
    primaryColor: '#0078D7',
    gradientColors: ['rgba(0, 120, 215, 0.15)', 'rgba(0, 180, 255, 0.1)'],
  },
  brave: {
    logo: LogoType.BRAVE,
    primaryColor: '#FB542B',
    gradientColors: ['rgba(251, 84, 43, 0.15)', 'rgba(255, 128, 0, 0.1)'],
  },
  arc: {
    logo: LogoType.ARC,
    primaryColor: '#FF6B6B',
    gradientColors: ['rgba(255, 107, 107, 0.15)', 'rgba(255, 166, 158, 0.1)'],
  },

  // Google Workspace
  gdocs: {
    logo: LogoType.GDOCS,
    primaryColor: '#4285F4',
    gradientColors: ['rgba(66, 133, 244, 0.15)', 'rgba(52, 168, 83, 0.1)'],
  },
  gsheets: {
    logo: LogoType.GSHEETS,
    primaryColor: '#0F9D58',
    gradientColors: ['rgba(15, 157, 88, 0.15)', 'rgba(52, 168, 83, 0.1)'],
  },
  gslides: {
    logo: LogoType.GSLIDES,
    primaryColor: '#F4B400',
    gradientColors: ['rgba(244, 180, 0, 0.15)', 'rgba(251, 188, 4, 0.1)'],
  },

  // Microsoft Office
  word: {
    logo: LogoType.WORD,
    primaryColor: '#2B579A',
    gradientColors: ['rgba(43, 87, 154, 0.15)', 'rgba(65, 120, 190, 0.1)'],
  },
  excel: {
    logo: LogoType.EXCEL,
    primaryColor: '#217346',
    gradientColors: ['rgba(33, 115, 70, 0.15)', 'rgba(52, 168, 83, 0.1)'],
  },
  powerpoint: {
    logo: LogoType.POWERPOINT,
    primaryColor: '#D24726',
    gradientColors: ['rgba(210, 71, 38, 0.15)', 'rgba(255, 140, 0, 0.1)'],
  },
  outlook: {
    logo: LogoType.OUTLOOK,
    primaryColor: '#0078D4',
    gradientColors: ['rgba(0, 120, 212, 0.15)', 'rgba(40, 153, 245, 0.1)'],
  },
  onenote: {
    logo: LogoType.ONENOTE,
    primaryColor: '#7719AA',
    gradientColors: ['rgba(119, 25, 170, 0.15)', 'rgba(155, 89, 182, 0.1)'],
  },

  // Apple Apps
  notes: {
    logo: LogoType.NOTES,
    primaryColor: '#FFCC00',
    gradientColors: ['rgba(255, 204, 0, 0.15)', 'rgba(255, 230, 128, 0.1)'],
  },
  reminders: {
    logo: LogoType.REMINDERS,
    primaryColor: '#FF9500',
    gradientColors: ['rgba(255, 149, 0, 0.15)', 'rgba(255, 179, 64, 0.1)'],
  },
  finder: {
    logo: LogoType.FINDER,
    primaryColor: '#007AFF',
    gradientColors: ['rgba(0, 122, 255, 0.15)', 'rgba(64, 156, 255, 0.1)'],
  },
  preview: {
    logo: LogoType.PREVIEW,
    primaryColor: '#007AFF',
    gradientColors: ['rgba(0, 122, 255, 0.15)', 'rgba(64, 156, 255, 0.1)'],
  },
  pages: {
    logo: LogoType.PAGES,
    primaryColor: '#FF9500',
    gradientColors: ['rgba(255, 149, 0, 0.15)', 'rgba(255, 179, 64, 0.1)'],
  },
  numbers: {
    logo: LogoType.NUMBERS,
    primaryColor: '#34C759',
    gradientColors: ['rgba(52, 199, 89, 0.15)', 'rgba(102, 212, 133, 0.1)'],
  },
  keynote: {
    logo: LogoType.KEYNOTE,
    primaryColor: '#007AFF',
    gradientColors: ['rgba(0, 122, 255, 0.15)', 'rgba(64, 156, 255, 0.1)'],
  },
  textedit: {
    logo: LogoType.TEXTEDIT,
    primaryColor: '#8E8E93',
    gradientColors: ['rgba(142, 142, 147, 0.15)', 'rgba(174, 174, 178, 0.1)'],
  },
  calculator: {
    logo: LogoType.CALCULATOR,
    primaryColor: '#FF9500',
    gradientColors: ['rgba(255, 149, 0, 0.15)', 'rgba(255, 179, 64, 0.1)'],
  },
  terminal: {
    logo: LogoType.TERMINAL,
    primaryColor: '#000000',
    gradientColors: ['rgba(0, 0, 0, 0.15)', 'rgba(64, 64, 64, 0.1)'],
  },

  // Development
  vscode: {
    logo: LogoType.VSCODE,
    primaryColor: '#007ACC',
    gradientColors: ['rgba(0, 122, 204, 0.15)', 'rgba(64, 156, 230, 0.1)'],
  },
  kiro: {
    logo: LogoType.KIRO,
    primaryColor: '#FF6B35',
    gradientColors: ['rgba(255, 107, 53, 0.15)', 'rgba(255, 153, 102, 0.1)'],
  },
  xcode: {
    logo: LogoType.XCODE,
    primaryColor: '#147EFB',
    gradientColors: ['rgba(20, 126, 251, 0.15)', 'rgba(76, 161, 255, 0.1)'],
  },
  jetbrains: {
    logo: LogoType.JETBRAINS,
    primaryColor: '#000000',
    gradientColors: ['rgba(0, 0, 0, 0.15)', 'rgba(255, 0, 135, 0.1)'],
  },

  // Other
  jira: {
    logo: LogoType.JIRA,
    primaryColor: '#0052CC',
    gradientColors: ['rgba(0, 82, 204, 0.15)', 'rgba(0, 101, 255, 0.1)'],
  },
  notion: {
    logo: LogoType.NOTION,
    primaryColor: '#000000',
    gradientColors: ['rgba(0, 0, 0, 0.15)', 'rgba(64, 64, 64, 0.1)'],
  },
  figma: {
    logo: LogoType.FIGMA,
    primaryColor: '#F24E1E',
    gradientColors: ['rgba(242, 78, 30, 0.15)', 'rgba(162, 89, 255, 0.1)'],
  },
  claude: {
    logo: LogoType.CLAUDE,
    primaryColor: '#D97757',
    gradientColors: ['rgba(217, 119, 87, 0.15)', 'rgba(255, 166, 140, 0.1)'],
  },
  spotify: {
    logo: LogoType.DOCUMENT,
    primaryColor: '#1DB954',
    gradientColors: ['rgba(29, 185, 84, 0.15)', 'rgba(80, 200, 120, 0.1)'],
  },

  default: {
    logo: LogoType.DOCUMENT,
    primaryColor: '#6B7280',
    gradientColors: ['rgba(107, 114, 128, 0.15)', 'rgba(156, 163, 175, 0.1)'],
  },
};

/**
 * Get the source configuration for a given source type
 * Returns the specific configuration for known types, or default for unknown types
 * 
 * @param sourceType - The source type string (e.g., 'slack', 'gmail', 'jira', 'browser')
 * @returns SourceConfig with logo, primaryColor, and gradientColors
 * 
 * Requirements:
 * - Known source types (slack, gmail, jira, browser) return their specific configurations
 * - Unknown source types return the default configuration
 */
export function getSourceConfig(sourceType: string): SourceConfig {
  const normalizedType = sourceType.toLowerCase().trim();
  // Use Object.prototype.hasOwnProperty to safely check for own properties (avoids __proto__ issues)
  if (Object.prototype.hasOwnProperty.call(SOURCE_CONFIG, normalizedType)) {
    return SOURCE_CONFIG[normalizedType];
  }
  return SOURCE_CONFIG.default;
}

/**
 * Check if a source type is a known type with specific configuration
 * 
 * @param sourceType - The source type string to check
 * @returns true if the source type has a specific configuration
 */
export function isKnownSourceType(sourceType: string): sourceType is KnownSourceType {
  const normalizedType = sourceType.toLowerCase().trim();
  return KNOWN_SOURCE_TYPES.includes(normalizedType as KnownSourceType);
}
