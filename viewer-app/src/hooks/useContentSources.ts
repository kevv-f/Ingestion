import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ContentSourceView, PaginatedResponse } from '../types';

/**
 * Hook for fetching paginated content sources from the backend
 * 
 * Manages pagination state, loading/error states, and provides
 * functions to load more content and refresh the list.
 * Auto-refreshes when database changes are detected.
 * 
 * @param pageSize - Number of items per page (default: 50)
 * @returns Object containing sources, loading state, error state, and control functions
 * 
 * Requirements: 2.1, 10.1, 10.3, 10.4
 */
export function useContentSources(pageSize: number = 50): {
  sources: ContentSourceView[];
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  loadMore: () => void;
  refresh: () => void;
} {
  const [sources, setSources] = useState<ContentSourceView[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState<boolean>(false);
  const [currentPage, setCurrentPage] = useState<number>(0); // 0-indexed for backend

  /**
   * Fetch content sources from the backend
   * @param page - Page number to fetch (0-indexed)
   * @param append - Whether to append to existing sources or replace
   */
  const fetchSources = useCallback(async (page: number, append: boolean = false) => {
    setLoading(true);
    setError(null);

    try {
      const response = await invoke<PaginatedResponse<ContentSourceView>>(
        'get_content_sources',
        { page, limit: pageSize }
      );

      if (append) {
        setSources(prev => [...prev, ...response.items]);
      } else {
        setSources(response.items);
      }

      setHasMore(response.has_more);
      setCurrentPage(response.page);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to fetch content sources:', err);
    } finally {
      setLoading(false);
    }
  }, [pageSize]);

  /**
   * Load the next page of content sources
   * Only loads if not currently loading and there are more items
   */
  const loadMore = useCallback(() => {
    if (!loading && hasMore) {
      fetchSources(currentPage + 1, true);
    }
  }, [loading, hasMore, currentPage, fetchSources]);

  /**
   * Refresh the content sources list from the beginning
   * Clears existing sources and fetches the first page
   */
  const refresh = useCallback(() => {
    setSources([]);
    setCurrentPage(0);
    setHasMore(false);
    fetchSources(0, false);
  }, [fetchSources]);

  // Initial fetch on mount
  useEffect(() => {
    fetchSources(0, false);
  }, [fetchSources]);

  // Listen for database changes and auto-refresh
  useEffect(() => {
    const unlisten = listen('db-changed', () => {
      console.log('Database changed, refreshing content...');
      refresh();
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [refresh]);

  return {
    sources,
    loading,
    error,
    hasMore,
    loadMore,
    refresh,
  };
}

export default useContentSources;
