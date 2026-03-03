// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! TPC-level GPU scheduling (LithOS-inspired architecture).
//!
//! The GPU Manager schedules at the Tensor Processing Cluster (TPC) granularity,
//! enabling fine-grained crew isolation and priority management. Directives flow
//! from the Cognitive Scheduler to the GPU Manager, which applies the specified
//! policy and allocates TPCs accordingly.
//!
//! Reference: Engineering Plan § Kernel Scheduling, LithOS Architecture

use crate::ids::TpcID;
use alloc::vec::Vec;
use core::fmt;

/// TPC allocation descriptor for a crew.
///
/// Describes which TPCs are allocated to a specific crew with associated
/// priority and scheduling parameters. Used by the GPU Manager to enforce
/// isolation and fair resource distribution.
///
/// Reference: Engineering Plan § TPC-Level Scheduling
#[derive(Clone, Debug)]
pub struct TpcAllocation {
    /// Set of TPC identifiers allocated to this crew.
    ///
    /// All TPCs in this set are exclusive to the crew (no sharing).
    /// Held in sorted order for efficient lookup.
    pub tpc_ids: Vec<TpcID>,

    /// Crew identifier (opaque to GPU Manager).
    ///
    /// Used for bookkeeping and attribution of resource usage.
    pub owner_crew: [u8; 16],

    /// Priority level (0=lowest, 255=highest).
    ///
    /// Used in PriorityBased and CrewAffinity policies.
    /// Higher priority crews get scheduling preference.
    pub priority: u8,

    /// Time slice per scheduling quantum in nanoseconds.
    ///
    /// In time-sliced policies (RoundRobin, CrewAffinity),
    /// kernels from this crew are preempted after this duration.
    pub time_slice_ns: u64,
}

impl TpcAllocation {
    /// Create a new TPC allocation.
    pub fn new(
        tpc_ids: Vec<TpcID>,
        owner_crew: [u8; 16],
        priority: u8,
        time_slice_ns: u64,
    ) -> Self {
        TpcAllocation {
            tpc_ids,
            owner_crew,
            priority,
            time_slice_ns,
        }
    }

    /// Get the count of allocated TPCs.
    pub fn tpc_count(&self) -> usize {
        self.tpc_ids.len()
    }
}

/// GPU scheduling policy applied by the scheduler.
///
/// Defines the algorithm used to arbitrate TPC access among competing crews.
/// More sophisticated policies (FairShare, CrewAffinity) require coordination
/// with the Cognitive Scheduler.
///
/// Reference: Engineering Plan § Scheduling Policies
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSchedulingPolicy {
    /// Simple round-robin time-slicing among allocated crews.
    ///
    /// Each crew's kernels execute for its configured time slice before
    /// yielding to the next crew in the queue. Provides basic fairness.
    RoundRobin,

    /// Priority-based preemption (higher priority preempts lower).
    ///
    /// Kernels from higher-priority crews are scheduled before lower-priority
    /// crews. Suitable for latency-sensitive inference tasks.
    PriorityBased,

    /// Fair-share scheduler (proportional allocation).
    ///
    /// Each crew receives GPU time proportional to its allocated TPC share.
    /// More sophisticated, requires inter-crew coordination.
    FairShare,

    /// Crew-affinity scheduling (NUMA-inspired).
    ///
    /// Crew kernels preferentially run on their affinity TPCs. Falls back to
    /// other TPCs under oversubscription. Reduces context switching overhead.
    CrewAffinity,
}

impl fmt::Display for GpuSchedulingPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuSchedulingPolicy::RoundRobin => write!(f, "RoundRobin"),
            GpuSchedulingPolicy::PriorityBased => write!(f, "PriorityBased"),
            GpuSchedulingPolicy::FairShare => write!(f, "FairShare"),
            GpuSchedulingPolicy::CrewAffinity => write!(f, "CrewAffinity"),
        }
    }
}

/// Directive from the Cognitive Scheduler to the GPU Manager.
///
/// Commands the GPU Manager to allocate/deallocate TPCs, change policies,
/// or modify scheduling parameters. Directives arrive asynchronously from
/// the Cognitive Scheduler based on workload changes.
///
/// Reference: Engineering Plan § Scheduler Integration
#[derive(Clone, Debug)]
pub struct SchedulerDirective {
    /// Directive sequence number (for ordering and deduplication).
    pub sequence_number: u64,

    /// Target policy to activate.
    pub target_policy: GpuSchedulingPolicy,

    /// TPC allocations for crews.
    ///
    /// Describes which TPCs are assigned to each crew.
    /// An empty list means no crews (all TPCs idle).
    pub allocations: Vec<TpcAllocation>,

