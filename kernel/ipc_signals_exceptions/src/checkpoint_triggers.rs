// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Checkpoint Trigger Conditions
//!
//! This module defines the five trigger conditions that initiate checkpoint creation:
//! 1. **PhaseTransition**: On CT phase change (Observe→Act→Tool→Observe)
//! 2. **PeriodicTimer**: Every 60 seconds
//! 3. **PrePreemption**: Before scheduler context switch
//! 4. **ExplicitSignal**: SIG_CHECKPOINT received
//! 5. **ExceptionHandler**: Before invoking exception handler
//!
//! Each trigger is evaluated independently, and multiple triggers can fire simultaneously.
//! The checkpointing engine activates when any trigger condition is met.
//!
//! ## References
//!
//! - Engineering Plan § 6.3 (Checkpointing - Trigger Conditions)
//! - Week 6 Objective: Multiple trigger conditions

use crate::Result;
use cs_ct_lifecycle::CTPhase;

/// Checkpoint trigger type enumeration.
///
/// Represents the different conditions that can trigger checkpoint creation.
///
/// See Engineering Plan § 6.3 (Checkpointing - Trigger Conditions)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckpointTrigger {
    /// Triggered on CT phase transition
    ///
    /// When the CT transitions between phases (e.g., Reason → Act → Reflect),
    /// a checkpoint is created to capture the state at phase boundaries.
    /// This enables recovery to known, consistent phase boundaries.
    PhaseTransition,

    /// Triggered by periodic timer
    ///
    /// Every 60 seconds, a checkpoint is automatically created.
    /// This provides regular snapshots for long-running CTs and bounds
    /// the amount of work that must be redone on failure.
    PeriodicTimer,

    /// Triggered before preemption (context switch)
    ///
    /// Before the scheduler switches to a different CT, a checkpoint is created.
    /// This ensures the CT can be resumed from a consistent state.
    PrePreemption,

    /// Triggered by explicit signal
    ///
    /// When SIG_CHECKPOINT is received, checkpoint creation is triggered immediately.
    /// This allows manual/programmatic checkpoint control.
    ExplicitSignal,

    /// Triggered before exception handling
    ///
    /// Before invoking an exception handler, a checkpoint is created.
    /// This allows recovery to the pre-exception state if handling fails.
    ExceptionHandler,
}

impl CheckpointTrigger {
    /// Get human-readable name for this trigger.
    pub fn name(&self) -> &'static str {
        match self {
            CheckpointTrigger::PhaseTransition => "PhaseTransition",
            CheckpointTrigger::PeriodicTimer => "PeriodicTimer",
            CheckpointTrigger::PrePreemption => "PrePreemption",
            CheckpointTrigger::ExplicitSignal => "ExplicitSignal",
            CheckpointTrigger::ExceptionHandler => "ExceptionHandler",
        }
    }
}

/// Phase transition trigger for checkpoint creation.
///
/// Tracks the previous phase to detect phase transitions.
/// When a transition is detected, a checkpoint is triggered.
///
/// See Engineering Plan § 6.3 (Checkpointing - Phase Transition Trigger)
pub struct PhaseTransitionTrigger {
    /// The last phase we observed
    previous_phase: Option<CTPhase>,
    /// Whether a transition was detected
    transition_detected: bool,
}

impl PhaseTransitionTrigger {
    /// Create a new phase transition trigger.
    pub fn new() -> Self {
        Self {
            previous_phase: None,
            transition_detected: false,
        }
    }

    /// Check if the CT has transitioned to a new phase.
    ///
    /// # Arguments
    ///
    /// * `current_phase` - The current phase of the CT
    ///
    /// # Returns
    ///
    /// true if a phase transition occurred, false otherwise
    pub fn check_transition(&mut self, current_phase: CTPhase) -> bool {
        let transition = if let Some(prev) = self.previous_phase {
            prev != current_phase
        } else {
            false
        };

        self.previous_phase = Some(current_phase);
        self.transition_detected = transition;
        transition
    }

    /// Get the previous phase (if any).
    pub fn previous_phase(&self) -> Option<CTPhase> {
        self.previous_phase
    }

    /// Get the current phase (if any).
    pub fn current_phase(&self) -> Option<CTPhase> {
        self.previous_phase
    }

    /// Reset the trigger state.
    pub fn reset(&mut self) {
        self.transition_detected = false;
    }
}

