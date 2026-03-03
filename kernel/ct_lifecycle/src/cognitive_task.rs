// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Cognitive Task (CT) Domain Model
//!
//! This module defines the core Cognitive Task type with all 19 properties
//! and enforcement of the 6 critical invariants via Rust's type system.
//!
//! ## The Six CT Invariants
//!
//! 1. **Capability Subset**: CT capabilities must always be a subset of parent Agent capabilities
//! 2. **Budget Constraint**: CT resource budget cannot exceed parent Agent quota
//! 3. **Dependency Resolution**: All dependencies must complete before CT transitions to Reason phase
//! 4. **Phase Transition Logging**: All phase transitions are logged atomically
//! 5. **DAG Acyclicity**: Dependency graph must be acyclic at spawn time
//! 6. **Watchdog Enforcement**: Deadline and iteration limits are monitored continuously
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 4.1.1 through § 4.1.19 (CT Properties)
//! - Engineering Plan § 5.2 (CT Invariants & Type-Safety)
use crate::ids::{AgentID, CapID, ChannelID, CheckpointID, CTID, TraceID};
use crate::phase::CTPhase;
use crate::resource::ResourceQuota;
use crate::watchdog::WatchdogConfig;
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::ToString;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use ulid::Ulid;


    #[test]

    fn test_cognitive_priority_new() {

        let priority = CognitivePriority::new(0.8, 0.7, 0.6, 0.5);

        assert_eq!(priority.chain_criticality, 0.8);

        assert_eq!(priority.resource_efficiency, 0.7);

    }

    #[test]

    fn test_cognitive_priority_clamping() {

        let priority = CognitivePriority::new(1.5, -0.5, 2.0, -1.0);

        assert_eq!(priority.chain_criticality, 1.0);

        assert_eq!(priority.resource_efficiency, 0.0);

        assert_eq!(priority.deadline_pressure, 1.0);

        assert_eq!(priority.capability_cost, 0.0);

    }

    #[test]

    fn test_cognitive_priority_score() {

        let priority = CognitivePriority::new(1.0, 0.5, 0.5, 0.0);

        let score = priority.composite_score();

        assert_eq!(score, 0.5);

    }

    #[test]

    fn test_cognitive_task_new() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let ct = CognitiveTask::new(agent_id, budget);

        assert_eq!(ct.parent_agent, agent_id);

        assert_eq!(ct.phase, CTPhase::Spawn);

        assert_eq!(ct.resource_budget, budget);

        assert!(ct.capabilities.is_empty());

    }

    #[test]

    fn test_phase_transition_valid() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let ct = CognitiveTask::new(agent_id, budget);

        let result = ct.transition_to(CTPhase::Plan, 100);

        assert!(result.is_ok());

        let ct2 = result.unwrap();

        assert_eq!(ct2.phase, CTPhase::Plan);

        assert_eq!(ct2.last_transition_ms, 100);

    }

    #[test]

    fn test_phase_transition_invalid() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let ct = CognitiveTask::new(agent_id, budget);

        // Cannot go directly from Spawn to Reason

        let result = ct.transition_to(CTPhase::Reason, 100);

        assert!(result.is_err());

    }

    #[test]

    fn test_phase_transition_with_dependencies() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let mut ct = CognitiveTask::new(agent_id, budget);

        // Add a dependency

        ct.dependencies.insert(CTID::new());

        // Try to transition to Reason with unsatisfied dependencies

        let result = ct.transition_to(CTPhase::Reason, 100);

        assert!(result.is_err());

    }

    #[test]

    fn test_full_transition_path() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let mut ct = CognitiveTask::new(agent_id, budget);

        let phases = [

            CTPhase::Plan,

            CTPhase::Reason,

            CTPhase::Act,

            CTPhase::Reflect,

            CTPhase::Yield,

            CTPhase::Complete,

        ];

        let mut timestamp = 100;

        for phase in phases.iter() {

            ct = ct.transition_to(*phase, timestamp).unwrap();

            assert_eq!(ct.phase, *phase);

            timestamp += 100;

        }

    }

    #[test]

    fn test_invariants_satisfied() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let ct = CognitiveTask::new(agent_id, budget);

        assert!(ct.invariants_satisfied());

    }

    #[test]

    fn test_context_window_ref_new() {

        let ctx = ContextWindowRef::new(8192, 2048);

        assert_eq!(ctx.max_tokens, 8192);

        assert_eq!(ctx.current_tokens, 2048);

    }

    #[test]

    fn test_context_window_available() {

        let ctx = ContextWindowRef::new(8192, 2048);

        assert_eq!(ctx.available(), 6144);

    }

    #[test]

    fn test_context_window_full() {

        let ctx = ContextWindowRef::new(8192, 8192);

        assert!(ctx.is_full());

        assert_eq!(ctx.available(), 0);

    }

    #[test]

    fn test_ct_context_has_space() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let ct = CognitiveTask::new(agent_id, budget);

        assert!(ct.context_has_space(1000));

    }

    #[test]

    fn test_ct_priority_score() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let mut ct = CognitiveTask::new(agent_id, budget);

        ct.priority = CognitivePriority::high_priority();

        let score = ct.priority_score();

        assert!(score > 0.6);

    }

    #[test]

    fn test_phase_name() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let ct = CognitiveTask::new(agent_id, budget);

        assert_eq!(ct.phase_name(), "Spawn");

    }

    #[test]

    fn test_is_terminal() {

        let agent_id = AgentID::new();

        let budget = ResourceQuota::unlimited();

        let mut ct = CognitiveTask::new(agent_id, budget);

        assert!(!ct.is_terminal());

        ct = ct.transition_to(CTPhase::Plan, 100).unwrap();

        ct = ct.transition_to(CTPhase::Reason, 200).unwrap();

        ct = ct.transition_to(CTPhase::Act, 300).unwrap();

        ct = ct.transition_to(CTPhase::Reflect, 400).unwrap();

        ct = ct.transition_to(CTPhase::Yield, 500).unwrap();

        ct = ct.transition_to(CTPhase::Complete, 600).unwrap();

        assert!(ct.is_terminal());

    }


