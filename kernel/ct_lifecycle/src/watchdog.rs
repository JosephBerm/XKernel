// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Watchdog Configuration and Deadline Enforcement
//!
//! This module defines the watchdog configuration that tracks and enforces
//! CT execution constraints including deadlines and loop detection.
//!
//! ## Invariant #6: Watchdog Enforcement
//!
//! The watchdog monitors:
//! - Wall-clock deadline (deadline_ms)
//! - Iteration count limit (max_iterations)
//! - State repetition (loop_detection_threshold)
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 5.2 (CT Invariant #6: Watchdog Enforcement)
use serde::{Deserialize, Serialize};
use super::*;


    #[test]

    fn test_watchdog_new() {

        let config = WatchdogConfig::new(60_000, 20, 15);

        assert_eq!(config.deadline_ms, 60_000);

        assert_eq!(config.max_iterations, 20);

        assert_eq!(config.loop_detection_threshold, 15);

    }

    #[test]

    fn test_watchdog_permissive() {

        let config = WatchdogConfig::permissive();

        assert_eq!(config.deadline_ms, 300_000);

        assert_eq!(config.max_iterations, 1000);

        assert_eq!(config.loop_detection_threshold, 100);

    }

    #[test]

    fn test_watchdog_strict() {

        let config = WatchdogConfig::strict();

        assert_eq!(config.deadline_ms, 30_000);

        assert_eq!(config.max_iterations, 10);

        assert_eq!(config.loop_detection_threshold, 5);

    }

    #[test]

    fn test_watchdog_balanced() {

        let config = WatchdogConfig::balanced();

        assert_eq!(config.deadline_ms, 120_000);

        assert_eq!(config.max_iterations, 50);

        assert_eq!(config.loop_detection_threshold, 30);

    }

    #[test]

    fn test_deadline_not_exceeded() {

        let config = WatchdogConfig::strict();

        assert!(!config.deadline_exceeded(10_000));

        assert!(!config.deadline_exceeded(29_999));

    }

    #[test]

    fn test_deadline_exceeded() {

        let config = WatchdogConfig::strict();

        assert!(config.deadline_exceeded(30_001));

        assert!(config.deadline_exceeded(60_000));

    }

    #[test]

    fn test_iteration_limit_not_exceeded() {

        let config = WatchdogConfig::strict();

        assert!(!config.iteration_limit_exceeded(5));

        assert!(!config.iteration_limit_exceeded(10));

    }

    #[test]

    fn test_iteration_limit_exceeded() {

        let config = WatchdogConfig::strict();

        assert!(config.iteration_limit_exceeded(11));

        assert!(config.iteration_limit_exceeded(100));

    }

    #[test]

    fn test_loop_not_detected() {

        let config = WatchdogConfig::strict();

        assert!(!config.loop_detected(3));

        assert!(!config.loop_detected(4));

    }

    #[test]

    fn test_loop_detected_at_threshold() {

        let config = WatchdogConfig::strict();

        assert!(config.loop_detected(5));

        assert!(config.loop_detected(10));

    }

    #[test]

    fn test_time_remaining() {

        let config = WatchdogConfig::strict();

        assert_eq!(config.time_remaining(10_000), 20_000);

        assert_eq!(config.time_remaining(0), 30_000);

    }

    #[test]

    fn test_time_remaining_exceeded() {

        let config = WatchdogConfig::strict();

        assert_eq!(config.time_remaining(30_001), 0);

        assert_eq!(config.time_remaining(60_000), 0);

    }

    #[test]

    fn test_iterations_remaining() {

        let config = WatchdogConfig::strict();

        assert_eq!(config.iterations_remaining(0), 10);

        assert_eq!(config.iterations_remaining(5), 5);

        assert_eq!(config.iterations_remaining(10), 0);

    }

    #[test]

    fn test_iterations_remaining_exceeded() {

        let config = WatchdogConfig::strict();

        assert_eq!(config.iterations_remaining(11), 0);

        assert_eq!(config.iterations_remaining(100), 0);

    }

    #[test]

    fn test_watchdog_default() {

        let config = WatchdogConfig::default();

        assert_eq!(config, WatchdogConfig::balanced());

    }