/// Periodic timer trigger for checkpoint creation.
///
/// Tracks elapsed time and triggers checkpoint creation every 60 seconds.
/// Uses millisecond-precision timestamps.
///
/// See Engineering Plan § 6.3 (Checkpointing - Periodic Timer Trigger)
pub struct PeriodicTimerTrigger {
    /// Interval between checkpoints in milliseconds
    interval_ms: u64,
    /// Last checkpoint time (Unix epoch milliseconds)
    last_checkpoint_ms: u64,
    /// Whether a checkpoint is due
    checkpoint_due: bool,
}

impl PeriodicTimerTrigger {
    /// Create a new periodic timer trigger with default 60-second interval.
    pub fn new() -> Self {
        Self::with_interval(60000) // 60 seconds
    }

    /// Create a periodic timer trigger with custom interval.
    ///
    /// # Arguments
    ///
    /// * `interval_ms` - Checkpoint interval in milliseconds
    pub fn with_interval(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_checkpoint_ms: 0,
            checkpoint_due: false,
        }
    }

    /// Check if a checkpoint is due based on elapsed time.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time in Unix epoch milliseconds
    ///
    /// # Returns
    ///
    /// true if interval has elapsed since last checkpoint, false otherwise
    pub fn check_due(&mut self, current_time_ms: u64) -> bool {
        let elapsed = current_time_ms.saturating_sub(self.last_checkpoint_ms);
        self.checkpoint_due = elapsed >= self.interval_ms;

        if self.checkpoint_due {
            self.last_checkpoint_ms = current_time_ms;
        }

        self.checkpoint_due
    }

    /// Record a checkpoint and reset the timer.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_time_ms` - Time of the checkpoint in Unix epoch milliseconds
    pub fn record_checkpoint(&mut self, checkpoint_time_ms: u64) {
        self.last_checkpoint_ms = checkpoint_time_ms;
        self.checkpoint_due = false;
    }

    /// Get the interval.
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Get milliseconds until next checkpoint.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time in Unix epoch milliseconds
    ///
    /// # Returns
    ///
    /// Milliseconds until next checkpoint (0 if checkpoint is due)
    pub fn ms_until_next(&self, current_time_ms: u64) -> u64 {
        let elapsed = current_time_ms.saturating_sub(self.last_checkpoint_ms);
        self.interval_ms.saturating_sub(elapsed)
    }
}

/// Pre-preemption trigger for checkpoint creation.
///
/// Triggers checkpoint when the scheduler is about to context switch away from
/// the current CT. This ensures the CT state is consistent before yielding the CPU.
///
/// See Engineering Plan § 6.3 (Checkpointing - Pre-Preemption Trigger)
pub struct PrePreemptionTrigger {
    /// Whether a preemption is imminent
    preemption_imminent: bool,
}

impl PrePreemptionTrigger {
    /// Create a new pre-preemption trigger.
    pub fn new() -> Self {
        Self {
            preemption_imminent: false,
        }
    }

    /// Signal that preemption is about to occur.
    ///
    /// Called by the scheduler just before context switching away from the CT.
    ///
    /// # Returns
    ///
    /// true indicating checkpoint should be created before preemption
    pub fn signal_preemption_imminent(&mut self) -> bool {
        self.preemption_imminent = true;
        true
    }

    /// Reset the trigger state after checkpoint is created.
    pub fn reset(&mut self) {
        self.preemption_imminent = false;
    }

    /// Check if preemption is imminent.
    pub fn is_imminent(&self) -> bool {
        self.preemption_imminent
    }
}

/// Explicit signal trigger for checkpoint creation.
///
/// Triggers checkpoint when SIG_CHECKPOINT is delivered to the CT.
/// Allows programmatic/manual control of checkpoint creation.
///
/// See Engineering Plan § 6.3 (Checkpointing - Explicit Signal Trigger)
pub struct ExplicitSignalTrigger {
    /// Whether checkpoint signal was received
    checkpoint_signal_received: bool,
}

impl ExplicitSignalTrigger {
    /// Create a new explicit signal trigger.
    pub fn new() -> Self {
        Self {
            checkpoint_signal_received: false,
        }
    }

    /// Signal that SIG_CHECKPOINT was received.
    ///
    /// Called when the signal dispatch system delivers SIG_CHECKPOINT to the CT.
    ///
    /// # Returns
    ///
    /// true indicating checkpoint should be created
    pub fn signal_checkpoint_requested(&mut self) -> bool {
        self.checkpoint_signal_received = true;
        true
    }

    /// Reset the trigger state after checkpoint is created.
    pub fn reset(&mut self) {
        self.checkpoint_signal_received = false;
    }

    /// Check if checkpoint signal was received.
    pub fn is_requested(&self) -> bool {
        self.checkpoint_signal_received
    }
}

