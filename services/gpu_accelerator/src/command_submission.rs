// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU command submission and queueing.
//!
//! Implements a priority-ordered command queue that stages kernel launches,
//! memory operations, and synchronization requests for execution.
//!
//! Scheduling discipline: crew-affinity first, deadline second, FIFO third.
//!
//! Reference: Engineering Plan § Command Submission, Scheduling Discipline

use crate::error::GpuError;
use alloc::collections::VecDeque;
use core::fmt;

/// Unique identifier for a submitted GPU command.
///
/// Returned by submit() for tracking and polling completion status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandHandle(u64);

impl fmt::Display for CommandHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandHandle({})", self.0)
    }
}

/// Stream handle for GPU stream references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamHandle(u64);

/// Memory allocation handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemHandle(u64);

/// Kernel launch configuration (command queue entry).
#[derive(Clone, Copy, Debug)]
pub struct KernelLaunchConfig {
    /// Function handle (opaque GPU function reference).
    pub function: u64,

    /// Grid dimensions (x, y, z).
    pub grid: (u32, u32, u32),

    /// Block dimensions (x, y, z).
    pub block: (u32, u32, u32),

    /// Shared memory size in bytes.
    pub shared_mem: u32,

    /// Target stream.
    pub stream: StreamHandle,

    /// Crew priority (higher = more important).
    pub priority: u32,

    /// Deadline in nanoseconds (for deadline-based scheduling).
    pub deadline_ns: u64,
}

/// Memory allocation configuration.
#[derive(Clone, Copy, Debug)]
pub struct AllocConfig {
    /// Allocation size in bytes.
    pub size: u64,

    /// Allocation type (0=DeviceLocal, 1=Unified, 2=HostPinned).
    pub alloc_type: u8,
}

/// Checkpoint configuration.
#[derive(Clone, Copy, Debug)]
pub struct CheckpointConfig {
    /// Checkpoint ID.
    pub checkpoint_id: u64,

    /// Sequence number for ordering.
    pub sequence_num: u64,
}

/// GPU command in the submission queue.
///
/// Represents a single GPU operation to be scheduled and executed.
#[derive(Clone, Debug)]
pub enum GpuCommand {
    /// Launch a kernel on the GPU.
    LaunchKernel(KernelLaunchConfig),

    /// Allocate device memory.
    AllocMemory(AllocConfig),

    /// Free device memory.
    FreeMemory(MemHandle),

    /// Synchronize a stream.
    Synchronize(StreamHandle),

    /// Checkpoint GPU state.
    Checkpoint(CheckpointConfig),
}

impl fmt::Display for GpuCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuCommand::LaunchKernel(cfg) => {
                write!(f, "LaunchKernel(grid={:?}, block={:?})", cfg.grid, cfg.block)
            }
            GpuCommand::AllocMemory(cfg) => write!(f, "AllocMemory({}B)", cfg.size),
            GpuCommand::FreeMemory(_) => write!(f, "FreeMemory()"),
            GpuCommand::Synchronize(_) => write!(f, "Synchronize()"),
            GpuCommand::Checkpoint(cfg) => write!(f, "Checkpoint(seq={})", cfg.sequence_num),
        }
    }
}

/// Command execution status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    /// Command queued but not yet running.
    Pending,

    /// Command is currently executing on GPU.
    Running,

    /// Command completed successfully.
    Completed,

    /// Command failed during execution.
    Failed,
}

impl fmt::Display for CommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandStatus::Pending => write!(f, "Pending"),
            CommandStatus::Running => write!(f, "Running"),
            CommandStatus::Completed => write!(f, "Completed"),
            CommandStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Result of a completed command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandResult {
    /// Handle of the completed command.
    pub handle: CommandHandle,

    /// Final status.
    pub status: CommandStatus,

    /// Execution latency in nanoseconds (0 if failed).
    pub latency_ns: u64,
}

/// Command queue entry for internal tracking.
#[derive(Clone, Debug)]
struct CommandQueueEntry {
    handle: CommandHandle,
    command: GpuCommand,
    crew_id: [u8; 16],
    priority: u32,
    deadline_ns: u64,
    status: CommandStatus,
}

/// GPU command submission queue.
///
/// Manages pending GPU commands with priority-based ordering:
/// 1. Crew affinity (prefer same crew as previous command)
/// 2. Deadline (earlier deadline = higher priority)
/// 3. FIFO (insertion order as tiebreaker)
///
/// Reference: Engineering Plan § Command Submission, Scheduling Discipline
#[derive(Debug)]
pub struct CommandQueue {
    /// Pending commands in priority order.
    commands: VecDeque<CommandQueueEntry>,

    /// Maximum queue depth (prevent unbounded growth).
    pub max_depth: u32,

    /// Owning crew identifier.
    pub owning_crew: [u8; 16],

    /// Next command handle counter.
    next_handle: u64,

    /// Statistics tracking.
    pub stats: CommandQueueStats,
}

