// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Agent Crew Domain Model
//!
//! This module defines the AgentCrew type, which represents a collaborative
//! group of agents working toward a shared mission.
//!
//! ## Agent Crew Properties
//!
//! An AgentCrew coordinates the activities of multiple agents and their CTs,
//! enforcing crew-wide resource budgets and scheduling constraints.
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 4.1.3 (AgentCrew Properties)
use crate::ids::{AgentID, CrewID, CTID};
use crate::resource::{AgentQuota, CostAttribution};
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use ulid::Ulid;
use alloc::string::ToString;


    #[test]

    fn test_shared_memory_new() {

        let mem = SharedMemory::new(1024 * 1024, true);

        assert_eq!(mem.capacity_bytes, 1024 * 1024);

        assert_eq!(mem.used_bytes, 0);

        assert!(mem.persistent);

    }

    #[test]

    fn test_shared_memory_available() {

        let mem = SharedMemory::new(1024 * 1024, true);

        assert_eq!(mem.available(), 1024 * 1024);

    }

    #[test]

    fn test_shared_memory_allocate() {

        let mut mem = SharedMemory::new(1024 * 1024, true);

        assert!(mem.allocate(512 * 1024));

        assert_eq!(mem.used_bytes, 512 * 1024);

        assert_eq!(mem.available(), 512 * 1024);

    }

    #[test]

    fn test_shared_memory_allocate_overflow() {

        let mut mem = SharedMemory::new(1024, true);

        assert!(mem.allocate(512));

        assert!(!mem.allocate(600)); // Not enough space

    }

    #[test]

    fn test_shared_memory_deallocate() {

        let mut mem = SharedMemory::new(1024 * 1024, true);

        mem.allocate(512 * 1024);

        mem.deallocate(256 * 1024);

        assert_eq!(mem.used_bytes, 256 * 1024);

    }

    #[test]

    fn test_crew_new() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        assert_eq!(crew.mission, "Test mission");

        assert_eq!(crew.coordinator, coordinator);

        assert!(crew.has_member(coordinator));

        assert_eq!(crew.member_count(), 1);

    }

    #[test]

    fn test_add_member() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let member = AgentID::new();

        assert!(crew.add_member(member));

        assert!(crew.has_member(member));

        assert_eq!(crew.member_count(), 2);

        // Adding again returns false

        assert!(!crew.add_member(member));

    }

    #[test]

    fn test_remove_member() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let member = AgentID::new();

        crew.add_member(member);

        assert!(crew.remove_member(member).is_ok());

        assert!(!crew.has_member(member));

    }

    #[test]

    fn test_cannot_remove_coordinator() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let result = crew.remove_member(coordinator);

        assert!(result.is_err());

    }

    #[test]

    fn test_change_coordinator() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let new_coord = AgentID::new();

        crew.add_member(new_coord);

        assert!(crew.change_coordinator(new_coord).is_ok());

        assert_eq!(crew.coordinator, new_coord);

    }

    #[test]

    fn test_change_coordinator_not_member() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(10000, 1000, 50000, 10 * 1024 * 1024, 500);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let not_member = AgentID::new();

        let result = crew.change_coordinator(not_member);

        assert!(result.is_err());

    }

    #[test]

    fn test_can_accommodate_cost() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 25);

        assert!(crew.can_accommodate_cost(&cost));

        let expensive_cost = CostAttribution::new(2000, 50, 2500, 512 * 1024, 25);

        assert!(!crew.can_accommodate_cost(&expensive_cost));

    }

    #[test]

    fn test_record_cost() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 25);

        assert!(crew.record_cost(&cost).is_ok());

        assert_eq!(crew.accumulated_cost.tokens, 500);

    }

    #[test]

    fn test_record_cost_exceeds_budget() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let cost = CostAttribution::new(1500, 50, 2500, 512 * 1024, 25);

        let result = crew.record_cost(&cost);

        assert!(result.is_err());

    }

    #[test]

    fn test_remaining_budget() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let cost = CostAttribution::new(300, 30, 1000, 256 * 1024, 10);

        let _ = crew.record_cost(&cost);

        let remaining = crew.remaining_budget();

        assert_eq!(remaining.max_tokens, 700);

        assert_eq!(remaining.gpu_ms, 70);

    }

    #[test]

    fn test_update_coordination() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        crew.update_coordination(1000);

        assert_eq!(crew.last_coordination_ms, 1000);

    }

    #[test]

    fn test_utilization_fraction() {

        let coordinator = AgentID::new();

        let budget = AgentQuota::new(1000, 100, 5000, 1024 * 1024, 100);

        let mut crew = AgentCrew::new("Test mission".to_string(), coordinator, budget);

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 50);

        let _ = crew.record_cost(&cost);

        let frac = crew.utilization_fraction();

        assert!(frac.is_some());

    }

    #[test]

    fn test_scheduling_affinity_default() {

        let affinity = SchedulingAffinity::default();

        assert!(!affinity.co_locate);

        assert!(!affinity.sequential_only);

    }

    #[test]

    fn test_scheduling_affinity_co_located() {

        let affinity = SchedulingAffinity::co_located();

        assert!(affinity.co_locate);

    }

    #[test]

    fn test_scheduling_affinity_sequential() {

        let affinity = SchedulingAffinity::sequential();

        assert!(affinity.sequential_only);

        assert_eq!(affinity.max_concurrent_crew_cts, 1);

    }


