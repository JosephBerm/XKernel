// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Vector indexing and semantic prefetch mechanisms for fast memory retrieval.

use alloc::vec::Vec;
use core::fmt;

/// Vector index for semantic search across memory tiers
#[derive(Debug, Clone)]
pub struct VectorIndex {
    /// Dimension of embedding vectors
    pub dimension: usize,
    /// Stored vectors with their entry IDs
    vectors: Vec<(u64, Vec<f32>)>,
    /// Maximum size before compaction
    max_vectors: usize,
}

impl VectorIndex {
    pub fn new(dimension: usize, max_vectors: usize) -> Self {
        Self {
            dimension,
            vectors: Vec::new(),
            max_vectors,
        }
    }

    /// Add a vector with entry ID
    pub fn insert(&mut self, entry_id: u64, embedding: Vec<f32>) -> Result<(), IndexError> {
        if embedding.len() != self.dimension {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimension,
                got: embedding.len(),
            });
        }

        // Remove if already present
        self.vectors.retain(|(id, _)| *id != entry_id);
        self.vectors.push((entry_id, embedding));

        Ok(())
    }

    /// Remove a vector by entry ID
    pub fn remove(&mut self, entry_id: u64) -> bool {
        let old_len = self.vectors.len();
        self.vectors.retain(|(id, _)| *id != entry_id);
        self.vectors.len() < old_len
    }

    /// Find vectors nearest to query embedding
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>, IndexError> {
        if query.len() != self.dimension {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }

        let mut results: Vec<_> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let distance = Self::cosine_distance(query, vec);
                SearchResult {
                    entry_id: *id,
                    distance,
                }
            })
            .collect();

        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        results.truncate(k);

        Ok(results)
    }

    /// Compute cosine similarity distance
    fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        let denominator = (norm_a.sqrt() * norm_b.sqrt()).max(1e-9);
        1.0 - (dot_product / denominator)
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.vectors.len() >= self.max_vectors
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new(768, 10000)
    }
}

/// Search result with distance score
#[derive(Debug, Clone, Copy)]
pub struct SearchResult {
    pub entry_id: u64,
    pub distance: f32,
}

/// Semantic prefetch strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchStrategy {
    /// No prefetching
    None,
    /// Prefetch semantically similar entries
    Semantic,
    /// Prefetch based on access patterns
    AccessPattern,
    /// Hybrid approach combining both
    Hybrid,
}

/// Semantic prefetch engine
#[derive(Debug, Clone)]
pub struct SemanticPrefetch {
    strategy: PrefetchStrategy,
    /// Queue of entry IDs to prefetch
    prefetch_queue: Vec<u64>,
    /// Maximum prefetch queue size
    max_queue_size: usize,
}

impl SemanticPrefetch {
    pub fn new(strategy: PrefetchStrategy, max_queue_size: usize) -> Self {
        Self {
            strategy,
            prefetch_queue: Vec::new(),
            max_queue_size,
        }
    }

    /// Add entries to prefetch based on similarity to current access
    pub fn add_prefetch_candidates(&mut self, candidates: Vec<u64>) {
        if self.strategy == PrefetchStrategy::None {
            return;
        }

        for candidate in candidates {
            if self.prefetch_queue.len() < self.max_queue_size
                && !self.prefetch_queue.contains(&candidate)
            {
                self.prefetch_queue.push(candidate);
            }
        }
    }

    /// Get next entry to prefetch
    pub fn next_prefetch(&mut self) -> Option<u64> {
        if self.prefetch_queue.is_empty() {
            None
        } else {
            Some(self.prefetch_queue.remove(0))
        }
    }

    pub fn pending_count(&self) -> usize {
        self.prefetch_queue.len()
    }

    pub fn clear_queue(&mut self) {
        self.prefetch_queue.clear();
    }
}

impl Default for SemanticPrefetch {
    fn default() -> Self {
        Self::new(PrefetchStrategy::Semantic, 100)
    }
}

/// Index errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    DimensionMismatch { expected: usize, got: usize },
    CapacityExceeded,
    EntryNotFound,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, got } => {
                write!(f, "dimension mismatch: expected {}, got {}", expected, got)
            }
            Self::CapacityExceeded => write!(f, "index capacity exceeded"),
            Self::EntryNotFound => write!(f, "entry not found in index"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_index_insert() {
        let mut idx = VectorIndex::new(3, 10);
        let vec = alloc::vec![0.1, 0.2, 0.3];
        assert!(idx.insert(1, vec).is_ok());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn test_vector_dimension_check() {
        let mut idx = VectorIndex::new(3, 10);
        let vec = alloc::vec![0.1, 0.2]; // Wrong dimension
        assert!(matches!(
            idx.insert(1, vec),
            Err(IndexError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_vector_search() {
        let mut idx = VectorIndex::new(2, 10);
        idx.insert(1, alloc::vec![1.0, 0.0]).unwrap();
        idx.insert(2, alloc::vec![0.0, 1.0]).unwrap();
        idx.insert(3, alloc::vec![0.99, 0.01]).unwrap(); // Similar to query

        let results = idx.search(&[1.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entry_id, 1); // Exact match closest
        assert_eq!(results[1].entry_id, 3); // Similar entry second
    }

    #[test]
    fn test_semantic_prefetch() {
        let mut prefetch = SemanticPrefetch::new(PrefetchStrategy::Semantic, 3);
        prefetch.add_prefetch_candidates(alloc::vec![1, 2, 3]);
        assert_eq!(prefetch.pending_count(), 3);

        assert_eq!(prefetch.next_prefetch(), Some(1));
        assert_eq!(prefetch.pending_count(), 2);
    }

    #[test]
    fn test_prefetch_no_strategy() {
        let mut prefetch = SemanticPrefetch::new(PrefetchStrategy::None, 10);
        prefetch.add_prefetch_candidates(alloc::vec![1, 2, 3]);
        assert_eq!(prefetch.pending_count(), 0); // Nothing prefetched
    }
}
