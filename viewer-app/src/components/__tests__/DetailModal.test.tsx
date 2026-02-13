/**
 * Property-based tests for DetailModal component
 * 
 * Feature: viewer-app
 * Property 6: Detail View Displays All Available Metadata
 * **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8**
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import * as fc from 'fast-check';
import type { ContentDetail } from '../../types';
import { KNOWN_SOURCE_TYPES } from '../../utils/sourceConfig';

// Mock the useContentDetail hook
vi.mock('../../hooks', () => ({
  useContentDetail: vi.fn(),
}));

// Import after mocking
import { DetailModal } from '../DetailModal';
import { useContentDetail } from '../../hooks';

const mockedUseContentDetail = vi.mocked(useContentDetail);

/**
 * Arbitrary generator for ContentDetail
 * Generates valid content detail objects for property testing
 */
const contentDetailArbitrary = fc.record({
  id: fc.integer({ min: 1 }),
  source_type: fc.oneof(
    fc.constantFrom(...KNOWN_SOURCE_TYPES),
    fc.string({ minLength: 1, maxLength: 20 }).filter(s => s.trim().length > 0)
  ),
  source_path: fc.webUrl(),
  ehl_doc_id: fc.uuid(),
  chunk_count: fc.integer({ min: 0, max: 1000 }),
  created_at: fc.date().map(d => d.toISOString()),
  updated_at: fc.date().map(d => d.toISOString()),
  title: fc.option(fc.string({ minLength: 1, maxLength: 200 }), { nil: null }),
  author: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
  channel: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
  full_text: fc.string({ maxLength: 5000 }),
  app_name: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
  bundle_id: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
});

