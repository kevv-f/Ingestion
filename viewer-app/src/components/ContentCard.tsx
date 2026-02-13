/**
 * ContentCard component - Displays content source as an interactive card
 * 
 * Requirements:
 * - 3.1: Display content sources as rectangular cards
 * - 3.2: Show source type logo, title, truncated preview text, and source type label
 * - 3.4: Apply glass morphism gradient effect to card background
 * - 3.5: Tint glass gradient to complement source logo colors
 * - 5.1: Display visual hover effect indicating interactivity
 * - 5.2: Handle click to select and open detail view
 */

import React, { useState } from 'react';
import { SourceLogo } from './SourceLogo';
import { getSourceConfig } from '../utils/sourceConfig';
import { truncateText } from '../utils/truncate';
import type { ContentSourceView } from '../types';

export interface ContentCardProps {
  source: ContentSourceView;
  onClick: () => void;
  onDelete?: (ehlDocId: string) => void;
}

/**
 * ContentCard component
 * Displays a content source as an interactive card with glass morphism styling
 * 
 * @param source - The content source data to display
 * @param onClick - Callback when the card is clicked
 * @param onDelete - Callback when the delete button is clicked
 */
export const ContentCard: React.FC<ContentCardProps> = ({ source, onClick, onDelete }) => {
  const [isHovered, setIsHovered] = useState(false);
  const config = getSourceConfig(source.source_type);
  
  // Create CSS custom property for source-specific gradient
  // This allows the gradient to be applied via the ::before pseudo-element
  // while maintaining the glass morphism base styles from CSS
  const cardStyle: React.CSSProperties = {
    '--card-gradient': `linear-gradient(135deg, ${config.gradientColors[0]}, ${config.gradientColors[1]})`,
  } as React.CSSProperties;

  // Display title or fallback to "Untitled"
  const displayTitle = source.title || 'Untitled';
  
  // Truncate preview text for card display
  const previewText = truncateText(source.preview_text, 150);

  // Format source type for display (capitalize first letter)
  const sourceTypeLabel = source.source_type.charAt(0).toUpperCase() + source.source_type.slice(1);

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (onDelete) {
      onDelete(source.ehl_doc_id);
    }
  };

  return (
    <article
      className="content-card"
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick();
        }
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      role="button"
      tabIndex={0}
      aria-label={`View details for ${displayTitle}`}
      data-testid="content-card"
      data-source-type={source.source_type}
      style={cardStyle}
    >
      {isHovered && onDelete && (
        <button
          className="content-card__delete"
          onClick={handleDelete}
          aria-label={`Delete ${displayTitle}`}
          title="Delete"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            <line x1="10" y1="11" x2="10" y2="17"></line>
            <line x1="14" y1="11" x2="14" y2="17"></line>
          </svg>
        </button>
      )}
      <div className="content-card__header">
        <SourceLogo sourceType={source.source_type} size="md" />
        <span 
          className="content-card__source-type"
          data-testid="source-type-label"
        >
          {sourceTypeLabel}
        </span>
      </div>
      
      <h3 
        className="content-card__title"
        data-testid="card-title"
      >
        {displayTitle}
      </h3>
      
      <p 
        className="content-card__preview"
        data-testid="card-preview"
      >
        {previewText}
      </p>
      
      <div className="content-card__footer">
        <span className="content-card__date">
          {new Date(source.updated_at).toLocaleDateString()}
        </span>
        <span className="content-card__chunks">
          {source.chunk_count} chunk{source.chunk_count !== 1 ? 's' : ''}
        </span>
      </div>
    </article>
  );
};

export default ContentCard;
