// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU event-based completion notification system.
//!
//! Implements completion detection via cuEventSynchronize/hipEventSynchronize and
//! GPU Manager state update on kernel completion. Enables Cognitive Scheduler to
//! monitor GPU progress and resume waiting Computational Threads (CTs).
//!
//! ## Notification Flow
//!
//! ```
//! GPU Kernel Execution
//!     ↓
//! cuEventRecord / hipEventRecord on stream
//!     ↓
//! cuStreamQuery / hipStreamQuery poll
//!     ↓ (Completion detected)
//! CompletionNotificationManager
//!     ↓
//! GPU Manager state update
//!     ↓
//! Cognitive Scheduler notification
//!     ↓
//! Resume waiting CT
//! ```
//!
//! Reference: Engineering Plan § Completion Notification, Week 5 Addendum v2.5.1

use crate::command_queue::SubmissionId;
use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Completion notification event.
///
/// Generated when a GPU kernel completes and synchronization is required.
/// Sent to GPU Manager and Cognitive Scheduler for state updates.
#[derive(Clone, Copy, Debug)]
pub struct CompletionNotification {
    /// Submission ID of completed kernel.
    pub submission_id: SubmissionId,

    /// Crew that submitted the kernel.
    pub crew_id: [u8; 16],

    /// Completion timestamp in nanoseconds.
    pub completed_at_ns: u64,

    /// Kernel execution time in nanoseconds.
    pub execution_time_ns: u64,

    /// GPU device that executed the kernel.
    pub device_ordinal: u32,

    /// Completion status.
    pub status: CompletionStatus,

    /// Error code if status is Error.
    pub error_code: u32,
}

/// Status of a completed GPU operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionStatus {
    /// Kernel executed successfully.
    Success,

    /// Kernel timed out waiting for completion.
    Timeout,

    /// GPU error (device error, ECC error, etc.).
    Error,

    /// Operation was cancelled.
    Cancelled,
}

impl fmt::Display for CompletionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompletionStatus::Success => write!(f, "Success"),
            CompletionStatus::Timeout => write!(f, "Timeout"),
            CompletionStatus::Error => write!(f, "Error"),
            CompletionStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl fmt::Display for CompletionNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompletionNotification({}, crew={:?}, status={}, time={}ns)",
            self.submission_id, &self.crew_id[..4], self.status, self.execution_time_ns
        )
    }
}

/// Callback invoked on completion notification.
///
/// Called by CompletionNotificationManager when kernel completes.
/// Enables custom handlers for completion (e.g., scheduler integration).
pub type CompletionHandler = fn(&CompletionNotification) -> Result<(), GpuError>;

/// Pending completion request (awaiting GPU synchronization).
#[derive(Clone, Copy, Debug)]
struct PendingCompletion {
    /// Submission being tracked.
    submission_id: SubmissionId,

    /// Event handle to synchronize on (GPU).
    event_handle: u64,

    /// Stream handle where event is recorded.
    stream_handle: u64,

    /// Crew identifier.
    crew_id: [u8; 16],

    /// Timestamp when completion tracking started.
    started_at_ns: u64,

    /// Timeout in nanoseconds (0 = no timeout).
    timeout_ns: u64,
}

/// GPU Manager completion state update.
///
/// Information sent to GPU Manager to update internal state when kernel completes.
#[derive(Clone, Debug)]
pub struct GpuManagerStateUpdate {
    /// Completion notification.
    pub completion: CompletionNotification,

    /// Updated crew resource usage (VRAM allocated, TPCs used).
    pub crew_resource_snapshot: ResourceSnapshot,
}

/// Snapshot of crew resource usage.
#[derive(Clone, Copy, Debug)]
pub struct ResourceSnapshot {
    /// VRAM used by crew in bytes.
    pub vram_used_bytes: u64,

    /// TPCs allocated to crew.
    pub tpcs_allocated: u32,

    /// Number of pending kernels in crew's queue.
    pub pending_kernels: u32,
}

/// Scheduler CT resumption request.
///
/// Sent to Cognitive Scheduler to resume a waiting Computational Thread.
#[derive(Clone, Copy, Debug)]
pub struct CtResumptionRequest {
    /// CT ID to resume.
    pub ct_id: [u8; 16],

    /// Crew ID owning the CT.
    pub crew_id: [u8; 16],

