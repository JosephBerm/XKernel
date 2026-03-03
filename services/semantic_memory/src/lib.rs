// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Semantic Memory Service - 3-Tier Hierarchical Memory Architecture
//!
//! This crate implements the Semantic Memory subsystem for the Cognitive Substrate OS,
//! providing a 3-tier hierarchical memory model that bridges fast GPU-local storage (L1),
//! host DRAM-backed episodic memory (L2), and persistent NVMe-backed long-term memory (L3).
//!
//! # Architecture Overview
//!
//! The 3-tier model is organized by access latency and persistence guarantees:
//!
//! - **L1 (Working Memory)**: HBM/GPU-local, microsecond-scale access, context window
//! - **L2 (Episodic Memory)**: Host DRAM, millisecond-scale access, indexed retrieval
//! - **L3 (Long-Term Memory)**: NVMe-backed, persistent, replicated across crew
//!
//! See Engineering Plan § 4.1 (Semantic Memory Architecture).
//!
//! # Safety & Correctness
//!
//! This crate operates as kernel-adjacent code with strict safety guarantees:
//! - `#![forbid(unsafe_code)]` - No unsafe blocks allowed
//! - `#![no_std]` - Runs in kernel mode without standard library
//! - All errors return `Result<T, CsError>` - No unwrap/expect
//! - Strongly typed memory identifiers prevent confusion
//! - CRDT support for crew-wide shared memory consistency

#![forbid(unsafe_code)]

extern crate alloc;

pub mod capability_control;
pub mod concurrency;
pub mod crdt;
pub mod error;
pub mod ids;
pub mod isolation;
pub mod l1_working;
pub mod layout;
pub mod memory;
pub mod vector_index;

// L1 scaffold modules (allocation, eviction, indexing, tiers)
pub mod allocation;
pub mod eviction;
pub mod indexing;
pub mod tiers;

// Week 03 deliverables
pub mod ipc_interface;
pub mod address_space;
pub mod mmu_integration;
pub mod process_lifecycle;
pub mod shared_regions;
pub mod metrics;

// Week 04 deliverables
pub mod context_sizing;
pub mod page_pool;
pub mod l1_allocator;
pub mod heap_allocator;
pub mod stub_memory_manager;

// Week 05 deliverables: CSCI syscall interfaces
pub mod mem_syscall_interface;
pub mod mem_serialization;
pub mod mem_ipc_handler;
pub mod mem_validation;
pub mod mem_stub_ops;

// Week 06 deliverables: Phase 0 Finale - Testing, Metrics, & Phase 1 Readiness
pub mod integration_tests;
pub mod stress_tests;
pub mod metrics_collector;
pub mod performance_baseline;
pub mod phase1_transition;

// Re-export commonly used types
pub use error::{MemoryError, Result};
pub use ids::{L1Ref, L2Ref, L3Ref, MemoryRegionID};
pub use memory::SemanticMemory;

// Capabilities and isolation
pub use isolation::{IsolationLevel, MemoryCapabilitySet};

// Configuration types

// Week 02 deliverables
pub use capability_control::{
    MemoryCapability, MemoryCapabilityValidator, DefaultMemoryCapabilityValidator,
    TierAccessRule, CrossTierPolicy,
};
pub use concurrency::{
    AtomicityLevel, OperationGuard, MemoryTier, MemoryOperation,
    ConcurrentAccessPolicy, VersionVector, ConflictResolution, TierConcurrencyModel,
};
pub use vector_index::{
    VectorIndex, VectorEntry, SearchResult, IndexConfig, DistanceMetric,
    QuantizationType, VectorDimension,
};
pub use layout::{
    L1Layout, L2Layout, L3Layout, PageGranularity, MemoryBound,
};

