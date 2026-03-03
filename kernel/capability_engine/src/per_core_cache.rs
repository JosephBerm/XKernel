// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Thread-local L1 cache for hot-path capability checks.
//!
//! Per-core cache with 256 entries per core, validation epoch on revocation.
//! Passive invalidation via memory barrier (no IPI required).
//! Target >95% cache hit rate in steady state.
//! See Engineering Plan § 3.2.0 and Week 6 § 4.

#![forbid(unsafe_code)]

use alloc::vec::Vec;
use core::fmt::{self, Debug};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::{AgentID, CapID};
use crate::operations::OperationSet;

/// Maximum number of entries in per-core L1 cache.
const L1_CACHE_SIZE: usize = 256;

/// Cache entry key: (agent_id, capid_hash, operation_bits).
/// Used for fast lookup without full CapID comparison.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CacheKey {
    /// Hash of agent ID (quick filter)
    pub agent_hash: u64,

    /// Hash of capability ID (quick filter)
    pub cap_id_hash: u64,

    /// Operation bits being requested
    pub operation_bits: u8,
}

impl CacheKey {
    /// Creates a new cache key.
    pub fn new(agent_hash: u64, cap_id_hash: u64, operation_bits: u8) -> Self {
        CacheKey {
            agent_hash,
            cap_id_hash,
            operation_bits,
        }
    }

    /// Computes a simple hash for the cache key.
    pub fn hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        h ^= self.agent_hash;
        h = h.wrapping_mul(0x100000001b3u64);
        h ^= self.cap_id_hash;
        h = h.wrapping_mul(0x100000001b3u64);
        h ^= self.operation_bits as u64;
        h = h.wrapping_mul(0x100000001b3u64);
        h
    }
}

/// A single L1 cache entry.
/// Stores cached capability check result with validation epoch.
#[derive(Clone, Debug)]
pub struct L1CacheEntry {
    /// Cache key for this entry
    pub key: CacheKey,

    /// Cached capability (or None if negative cache)
    pub capability: Option<Capability>,

    /// Epoch version when this entry was cached
    pub epoch: u64,

    /// Number of times this entry was hit
    pub hit_count: u64,

    /// When this entry was last accessed (nanoseconds)
    pub last_accessed_ns: u64,
}

impl L1CacheEntry {
    /// Creates a new cache entry.
    pub fn new(key: CacheKey, capability: Option<Capability>, epoch: u64, now_ns: u64) -> Self {
        L1CacheEntry {
            key,
            capability,
            epoch,
            hit_count: 0,
            last_accessed_ns: now_ns,
        }
    }

    /// Checks if this entry is still valid (epoch matches).
    pub fn is_valid(&self, current_epoch: u64) -> bool {
        self.epoch == current_epoch
    }

    /// Records a hit and updates last access time.
    pub fn record_hit(&mut self, now_ns: u64) {
        self.hit_count = self.hit_count.saturating_add(1);
        self.last_accessed_ns = now_ns;
    }
}

/// Per-core L1 cache for capability check results.
///
/// - 256 entries per core
/// - Key: (agent_id_hash, capid_hash, operation_bits)
/// - Validation epoch incremented on each Revoke
/// - Passive invalidation via memory barrier
/// - Target >95% cache hit rate
///
/// See Week 6 § 4.
pub struct PerCoreCache {
    /// Cache entries (fixed size vector)
    entries: Vec<L1CacheEntry>,

    /// Current validation epoch (incremented on revocation)
    epoch: AtomicU64,

    /// Cache statistics
    hits: AtomicU64,
    misses: AtomicU64,

    /// Current time for cache decisions (mock, typically real time)
    current_time_ns: u64,
}

impl PerCoreCache {
    /// Creates a new per-core cache with L1_CACHE_SIZE capacity.
    pub fn new() -> Result<Self, CapError> {
        let mut entries = Vec::new();
        entries.reserve(L1_CACHE_SIZE);

        Ok(PerCoreCache {
            entries,
            epoch: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            current_time_ns: 0,
        })
    }

