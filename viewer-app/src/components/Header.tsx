/**
 * Header component - Displays app title and total content count
 * 
 * Requirement 2.3: WHEN content sources are loaded, THE Viewer_App SHALL
 * display the total count of sources
 */

import React from 'react';

export interface HeaderProps {
  /** Total count of content sources */
  totalCount: number;
  /** Whether the count is still loading */
  loading?: boolean;
  /** Callback to refresh content */
  onRefresh?: () => void;
}

/**
 * Header component
 * Displays the application title and total content count
 * 
 * @param totalCount - Total number of content sources
 * @param loading - Whether the count is still loading
 * @param onRefresh - Callback to refresh content
 */
export const Header: React.FC<HeaderProps> = ({
  totalCount,
  loading = false,
  onRefresh,
}) => {
  return (
    <header className="app-header" data-testid="app-header">
      <div className="app-header__content">
        <h1 className="app-header__title" data-testid="app-title">
          Content Viewer
        </h1>
        <p className="app-header__subtitle">
          Browse your ingested content
        </p>
      </div>
      
      <div className="app-header__actions" data-testid="header-actions">
        {onRefresh && (
          <button
            className="app-header__refresh"
            onClick={onRefresh}
            disabled={loading}
            aria-label="Refresh content"
            data-testid="refresh-button"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={loading ? 'spinning' : ''}
            >
              <polyline points="23 4 23 10 17 10" />
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
            </svg>
          </button>
        )}
        {loading ? (
          <span className="app-header__count app-header__count--loading">
            Loading...
          </span>
        ) : (
          <span className="app-header__count" data-testid="content-count">
            {totalCount} {totalCount === 1 ? 'item' : 'items'}
          </span>
        )}
      </div>
    </header>
  );
};

export default Header;
