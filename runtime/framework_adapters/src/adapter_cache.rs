// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Adapter-Side Caching and Buffering
//!
//! Provides translation result caching to avoid redundant translations and improve performance.
//! Caches include: translated DAGs, tool bindings, memory mappings, and framework-specific metadata.
//!
//! Implements TTL-based eviction for automatic cache cleanup and hit/miss tracking for analytics.
//!
//! Sec 4.2: Adapter Cache Architecture
//! Sec 5.1: Performance Optimization through Caching

use std::collections::BTreeMap;
use crate::AdapterError;

/// Cached translation entry combining original and translated artifacts.
/// Sec 4.2: Cached Translation Structure
#[derive(Debug, Clone)]
pub struct CachedTranslation {
    /// Original chain definition
    pub original_chain: String,
    /// Translated CT DAG
    pub translated_dag: String,
    /// Creation timestamp in milliseconds
    pub created_at: u64,
    /// Cache entry hit count
    pub hit_count: u64,
}

impl CachedTranslation {
    /// Creates a new cached translation entry.
    pub fn new(original_chain: String, translated_dag: String, created_at: u64) -> Self {
        CachedTranslation {
            original_chain,
            translated_dag,
            created_at,
            hit_count: 0,
        }
    }

    /// Increments the hit counter.
    pub fn record_hit(&mut self) {
        self.hit_count = self.hit_count.saturating_add(1);
    }

    /// Returns true if the cache entry has expired given current time and TTL.
    pub fn is_expired(&self, current_time_ms: u64, ttl_ms: u64) -> bool {
        current_time_ms.saturating_sub(self.created_at) > ttl_ms
    }
}

/// Cache statistics tracking performance metrics.
/// Sec 5.1: Cache Statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total number of entries currently cached
    pub total_entries: u64,
    /// Number of entries evicted due to TTL
    pub evictions: u64,
}

impl CacheStats {
    /// Creates new cache statistics.
    pub fn new() -> Self {
        CacheStats {
            hits: 0,
            misses: 0,
            total_entries: 0,
            evictions: 0,
        }
    }

    /// Records a cache hit.
    pub fn record_hit(&mut self) {
        self.hits = self.hits.saturating_add(1);
    }

    /// Records a cache miss.
    pub fn record_miss(&mut self) {
        self.misses = self.misses.saturating_add(1);
    }

    /// Records entry eviction.
    pub fn record_eviction(&mut self) {
        self.evictions = self.evictions.saturating_add(1);
    }

    /// Returns the hit rate as a percentage (0-100).
    pub fn hit_rate_pct(&self) -> u8 {
        let total = self.hits.saturating_add(self.misses);
        if total == 0 {
            return 0;
        }
        ((self.hits * 100) / total) as u8
    }
}

/// Adapter-side cache for translation results and metadata.
/// Sec 4.2: Adapter Cache Structure
#[derive(Debug, Clone)]
pub struct AdapterCache {
    /// Mapping of chain hash to cached translations
    pub cached_translations: BTreeMap<String, CachedTranslation>,
    /// Cached tool bindings (tool_id -> binding_config)
    pub cached_tool_bindings: BTreeMap<String, String>,
    /// Time-to-live for cache entries in milliseconds
    pub ttl_ms: u64,
    /// Cache statistics
    pub stats: CacheStats,
}

impl AdapterCache {
    /// Creates a new adapter cache with specified TTL.
    /// Sec 4.2: Cache Creation
    pub fn new(ttl_ms: u64) -> Self {
        AdapterCache {
            cached_translations: BTreeMap::new(),
            cached_tool_bindings: BTreeMap::new(),
            ttl_ms,
            stats: CacheStats::new(),
        }
    }

    /// Looks up a cached translation by chain hash.
    /// Sec 4.2: Cache Lookup
    pub fn cache_lookup(&self, chain_hash: &str) -> Option<CachedTranslation> {
        self.cached_translations.get(chain_hash).cloned()
    }

    /// Inserts or updates a translation in the cache.
    /// Sec 4.2: Cache Insertion
    pub fn cache_insert(
        &mut self,
        chain_hash: String,
        dag: String,
        current_time_ms: u64,
    ) -> Result<(), AdapterError> {
        let original_chain = chain_hash.clone();
        let translation = CachedTranslation::new(original_chain, dag, current_time_ms);
        self.cached_translations.insert(chain_hash, translation);
        self.stats.total_entries = self.cached_translations.len() as u64;
        Ok(())
    }

