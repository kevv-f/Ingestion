/**
 * GridView component - Renders responsive grid of ContentCard components
 * 
 * Requirements:
 * - 3.1: Display content sources as rectangular cards arranged in a responsive grid layout
 * - 3.6: Adjust grid column count based on window width to maintain responsive layout
 * - 10.1: Implement pagination or virtual scrolling for datasets exceeding 50 items
 * - 10.4: Display loading indicator while fetching content
 */

import React from 'react';
import { ContentCard } from './ContentCard';
import { LoadingState } from './LoadingState';
import type { ContentSourceView } from '../types';

export interface GridViewProps {
  sources: ContentSourceView[];
  onCardClick: (ehlDocId: string) => void;
  onLoadMore: () => void;
  onDelete?: (ehlDocId: string) => void;
  hasMore: boolean;
  loading: boolean;
}

/**
 * GridView component
 * Renders a responsive grid of content cards with load more functionality
 * 
 * @param sources - Array of content sources to display
 * @param onCardClick - Callback when a card is clicked, receives ehlDocId
 * @param onLoadMore - Callback to load more content
 * @param onDelete - Callback when delete button is clicked, receives ehlDocId
 * @param hasMore - Whether there are more items to load
 * @param loading - Whether content is currently being loaded
 */
export const GridView: React.FC<GridViewProps> = ({
  sources,
  onCardClick,
  onLoadMore,
  onDelete,
  hasMore,
  loading,
}) => {
  return (
    <div className="grid-view" data-testid="grid-view">
      <div className="grid-view__grid" data-testid="grid-container">
        {sources.map((source) => (
          <ContentCard
            key={source.ehl_doc_id}
            source={source}
            onClick={() => onCardClick(source.ehl_doc_id)}
            onDelete={onDelete}
          />
        ))}
      </div>
      
      {/* Loading indicator */}
      {loading && (
        <div className="grid-view__loading" data-testid="grid-loading">
          <LoadingState message="Loading content..." />
        </div>
      )}
      
      {/* Load more button */}
      {hasMore && !loading && (
        <div className="grid-view__load-more">
          <button
            className="grid-view__load-more-button"
            onClick={onLoadMore}
            data-testid="load-more-button"
          >
            Load More
          </button>
        </div>
      )}
      
      {/* End of content indicator */}
      {!hasMore && sources.length > 0 && !loading && (
        <div className="grid-view__end" data-testid="end-of-content">
          <span>You've reached the end</span>
        </div>
      )}
    </div>
  );
};

export default GridView;
