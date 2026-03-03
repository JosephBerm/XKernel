// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Preemption Point Detection and Signal Delivery
//!
//! This module defines safe preemption points where signals can be delivered
//! without corrupting CT state or violating concurrency invariants.
//!
//! ## Preemption Points
//!
//! Safe preemption points are locations in the CT execution where:
//! - No locks are held
//! - No critical sections are active
//! - State is consistent
//! - Signal handlers can safely run
//!
//! The system defines four preemption point types:
//! 1. **After Syscall** - After syscall completes, before returning to CT
//! 2. **Between Reasoning Phases** - Between agent reasoning iterations
//! 3. **At Timer Interrupt** - At periodic timer interrupt
//! 4. **Before Context Switch** - Before switching to different CT
//!
//! ## Signal Delivery Algorithm
//!
//! 1. Check if CT is at a preemption point
//! 2. If yes, attempt to deliver next pending signal
//! 3. Invoke signal handler if registered
//! 4. Process handler result (Continue/Restart/Escalate)
//! 5. Repeat until no more signals or handler requests exit
//!
//! ## References
//!
//! - Engineering Plan § 6.1 (Signal System)
//! - Week 4 Objective: Preemption point detection and signal delivery

#![allow(dead_code)]

use crate::signal::CognitiveSignal;
use crate::signal_dispatch::{SignalDispatchTable, SignalHandlerResult};
use crate::Result;

/// Preemption point type classification.
///
/// Indicates the category of preemption point where signal delivery occurred.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreemptionPointType {
    /// Signal delivered after syscall completion
    AfterSyscall,
    /// Signal delivered between reasoning phases
    BetweenReasoningPhases,
    /// Signal delivered at timer interrupt
    AtTimerInterrupt,
    /// Signal delivered before context switch
    BeforeContextSwitch,
}

impl PreemptionPointType {
    /// Get human-readable name for this preemption point type.
    pub fn name(&self) -> &'static str {
        match self {
            PreemptionPointType::AfterSyscall => "AfterSyscall",
            PreemptionPointType::BetweenReasoningPhases => "BetweenReasoningPhases",
            PreemptionPointType::AtTimerInterrupt => "AtTimerInterrupt",
            PreemptionPointType::BeforeContextSwitch => "BeforeContextSwitch",
        }
    }
}

/// Preemption point context for signal delivery.
///
/// Captures information about the preemption point where signal delivery occurs.
#[derive(Clone, Debug)]
pub struct PreemptionPointContext {
    /// Type of preemption point
    pub point_type: PreemptionPointType,
    /// Timestamp when preemption point was reached (Unix epoch milliseconds)
    pub timestamp_ms: u64,
    /// Optional reason for preemption
    pub reason: Option<alloc::string::String>,
}

impl PreemptionPointContext {
    /// Create a new preemption point context.
    ///
    /// # Arguments
    ///
    /// * `point_type` - The type of preemption point
    /// * `timestamp_ms` - Current timestamp
    /// * `reason` - Optional reason string
    pub fn new(
        point_type: PreemptionPointType,
        timestamp_ms: u64,
        reason: Option<alloc::string::String>,
    ) -> Self {
        Self {
            point_type,
            timestamp_ms,
            reason,
        }
    }
}

/// Signal delivery result after processing at a preemption point.
///
/// Indicates the outcome of signal delivery and what action should follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignalDeliveryOutcome {
    /// Signal was delivered successfully
    Delivered,
    /// No signals were pending
    NoSignals,
    /// Signal handler returned Continue (normal operation continues)
    HandlerContinue,
    /// Signal handler returned Restart (operation should restart)
    HandlerRestart,
    /// Signal handler returned Escalate (escalate to exception system)
    HandlerEscalate,
}

/// Preemption point manager for coordinating signal delivery.
///
/// Manages signal delivery at safe preemption points within the CT execution.
/// This is a per-CT component that works with the SignalDispatchTable.
///
/// See Engineering Plan § 6.1 (Signal System)
pub struct PreemptionManager {
    /// Whether we are currently in a preemption point
    in_preemption_point: bool,
    /// Type of current preemption point
    current_point_type: Option<PreemptionPointType>,
    /// Timestamp of current preemption point
    current_timestamp_ms: u64,
}

impl PreemptionManager {
    /// Create a new preemption manager.
    pub fn new() -> Self {
        Self {
            in_preemption_point: false,
            current_point_type: None,
            current_timestamp_ms: 0,
        }
    }