/// Exception handler trigger for checkpoint creation.
///
/// Triggers checkpoint before invoking an exception handler.
/// This allows recovery to the pre-exception state if exception handling fails.
///
/// See Engineering Plan § 6.3 (Checkpointing - Exception Handler Trigger)
pub struct ExceptionHandlerTrigger {
    /// Whether exception handling is imminent
    exception_handling_imminent: bool,
    /// Optional exception type/code for logging
    exception_code: Option<u32>,
}

impl ExceptionHandlerTrigger {
    /// Create a new exception handler trigger.
    pub fn new() -> Self {
        Self {
            exception_handling_imminent: false,
            exception_code: None,
        }
    }

    /// Signal that exception handling is about to occur.
    ///
    /// Called by the exception engine just before invoking the exception handler.
    ///
    /// # Arguments
    ///
    /// * `exception_code` - Optional code/type of the exception
    ///
    /// # Returns
    ///
    /// true indicating checkpoint should be created before exception handling
    pub fn signal_exception_handling(&mut self, exception_code: Option<u32>) -> bool {
        self.exception_handling_imminent = true;
        self.exception_code = exception_code;
        true
    }

    /// Reset the trigger state after checkpoint is created.
    pub fn reset(&mut self) {
        self.exception_handling_imminent = false;
        self.exception_code = None;
    }

    /// Check if exception handling is imminent.
    pub fn is_imminent(&self) -> bool {
        self.exception_handling_imminent
    }

    /// Get the exception code (if any).
    pub fn exception_code(&self) -> Option<u32> {
        self.exception_code
    }
}

/// Composite checkpoint trigger coordinator.
///
/// Manages all five trigger types and evaluates them to determine if a checkpoint
/// should be created. Any trigger being active will cause a checkpoint.
///
/// See Engineering Plan § 6.3 (Checkpointing - Trigger Coordination)
pub struct CheckpointTriggerCoordinator {
    /// Phase transition trigger
    phase_trigger: PhaseTransitionTrigger,
    /// Periodic timer trigger
    periodic_trigger: PeriodicTimerTrigger,
    /// Pre-preemption trigger
    preemption_trigger: PrePreemptionTrigger,
    /// Explicit signal trigger
    signal_trigger: ExplicitSignalTrigger,
    /// Exception handler trigger
    exception_trigger: ExceptionHandlerTrigger,
}

impl CheckpointTriggerCoordinator {
    /// Create a new checkpoint trigger coordinator.
    pub fn new() -> Self {
        Self {
            phase_trigger: PhaseTransitionTrigger::new(),
            periodic_trigger: PeriodicTimerTrigger::new(),
            preemption_trigger: PrePreemptionTrigger::new(),
            signal_trigger: ExplicitSignalTrigger::new(),
            exception_trigger: ExceptionHandlerTrigger::new(),
        }
    }

    /// Evaluate all triggers and return which ones are active.
    ///
    /// # Arguments
    ///
    /// * `current_phase` - Current CT phase
    /// * `current_time_ms` - Current time in milliseconds
    ///
    /// # Returns
    ///
    /// Vec of active triggers
    pub fn evaluate(&mut self, current_phase: CTPhase, current_time_ms: u64) -> alloc::vec::Vec<CheckpointTrigger> {
        let mut active_triggers = alloc::vec![];

        // Check phase transition
        if self.phase_trigger.check_transition(current_phase) {
            active_triggers.push(CheckpointTrigger::PhaseTransition);
        }

        // Check periodic timer
        if self.periodic_trigger.check_due(current_time_ms) {
            active_triggers.push(CheckpointTrigger::PeriodicTimer);
        }

        // Check pre-preemption
        if self.preemption_trigger.is_imminent() {
            active_triggers.push(CheckpointTrigger::PrePreemption);
            self.preemption_trigger.reset();
        }

        // Check explicit signal
        if self.signal_trigger.is_requested() {
            active_triggers.push(CheckpointTrigger::ExplicitSignal);
            self.signal_trigger.reset();
        }

        // Check exception handler
        if self.exception_trigger.is_imminent() {
            active_triggers.push(CheckpointTrigger::ExceptionHandler);
            self.exception_trigger.reset();
        }

        active_triggers
    }

    /// Check if any trigger is currently active.
    pub fn any_active(&self) -> bool {
        self.phase_trigger.transition_detected
            || self.preemption_trigger.is_imminent()
            || self.signal_trigger.is_requested()
            || self.exception_trigger.is_imminent()
    }