    /// Sets the current time for cache operations (mainly for testing).
    pub fn set_current_time(&mut self, time_ns: u64) {
        self.current_time_ns = time_ns;
    }

    /// Looks up a cache entry by key (may return invalidated entries).
    /// Returns Ok(Some(capability)) if hit and still valid
    /// Returns Ok(None) if hit but invalidated
    /// Returns Err(CapError) if miss (not in cache)
    pub fn lookup(&mut self, key: &CacheKey) -> Result<Option<Capability>, CapError> {
        let current_epoch = self.epoch.load(Ordering::Acquire);

        // Linear search for now (could be optimized to hash table)
        for entry in &mut self.entries {
            if entry.key == *key {
                // Found entry in cache
                if entry.is_valid(current_epoch) {
                    // Valid entry: record hit and return
                    entry.record_hit(self.current_time_ns);
                    self.hits.fetch_add(1, Ordering::Release);
                    return Ok(entry.capability.clone());
                } else {
                    // Invalidated entry: remove from cache
                    entry.epoch = u64::MAX; // Mark as invalid
                    self.misses.fetch_add(1, Ordering::Release);
                    return Err(CapError::Other("cache entry invalidated".into()));
                }
            }
        }

        // Not found in cache
        self.misses.fetch_add(1, Ordering::Release);
        Err(CapError::Other("cache miss".into()))
    }

    /// Inserts a cache entry (may evict LRU entry if at capacity).
    pub fn insert(&mut self, key: CacheKey, capability: Option<Capability>) -> Result<(), CapError> {
        let epoch = self.epoch.load(Ordering::Acquire);
        let entry = L1CacheEntry::new(key, capability, epoch, self.current_time_ns);

        if self.entries.len() < L1_CACHE_SIZE {
            // Cache not full, just append
            self.entries.push(entry);
        } else {
            // Cache full: find and evict LRU entry
            let mut lru_idx = 0;
            let mut lru_time = self.entries[0].last_accessed_ns;

            for (i, e) in self.entries.iter().enumerate() {
                if e.last_accessed_ns < lru_time {
                    lru_idx = i;
                    lru_time = e.last_accessed_ns;
                }
            }

            self.entries[lru_idx] = entry;
        }

        Ok(())
    }

    /// Invalidates all cache entries (called on capability revocation).
    /// Increments the epoch to mark all entries as invalid.
    pub fn invalidate_all(&self) {
        // Increment epoch to invalidate all entries
        let old_epoch = self.epoch.load(Ordering::Acquire);
        self.epoch.store(old_epoch.wrapping_add(1), Ordering::Release);

        // Implicit memory barrier: Release ordering ensures all cores see update
    }

    /// Clears all cache entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits.store(0, Ordering::Release);
        self.misses.store(0, Ordering::Release);
    }

    /// Returns cache hit count.
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Acquire)
    }

    /// Returns cache miss count.
    pub fn miss_count(&self) -> u64 {
        self.misses.load(Ordering::Acquire)
    }

    /// Returns total cache lookups.
    pub fn total_lookups(&self) -> u64 {
        self.hit_count() + self.miss_count()
    }

    /// Returns cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_lookups();
        if total == 0 {
            return 0.0;
        }
        self.hit_count() as f64 / total as f64
    }

    /// Returns current cache entry count.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns cache capacity.
    pub fn capacity(&self) -> usize {
        L1_CACHE_SIZE
    }

    /// Returns current validation epoch.
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }
}

impl Default for PerCoreCache {
    fn default() -> Self {
        Self::new().expect("cache creation")
    }
}

impl Debug for PerCoreCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerCoreCache")
            .field("capacity", &self.capacity())
            .field("entry_count", &self.entry_count())
            .field("hit_count", &self.hit_count())
            .field("miss_count", &self.miss_count())
            .field("hit_rate", &self.hit_rate())
            .field("current_epoch", &self.current_epoch())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Timestamp;
    use crate::ids::{ResourceID, ResourceType, AgentID, CapID};
