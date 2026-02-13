/**
 * EmptyState component - Displays message when no content exists
 * 
 * Requirement 2.4: IF no content sources exist, THEN THE Viewer_App SHALL
 * display an empty state message indicating no content has been ingested
 */

import React from 'react';

export interface EmptyStateProps {
  /** Custom message to display (optional) */
  message?: string;
  /** Custom title to display (optional) */
  title?: string;
}

/**
 * EmptyState component
 * Displays a friendly message when there is no content to show
 * 
 * @param message - Custom message to display
 * @param title - Custom title to display
 */
export const EmptyState: React.FC<EmptyStateProps> = ({
  message = 'No content has been ingested yet. Start capturing content from your browser to see it here.',
  title = 'No Content Found',
}) => {
  return (
    <div className="empty-state" data-testid="empty-state">
      {/* Empty state icon */}
      <div className="empty-state__icon" aria-hidden="true">
        <svg
          width="64"
          height="64"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
          <polyline points="13 2 13 9 20 9" />
          <line x1="9" y1="13" x2="15" y2="13" />
          <line x1="9" y1="17" x2="15" y2="17" />
        </svg>
      </div>
      
      <h2 className="empty-state__title" data-testid="empty-state-title">
        {title}
      </h2>
      
      <p className="empty-state__message" data-testid="empty-state-message">
        {message}
      </p>
    </div>
  );
};

export default EmptyState;
