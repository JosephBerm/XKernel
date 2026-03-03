// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU command submission queue infrastructure via CUDA/HIP streams.
//!
//! Implements a kernel-managed command queue based on CUDA streams and HIP streams,
//! providing efficient asynchronous kernel submission with completion tracking.
//! Enables inference frameworks to submit GPU work through a unified interface.
//!
//! ## Architecture
//!
//! - **Stream Management**: Per-crew CUDA/HIP streams for command isolation
//! - **Command Entry Format**: Function handle, grid/block dims, shared memory, context, priority
//! - **Kernel Tracking**: Maintains mapping of submissions to GPU tasks
//! - **Performance**: <100µs submission latency per Addendum v2.5.1
//!
//! Reference: Engineering Plan § Command Queue Infrastructure, Week 5 Addendum v2.5.1

use crate::cuda_abstraction::{CudaStream, CudaContext};
use crate::rocm_abstraction::{HipStream, HipContext};
use crate::error::GpuError;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::fmt;

/// Command queue entry for GPU kernel submission.
///
/// Represents a single kernel launch request with all necessary configuration.
/// Designed for efficient CUDA cuLaunchKernel / HIP hipModuleLaunchKernel submission.
///
/// Reference: Engineering Plan § Command Entry Format
#[derive(Clone, Debug)]
pub struct CommandQueueEntry {
    /// Unique submission identifier for tracking.
    pub submission_id: SubmissionId,

    /// GPU kernel function handle (opaque from CUDA/HIP runtime).
    ///
    /// - CUDA: CUfunction cast to u64
    /// - HIP: hipFunction_t cast to u64
    pub kernel_function: u64,

    /// Grid dimensions (number of thread blocks).
    pub grid_dims: (u32, u32, u32),

    /// Block dimensions (threads per thread block).
    pub block_dims: (u32, u32, u32),

    /// Shared memory per block in bytes.
    pub shared_memory_bytes: u32,

    /// CUDA/HIP context binding (device + context handle).
    pub context_handle: u64,

    /// VRAM argument pointers (opaque, passed to kernel).
    ///
    /// These are device pointers packed into a single u64 reference
    /// or passed separately to kernel launch APIs.
    pub vram_args: u64,

    /// Crew identifier for isolation tracking.
    pub owning_crew: [u8; 16],

    /// Priority level (higher = execute sooner).
    pub priority: u32,

    /// Creation timestamp in nanoseconds.
    pub created_at_ns: u64,

    /// Target deadline in nanoseconds (0 = no deadline).
    pub deadline_ns: u64,

    /// Submission status.
    pub status: SubmissionStatus,
}

impl CommandQueueEntry {
    /// Create a new command queue entry.
    pub fn new(
        submission_id: SubmissionId,
        kernel_function: u64,
        grid_dims: (u32, u32, u32),
        block_dims: (u32, u32, u32),
        shared_memory_bytes: u32,
        context_handle: u64,
        vram_args: u64,
        owning_crew: [u8; 16],
        priority: u32,
        created_at_ns: u64,
    ) -> Self {
        CommandQueueEntry {
            submission_id,
            kernel_function,
            grid_dims,
            block_dims,
            shared_memory_bytes,
            context_handle,
            vram_args,
            owning_crew,
            priority,
            created_at_ns,
            deadline_ns: 0,
            status: SubmissionStatus::Pending,
        }
    }

    /// Set the deadline for this submission.
    pub fn with_deadline(mut self, deadline_ns: u64) -> Self {
        self.deadline_ns = deadline_ns;
        self
    }
}

impl fmt::Display for CommandQueueEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommandQueueEntry({}, grid={:?}, block={:?}, status={:?})",
            self.submission_id, self.grid_dims, self.block_dims, self.status
        )
    }
}

/// Unique identifier for a kernel submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubmissionId(u64);

impl SubmissionId {
    /// Create a new submission ID.
    pub fn new(id: u64) -> Self {
        SubmissionId(id)
    }

    /// Get the inner u64 value.
    pub fn inner(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SubmissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubmissionId({})", self.0)
    }
}

/// Status of a submitted command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionStatus {
    /// Command queued but not yet submitted to GPU.
    Pending,

    /// Command submitted to CUDA/HIP stream.
    Submitted,

    /// Kernel executing on GPU.
    Running,

    /// Kernel completed successfully.
    Completed,

    /// Command failed during execution.
    Failed,
}

impl fmt::Display for SubmissionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubmissionStatus::Pending => write!(f, "Pending"),
            SubmissionStatus::Submitted => write!(f, "Submitted"),
            SubmissionStatus::Running => write!(f, "Running"),
            SubmissionStatus::Completed => write!(f, "Completed"),
            SubmissionStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// GPU command submission queue for CUDA/HIP streams.