use alloc::format;

    #[test]
    fn test_cache_key_creation() {
        let key = CacheKey::new(123, 456, OperationSet::READ);
        assert_eq!(key.agent_hash, 123);
        assert_eq!(key.cap_id_hash, 456);
        assert_eq!(key.operation_bits, OperationSet::READ);
    }

    #[test]
    fn test_cache_key_hash() {
        let key1 = CacheKey::new(123, 456, OperationSet::READ);
        let key2 = CacheKey::new(123, 456, OperationSet::READ);
        assert_eq!(key1.hash(), key2.hash());

        let key3 = CacheKey::new(123, 457, OperationSet::READ);
        assert_ne!(key1.hash(), key3.hash());
    }

    #[test]
    fn test_cache_creation() {
        let cache = PerCoreCache::new().expect("cache creation");
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.capacity(), 256);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_cache_insert_and_lookup() {
        let mut cache = PerCoreCache::new().expect("cache creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let key = CacheKey::new(100, 200, OperationSet::READ);
        cache.insert(key.clone(), Some(cap.clone())).expect("insert");

        let result = cache.lookup(&key).expect("lookup success");
        assert!(result.is_some());
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = PerCoreCache::new().expect("cache creation");
        let key = CacheKey::new(100, 200, OperationSet::READ);

        let result = cache.lookup(&key);
        assert!(result.is_err());
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = PerCoreCache::new().expect("cache creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let key = CacheKey::new(100, 200, OperationSet::READ);
        cache.insert(key.clone(), Some(cap)).expect("insert");

        // One hit
        let _ = cache.lookup(&key);
        assert_eq!(cache.hit_rate(), 1.0);

        // One miss
        let bad_key = CacheKey::new(999, 999, OperationSet::WRITE);
        let _ = cache.lookup(&bad_key);
        assert!(cache.hit_rate() > 0.4 && cache.hit_rate() < 0.6);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = PerCoreCache::new().expect("cache creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let key = CacheKey::new(100, 200, OperationSet::READ);
        cache.insert(key.clone(), Some(cap)).expect("insert");

        assert!(cache.lookup(&key).is_ok());
        let epoch_before = cache.current_epoch();

        cache.invalidate_all();
        let epoch_after = cache.current_epoch();

        assert_ne!(epoch_before, epoch_after);
        assert!(cache.lookup(&key).is_err());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = PerCoreCache::new().expect("cache creation");

        // Fill cache to capacity
        for i in 0..256 {
            let key = CacheKey::new(i as u64, i as u64, OperationSet::READ);
            let cap = Capability::new(
                CapID::from_bytes([i as u8; 32]),
                AgentID::new(&format!("agent-{}", i)),
                ResourceType::file(),
                ResourceID::new(&format!("file-{}", i)),
                OperationSet::read(),
                Timestamp::new(1000),
            );
            cache.insert(key, Some(cap)).expect("insert");
        }

        assert_eq!(cache.entry_count(), 256);

        // Insert one more (should evict LRU)
        let key = CacheKey::new(999, 999, OperationSet::READ);
        let cap = Capability::new(
            CapID::from_bytes([99u8; 32]),
            AgentID::new("agent-new"),
            ResourceType::file(),
            ResourceID::new("file-new"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cache.insert(key, Some(cap)).expect("insert");

        assert_eq!(cache.entry_count(), 256); // Still at capacity
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = PerCoreCache::new().expect("cache creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let key = CacheKey::new(100, 200, OperationSet::READ);
        cache.insert(key.clone(), Some(cap)).expect("insert");

        assert!(cache.entry_count() > 0);
        cache.clear();
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_cache_negative_caching() {
        let mut cache = PerCoreCache::new().expect("cache creation");

        // Cache a "not found" result (None capability)
        let key = CacheKey::new(100, 200, OperationSet::READ);
        cache.insert(key.clone(), None).expect("insert");

        let result = cache.lookup(&key).expect("lookup success");
        assert!(result.is_none());
        assert_eq!(cache.hit_count(), 1);
    }
}
