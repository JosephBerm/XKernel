// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Capability Validation for CT Spawn
//!
//! This module enforces the critical Invariant #1: **C_ct ⊆ C_parent**
//!
//! All CT capabilities must be a subset of the parent Agent's capabilities.
//! This validation occurs at CT spawn time with O(n) complexity.
//!
//! ## Design Principles
//!
//! - **Mandatory Enforcement**: Every CT spawn is validated before execution
//! - **Fail-Safe Default**: If validation fails, spawn is rejected
//! - **Efficient Validation**: O(n) scan of parent capabilities
//! - **Clear Error Reporting**: Exact capability violations are reported
//!
//! ## References
//!
//! - Engineering Plan § 5.2: CT Invariants & Type-Safety (Invariant #1)
//! - Engineering Plan § 4.1.6: CT Capabilities Property
//! - Engineering Plan § 3.1.0: Capability Domain Model
//! - Week 5 Deliverable: capability_validation.rs
use crate::agent::Agent;
use crate::agent::MemoryState;
use crate::cognitive_task::CognitiveTask;
use crate::cognitive_task::{CognitivePriority, ContextWindowRef, FrameworkAdapterRef};
use crate::error::CsError;
use crate::ids::TraceID;
use crate::ids::{AgentID, CapID};
use crate::phase::CTPhase;
use crate::resource::ResourceQuota;
use crate::watchdog::WatchdogConfig;
use crate::{Result, CTID};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;


    fn make_test_agent(cap_ids: Vec<&str>) -> Agent {

        let caps: BTreeSet<CapID> = cap_ids

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        Agent {

            id: AgentID::new("test-agent"),

            capabilities: caps,

            capability_graph: Default::default(),

            memory_state: MemoryState::default(),

            max_concurrent_cts: 10,

            ct_failure_restart_count: 3,

            checkpoint_enabled: true,

            trace_enabled: true,

            communication_protocol: Default::default(),

            framework_adapter: None,

            created_at_ms: 0,

            last_modified_ms: 0,

        }

    }

    // Helper to create a test CT

    fn make_test_ct(cap_ids: Vec<&str>, parent_id: AgentID) -> CognitiveTask {

        let caps: BTreeSet<CapID> = cap_ids

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        CognitiveTask {

            id: CTID::new("test-ct"),

            parent_agent: parent_id,

            crew: None,

            phase: CTPhase::Planning,

            priority: CognitivePriority::default_balanced(),

            capabilities: caps,

            context_window: ContextWindowRef {

                start: 0,

                end: 1000,

            },

            resource_budget: ResourceQuota::default(),

            dependencies: BTreeSet::new(),

            trace_log: TraceID::new("test-trace"),

            checkpoint_refs: Vec::new(),

            signal_handlers: Vec::new(),

            exception_handler: None,

            working_memory_ref: None,

            watchdog_config: WatchdogConfig::default(),

            communication_protocols: Vec::new(),

            framework_adapter: None,

            created_at_ms: 0,

            last_modified_ms: 0,

        }

    }

    #[test]

    fn test_valid_spawn_subset() {

        let parent = make_test_agent(vec!["read", "write", "execute"]);

        let child = make_test_ct(vec!["read", "write"], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_ok());

    }

    #[test]

    fn test_valid_spawn_exact_match() {

        let parent = make_test_agent(vec!["read", "write"]);

        let child = make_test_ct(vec!["read", "write"], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_ok());

    }

    #[test]

    fn test_valid_spawn_empty_child() {

        let parent = make_test_agent(vec!["read", "write"]);

        let child = make_test_ct(vec![], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_ok());

    }

    #[test]

    fn test_invalid_spawn_single_missing_cap() {

        let parent = make_test_agent(vec!["read", "write"]);

        let child = make_test_ct(vec!["read", "admin"], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_err());

        match result.unwrap_err() {

            CapabilityValidationError::CapabilityNotInParent { .. } => {

                // Expected

            }

            other => panic!("Unexpected error variant: {:?}", other),

        }

    }

    #[test]

    fn test_invalid_spawn_multiple_missing_caps() {

        let parent = make_test_agent(vec!["read", "write"]);

        let child = make_test_ct(vec!["read", "admin", "delete"], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_err());

        match result.unwrap_err() {

            CapabilityValidationError::MultipleCapabilityViolations { missing_caps, .. } => {

                assert_eq!(missing_caps.len(), 2); // admin, delete

            }

            other => panic!("Unexpected error variant: {:?}", other),

        }

    }

    #[test]

    fn test_invalid_spawn_empty_parent() {

        let parent = make_test_agent(vec![]);

        let child = make_test_ct(vec!["read"], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_err());

        match result.unwrap_err() {

            CapabilityValidationError::EmptyParentCapabilities { .. } => {

                // Expected

            }

            other => panic!("Unexpected error variant: {:?}", other),

        }

    }

    #[test]

    fn test_invalid_spawn_both_empty() {

        // Both parent and child empty: should be valid

        let parent = make_test_agent(vec![]);

        let child = make_test_ct(vec![], parent.id.clone());

        let result = CapabilityValidator::validate_spawn(

            &parent,

            &child,

            parent.id.clone(),

            child.id.clone(),

        );

        assert!(result.is_ok());

    }

    #[test]

    fn test_capability_set_validation() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write", "execute"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> = vec!["read", "write"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let result = CapabilityValidator::validate_capability_set(

            &parent_caps,

            &child_caps,

            AgentID::new("parent"),

            CTID::new("child"),

        );

        assert!(result.is_ok());

    }

    #[test]

    fn test_capability_set_validation_fails() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> = vec!["read", "admin"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let result = CapabilityValidator::validate_capability_set(

            &parent_caps,

            &child_caps,

            AgentID::new("parent"),

            CTID::new("child"),

        );

        assert!(result.is_err());

    }

    #[test]

    fn test_compute_capability_diff() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write", "execute"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> = vec!["read", "admin"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let (only_in_parent, only_in_child) =

            CapabilityValidator::compute_capability_diff(&parent_caps, &child_caps);

        assert_eq!(only_in_parent.len(), 2); // write, execute

        assert_eq!(only_in_child.len(), 1); // admin

    }

    #[test]

    fn test_is_subset_true() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write", "execute"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> =

            vec!["read", "write"].into_iter().map(|s| CapID::new(s)).collect();

        assert!(CapabilityValidator::is_subset(&parent_caps, &child_caps));

    }

    #[test]

    fn test_is_subset_false() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> = vec!["read", "admin"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        assert!(!CapabilityValidator::is_subset(&parent_caps, &child_caps));

    }

    #[test]

    fn test_is_subset_empty_child() {

        let parent_caps: BTreeSet<CapID> = vec!["read", "write"]

            .into_iter()

            .map(|s| CapID::new(s))

            .collect();

        let child_caps: BTreeSet<CapID> = BTreeSet::new();

        assert!(CapabilityValidator::is_subset(&parent_caps, &child_caps));

    }

    #[test]

    fn test_large_capability_set() {

        // Test with larger sets to ensure scalability

        let parent_caps: BTreeSet<CapID> = (0..100)

            .map(|i| CapID::new(&format!("cap-{}", i)))

            .collect();

        let child_caps: BTreeSet<CapID> =

            (0..50).map(|i| CapID::new(&format!("cap-{}", i))).collect();

        assert!(CapabilityValidator::is_subset(&parent_caps, &child_caps));

    }

    #[test]

    fn test_large_capability_set_with_violation() {

        // Test with larger sets where one capability violates the invariant

        let parent_caps: BTreeSet<CapID> = (0..100)

            .map(|i| CapID::new(&format!("cap-{}", i)))

            .collect();

        let mut child_caps: BTreeSet<CapID> =

            (0..50).map(|i| CapID::new(&format!("cap-{}", i))).collect();

        child_caps.insert(CapID::new("cap-invalid")); // Add capability not in parent

        assert!(!CapabilityValidator::is_subset(&parent_caps, &child_caps));

    }


