// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Watchdog Timer and Loop Detection
//!
//! This module implements deadline enforcement and infinite loop detection for
//! cognitive task execution. The watchdog monitors task progress and triggers
//! recovery actions when deadlines are exceeded or suspicious patterns detected.
//!
//! ## Watchdog Configuration
//!
//! Key parameters:
//! - **deadline_ms**: Hard deadline for task completion
//! - **max_phase_iterations**: Maximum iterations per phase (prevents infinite loops)
//! - **tool_retry_limit**: Maximum retries per tool call
//! - **loop_detection_threshold**: Iteration count threshold for loop detection
//!
//! ## Watchdog State Machine
//!
//! - **Active**: Normal operation
//! - **TriggeredWarning**: Deadline approaching, continue with caution
//! - **TriggeredTermination**: Deadline exceeded or loop detected, force termination
//! - **Disabled**: Watchdog temporarily disabled
//!
//! ## References
//!
//! - Engineering Plan § 6.5 (Deadline & Loop Management)

use serde::{Deserialize, Serialize};

/// Watchdog configuration parameters.
///
/// Defines timeout and loop detection thresholds for task execution.
///
/// See Engineering Plan § 6.5 (Deadline & Loop Management)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchdogConfig {
    /// Absolute deadline in milliseconds (task must complete before this time).
    /// Default: 30000 (30 seconds)
    pub deadline_ms: u64,

    /// Maximum iterations allowed per phase (prevents infinite loops).
    /// Default: 10
    pub max_phase_iterations: u32,

    /// Maximum retry attempts per tool call.
    /// Default: 3
    pub tool_retry_limit: u32,

    /// Iteration count threshold for triggering loop detection.
    /// Default: 5 (if iterations exceed this without visible progress)
    pub loop_detection_threshold: u32,
}

impl WatchdogConfig {
    /// Create a new watchdog configuration with default parameters.
    ///
    /// Defaults:
    /// - deadline_ms: 30000 (30 seconds)
    /// - max_phase_iterations: 10
    /// - tool_retry_limit: 3
    /// - loop_detection_threshold: 5
    pub fn new() -> Self {
        Self {
            deadline_ms: 30000,
            max_phase_iterations: 10,
            tool_retry_limit: 3,
            loop_detection_threshold: 5,
        }
    }

    /// Create a watchdog configuration with a custom deadline.
    pub fn with_deadline(deadline_ms: u64) -> Self {
        Self {
            deadline_ms,
            ..Self::new()
        }
    }

    /// Validate the configuration for consistency.
    ///
    /// Returns Ok if the configuration is valid, Err with a message otherwise.
    pub fn validate(&self) -> Result<(), String> {
        if self.deadline_ms == 0 {
            return Err(alloc::string::String::from("deadline_ms must be > 0"));
        }
        if self.max_phase_iterations == 0 {
            return Err(alloc::string::String::from(
                "max_phase_iterations must be > 0",
            ));
        }
        if self.tool_retry_limit == 0 {
            return Err(alloc::string::String::from("tool_retry_limit must be > 0"));
        }
        if self.loop_detection_threshold == 0 {
            return Err(alloc::string::String::from(
                "loop_detection_threshold must be > 0",
            ));
        }
        Ok(())
    }
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Watchdog state indicating current operational status.
///
/// See Engineering Plan § 6.5 (Deadline & Loop Management)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WatchdogState {
    /// Watchdog is active and monitoring.
    Active,

    /// Deadline approaching - warning issued, task continues with caution.
    TriggeredWarning,

    /// Deadline exceeded or loop detected - force termination.
    TriggeredTermination,

    /// Watchdog temporarily disabled (e.g., during critical operations).
    Disabled,
}

impl WatchdogState {
    /// Check if the watchdog is currently monitoring.
    pub fn is_active(&self) -> bool {
        matches!(self, WatchdogState::Active | WatchdogState::TriggeredWarning)
    }

    /// Check if the watchdog has triggered termination.
    pub fn should_terminate(&self) -> bool {
        matches!(self, WatchdogState::TriggeredTermination)
    }
}

