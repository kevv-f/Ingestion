/**
 * TypeScript type definitions for the Viewer App
 * These types mirror the Rust backend types for type-safe communication
 */

/**
 * Source type representing the origin platform of content
 * Known types: slack, gmail, jira, browser, chrome, gdocs, gsheets
 * Can also be any other string for unknown/custom sources
 */
export type SourceType = 'slack' | 'gmail' | 'jira' | 'browser' | 'chrome' | 'gdocs' | 'gsheets' | 'gslides' | string;

/**
 * View model for content source list items
 * Used in the grid view to display content cards
 */
export interface ContentSourceView {
  id: number;
  source_type: SourceType;
  source_path: string;
  ehl_doc_id: string;
  chunk_count: number;
  created_at: string;
  updated_at: string;
  /** Title extracted from first chunk's meta, may be null */
  title: string | null;
  /** Preview text for card display */
  preview_text: string;
  /** Application display name (e.g., "Microsoft Word") */
  app_name: string | null;
  /** Application bundle ID (e.g., "com.microsoft.Word") */
  bundle_id: string | null;
}

/**
 * Full content detail for detail view
 * Contains all metadata and reconstructed full text
 */
export interface ContentDetail {
  id: number;
  source_type: SourceType;
  source_path: string;
  ehl_doc_id: string;
  chunk_count: number;
  created_at: string;
  updated_at: string;
  /** Full title, may be null */
  title: string | null;
  /** Author from chunk metadata, may be null */
  author: string | null;
  /** Channel/workspace from chunk metadata, may be null */
  channel: string | null;
  /** Full reconstructed text from all chunks */
  full_text: string;
  /** Application display name (e.g., "Microsoft Word") */
  app_name: string | null;
  /** Application bundle ID (e.g., "com.microsoft.Word") */
  bundle_id: string | null;
}

/**
 * Generic paginated response wrapper
 * Used for all paginated API responses
 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
}

/**
 * Configuration for source type visual styling
 * Used for card gradients and logo selection
 */
export interface SourceConfig {
  /** Logo component or identifier for the source type */
  logo: string;
  /** Primary brand color for the source */
  primaryColor: string;
  /** Gradient colors for glass morphism effect [start, end] */
  gradientColors: [string, string];
}

/**
 * Database statistics returned by get_stats command
 */
export interface DbStats {
  total_sources: number;
  total_chunks: number;
}

/**
 * Error state for UI error handling
 */
export interface ErrorState {
  type: 'connection' | 'query' | 'parse';
  message: string;
  details?: string;
  retryable: boolean;
}

/**
 * Chunk metadata JSON structure
 * Stored in the meta field of chunks table
 */
export interface ChunkMeta {
  /** ehl_doc_id - links chunk to content_source */
  id: string;
  /** source type */
  source: string;
  /** source URL */
  url: string;
  /** optional title */
  title?: string;
  /** optional author */
  author?: string;
  /** optional channel/workspace */
  channel?: string;
  /** position in sequence */
  chunk_index: number;
  /** total chunks for this document */
  total_chunks: number;
  /** source type identifier */
  source_type: string;
  /** Application display name (e.g., "Microsoft Word") */
  app_name?: string;
  /** Application bundle ID (e.g., "com.microsoft.Word") */
  bundle_id?: string;
}
