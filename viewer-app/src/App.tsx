/**
 * Main App component - Coordinates all views and state management
 * 
 * Requirements:
 * - 1.5: WHEN the database connection succeeds, THE Viewer_App SHALL load and display content sources
 * - 5.2: WHEN the user clicks a card, THE Viewer_App SHALL open the Detail_View
 * - 5.3: THE Viewer_App SHALL provide smooth transition animations when opening the Detail_View
 * - 9.2: THE Viewer_App SHALL use smooth CSS transitions for hover effects and view changes
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Header,
  GridView,
  DetailModal,
  EmptyState,
  ErrorState,
  LoadingState,
} from './components';
import { useContentSources } from './hooks';
import './styles.css';
import './App.css';

/**
 * Main App component
 * 
 * Manages the application state including:
 * - Selected item state for detail view
 * - Coordination between GridView and DetailModal
 * - View transitions between grid and detail views
 */
function App() {
  // State for selected item (ehl_doc_id) to show in detail view
  const [selectedEhlDocId, setSelectedEhlDocId] = useState<string | null>(null);

  // Fetch content sources using the custom hook
  const {
    sources,
    loading,
    error,
    hasMore,
    loadMore,
    refresh,
  } = useContentSources(50);

  /**
   * Handle card click - opens detail modal
   * @param ehlDocId - The EHL document ID of the clicked card
   */
  const handleCardClick = useCallback((ehlDocId: string) => {
    setSelectedEhlDocId(ehlDocId);
  }, []);

  /**
   * Handle detail modal close
   */
  const handleCloseDetail = useCallback(() => {
    setSelectedEhlDocId(null);
  }, []);

  /**
   * Handle delete action - deletes content source from database
   * @param ehlDocId - The EHL document ID to delete
   */
  const handleDelete = useCallback(async (ehlDocId: string) => {
    try {
      await invoke('delete_content_source', { ehlDocId });
      // Refresh the list after deletion
      refresh();
    } catch (err) {
      console.error('Failed to delete content source:', err);
    }
  }, [refresh]);

  /**
   * Handle retry action for error state
   */
  const handleRetry = useCallback(() => {
    refresh();
  }, [refresh]);

  // Calculate total count from sources length
  // Note: In a real app, we might want to get this from the API response
  const totalCount = sources.length;

  // Determine if this is the initial loading state (no sources yet)
  const isInitialLoading = loading && sources.length === 0;

  // Determine if we should show empty state
  const showEmptyState = !loading && !error && sources.length === 0;

  // Determine if we should show error state
  const showErrorState = error !== null && sources.length === 0;

  // Determine if we should show the grid
  const showGrid = sources.length > 0;

  return (
    <div className="app" data-testid="app">
      {/* Header with title and count */}
      <Header
        totalCount={totalCount}
        loading={isInitialLoading}
        onRefresh={refresh}
      />

      {/* Main content area */}
      <main className="app__main">
        {/* Initial loading state */}
        {isInitialLoading && (
          <div className="app__loading" data-testid="app-loading">
            <LoadingState
              message="Loading content sources..."
              size="lg"
            />
          </div>
        )}

        {/* Error state */}
        {showErrorState && (
          <div className="app__error" data-testid="app-error">
            <ErrorState
              message="Failed to load content sources"
              details={error}
              onRetry={handleRetry}
            />
          </div>
        )}

        {/* Empty state */}
        {showEmptyState && (
          <div className="app__empty" data-testid="app-empty">
            <EmptyState />
          </div>
        )}

        {/* Grid view */}
        {showGrid && (
          <GridView
            sources={sources}
            onCardClick={handleCardClick}
            onLoadMore={loadMore}
            onDelete={handleDelete}
            hasMore={hasMore}
            loading={loading}
          />
        )}
      </main>

      {/* Detail modal - rendered when an item is selected */}
      {selectedEhlDocId && (
        <DetailModal
          ehlDocId={selectedEhlDocId}
          onClose={handleCloseDetail}
        />
      )}
    </div>
  );
}

export default App;