/// Watchdog events indicating conditions that require action.
///
/// See Engineering Plan § 6.5 (Deadline & Loop Management)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WatchdogEvent {
    /// Deadline is approaching (80% or more of deadline consumed).
    DeadlineApproaching,

    /// Deadline has been exceeded - immediate termination required.
    DeadlineExceeded,

    /// Infinite loop detected based on iteration count and progress metrics.
    LoopDetected,

    /// Maximum iterations for a phase reached.
    IterationLimitReached,
}

impl WatchdogEvent {
    /// Check if this event is critical and requires immediate action.
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            WatchdogEvent::DeadlineExceeded | WatchdogEvent::LoopDetected
        )
    }
}

/// Watchdog monitor for deadline and loop detection.
///
/// Tracks task execution time and iteration counts, triggering recovery
/// when deadlines are exceeded or infinite loops are detected.
///
/// See Engineering Plan § 6.5 (Deadline & Loop Management)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchdogMonitor {
    /// Configuration parameters
    pub config: WatchdogConfig,

    /// Current watchdog state
    pub state: WatchdogState,

    /// Start time of task execution (Unix epoch milliseconds)
    pub start_time_ms: u64,

    /// Current iteration count in the active phase
    pub current_phase_iterations: u32,

    /// Iteration count at last progress checkpoint
    pub last_progress_iteration: u32,

    /// Number of consecutive iterations without visible progress
    pub iterations_without_progress: u32,

    /// Total tool call attempts in current phase
    pub tool_call_attempts: u32,
}

impl WatchdogMonitor {
    /// Create a new watchdog monitor.
    pub fn new(config: WatchdogConfig, start_time_ms: u64) -> Self {
        Self {
            config,
            state: WatchdogState::Active,
            start_time_ms,
            current_phase_iterations: 0,
            last_progress_iteration: 0,
            iterations_without_progress: 0,
            tool_call_attempts: 0,
        }
    }

    /// Check the watchdog and return any triggered events.
    ///
    /// Should be called periodically during task execution.
    pub fn check(&mut self, current_time_ms: u64) -> Option<WatchdogEvent> {
        if self.state == WatchdogState::Disabled {
            return None;
        }

        let elapsed_ms = current_time_ms.saturating_sub(self.start_time_ms);

        // Check deadline exceeded
        if elapsed_ms >= self.config.deadline_ms {
            self.state = WatchdogState::TriggeredTermination;
            return Some(WatchdogEvent::DeadlineExceeded);
        }

        // Check deadline approaching (80% of deadline)
        let threshold_80_percent = (self.config.deadline_ms * 80) / 100;
        if elapsed_ms >= threshold_80_percent && self.state != WatchdogState::TriggeredWarning {
            self.state = WatchdogState::TriggeredWarning;
            return Some(WatchdogEvent::DeadlineApproaching);
        }

        // Check phase iteration limit
        if self.current_phase_iterations >= self.config.max_phase_iterations {
            self.state = WatchdogState::TriggeredTermination;
            return Some(WatchdogEvent::IterationLimitReached);
        }

        // Check for infinite loops
        if self.iterations_without_progress >= self.config.loop_detection_threshold {
            self.state = WatchdogState::TriggeredTermination;
            return Some(WatchdogEvent::LoopDetected);
        }

        None
    }

    /// Record an iteration in the current phase.
    pub fn record_iteration(&mut self) {
        self.current_phase_iterations += 1;
        self.iterations_without_progress += 1;
    }

    /// Record progress in the current phase (resets no-progress counter).
    pub fn record_progress(&mut self) {
        self.last_progress_iteration = self.current_phase_iterations;
        self.iterations_without_progress = 0;
    }

    /// Record a tool call attempt.
    pub fn record_tool_call(&mut self) {
        self.tool_call_attempts += 1;
    }

    /// Reset the phase counters when transitioning to a new phase.
    pub fn reset_phase(&mut self) {
        self.current_phase_iterations = 0;
        self.last_progress_iteration = 0;
        self.iterations_without_progress = 0;
        self.tool_call_attempts = 0;
    }