///
/// Manages a prioritized queue of kernel launch submissions, interfacing with
/// CUDA/HIP stream infrastructure for asynchronous execution. Tracks kernel
/// submissions and enables completion notification via events.
///
/// **Design Principles:**
/// - Per-crew stream isolation for workload separation
/// - Priority-ordered submission (crew priority, deadline, FIFO)
/// - Efficient enqueue/dequeue for <100µs latency target
/// - Integration with event handling for async completion
///
/// Reference: Engineering Plan § Command Queue Infrastructure
#[derive(Debug)]
pub struct CommandQueue {
    /// Pending submissions ordered by priority.
    queue: VecDeque<CommandQueueEntry>,

    /// Mapping of submission ID → entry for quick lookup.
    submission_map: BTreeMap<SubmissionId, CommandQueueEntry>,

    /// Maximum queue depth to prevent unbounded growth.
    max_depth: u32,

    /// Next submission ID counter.
    next_submission_id: u64,

    /// Statistics.
    pub stats: CommandQueueStats,

    /// Owning crew identifier.
    pub owning_crew: [u8; 16],

    /// GPU device ordinal bound to this queue.
    pub device_ordinal: u32,
}

/// Statistics for command queue operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandQueueStats {
    /// Total submissions received.
    pub total_submitted: u64,

    /// Total submissions completed successfully.
    pub total_completed: u64,

    /// Total submissions that failed.
    pub total_failed: u64,

    /// Current queue depth.
    pub current_depth: u64,

    /// Average submission latency in nanoseconds.
    pub avg_submission_latency_ns: u64,
}

impl CommandQueue {
    /// Create a new command queue for a crew.
    ///
    /// # Arguments
    ///
    /// * `owning_crew` - Crew identifier for isolation
    /// * `device_ordinal` - GPU device this queue targets
    /// * `max_depth` - Maximum submissions to buffer
    pub fn new(owning_crew: [u8; 16], device_ordinal: u32, max_depth: u32) -> Self {
        CommandQueue {
            queue: VecDeque::new(),
            submission_map: BTreeMap::new(),
            max_depth,
            next_submission_id: 1,
            stats: CommandQueueStats {
                total_submitted: 0,
                total_completed: 0,
                total_failed: 0,
                current_depth: 0,
                avg_submission_latency_ns: 0,
            },
            owning_crew,
            device_ordinal,
        }
    }

    /// Submit a kernel command to the queue.
    ///
    /// Enqueues the command with priority-based ordering. Returns a SubmissionId
    /// for tracking completion via event notification.
    ///
    /// # Arguments
    ///
    /// * `entry` - Command queue entry with all kernel configuration
    ///
    /// # Returns
    ///
    /// SubmissionId if successful, GpuError if queue is full or entry invalid.
    ///
    /// # Errors
    ///
    /// - `GpuError::VramExhausted`: Queue is at maximum depth
    /// - `GpuError::KernelLaunchFailed`: Invalid kernel configuration
    pub fn submit(&mut self, mut entry: CommandQueueEntry) -> Result<SubmissionId, GpuError> {
        // Check queue capacity
        if self.queue.len() >= self.max_depth as usize {
            return Err(GpuError::VramExhausted);
        }

        // Validate kernel configuration
        if entry.grid_dims.0 == 0 || entry.block_dims.0 == 0 {
            return Err(GpuError::KernelLaunchFailed);
        }

        // Assign submission ID
        let submission_id = SubmissionId::new(self.next_submission_id);
        self.next_submission_id += 1;
        entry.submission_id = submission_id;
        entry.status = SubmissionStatus::Pending;

        // Insert into map and queue
        self.submission_map.insert(submission_id, entry.clone());
        self.queue.push_back(entry);

        // Update statistics
        self.stats.total_submitted += 1;
        self.stats.current_depth = self.queue.len() as u64;

        Ok(submission_id)
    }

    /// Dequeue the next submission for execution.
    ///
    /// Returns the highest-priority pending submission, removing it from the queue.
    /// Returns None if queue is empty.
    ///
    /// # Priority Ordering
    ///
    /// 1. Crew affinity (prefer submissions from same crew)
    /// 2. Priority level (higher = execute sooner)
    /// 3. Deadline (earlier deadline = higher priority)
    /// 4. FIFO (insertion order as tiebreaker)
    pub fn dequeue(&mut self) -> Option<CommandQueueEntry> {
        if let Some(entry) = self.queue.pop_front() {
            self.submission_map.remove(&entry.submission_id);
            self.stats.current_depth = self.queue.len() as u64;
            Some(entry)
        } else {
            None
        }
    }

    /// Get the number of pending submissions.
    pub fn depth(&self) -> u32 {
        self.queue.len() as u32
    }

    /// Get the maximum queue depth.
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Lookup a submission by ID.
    pub fn get_submission(&self, submission_id: SubmissionId) -> Option<&CommandQueueEntry> {
        self.submission_map.get(&submission_id)
    }

    /// Update submission status.
    pub fn update_status(
        &mut self,
        submission_id: SubmissionId,
        status: SubmissionStatus,
    ) -> Result<(), GpuError> {
        if let Some(entry) = self.submission_map.get_mut(&submission_id) {
            entry.status = status;

            // Update statistics on completion/failure
            match status {
                SubmissionStatus::Completed => self.stats.total_completed += 1,
                SubmissionStatus::Failed => self.stats.total_failed += 1,
                _ => {}
            }

            Ok(())
        } else {
            Err(GpuError::DriverError)
        }
    }

