import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ContentDetail } from '../types';

/**
 * Hook for fetching content detail from the backend
 * 
 * Fetches full content detail when ehlDocId changes, including
 * reconstructed text from all chunks and metadata.
 * 
 * @param ehlDocId - The EHL document ID to fetch detail for, or null to clear
 * @returns Object containing detail data, loading state, and error state
 * 
 * Requirements: 6.1-6.8
 */
export function useContentDetail(ehlDocId: string | null): {
  detail: ContentDetail | null;
  loading: boolean;
  error: string | null;
} {
  const [detail, setDetail] = useState<ContentDetail | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Clear detail and error when ehlDocId is null
    if (ehlDocId === null) {
      setDetail(null);
      setError(null);
      setLoading(false);
      return;
    }

    // Fetch detail for the given ehlDocId
    const fetchDetail = async () => {
      setLoading(true);
      setError(null);

      try {
        const response = await invoke<ContentDetail>(
          'get_content_detail',
          { ehlDocId }
        );
        setDetail(response);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(errorMessage);
        setDetail(null);
        console.error('Failed to fetch content detail:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchDetail();
  }, [ehlDocId]);

  return {
    detail,
    loading,
    error,
  };
}

export default useContentDetail;