    /// Evicts expired entries from the cache.
    /// Sec 4.2: Cache Eviction
    pub fn evict_expired(&mut self, current_time_ms: u64) -> usize {
        let ttl_ms = self.ttl_ms;
        let initial_count = self.cached_translations.len();

        // Collect keys to remove to avoid borrow checker issues
        let expired_keys: Vec<String> = self
            .cached_translations
            .iter()
            .filter(|(_, entry)| entry.is_expired(current_time_ms, ttl_ms))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cached_translations.remove(&key);
            self.stats.record_eviction();
        }

        let evicted_count = initial_count - self.cached_translations.len();
        self.stats.total_entries = self.cached_translations.len() as u64;
        evicted_count
    }

    /// Clears the entire cache.
    pub fn clear(&mut self) {
        self.cached_translations.clear();
        self.cached_tool_bindings.clear();
        self.stats = CacheStats::new();
    }

    /// Returns current cache statistics.
    pub fn get_stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Caches a tool binding configuration.
    pub fn cache_tool_binding(&mut self, tool_id: String, binding_config: String) {
        self.cached_tool_bindings.insert(tool_id, binding_config);
    }

    /// Looks up a cached tool binding.
    pub fn lookup_tool_binding(&self, tool_id: &str) -> Option<String> {
        self.cached_tool_bindings.get(tool_id).cloned()
    }

    /// Returns the number of cached translations.
    pub fn translation_count(&self) -> usize {
        self.cached_translations.len()
    }

    /// Returns the number of cached tool bindings.
    pub fn tool_binding_count(&self) -> usize {
        self.cached_tool_bindings.len()
    }
}

/// Generates a hash for a chain definition (simple content hash).
/// Sec 4.2: Cache Key Generation
pub fn hash_chain(chain_def: &str) -> String {
    // Simple hash: count characters and first/last bytes
    let len = chain_def.len();
    let first_byte = chain_def.bytes().next().unwrap_or(0);
    let last_byte = chain_def.bytes().last().unwrap_or(0);

    format!("chain_{}_{}_{}_{}", len, first_byte, last_byte, len % 256)
}

#[cfg(test)]
mod tests {
    use super::*;
use std::collections::BTreeMap;

    #[test]
    fn test_cached_translation_creation() {
        let cached = CachedTranslation::new(
            "chain_def".into(),
            "dag_output".into(),
            1000,
        );
        assert_eq!(cached.original_chain, "chain_def");
        assert_eq!(cached.translated_dag, "dag_output");
        assert_eq!(cached.hit_count, 0);
    }

    #[test]
    fn test_cached_translation_record_hit() {
        let mut cached = CachedTranslation::new(
            "chain_def".into(),
            "dag_output".into(),
            1000,
        );
        cached.record_hit();
        cached.record_hit();
        cached.record_hit();

        assert_eq!(cached.hit_count, 3);
    }

    #[test]
    fn test_cached_translation_is_expired() {
        let cached = CachedTranslation::new(
            "chain_def".into(),
            "dag_output".into(),
            1000,
        );

        let ttl_ms = 5000;
        assert!(!cached.is_expired(1000, ttl_ms)); // Same time
        assert!(!cached.is_expired(5000, ttl_ms)); // Within TTL
        assert!(cached.is_expired(6001, ttl_ms)); // Expired
    }