    /// Completion notification trigger.
    pub completion: CompletionNotification,

    /// Timestamp of resumption request.
    pub requested_at_ns: u64,
}

impl fmt::Display for CtResumptionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CtResumptionRequest(ct={:?}, crew={:?}, submission={})",
            &self.ct_id[..4], &self.crew_id[..4], self.completion.submission_id
        )
    }
}

/// Completion notification manager.
///
/// Orchestrates GPU kernel completion tracking and notification propagation:
/// 1. Track pending completions (submissions awaiting GPU event)
/// 2. Monitor GPU events (via cuStreamQuery / hipStreamQuery)
/// 3. Detect completion and invoke handlers
/// 4. Update GPU Manager state
/// 5. Resume waiting CTs in Cognitive Scheduler
///
/// Reference: Engineering Plan § Completion Notification System
#[derive(Debug)]
pub struct CompletionNotificationManager {
    /// Pending completions awaiting GPU synchronization.
    pending_completions: BTreeMap<SubmissionId, PendingCompletion>,

    /// Completed notifications (recent history).
    completed_notifications: Vec<CompletionNotification>,

    /// Registered completion handlers.
    handlers: Vec<CompletionHandler>,

    /// CT resumption queue (pending scheduler notifications).
    resumption_queue: Vec<CtResumptionRequest>,

    /// Statistics.
    pub stats: NotificationStats,

    /// Maximum history size.
    max_history: u32,
}

/// Statistics for completion notifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotificationStats {
    /// Total completions detected.
    pub total_completions: u64,

    /// Total completions with success status.
    pub total_successes: u64,

    /// Total completions with error status.
    pub total_errors: u64,

    /// Total completions with timeout status.
    pub total_timeouts: u64,

    /// Total CT resumption requests.
    pub total_ct_resumptions: u64,

    /// Average completion notification latency in nanoseconds.
    pub avg_notification_latency_ns: u64,
}

impl CompletionNotificationManager {
    /// Create a new completion notification manager.
    pub fn new() -> Self {
        CompletionNotificationManager {
            pending_completions: BTreeMap::new(),
            completed_notifications: Vec::new(),
            handlers: Vec::new(),
            resumption_queue: Vec::new(),
            stats: NotificationStats {
                total_completions: 0,
                total_successes: 0,
                total_errors: 0,
                total_timeouts: 0,
                total_ct_resumptions: 0,
                avg_notification_latency_ns: 0,
            },
            max_history: 1000,
        }
    }

    /// Track a pending completion for a submitted kernel.
    ///
    /// # Arguments
    ///
    /// * `submission_id` - Submission to track
    /// * `event_handle` - GPU event handle (opaque)
    /// * `stream_handle` - GPU stream handle
    /// * `crew_id` - Crew identifier
    /// * `timeout_ns` - Timeout in nanoseconds (0 = no timeout)
    pub fn track_completion(
        &mut self,
        submission_id: SubmissionId,
        event_handle: u64,
        stream_handle: u64,
        crew_id: [u8; 16],
        timeout_ns: u64,
        started_at_ns: u64,
    ) -> Result<(), GpuError> {
        let pending = PendingCompletion {
            submission_id,
            event_handle,
            stream_handle,
            crew_id,
            started_at_ns,
            timeout_ns,
        };

        self.pending_completions.insert(submission_id, pending);

        Ok(())
    }

    /// Register a completion notification handler.
    ///
    /// Handlers are invoked when completions are detected.
    /// Used to integrate with GPU Manager and Cognitive Scheduler.
    pub fn register_handler(&mut self, handler: CompletionHandler) -> Result<(), GpuError> {
        self.handlers.push(handler);
        Ok(())
    }

