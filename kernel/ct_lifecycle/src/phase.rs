// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # CT Phase Types and Transition Rules
//!
//! This module defines the CTPhase enumeration and valid state transitions
//! for Cognitive Tasks. Phase transitions enforce the CT lifecycle model
//! and ensure all invariants are maintained throughout execution.
//!
//! ## Phase Transitions
//!
//! The valid state transitions form a DAG:
//! - Spawn -> Plan
//! - Plan -> Reason
//! - Reason -> Act
//! - Act -> Reflect
//! - Reflect -> Yield
//! - Yield -> Plan | Complete
//! - Complete (terminal)
//! - Failed (terminal)
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 5.2 (CT Invariants & Type-Safety)
use serde::{Deserialize, Serialize};
use super::*;

use alloc::format;


    #[test]

    fn test_spawn_to_plan_valid() {

        assert!(CTPhase::Spawn.can_transition_to(CTPhase::Plan));

    }

    #[test]

    fn test_spawn_to_reason_invalid() {

        assert!(!CTPhase::Spawn.can_transition_to(CTPhase::Reason));

    }

    #[test]

    fn test_plan_to_reason_valid() {

        assert!(CTPhase::Plan.can_transition_to(CTPhase::Reason));

    }

    #[test]

    fn test_reason_to_act_valid() {

        assert!(CTPhase::Reason.can_transition_to(CTPhase::Act));

    }

    #[test]

    fn test_act_to_reflect_valid() {

        assert!(CTPhase::Act.can_transition_to(CTPhase::Reflect));

    }

    #[test]

    fn test_reflect_to_yield_valid() {

        assert!(CTPhase::Reflect.can_transition_to(CTPhase::Yield));

    }

    #[test]

    fn test_yield_to_plan_valid() {

        assert!(CTPhase::Yield.can_transition_to(CTPhase::Plan));

    }

    #[test]

    fn test_yield_to_complete_valid() {

        assert!(CTPhase::Yield.can_transition_to(CTPhase::Complete));

    }

    #[test]

    fn test_complete_is_terminal() {

        assert!(CTPhase::Complete.is_terminal());

        assert!(!CTPhase::Complete.can_transition_to(CTPhase::Plan));

        assert!(!CTPhase::Complete.can_transition_to(CTPhase::Yield));

    }

    #[test]

    fn test_failed_is_terminal() {

        assert!(CTPhase::Failed.is_terminal());

        assert!(!CTPhase::Failed.can_transition_to(CTPhase::Plan));

        assert!(!CTPhase::Failed.can_transition_to(CTPhase::Complete));

    }

    #[test]

    fn test_backward_transition_invalid() {

        assert!(!CTPhase::Plan.can_transition_to(CTPhase::Spawn));

        assert!(!CTPhase::Reason.can_transition_to(CTPhase::Plan));

        assert!(!CTPhase::Act.can_transition_to(CTPhase::Reason));

    }

    #[test]

    fn test_is_reason_phase() {

        assert!(CTPhase::Reason.is_reason());

        assert!(!CTPhase::Plan.is_reason());

        assert!(!CTPhase::Act.is_reason());

    }

    #[test]

    fn test_as_str() {

        assert_eq!(CTPhase::Spawn.as_str(), "Spawn");

        assert_eq!(CTPhase::Plan.as_str(), "Plan");

        assert_eq!(CTPhase::Reason.as_str(), "Reason");

        assert_eq!(CTPhase::Act.as_str(), "Act");

        assert_eq!(CTPhase::Reflect.as_str(), "Reflect");

        assert_eq!(CTPhase::Yield.as_str(), "Yield");

        assert_eq!(CTPhase::Complete.as_str(), "Complete");

        assert_eq!(CTPhase::Failed.as_str(), "Failed");

    }

    #[test]

    fn test_display_impl() {

        let phase = CTPhase::Plan;

        let s = alloc::format!("{}", phase);

        assert_eq!(s, "Plan");

    }

    #[test]

    fn test_full_valid_path() {

        let phases = [

            CTPhase::Spawn,

            CTPhase::Plan,

            CTPhase::Reason,

            CTPhase::Act,

            CTPhase::Reflect,

            CTPhase::Yield,

            CTPhase::Complete,

        ];

        for i in 0..phases.len() - 1 {

            assert!(phases[i].can_transition_to(phases[i + 1]));

        }

    }