    /// Get the elapsed time since task start (milliseconds).
    pub fn elapsed_ms(&self, current_time_ms: u64) -> u64 {
        current_time_ms.saturating_sub(self.start_time_ms)
    }

    /// Get the remaining time before deadline (milliseconds).
    /// Returns 0 if deadline has been exceeded.
    pub fn remaining_ms(&self, current_time_ms: u64) -> u64 {
        let elapsed = self.elapsed_ms(current_time_ms);
        if elapsed >= self.config.deadline_ms {
            0
        } else {
            self.config.deadline_ms - elapsed
        }
    }

    /// Check if tool retry limit has been exceeded.
    pub fn tool_retry_limit_exceeded(&self) -> bool {
        self.tool_call_attempts > self.config.tool_retry_limit
    }

    /// Enable or disable the watchdog (for critical sections).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.state = if enabled {
            WatchdogState::Active
        } else {
            WatchdogState::Disabled
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;

    // ============================================================================
    // Watchdog Configuration Tests
    // ============================================================================

    #[test]
    fn test_watchdog_config_new() {
        let config = WatchdogConfig::new();
        assert_eq!(config.deadline_ms, 30000);
        assert_eq!(config.max_phase_iterations, 10);
        assert_eq!(config.tool_retry_limit, 3);
        assert_eq!(config.loop_detection_threshold, 5);
    }

    #[test]
    fn test_watchdog_config_with_deadline() {
        let config = WatchdogConfig::with_deadline(60000);
        assert_eq!(config.deadline_ms, 60000);
        assert_eq!(config.max_phase_iterations, 10);
    }

    #[test]
    fn test_watchdog_config_validate() {
        let config = WatchdogConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_watchdog_config_validate_zero_deadline() {
        let mut config = WatchdogConfig::new();
        config.deadline_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_watchdog_config_validate_zero_iterations() {
        let mut config = WatchdogConfig::new();
        config.max_phase_iterations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_watchdog_config_default() {
        let config = WatchdogConfig::default();
        assert_eq!(config.deadline_ms, 30000);
    }

    // ============================================================================
    // Watchdog State Tests
    // ============================================================================

    #[test]
    fn test_watchdog_state_active_is_active() {
        assert!(WatchdogState::Active.is_active());
    }

    #[test]
    fn test_watchdog_state_warning_is_active() {
        assert!(WatchdogState::TriggeredWarning.is_active());
    }

    #[test]
    fn test_watchdog_state_termination_not_active() {
        assert!(!WatchdogState::TriggeredTermination.is_active());
    }

    #[test]
    fn test_watchdog_state_disabled_not_active() {
        assert!(!WatchdogState::Disabled.is_active());
    }

    #[test]
    fn test_watchdog_state_should_terminate() {
        assert!(WatchdogState::TriggeredTermination.should_terminate());
        assert!(!WatchdogState::Active.should_terminate());
    }

    // ============================================================================
    // Watchdog Event Tests
    // ============================================================================

    #[test]
    fn test_watchdog_event_deadline_exceeded_critical() {
        assert!(WatchdogEvent::DeadlineExceeded.is_critical());
    }

    #[test]
    fn test_watchdog_event_loop_detected_critical() {
        assert!(WatchdogEvent::LoopDetected.is_critical());
    }

    #[test]
    fn test_watchdog_event_approaching_not_critical() {
        assert!(!WatchdogEvent::DeadlineApproaching.is_critical());
    }

    // ============================================================================
    // Watchdog Monitor Tests
    // ============================================================================

    #[test]
    fn test_watchdog_monitor_new() {
        let config = WatchdogConfig::new();
        let monitor = WatchdogMonitor::new(config, 1000);
        assert_eq!(monitor.state, WatchdogState::Active);
        assert_eq!(monitor.current_phase_iterations, 0);
        assert_eq!(monitor.start_time_ms, 1000);
    }

    #[test]
    fn test_watchdog_monitor_elapsed_ms() {
        let config = WatchdogConfig::new();
        let monitor = WatchdogMonitor::new(config, 1000);
        let elapsed = monitor.elapsed_ms(3000);
        assert_eq!(elapsed, 2000);
    }

    #[test]
    fn test_watchdog_monitor_remaining_ms() {
        let config = WatchdogConfig::new();
        let monitor = WatchdogMonitor::new(config, 1000);
        // deadline is 30000 ms from 1000 = 31000
        let remaining = monitor.remaining_ms(11000);
        assert_eq!(remaining, 20000);
    }

    #[test]
    fn test_watchdog_monitor_remaining_ms_exceeded() {
        let config = WatchdogConfig::new();
        let monitor = WatchdogMonitor::new(config, 1000);
        let remaining = monitor.remaining_ms(35000);
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_watchdog_monitor_check_active() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        let event = monitor.check(5000);
        assert!(event.is_none());
        assert_eq!(monitor.state, WatchdogState::Active);
    }

    #[test]
    fn test_watchdog_monitor_check_deadline_exceeded() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        let event = monitor.check(35000);
        assert_eq!(event, Some(WatchdogEvent::DeadlineExceeded));
        assert_eq!(monitor.state, WatchdogState::TriggeredTermination);
    }

    #[test]
    fn test_watchdog_monitor_check_deadline_approaching() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        // 80% of 30000 is 24000, so deadline approaches at 1000 + 24000 = 25000
        let event = monitor.check(26000);
        assert_eq!(event, Some(WatchdogEvent::DeadlineApproaching));
        assert_eq!(monitor.state, WatchdogState::TriggeredWarning);
    }

    #[test]
    fn test_watchdog_monitor_check_iteration_limit() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.current_phase_iterations = 10;
        let event = monitor.check(5000);
        assert_eq!(event, Some(WatchdogEvent::IterationLimitReached));
        assert_eq!(monitor.state, WatchdogState::TriggeredTermination);
    }

    #[test]
    fn test_watchdog_monitor_check_loop_detected() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.iterations_without_progress = 5;
        let event = monitor.check(5000);
        assert_eq!(event, Some(WatchdogEvent::LoopDetected));
        assert_eq!(monitor.state, WatchdogState::TriggeredTermination);
    }

    #[test]
    fn test_watchdog_monitor_record_iteration() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.record_iteration();
        assert_eq!(monitor.current_phase_iterations, 1);
        assert_eq!(monitor.iterations_without_progress, 1);
    }

    #[test]
    fn test_watchdog_monitor_record_progress() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.record_iteration();
        monitor.record_iteration();
        assert_eq!(monitor.iterations_without_progress, 2);
        monitor.record_progress();
        assert_eq!(monitor.iterations_without_progress, 0);
        assert_eq!(monitor.last_progress_iteration, 2);
    }

    #[test]
    fn test_watchdog_monitor_record_tool_call() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.record_tool_call();
        assert_eq!(monitor.tool_call_attempts, 1);
    }

    #[test]
    fn test_watchdog_monitor_tool_retry_limit_exceeded() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        for _ in 0..4 {
            monitor.record_tool_call();
        }
        assert!(monitor.tool_retry_limit_exceeded());
    }

    #[test]
    fn test_watchdog_monitor_reset_phase() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.record_iteration();
        monitor.record_tool_call();
        monitor.reset_phase();
        assert_eq!(monitor.current_phase_iterations, 0);
        assert_eq!(monitor.tool_call_attempts, 0);
    }

    #[test]
    fn test_watchdog_monitor_set_enabled() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        assert_eq!(monitor.state, WatchdogState::Active);
        monitor.set_enabled(false);
        assert_eq!(monitor.state, WatchdogState::Disabled);
        monitor.set_enabled(true);
        assert_eq!(monitor.state, WatchdogState::Active);
    }

    #[test]
    fn test_watchdog_monitor_disabled_skips_checks() {
        let config = WatchdogConfig::new();
        let mut monitor = WatchdogMonitor::new(config, 1000);
        monitor.set_enabled(false);
        let event = monitor.check(35000); // Well past deadline
        assert!(event.is_none()); // No event when disabled
    }
}
