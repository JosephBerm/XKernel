// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Typestate Phase State Machine for Cognitive Tasks
//!
//! This module implements a strongly-typed state machine for CT phase transitions
//! using the typestate pattern. Illegal phase transitions are impossible to express
//! at compile-time, making the state machine type-safe.
//!
//! ## The Typestate Pattern
//!
//! Phase transitions are implemented as generic methods on `CTState<S>` where `S` is
//! a marker type representing the current phase. Only valid transitions have
//! corresponding methods, so illegal transitions become compiler errors.
//!
//! ## Phase Markers
//!
//! Each phase is represented by a distinct marker type:
//! - `Spawn` -> `Plan` -> `Reason` -> `Act` -> `Reflect` -> `Yield` -\
//!   - `Yield` can loop back to `Plan` for iteration
//!   - `Yield` can transition to `Complete`
//! - From any state to `Failed`
//! - `Complete` and `Failed` are terminal (no transitions)
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 5.2 (CT Invariants & Type-Safety)
//! - Engineering Plan § 4.1.3 (CTState State Machine)
use crate::error::Result;
use crate::ids::{AgentID, CapID, CTID, TraceID};
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_spawn_to_plan_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![]);

        assert_eq!(state.current_phase(), "Spawn");

        assert_eq!(state.ct_id(), ct_id);

        let state = state.transition_to_plan();

        assert_eq!(state.current_phase(), "Plan");

        assert_eq!(state.trace_log().len(), 1);

        assert_eq!(state.trace_log()[0].from_phase, "Spawn");

        assert_eq!(state.trace_log()[0].to_phase, "Plan");

    }

    #[test]

    fn test_plan_to_reason_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason();

        assert_eq!(state.current_phase(), "Reason");

        assert_eq!(state.trace_log().len(), 2);

    }

    #[test]

    fn test_reason_to_act_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act();

        assert_eq!(state.current_phase(), "Act");

    }

    #[test]

    fn test_act_to_reflect_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect();

        assert_eq!(state.current_phase(), "Reflect");

    }

    #[test]

    fn test_reflect_to_yield_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield();

        assert_eq!(state.current_phase(), "Yield");

    }

    #[test]

    fn test_yield_to_plan_iteration() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_plan();

        assert_eq!(state.current_phase(), "Plan");

        assert_eq!(state.trace_log().len(), 6);

    }

    #[test]

    fn test_reflect_to_complete_direct() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_complete();

        assert_eq!(state.current_phase(), "Complete");

    }

    #[test]

    fn test_yield_to_complete_transition() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_complete();

        assert_eq!(state.current_phase(), "Complete");

    }

    #[test]

    fn test_transition_to_failed_from_spawn() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_failed("Spawn phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

        assert!(state.trace_log()[0].reason.as_ref().unwrap().contains("error"));

    }

    #[test]

    fn test_transition_to_failed_from_plan() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_failed("Plan phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

    }

    #[test]

    fn test_transition_to_failed_from_reason() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_failed("Reason phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

    }

    #[test]

    fn test_transition_to_failed_from_act() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_failed("Act phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

    }

    #[test]

    fn test_transition_to_failed_from_reflect() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_failed("Reflect phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

    }

    #[test]

    fn test_transition_to_failed_from_yield() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_failed("Yield phase error".to_string());

        assert_eq!(state.current_phase(), "Failed");

    }

    #[test]

    fn test_full_happy_path() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let cap_id = CapID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![cap_id])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_complete();

        assert_eq!(state.current_phase(), "Complete");

        assert_eq!(state.capabilities(), &[cap_id]);

        assert_eq!(state.ct_id(), ct_id);

        assert_eq!(state.agent_ref(), agent_id);

        assert_eq!(state.trace_log().len(), 6);

    }

    #[test]

    fn test_iterative_path() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_plan()      // Iteration 1

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_yield()

            .transition_to_plan()      // Iteration 2

            .transition_to_reason()

            .transition_to_act()

            .transition_to_reflect()

            .transition_to_complete();

        assert_eq!(state.current_phase(), "Complete");

        assert_eq!(state.trace_log().len(), 15); // 3 cycles through the main loop

    }

    #[test]

    fn test_trace_log_entries() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![])

            .transition_to_plan()

            .transition_to_reason();

        let log = state.trace_log();

        assert_eq!(log.len(), 2);

        assert_eq!(log[0].from_phase, "Spawn");

        assert_eq!(log[0].to_phase, "Plan");

        assert!(log[0].reason.is_some());

        assert_eq!(log[1].from_phase, "Plan");

        assert_eq!(log[1].to_phase, "Reason");

        assert!(log[1].reason.is_some());

    }

    #[test]

    fn test_context_preservation() {

        let ct_id = CTID::new();

        let agent_id = AgentID::new();

        let mut state = CTState::<Spawn>::new(ct_id, agent_id, alloc::vec![]);

        // Simulate setting context

        let original_context = state.context.clone();

        let state = state.transition_to_plan();

        assert_eq!(state.context(), original_context.as_slice());

    }


