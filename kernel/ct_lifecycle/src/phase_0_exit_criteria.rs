// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Phase 0 Exit Criteria Verification
//!
//! This module provides a comprehensive checklist for verifying all Phase 0
//! requirements have been satisfied before progression to Phase 1.
//!
//! ## Exit Criteria (Engineering Plan § 2.2)
//!
//! Phase 0 is complete when all of the following are satisfied:
//!
//! 1. **Bare-metal kernel boots in QEMU** - No Linux, no POSIX abstractions
//! 2. **100 CTs can be spawned** - Full lifecycle management
//! 3. **Cognitive priority scheduling** - 4-dimensional scoring algorithm
//! 4. **Capability enforcement** - Mandatory security policies enforced
//! 5. **ContextOverflow handling** - Exception recovery and eviction
//! 6. **SIG_DEADLINE_WARN dispatch** - Signal delivery at 80% deadline
//! 7. **Checkpoint and restore** - State serialization and reconstruction
//! 8. **Cycle detection** - Dependency DAG validation at spawn time
//!
//! ## Reference
//!
//! - Engineering Plan § 2.2 (Phase 0 Exit Criteria)
//! - Week 6 Objective: Phase 0 Finale with integration testing
//! - Week 6 Deliverable: `phase_0_exit_criteria.rs`
use crate::error::Result;
use super::*;

use alloc::string::{String, ToString};
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_exit_criterion_creation() {

        let criterion = ExitCriterion::new(

            1,

            "Test Criterion".to_string(),

            "This is a test criterion".to_string(),

        );

        assert_eq!(criterion.id, 1);

        assert_eq!(criterion.name, "Test Criterion");

        assert_eq!(criterion.status, CriterionStatus::NotStarted);

        assert_eq!(criterion.result_message, None);

    }

    #[test]

    fn test_criterion_status_transitions() {

        let mut criterion = ExitCriterion::new(1, "Test".to_string(), "Desc".to_string());

        assert_eq!(criterion.status, CriterionStatus::NotStarted);

        criterion.mark_in_progress();

        assert_eq!(criterion.status, CriterionStatus::InProgress);

        criterion.mark_passed(Some("All checks passed".to_string()));

        assert_eq!(criterion.status, CriterionStatus::Passed);

        assert!(criterion.result_message.is_some());

    }

    #[test]

    fn test_exit_criteria_checklist_creation() {

        let checklist = Phase0ExitCriteria::new();

        assert_eq!(checklist.criteria().len(), 8);

        for criterion in checklist.criteria() {

            assert_eq!(criterion.status, CriterionStatus::NotStarted);

        }

    }

    #[test]

    fn test_get_criterion_by_id() {

        let checklist = Phase0ExitCriteria::new();

        let crit1 = checklist.get_criterion(1);

        assert!(crit1.is_some());

        assert_eq!(crit1.unwrap().name, "Bare-metal kernel boot in QEMU");

        let crit8 = checklist.get_criterion(8);

        assert!(crit8.is_some());

        assert_eq!(crit8.unwrap().name, "Dependency cycle detection and rejection");

        let crit99 = checklist.get_criterion(99);

        assert!(crit99.is_none());

    }

    #[test]

    fn test_count_by_status() {

        let mut checklist = Phase0ExitCriteria::new();

        assert_eq!(checklist.count_by_status(CriterionStatus::NotStarted), 8);

        assert_eq!(checklist.count_by_status(CriterionStatus::Passed), 0);

        if let Some(crit) = checklist.get_criterion_mut(1) {

            crit.mark_passed(None);

        }

        if let Some(crit) = checklist.get_criterion_mut(2) {

            crit.mark_passed(None);

        }

        assert_eq!(checklist.count_by_status(CriterionStatus::Passed), 2);

        assert_eq!(checklist.count_by_status(CriterionStatus::NotStarted), 6);

    }

    #[test]

    fn test_all_passed() {

        let mut checklist = Phase0ExitCriteria::new();

        assert!(!checklist.all_passed());

        for id in 1..=8 {

            if let Some(crit) = checklist.get_criterion_mut(id) {

                crit.mark_passed(None);

            }

        }

        assert!(checklist.all_passed());

    }

    #[test]

    fn test_any_failed() {

        let mut checklist = Phase0ExitCriteria::new();

        assert!(!checklist.any_failed());

        if let Some(crit) = checklist.get_criterion_mut(5) {

            crit.mark_failed("Test failure".to_string());

        }

        assert!(checklist.any_failed());

    }

    #[test]

    fn test_summary() {

        let mut checklist = Phase0ExitCriteria::new();

        if let Some(crit) = checklist.get_criterion_mut(1) {

            crit.mark_passed(None);

        }

        if let Some(crit) = checklist.get_criterion_mut(2) {

            crit.mark_failed("Failed".to_string());

        }

        if let Some(crit) = checklist.get_criterion_mut(3) {

            crit.mark_in_progress();

        }

        let (passed, failed, in_progress, not_started, skipped) = checklist.summary();

        assert_eq!(passed, 1);

        assert_eq!(failed, 1);

        assert_eq!(in_progress, 1);

        assert_eq!(not_started, 5);

        assert_eq!(skipped, 0);

    }

    #[test]

    fn test_report_generation() {

        let mut checklist = Phase0ExitCriteria::new();

        if let Some(crit) = checklist.get_criterion_mut(1) {

            crit.mark_passed(Some("Kernel boots successfully".to_string()));

        }

        let report = checklist.report();

        assert!(report.contains("Phase 0 Exit Criteria Report"));

        assert!(report.contains("Passed:      1/8"));

        assert!(report.contains("Bare-metal kernel boot in QEMU"));

        assert!(report.contains("Kernel boots successfully"));

    }


