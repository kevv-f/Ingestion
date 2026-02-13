/**
 * Text truncation utility for preview text display
 * 
 * Requirement 3.3: THE Viewer_App SHALL truncate preview text to display
 * only the first 3-4 lines with ellipsis
 */

/**
 * Default character limit for truncation
 * Approximately 3-4 lines of text at typical card width
 */
export const DEFAULT_TRUNCATE_LIMIT = 150;

/**
 * Ellipsis string appended to truncated text
 */
export const ELLIPSIS = '...';

/**
 * Truncate text to a specified character limit with ellipsis
 * 
 * @param text - The text to truncate
 * @param limit - Maximum character length (default: 150)
 * @returns Truncated text with ellipsis if longer than limit, original text otherwise
 * 
 * Behavior:
 * - Empty or null/undefined text returns empty string
 * - Text shorter than or equal to limit is returned unchanged
 * - Text longer than limit is truncated and "..." is appended
 * - Truncation attempts to break at word boundaries when possible
 * 
 * Requirement 3.3: Truncate preview text to display only the first 3-4 lines with ellipsis
 */
export function truncateText(
  text: string | null | undefined,
  limit: number = DEFAULT_TRUNCATE_LIMIT
): string {
  // Handle null, undefined, or empty text
  if (!text) {
    return '';
  }

  // Handle negative or zero limit
  if (limit <= 0) {
    return ELLIPSIS;
  }

  // If text is within limit, return as-is
  if (text.length <= limit) {
    return text;
  }

  // Find a good break point (word boundary) near the limit
  // Look for the last space within the limit
  const truncateAt = limit - ELLIPSIS.length;
  
  if (truncateAt <= 0) {
    // If limit is too small for ellipsis, just return ellipsis
    return ELLIPSIS;
  }

  let breakPoint = truncateAt;
  
  // Try to find a word boundary (space) to break at
  const lastSpaceIndex = text.lastIndexOf(' ', truncateAt);
  
  // Only use word boundary if it's reasonably close to the limit
  // (within 20% of the truncate point to avoid too-short truncations)
  if (lastSpaceIndex > truncateAt * 0.8) {
    breakPoint = lastSpaceIndex;
  }

  // Truncate and add ellipsis
  return text.slice(0, breakPoint).trimEnd() + ELLIPSIS;
}

/**
 * Truncate text by line count
 * 
 * @param text - The text to truncate
 * @param maxLines - Maximum number of lines (default: 4)
 * @returns Truncated text with ellipsis if more lines than limit
 */
export function truncateByLines(
  text: string | null | undefined,
  maxLines: number = 4
): string {
  if (!text) {
    return '';
  }

  if (maxLines <= 0) {
    return ELLIPSIS;
  }

  const lines = text.split('\n');
  
  if (lines.length <= maxLines) {
    return text;
  }

  return lines.slice(0, maxLines).join('\n').trimEnd() + ELLIPSIS;
}