    #[test]
    fn test_cache_stats_creation() {
        let stats = CacheStats::new();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.evictions, 0);
    }

    #[test]
    fn test_cache_stats_recording() {
        let mut stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        stats.record_eviction();

        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.hit_rate_pct(), 66);
    }

    #[test]
    fn test_cache_stats_hit_rate_empty() {
        let stats = CacheStats::new();
        assert_eq!(stats.hit_rate_pct(), 0);
    }

    #[test]
    fn test_cache_stats_hit_rate_all_hits() {
        let mut stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_hit();

        assert_eq!(stats.hit_rate_pct(), 100);
    }

    #[test]
    fn test_adapter_cache_creation() {
        let cache = AdapterCache::new(5000);
        assert_eq!(cache.ttl_ms, 5000);
        assert!(cache.cached_translations.is_empty());
        assert!(cache.cached_tool_bindings.is_empty());
    }

    #[test]
    fn test_adapter_cache_insert_and_lookup() {
        let mut cache = AdapterCache::new(5000);
        let hash = "chain_hash_123".to_string();
        let dag = "translated_dag".to_string();

        cache.cache_insert(hash.clone(), dag.clone(), 1000).unwrap();

        let result = cache.cache_lookup(&hash);
        assert!(result.is_some());
        assert_eq!(result.unwrap().translated_dag, dag);
    }

    #[test]
    fn test_adapter_cache_lookup_miss() {
        let cache = AdapterCache::new(5000);
        let result = cache.cache_lookup("nonexistent_hash");
        assert!(result.is_none());
    }

    #[test]
    fn test_adapter_cache_evict_expired() {
        let mut cache = AdapterCache::new(5000);

        // Insert three entries at different times
        cache.cache_insert("hash1".into(), "dag1".into(), 1000).unwrap();
        cache.cache_insert("hash2".into(), "dag2".into(), 2000).unwrap();
        cache.cache_insert("hash3".into(), "dag3".into(), 3000).unwrap();

        assert_eq!(cache.translation_count(), 3);

        // Evict entries expired before time 7500 (5000ms TTL)
        let evicted = cache.evict_expired(7500);
        assert_eq!(evicted, 2); // hash1 and hash2 should be evicted
        assert_eq!(cache.translation_count(), 1);
    }

    #[test]
    fn test_adapter_cache_clear() {
        let mut cache = AdapterCache::new(5000);
        cache.cache_insert("hash1".into(), "dag1".into(), 1000).unwrap();
        cache.cache_tool_binding("tool1".into(), "binding1".into());

        assert!(!cache.cached_translations.is_empty());
        assert!(!cache.cached_tool_bindings.is_empty());

        cache.clear();

        assert!(cache.cached_translations.is_empty());
        assert!(cache.cached_tool_bindings.is_empty());
    }

    #[test]
    fn test_adapter_cache_tool_binding_management() {
        let mut cache = AdapterCache::new(5000);
        cache.cache_tool_binding("tool1".into(), "binding_config1".into());
        cache.cache_tool_binding("tool2".into(), "binding_config2".into());

        assert_eq!(cache.tool_binding_count(), 2);

        let result = cache.lookup_tool_binding("tool1");
        assert_eq!(result, Some("binding_config1".into()));

        let result = cache.lookup_tool_binding("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_adapter_cache_stats() {
        let mut cache = AdapterCache::new(5000);
        cache.cache_insert("hash1".into(), "dag1".into(), 1000).unwrap();
        cache.stats.record_hit();
        cache.stats.record_hit();
        cache.stats.record_miss();

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_entries, 1);
    }

    #[test]
    fn test_hash_chain_consistency() {
        let chain1 = "my_chain_definition";
        let hash1 = hash_chain(chain1);
        let hash2 = hash_chain(chain1);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_chain_different_inputs() {
        let chain1 = "chain_one";
        let chain2 = "chain_two";

        let hash1 = hash_chain(chain1);
        let hash2 = hash_chain(chain2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cached_translation_hit_count_saturation() {
        let mut cached = CachedTranslation::new(
            "chain".into(),
            "dag".into(),
            1000,
        );

        // Record many hits to test saturation behavior
        for _ in 0..1000 {
            cached.record_hit();
        }

        assert_eq!(cached.hit_count, 1000);
    }

    #[test]
    fn test_adapter_cache_multiple_operations() {
        let mut cache = AdapterCache::new(10000);

        // Insert entries
        for i in 0..5 {
            let hash = format!("hash_{}", i);
            let dag = format!("dag_{}", i);
            cache.cache_insert(hash, dag, 1000 + (i as u64) * 1000).unwrap();
        }

        assert_eq!(cache.translation_count(), 5);

        // Evict some entries
        let evicted = cache.evict_expired(6000);
        assert!(evicted > 0);
        assert!(cache.translation_count() < 5);

        // Clear remaining
        cache.clear();
        assert_eq!(cache.translation_count(), 0);
    }

    #[test]
    fn test_adapter_cache_stats_tracking() {
        let mut cache = AdapterCache::new(5000);
        cache.cache_insert("h1".into(), "d1".into(), 1000).unwrap();
        cache.cache_insert("h2".into(), "d2".into(), 2000).unwrap();

        cache.stats.record_hit();
        cache.stats.record_hit();
        cache.stats.record_miss();
        cache.evict_expired(7500);

        let stats = cache.get_stats();
        assert!(stats.evictions > 0);
        assert_eq!(stats.hit_rate_pct(), 66);
    }
}
