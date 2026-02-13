//! Database module tests
//!
//! This module contains unit tests and property-based tests for the database layer.
//! Property tests use proptest with minimum 100 iterations per test.

use super::*;
use proptest::prelude::*;
use rusqlite::Connection;

/// Helper to create an in-memory test database with schema
fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    conn.execute_batch(
        r#"
        CREATE TABLE content_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type TEXT NOT NULL,
            source_path TEXT NOT NULL UNIQUE,
            content_hash TEXT NOT NULL,
            ehl_doc_id TEXT NOT NULL UNIQUE,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            ingestion_status TEXT NOT NULL DEFAULT 'ingested',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vector_index INTEGER,
            text TEXT NOT NULL,
            meta TEXT NOT NULL,
            is_deleted INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .expect("Failed to create schema");

    conn
}

/// Helper to create a ViewerDb from an existing connection (for testing)
fn viewer_db_from_conn(conn: Connection) -> ViewerDb {
    ViewerDb::from_connection(conn)
}

/// Generate a valid timestamp string for testing
fn generate_timestamp(base_seconds: u32) -> String {
    // Generate timestamps in format: 2024-01-01 HH:MM:SS
    let hours = (base_seconds / 3600) % 24;
    let minutes = (base_seconds / 60) % 60;
    let seconds = base_seconds % 60;
    format!(
        "2024-01-{:02} {:02}:{:02}:{:02}",
        1 + (base_seconds / 86400) % 28,
        hours,
        minutes,
        seconds
    )
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_get_default_db_path() {
        let path = get_default_db_path();
        assert!(path.contains(".ehl"));
        assert!(path.contains("ingestion.db"));
    }

    #[test]
    fn test_db_error_display() {
        let err = DbError::NotFound("/path/to/db".to_string());
        assert!(err.to_string().contains("/path/to/db"));

        let err = DbError::NotFound("test.db".to_string());
        assert!(err.to_string().contains("test.db"));
    }

    #[test]
    fn test_empty_database() {
        let conn = create_test_db();
        let db = viewer_db_from_conn(conn);

        let result = db.get_sources(0, 50).unwrap();
        assert_eq!(result.items.len(), 0);
        assert_eq!(result.total, 0);
        assert!(!result.has_more);
    }

    #[test]
    fn test_get_source_count_empty() {
        let conn = create_test_db();
        let db = viewer_db_from_conn(conn);

        let count = db.get_source_count().unwrap();
        assert_eq!(count, 0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 1: Content Sources Sorted by Updated Timestamp**
        ///
        /// *For any* set of content sources returned by the listing query, the sources
        /// SHALL be ordered by `updated_at` timestamp in descending order (newest first),
        /// such that for any two adjacent items, the first item's `updated_at` is greater
        /// than or equal to the second item's `updated_at`.
        ///
        /// **Validates: Requirements 2.2**
        #[test]
        fn property_1_sources_sorted_by_updated_timestamp(
            num_sources in 1usize..20,
            timestamps in prop::collection::vec(0u32..100000, 1..20)
        ) {
            let conn = create_test_db();

            // Insert sources with varying timestamps
            let actual_count = num_sources.min(timestamps.len());
            for i in 0..actual_count {
                let ehl_doc_id = format!("doc-{:08x}-0000-0000-0000-000000000000", i);
                let timestamp = generate_timestamp(timestamps[i]);

                conn.execute(
                    "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count, updated_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    rusqlite::params![
                        "browser",
                        format!("https://example.com/{}", i),
                        format!("hash{}", i),
                        ehl_doc_id,
                        0,
                        timestamp
                    ],
                ).expect("Failed to insert source");
            }

            let db = viewer_db_from_conn(conn);
            let result = db.get_sources(0, 100).expect("Failed to get sources");

            // Verify sorting: each item's updated_at should be >= next item's updated_at
            for window in result.items.windows(2) {
                prop_assert!(
                    window[0].updated_at >= window[1].updated_at,
                    "Sources not sorted by updated_at DESC: {} should be >= {}",
                    window[0].updated_at,
                    window[1].updated_at
                );
            }
        }

        /// **Property 2: Source Count Accuracy**
        ///
        /// *For any* database state, the total count returned by the listing query
        /// SHALL equal the actual number of content_sources records in the database.
        ///
        /// **Validates: Requirements 2.3**
        #[test]
        fn property_2_source_count_accuracy(
            num_sources in 0usize..50
        ) {
            let conn = create_test_db();

            // Insert random number of sources
            for i in 0..num_sources {
                let ehl_doc_id = format!("doc-{:08x}-0000-0000-0000-000000000000", i);

                conn.execute(
                    "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        "browser",
                        format!("https://example.com/{}", i),
                        format!("hash{}", i),
                        ehl_doc_id,
                        0
                    ],
                ).expect("Failed to insert source");
            }

            let db = viewer_db_from_conn(conn);

            // Verify get_source_count matches actual count
            let count = db.get_source_count().expect("Failed to get count");
            prop_assert_eq!(count as usize, num_sources, "Count mismatch");

            // Verify paginated response total also matches
            let result = db.get_sources(0, 100).expect("Failed to get sources");
            prop_assert_eq!(result.total as usize, num_sources, "Paginated total mismatch");
        }

        /// **Property 7: Chunk Reconstruction Correctness**
        ///
        /// *For any* set of chunks associated with an ehl_doc_id, the reconstruction function SHALL:
        /// 1. Include only chunks where `meta.id` matches the ehl_doc_id
        /// 2. Exclude all chunks where `is_deleted = 1`
        /// 3. Order chunks by `chunk_index` from the meta JSON in ascending order
        /// 4. Concatenate the `text` fields in order to produce the full content
        ///
        /// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
        #[test]
        fn property_7_chunk_reconstruction_correctness(
            num_chunks in 1usize..10,
            deleted_indices in prop::collection::vec(prop::bool::ANY, 1..10),
            chunk_texts in prop::collection::vec("[a-zA-Z0-9 ]{5,50}", 1..10)
        ) {
            let conn = create_test_db();
            let ehl_doc_id = "test-doc-0000-0000-0000-000000000001";

            // Insert content source
            conn.execute(
                "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
                 VALUES ('browser', 'https://test.com', 'hash', ?1, ?2)",
                rusqlite::params![ehl_doc_id, num_chunks as i32],
            ).expect("Failed to insert source");

            // Insert chunks with varying is_deleted status
            let actual_chunks = num_chunks.min(chunk_texts.len()).min(deleted_indices.len());
            let mut expected_text = String::new();

            for i in 0..actual_chunks {
                let is_deleted = deleted_indices[i];
                let text = &chunk_texts[i];
                let meta = serde_json::json!({
                    "id": ehl_doc_id,
                    "chunk_index": i,
                    "total_chunks": actual_chunks,
                    "source_type": "capture"
                });

                conn.execute(
                    "INSERT INTO chunks (text, meta, is_deleted) VALUES (?1, ?2, ?3)",
                    rusqlite::params![text, meta.to_string(), if is_deleted { 1 } else { 0 }],
                ).expect("Failed to insert chunk");

                // Build expected text from non-deleted chunks
                if !is_deleted {
                    expected_text.push_str(text);
                }
            }

            // Also insert a chunk for a different document (should be excluded)
            let other_meta = serde_json::json!({
                "id": "other-doc-0000-0000-0000-000000000002",
                "chunk_index": 0,
                "total_chunks": 1,
                "source_type": "capture"
            });
            conn.execute(
                "INSERT INTO chunks (text, meta, is_deleted) VALUES ('SHOULD NOT APPEAR', ?1, 0)",
                rusqlite::params![other_meta.to_string()],
            ).expect("Failed to insert other chunk");

            let db = viewer_db_from_conn(conn);
            let detail = db.get_detail(ehl_doc_id).expect("Failed to get detail");

            // Verify reconstructed text matches expected
            prop_assert_eq!(
                detail.full_text,
                expected_text,
                "Reconstructed text doesn't match expected"
            );
        }

        /// **Property 8: Pagination Threshold Behavior**
        ///
        /// *For any* dataset with more than 50 content sources, the initial query SHALL
        /// return at most 50 items and indicate `has_more = true`. *For any* dataset with
        /// 50 or fewer sources, the query SHALL return all items and indicate `has_more = false`.
        ///
        /// **Validates: Requirements 10.1**
        #[test]
        fn property_8_pagination_threshold_behavior(
            num_sources in 0usize..100
        ) {
            let conn = create_test_db();

            // Insert sources
            for i in 0..num_sources {
                let ehl_doc_id = format!("doc-{:08x}-0000-0000-0000-000000000000", i);

                conn.execute(
                    "INSERT INTO content_sources (source_type, source_path, content_hash, ehl_doc_id, chunk_count)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        "browser",
                        format!("https://example.com/{}", i),
                        format!("hash{}", i),
                        ehl_doc_id,
                        0
                    ],
                ).expect("Failed to insert source");
            }

            let db = viewer_db_from_conn(conn);

            // Query with page_size = 50 (the threshold)
            let result = db.get_sources(0, 50).expect("Failed to get sources");

            if num_sources > 50 {
                // Should return exactly 50 items with has_more = true
                prop_assert_eq!(result.items.len(), 50, "Should return exactly 50 items when > 50 sources");
                prop_assert!(result.has_more, "has_more should be true when > 50 sources");
            } else {
                // Should return all items with has_more = false
                prop_assert_eq!(result.items.len(), num_sources, "Should return all items when <= 50 sources");
                prop_assert!(!result.has_more, "has_more should be false when <= 50 sources");
            }

            // Total should always match actual count
            prop_assert_eq!(result.total as usize, num_sources, "Total should match actual count");
        }
    }
}