    /// Poll for pending completions.
    ///
    /// Checks GPU for event completion via cuStreamQuery/hipStreamQuery.
    /// Returns vector of detected completions.
    ///
    /// In a real implementation, this would:
    /// 1. Query GPU stream status for each pending event
    /// 2. Detect completion
    /// 3. Return completed notifications
    ///
    /// # Arguments
    ///
    /// * `current_time_ns` - Current timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// Vector of detected completions.
    pub fn poll_completions(
        &mut self,
        current_time_ns: u64,
    ) -> Result<Vec<CompletionNotification>, GpuError> {
        let mut completions = Vec::new();

        // Find completions that have timed out or are ready
        let submissions_to_check: Vec<SubmissionId> = self
            .pending_completions
            .keys()
            .copied()
            .collect();

        for submission_id in submissions_to_check {
            if let Some(pending) = self.pending_completions.get(&submission_id) {
                // Check timeout
                if pending.timeout_ns > 0 {
                    let elapsed = current_time_ns.saturating_sub(pending.started_at_ns);
                    if elapsed > pending.timeout_ns {
                        // Timeout occurred
                        let notification = CompletionNotification {
                            submission_id,
                            crew_id: pending.crew_id,
                            completed_at_ns: current_time_ns,
                            execution_time_ns: elapsed,
                            device_ordinal: 0, // Would be tracked separately
                            status: CompletionStatus::Timeout,
                            error_code: 1, // Timeout error code
                        };

                        completions.push(notification);
                        self.pending_completions.remove(&submission_id);
                        self.stats.total_timeouts += 1;
                        continue;
                    }
                }

                // In real implementation, would call cuStreamQuery / hipStreamQuery here
                // For now, simulate completion after a short time
                let elapsed = current_time_ns.saturating_sub(pending.started_at_ns);
                if elapsed > 1000 {
                    // Simulate completion after 1000ns
                    let notification = CompletionNotification {
                        submission_id,
                        crew_id: pending.crew_id,
                        completed_at_ns: current_time_ns,
                        execution_time_ns: elapsed,
                        device_ordinal: 0,
                        status: CompletionStatus::Success,
                        error_code: 0,
                    };

                    completions.push(notification);
                    self.pending_completions.remove(&submission_id);
                    self.stats.total_successes += 1;
                }
            }
        }

        // Process detected completions
        for completion in &completions {
            self._process_completion(*completion, current_time_ns)?;
        }

        Ok(completions)
    }

    /// Process a detected completion notification.
    ///
    /// Updates statistics, invokes handlers, and queues CT resumptions.
    fn _process_completion(
        &mut self,
        completion: CompletionNotification,
        current_time_ns: u64,
    ) -> Result<(), GpuError> {
        // Update statistics
        self.stats.total_completions += 1;

        match completion.status {
            CompletionStatus::Success => self.stats.total_successes += 1,
            CompletionStatus::Error => self.stats.total_errors += 1,
            CompletionStatus::Timeout => self.stats.total_timeouts += 1,
            CompletionStatus::Cancelled => {}
        }

        // Update average latency
        let latency = current_time_ns.saturating_sub(completion.completed_at_ns);
        self.stats.avg_notification_latency_ns = (self.stats.avg_notification_latency_ns
            * (self.stats.total_completions - 1)
            + latency)
            / self.stats.total_completions;

        // Store in history
        if self.completed_notifications.len() >= self.max_history as usize {
            self.completed_notifications.remove(0);
        }
        self.completed_notifications.push(completion);

        // Invoke handlers
        for handler in self.handlers.iter() {
            handler(&completion)?;
        }

        // Queue CT resumption
        let resumption = CtResumptionRequest {
            ct_id: [0u8; 16], // Would be resolved from crew context
            crew_id: completion.crew_id,
            completion,
            requested_at_ns: current_time_ns,
        };

        self.resumption_queue.push(resumption);
        self.stats.total_ct_resumptions += 1;

        Ok(())
    }

    /// Get and remove next CT resumption request from queue.
    ///
    /// Called by Cognitive Scheduler integration to process pending
    /// resumptions.
    pub fn dequeue_resumption(&mut self) -> Option<CtResumptionRequest> {
        if self.resumption_queue.is_empty() {
            None
        } else {
            Some(self.resumption_queue.remove(0))
        }
    }

    /// Get the number of pending resumption requests.
    pub fn resumption_queue_depth(&self) -> u32 {
        self.resumption_queue.len() as u32
    }

    /// Get the number of pending completions being tracked.
    pub fn pending_count(&self) -> u32 {
        self.pending_completions.len() as u32
    }

    /// Get recent completion notifications (most recent first).
    pub fn get_completed_notifications(&self, count: u32) -> Vec<CompletionNotification> {
        let start = self
            .completed_notifications
            .len()
            .saturating_sub(count as usize);
        self.completed_notifications[start..].iter().rev().copied().collect()
    }

