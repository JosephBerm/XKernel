// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Asynchronous GPU kernel execution model via CUDA/HIP events.
//!
//! Implements async execution with completion notifications using CUDA/HIP events
//! (cuEventRecord/hipEventRecord) and stream monitoring (cuStreamQuery/hipStreamQuery).
//! Enables Cognitive Scheduler to track kernel completion and resume waiting CTs
//! (Computational Threads) without blocking.
//!
//! ## Async Execution Model
//!
//! ```
//! Framework Submits Kernel
//!     ↓ (async)
//! GPU Manager enqueues to stream
//!     ↓ (async)
//! GPU executes kernel
//!     ↓ (event record on stream)
//! cuEventRecord / hipEventRecord
//!     ↓ (poll: cuStreamQuery / hipStreamQuery)
//! Completion detected
//!     ↓
//! Event callback / Scheduler notification
//!     ↓
//! Resume waiting CT
//! ```
//!
//! Reference: Engineering Plan § Async Execution Model, Week 5 Addendum v2.5.1

use crate::command_queue::SubmissionId;
use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// GPU event for kernel completion tracking.
///
/// Represents a synchronization point on a GPU stream.
/// Can be polled (cuStreamQuery) or waited on (cuEventSynchronize).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuEventHandle {
    /// Opaque event handle from CUDA/HIP driver.
    ///
    /// - CUDA: CUevent cast to u64
    /// - HIP: hipEvent_t cast to u64
    pub event_handle: u64,

    /// Associated submission ID for tracking.
    pub submission_id: SubmissionId,

    /// GPU stream on which event was recorded.
    pub stream_handle: u64,
}

impl fmt::Display for GpuEventHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuEventHandle({}, stream=0x{:x})",
            self.submission_id, self.stream_handle
        )
    }
}

/// Status of a GPU event (completion tracking).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStatus {
    /// Event recorded but kernel still executing.
    Pending,

    /// Kernel completed, event ready.
    Completed,

    /// Error during kernel execution.
    Error,
}

impl fmt::Display for EventStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventStatus::Pending => write!(f, "Pending"),
            EventStatus::Completed => write!(f, "Completed"),
            EventStatus::Error => write!(f, "Error"),
        }
    }
}

/// Event completion record with timing information.
#[derive(Clone, Copy, Debug)]
pub struct EventCompletion {
    /// Event handle.
    pub event: GpuEventHandle,

    /// Event status.
    pub status: EventStatus,

    /// Elapsed time in nanoseconds (kernel runtime).
    pub elapsed_ns: u64,

    /// Timestamp when completion was detected.
    pub completed_at_ns: u64,

    /// Error code if status == Error.
    pub error_code: u32,
}

impl fmt::Display for EventCompletion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EventCompletion({}, status={}, elapsed={}ns)",
            self.event, self.status, self.elapsed_ns
        )
    }
}

/// Callback invoked when a kernel completes.
///
/// Called by AsyncExecutionManager when an event is detected as completed.
/// Enables Cognitive Scheduler to resume waiting CTs and perform cleanup.
pub type CompletionCallback = fn(&EventCompletion) -> Result<(), GpuError>;

/// Async execution manager for GPU kernels.
///
/// Tracks in-flight kernels via GPU events, polls for completion, and invokes
/// callbacks when kernels finish. Enables non-blocking kernel execution with
/// efficient completion detection.
///
/// **Key Operations:**
/// - Record event on stream after kernel launch
/// - Poll streams for completion (cuStreamQuery/hipStreamQuery)
/// - Detect completion and invoke callbacks
/// - Support multiple concurrent streams per crew
///
/// Reference: Engineering Plan § Async Execution Manager
#[derive(Debug)]
pub struct AsyncExecutionManager {
    /// In-flight events tracking submissions to completions.
    /// Mapping: SubmissionId → EventCompletion (updated as events complete)
    events: BTreeMap<SubmissionId, EventCompletion>,

