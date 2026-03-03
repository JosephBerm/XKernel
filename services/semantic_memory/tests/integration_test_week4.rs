// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Integration tests for Week 4 Memory Manager deliverables.
//!
//! Tests:
//! 1. CT spawn with L1 memory mapped into address space
//! 2. Scale testing: allocate/deallocate 1K-1M pages
//! 3. L1 allocator integration with page pool
//! 4. Context sizing and memory initialization
//! 5. Memory Manager initialization and IPC handling
//!
//! See Engineering Plan § 4.1.1: Integration Testing.

#[cfg(test)]
mod integration_tests {
    use cs_semantic_memory::{
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;
        StubMemoryManager, MemoryManagerConfig, PagePool, L1Allocator,
        MemoryManagerState, MemoryRequest, MemoryTierSpec,
        L1SizingCalculator, ModelContextWindow, HeapAllocator,
        MemoryRegionID, PAGE_SIZE,
    };

    /// Integration test: CT spawn with L1 memory mapping.
    #[test]
    fn test_ct_spawn_with_l1_mapping() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        assert_eq!(mm.state(), &MemoryManagerState::Ready);

        let ct_id = 1;
        let l1_size = 256 * 1024 * 1024;

        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L1,
            size: l1_size,
            capability: format!("ct-{}-l1-alloc", ct_id),
        };

        let response = mm.handle_request(request).unwrap();

        if let cs_semantic_memory::MemoryResponse::Allocated {
            region_id,
            mapped_addr,
        } = response
        {
            assert!(!region_id.is_empty());
            assert!(mapped_addr > 0);
        } else {
            panic!("Expected Allocated response");
        }

        let allocator = mm.l1_allocator().unwrap();
        assert_eq!(allocator.allocation_count(), 1);
    }

    /// Scale test: Allocate and deallocate 1K pages.
    #[test]
    fn test_scale_1k_pages() {
        let mut page_pool = PagePool::new(2048, 0x1000_0000).unwrap();
        let mut allocations = Vec::new();

        for _ in 0..1024 {
            let (page_idx, _) = page_pool.allocate_page(1).unwrap();
            allocations.push(page_idx);
        }

        assert_eq!(page_pool.allocated_pages_count(), 1024);

        for page_idx in allocations {
            page_pool.deallocate_page(page_idx).unwrap();
        }

        assert_eq!(page_pool.allocated_pages_count(), 0);
    }

    /// Scale test: Allocate and deallocate 10K pages.
    #[test]
    fn test_scale_10k_pages() {
        let mut page_pool = PagePool::new(20000, 0x1000_0000).unwrap();
        let mut allocations = Vec::new();

        for _ in 0..10000 {
            let (page_idx, _) = page_pool.allocate_page(1).unwrap();
            allocations.push(page_idx);
        }

        assert_eq!(page_pool.allocated_pages_count(), 10000);

        for page_idx in allocations {
            page_pool.deallocate_page(page_idx).unwrap();
        }

        assert_eq!(page_pool.allocated_pages_count(), 0);
    }

    /// Scale test: Bulk allocation of 100K pages.
    #[test]
    fn test_scale_100k_pages_bulk() {
        let mut page_pool = PagePool::new(200000, 0x1000_0000).unwrap();
        let (first_idx, _addr) = page_pool.allocate_pages(100000, 1).unwrap();

        assert_eq!(page_pool.allocated_pages_count(), 100000);

        page_pool.deallocate_pages(first_idx, 100000).unwrap();
        assert_eq!(page_pool.allocated_pages_count(), 0);
    }

    /// Scale test: L1 allocator with 1M page pool.
    #[test]
    fn test_scale_l1_allocator_1m_pages() {
        let mut allocator = L1Allocator::new(
            MemoryRegionID::l1_gpu_local(),
            1_000_000,
            0x1000_0000,
        )
        .unwrap();

        let mut alloc_ids = Vec::new();
        for i in 0..40 {
            let size = 100 * 1024 * 1024;
            let (alloc_id, _, _) = allocator.allocate(size, i).unwrap();
            alloc_ids.push(alloc_id);
        }

        assert_eq!(allocator.allocation_count(), 40);

        for alloc_id in alloc_ids {
            allocator.deallocate(alloc_id).unwrap();
        }

        assert_eq!(allocator.allocation_count(), 0);
    }

    /// Integration test: Context sizing and L1 initialization.
    #[test]
    fn test_context_sizing_initialization() {
        let models = vec![
            ModelContextWindow::context_32k(),
            ModelContextWindow::claude_128k(),
            ModelContextWindow::context_512k(),
        ];

        for model in models {
            let calc = L1SizingCalculator::new(
                model.clone(),
                8 * 1024 * 1024 * 1024,
                0.10,
            );

            let l1_size = calc.calculate_l1_size().unwrap();
            assert!(l1_size > 0);
            assert!(l1_size <= 8 * 1024 * 1024 * 1024);

            let page_count = (l1_size + PAGE_SIZE - 1) / PAGE_SIZE;
            let allocator = L1Allocator::new(
                MemoryRegionID::l1_gpu_local(),
                page_count,
                0x1000_0000,
            )
            .unwrap();

            assert_eq!(allocator.total_capacity_bytes(), page_count * PAGE_SIZE);
        }
    }

    /// Integration test: Memory Manager initialization and serving.
    #[test]
    fn test_memory_manager_initialization_and_serving() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();

        assert_eq!(mm.state(), &MemoryManagerState::Initializing);

        mm.initialize().unwrap();
        assert_eq!(mm.state(), &MemoryManagerState::Ready);

        for i in 0..10 {
            let request = MemoryRequest::Allocate {
                tier: MemoryTierSpec::L1,
                size: (i + 1) * 4096,
                capability: format!("cap-{}", i),
            };

            let result = mm.handle_request(request);
            assert!(result.is_ok());
        }

        assert_eq!(mm.request_count(), 10);
        assert_eq!(mm.error_count(), 0);

        mm.shutdown().unwrap();
        assert_eq!(mm.state(), &MemoryManagerState::Terminated);
    }

    /// Integration test: Heap allocator for MM internals.
    #[test]
    fn test_heap_allocator_internals() {
        let mut heap = HeapAllocator::new(0x2000_0000, 256 * 1024 * 1024);

        let addr1 = heap.allocate_aligned8(1024).unwrap();
        let addr2 = heap.allocate_aligned16(2048).unwrap();
        let _addr3 = heap.allocate_unaligned(512).unwrap();

        assert!(addr1 < addr2);
        assert_eq!(addr1 & 0x7, 0);
        assert_eq!(addr2 & 0xF, 0);
        assert!(heap.utilization() > 0.0);
    }

    /// Integration test: Multiple concurrent allocations.
    #[test]
    fn test_multiple_concurrent_allocations() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let ct_ids = vec![1, 2, 3, 4, 5];
        let mut total_allocated = 0u64;

        for ct_id in ct_ids {
            for size_mb in 1..5 {
                let size = size_mb * 1024 * 1024;
                let request = MemoryRequest::Allocate {
                    tier: MemoryTierSpec::L1,
                    size,
                    capability: format!("ct-{}-alloc", ct_id),
                };

                if let Ok(_response) = mm.handle_request(request) {
                    total_allocated += size;
                }
            }
        }

        assert!(total_allocated > 0);

        let query = MemoryRequest::Query {
            region_id: "l1-gpu-local".to_string(),
            capability: "query".to_string(),
        };

        let response = mm.handle_request(query).unwrap();
        if let cs_semantic_memory::MemoryResponse::QueryResult { stats } = response {
            assert!(stats.used_bytes >= total_allocated);
        }
    }

    /// Integration test: L1 allocation with resizing.
    #[test]
    fn test_l1_allocation_with_resize() {
        let mut allocator = L1Allocator::new(
            MemoryRegionID::l1_gpu_local(),
            10000,
            0x1000_0000,
        )
        .unwrap();

        let (alloc_id, _, _addr1) = allocator.allocate(100 * PAGE_SIZE, 1).unwrap();

        let _addr2 = allocator.resize(alloc_id, 150 * PAGE_SIZE).unwrap();
        assert!(allocator.total_allocated_bytes() >= 150 * PAGE_SIZE);

        let _addr3 = allocator.resize(alloc_id, 50 * PAGE_SIZE).unwrap();
        assert_eq!(allocator.total_allocated_bytes(), 50 * PAGE_SIZE);

        allocator.deallocate(alloc_id).unwrap();
        assert_eq!(allocator.total_allocated_bytes(), 0);
    }

    /// Integration test: Page pool fragmentation recovery.
    #[test]
    fn test_page_pool_fragmentation_recovery() {
        let mut pool = PagePool::new(1000, 0x1000_0000).unwrap();
        let mut allocations = Vec::new();

        for i in 0..500 {
            let (page_idx, _) = pool.allocate_page(i % 100).unwrap();
            allocations.push(page_idx);
        }

        for (i, &page_idx) in allocations.iter().enumerate() {
            if i % 2 == 0 {
                let _ = pool.deallocate_page(page_idx);
            }
        }

        let (_, _) = pool.allocate_page(999).unwrap();
    }

    /// Integration test: Page pinning for critical allocations.
    #[test]
    fn test_page_pinning_for_critical_allocations() {
        let mut allocator = L1Allocator::new(
            MemoryRegionID::l1_gpu_local(),
            10000,
            0x1000_0000,
        )
        .unwrap();

        let (alloc_id, _, _) = allocator.allocate(100 * PAGE_SIZE, 1).unwrap();

        allocator.pin(alloc_id).unwrap();

        let result = allocator.deallocate(alloc_id);
        assert!(result.is_err());

        allocator.unpin(alloc_id).unwrap();
        allocator.deallocate(alloc_id).unwrap();
    }
}
