/**
 * Property-based tests for text truncation utility
 * 
 * Feature: viewer-app
 * Property 4: Text Truncation with Ellipsis
 * **Validates: Requirements 3.3**
 */

import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import {
  truncateText,
  truncateByLines,
  DEFAULT_TRUNCATE_LIMIT,
  ELLIPSIS,
} from '../truncate';

describe('Feature: viewer-app, Property 4: Text Truncation with Ellipsis', () => {
  /**
   * Property 4: Text Truncation with Ellipsis
   * 
   * For any text string longer than the truncation limit (approximately 150 characters
   * or 3-4 lines), the truncation function SHALL return a string that is shorter than
   * the original, ends with an ellipsis ("..."), and preserves the beginning of the
   * original text.
   * 
   * **Validates: Requirements 3.3**
   */
  describe('Property: Text longer than limit is truncated with ellipsis', () => {
    it('should return shorter string ending with ellipsis for text longer than limit', () => {
      // Generate text that is definitely longer than the default limit
      const longText = fc.string({ minLength: DEFAULT_TRUNCATE_LIMIT + 1 });

      fc.assert(
        fc.property(longText, (text) => {
          const result = truncateText(text);
          
          // Result should be shorter than original
          expect(result.length).toBeLessThan(text.length);
          
          // Result should end with ellipsis
          expect(result.endsWith(ELLIPSIS)).toBe(true);
          
          // Result length should be at most limit
          expect(result.length).toBeLessThanOrEqual(DEFAULT_TRUNCATE_LIMIT);
        }),
        { numRuns: 100 }
      );
    });

    it('should preserve the beginning of the original text', () => {
      const longText = fc.string({ minLength: DEFAULT_TRUNCATE_LIMIT + 1 });

      fc.assert(
        fc.property(longText, (text) => {
          const result = truncateText(text);
          
          // Remove ellipsis to get the preserved portion
          const preservedPortion = result.slice(0, -ELLIPSIS.length);
          
          // The preserved portion should be a prefix of the original text
          // (accounting for possible trimming at word boundaries)
          expect(text.startsWith(preservedPortion.trimEnd())).toBe(true);
        }),
        { numRuns: 100 }
      );
    });

    it('should work with custom limit parameter', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 50 }),
          fc.integer({ min: 10, max: 40 }),
          (text, limit) => {
            const result = truncateText(text, limit);
            
            if (text.length > limit) {
              // Should be truncated
              expect(result.length).toBeLessThanOrEqual(limit);
              expect(result.endsWith(ELLIPSIS)).toBe(true);
            } else {
              // Should be unchanged
              expect(result).toBe(text);
            }
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Text shorter than or equal to limit is unchanged', () => {
    it('should return original text when length is within limit', () => {
      // Generate text that is at most the default limit
      const shortText = fc.string({ maxLength: DEFAULT_TRUNCATE_LIMIT });

      fc.assert(
        fc.property(shortText, (text) => {
          const result = truncateText(text);
          
          // Result should be exactly the original text
          expect(result).toBe(text);
        }),
        { numRuns: 100 }
      );
    });

    it('should return original text when length equals limit exactly', () => {
      // Generate text of exactly the limit length
      const exactLengthText = fc.string({
        minLength: DEFAULT_TRUNCATE_LIMIT,
        maxLength: DEFAULT_TRUNCATE_LIMIT,
      });

      fc.assert(
        fc.property(exactLengthText, (text) => {
          const result = truncateText(text);
          expect(result).toBe(text);
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Edge cases are handled correctly', () => {
    it('should return empty string for null input', () => {
      expect(truncateText(null)).toBe('');
    });

    it('should return empty string for undefined input', () => {
      expect(truncateText(undefined)).toBe('');
    });

    it('should return empty string for empty string input', () => {
      expect(truncateText('')).toBe('');
    });

    it('should handle whitespace-only strings', () => {
      fc.assert(
        fc.property(
          fc.stringOf(fc.constantFrom(' ', '\t', '\n'), { maxLength: DEFAULT_TRUNCATE_LIMIT }),
          (whitespace) => {
            const result = truncateText(whitespace);
            // Whitespace within limit should be returned as-is
            expect(result).toBe(whitespace);
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should handle very small limits gracefully', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 10 }),
          fc.integer({ min: 1, max: 5 }),
          (text, limit) => {
            const result = truncateText(text, limit);
            
            // Should not throw and should return something
            expect(typeof result).toBe('string');
            
            // If text is longer than limit, result should end with ellipsis
            if (text.length > limit) {
              expect(result.endsWith(ELLIPSIS)).toBe(true);
              
              // When limit is smaller than ellipsis length, we just return ellipsis
              // Otherwise, result should be at most the limit length
              if (limit >= ELLIPSIS.length) {
                expect(result.length).toBeLessThanOrEqual(limit);
              } else {
                // For very small limits, we return just ellipsis
                expect(result).toBe(ELLIPSIS);
              }
            }
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should handle zero limit', () => {
      fc.assert(
        fc.property(fc.string({ minLength: 1 }), (text) => {
          const result = truncateText(text, 0);
          expect(result).toBe(ELLIPSIS);
        }),
        { numRuns: 100 }
      );
    });

    it('should handle negative limit', () => {
      fc.assert(
        fc.property(
          fc.string({ minLength: 1 }),
          fc.integer({ min: -100, max: -1 }),
          (text, limit) => {
            const result = truncateText(text, limit);
            expect(result).toBe(ELLIPSIS);
          }
        ),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Word boundary handling', () => {
    it('should attempt to break at word boundaries when possible', () => {
      // Generate text with spaces that is longer than limit
      const textWithSpaces = fc
        .array(fc.string({ minLength: 5, maxLength: 20 }), { minLength: 10, maxLength: 20 })
        .map((words) => words.join(' '));

      fc.assert(
        fc.property(textWithSpaces, (text) => {
          if (text.length <= DEFAULT_TRUNCATE_LIMIT) {
            return; // Skip if text is too short
          }

          const result = truncateText(text);
          
          // Result should end with ellipsis
          expect(result.endsWith(ELLIPSIS)).toBe(true);
          
          // The character before ellipsis should not be a space
          // (we trim trailing spaces before adding ellipsis)
          const beforeEllipsis = result.slice(0, -ELLIPSIS.length);
          expect(beforeEllipsis.endsWith(' ')).toBe(false);
        }),
        { numRuns: 100 }
      );
    });
  });
});

describe('truncateByLines', () => {
  describe('Property: Lines beyond limit are removed with ellipsis', () => {
    it('should truncate text with more lines than limit', () => {
      // Generate text with multiple lines
      const multiLineText = fc
        .array(fc.string({ minLength: 1, maxLength: 50 }), { minLength: 5, maxLength: 20 })
        .map((lines) => lines.join('\n'));

      fc.assert(
        fc.property(multiLineText, (text) => {
          const maxLines = 4;
          const result = truncateByLines(text, maxLines);
          const originalLineCount = text.split('\n').length;
          const resultLineCount = result.split('\n').length;

          if (originalLineCount > maxLines) {
            // Should be truncated
            expect(resultLineCount).toBeLessThanOrEqual(maxLines);
            expect(result.endsWith(ELLIPSIS)).toBe(true);
          } else {
            // Should be unchanged
            expect(result).toBe(text);
          }
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Edge cases', () => {
    it('should return empty string for null input', () => {
      expect(truncateByLines(null)).toBe('');
    });

    it('should return empty string for undefined input', () => {
      expect(truncateByLines(undefined)).toBe('');
    });

    it('should return empty string for empty string input', () => {
      expect(truncateByLines('')).toBe('');
    });

    it('should handle zero maxLines', () => {
      expect(truncateByLines('some text', 0)).toBe(ELLIPSIS);
    });

    it('should handle negative maxLines', () => {
      expect(truncateByLines('some text', -1)).toBe(ELLIPSIS);
    });
  });
});
