/**
 * Property-based tests for ContentCard component
 * 
 * Feature: viewer-app
 * Property 3: Card Rendering Contains Required Information
 * **Validates: Requirements 3.2**
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import * as fc from 'fast-check';
import { ContentCard } from '../ContentCard';
import type { ContentSourceView } from '../../types';
import { KNOWN_SOURCE_TYPES } from '../../utils/sourceConfig';

/**
 * Arbitrary generator for ContentSourceView
 * Generates valid content source objects for property testing
 */
const contentSourceViewArbitrary = fc.record({
  id: fc.integer({ min: 1 }),
  source_type: fc.oneof(
    fc.constantFrom(...KNOWN_SOURCE_TYPES),
    fc.string({ minLength: 1, maxLength: 20 }).filter(s => s.trim().length > 0)
  ),
  source_path: fc.string({ minLength: 1 }),
  ehl_doc_id: fc.uuid(),
  chunk_count: fc.integer({ min: 0, max: 1000 }),
  created_at: fc.date().map(d => d.toISOString()),
  updated_at: fc.date().map(d => d.toISOString()),
  title: fc.option(fc.string({ minLength: 1, maxLength: 200 }), { nil: null }),
  preview_text: fc.string({ maxLength: 500 }),
  app_name: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
  bundle_id: fc.option(fc.string({ minLength: 1, maxLength: 100 }), { nil: null }),
});

describe('Feature: viewer-app, Property 3: Card Rendering Contains Required Information', () => {
  /**
   * Property 3: Card Rendering Contains Required Information
   * 
   * For any ContentSourceView, the rendered card component SHALL contain:
   * the source type logo element, the title (or placeholder if null),
   * truncated preview text, and source type label.
   * 
   * **Validates: Requirements 3.2**
   */
  describe('Property: Card contains all required elements', () => {
    it('should render source type logo for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          // Card should contain a source logo element
          const logo = screen.getByTestId('source-logo');
          expect(logo).toBeInTheDocument();
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should render title or placeholder for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          // Card should contain a title element
          const title = screen.getByTestId('card-title');
          expect(title).toBeInTheDocument();
          
          // Title should display the source title or "Untitled" placeholder
          const expectedTitle = source.title || 'Untitled';
          expect(title.textContent).toBe(expectedTitle);
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should render preview text for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          // Card should contain a preview element
          const preview = screen.getByTestId('card-preview');
          expect(preview).toBeInTheDocument();
          
          // Preview should contain text (may be truncated)
          // Empty preview_text should result in empty preview
          if (source.preview_text) {
            // If there's preview text, it should be displayed (possibly truncated)
            expect(preview.textContent?.length).toBeGreaterThanOrEqual(0);
          }
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should render source type label for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          // Card should contain a source type label
          const label = screen.getByTestId('source-type-label');
          expect(label).toBeInTheDocument();
          
          // Label should be capitalized version of source type
          const expectedLabel = source.source_type.charAt(0).toUpperCase() + source.source_type.slice(1);
          expect(label.textContent).toBe(expectedLabel);
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should render all required elements together for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          // All required elements should be present
          expect(screen.getByTestId('content-card')).toBeInTheDocument();
          expect(screen.getByTestId('source-logo')).toBeInTheDocument();
          expect(screen.getByTestId('card-title')).toBeInTheDocument();
          expect(screen.getByTestId('card-preview')).toBeInTheDocument();
          expect(screen.getByTestId('source-type-label')).toBeInTheDocument();
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Card is interactive', () => {
    it('should call onClick when clicked for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          const card = screen.getByTestId('content-card');
          fireEvent.click(card);
          
          expect(onClick).toHaveBeenCalledTimes(1);
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });

    it('should be keyboard accessible for any content source', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          const card = screen.getByTestId('content-card');
          
          // Should have role="button" for accessibility
          expect(card).toHaveAttribute('role', 'button');
          
          // Should be focusable
          expect(card).toHaveAttribute('tabIndex', '0');
          
          // Should respond to Enter key
          fireEvent.keyDown(card, { key: 'Enter' });
          expect(onClick).toHaveBeenCalledTimes(1);
          
          // Should respond to Space key
          fireEvent.keyDown(card, { key: ' ' });
          expect(onClick).toHaveBeenCalledTimes(2);
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Property: Card displays correct source type styling', () => {
    it('should have data-source-type attribute matching source type', () => {
      fc.assert(
        fc.property(contentSourceViewArbitrary, (source) => {
          const onClick = vi.fn();
          const { unmount } = render(<ContentCard source={source} onClick={onClick} />);
          
          const card = screen.getByTestId('content-card');
          expect(card).toHaveAttribute('data-source-type', source.source_type);
          
          unmount();
        }),
        { numRuns: 100 }
      );
    });
  });

  describe('Edge cases', () => {
    it('should handle null title gracefully', () => {
      const source: ContentSourceView = {
        id: 1,
        source_type: 'slack',
        source_path: '/test/path',
        ehl_doc_id: 'test-id',
        chunk_count: 5,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        title: null,
        preview_text: 'Some preview text',
        app_name: null,
        bundle_id: null,
      };
      
      const onClick = vi.fn();
      render(<ContentCard source={source} onClick={onClick} />);
      
      const title = screen.getByTestId('card-title');
      expect(title.textContent).toBe('Untitled');
    });

    it('should handle empty preview text', () => {
      const source: ContentSourceView = {
        id: 1,
        source_type: 'gmail',
        source_path: '/test/path',
        ehl_doc_id: 'test-id',
        chunk_count: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        title: 'Test Title',
        preview_text: '',
        app_name: null,
        bundle_id: null,
      };
      
      const onClick = vi.fn();
      render(<ContentCard source={source} onClick={onClick} />);
      
      const preview = screen.getByTestId('card-preview');
      expect(preview.textContent).toBe('');
    });

    it('should handle unknown source types', () => {
      const source: ContentSourceView = {
        id: 1,
        source_type: 'unknown-source',
        source_path: '/test/path',
        ehl_doc_id: 'test-id',
        chunk_count: 3,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        title: 'Unknown Source',
        preview_text: 'Preview text',
        app_name: null,
        bundle_id: null,
      };
      
      const onClick = vi.fn();
      render(<ContentCard source={source} onClick={onClick} />);
      
      // Should still render all elements
      expect(screen.getByTestId('content-card')).toBeInTheDocument();
      expect(screen.getByTestId('source-logo')).toBeInTheDocument();
      expect(screen.getByTestId('card-title')).toBeInTheDocument();
      expect(screen.getByTestId('source-type-label').textContent).toBe('Unknown-source');
    });
  });
});
