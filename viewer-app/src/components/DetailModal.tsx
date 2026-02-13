/**
 * DetailModal component - Displays full content detail in a modal
 * 
 * Requirements:
 * - 6.1: Display the full title of the content source
 * - 6.2: Display the source type with its logo
 * - 6.3: Display the source URL as a clickable link
 * - 6.4: Display the author if available in chunk metadata
 * - 6.5: Display the channel if available in chunk metadata
 * - 6.6: Reconstruct and display the full text content
 * - 6.7: Display the created_at and updated_at timestamps
 * - 6.8: Display the chunk count
 * - 8.1: Display a close or back button
 * - 8.2: Close on close button click
 * - 8.3: Close on Escape key press
 */

import React, { useEffect, useCallback } from 'react';
import { SourceLogo } from './SourceLogo';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';
import { useContentDetail } from '../hooks';
import { getSourceConfig } from '../utils/sourceConfig';

export interface DetailModalProps {
  ehlDocId: string;
  onClose: () => void;
}

/**
 * Format a date string for display
 */
const formatDate = (dateString: string): string => {
  try {
    const date = new Date(dateString);
    return date.toLocaleString();
  } catch {
    return dateString;
  }
};

/**
 * Truncate a URL if it exceeds maxLength characters
 */
const truncateUrl = (url: string, maxLength: number = 100): string => {
  if (url.length <= maxLength) {
    return url;
  }
  return url.substring(0, maxLength - 3) + '...';
};

/**
 * DetailModal component
 * Displays full content detail with all metadata fields
 * 
 * @param ehlDocId - The EHL document ID to display
 * @param onClose - Callback when the modal should close
 */
export const DetailModal: React.FC<DetailModalProps> = ({ ehlDocId, onClose }) => {
  const { detail, loading, error } = useContentDetail(ehlDocId);

  // Handle Escape key to close modal
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    },
    [onClose]
  );

  // Add and remove keyboard event listener
  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleKeyDown]);

  // Prevent body scroll when modal is open
  useEffect(() => {
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = '';
    };
  }, []);

  // Handle click on backdrop to close
  const handleBackdropClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.target === event.currentTarget) {
      onClose();
    }
  };

  // Get source config for styling
  const sourceConfig = detail ? getSourceConfig(detail.source_type) : null;

  return (
    <div
      className="detail-modal__backdrop"
      onClick={handleBackdropClick}
      data-testid="detail-modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="detail-modal-title"
    >
      <div
        className="detail-modal"
        data-testid="detail-modal"
        style={
          sourceConfig
            ? {
                borderTop: `4px solid ${sourceConfig.primaryColor}`,
              }
            : undefined
        }
      >
        {/* Close button */}
        <button
          className="detail-modal__close"
          onClick={onClose}
          aria-label="Close detail view"
          data-testid="close-button"
        >
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>

        {/* Loading state */}
        {loading && (
          <div className="detail-modal__loading" data-testid="detail-loading">
            <LoadingState message="Loading content details..." />
          </div>
        )}

        {/* Error state */}
        {error && !loading && (
          <div className="detail-modal__error" data-testid="detail-error">
            <ErrorState
              message="Failed to load content details"
              details={error}
              onRetry={() => {
                // Trigger a re-fetch by closing and reopening
                // In a real app, we'd have a refresh function
              }}
            />
          </div>
        )}

        {/* Content */}
        {detail && !loading && !error && (
          <div className="detail-modal__content" data-testid="detail-content">
            {/* Header with logo and source type */}
            <div className="detail-modal__header">
              <SourceLogo sourceType={detail.source_type} size="md" />
              <span
                className="detail-modal__source-type"
                data-testid="detail-source-type"
              >
                {detail.source_type.charAt(0).toUpperCase() +
                  detail.source_type.slice(1)}
              </span>
            </div>

            {/* Title */}
            <h2
              id="detail-modal-title"
              className="detail-modal__title"
              data-testid="detail-title"
            >
              {detail.title || 'Untitled'}
            </h2>

            {/* Metadata section */}
            <div className="detail-modal__meta" data-testid="detail-meta">
              {/* Source URL */}
              <div className="detail-modal__meta-item">
                <span className="detail-modal__meta-label">Source:</span>
                <a
                  href={detail.source_path}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="detail-modal__link"
                  data-testid="detail-source-url"
                  title={detail.source_path}
                >
                  {truncateUrl(detail.source_path)}
                </a>
              </div>

              {/* Author (if available) */}
              {detail.author && (
                <div className="detail-modal__meta-item">
                  <span className="detail-modal__meta-label">Author:</span>
                  <span data-testid="detail-author">{detail.author}</span>
                </div>
              )}

              {/* Channel (if available) */}
              {detail.channel && (
                <div className="detail-modal__meta-item">
                  <span className="detail-modal__meta-label">Channel:</span>
                  <span data-testid="detail-channel">{detail.channel}</span>
                </div>
              )}

              {/* Chunk count */}
              <div className="detail-modal__meta-item">
                <span className="detail-modal__meta-label">Chunks:</span>
                <span data-testid="detail-chunk-count">{detail.chunk_count}</span>
              </div>

              {/* Created at */}
              <div className="detail-modal__meta-item">
                <span className="detail-modal__meta-label">Created:</span>
                <span data-testid="detail-created-at">
                  {formatDate(detail.created_at)}
                </span>
              </div>

              {/* Updated at */}
              <div className="detail-modal__meta-item">
                <span className="detail-modal__meta-label">Updated:</span>
                <span data-testid="detail-updated-at">
                  {formatDate(detail.updated_at)}
                </span>
              </div>
            </div>

            {/* Full text content */}
            <div className="detail-modal__text-section">
              <h3 className="detail-modal__text-heading">Content</h3>
              <div
                className="detail-modal__full-text"
                data-testid="detail-full-text"
              >
                {detail.full_text || (
                  <span className="detail-modal__no-content">
                    No content available
                  </span>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default DetailModal;
