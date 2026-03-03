// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Integration tests for GPU accelerator L1 scaffolds (scheduling, vram, profiling).

use cs_gpu_accelerator::profiling::{Bottleneck, GpuMs, GpuProfile, GpuUtilization, ProfileCollector};
use cs_gpu_accelerator::scheduling::{RightSizer, SpatialScheduler, TpcAllocation};
use cs_gpu_accelerator::vram_management::{
    IsolatedVramRegion, IsolationLevel, KvcacheAllocator, VramError, VramPool,
};

#[test]
fn test_tpc_allocation_throughput() {
    let small = TpcAllocation::new(20, 1500, 150);
    let large = TpcAllocation::new(80, 1500, 250);

    assert!(large.estimated_throughput_tflops() > small.estimated_throughput_tflops());
}

#[test]
fn test_spatial_scheduler_basic() {
    let mut scheduler = SpatialScheduler::new(80);

    let alloc1 = scheduler.schedule_kernel(1, 30).unwrap();
    assert_eq!(alloc1.tpc_count, 30);

    let alloc2 = scheduler.schedule_kernel(2, 40).unwrap();
    assert_eq!(alloc2.tpc_count, 40);

    assert_eq!(scheduler.free_tpcs(), 10);
    assert_eq!(scheduler.scheduled_count(), 2);
}

#[test]
fn test_scheduler_insufficient_tpcs() {
    let mut scheduler = SpatialScheduler::new(50);
    scheduler.schedule_kernel(1, 40).unwrap();

    assert!(scheduler.schedule_kernel(2, 20).is_err());
}

#[test]
fn test_scheduler_unschedule() {
    let mut scheduler = SpatialScheduler::new(80);
    scheduler.schedule_kernel(1, 50).unwrap();
    assert_eq!(scheduler.free_tpcs(), 30);

    assert!(scheduler.unschedule_kernel(1));
    assert_eq!(scheduler.free_tpcs(), 80);
}

#[test]
fn test_right_sizer_profiling() {
    use cs_gpu_accelerator::scheduling::KernelProfile;

    let mut sizer = RightSizer::new(8, 80);

    sizer.profile_kernel(KernelProfile {
        kernel_id: 1,
        flops: 500_000_000,
        memory_bytes: 500_000,
        latency_ms: 100,
    });

    let sized = sizer.right_size(1, 100);
    assert!(sized >= sizer.min_tpc && sized <= sizer.max_tpc);
}

#[test]
fn test_vram_pool_allocation() {
    let mut pool = VramPool::new(0, 1_000_000);

    let alloc1 = pool.allocate(100_000).unwrap();
    let alloc2 = pool.allocate(200_000).unwrap();

    assert_eq!(pool.allocated_bytes, 300_000);
    assert!(pool.utilization() > 0.29 && pool.utilization() < 0.31);

    pool.free(alloc1.alloc_id).unwrap();
    assert_eq!(pool.allocated_bytes, 200_000);
}

#[test]
fn test_vram_exhaustion() {
    let mut pool = VramPool::new(0, 1000);
    pool.allocate(600).unwrap();

    assert!(matches!(
        pool.allocate(500),
        Err(VramError::OutOfMemory { .. })
    ));
}

#[test]
fn test_isolated_vram_regions() {
    let logical = IsolatedVramRegion::new(1, 1, 0, 1_000_000, IsolationLevel::Logical);
    let physical = IsolatedVramRegion::new(2, 2, 1, 1_000_000, IsolationLevel::Physical);

    assert!(!logical.is_physically_isolated());
    assert!(physical.is_physically_isolated());
}

#[test]
fn test_kvcache_block_allocation() {
    let mut alloc = KvcacheAllocator::new(10);

    let block1 = alloc.create_block(256).unwrap();
    let block2 = alloc.create_block(512).unwrap();

    alloc.allocate_tokens(block1.block_id, 100).unwrap();
    alloc.allocate_tokens(block2.block_id, 300).unwrap();

    assert_eq!(alloc.total_tokens_allocated(), 400);
    assert_eq!(alloc.total_capacity(), 768);
}

#[test]
fn test_kvcache_overflow() {
    let mut alloc = KvcacheAllocator::new(10);
    let block = alloc.create_block(100).unwrap();

    alloc.allocate_tokens(block.block_id, 80).unwrap();

    assert!(matches!(
        alloc.allocate_tokens(block.block_id, 30),
        Err(VramError::KvcacheFull { .. })
    ));
}

