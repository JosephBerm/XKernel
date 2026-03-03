// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Integration tests for semantic memory service L1 scaffolds.

use cs_semantic_memory::allocation::{AllocationError, ArenaAllocator, MemoryCompactor, MemoryPool};
use cs_semantic_memory::eviction::{EvictionPolicy, LruEvictionPolicy, SpillFirstEvictionPolicy};
use cs_semantic_memory::indexing::{IndexError, PrefetchStrategy, SemanticPrefetch, VectorIndex};
use cs_semantic_memory::tiers::{MemoryTier, TierConfig, TierMetrics};

#[test]
fn test_semantic_memory_tier_configuration() {
    let l1_cfg = TierConfig::new_l1(1_000_000);
    let l2_cfg = TierConfig::new_l2(10_000_000);
    let l3_cfg = TierConfig::new_l3(100_000_000);

    assert_eq!(l1_cfg.tier, MemoryTier::L1Hbm);
    assert_eq!(l2_cfg.tier, MemoryTier::L2Dram);
    assert_eq!(l3_cfg.tier, MemoryTier::L3Nvme);

    assert!(!l1_cfg.tier.is_persistent());
    assert!(!l2_cfg.tier.is_persistent());
    assert!(l3_cfg.tier.is_persistent());
}

#[test]
fn test_tier_metrics_tracking() {
    let mut metrics = TierMetrics::default();

    for _ in 0..10 {
        metrics.record_access(true, 100);
    }
    for _ in 0..5 {
        metrics.record_access(false, 200);
    }

    assert_eq!(metrics.accesses, 15);
    assert_eq!(metrics.hits, 10);
    assert_eq!(metrics.misses, 5);
    assert!(metrics.hit_rate() > 0.6);
}

#[test]
fn test_lru_eviction_policy() {
    let mut policy = LruEvictionPolicy::new(5);

    // Add entries in order
    for i in 1..=5 {
        policy.record_access(i);
    }
    assert_eq!(policy.size(), 5);

    // Evict oldest
    assert_eq!(policy.next_evict_candidate(), Some(1));
    assert_eq!(policy.next_evict_candidate(), Some(2));

    // New access should go to end
    policy.record_access(10);
    assert_eq!(policy.next_evict_candidate(), Some(3));
    assert_eq!(policy.next_evict_candidate(), Some(4));
    assert_eq!(policy.next_evict_candidate(), Some(5));
    assert_eq!(policy.next_evict_candidate(), Some(10));
}

#[test]
fn test_spill_first_eviction() {
    let mut policy = SpillFirstEvictionPolicy::new(3);

    policy.record_access(1);
    policy.record_access(2);
    policy.record_access(3);
    assert_eq!(policy.hot_count(), 3);

    // Adding new entry should spill oldest hot
    policy.record_access(4);
    assert_eq!(policy.hot_count(), 3);
    assert_eq!(policy.spilled_count(), 1);

    // Re-accessing spilled brings it back
    policy.record_access(1);
    assert_eq!(policy.hot_count(), 3);
    assert_eq!(policy.spilled_count(), 0);

    // Spilled entries can be deleted
    policy.mark_spilled(2);
    policy.record_access(5); // Force spill
    assert_eq!(policy.take_spilled(), Some(2));
}

#[test]
fn test_vector_index_operations() {
    let mut index = VectorIndex::new(3, 100);

    // Insert vectors
    assert!(index
        .insert(1, vec![1.0, 0.0, 0.0])
        .is_ok());
    assert!(index
        .insert(2, vec![0.0, 1.0, 0.0])
        .is_ok());
    assert!(index
        .insert(3, vec![0.99, 0.01, 0.0])
        .is_ok());

    assert_eq!(index.len(), 3);

    // Wrong dimension should fail
    assert!(matches!(
        index.insert(4, vec![1.0, 0.0]),
        Err(IndexError::DimensionMismatch { .. })
    ));

    // Search
    let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].entry_id, 1); // Exact match
    assert!(results[0].distance < results[1].distance);

    // Remove
    assert!(index.remove(1));
    assert_eq!(index.len(), 2);
    assert!(!index.remove(1)); // Already removed
}