    /// Registered completion callbacks (submission_id → callback).
    callbacks: BTreeMap<SubmissionId, CompletionCallback>,

    /// Stream handles for polling (stream_handle → crew_id).
    active_streams: BTreeMap<u64, [u8; 16]>,

    /// Statistics.
    pub stats: AsyncExecutionStats,
}

/// Statistics for async execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsyncExecutionStats {
    /// Total events recorded.
    pub total_events_recorded: u64,

    /// Total events completed.
    pub total_events_completed: u64,

    /// Total completion callbacks invoked.
    pub total_callbacks_invoked: u64,

    /// Total completion timeouts.
    pub total_timeouts: u64,

    /// Average event completion latency in nanoseconds.
    pub avg_completion_latency_ns: u64,
}

impl AsyncExecutionManager {
    /// Create a new async execution manager.
    pub fn new() -> Self {
        AsyncExecutionManager {
            events: BTreeMap::new(),
            callbacks: BTreeMap::new(),
            active_streams: BTreeMap::new(),
            stats: AsyncExecutionStats {
                total_events_recorded: 0,
                total_events_completed: 0,
                total_callbacks_invoked: 0,
                total_timeouts: 0,
                avg_completion_latency_ns: 0,
            },
        }
    }

    /// Record an event for a submitted kernel.
    ///
    /// Called after cuLaunchKernel/hipModuleLaunchKernel to track completion
    /// via GPU event. The event is polled periodically for completion status.
    ///
    /// # Arguments
    ///
    /// * `event_handle` - CUDA/HIP event handle (opaque)
    /// * `submission_id` - Submission ID for tracking
    /// * `stream_handle` - GPU stream handle
    /// * `recorded_at_ns` - Timestamp when event was recorded
    ///
    /// # Returns
    ///
    /// Ok(()) if event recorded successfully.
    pub fn record_event(
        &mut self,
        event_handle: u64,
        submission_id: SubmissionId,
        stream_handle: u64,
        recorded_at_ns: u64,
        crew_id: [u8; 16],
    ) -> Result<(), GpuError> {
        let event = GpuEventHandle {
            event_handle,
            submission_id,
            stream_handle,
        };

        let completion = EventCompletion {
            event,
            status: EventStatus::Pending,
            elapsed_ns: 0,
            completed_at_ns: 0,
            error_code: 0,
        };

        self.events.insert(submission_id, completion);
        self.active_streams.insert(stream_handle, crew_id);

        self.stats.total_events_recorded += 1;

        Ok(())
    }

    /// Poll for event completion (cuStreamQuery / hipStreamQuery).
    ///
    /// Checks a stream for completion of pending events. Updates event status
    /// and invokes registered callbacks when kernels complete.
    ///
    /// **Note:** In real implementation, this would call:
    /// - CUDA: cuStreamQuery(stream) to check if all work is complete
    /// - HIP: hipStreamQuery(stream)
    ///
    /// # Arguments
    ///
    /// * `stream_handle` - Stream to poll
    /// * `current_time_ns` - Current timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// Vector of completed events for this stream.
    pub fn poll_stream(
        &mut self,
        stream_handle: u64,
        current_time_ns: u64,
    ) -> Result<Vec<EventCompletion>, GpuError> {
        let mut completions = Vec::new();

        // Find all events on this stream
        let submissions_to_check: Vec<SubmissionId> = self
            .events
            .iter()
            .filter(|(_, ev)| ev.event.stream_handle == stream_handle && ev.status == EventStatus::Pending)
            .map(|(sid, _)| *sid)
            .collect();

        for submission_id in submissions_to_check {
            if let Some(completion) = self.events.get_mut(&submission_id) {
                // In a real implementation, this would call cuStreamQuery / hipStreamQuery
                // For now, we simulate completion detection
                if completion.status == EventStatus::Pending {
                    // Mark as completed (in real code, check actual GPU status)
                    completion.status = EventStatus::Completed;
                    completion.completed_at_ns = current_time_ns;

                    // Update statistics
                    let latency = current_time_ns.saturating_sub(completion.completed_at_ns);
                    self.stats.total_events_completed += 1;
                    self.stats.avg_completion_latency_ns = (self.stats.avg_completion_latency_ns
                        * (self.stats.total_events_completed - 1)
                        + latency)
                        / self.stats.total_events_completed;

                    completions.push(*completion);

                    // Invoke callback if registered
                    if let Some(callback) = self.callbacks.get(&submission_id) {
                        callback(completion)?;
                        self.stats.total_callbacks_invoked += 1;
                    }
                }
            }
        }

        Ok(completions)
    }

