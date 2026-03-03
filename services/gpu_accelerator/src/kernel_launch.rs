// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Kernel atomization and launch management (LithOS-inspired).
//!
//! Implements kernel launch abstraction with preemption points and maximum
//! duration bounds. Kernels are "atomized" into small compute units that
//! can be preempted at defined boundaries to enable fair scheduling across
//! multiple competing crews.
//!
//! Reference: Engineering Plan § Kernel Atomization, Preemptive Scheduling

use crate::ids::KernelLaunchID;
use alloc::vec::Vec;
use core::fmt;

/// Kernel launch parameters (grid and block dimensions).
///
/// Standard CUDA/HIP kernel configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelDimensions {
    /// Grid dimensions (number of blocks).
    pub grid_x: u32,
    pub grid_y: u32,
    pub grid_z: u32,

    /// Block dimensions (threads per block).
    pub block_x: u32,
    pub block_y: u32,
    pub block_z: u32,
}

impl KernelDimensions {
    /// Create new kernel dimensions.
    pub const fn new(
        grid_x: u32,
        grid_y: u32,
        grid_z: u32,
        block_x: u32,
        block_y: u32,
        block_z: u32,
    ) -> Self {
        KernelDimensions {
            grid_x,
            grid_y,
            grid_z,
            block_x,
            block_y,
            block_z,
        }
    }

    /// Get total number of blocks (grid size).
    pub fn total_blocks(&self) -> u64 {
        (self.grid_x as u64) * (self.grid_y as u64) * (self.grid_z as u64)
    }

    /// Get total number of threads (grid size * block size).
    pub fn total_threads(&self) -> u64 {
        self.total_blocks()
            * (self.block_x as u64)
            * (self.block_y as u64)
            * (self.block_z as u64)
    }
}

impl fmt::Display for KernelDimensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "grid<{},{},{}> block<{},{},{}>",
            self.grid_x, self.grid_y, self.grid_z, self.block_x, self.block_y, self.block_z
        )
    }
}

/// Kernel launch descriptor.
///
/// Represents a single kernel invocation with configuration parameters,
/// resource requirements, and tracking information.
///
/// Reference: Engineering Plan § Kernel Atomization
#[derive(Clone, Debug)]
pub struct KernelLaunch {
    /// Unique launch identifier.
    pub id: KernelLaunchID,

    /// Kernel program (opaque binary or source, driver-specific).
    pub program: [u8; 32], // Hash or identifier of the kernel binary

    /// Grid and block dimensions.
    pub dimensions: KernelDimensions,

    /// Shared memory per block in bytes.
    pub shared_mem_bytes: u32,

    /// CUDA stream number (0 = default stream).
    pub stream: u32,

    /// Crew that submitted this launch.
    pub owner_crew: [u8; 16],
}

impl KernelLaunch {
    /// Create a new kernel launch.
    pub fn new(
        id: KernelLaunchID,
        program: [u8; 32],
        dimensions: KernelDimensions,
        shared_mem_bytes: u32,
        stream: u32,
        owner_crew: [u8; 16],
    ) -> Self {
        KernelLaunch {
            id,
            program,
            dimensions,
            shared_mem_bytes,
            stream,
            owner_crew,
        }
    }

    /// Get total work (blocks * threads per block).
    pub fn total_threads(&self) -> u64 {
        self.dimensions.total_threads()
    }
}

impl fmt::Display for KernelLaunch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelLaunch({}, {}, smem={}B, stream={})",
            self.id, self.dimensions, self.shared_mem_bytes, self.stream
        )
    }
}

/// Atomization configuration for kernel preemption.
///
/// Specifies how a kernel is decomposed into preemptable atomic units.
/// Enables fair time-slicing across competing crews.
///
/// Reference: Engineering Plan § Kernel Atomization
#[derive(Clone, Copy, Debug)]
pub struct AtomizationConfig {
    /// Maximum execution time per atom in microseconds.
    ///
    /// If a kernel exceeds this duration without a preemption point,
    /// it may be forcibly preempted. Smaller values enable finer-grained
    /// scheduling at higher overhead cost.
    pub max_duration_us: u32,

    /// Number of preemption points in the kernel.
    ///
    /// 0 = no preemption (run to completion), 1+ = preemption enabled.
    /// More points enable better interleaving at higher synchronization overhead.
    pub preemption_points: u32,
}

impl AtomizationConfig {
    /// Create default atomization config (coarse-grained).
    ///
    /// Max duration: 1ms, 1 preemption point (at kernel end).
    pub fn default() -> Self {
        AtomizationConfig {
            max_duration_us: 1000,
            preemption_points: 1,
        }
    }

    /// Create fine-grained atomization config.
    ///
    /// Max duration: 100us, high preemption points.
    pub fn fine_grained() -> Self {
        AtomizationConfig {
            max_duration_us: 100,
            preemption_points: 10,
        }
    }
}

impl fmt::Display for AtomizationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AtomizationConfig(max_duration={}us, preemption_points={})",
            self.max_duration_us, self.preemption_points
        )
    }
}

/// Launch queue for kernel submissions.
///
/// Maintains a prioritized queue of pending kernel launches.
/// The scheduler dequeues kernels for execution based on crew priority
/// and resource availability.
///
/// Reference: Engineering Plan § Launch Queue Management
#[derive(Clone, Debug)]
pub struct LaunchQueue {
    /// Pending kernel launches (unsorted).
    pub launches: Vec<KernelLaunch>,

    /// Maximum queue depth (bounded to prevent unbounded growth).
    pub max_capacity: usize,

    /// Total enqueued count (lifetime metric).
    pub total_enqueued: u64,

    /// Total dequeued count (lifetime metric).
    pub total_dequeued: u64,
}

