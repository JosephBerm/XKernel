// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Phase 0 Integration Test Suite
//!
//! Comprehensive end-to-end testing for Phase 0 requirements.
//! This module contains 5 integration test scenarios that validate all
//! critical Phase 0 functionality in realistic deployment conditions.
//!
//! ## Test Scenarios
//!
//! 1. **Scenario 1**: [Phase 1] Cognitive priority scoring (deferred to Week 7-9)
//!
//! 2. **Scenario 2**: ContextOverflow exception handling
//!    - Trigger L1 working memory overflow
//!    - Verify exception is raised and caught
//!    - Verify eviction to L2 spillover
//!    - Verify CT can continue execution
//!
//! 3. **Scenario 3**: SIG_DEADLINE_WARN signal dispatch
//!    - Set deadline on CT
//!    - Advance time to 80% of deadline
//!    - Verify signal is dispatched to CT handler
//!    - Verify handler receives correct deadline metadata
//!
//! 4. **Scenario 4**: Checkpoint and restore
//!    - Create CT in Reason phase
//!    - Checkpoint state to persistent storage
//!    - Restore from checkpoint
//!    - Verify state consistency and execution determinism
//!
//! 5. **Scenario 5**: Dependency cycle rejection
//!    - Attempt to spawn CT with circular dependencies
//!    - Verify spawn is rejected with clear error
//!    - Verify error message identifies cycle members
//!    - Verify no partial state corruption
//!
//! ## References
//!
//! - Engineering Plan § 2.2 (Phase 0 Exit Criteria)
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Week 6 Objective: Phase 0 Finale with integration testing
//! - Week 6 Deliverable: `phase_0_integration_tests.rs`
use crate::cognitive_task::{CognitivePriority, CognitiveTask};
use crate::dependency_dag::DependencyDag;
use crate::error::Result;
use crate::ids::{CTID, AgentID};
use crate::phase::CTPhase;
use crate::trace_log::{KernelRingBuffer, TraceEntry};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_scenario_result_creation() {

        let result = ScenarioResult::new("Test Scenario".to_string());

        assert_eq!(result.name, "Test Scenario");

        assert!(result.passed);

        assert!(result.failures.is_empty());

    }

    #[test]

    fn test_scenario_result_failure() {

        let mut result = ScenarioResult::new("Test".to_string());

        assert!(result.passed);

        result.fail("Test failed".to_string());

        assert!(!result.passed);

        assert_eq!(result.failures.len(), 1);

    }

    #[test]

    fn test_scenario_result_metrics() {

        let mut result = ScenarioResult::new("Test".to_string());

        result.record_metric("test_metric".to_string(), 42);

        assert_eq!(result.metrics.get("test_metric"), Some(&42));

    }

    #[test]

    fn test_scenario_2_overflow_handling() {

        let result = scenario_2_context_overflow_exception_handling();

        assert!(result.passed);

        assert!(result.metrics.contains_key("l1_capacity_bytes"));

    }

    #[test]

    fn test_scenario_3_deadline_warning() {

        let result = scenario_3_sig_deadline_warn_dispatch();

        assert!(result.passed);

        assert!(result.metrics.contains_key("deadline_ms"));

    }

    #[test]

    fn test_scenario_4_checkpoint() {

        let result = scenario_4_checkpoint_and_restore();

        assert!(result.passed);

        assert!(result.metrics.contains_key("checkpoint_size_bytes"));

    }

    #[test]

    fn test_scenario_5_cycle_rejection() {

        let result = scenario_5_dependency_cycle_rejection();

        assert!(result.passed);

        assert_eq!(result.metrics.get("cycle_detected"), Some(&1));

    }

    #[test]

    fn test_generate_report() {

        let mut results = Vec::new();

        results.push(ScenarioResult::new("Scenario 1".to_string()));

        results.push(ScenarioResult::new("Scenario 2".to_string()));

        let report = generate_test_report(&results);

        assert!(report.contains("Summary: 2/2 scenarios passed"));

        assert!(report.contains("Scenario 1"));

        assert!(report.contains("Scenario 2"));

    }


