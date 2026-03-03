// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Eviction policies for managing memory across tiers (LRU, Spill-First).

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::fmt;

/// Eviction policy trait for memory tier management
pub trait EvictionPolicy: Send + Sync {
    /// Record an access to an entry (for LRU tracking)
    fn record_access(&mut self, entry_id: u64);

    /// Mark an entry for eviction
    fn mark_for_eviction(&mut self, entry_id: u64);

    /// Get the next candidate for eviction
    fn next_evict_candidate(&mut self) -> Option<u64>;

    /// Reset state (e.g., at tier boundary)
    fn reset(&mut self);
}

/// LRU (Least Recently Used) eviction policy
#[derive(Debug, Clone)]
pub struct LruEvictionPolicy {
    /// Queue of entry IDs in LRU order (oldest first)
    access_queue: VecDeque<u64>,
    max_size: usize,
}

impl LruEvictionPolicy {
    pub fn new(max_size: usize) -> Self {
        Self {
            access_queue: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn size(&self) -> usize {
        self.access_queue.len()
    }

    pub fn is_full(&self) -> bool {
        self.access_queue.len() >= self.max_size
    }
}

impl Default for LruEvictionPolicy {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl EvictionPolicy for LruEvictionPolicy {
    fn record_access(&mut self, entry_id: u64) {
        // Remove if already present, then add to end (most recent)
        self.access_queue.retain(|&id| id != entry_id);
        self.access_queue.push_back(entry_id);

        // Trim if over capacity
        while self.access_queue.len() > self.max_size {
            self.access_queue.pop_front();
        }
    }

    fn mark_for_eviction(&mut self, entry_id: u64) {
        self.access_queue.retain(|&id| id != entry_id);
    }

    fn next_evict_candidate(&mut self) -> Option<u64> {
        self.access_queue.pop_front()
    }

    fn reset(&mut self) {
        self.access_queue.clear();
    }
}

/// Spill-First eviction: prioritize moving data to lower tiers rather than dropping
#[derive(Debug, Clone)]
pub struct SpillFirstEvictionPolicy {
    /// Entries spilled to lower tier (ready for eviction)
    spilled_entries: Vec<u64>,
    /// Hot entries (recently accessed)
    hot_entries: VecDeque<u64>,
    max_hot_size: usize,
}

impl SpillFirstEvictionPolicy {
    pub fn new(max_hot_size: usize) -> Self {
        Self {
            spilled_entries: Vec::new(),
            hot_entries: VecDeque::with_capacity(max_hot_size),
            max_hot_size,
        }
    }

    pub fn spilled_count(&self) -> usize {
        self.spilled_entries.len()
    }

    pub fn hot_count(&self) -> usize {
        self.hot_entries.len()
    }

    /// Mark entry as spilled to lower tier
    pub fn mark_spilled(&mut self, entry_id: u64) {
        if !self.spilled_entries.contains(&entry_id) {
            self.spilled_entries.push(entry_id);
        }
        // Remove from hot if present
        self.hot_entries.retain(|&id| id != entry_id);
    }

    /// Get a spilled entry for deletion
    pub fn take_spilled(&mut self) -> Option<u64> {
        if self.spilled_entries.is_empty() {
            None
        } else {
            Some(self.spilled_entries.remove(0))
        }
    }
}

impl Default for SpillFirstEvictionPolicy {
    fn default() -> Self {
        Self::new(500)
    }
}

impl EvictionPolicy for SpillFirstEvictionPolicy {
    fn record_access(&mut self, entry_id: u64) {
        // Move from spilled to hot on access
        self.spilled_entries.retain(|&id| id != entry_id);
        self.hot_entries.retain(|&id| id != entry_id);
        self.hot_entries.push_back(entry_id);

        // Keep hot entries trimmed
        while self.hot_entries.len() > self.max_hot_size {
            let evicted = self.hot_entries.pop_front().unwrap();
            self.spilled_entries.push(evicted);
        }
    }

    fn mark_for_eviction(&mut self, entry_id: u64) {
        self.spilled_entries.retain(|&id| id != entry_id);
        self.hot_entries.retain(|&id| id != entry_id);
    }

    fn next_evict_candidate(&mut self) -> Option<u64> {
        self.take_spilled()
    }

    fn reset(&mut self) {
        self.spilled_entries.clear();
        self.hot_entries.clear();
    }
}

/// Eviction decision result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionAction {
    /// Keep in current tier
    Keep,
    /// Move to lower tier (spill)
    Spill,
    /// Delete from all tiers
    Delete,
}

impl fmt::Display for EvictionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keep => write!(f, "Keep"),
            Self::Spill => write!(f, "Spill"),
            Self::Delete => write!(f, "Delete"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_eviction() {
        let mut lru = LruEvictionPolicy::new(3);

        lru.record_access(1);
        lru.record_access(2);
        lru.record_access(3);
        assert_eq!(lru.size(), 3);

        // Oldest is 1
        assert_eq!(lru.next_evict_candidate(), Some(1));
        assert_eq!(lru.next_evict_candidate(), Some(2));
        assert_eq!(lru.next_evict_candidate(), Some(3));
        assert_eq!(lru.next_evict_candidate(), None);
    }

    #[test]
    fn test_lru_reaccess() {
        let mut lru = LruEvictionPolicy::new(3);
        lru.record_access(1);
        lru.record_access(2);
        lru.record_access(3);

        // Re-access 1 makes it newest
        lru.record_access(1);

        assert_eq!(lru.next_evict_candidate(), Some(2));
        assert_eq!(lru.next_evict_candidate(), Some(3));
        assert_eq!(lru.next_evict_candidate(), Some(1));
    }

    #[test]
    fn test_spill_first() {
        let mut policy = SpillFirstEvictionPolicy::new(2);

        policy.record_access(1);
        policy.record_access(2);
        assert_eq!(policy.hot_count(), 2);

        policy.record_access(3); // Causes 1 to spill
        assert_eq!(policy.hot_count(), 2);
        assert_eq!(policy.spilled_count(), 1);

        assert_eq!(policy.take_spilled(), Some(1));
        assert_eq!(policy.spilled_count(), 0);
    }

    #[test]
    fn test_spill_reaccess() {
        let mut policy = SpillFirstEvictionPolicy::new(2);
        policy.record_access(1);
        policy.record_access(2);
        policy.record_access(3); // 1 spilled

        // Re-accessing spilled brings it back to hot
        policy.record_access(1);
        assert_eq!(policy.hot_count(), 2);
        assert_eq!(policy.spilled_count(), 0);
    }
}