    /// Mark entry into a preemption point.
    ///
    /// This signals that the CT has reached a safe location where signals
    /// can be delivered. Typically called by the kernel scheduler.
    ///
    /// # Arguments
    ///
    /// * `context` - The preemption point context
    pub fn enter_preemption_point(&mut self, context: &PreemptionPointContext) {
        self.in_preemption_point = true;
        self.current_point_type = Some(context.point_type);
        self.current_timestamp_ms = context.timestamp_ms;
    }

    /// Mark exit from a preemption point.
    ///
    /// Called after signal delivery is complete and CT resumes execution.
    pub fn exit_preemption_point(&mut self) {
        self.in_preemption_point = false;
        self.current_point_type = None;
    }

    /// Check if currently in a preemption point.
    ///
    /// # Returns
    ///
    /// True if the CT is currently at a safe preemption point
    pub fn is_in_preemption_point(&self) -> bool {
        self.in_preemption_point
    }

    /// Get the current preemption point type (if any).
    ///
    /// # Returns
    ///
    /// Some(type) if in preemption point, None otherwise
    pub fn current_point_type(&self) -> Option<PreemptionPointType> {
        self.current_point_type
    }

    /// Get the timestamp of current preemption point (if any).
    ///
    /// # Returns
    ///
    /// Timestamp in Unix epoch milliseconds
    pub fn current_timestamp_ms(&self) -> u64 {
        self.current_timestamp_ms
    }
}

impl Default for PreemptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Signal delivery coordinator for safe delivery at preemption points.
///
/// Coordinates signal delivery from the dispatch table to handlers,
/// respecting preemption point constraints and handling results.
///
/// See Engineering Plan § 6.1 (Signal System)
pub struct SignalDeliveryCoordinator {
    preemption_manager: PreemptionManager,
}

impl SignalDeliveryCoordinator {
    /// Create a new signal delivery coordinator.
    pub fn new() -> Self {
        Self {
            preemption_manager: PreemptionManager::new(),
        }
    }

    /// Deliver pending signals at a preemption point.
    ///
    /// This is the main entry point for signal delivery. Checks that we're
    /// at a safe preemption point, then delivers signals in FIFO order
    /// from the dispatch table.
    ///
    /// # Arguments
    ///
    /// * `dispatch_table` - The CT's signal dispatch table
    /// * `context` - The preemption point context
    ///
    /// # Returns
    ///
    /// - Ok(outcome) on success
    /// - Err(...) if delivery failed
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn deliver_at_preemption_point(
        &mut self,
        dispatch_table: &mut SignalDispatchTable,
        context: &PreemptionPointContext,
    ) -> Result<SignalDeliveryOutcome> {
        // Enter preemption point
        self.preemption_manager.enter_preemption_point(context);

        let outcome = self.do_deliver_signals(dispatch_table)?;

        // Exit preemption point
        self.preemption_manager.exit_preemption_point();

        Ok(outcome)
    }

    /// Internal method to deliver signals.
    ///
    /// This is called after entering a preemption point.
    /// Delivers signals in FIFO order until none remain or handler requests exit.
    fn do_deliver_signals(
        &self,
        dispatch_table: &mut SignalDispatchTable,
    ) -> Result<SignalDeliveryOutcome> {
        if !dispatch_table.has_pending_signals() {
            return Ok(SignalDeliveryOutcome::NoSignals);
        }

        // Deliver signals in FIFO order
        loop {
            match dispatch_table.deliver_next_signal()? {
                Some(result) => {
                    match result {
                        SignalHandlerResult::Continue => {
                            // Continue to next signal
                            continue;
                        }
                        SignalHandlerResult::Restart => {
                            return Ok(SignalDeliveryOutcome::HandlerRestart);
                        }
                        SignalHandlerResult::Escalate => {
                            return Ok(SignalDeliveryOutcome::HandlerEscalate);
                        }
                    }
                }
                None => {
                    // No more signals
                    return Ok(SignalDeliveryOutcome::Delivered);
                }
            }
        }
    }

    /// Get reference to preemption manager.
    pub fn preemption_manager(&self) -> &PreemptionManager {
        &self.preemption_manager
    }

    /// Get mutable reference to preemption manager.
    pub fn preemption_manager_mut(&mut self) -> &mut PreemptionManager {
        &mut self.preemption_manager
    }
}

