// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Embedded vector indexing for L2 episodic memory.
//!
//! This module provides vector embedding indexing capabilities for semantic
//! similarity search in L2 memory, without external dependencies like pgvector.
//! Supports multiple distance metrics and quantization schemes.
//!
//! See Engineering Plan § 4.1.2: L2 Vector Indexing.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};

/// Vector dimension type (typical values: 768, 1024, 1536 for embedding models).
///
/// Represents the dimensionality of embeddings.
/// See Engineering Plan § 4.1.2: Vector Dimensions.
pub type VectorDimension = u16;

/// Quantization scheme for vector storage.
///
/// Allows trading off storage space for precision in vector indexing.
/// See Engineering Plan § 4.1.2: Vector Quantization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantizationType {
    /// Full-precision floating point (32-bit)
    Float32,

    /// Half-precision floating point (16-bit)
    Float16,

    /// 8-bit signed integer quantization
    Int8,

    /// Binary (1-bit) quantization via hashing
    Binary,
}

impl QuantizationType {
    /// Returns the bytes per vector element for this quantization type.
    pub fn bytes_per_element(&self) -> u32 {
        match self {
            QuantizationType::Float32 => 4,
            QuantizationType::Float16 => 2,
            QuantizationType::Int8 => 1,
            QuantizationType::Binary => 1, // 8 bits per byte
        }
    }

    /// Returns the compression ratio relative to Float32.
    pub fn compression_ratio(&self) -> f64 {
        match self {
            QuantizationType::Float32 => 1.0,
            QuantizationType::Float16 => 0.5,
            QuantizationType::Int8 => 0.25,
            QuantizationType::Binary => 0.03125, // 1/32
        }
    }

    /// Returns a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            QuantizationType::Float32 => "float32",
            QuantizationType::Float16 => "float16",
            QuantizationType::Int8 => "int8",
            QuantizationType::Binary => "binary",
        }
    }
}

/// Distance metric for vector similarity search.
///
/// Defines how to measure similarity between vectors.
/// See Engineering Plan § 4.1.2: Distance Metrics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Cosine similarity (angle-based): 1 - (u·v)/(|u||v|)
    /// Range: [0, 2], lower is more similar
    Cosine,

    /// Euclidean distance: sqrt(sum((u_i - v_i)^2))
    /// Range: [0, inf), lower is more similar
    Euclidean,

    /// Dot product: u·v
    /// Range: (-inf, inf), higher is more similar (inverted)
    DotProduct,

    /// Manhattan distance (L1): sum(|u_i - v_i|)
    /// Range: [0, inf), lower is more similar
    Manhattan,
}

impl DistanceMetric {
    /// Returns a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            DistanceMetric::Cosine => "cosine",
            DistanceMetric::Euclidean => "euclidean",
            DistanceMetric::DotProduct => "dot_product",
            DistanceMetric::Manhattan => "manhattan",
        }
    }

    /// Returns whether higher distance values indicate more similarity.
    pub fn higher_is_better(&self) -> bool {
        matches!(self, DistanceMetric::DotProduct)
    }
}

/// Configuration for vector index creation.
///
/// See Engineering Plan § 4.1.2: Vector Index Configuration.
#[derive(Clone, Debug)]
pub struct IndexConfig {
    /// Vector dimensionality
    pub dimension: VectorDimension,

    /// Quantization scheme
    pub quantization: QuantizationType,

    /// Distance metric for similarity search
    pub distance_metric: DistanceMetric,

    /// Maximum number of entries the index can hold
    pub max_entries: u32,

    /// HNSW parameter: size of the candidate list during construction
    pub ef_construction: u32,

    /// HNSW parameter: number of bidirectional links each element has
    pub m_connections: u32,
}

impl IndexConfig {
    /// Creates a new index configuration with default HNSW parameters.
    ///
    /// # Arguments
    ///
    /// * `dimension` - Dimensionality of vectors
    /// * `distance_metric` - Metric to use for similarity
    /// * `max_entries` - Maximum capacity
    pub fn new(
        dimension: VectorDimension,
        distance_metric: DistanceMetric,
        max_entries: u32,
    ) -> Self {
        IndexConfig {
            dimension,
            quantization: QuantizationType::Float32,
            distance_metric,
            max_entries,
            ef_construction: 200,  // HNSW default
            m_connections: 16,     // HNSW default
        }
    }