    /// Get mutable reference to phase transition trigger.
    pub fn phase_trigger_mut(&mut self) -> &mut PhaseTransitionTrigger {
        &mut self.phase_trigger
    }

    /// Get mutable reference to periodic timer trigger.
    pub fn periodic_trigger_mut(&mut self) -> &mut PeriodicTimerTrigger {
        &mut self.periodic_trigger
    }

    /// Get mutable reference to pre-preemption trigger.
    pub fn preemption_trigger_mut(&mut self) -> &mut PrePreemptionTrigger {
        &mut self.preemption_trigger
    }

    /// Get mutable reference to explicit signal trigger.
    pub fn signal_trigger_mut(&mut self) -> &mut ExplicitSignalTrigger {
        &mut self.signal_trigger
    }

    /// Get mutable reference to exception handler trigger.
    pub fn exception_trigger_mut(&mut self) -> &mut ExceptionHandlerTrigger {
        &mut self.exception_trigger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_checkpoint_trigger_names() {
        assert_eq!(CheckpointTrigger::PhaseTransition.name(), "PhaseTransition");
        assert_eq!(CheckpointTrigger::PeriodicTimer.name(), "PeriodicTimer");
        assert_eq!(CheckpointTrigger::PrePreemption.name(), "PrePreemption");
        assert_eq!(CheckpointTrigger::ExplicitSignal.name(), "ExplicitSignal");
        assert_eq!(CheckpointTrigger::ExceptionHandler.name(), "ExceptionHandler");
    }

    #[test]
    fn test_phase_transition_trigger_no_initial_transition() {
        let mut trigger = PhaseTransitionTrigger::new();
        assert!(!trigger.check_transition(CTPhase::Reason));
    }

    #[test]
    fn test_phase_transition_trigger_detects_transition() {
        let mut trigger = PhaseTransitionTrigger::new();
        trigger.check_transition(CTPhase::Plan);
        assert!(trigger.check_transition(CTPhase::Reason));
    }

    #[test]
    fn test_periodic_timer_trigger_due() {
        let mut trigger = PeriodicTimerTrigger::new();
        assert!(!trigger.check_due(30000)); // 30s elapsed
        assert!(trigger.check_due(120000)); // 60s+ elapsed
    }

    #[test]
    fn test_periodic_timer_trigger_custom_interval() {
        let mut trigger = PeriodicTimerTrigger::with_interval(10000); // 10s
        assert!(!trigger.check_due(5000)); // 5s elapsed
        assert!(trigger.check_due(15000)); // 10s+ elapsed
    }

    #[test]
    fn test_periodic_timer_trigger_record_checkpoint() {
        let mut trigger = PeriodicTimerTrigger::new();
        trigger.record_checkpoint(100000);
        assert_eq!(trigger.ms_until_next(110000), 50000); // 50s until next
    }

    #[test]
    fn test_pre_preemption_trigger() {
        let mut trigger = PrePreemptionTrigger::new();
        assert!(trigger.signal_preemption_imminent());
        assert!(trigger.is_imminent());
        trigger.reset();
        assert!(!trigger.is_imminent());
    }

    #[test]
    fn test_explicit_signal_trigger() {
        let mut trigger = ExplicitSignalTrigger::new();
        assert!(trigger.signal_checkpoint_requested());
        assert!(trigger.is_requested());
        trigger.reset();
        assert!(!trigger.is_requested());
    }

    #[test]
    fn test_exception_handler_trigger() {
        let mut trigger = ExceptionHandlerTrigger::new();
        assert!(trigger.signal_exception_handling(Some(42)));
        assert!(trigger.is_imminent());
        assert_eq!(trigger.exception_code(), Some(42));
        trigger.reset();
        assert!(!trigger.is_imminent());
    }

    #[test]
    fn test_checkpoint_trigger_coordinator_evaluate() {
        let mut coord = CheckpointTriggerCoordinator::new();
        coord.phase_trigger_mut().check_transition(CTPhase::Plan);
        
        let triggers = coord.evaluate(CTPhase::Reason, 100000);
        assert!(triggers.iter().any(|t| *t == CheckpointTrigger::PhaseTransition));
    }

    #[test]
    fn test_checkpoint_trigger_coordinator_multiple() {
        let mut coord = CheckpointTriggerCoordinator::new();
        coord.phase_trigger_mut().check_transition(CTPhase::Plan);
        coord.preemption_trigger_mut().signal_preemption_imminent();
        
        let triggers = coord.evaluate(CTPhase::Reason, 100000);
        assert_eq!(triggers.len(), 2);
    }
}