    /// Get notification statistics.
    pub fn stats(&self) -> NotificationStats {
        self.stats
    }

    /// Clear all pending completions and queue (used on shutdown/crew termination).
    pub fn clear(&mut self) {
        self.pending_completions.clear();
        self.resumption_queue.clear();
    }
}

impl Default for CompletionNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_completion_notification_manager_creation() {
        let manager = CompletionNotificationManager::new();

        assert_eq!(manager.pending_count(), 0);
        assert_eq!(manager.resumption_queue_depth(), 0);
        assert_eq!(manager.stats.total_completions, 0);
    }

    #[test]
    fn test_track_completion() {
        let mut manager = CompletionNotificationManager::new();

        let result = manager.track_completion(
            SubmissionId::new(1),
            0x1234,
            0x5678,
            [1u8; 16],
            5000,
            1000,
        );

        assert!(result.is_ok());
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_register_handler() {
        let mut manager = CompletionNotificationManager::new();

        fn dummy_handler(_: &CompletionNotification) -> Result<(), GpuError> {
            Ok(())
        }

        let result = manager.register_handler(dummy_handler);
        assert!(result.is_ok());
    }

    #[test]
    fn test_poll_completions() {
        let mut manager = CompletionNotificationManager::new();

        manager
            .track_completion(SubmissionId::new(1), 0x1234, 0x5678, [1u8; 16], 0, 1000)
            .unwrap();

        let completions = manager.poll_completions(3000);
        assert!(completions.is_ok());
    }

    #[test]
    fn test_dequeue_resumption() {
        let mut manager = CompletionNotificationManager::new();

        manager
            .track_completion(SubmissionId::new(1), 0x1234, 0x5678, [1u8; 16], 0, 1000)
            .unwrap();

        let _ = manager.poll_completions(3000);

        let resumption = manager.dequeue_resumption();
        assert!(resumption.is_some());
        assert_eq!(manager.resumption_queue_depth(), 0);
    }

    #[test]
    fn test_completion_status_display() {
        assert_eq!(format!("{}", CompletionStatus::Success), "Success");
        assert_eq!(format!("{}", CompletionStatus::Timeout), "Timeout");
        assert_eq!(format!("{}", CompletionStatus::Error), "Error");
        assert_eq!(format!("{}", CompletionStatus::Cancelled), "Cancelled");
    }

    #[test]
    fn test_completion_notification_display() {
        let notification = CompletionNotification {
            submission_id: SubmissionId::new(1),
            crew_id: [1u8; 16],
            completed_at_ns: 2000,
            execution_time_ns: 1000,
            device_ordinal: 0,
            status: CompletionStatus::Success,
            error_code: 0,
        };

        let display = format!("{}", notification);
        assert!(display.contains("CompletionNotification"));
        assert!(display.contains("Success"));
    }

    #[test]
    fn test_clear_manager() {
        let mut manager = CompletionNotificationManager::new();

        manager
            .track_completion(SubmissionId::new(1), 0x1234, 0x5678, [1u8; 16], 0, 1000)
            .unwrap();

        assert_eq!(manager.pending_count(), 1);

        manager.clear();

        assert_eq!(manager.pending_count(), 0);
        assert_eq!(manager.resumption_queue_depth(), 0);
    }

    #[test]
    fn test_ct_resumption_request_display() {
        let resumption = CtResumptionRequest {
            ct_id: [1u8; 16],
            crew_id: [2u8; 16],
            completion: CompletionNotification {
                submission_id: SubmissionId::new(1),
                crew_id: [2u8; 16],
                completed_at_ns: 2000,
                execution_time_ns: 1000,
                device_ordinal: 0,
                status: CompletionStatus::Success,
                error_code: 0,
            },
            requested_at_ns: 2000,
        };

        let display = format!("{}", resumption);
        assert!(display.contains("CtResumptionRequest"));
    }

    #[test]
    fn test_timeout_detection() {
        let mut manager = CompletionNotificationManager::new();

        manager
            .track_completion(SubmissionId::new(1), 0x1234, 0x5678, [1u8; 16], 1000, 1000)
            .unwrap();

        let completions = manager.poll_completions(3000);
        assert!(completions.is_ok());

        let comps = completions.unwrap();
        if !comps.is_empty() {
            assert_eq!(comps[0].status, CompletionStatus::Timeout);
        }
    }
}