    /// Sets the quantization type.
    pub fn with_quantization(mut self, quant: QuantizationType) -> Self {
        self.quantization = quant;
        self
    }

    /// Sets HNSW construction parameter.
    pub fn with_ef_construction(mut self, ef: u32) -> Self {
        self.ef_construction = ef;
        self
    }

    /// Sets HNSW connection parameter.
    pub fn with_m_connections(mut self, m: u32) -> Self {
        self.m_connections = m;
        self
    }
}

/// A vector entry in the index.
///
/// Represents a single stored vector with its metadata.
#[derive(Clone, Debug)]
pub struct VectorEntry {
    /// Unique identifier for this entry
    pub id: alloc::string::String,

    /// The vector data (fixed-size reference)
    pub vector: Vec<f32>,

    /// Associated metadata
    pub metadata: alloc::string::String,

    /// Timestamp when this entry was added
    pub timestamp: u64,
}

impl VectorEntry {
    /// Creates a new vector entry.
    pub fn new(
        id: impl Into<alloc::string::String>,
        vector: Vec<f32>,
        metadata: impl Into<alloc::string::String>,
        timestamp: u64,
    ) -> Self {
        VectorEntry {
            id: id.into(),
            vector,
            metadata: metadata.into(),
            timestamp,
        }
    }
}

/// Search result from vector index.
///
/// Contains the matched entry ID, distance score, and metadata.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// ID of the matched entry
    pub id: alloc::string::String,

    /// Distance score (metric-dependent interpretation)
    pub distance: f32,

    /// Associated metadata
    pub metadata: alloc::string::String,
}

impl SearchResult {
    /// Creates a new search result.
    pub fn new(
        id: impl Into<alloc::string::String>,
        distance: f32,
        metadata: impl Into<alloc::string::String>,
    ) -> Self {
        SearchResult {
            id: id.into(),
            distance,
            metadata: metadata.into(),
        }
    }
}

/// Vector index for L2 episodic memory.
///
/// Implements embedded vector indexing without external dependencies.
/// Currently provides a simple flat-search implementation; HNSW-like
/// hierarchical search can be added for production use.
///
/// See Engineering Plan § 4.1.2: Vector Indexing.
#[derive(Clone, Debug)]
pub struct VectorIndex {
    /// Index configuration
    config: IndexConfig,

    /// Stored vector entries
    entries: BTreeMap<alloc::string::String, VectorEntry>,

    /// Total bytes used by this index
    total_bytes: u64,
}

impl VectorIndex {
    /// Creates a new vector index with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Index configuration (dimension, metric, capacity)
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.2: Vector Index Initialization.
    pub fn new(config: IndexConfig) -> Self {
        VectorIndex {
            config,
            entries: BTreeMap::new(),
            total_bytes: 0,
        }
    }

    /// Returns the index configuration.
    pub fn config(&self) -> &IndexConfig {
        &self.config
    }

    /// Returns the number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the total bytes used by this index.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Returns the available capacity in the index.
    pub fn available_capacity(&self) -> u32 {
        self.config.max_entries.saturating_sub(self.entries.len() as u32)
    }

    /// Estimates the memory required for storing a vector.
    fn estimate_vector_size(&self) -> u64 {
        // Vector data + metadata overhead
        let quant_bytes = self.config.dimension as u64
            * self.config.quantization.bytes_per_element() as u64;
        let overhead = 64; // entry ID, metadata, timestamp
        quant_bytes + overhead
    }