    /// Timestamp in nanoseconds since boot.
    pub timestamp_ns: u64,
}

impl SchedulerDirective {
    /// Create a new scheduler directive.
    pub fn new(
        sequence_number: u64,
        target_policy: GpuSchedulingPolicy,
        allocations: Vec<TpcAllocation>,
        timestamp_ns: u64,
    ) -> Self {
        SchedulerDirective {
            sequence_number,
            target_policy,
            allocations,
            timestamp_ns,
        }
    }

    /// Get total TPC count across all allocations.
    pub fn total_allocated_tpcs(&self) -> usize {
        self.allocations
            .iter()
            .map(|alloc| alloc.tpc_count())
            .sum()
    }

    /// Find allocation for a specific crew.
    pub fn find_allocation(&self, crew_id: &[u8; 16]) -> Option<&TpcAllocation> {
        self.allocations.iter().find(|alloc| &alloc.owner_crew == crew_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::GpuDeviceID;
use alloc::format;
use alloc::vec;

    #[test]
    fn test_tpc_allocation_creation() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);
        let tpc_ids = vec![TpcID::new(device_id, 0), TpcID::new(device_id, 1)];
        let crew_id = [1u8; 16];

        let alloc = TpcAllocation::new(tpc_ids.clone(), crew_id, 10, 1_000_000);

        assert_eq!(alloc.tpc_count(), 2);
        assert_eq!(alloc.owner_crew, crew_id);
        assert_eq!(alloc.priority, 10);
        assert_eq!(alloc.time_slice_ns, 1_000_000);
    }

    #[test]
    fn test_scheduling_policy_display() {
        assert_eq!(format!("{}", GpuSchedulingPolicy::RoundRobin), "RoundRobin");
        assert_eq!(
            format!("{}", GpuSchedulingPolicy::PriorityBased),
            "PriorityBased"
        );
    }

    #[test]
    fn test_scheduler_directive_creation() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);
        let tpc_ids = vec![TpcID::new(device_id, 0)];
        let crew_id = [1u8; 16];
        let alloc = TpcAllocation::new(tpc_ids, crew_id, 50, 5_000_000);

        let directive = SchedulerDirective::new(
            1,
            GpuSchedulingPolicy::CrewAffinity,
            vec![alloc],
            1_000_000_000,
        );

        assert_eq!(directive.sequence_number, 1);
        assert_eq!(directive.target_policy, GpuSchedulingPolicy::CrewAffinity);
        assert_eq!(directive.allocations.len(), 1);
        assert_eq!(directive.timestamp_ns, 1_000_000_000);
    }

    #[test]
    fn test_scheduler_directive_total_tpcs() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let crew1 = [1u8; 16];
        let tpc_ids_1 = vec![TpcID::new(device_id, 0), TpcID::new(device_id, 1)];
        let alloc1 = TpcAllocation::new(tpc_ids_1, crew1, 10, 1_000_000);

        let crew2 = [2u8; 16];
        let tpc_ids_2 = vec![TpcID::new(device_id, 2)];
        let alloc2 = TpcAllocation::new(tpc_ids_2, crew2, 20, 1_000_000);

        let directive = SchedulerDirective::new(
            1,
            GpuSchedulingPolicy::PriorityBased,
            vec![alloc1, alloc2],
            0,
        );

        assert_eq!(directive.total_allocated_tpcs(), 3);
    }

    #[test]
    fn test_scheduler_directive_find_allocation() {
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let crew1 = [1u8; 16];
        let tpc_ids_1 = vec![TpcID::new(device_id, 0)];
        let alloc1 = TpcAllocation::new(tpc_ids_1, crew1, 10, 1_000_000);

        let crew2 = [2u8; 16];
        let tpc_ids_2 = vec![TpcID::new(device_id, 1)];
        let alloc2 = TpcAllocation::new(tpc_ids_2, crew2, 20, 1_000_000);

        let directive = SchedulerDirective::new(
            1,
            GpuSchedulingPolicy::RoundRobin,
            vec![alloc1, alloc2],
            0,
        );

        let found = directive.find_allocation(&crew1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().owner_crew, crew1);

        let not_found = directive.find_allocation(&[99u8; 16]);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_empty_directive() {
        let directive = SchedulerDirective::new(
            0,
            GpuSchedulingPolicy::RoundRobin,
            vec![],
            0,
        );

        assert_eq!(directive.total_allocated_tpcs(), 0);
        assert_eq!(directive.allocations.len(), 0);
    }
}
