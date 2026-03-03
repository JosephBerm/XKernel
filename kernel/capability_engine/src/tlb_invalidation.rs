// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! TLB (Translation Lookaside Buffer) invalidation strategies.
//!
//! This module implements efficient TLB invalidation for single and multi-core systems.
//!
//! Strategies:
//! 1. **Local Invalidation**: INVLPG (x86_64) or TLBI (ARM64) for single vaddr
//! 2. **Global Invalidation**: Full TLB flush for all addresses
//! 3. **IPI Broadcast**: Multi-core TLB shootdown via inter-processor interrupts
//!
//! Target: <5000ns (5 µs) on 8-core systems for single-page invalidation.
//!
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration,
//! specifically § 5.4: TLB Invalidation.

use core::fmt::{self, Debug, Display};
use alloc::vec::Vec;

use crate::error::CapError;
use crate::mmu_abstraction::VirtualAddr;
use crate::ids::AgentID;

/// TLB invalidation method selection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TlbInvalidationMethod {
    /// Invalidate a single virtual address (fast path).
    /// x86_64: INVLPG; ARM64: TLBI VAAE1IS
    SingleAddress,

    /// Invalidate all TLB entries (slower but complete).
    /// x86_64: MOV CR3, CR3; ARM64: TLBI VMALLE1IS
    All,

    /// Invalidate a range of virtual addresses (for bulk operations).
    /// Uses multiple single-address invalidations with batching.
    Range,
}

impl Display for TlbInvalidationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlbInvalidationMethod::SingleAddress => write!(f, "SingleAddress"),
            TlbInvalidationMethod::All => write!(f, "All"),
            TlbInvalidationMethod::Range => write!(f, "Range"),
        }
    }
}

/// Represents a single TLB invalidation operation.
#[derive(Clone, Debug)]
pub struct TlbInvalidationOp {
    /// The virtual address to invalidate (or None for all-TLB flush).
    pub virtual_address: Option<VirtualAddr>,

    /// The invalidation method used.
    pub method: TlbInvalidationMethod,

    /// Whether this operation needs to broadcast via IPI on multi-core.
    pub needs_ipi_broadcast: bool,

    /// Target CPUs for IPI broadcast (bitmask, 0 = current CPU only).
    pub target_cpus: u64,
}

impl TlbInvalidationOp {
    /// Creates a single-address invalidation operation.
    pub fn single_address(vaddr: VirtualAddr, needs_ipi: bool) -> Self {
        TlbInvalidationOp {
            virtual_address: Some(vaddr),
            method: TlbInvalidationMethod::SingleAddress,
            needs_ipi_broadcast: needs_ipi,
            target_cpus: 0,
        }
    }

    /// Creates an all-TLB invalidation operation.
    pub fn all_tlb(needs_ipi: bool) -> Self {
        TlbInvalidationOp {
            virtual_address: None,
            method: TlbInvalidationMethod::All,
            needs_ipi_broadcast: needs_ipi,
            target_cpus: 0,
        }
    }

    /// Creates a range invalidation operation.
    pub fn range(start: VirtualAddr, end: VirtualAddr, needs_ipi: bool) -> Self {
        TlbInvalidationOp {
            virtual_address: Some(start),
            method: TlbInvalidationMethod::Range,
            needs_ipi_broadcast: needs_ipi,
            target_cpus: 0,
        }
    }

    /// Sets the target CPUs for IPI broadcast.
    /// Bit i set = CPU i is targeted.
    pub fn with_target_cpus(mut self, cpus: u64) -> Self {
        self.target_cpus = cpus;
        self
    }
}

impl Display for TlbInvalidationOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.virtual_address {
            Some(vaddr) => write!(
                f,
                "TlbOp({}, vaddr=0x{:x}, ipi={})",
                self.method, vaddr, self.needs_ipi_broadcast
            ),
            None => write!(
                f,
                "TlbOp({}, all-tlb, ipi={})",
                self.method, self.needs_ipi_broadcast
            ),
        }
    }
}

/// Statistics on TLB invalidation operations.
#[derive(Clone, Copy, Debug, Default)]
pub struct TlbInvalidationStats {
    /// Total number of TLB invalidations performed.
    pub total_invalidations: u64,

    /// Number of single-address invalidations.
    pub single_address_invalidations: u64,

    /// Number of all-TLB invalidations.
    pub all_tlb_invalidations: u64,

    /// Number of range invalidations.
    pub range_invalidations: u64,

    /// Number of IPI broadcasts sent.
    pub ipi_broadcasts: u64,