#[test]
fn test_gpu_timing_metrics() {
    let timing = GpuMs::new(60, 25, 15);
    assert_eq!(timing.total_ms, 100);
    assert!(timing.kernel_efficiency() > 0.5);
    assert!(timing.memory_overhead() > 0.2);
}

#[test]
fn test_gpu_utilization_bottleneck() {
    // Compute-bound workload
    let compute = GpuUtilization::new(95, 40, 75, 30);
    assert!(compute.is_compute_bound());
    assert_eq!(compute.bottleneck(), Bottleneck::Compute);

    // Memory-bound workload
    let memory = GpuUtilization::new(40, 95, 85, 30);
    assert!(memory.is_memory_bound());
    assert_eq!(memory.bottleneck(), Bottleneck::Memory);

    // Thermal-limited
    let thermal = GpuUtilization::new(70, 70, 70, 99);
    assert!(thermal.is_thermal_throttled());
    assert_eq!(thermal.bottleneck(), Bottleneck::Thermal);
}

#[test]
fn test_gpu_profile_collection() {
    let mut collector = ProfileCollector::new(100);

    for i in 0..10 {
        let timing = GpuMs::new(50 + i as u32 * 5, 20, 5);
        let util = GpuUtilization::new(80, 70, 85, 45);
        let profile = GpuProfile::new(i as u64, timing, util);
        collector.record(profile);
    }

    assert_eq!(collector.profile_count(), 10);

    let avg_util = collector.average_utilization().unwrap();
    assert_eq!(avg_util.compute_percent, 80);

    let avg_time = collector.average_execution_time().unwrap();
    assert!(avg_time.total_ms > 0);
}

#[test]
fn test_vram_and_kvcache_interaction() {
    let mut vram = VramPool::new(0, 8_000_000);
    let mut kvcache = KvcacheAllocator::new(16);

    // Allocate VRAM
    let vram_alloc = vram.allocate(1_000_000).unwrap();
    assert_eq!(vram.allocation_count(), 1);

    // Allocate KV-cache blocks
    for _ in 0..8 {
        let _ = kvcache.create_block(256).unwrap();
    }

    assert_eq!(kvcache.block_count(), 8);

    // Allocate tokens
    for i in 0..8 {
        kvcache.allocate_tokens(i, 128).unwrap();
    }

    assert_eq!(kvcache.total_tokens_allocated(), 1024);
    assert_eq!(vram.utilization(), 0.125); // 1MB of 8MB
}

#[test]
fn test_scheduler_with_vram() {
    let mut scheduler = SpatialScheduler::new(80);
    let mut vram = VramPool::new(0, 16_000_000_000);

    // Schedule kernel
    let alloc = scheduler.schedule_kernel(1, 40).unwrap();
    assert_eq!(alloc.tpc_count, 40);

    // Allocate its working memory
    let mem_alloc = vram.allocate(100_000_000).unwrap(); // 100MB
    assert_eq!(vram.allocated_bytes, 100_000_000);

    scheduler.unschedule_kernel(1);
    vram.free(mem_alloc.alloc_id).unwrap();
}

#[test]
fn test_profiling_workflow() {
    let mut collector = ProfileCollector::new(50);
    let mut scheduler = SpatialScheduler::new(80);

    // Simulate kernel execution and profiling
    for kernel_id in 0..5 {
        // Schedule
        let _ = scheduler.schedule_kernel(kernel_id, 20 + kernel_id * 5);

        // Profile execution
        let timing = GpuMs::new(50, 20, 10);
        let util = GpuUtilization::new(80 + kernel_id as u8, 70, 85, 40);
        let profile = GpuProfile::new(kernel_id, timing, util);
        collector.record(profile);

        // Unschedule
        scheduler.unschedule_kernel(kernel_id);
    }

    assert_eq!(collector.profile_count(), 5);
    assert_eq!(scheduler.scheduled_count(), 0);
}

#[test]
fn test_error_handling() {
    let mut pool = VramPool::new(0, 100);
    assert!(pool.allocate(50).is_ok());
    assert!(matches!(pool.allocate(100), Err(VramError::OutOfMemory { .. })));

    let mut kvcache = KvcacheAllocator::new(1);
    let block = kvcache.create_block(100).unwrap();
    kvcache.allocate_tokens(block.block_id, 100).unwrap();

    assert!(matches!(
        kvcache.allocate_tokens(block.block_id, 1),
        Err(VramError::KvcacheFull { .. })
    ));
}