/// Command queue statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandQueueStats {
    /// Total commands submitted.
    pub submitted: u64,

    /// Total commands completed.
    pub completed: u64,

    /// Total commands failed.
    pub failed: u64,

    /// Average command latency in nanoseconds.
    pub avg_latency_ns: u64,
}

impl CommandQueue {
    /// Create a new command queue.
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum number of pending commands
    /// * `owning_crew` - Crew that owns this queue
    pub fn new(max_depth: u32, owning_crew: [u8; 16]) -> Self {
        CommandQueue {
            commands: VecDeque::new(),
            max_depth,
            owning_crew,
            next_handle: 1,
            stats: CommandQueueStats {
                submitted: 0,
                completed: 0,
                failed: 0,
                avg_latency_ns: 0,
            },
        }
    }

    /// Submit a command to the queue.
    ///
    /// # Arguments
    ///
    /// * `command` - GPU command to submit
    /// * `crew_id` - Crew submitting the command
    /// * `priority` - Priority level (higher = more important)
    /// * `deadline_ns` - Deadline in nanoseconds from epoch
    ///
    /// # Returns
    ///
    /// A CommandHandle if successful, or GpuError if queue is full.
    pub fn submit(
        &mut self,
        command: GpuCommand,
        crew_id: [u8; 16],
        priority: u32,
        deadline_ns: u64,
    ) -> Result<CommandHandle, GpuError> {
        if self.commands.len() >= self.max_depth as usize {
            return Err(GpuError::AllocationFailed); // Queue overflow
        }

        let handle = CommandHandle(self.next_handle);
        self.next_handle += 1;

        let entry = CommandQueueEntry {
            handle,
            command,
            crew_id,
            priority,
            deadline_ns,
            status: CommandStatus::Pending,
        };

        // Insert in priority order
        self.insert_in_priority_order(entry);
        self.stats.submitted += 1;

        Ok(handle)
    }

    /// Insert command in priority order.
    ///
    /// Priority ordering: crew-affinity, deadline, then FIFO.
    fn insert_in_priority_order(&mut self, entry: CommandQueueEntry) {
        // Find insertion point
        let mut insert_pos = self.commands.len();

        for (idx, existing) in self.commands.iter().enumerate() {
            if self.should_insert_before(&entry, existing) {
                insert_pos = idx;
                break;
            }
        }

        self.commands.insert(insert_pos, entry);
    }

    /// Determine if a new entry should be inserted before an existing one.
    fn should_insert_before(
        &self,
        new: &CommandQueueEntry,
        existing: &CommandQueueEntry,
    ) -> bool {
        // 1. Crew affinity: same crew as owning_crew comes first
        let new_crew_affinity = new.crew_id == self.owning_crew;
        let existing_crew_affinity = existing.crew_id == self.owning_crew;

        if new_crew_affinity != existing_crew_affinity {
            return new_crew_affinity;
        }

        // 2. Deadline: earlier deadline comes first
        if new.deadline_ns != existing.deadline_ns {
            return new.deadline_ns < existing.deadline_ns;
        }

        // 3. Priority: higher priority comes first
        if new.priority != existing.priority {
            return new.priority > existing.priority;
        }

        // 4. FIFO: maintained by handle ordering
        new.handle.0 < existing.handle.0
    }

