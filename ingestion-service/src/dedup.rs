//! Deduplication cache and logic

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Entry in the dedup cache
#[derive(Debug, Clone)]
struct CacheEntry {
    content_hash: String,
    ehl_doc_id: String,
    last_seen: Instant,
}

/// In-memory deduplication cache
/// Caches source_path → content_hash for fast duplicate detection
pub struct DedupCache {
    entries: HashMap<String, CacheEntry>,
    /// How long entries stay in cache without being accessed
    ttl: Duration,
    /// Maximum cache size
    max_entries: usize,
}

impl DedupCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
            max_entries,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(
            Duration::from_secs(24 * 60 * 60), // 24 hours
            10_000,                             // 10k entries
        )
    }

    /// Check if content is a duplicate
    /// Returns Some(ehl_doc_id) if duplicate, None if new/changed
    pub fn check(&mut self, source_path: &str, content_hash: &str) -> DedupResult {
        // Clean expired entries periodically
        if self.entries.len() > self.max_entries {
            self.evict_expired();
        }

        match self.entries.get_mut(source_path) {
            Some(entry) => {
                entry.last_seen = Instant::now();
                if entry.content_hash == content_hash {
                    DedupResult::Duplicate(entry.ehl_doc_id.clone())
                } else {
                    DedupResult::Changed(entry.ehl_doc_id.clone())
                }
            }
            None => DedupResult::New,
        }
    }

    /// Insert or update a cache entry
    pub fn insert(&mut self, source_path: String, content_hash: String, ehl_doc_id: String) {
        self.entries.insert(
            source_path,
            CacheEntry {
                content_hash,
                ehl_doc_id,
                last_seen: Instant::now(),
            },
        );
    }

    /// Remove expired entries
    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.duration_since(entry.last_seen) < self.ttl);
    }

    /// Get cache stats
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupResult {
    /// Content is new, not seen before
    New,
    /// Content exists and is unchanged
    Duplicate(String), // ehl_doc_id
    /// Content exists but has changed
    Changed(String), // ehl_doc_id
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
}

/// Compute SHA-256 hash of content
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_content() {
        let mut cache = DedupCache::with_defaults();
        let result = cache.check("https://example.com", "hash123");
        assert_eq!(result, DedupResult::New);
    }

    #[test]
    fn test_duplicate_content() {
        let mut cache = DedupCache::with_defaults();
        cache.insert(
            "https://example.com".to_string(),
            "hash123".to_string(),
            "doc-uuid".to_string(),
        );

        let result = cache.check("https://example.com", "hash123");
        assert_eq!(result, DedupResult::Duplicate("doc-uuid".to_string()));
    }

    #[test]
    fn test_changed_content() {
        let mut cache = DedupCache::with_defaults();
        cache.insert(
            "https://example.com".to_string(),
            "hash123".to_string(),
            "doc-uuid".to_string(),
        );

        let result = cache.check("https://example.com", "hash456");
        assert_eq!(result, DedupResult::Changed("doc-uuid".to_string()));
    }

    #[test]
    fn test_hash_computation() {
        let hash1 = compute_hash("hello world");
        let hash2 = compute_hash("hello world");
        let hash3 = compute_hash("hello world!");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