    /// Register a callback for a submission.
    ///
    /// Callback will be invoked when the submission's event completes.
    ///
    /// # Arguments
    ///
    /// * `submission_id` - Submission to track
    /// * `callback` - Callback function
    pub fn register_callback(
        &mut self,
        submission_id: SubmissionId,
        callback: CompletionCallback,
    ) -> Result<(), GpuError> {
        self.callbacks.insert(submission_id, callback);
        Ok(())
    }

    /// Wait for a specific event to complete (cuEventSynchronize / hipEventSynchronize).
    ///
    /// Blocks until the event completes or timeout expires.
    /// Used when caller needs to ensure kernel completion before proceeding.
    ///
    /// **Note:** In real implementation, this would call:
    /// - CUDA: cuEventSynchronize(event) — blocks CPU until GPU event ready
    /// - HIP: hipEventSynchronize(event)
    ///
    /// # Arguments
    ///
    /// * `submission_id` - Submission to wait for
    /// * `timeout_ns` - Timeout in nanoseconds (0 = no timeout)
    ///
    /// # Returns
    ///
    /// EventCompletion if event completed, GpuError on timeout/error.
    pub fn wait_event(
        &mut self,
        submission_id: SubmissionId,
        _timeout_ns: u64,
    ) -> Result<EventCompletion, GpuError> {
        self.events
            .get(&submission_id)
            .copied()
            .ok_or(GpuError::DriverError)
    }

    /// Get event status without blocking.
    pub fn get_event_status(&self, submission_id: SubmissionId) -> Result<EventStatus, GpuError> {
        self.events
            .get(&submission_id)
            .map(|ev| ev.status)
            .ok_or(GpuError::DriverError)
    }

    /// Remove a completed event from tracking.
    pub fn clear_event(&mut self, submission_id: SubmissionId) -> Result<(), GpuError> {
        self.events
            .remove(&submission_id)
            .ok_or(GpuError::DriverError)?;
        self.callbacks.remove(&submission_id);
        Ok(())
    }

    /// Get the number of pending events.
    pub fn pending_event_count(&self) -> usize {
        self.events
            .values()
            .filter(|ev| ev.status == EventStatus::Pending)
            .count()
    }

    /// Get the number of completed events.
    pub fn completed_event_count(&self) -> usize {
        self.events
            .values()
            .filter(|ev| ev.status == EventStatus::Completed)
            .count()
    }

    /// Poll all active streams for completion.
    ///
    /// Called periodically (e.g., every 1ms) to detect kernel completions
    /// and invoke callbacks. Returns vector of all completed events.
    pub fn poll_all_streams(&mut self, current_time_ns: u64) -> Result<Vec<EventCompletion>, GpuError> {
        let mut all_completions = Vec::new();

        // Get list of active streams to avoid borrow checker issues
        let streams: Vec<u64> = self.active_streams.keys().copied().collect();

        for stream_handle in streams {
            match self.poll_stream(stream_handle, current_time_ns) {
                Ok(completions) => all_completions.extend(completions),
                Err(e) => return Err(e),
            }
        }

        Ok(all_completions)
    }

    /// Get async execution statistics.
    pub fn stats(&self) -> AsyncExecutionStats {
        self.stats
    }

    /// Clear all events and callbacks (used on shutdown).
    pub fn clear_all(&mut self) {
        self.events.clear();
        self.callbacks.clear();
        self.active_streams.clear();
    }
}

