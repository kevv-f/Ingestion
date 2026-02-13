/**
 * Property-based tests for source configuration lookup
 * 
 * Feature: viewer-app
 * Property 5: Source Type to Visual Configuration Mapping
 * **Validates: Requirements 3.5, 4.1, 4.2, 4.3, 4.4, 4.5**
 */

import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import {
  getSourceConfig,
  SOURCE_CONFIG,
  KNOWN_SOURCE_TYPES,
  isKnownSourceType,
  LogoType,
} from '../sourceConfig';

describe('Feature: viewer-app, Property 5: Source Type to Visual Configuration Mapping', () => {
  /**
   * Property 5: Source Type to Visual Configuration Mapping
   * 
   * For any source type string, the configuration lookup SHALL return a valid
   * SourceConfig with logo component and gradient colors. Known source types
   * (slack, gmail, jira, browser) SHALL return their specific configurations,
   * and unknown source types SHALL return the default configuration.
   * 
   * **Validates: Requirements 3.5, 4.1, 4.2, 4.3, 4.4, 4.5**
   */
  describe('Property: Any source type returns valid configuration', () => {
    it('should return a valid SourceConfig for any string input', () => {
      fc.assert(
        fc.property(fc.string(), (sourceType) => {
          const config = getSourceConfig(sourceType);
          
          // Config must exist and have all required properties
          expect(config).toBeDefined();
          expect(config.logo).toBeDefined();
          expect(typeof config.logo).toBe('string');
          expect(config.primaryColor).toBeDefined();
          expect(typeof config.primaryColor).toBe('string');
          expect(config.gradientColors).toBeDefined();
          expect(Array.isArray(config.gradientColors)).toBe(true);
          expect(config.gradientColors).toHaveLength(2);
          expect(typeof config.gradientColors[0]).toBe('string');
          expect(typeof config.gradientColors[1]).toBe('string');
        }),
        { numRuns: 100 }
      );
    });

    it('should return valid hex color or rgba for primaryColor', () => {
      fc.assert(
        fc.property(fc.string(), (sourceType) => {
          const config = getSourceConfig(sourceType);
          
          // Primary color should be a valid hex color
          const hexColorRegex = /^#[0-9A-Fa-f]{6}$/;
          expect(config.primaryColor).toMatch(hexColorRegex);
        }),
        { numRuns: 100 }
      );
    });

    it('should return valid rgba colors for gradientColors', () => {
      fc.assert(
        fc.property(fc.string(), (sourceType) => {
          const config = getSourceConfig(sourceType);
          
          // Gradient colors should be valid rgba strings
          const rgbaRegex = /^rgba\(\d{1,3},\s*\d{1,3},\s*\d{1,3},\s*[\d.]+\)$/;
          expect(config.gradientColors[0]).toMatch(rgbaRegex);
          expect(config.gradientColors[1]).toMatch(rgbaRegex);
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Known source types return specific configurations', () => {
    /**
     * Requirement 4.1: WHEN displaying a card with source_type "slack",
     * THE Viewer_App SHALL display the Slack logo
     */
    it('should return Slack configuration for "slack" source type', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('slack', 'Slack', 'SLACK', ' slack ', 'slack '),
          (sourceType) => {
            const config = getSourceConfig(sourceType);
            expect(config.logo).toBe(LogoType.SLACK);
            expect(config.primaryColor).toBe('#4A154B');
          }
        ),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 4.2: WHEN displaying a card with source_type "gmail",
     * THE Viewer_App SHALL display the Gmail logo
     */
    it('should return Gmail configuration for "gmail" source type', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('gmail', 'Gmail', 'GMAIL', ' gmail ', 'gmail '),
          (sourceType) => {
            const config = getSourceConfig(sourceType);
            expect(config.logo).toBe(LogoType.GMAIL);
            expect(config.primaryColor).toBe('#EA4335');
          }
        ),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 4.3: WHEN displaying a card with source_type "jira",
     * THE Viewer_App SHALL display the Jira logo
     */
    it('should return Jira configuration for "jira" source type', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('jira', 'Jira', 'JIRA', ' jira ', 'jira '),
          (sourceType) => {
            const config = getSourceConfig(sourceType);
            expect(config.logo).toBe(LogoType.JIRA);
            expect(config.primaryColor).toBe('#0052CC');
          }
        ),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 4.4: WHEN displaying a card with source_type "browser",
     * THE Viewer_App SHALL display a browser/globe icon
     */
    it('should return Browser configuration for "browser" source type', () => {
      fc.assert(
        fc.property(
          fc.constantFrom('browser', 'Browser', 'BROWSER', ' browser ', 'browser '),
          (sourceType) => {
            const config = getSourceConfig(sourceType);
            expect(config.logo).toBe(LogoType.BROWSER);
            expect(config.primaryColor).toBe('#5F6368');
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Unknown source types return default configuration', () => {
    /**
     * Requirement 4.5: WHEN displaying a card with an unrecognized source_type,
     * THE Viewer_App SHALL display a generic document icon
     */
    it('should return default configuration for unknown source types', () => {
      // Generate strings that are NOT known source types
      const unknownSourceType = fc.string().filter((s) => {
        const normalized = s.toLowerCase().trim();
        return !KNOWN_SOURCE_TYPES.includes(normalized as any);
      });

      fc.assert(
        fc.property(unknownSourceType, (sourceType) => {
          const config = getSourceConfig(sourceType);
          expect(config.logo).toBe(LogoType.DOCUMENT);
          expect(config.primaryColor).toBe('#6B7280');
          expect(config.gradientColors).toEqual([
            'rgba(107, 114, 128, 0.15)',
            'rgba(156, 163, 175, 0.1)',
          ]);
        }),
        { numRuns: 100 }
      );
    });

    it('should return default for empty string', () => {
      const config = getSourceConfig('');
      expect(config.logo).toBe(LogoType.DOCUMENT);
      expect(config).toEqual(SOURCE_CONFIG.default);
    });

    it('should return default for whitespace-only string', () => {
      fc.assert(
        fc.property(
          fc.stringOf(fc.constantFrom(' ', '\t', '\n')),
          (whitespace) => {
            const config = getSourceConfig(whitespace);
            expect(config.logo).toBe(LogoType.DOCUMENT);
            expect(config).toEqual(SOURCE_CONFIG.default);
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Configuration is case-insensitive', () => {
    /**
     * Requirement 3.5: Source type matching should be case-insensitive
     */
    it('should return same config regardless of case for known types', () => {
      fc.assert(
        fc.property(
          fc.constantFrom(...KNOWN_SOURCE_TYPES),
          fc.constantFrom(
            (s: string) => s.toLowerCase(),
            (s: string) => s.toUpperCase(),
            (s: string) => s.charAt(0).toUpperCase() + s.slice(1).toLowerCase()
          ),
          (sourceType, transform) => {
            const originalConfig = getSourceConfig(sourceType);
            const transformedConfig = getSourceConfig(transform(sourceType));
            expect(transformedConfig).toEqual(originalConfig);
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('isKnownSourceType helper', () => {
    it('should return true for all known source types', () => {
      fc.assert(
        fc.property(fc.constantFrom(...KNOWN_SOURCE_TYPES), (sourceType) => {
          expect(isKnownSourceType(sourceType)).toBe(true);
        }),
        { numRuns: 100 }
      );
    });

    it('should return false for unknown source types', () => {
      const unknownSourceType = fc.string().filter((s) => {
        const normalized = s.toLowerCase().trim();
        return !KNOWN_SOURCE_TYPES.includes(normalized as any);
      });

      fc.assert(
        fc.property(unknownSourceType, (sourceType) => {
          expect(isKnownSourceType(sourceType)).toBe(false);
        }),
        { numRuns: 100 }
      );
    });
  });
});