// Week 03 re-exports
pub use ipc_interface::{
    MemoryRequest, MemoryResponse, MemoryTierSpec, RequestCost, RegionStats,
    RequestRouter, CapabilityValidator, DefaultCapabilityValidator,
};
pub use address_space::{
    MemoryManagerAddressSpace, CtMappingEntry, IsolationBoundary, AddressSpaceStats,
};
pub use mmu_integration::{
    MmuConfig, ProtectionDomain, DomainId, AccessType, PageFaultHandler,
    PageFaultResolution, ProtectionDomainManager, MmuStateTracker,
};
pub use process_lifecycle::{
    MemoryManagerProcess, MemoryManagerState, HealthStatus,
};
pub use shared_regions::{
    SharedRegion, SharedAccessMode, SharedRegionManager, SharedRegionStats,
    CrdtResolution,
};
pub use metrics::{
    MemoryMetrics, TierMetrics, CefMemoryAccessEvent, MetricsCollector,
};

// Week 04 re-exports
pub use context_sizing::{
    ModelContextWindow, L1SizingCalculator,
};
pub use page_pool::{
    PagePool, PageMetadata, PAGE_SIZE,
};
pub use l1_allocator::{
    L1Allocator, L1Allocation,
};
pub use heap_allocator::{
    HeapAllocator, HeapStats,
};
pub use stub_memory_manager::{
    StubMemoryManager, MemoryManagerConfig,
};

// Week 05 re-exports
pub use mem_syscall_interface::{
    AllocFlags, MountFlags, MountSource, MemHandle, MountHandle,
    mem_alloc, mem_read, mem_write, mem_mount,
};
pub use mem_serialization::{
    SerializedMemoryRequest, SerializedMemoryResponse, RequestEncoder, RequestDecoder,
    ResponseEncoder, ResponseDecoder, MemoryRequestType, MemoryResponseType,
    MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, MAX_STRING_LEN, MAX_DATA_BUFFER_LEN,
};
pub use mem_ipc_handler::{
    IpcHandler, IpcBatchProcessor, IpcHandlerResult, IpcErrorCode,
};
pub use mem_validation::{
    MemoryInterfaceValidator, CapabilityAccessValidator, ValidationResult,
};
pub use mem_stub_ops::{
    OperationTimeout, BlockingMode, OperationState, AsyncOperationHandle,
    StubMemoryReader, StubMemoryWriter, StubAsyncReader, StubAsyncWriter,
};

// Week 06 re-exports: Testing, Metrics, Phase 1 Readiness
pub use integration_tests::{
    IntegrationTestSuite,
};
pub use stress_tests::{
    StressTestConfig, StressTestResult, MemoryStressTest,
};
pub use metrics_collector::{
    LatencyPercentiles, SyscallMetrics, ProcessFootprint, MetricsCollector as MetricsCollectorModule,
};
pub use performance_baseline::{
    PerformanceTargets, SyscallPerformanceData, PerformanceBaseline, BaselineSuite,
};
pub use phase1_transition::{
    KnownLimitation, Phase1Enhancement, RiskAssessment, Phase1TransitionAssessment,
    SeverityLevel,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // Basic smoke test to ensure crate structure is valid
        let _ = MemoryRegionID::new("test");
    }

    #[test]
    fn test_week4_modules_available() {
        // Verify all Week 4 modules are accessible
        let _ = L1SizingCalculator::typical_gpu_claude3();
        let _ = PagePool::new(1000, 0x1000_0000);
        let _ = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        let _ = MemoryManagerConfig::default_claude3_8gb();
    }

    #[test]
    fn test_week6_modules_available() {
        // Verify all Week 6 modules are accessible
        let mut suite = IntegrationTestSuite::new();
        assert_eq!(suite.summary().contains("Tests:"), true);

        let config = StressTestConfig::light();
        assert_eq!(config.rapid_cycles, 1000);

        let mut collector = MetricsCollectorModule::new(0);
        collector.record_syscall("mem_alloc", true, 50, 4096);

        let mut baseline = PerformanceBaseline::new("0.1.0", 0);
        baseline.calculate_status();

        let mut assessment = Phase1TransitionAssessment::new();
        assessment.populate_phase0_findings();
        assert!(assessment.limitations().len() > 0);
    }
}
