//! Content chunker - splits text into fixed-size token chunks
//! 
//! For tabular data (Excel, CSV), preserves row structure by chunking at row boundaries.

/// Chunk configuration
pub struct ChunkerConfig {
    /// Maximum tokens per chunk (1 word = 1 token)
    pub max_tokens: usize,
    /// Overlap tokens between chunks for context continuity
    pub overlap_tokens: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_tokens: 1024,
            overlap_tokens: 100,
        }
    }
}

/// A single chunk of content
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub token_count: usize,
}

/// Chunker splits content into fixed-size token chunks
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Check if content appears to be tabular (Excel/CSV-like)
    fn is_tabular_content(content: &str) -> bool {
        // Check if content has tabs or consistent structure suggesting tabular data
        let lines: Vec<&str> = content.lines().take(10).collect();
        if lines.len() < 2 {
            return false;
        }
        
        // Check if multiple lines have tabs
        let lines_with_tabs = lines.iter().filter(|l| l.contains('\t')).count();
        lines_with_tabs >= 2
    }

    /// Split content into chunks, preserving structure for tabular data
    pub fn chunk(&self, content: &str) -> Vec<Chunk> {
        if content.trim().is_empty() {
            return vec![];
        }

        // For tabular content, use line-based chunking to preserve structure
        if Self::is_tabular_content(content) {
            return self.chunk_tabular(content);
        }

        // For regular text, use word-based chunking
        self.chunk_text(content)
    }

    /// Chunk tabular content by lines, preserving row structure
    fn chunk_tabular(&self, content: &str) -> Vec<Chunk> {
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() {
            return vec![];
        }

        // Estimate tokens per line (count words in each line)
        let mut chunks = Vec::new();
        let mut current_lines = Vec::new();
        let mut current_tokens = 0;

        for line in lines {
            let line_tokens = line.split_whitespace().count().max(1);
            
            // If adding this line would exceed max_tokens, start a new chunk
            if current_tokens + line_tokens > self.config.max_tokens && !current_lines.is_empty() {
                let chunk_text = current_lines.join("\n");
                chunks.push(Chunk {
                    text: chunk_text,
                    chunk_index: chunks.len(),
                    total_chunks: 0,
                    token_count: current_tokens,
                });
                
                // Keep some overlap lines for context
                let overlap_lines = current_lines.len().min(3);
                current_lines = current_lines.split_off(current_lines.len() - overlap_lines);
                current_tokens = current_lines.iter()
                    .map(|l: &&str| l.split_whitespace().count().max(1))
                    .sum();
            }
            
            current_lines.push(line);
            current_tokens += line_tokens;
        }

        // Add remaining content
        if !current_lines.is_empty() {
            let chunk_text = current_lines.join("\n");
            chunks.push(Chunk {
                text: chunk_text,
                chunk_index: chunks.len(),
                total_chunks: 0,
                token_count: current_tokens,
            });
        }

        // Set total_chunks
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total;
        }

        chunks
    }

    /// Chunk regular text content by words
    fn chunk_text(&self, content: &str) -> Vec<Chunk> {
        let words: Vec<&str> = content.split_whitespace().collect();

        if words.is_empty() {
            return vec![];
        }

        // If content fits in one chunk, return as-is (preserve original formatting)
        if words.len() <= self.config.max_tokens {
            return vec![Chunk {
                text: content.to_string(),
                chunk_index: 0,
                total_chunks: 1,
                token_count: words.len(),
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let step = self.config.max_tokens - self.config.overlap_tokens;

        while start < words.len() {
            let end = (start + self.config.max_tokens).min(words.len());
            let chunk_words = &words[start..end];
            let chunk_text = chunk_words.join(" ");

            chunks.push(Chunk {
                text: chunk_text,
                chunk_index: chunks.len(),
                total_chunks: 0, // Will be set after we know total
                token_count: chunk_words.len(),
            });

            start += step;

            // Avoid tiny final chunks
            if words.len() - start < self.config.overlap_tokens && start < words.len() {
                // Include remaining in last chunk
                let remaining = &words[start..];
                if let Some(last) = chunks.last_mut() {
                    last.text = format!("{} {}", last.text, remaining.join(" "));
                    last.token_count += remaining.len();
                }
                break;
            }
        }

        // Set total_chunks
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total;
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_content_single_chunk() {
        let chunker = Chunker::with_defaults();
        let content = "Hello world this is a test";
        let chunks = chunker.chunk(content);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].total_chunks, 1);
        assert_eq!(chunks[0].token_count, 6);
    }

    #[test]
    fn test_large_content_multiple_chunks() {
        let config = ChunkerConfig {
            max_tokens: 10,
            overlap_tokens: 2,
        };
        let chunker = Chunker::new(config);

        // 25 words
        let content = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four twenty-five";
        let chunks = chunker.chunk(content);

        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.token_count <= 12); // max + some overlap tolerance
        }
    }

    #[test]
    fn test_empty_content() {
        let chunker = Chunker::with_defaults();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_tabular_content_preserves_structure() {
        let chunker = Chunker::with_defaults();
        // Simulated Excel content with tabs
        let content = "Name\tAge\tCity\nAlice\t30\tNew York\nBob\t25\tLos Angeles\nCharlie\t35\tChicago";
        let chunks = chunker.chunk(content);

        assert_eq!(chunks.len(), 1);
        // Verify tabs are preserved
        assert!(chunks[0].text.contains('\t'));
        // Verify newlines are preserved
        assert!(chunks[0].text.contains('\n'));
    }

    #[test]
    fn test_tabular_detection() {
        // Should detect as tabular
        let tabular = "A\tB\tC\n1\t2\t3\n4\t5\t6";
        assert!(Chunker::is_tabular_content(tabular));

        // Should not detect as tabular (no tabs)
        let regular = "Hello world\nThis is text\nNo tabs here";
        assert!(!Chunker::is_tabular_content(regular));
    }
}