    /// Poll completion status of a command.
    ///
    /// # Arguments
    ///
    /// * `handle` - Command handle to poll
    ///
    /// # Returns
    ///
    /// The current CommandStatus.
    pub fn poll_completion(&self, handle: CommandHandle) -> CommandStatus {
        self.commands
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| e.status)
            .unwrap_or(CommandStatus::Completed) // Completed and removed from queue
    }

    /// Flush the queue: execute all pending commands and return results.
    ///
    /// In a real implementation, this would send commands to GPU driver.
    /// For now, this is a placeholder that marks commands as completed.
    ///
    /// # Returns
    ///
    /// Vector of command results for completed/failed commands.
    pub fn flush(&mut self) -> Result<Vec<CommandResult>, GpuError> {
        let mut results = Vec::new();

        while let Some(mut entry) = self.commands.pop_front() {
            // Simulate execution
            entry.status = CommandStatus::Completed;
            let latency_ns = 1000; // Mock latency

            results.push(CommandResult {
                handle: entry.handle,
                status: CommandStatus::Completed,
                latency_ns,
            });

            self.stats.completed += 1;
        }

        // Update average latency
        if self.stats.completed > 0 {
            let total_latency: u64 = results.iter().map(|r| r.latency_ns).sum();
            self.stats.avg_latency_ns = total_latency / results.len() as u64;
        }

        Ok(results)
    }

    /// Get number of pending commands.
    pub fn pending_count(&self) -> usize {
        self.commands.len()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_command_handle_creation() {
        let h1 = CommandHandle(1);
        let h2 = CommandHandle(2);

        assert_ne!(h1, h2);
        assert!(h1 < h2);
    }

    #[test]
    fn test_command_queue_creation() {
        let crew = [1u8; 16];
        let queue = CommandQueue::new(100, crew);

        assert_eq!(queue.max_depth, 100);
        assert_eq!(queue.owning_crew, crew);
        assert!(queue.is_empty());
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.stats.submitted, 0);
    }

    #[test]
    fn test_command_queue_submit() {
        let crew = [1u8; 16];
        let mut queue = CommandQueue::new(100, crew);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));
        let result = queue.submit(cmd, crew, 10, 1000);

        assert!(result.is_ok());
        let handle = result.unwrap();

        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.stats.submitted, 1);
        assert_eq!(queue.poll_completion(handle), CommandStatus::Pending);
    }

    #[test]
    fn test_command_queue_max_depth() {
        let crew = [1u8; 16];
        let mut queue = CommandQueue::new(2, crew);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));

        // Submit first command
        assert!(queue.submit(cmd.clone(), crew, 10, 1000).is_ok());

        // Submit second command
        assert!(queue.submit(cmd.clone(), crew, 10, 1000).is_ok());

        // Submit third command (should fail)
        assert!(queue.submit(cmd, crew, 10, 1000).is_err());

        assert_eq!(queue.pending_count(), 2);
    }

    #[test]
    fn test_command_queue_priority_ordering() {
        let crew1 = [1u8; 16];
        let crew2 = [2u8; 16];
        let mut queue = CommandQueue::new(100, crew1);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));

        // Submit commands with different priorities
        let h1 = queue.submit(cmd.clone(), crew2, 5, 2000).unwrap();
        let h2 = queue.submit(cmd.clone(), crew1, 1, 3000).unwrap(); // Crew affinity
        let h3 = queue.submit(cmd.clone(), crew2, 10, 1000).unwrap(); // Higher priority

        assert_eq!(queue.pending_count(), 3);

        // Check ordering: crew affinity first, then deadline
        let mut handles = vec![];
        while !queue.is_empty() {
            if let Some(entry) = queue.commands.pop_front() {
                handles.push(entry.handle);
            }
        }

        // h2 should come first (crew affinity)
        // Then h3 (earlier deadline)
        // Then h1
        assert_eq!(handles[0], h2);
    }

    #[test]
    fn test_command_queue_deadline_ordering() {
        let crew = [1u8; 16];
        let mut queue = CommandQueue::new(100, crew);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));

        let h1 = queue.submit(cmd.clone(), crew, 10, 3000).unwrap();
        let h2 = queue.submit(cmd.clone(), crew, 10, 1000).unwrap();
        let h3 = queue.submit(cmd.clone(), crew, 10, 2000).unwrap();

        // Should be ordered by deadline: h2, h3, h1
        let first = queue.commands.pop_front().unwrap();
        assert_eq!(first.handle, h2);

        let second = queue.commands.pop_front().unwrap();
        assert_eq!(second.handle, h3);

        let third = queue.commands.pop_front().unwrap();
        assert_eq!(third.handle, h1);
    }

    #[test]
    fn test_command_queue_flush() {
        let crew = [1u8; 16];
        let mut queue = CommandQueue::new(100, crew);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));
        let h1 = queue.submit(cmd.clone(), crew, 10, 1000).unwrap();
        let h2 = queue.submit(cmd, crew, 10, 2000).unwrap();

        let results = queue.flush().unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.stats.completed, 2);

        // Commands should be completed
        assert!(results.iter().all(|r| r.status == CommandStatus::Completed));
    }

    #[test]
    fn test_command_status_display() {
        assert_eq!(format!("{}", CommandStatus::Pending), "Pending");
        assert_eq!(format!("{}", CommandStatus::Running), "Running");
        assert_eq!(format!("{}", CommandStatus::Completed), "Completed");
        assert_eq!(format!("{}", CommandStatus::Failed), "Failed");
    }

    #[test]
    fn test_gpu_command_display() {
        let cmd1 = GpuCommand::LaunchKernel(KernelLaunchConfig {
            function: 0x123,
            grid: (8, 1, 1),
            block: (256, 1, 1),
            shared_mem: 4096,
            stream: StreamHandle(0),
            priority: 10,
            deadline_ns: 1000,
        });

        let display_str = format!("{}", cmd1);
        assert!(display_str.contains("LaunchKernel"));
        assert!(display_str.contains("(8, 1, 1)"));

        let cmd2 = GpuCommand::AllocMemory(AllocConfig {
            size: 1024 * 1024,
            alloc_type: 0,
        });
        let display_str = format!("{}", cmd2);
        assert!(display_str.contains("AllocMemory"));
        assert!(display_str.contains("1048576B"));
    }

    #[test]
    fn test_command_queue_stats() {
        let crew = [1u8; 16];
        let mut queue = CommandQueue::new(100, crew);

        assert_eq!(queue.stats.submitted, 0);
        assert_eq!(queue.stats.completed, 0);
        assert_eq!(queue.stats.failed, 0);

        let cmd = GpuCommand::Synchronize(StreamHandle(0));
        queue.submit(cmd.clone(), crew, 10, 1000).unwrap();
        queue.submit(cmd, crew, 10, 2000).unwrap();

        assert_eq!(queue.stats.submitted, 2);

        let _ = queue.flush();

        assert_eq!(queue.stats.completed, 2);
    }
}