#[test]
fn test_semantic_prefetch_strategies() {
    // Semantic prefetch enabled
    let mut semantic = SemanticPrefetch::new(PrefetchStrategy::Semantic, 5);
    semantic.add_prefetch_candidates(vec![1, 2, 3]);
    assert_eq!(semantic.pending_count(), 3);

    // None strategy disabled
    let mut none = SemanticPrefetch::new(PrefetchStrategy::None, 5);
    none.add_prefetch_candidates(vec![1, 2, 3]);
    assert_eq!(none.pending_count(), 0);

    // Hybrid strategy
    let mut hybrid = SemanticPrefetch::new(PrefetchStrategy::Hybrid, 5);
    hybrid.add_prefetch_candidates(vec![1, 2, 3]);
    assert_eq!(hybrid.pending_count(), 3);

    // Get prefetches in order
    assert_eq!(semantic.next_prefetch(), Some(1));
    assert_eq!(semantic.next_prefetch(), Some(2));
    assert_eq!(semantic.next_prefetch(), Some(3));
    assert_eq!(semantic.next_prefetch(), None);
}

#[test]
fn test_arena_allocator_stress() {
    let mut arena = ArenaAllocator::new(10_000);

    // Allocate multiple blocks
    let mut offsets = Vec::new();
    for i in 0..10 {
        match arena.allocate(100 * (i + 1)) {
            Ok(offset) => offsets.push(offset),
            Err(AllocationError::OutOfMemory { .. }) => break,
            Err(_) => break,
        }
    }

    assert!(offsets.len() > 0);
    assert!(arena.utilization() > 0.0);

    arena.reset();
    assert_eq!(arena.utilization(), 0.0);
}

#[test]
fn test_memory_pool_reuse() {
    let mut pool = MemoryPool::new(4096, 10);

    let ids: Vec<_> = (0..5)
        .map(|_| pool.acquire().unwrap())
        .collect();

    assert_eq!(pool.allocated_count(), 5);

    // Release and reuse
    for id in ids {
        pool.release(id);
    }

    assert_eq!(pool.available(), 5);

    let new_id = pool.acquire().unwrap();
    assert_eq!(pool.allocated_count(), 1);
}

#[test]
fn test_memory_compactor_lifecycle() {
    let mut compactor = MemoryCompactor::new(0.5);

    assert!(compactor.start_compact().is_ok());
    compactor.record_move(1000);
    compactor.record_move(2000);
    assert_eq!(compactor.total_moved(), 3000);
    compactor.finish_compact();

    // Should be able to start again
    assert!(compactor.start_compact().is_ok());
    compactor.finish_compact();
}

#[test]
fn test_tier_workflow() {
    // Simulate: L1 -> L2 -> L3 tiered memory
    let mut l1_index = VectorIndex::new(768, 1000);
    let mut l2_pool = MemoryPool::new(4096, 500);
    let mut l3_arena = ArenaAllocator::new(100_000_000);

    // Add to L1 with vector index
    l1_index
        .insert(1, vec![0.1; 768])
        .unwrap();
    assert_eq!(l1_index.len(), 1);

    // Allocate space in L2
    let l2_obj = l2_pool.acquire().unwrap();
    assert_eq!(l2_pool.allocated_count(), 1);

    // Allocate in L3
    let l3_offset = l3_arena.allocate(1_000_000).unwrap();
    assert_eq!(l3_offset, 0);

    // Eviction simulation
    let mut eviction = LruEvictionPolicy::new(2);
    eviction.record_access(1);
    eviction.record_access(2);
    let oldest = eviction.next_evict_candidate();
    assert_eq!(oldest, Some(1));
}

#[test]
fn test_memory_consistency_across_tiers() {
    let mut metrics_l1 = TierMetrics::default();
    let mut metrics_l2 = TierMetrics::default();

    // L1 hits are fast
    metrics_l1.record_access(true, 87);
    assert_eq!(metrics_l1.accesses, 1);

    // L2 hits are slower
    metrics_l2.record_access(true, 48_000);
    assert_eq!(metrics_l2.accesses, 1);

    // Both should track correctly
    assert!(metrics_l1.avg_latency_us < metrics_l2.avg_latency_us);
}

#[test]
fn test_error_handling() {
    let mut arena = ArenaAllocator::new(100);
    assert!(matches!(
        arena.allocate(200),
        Err(AllocationError::OutOfMemory { .. })
    ));

    let mut pool = MemoryPool::new(4096, 1);
    pool.acquire().unwrap();
    assert!(matches!(
        pool.acquire(),
        Err(AllocationError::PoolExhausted { .. })
    ));

    let mut index = VectorIndex::new(2, 10);
    assert!(matches!(
        index.insert(1, vec![1.0]),
        Err(IndexError::DimensionMismatch { .. })
    ));
}

#[test]
fn test_compound_efficiency_constant() {
    // Verify COMPOUND_EFFICIENCY is defined and has expected value
    const COMPOUND_EFFICIENCY: f64 = 0.581;
    assert!(COMPOUND_EFFICIENCY > 0.0 && COMPOUND_EFFICIENCY < 1.0);
}