    /// Total nanoseconds spent in TLB invalidation.
    pub total_nanoseconds: u64,
}

impl TlbInvalidationStats {
    /// Averages nanoseconds per invalidation.
    pub fn avg_nanoseconds_per_op(&self) -> u64 {
        if self.total_invalidations > 0 {
            self.total_nanoseconds / self.total_invalidations
        } else {
            0
        }
    }

    /// Records an invalidation operation.
    pub fn record_op(&mut self, op: &TlbInvalidationOp, nanos: u64) {
        self.total_invalidations += 1;
        self.total_nanoseconds += nanos;

        match op.method {
            TlbInvalidationMethod::SingleAddress => {
                self.single_address_invalidations += 1;
            }
            TlbInvalidationMethod::All => {
                self.all_tlb_invalidations += 1;
            }
            TlbInvalidationMethod::Range => {
                self.range_invalidations += 1;
            }
        }

        if op.needs_ipi_broadcast {
            self.ipi_broadcasts += 1;
        }
    }
}

impl Display for TlbInvalidationStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TlbStats(total={}, single={}, all={}, range={}, ipi={}, avg_ns={})",
            self.total_invalidations,
            self.single_address_invalidations,
            self.all_tlb_invalidations,
            self.range_invalidations,
            self.ipi_broadcasts,
            self.avg_nanoseconds_per_op()
        )
    }
}

/// Abstract TLB invalidation service.
///
/// This trait abstracts TLB invalidation across architectures (x86_64, ARM64).
pub trait TlbInvalidationService: Send + Sync {
    /// Performs a local TLB invalidation on the current CPU.
    ///
    /// For single-address: executes INVLPG (x86_64) or TLBI VAAE1IS (ARM64)
    /// For all-TLB: executes CR3 reload (x86_64) or TLBI VMALLE1IS (ARM64)
    ///
    /// Target latency: <1000ns (1 µs) for single-address on warm cache.
    fn invalidate_local(&mut self, op: &TlbInvalidationOp) -> Result<u64, CapError>;

    /// Performs a global TLB invalidation via IPI broadcast on multi-core.
    ///
    /// Sends an IPI (inter-processor interrupt) to target CPUs to invalidate
    /// their TLB entries for the specified addresses. The calling CPU's TLB
    /// is also invalidated.
    ///
    /// Target latency: <5000ns (5 µs) on 8-core for single-page invalidation.
    ///
    /// # Arguments
    /// * `op` - The invalidation operation (must have `needs_ipi_broadcast = true`)
    ///
    /// # Returns
    /// Ok(nanos_elapsed) if successful, or an error.
    fn invalidate_global(&mut self, op: &TlbInvalidationOp) -> Result<u64, CapError>;

    /// Performs a TLB invalidation (selecting local vs global automatically).
    ///
    /// This is the high-level interface: automatically chooses between
    /// local and global invalidation based on the operation's needs.
    fn invalidate(&mut self, op: &TlbInvalidationOp) -> Result<u64, CapError> {
        if op.needs_ipi_broadcast {
            self.invalidate_global(op)
        } else {
            self.invalidate_local(op)
        }
    }

    /// Batches multiple invalidation operations for efficiency.
    ///
    /// Used when multiple pages need to be invalidated together.
    /// Implementation may optimize by combining operations.
    ///
    /// # Arguments
    /// * `ops` - Vector of TLB invalidation operations
    ///
    /// # Returns
    /// Ok(total_nanoseconds_elapsed) if successful, or an error.
    fn invalidate_batch(&mut self, ops: &[TlbInvalidationOp]) -> Result<u64, CapError>;

    /// Gets current TLB invalidation statistics.
    fn stats(&self) -> &TlbInvalidationStats;

    /// Returns the number of CPUs in the system.
    fn cpu_count(&self) -> usize;

    /// Returns the current CPU number.
    fn current_cpu(&self) -> usize;
}

/// A simple in-memory TLB invalidation tracker (for testing).
#[derive(Clone, Debug, Default)]
pub struct MockTlbInvalidationService {
    /// Log of all invalidation operations.
    pub operations: Vec<TlbInvalidationOp>,

    /// Statistics.
    pub stats: TlbInvalidationStats,

    /// Number of CPUs (for testing).
    num_cpus: usize,
}

impl MockTlbInvalidationService {
    /// Creates a new mock TLB service with the specified number of CPUs.
    pub fn new(num_cpus: usize) -> Self {
        MockTlbInvalidationService {
            operations: Vec::new(),
            stats: TlbInvalidationStats::default(),
            num_cpus,
        }
    }
}

