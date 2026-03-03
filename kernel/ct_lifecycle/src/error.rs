// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Error Types for CT Lifecycle
//!
//! This module defines all error types that can occur during CT lifecycle management.
//! Errors are organized by subsystem for better debugging and error handling.
//!
//! ## References
//!
//! - Engineering Plan § 4.3 (Error Handling & Recovery)
//! - Engineering Plan § 5.2 (CT Invariants & Type-Safety)
use core::fmt;
use super::*;

use alloc::string::{String, ToString};
use alloc::string::ToString;


    #[test]

    fn test_invalid_phase_transition_display() {

        let err = CsError::InvalidPhaseTransition {

            current: "Plan".to_string(),

            target: "Spawn".to_string(),

            reason: "Cannot transition backwards".to_string(),

        };

        let msg = err.to_string();

        assert!(msg.contains("Plan"));

        assert!(msg.contains("Spawn"));

    }

    #[test]

    fn test_cyclic_dependency_display() {

        let err = CsError::CyclicDependency {

            ct_id: "ct-001".to_string(),

            cycle: "ct-001 -> ct-002 -> ct-001".to_string(),

        };

        let msg = err.to_string();

        assert!(msg.contains("Cyclic"));

    }

    #[test]

    fn test_budget_exceeded_display() {

        let err = CsError::BudgetExceeded {

            resource: "GPU".to_string(),

            requested: 1000,

            available: 500,

        };

        let msg = err.to_string();

        assert!(msg.contains("GPU"));

        assert!(msg.contains("1000"));

    }

    #[test]

    fn test_watchdog_timeout_display() {

        let err = CsError::WatchdogTimeout {

            timeout_type: "deadline".to_string(),

            limit: 5000,

            actual: 5100,

        };

        let msg = err.to_string();

        assert!(msg.contains("deadline"));

    }

    #[test]

    fn test_loop_detected_display() {

        let err = CsError::LoopDetected {

            repeat_count: 50,

            threshold: 30,

        };

        let msg = err.to_string();

        assert!(msg.contains("Loop"));

    }

    #[test]

    fn test_context_window_full_display() {

        let err = CsError::ContextWindowFull {

            current_size: 8000,

            max_capacity: 8192,

            requested: 500,

        };

        let msg = err.to_string();

        assert!(msg.contains("Context"));

    }

    #[test]

    fn test_error_clone() {

        let err = CsError::InternalError {

            message: "test error".to_string(),

        };

        let _cloned = err.clone();

    }

    #[test]

    fn test_capability_violation_display() {

        let err = CsError::CapabilityViolation {

            capability_id: "CAP-123".to_string(),

            reason: "Not in parent capability set".to_string(),

        };

        let msg = err.to_string();

        assert!(msg.contains("Capability"));

    }