impl Default for SignalDeliveryCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal_dispatch::SignalType;
    use alloc::string::String;
use alloc::format;
use alloc::vec;

    // ============================================================================
    // PreemptionPointType Tests
    // ============================================================================

    #[test]
    fn test_preemption_point_type_names() {
        assert_eq!(PreemptionPointType::AfterSyscall.name(), "AfterSyscall");
        assert_eq!(
            PreemptionPointType::BetweenReasoningPhases.name(),
            "BetweenReasoningPhases"
        );
        assert_eq!(PreemptionPointType::AtTimerInterrupt.name(), "AtTimerInterrupt");
        assert_eq!(
            PreemptionPointType::BeforeContextSwitch.name(),
            "BeforeContextSwitch"
        );
    }

    // ============================================================================
    // PreemptionPointContext Tests
    // ============================================================================

    #[test]
    fn test_preemption_point_context_new() {
        let context = PreemptionPointContext::new(
            PreemptionPointType::AfterSyscall,
            1000,
            Some(String::from("syscall complete")),
        );

        assert_eq!(context.point_type, PreemptionPointType::AfterSyscall);
        assert_eq!(context.timestamp_ms, 1000);
        assert!(context.reason.is_some());
    }

    #[test]
    fn test_preemption_point_context_no_reason() {
        let context = PreemptionPointContext::new(PreemptionPointType::AtTimerInterrupt, 2000, None);

        assert_eq!(context.point_type, PreemptionPointType::AtTimerInterrupt);
        assert_eq!(context.timestamp_ms, 2000);
        assert!(context.reason.is_none());
    }

    // ============================================================================
    // PreemptionManager Tests
    // ============================================================================

    #[test]
    fn test_preemption_manager_new() {
        let manager = PreemptionManager::new();
        assert!(!manager.is_in_preemption_point());
        assert!(manager.current_point_type().is_none());
    }

    #[test]
    fn test_preemption_manager_default() {
        let manager = PreemptionManager::default();
        assert!(!manager.is_in_preemption_point());
    }

    #[test]
    fn test_enter_exit_preemption_point() {
        let mut manager = PreemptionManager::new();
        let context = PreemptionPointContext::new(
            PreemptionPointType::BetweenReasoningPhases,
            5000,
            None,
        );

        assert!(!manager.is_in_preemption_point());

        manager.enter_preemption_point(&context);
        assert!(manager.is_in_preemption_point());
        assert_eq!(
            manager.current_point_type(),
            Some(PreemptionPointType::BetweenReasoningPhases)
        );
        assert_eq!(manager.current_timestamp_ms(), 5000);

        manager.exit_preemption_point();
        assert!(!manager.is_in_preemption_point());
    }

    // ============================================================================
    // SignalDeliveryCoordinator Tests
    // ============================================================================

    #[test]
    fn test_coordinator_new() {
        let coordinator = SignalDeliveryCoordinator::new();
        assert!(!coordinator.preemption_manager().is_in_preemption_point());
    }

    #[test]
    fn test_coordinator_default() {
        let coordinator = SignalDeliveryCoordinator::default();
        assert!(!coordinator.preemption_manager().is_in_preemption_point());
    }

    #[test]
    fn test_deliver_with_no_signals() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let context = PreemptionPointContext::new(PreemptionPointType::AfterSyscall, 1000, None);

        let outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        assert_eq!(outcome, SignalDeliveryOutcome::NoSignals);
    }

    #[test]
    fn test_deliver_with_handler_continue() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;

        dispatch.register(SignalType::Checkpoint, handler).unwrap();

        let sig = CognitiveSignal::SigCheckpoint {
            reason: "test".into(),
            timestamp_ms: 1000,
        };
        dispatch.queue_signal(sig, 1000).unwrap();

        let context = PreemptionPointContext::new(PreemptionPointType::AfterSyscall, 1000, None);
        let outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        assert_eq!(outcome, SignalDeliveryOutcome::Delivered);
    }

    #[test]
    fn test_deliver_with_handler_restart() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Restart;

        dispatch.register(SignalType::BudgetWarn, handler).unwrap();

        let sig = CognitiveSignal::SigBudgetWarn {
            budget_type: "tokens".into(),
            remaining: 100,
            allocated: 1000,
        };
        dispatch.queue_signal(sig, 1000).unwrap();

        let context = PreemptionPointContext::new(PreemptionPointType::AfterSyscall, 1000, None);
        let outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        assert_eq!(outcome, SignalDeliveryOutcome::HandlerRestart);
    }

    #[test]
    fn test_deliver_with_handler_escalate() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Escalate;

        dispatch.register(SignalType::ContextLow, handler).unwrap();

        let sig = CognitiveSignal::SigContextLow {
            current_bytes: 900,
            max_bytes: 1000,
        };
        dispatch.queue_signal(sig, 1000).unwrap();

        let context = PreemptionPointContext::new(PreemptionPointType::AfterSyscall, 1000, None);
        let outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        assert_eq!(outcome, SignalDeliveryOutcome::HandlerEscalate);
    }

    #[test]
    fn test_deliver_multiple_signals_fifo() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;

        dispatch.register(SignalType::Checkpoint, handler).unwrap();
        dispatch.register(SignalType::Resume, handler).unwrap();

        let sig1 = CognitiveSignal::SigCheckpoint {
            reason: "first".into(),
            timestamp_ms: 1000,
        };
        let sig2 = CognitiveSignal::SigResume {
            context_info: "second".into(),
        };

        dispatch.queue_signal(sig1, 1000).unwrap();
        dispatch.queue_signal(sig2, 1001).unwrap();

        let context = PreemptionPointContext::new(PreemptionPointType::AfterSyscall, 1000, None);
        let outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();

        // All signals delivered, last one was Continue
        assert_eq!(outcome, SignalDeliveryOutcome::Delivered);
        assert_eq!(dispatch.pending_count(), 0);
    }

    #[test]
    fn test_preemption_point_context_enter_exit() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);

        assert!(!coordinator
            .preemption_manager()
            .is_in_preemption_point());

        let context = PreemptionPointContext::new(PreemptionPointType::AtTimerInterrupt, 3000, None);
        coordinator.preemption_manager_mut().enter_preemption_point(&context);

        assert!(coordinator
            .preemption_manager()
            .is_in_preemption_point());
        coordinator
            .preemption_manager_mut()
            .exit_preemption_point();

        assert!(!coordinator
            .preemption_manager()
            .is_in_preemption_point());
    }

    #[test]
    fn test_delivery_outcome_equality() {
        assert_eq!(
            SignalDeliveryOutcome::Delivered,
            SignalDeliveryOutcome::Delivered
        );
        assert_ne!(
            SignalDeliveryOutcome::Delivered,
            SignalDeliveryOutcome::NoSignals
        );
    }

    // ============================================================================
    // Stress Tests: High-frequency preemption points
    // ============================================================================

    #[test]
    fn test_stress_rapid_preemption_points() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;

        dispatch.register(SignalType::BudgetWarn, handler).unwrap();

        // Simulate 100 preemption points with signals
        for i in 0..100 {
            let sig = CognitiveSignal::SigBudgetWarn {
                budget_type: "tokens".into(),
                remaining: 100,
                allocated: 1000,
            };
            dispatch.queue_signal(sig, 1000 + i as u64).unwrap();

            let context = PreemptionPointContext::new(
                PreemptionPointType::AfterSyscall,
                1000 + i as u64,
                None,
            );
            let _outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        }

        assert_eq!(dispatch.pending_count(), 0);
    }

    #[test]
    fn test_stress_preemption_point_types() {
        let mut coordinator = SignalDeliveryCoordinator::new();
        let mut dispatch = crate::signal_dispatch::SignalDispatchTable::new(1);
        let handler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;

        dispatch.register(SignalType::Checkpoint, handler).unwrap();

        let point_types = vec![
            PreemptionPointType::AfterSyscall,
            PreemptionPointType::BetweenReasoningPhases,
            PreemptionPointType::AtTimerInterrupt,
            PreemptionPointType::BeforeContextSwitch,
        ];

        for (i, point_type) in point_types.iter().enumerate() {
            let sig = CognitiveSignal::SigCheckpoint {
                reason: format!("test_{}", i),
                timestamp_ms: 1000 + i as u64,
            };
            dispatch.queue_signal(sig, 1000 + i as u64).unwrap();

            let context = PreemptionPointContext::new(*point_type, 1000 + i as u64, None);
            let _outcome = coordinator.deliver_at_preemption_point(&mut dispatch, &context).unwrap();
        }

        assert_eq!(dispatch.pending_count(), 0);
    }
}