impl Default for AsyncExecutionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_async_execution_manager_creation() {
        let manager = AsyncExecutionManager::new();
        assert_eq!(manager.pending_event_count(), 0);
        assert_eq!(manager.completed_event_count(), 0);
    }

    #[test]
    fn test_record_event() {
        let mut manager = AsyncExecutionManager::new();

        let result = manager.record_event(0x1234, SubmissionId::new(1), 0x5678, 1000, [1u8; 16]);
        assert!(result.is_ok());
        assert_eq!(manager.pending_event_count(), 1);
        assert_eq!(manager.stats.total_events_recorded, 1);
    }

    #[test]
    fn test_get_event_status() {
        let mut manager = AsyncExecutionManager::new();

        manager
            .record_event(0x1234, SubmissionId::new(1), 0x5678, 1000, [1u8; 16])
            .unwrap();

        let status = manager.get_event_status(SubmissionId::new(1));
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), EventStatus::Pending);
    }

    #[test]
    fn test_poll_stream() {
        let mut manager = AsyncExecutionManager::new();

        manager
            .record_event(0x1234, SubmissionId::new(1), 0x5678, 1000, [1u8; 16])
            .unwrap();

        let completions = manager.poll_stream(0x5678, 2000);
        assert!(completions.is_ok());
        assert_eq!(completions.unwrap().len(), 1);
    }

    #[test]
    fn test_register_callback() {
        let mut manager = AsyncExecutionManager::new();

        fn dummy_callback(_: &EventCompletion) -> Result<(), GpuError> {
            Ok(())
        }

        let result = manager.register_callback(SubmissionId::new(1), dummy_callback);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wait_event() {
        let mut manager = AsyncExecutionManager::new();

        manager
            .record_event(0x1234, SubmissionId::new(1), 0x5678, 1000, [1u8; 16])
            .unwrap();

        let result = manager.wait_event(SubmissionId::new(1), 5000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_event() {
        let mut manager = AsyncExecutionManager::new();

        manager
            .record_event(0x1234, SubmissionId::new(1), 0x5678, 1000, [1u8; 16])
            .unwrap();

        assert_eq!(manager.pending_event_count(), 1);

        let result = manager.clear_event(SubmissionId::new(1));
        assert!(result.is_ok());
        assert_eq!(manager.pending_event_count(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = AsyncExecutionManager::new();

        for i in 0..5 {
            manager
                .record_event(0x1000 + i, SubmissionId::new(i as u64), 0x5678, 1000 + i, [1u8; 16])
                .unwrap();
        }

        assert_eq!(manager.pending_event_count(), 5);
        manager.clear_all();
        assert_eq!(manager.pending_event_count(), 0);
    }

    #[test]
    fn test_event_completion_display() {
        let completion = EventCompletion {
            event: GpuEventHandle {
                event_handle: 0x1234,
                submission_id: SubmissionId::new(1),
                stream_handle: 0x5678,
            },
            status: EventStatus::Completed,
            elapsed_ns: 1500,
            completed_at_ns: 2500,
            error_code: 0,
        };

        let display = format!("{}", completion);
        assert!(display.contains("EventCompletion"));
        assert!(display.contains("elapsed=1500ns"));
    }

    #[test]
    fn test_event_status_display() {
        assert_eq!(format!("{}", EventStatus::Pending), "Pending");
        assert_eq!(format!("{}", EventStatus::Completed), "Completed");
        assert_eq!(format!("{}", EventStatus::Error), "Error");
    }

    #[test]
    fn test_poll_all_streams() {
        let mut manager = AsyncExecutionManager::new();

        for i in 0..3 {
            manager
                .record_event(0x1000 + i, SubmissionId::new(i as u64), 0x5678, 1000, [1u8; 16])
                .unwrap();
        }

        let completions = manager.poll_all_streams(2000);
        assert!(completions.is_ok());
        assert_eq!(completions.unwrap().len(), 3);
    }
}