impl TlbInvalidationService for MockTlbInvalidationService {
    fn invalidate_local(&mut self, op: &TlbInvalidationOp) -> Result<u64, CapError> {
        let nanos = 500; // Simulate 500ns
        self.operations.push(op.clone());
        self.stats.record_op(op, nanos);
        Ok(nanos)
    }

    fn invalidate_global(&mut self, op: &TlbInvalidationOp) -> Result<u64, CapError> {
        let nanos = 2000 + (self.num_cpus as u64 * 500); // Base 2µs + 500ns per CPU
        self.operations.push(op.clone());
        self.stats.record_op(op, nanos);
        Ok(nanos)
    }

    fn invalidate_batch(&mut self, ops: &[TlbInvalidationOp]) -> Result<u64, CapError> {
        let mut total = 0u64;
        for op in ops {
            total += self.invalidate(op)?;
        }
        Ok(total)
    }

    fn stats(&self) -> &TlbInvalidationStats {
        &self.stats
    }

    fn cpu_count(&self) -> usize {
        self.num_cpus
    }

    fn current_cpu(&self) -> usize {
        0 // Always CPU 0 in mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec;

    #[test]
    fn test_tlb_invalidation_op_single_address() {
        let op = TlbInvalidationOp::single_address(0x10000, false);
        assert_eq!(op.virtual_address, Some(0x10000));
        assert_eq!(op.method, TlbInvalidationMethod::SingleAddress);
        assert!(!op.needs_ipi_broadcast);
    }

    #[test]
    fn test_tlb_invalidation_op_all_tlb() {
        let op = TlbInvalidationOp::all_tlb(true);
        assert_eq!(op.virtual_address, None);
        assert_eq!(op.method, TlbInvalidationMethod::All);
        assert!(op.needs_ipi_broadcast);
    }

    #[test]
    fn test_tlb_invalidation_op_with_target_cpus() {
        let op = TlbInvalidationOp::single_address(0x10000, true)
            .with_target_cpus(0b1111); // CPUs 0-3
        assert_eq!(op.target_cpus, 0b1111);
    }

    #[test]
    fn test_tlb_stats_record() {
        let mut stats = TlbInvalidationStats::default();
        let op = TlbInvalidationOp::single_address(0x10000, false);

        stats.record_op(&op, 500);
        assert_eq!(stats.total_invalidations, 1);
        assert_eq!(stats.single_address_invalidations, 1);
        assert_eq!(stats.total_nanoseconds, 500);

        stats.record_op(&op, 600);
        assert_eq!(stats.total_invalidations, 2);
        assert_eq!(stats.avg_nanoseconds_per_op(), 550);
    }

    #[test]
    fn test_mock_tlb_invalidation_service() {
        let mut service = MockTlbInvalidationService::new(8);

        let op = TlbInvalidationOp::single_address(0x10000, false);
        let nanos = service.invalidate_local(&op).unwrap();

        assert_eq!(service.operations.len(), 1);
        assert!(nanos > 0);
    }

    #[test]
    fn test_mock_tlb_global_invalidation() {
        let mut service = MockTlbInvalidationService::new(8);

        let op = TlbInvalidationOp::single_address(0x10000, true);
        let nanos = service.invalidate_global(&op).unwrap();

        // Should be slower than local (base 2µs + per-CPU overhead)
        assert!(nanos >= 2000);
    }

    #[test]
    fn test_mock_tlb_invalidate_batch() {
        let mut service = MockTlbInvalidationService::new(8);

        let ops = vec![
            TlbInvalidationOp::single_address(0x10000, false),
            TlbInvalidationOp::single_address(0x20000, false),
            TlbInvalidationOp::single_address(0x30000, false),
        ];

        let total_nanos = service.invalidate_batch(&ops).unwrap();

        assert_eq!(service.operations.len(), 3);
        assert!(total_nanos > 0);
    }

    #[test]
    fn test_tlb_stats_avg_calculation() {
        let mut stats = TlbInvalidationStats::default();
        let op1 = TlbInvalidationOp::single_address(0x10000, false);
        let op2 = TlbInvalidationOp::single_address(0x20000, true);

        stats.record_op(&op1, 1000);
        stats.record_op(&op2, 5000);

        assert_eq!(stats.total_invalidations, 2);
        assert_eq!(stats.total_nanoseconds, 6000);
        assert_eq!(stats.avg_nanoseconds_per_op(), 3000);
        assert_eq!(stats.ipi_broadcasts, 1);
    }
}