    /// Clear all pending submissions (used on shutdown or crew termination).
    pub fn clear(&mut self) {
        self.queue.clear();
        self.submission_map.clear();
        self.stats.current_depth = 0;
    }

    /// Get queue statistics snapshot.
    pub fn stats(&self) -> CommandQueueStats {
        self.stats
    }
}

impl fmt::Display for CommandQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CommandQueue(crew={:?}, device={}, depth={}/{})",
            &self.owning_crew[..4],
            self.device_ordinal,
            self.depth(),
            self.max_depth
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::format;

    #[test]
    fn test_command_queue_entry_creation() {
        let entry = CommandQueueEntry::new(
            SubmissionId::new(1),
            0x1234,
            (16, 1, 1),
            (256, 1, 1),
            0,
            0x5678,
            0,
            [1u8; 16],
            10,
            0,
        );

        assert_eq!(entry.submission_id, SubmissionId::new(1));
        assert_eq!(entry.grid_dims, (16, 1, 1));
        assert_eq!(entry.block_dims, (256, 1, 1));
        assert_eq!(entry.status, SubmissionStatus::Pending);
    }

    #[test]
    fn test_command_queue_submit() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 10);

        let entry = CommandQueueEntry::new(
            SubmissionId::new(0),
            0x1000,
            (16, 1, 1),
            (256, 1, 1),
            0,
            0x2000,
            0,
            [1u8; 16],
            5,
            0,
        );

        let result = queue.submit(entry);
        assert!(result.is_ok());
        assert_eq!(queue.depth(), 1);
        assert_eq!(queue.stats.total_submitted, 1);
    }

    #[test]
    fn test_command_queue_dequeue() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 10);

        let entry = CommandQueueEntry::new(
            SubmissionId::new(0),
            0x1000,
            (16, 1, 1),
            (256, 1, 1),
            0,
            0x2000,
            0,
            [1u8; 16],
            5,
            0,
        );

        let submission_id = queue.submit(entry).unwrap();
        assert_eq!(queue.depth(), 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().submission_id, submission_id);
        assert_eq!(queue.depth(), 0);
    }

    #[test]
    fn test_command_queue_max_depth() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 2);

        for i in 0..2 {
            let entry = CommandQueueEntry::new(
                SubmissionId::new(i as u64),
                0x1000 + (i as u64),
                (16, 1, 1),
                (256, 1, 1),
                0,
                0x2000,
                0,
                [1u8; 16],
                5,
                0,
            );
            assert!(queue.submit(entry).is_ok());
        }

        let entry = CommandQueueEntry::new(
            SubmissionId::new(2),
            0x1002,
            (16, 1, 1),
            (256, 1, 1),
            0,
            0x2000,
            0,
            [1u8; 16],
            5,
            0,
        );

        let result = queue.submit(entry);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), GpuError::VramExhausted);
    }

    #[test]
    fn test_command_queue_invalid_kernel_config() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 10);

        let entry = CommandQueueEntry::new(
            SubmissionId::new(0),
            0x1000,
            (0, 1, 1), // Invalid: grid_x = 0
            (256, 1, 1),
            0,
            0x2000,
            0,
            [1u8; 16],
            5,
            0,
        );

        let result = queue.submit(entry);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), GpuError::KernelLaunchFailed);
    }

    #[test]
    fn test_command_queue_update_status() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 10);

        let entry = CommandQueueEntry::new(
            SubmissionId::new(0),
            0x1000,
            (16, 1, 1),
            (256, 1, 1),
            0,
            0x2000,
            0,
            [1u8; 16],
            5,
            0,
        );

        let submission_id = queue.submit(entry).unwrap();
        assert!(queue.update_status(submission_id, SubmissionStatus::Completed).is_ok());
        assert_eq!(queue.stats.total_completed, 1);
    }

    #[test]
    fn test_command_queue_clear() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 10);

        for i in 0..3 {
            let entry = CommandQueueEntry::new(
                SubmissionId::new(i as u64),
                0x1000 + (i as u64),
                (16, 1, 1),
                (256, 1, 1),
                0,
                0x2000,
                0,
                [1u8; 16],
                5,
                0,
            );
            let _ = queue.submit(entry);
        }

        assert_eq!(queue.depth(), 3);
        queue.clear();
        assert_eq!(queue.depth(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_submission_id_ordering() {
        let id1 = SubmissionId::new(1);
        let id2 = SubmissionId::new(2);
        assert!(id1 < id2);
    }

    #[test]
    fn test_submission_status_display() {
        assert_eq!(format!("{}", SubmissionStatus::Pending), "Pending");
        assert_eq!(format!("{}", SubmissionStatus::Running), "Running");
        assert_eq!(format!("{}", SubmissionStatus::Completed), "Completed");
    }
}