impl LaunchQueue {
    /// Create a new launch queue.
    pub fn new(max_capacity: usize) -> Self {
        LaunchQueue {
            launches: Vec::new(),
            max_capacity,
            total_enqueued: 0,
            total_dequeued: 0,
        }
    }

    /// Enqueue a kernel launch.
    ///
    /// Returns false if the queue is full.
    pub fn enqueue(&mut self, launch: KernelLaunch) -> bool {
        if self.launches.len() >= self.max_capacity {
            return false;
        }

        self.launches.push(launch);
        self.total_enqueued += 1;
        true
    }

    /// Dequeue the first kernel launch (FIFO).
    ///
    /// For priority-based scheduling, callers should sort launches
    /// by crew priority before dequeuing.
    pub fn dequeue(&mut self) -> Option<KernelLaunch> {
        if self.launches.is_empty() {
            return None;
        }

        self.total_dequeued += 1;
        Some(self.launches.remove(0))
    }

    /// Get queue depth (pending launches).
    pub fn depth(&self) -> usize {
        self.launches.len()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.launches.is_empty()
    }

    /// Check if queue is full.
    pub fn is_full(&self) -> bool {
        self.launches.len() >= self.max_capacity
    }

    /// Clear all pending launches (for debugging/testing).
    pub fn clear(&mut self) {
        self.launches.clear();
    }
}

impl fmt::Display for LaunchQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LaunchQueue(depth={}/{}, enqueued={}, dequeued={})",
            self.launches.len(),
            self.max_capacity,
            self.total_enqueued,
            self.total_dequeued
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_dimensions_creation() {
        let dims = KernelDimensions::new(2, 1, 1, 32, 32, 1);
        assert_eq!(dims.grid_x, 2);
        assert_eq!(dims.block_x, 32);
        assert_eq!(dims.total_blocks(), 2);
        assert_eq!(dims.total_threads(), 2 * 32 * 32);
    }

    #[test]
    fn test_kernel_dimensions_3d() {
        let dims = KernelDimensions::new(4, 4, 2, 16, 16, 4);
        assert_eq!(dims.total_blocks(), 4 * 4 * 2);
        assert_eq!(dims.total_threads(), 4 * 4 * 2 * 16 * 16 * 4);
    }

    #[test]
    fn test_kernel_launch_creation() {
        let launch_id = KernelLaunchID::from_bytes([0u8; 16]);
        let dims = KernelDimensions::new(1, 1, 1, 32, 1, 1);
        let crew_id = [1u8; 16];

        let launch = KernelLaunch::new(launch_id, [0u8; 32], dims, 4096, 0, crew_id);

        assert_eq!(launch.id, launch_id);
        assert_eq!(launch.shared_mem_bytes, 4096);
        assert_eq!(launch.stream, 0);
        assert_eq!(launch.owner_crew, crew_id);
    }

    #[test]
    fn test_atomization_config_default() {
        let config = AtomizationConfig::default();
        assert_eq!(config.max_duration_us, 1000);
        assert_eq!(config.preemption_points, 1);
    }

    #[test]
    fn test_atomization_config_fine_grained() {
        let config = AtomizationConfig::fine_grained();
        assert_eq!(config.max_duration_us, 100);
        assert!(config.preemption_points > config.preemption_points);
    }

    #[test]
    fn test_launch_queue_creation() {
        let queue = LaunchQueue::new(100);
        assert_eq!(queue.depth(), 0);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
    }

    #[test]
    fn test_launch_queue_enqueue_dequeue() {
        let mut queue = LaunchQueue::new(10);

        let launch_id = KernelLaunchID::from_bytes([0u8; 16]);
        let dims = KernelDimensions::new(1, 1, 1, 32, 1, 1);
        let crew_id = [1u8; 16];

        let launch = KernelLaunch::new(launch_id, [0u8; 32], dims, 0, 0, crew_id);

        assert!(queue.enqueue(launch.clone()));
        assert_eq!(queue.depth(), 1);
        assert_eq!(queue.total_enqueued, 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, launch_id);
        assert_eq!(queue.depth(), 0);
        assert_eq!(queue.total_dequeued, 1);
    }

    #[test]
    fn test_launch_queue_full() {
        let mut queue = LaunchQueue::new(2);

        let launch_id1 = KernelLaunchID::from_bytes([1u8; 16]);
        let launch_id2 = KernelLaunchID::from_bytes([2u8; 16]);
        let launch_id3 = KernelLaunchID::from_bytes([3u8; 16]);

        let dims = KernelDimensions::new(1, 1, 1, 32, 1, 1);
        let crew_id = [1u8; 16];

        let launch1 = KernelLaunch::new(launch_id1, [0u8; 32], dims, 0, 0, crew_id);
        let launch2 = KernelLaunch::new(launch_id2, [0u8; 32], dims, 0, 0, crew_id);
        let launch3 = KernelLaunch::new(launch_id3, [0u8; 32], dims, 0, 0, crew_id);

        assert!(queue.enqueue(launch1));
        assert!(queue.enqueue(launch2));
        assert!(!queue.enqueue(launch3)); // Queue is full

        assert!(queue.is_full());
        assert_eq!(queue.depth(), 2);
    }

    #[test]
    fn test_launch_queue_clear() {
        let mut queue = LaunchQueue::new(100);

        let launch_id = KernelLaunchID::from_bytes([0u8; 16]);
        let dims = KernelDimensions::new(1, 1, 1, 32, 1, 1);
        let crew_id = [1u8; 16];

        let launch = KernelLaunch::new(launch_id, [0u8; 32], dims, 0, 0, crew_id);

        queue.enqueue(launch);
        assert_eq!(queue.depth(), 1);

        queue.clear();
        assert_eq!(queue.depth(), 0);
        assert!(queue.is_empty());
    }
}