    /// Inserts a vector entry into the index.
    ///
    /// # Arguments
    ///
    /// * `entry` - Vector entry to insert
    ///
    /// # Returns
    ///
    /// Ok if inserted, Err if index is full or vector dimension mismatch.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.2: Vector Insertion.
    pub fn insert(&mut self, entry: VectorEntry) -> Result<()> {
        // Check capacity
        if self.entries.len() >= self.config.max_entries as usize {
            return Err(MemoryError::RegionFull {
                region_id: "vector_index".to_string(),
                used: self.entries.len() as u64,
                capacity: self.config.max_entries as u64,
            });
        }

        // Validate vector dimension
        if entry.vector.len() as u16 != self.config.dimension {
            return Err(MemoryError::InvalidReference {
                reason: format!(
                    "vector dimension mismatch: got {}, expected {}",
                    entry.vector.len(),
                    self.config.dimension
                ),
            });
        }

        let entry_size = self.estimate_vector_size();
        let entry_id = entry.id.clone();

        self.entries.insert(entry_id, entry);
        self.total_bytes = self.total_bytes.saturating_add(entry_size);

        Ok(())
    }

    /// Removes a vector entry from the index.
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the entry to remove
    ///
    /// # Returns
    ///
    /// Ok if removed, Err if entry not found.
    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(_entry) = self.entries.remove(id) {
            let entry_size = self.estimate_vector_size();
            self.total_bytes = self.total_bytes.saturating_sub(entry_size);
            Ok(())
        } else {
            Err(MemoryError::InvalidReference {
                reason: format!("vector entry not found: {}", id),
            })
        }
    }

    /// Searches for vectors similar to the query vector.
    ///
    /// # Arguments
    ///
    /// * `query` - Query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vector of search results, sorted by distance (closest first).
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.2: Vector Search.
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        // Validate query dimension
        if query.len() as u16 != self.config.dimension {
            return Err(MemoryError::InvalidReference {
                reason: format!(
                    "query dimension mismatch: got {}, expected {}",
                    query.len(),
                    self.config.dimension
                ),
            });
        }

        if self.entries.is_empty() {
            return Ok(Vec::new());
        }

        // Compute distances to all entries
        let mut results = Vec::new();

        for (_id, entry) in &self.entries {
            let distance = self.compute_distance(&entry.vector, query);
            results.push((entry.id.clone(), distance, entry.metadata.clone()));
        }

        // Sort by distance
        if self.config.distance_metric.higher_is_better() {
            // For dot product, higher is better
            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        } else {
            // For other metrics, lower is better
            results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        }

        // Return top k
        let result_vec = results
            .into_iter()
            .take(k)
            .map(|(id, distance, metadata)| SearchResult::new(id, distance, metadata))
            .collect();

        Ok(result_vec)
    }

    /// Computes the distance between two vectors using the configured metric.
    fn compute_distance(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        match self.config.distance_metric {
            DistanceMetric::Cosine => self.cosine_distance(vec_a, vec_b),
            DistanceMetric::Euclidean => self.euclidean_distance(vec_a, vec_b),
            DistanceMetric::DotProduct => self.dot_product(vec_a, vec_b),
            DistanceMetric::Manhattan => self.manhattan_distance(vec_a, vec_b),
        }
    }

    /// Computes cosine distance: 1 - (u·v)/(|u||v|)
    fn cosine_distance(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        let dot = self.dot_product(vec_a, vec_b);
        let mag_a = self.magnitude(vec_a);
        let mag_b = self.magnitude(vec_b);

        if mag_a == 0.0 || mag_b == 0.0 {
            1.0
        } else {
            let cos_sim = dot / (mag_a * mag_b);
            (1.0 - cos_sim).max(0.0).min(2.0)
        }
    }

    /// Computes Euclidean distance: sqrt(sum((u_i - v_i)^2))
    fn euclidean_distance(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        let mut sum_sq = 0.0_f32;
        for (a, b) in vec_a.iter().zip(vec_b.iter()) {
            let diff = a - b;
            sum_sq += diff * diff;
        }
        sum_sq.sqrt()
    }

    /// Computes dot product: sum(u_i * v_i)
    fn dot_product(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        vec_a.iter()
            .zip(vec_b.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// Computes Manhattan distance: sum(|u_i - v_i|)
    fn manhattan_distance(&self, vec_a: &[f32], vec_b: &[f32]) -> f32 {
        vec_a.iter()
            .zip(vec_b.iter())
            .map(|(a, b)| (a - b).abs())
            .sum()
    }

    /// Returns the magnitude of a vector
    fn magnitude(&self, vec: &[f32]) -> f32 {
        vec.iter().map(|v| v * v).sum::<f32>().sqrt()
    }

    /// Clears all entries from the index.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }

    /// Returns whether this index contains an entry with the given ID.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Returns a reference to an entry by ID.
    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_quantization_type_bytes() {
        assert_eq!(QuantizationType::Float32.bytes_per_element(), 4);
        assert_eq!(QuantizationType::Float16.bytes_per_element(), 2);
        assert_eq!(QuantizationType::Int8.bytes_per_element(), 1);
        assert_eq!(QuantizationType::Binary.bytes_per_element(), 1);
    }

    #[test]
    fn test_quantization_type_compression() {
        assert_eq!(QuantizationType::Float32.compression_ratio(), 1.0);
        assert_eq!(QuantizationType::Float16.compression_ratio(), 0.5);
        assert_eq!(QuantizationType::Int8.compression_ratio(), 0.25);
    }

    #[test]
    fn test_distance_metric_names() {
        assert_eq!(DistanceMetric::Cosine.name(), "cosine");
        assert_eq!(DistanceMetric::Euclidean.name(), "euclidean");
        assert_eq!(DistanceMetric::DotProduct.name(), "dot_product");
        assert_eq!(DistanceMetric::Manhattan.name(), "manhattan");
    }

    #[test]
    fn test_distance_metric_higher_is_better() {
        assert!(!DistanceMetric::Cosine.higher_is_better());
        assert!(!DistanceMetric::Euclidean.higher_is_better());
        assert!(DistanceMetric::DotProduct.higher_is_better());
        assert!(!DistanceMetric::Manhattan.higher_is_better());
    }

    #[test]
    fn test_index_config_creation() {
        let config = IndexConfig::new(768, DistanceMetric::Cosine, 10000);
        assert_eq!(config.dimension, 768);
        assert_eq!(config.distance_metric, DistanceMetric::Cosine);
        assert_eq!(config.max_entries, 10000);
        assert_eq!(config.quantization, QuantizationType::Float32);
    }

    #[test]
    fn test_index_config_with_quantization() {
        let config = IndexConfig::new(768, DistanceMetric::Cosine, 10000)
            .with_quantization(QuantizationType::Int8);
        assert_eq!(config.quantization, QuantizationType::Int8);
    }

    #[test]
    fn test_vector_entry_creation() {
        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec.clone(), "metadata", 12345);
        assert_eq!(entry.id, "entry-001");
        assert_eq!(entry.vector, vec);
        assert_eq!(entry.metadata, "metadata");
        assert_eq!(entry.timestamp, 12345);
    }

    #[test]
    fn test_vector_index_creation() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let index = VectorIndex::new(config);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert_eq!(index.available_capacity(), 100);
    }

    #[test]
    fn test_vector_index_insert() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec, "test", 0);

        assert!(index.insert(entry).is_ok());
        assert_eq!(index.len(), 1);
        assert_eq!(index.available_capacity(), 99);
    }

    #[test]
    fn test_vector_index_insert_dimension_mismatch() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0]; // Wrong dimension
        let entry = VectorEntry::new("entry-001", vec, "test", 0);

        assert!(index.insert(entry).is_err());
    }

    #[test]
    fn test_vector_index_insert_full() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 1);
        let mut index = VectorIndex::new(config);

        let vec1 = alloc::vec![1.0, 2.0, 3.0];
        let entry1 = VectorEntry::new("entry-001", vec1, "test1", 0);
        assert!(index.insert(entry1).is_ok());

        let vec2 = alloc::vec![4.0, 5.0, 6.0];
        let entry2 = VectorEntry::new("entry-002", vec2, "test2", 0);
        assert!(index.insert(entry2).is_err());
    }

    #[test]
    fn test_vector_index_remove() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec, "test", 0);
        index.insert(entry).unwrap();

        assert!(index.remove("entry-001").is_ok());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_vector_index_remove_nonexistent() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);
        assert!(index.remove("nonexistent").is_err());
    }

    #[test]
    fn test_vector_index_contains() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec, "test", 0);
        index.insert(entry).unwrap();

        assert!(index.contains("entry-001"));
        assert!(!index.contains("nonexistent"));
    }

    #[test]
    fn test_vector_index_get() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec.clone(), "test", 0);
        index.insert(entry).unwrap();

        assert!(index.get("entry-001").is_some());
        assert!(index.get("nonexistent").is_none());
    }

    #[test]
    fn test_vector_index_clear() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        let vec = alloc::vec![1.0, 2.0, 3.0];
        let entry = VectorEntry::new("entry-001", vec, "test", 0);
        index.insert(entry).unwrap();

        index.clear();
        assert_eq!(index.len(), 0);
        assert_eq!(index.total_bytes(), 0);
    }

    #[test]
    fn test_vector_search_euclidean() {
        let config = IndexConfig::new(2, DistanceMetric::Euclidean, 100);
        let mut index = VectorIndex::new(config);

        // Insert some vectors
        let vec1 = alloc::vec![0.0, 0.0];
        let entry1 = VectorEntry::new("origin", vec1, "at origin", 0);
        index.insert(entry1).unwrap();

        let vec2 = alloc::vec![3.0, 4.0]; // distance 5 from origin
        let entry2 = VectorEntry::new("far", vec2, "far away", 0);
        index.insert(entry2).unwrap();

        let vec3 = alloc::vec![1.0, 1.0]; // distance sqrt(2) from origin
        let entry3 = VectorEntry::new("close", vec3, "nearby", 0);
        index.insert(entry3).unwrap();

        // Search for vectors near origin
        let query = alloc::vec![0.0, 0.0];
        let results = index.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "origin"); // Closest
        assert_eq!(results[1].id, "close"); // Second closest
    }

    #[test]
    fn test_vector_search_cosine() {
        let config = IndexConfig::new(2, DistanceMetric::Cosine, 100);
        let mut index = VectorIndex::new(config);

        // Insert vectors
        let vec1 = alloc::vec![1.0, 0.0];
        let entry1 = VectorEntry::new("right", vec1, "pointing right", 0);
        index.insert(entry1).unwrap();

        let vec2 = alloc::vec![0.0, 1.0]; // Orthogonal to vec1
        let entry2 = VectorEntry::new("up", vec2, "pointing up", 0);
        index.insert(entry2).unwrap();

        let query = alloc::vec![1.0, 0.0]; // Same as vec1
        let results = index.search(&query, 1).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "right"); // Identical
    }

    #[test]
    fn test_vector_search_dimension_mismatch() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let index = VectorIndex::new(config);

        let query = alloc::vec![1.0, 2.0]; // Wrong dimension
        let result = index.search(&query, 1);

        assert!(result.is_err());
    }

    #[test]
    fn test_vector_search_empty_index() {
        let config = IndexConfig::new(3, DistanceMetric::Euclidean, 100);
        let index = VectorIndex::new(config);

        let query = alloc::vec![1.0, 2.0, 3.0];
        let results = index.search(&query, 10).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_dot_product_distance() {
        let config = IndexConfig::new(2, DistanceMetric::DotProduct, 100);
        let mut index = VectorIndex::new(config);

        let vec1 = alloc::vec![1.0, 0.0];
        let entry1 = VectorEntry::new("v1", vec1, "test", 0);
        index.insert(entry1).unwrap();

        let vec2 = alloc::vec![0.0, 1.0];
        let entry2 = VectorEntry::new("v2", vec2, "test", 0);
        index.insert(entry2).unwrap();

        let query = alloc::vec![1.0, 1.0];
        let results = index.search(&query, 2).unwrap();

        // Both have dot product 1.0, but we should get consistent ordering
        assert_eq!(results.len(), 2);
    }
}