describe('Feature: viewer-app, Property 6: Detail View Displays All Available Metadata', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  /**
   * Property 6: Detail View Displays All Available Metadata
   * 
   * For any ContentDetail object, the rendered detail view SHALL display:
   * the full title (if present), source type with logo, source URL,
   * author (if present), channel (if present), full reconstructed text,
   * created_at timestamp, updated_at timestamp, and chunk count.
   * 
   * **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8**
   */
  describe('Property: Detail view displays all required metadata', () => {
    /**
     * Requirement 6.1: Display the full title of the content source
     */
    it('should display title or placeholder for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const titleElement = screen.getByTestId('detail-title');
          expect(titleElement).toBeInTheDocument();
          
          const expectedTitle = detail.title || 'Untitled';
          expect(titleElement.textContent).toBe(expectedTitle);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.2: Display the source type with its logo
     */
    it('should display source type with logo for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          // Should have source logo
          const logo = screen.getByTestId('source-logo');
          expect(logo).toBeInTheDocument();

          // Should have source type label
          const sourceType = screen.getByTestId('detail-source-type');
          expect(sourceType).toBeInTheDocument();
          
          const expectedLabel = detail.source_type.charAt(0).toUpperCase() + detail.source_type.slice(1);
          expect(sourceType.textContent).toBe(expectedLabel);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.3: Display the source URL as a clickable link
     */
    it('should display source URL as clickable link for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const sourceUrl = screen.getByTestId('detail-source-url');
          expect(sourceUrl).toBeInTheDocument();
          expect(sourceUrl.tagName.toLowerCase()).toBe('a');
          expect(sourceUrl).toHaveAttribute('href', detail.source_path);
          expect(sourceUrl).toHaveAttribute('target', '_blank');
          expect(sourceUrl).toHaveAttribute('rel', 'noopener noreferrer');

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.4: Display the author if available in chunk metadata
     */
    it('should display author when available for any content detail', () => {
      // Generate details with author present
      const detailWithAuthor = contentDetailArbitrary.filter(d => d.author !== null);

      fc.assert(
        fc.property(detailWithAuthor, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const authorElement = screen.getByTestId('detail-author');
          expect(authorElement).toBeInTheDocument();
          expect(authorElement.textContent).toBe(detail.author);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.5: Display the channel if available in chunk metadata
     */
    it('should display channel when available for any content detail', () => {
      // Generate details with channel present
      const detailWithChannel = contentDetailArbitrary.filter(d => d.channel !== null);

      fc.assert(
        fc.property(detailWithChannel, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const channelElement = screen.getByTestId('detail-channel');
          expect(channelElement).toBeInTheDocument();
          expect(channelElement.textContent).toBe(detail.channel);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.6: Reconstruct and display the full text content
     */
    it('should display full text content for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const fullText = screen.getByTestId('detail-full-text');
          expect(fullText).toBeInTheDocument();
          
          // Full text should be displayed (or "No content available" if empty)
          if (detail.full_text) {
            expect(fullText.textContent).toBe(detail.full_text);
          }

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.7: Display the created_at and updated_at timestamps
     */
    it('should display timestamps for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          // Created at should be displayed
          const createdAt = screen.getByTestId('detail-created-at');
          expect(createdAt).toBeInTheDocument();
          expect(createdAt.textContent).toBeTruthy();

          // Updated at should be displayed
          const updatedAt = screen.getByTestId('detail-updated-at');
          expect(updatedAt).toBeInTheDocument();
          expect(updatedAt.textContent).toBeTruthy();

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 6.8: Display the chunk count
     */
    it('should display chunk count for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const chunkCount = screen.getByTestId('detail-chunk-count');
          expect(chunkCount).toBeInTheDocument();
          expect(chunkCount.textContent).toBe(String(detail.chunk_count));

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Combined test: All metadata fields present
     */
    it('should display all metadata fields together for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          // All required elements should be present
          expect(screen.getByTestId('detail-modal')).toBeInTheDocument();
          expect(screen.getByTestId('detail-title')).toBeInTheDocument();
          expect(screen.getByTestId('source-logo')).toBeInTheDocument();
          expect(screen.getByTestId('detail-source-type')).toBeInTheDocument();
          expect(screen.getByTestId('detail-source-url')).toBeInTheDocument();
          expect(screen.getByTestId('detail-chunk-count')).toBeInTheDocument();
          expect(screen.getByTestId('detail-created-at')).toBeInTheDocument();
          expect(screen.getByTestId('detail-updated-at')).toBeInTheDocument();
          expect(screen.getByTestId('detail-full-text')).toBeInTheDocument();

          // Optional fields should be present only when data exists
          if (detail.author) {
            expect(screen.getByTestId('detail-author')).toBeInTheDocument();
          }
          if (detail.channel) {
            expect(screen.getByTestId('detail-channel')).toBeInTheDocument();
          }

          unmount();
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Modal navigation and accessibility', () => {
    /**
     * Requirement 8.1: Display a close or back button
     */
    it('should display close button for any content detail', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const closeButton = screen.getByTestId('close-button');
          expect(closeButton).toBeInTheDocument();
          expect(closeButton).toHaveAttribute('aria-label', 'Close detail view');

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 8.2: Close on close button click
     */
    it('should call onClose when close button is clicked', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const closeButton = screen.getByTestId('close-button');
          fireEvent.click(closeButton);

          expect(onClose).toHaveBeenCalledTimes(1);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    /**
     * Requirement 8.3: Close on Escape key press
     */
    it('should call onClose when Escape key is pressed', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          fireEvent.keyDown(document, { key: 'Escape' });

          expect(onClose).toHaveBeenCalledTimes(1);

          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should have proper ARIA attributes for accessibility', () => {
      fc.assert(
        fc.property(contentDetailArbitrary, (detail) => {
          mockedUseContentDetail.mockReturnValue({
            detail,
            loading: false,
            error: null,
          });

          const onClose = vi.fn();
          const { unmount } = render(
            <DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />
          );

          const backdrop = screen.getByTestId('detail-modal-backdrop');
          expect(backdrop).toHaveAttribute('role', 'dialog');
          expect(backdrop).toHaveAttribute('aria-modal', 'true');

          unmount();
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Edge cases', () => {
    it('should handle null author gracefully', () => {
      const detail: ContentDetail = {
        id: 1,
        source_type: 'slack',
        source_path: 'https://example.com',
        ehl_doc_id: 'test-id',
        chunk_count: 5,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        title: 'Test Title',
        author: null,
        channel: 'general',
        full_text: 'Some content',
        app_name: null,
        bundle_id: null,
      };

      mockedUseContentDetail.mockReturnValue({
        detail,
        loading: false,
        error: null,
      });

      const onClose = vi.fn();
      render(<DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />);

      // Author should not be rendered
      expect(screen.queryByTestId('detail-author')).not.toBeInTheDocument();
      
      // Channel should still be rendered
      expect(screen.getByTestId('detail-channel')).toBeInTheDocument();
    });

    it('should handle null channel gracefully', () => {
      const detail: ContentDetail = {
        id: 1,
        source_type: 'gmail',
        source_path: 'https://example.com',
        ehl_doc_id: 'test-id',
        chunk_count: 3,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        title: 'Test Title',
        author: 'John Doe',
        channel: null,
        full_text: 'Some content',
        app_name: null,
        bundle_id: null,
      };

      mockedUseContentDetail.mockReturnValue({
        detail,
        loading: false,
        error: null,
      });

      const onClose = vi.fn();
      render(<DetailModal ehlDocId={detail.ehl_doc_id} onClose={onClose} />);

      // Channel should not be rendered
      expect(screen.queryByTestId('detail-channel')).not.toBeInTheDocument();
      
      // Author should still be rendered
      expect(screen.getByTestId('detail-author')).toBeInTheDocument();
    });

    it('should show loading state', () => {
      mockedUseContentDetail.mockReturnValue({
        detail: null,
        loading: true,
        error: null,
      });

      const onClose = vi.fn();
      render(<DetailModal ehlDocId="test-id" onClose={onClose} />);

      expect(screen.getByTestId('detail-loading')).toBeInTheDocument();
    });

    it('should show error state', () => {
      mockedUseContentDetail.mockReturnValue({
        detail: null,
        loading: false,
        error: 'Failed to fetch',
      });

      const onClose = vi.fn();
      render(<DetailModal ehlDocId="test-id" onClose={onClose} />);

      expect(screen.getByTestId('detail-error')).toBeInTheDocument();
    });
  });
});
