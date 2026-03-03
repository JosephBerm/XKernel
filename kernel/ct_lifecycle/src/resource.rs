// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Resource Management Types
//!
//! This module defines resource quota and cost attribution types that govern
//! resource allocation and tracking throughout the CT lifecycle.
//!
//! ## Key Types
//!
//! - `ResourceQuota` - The resource budget allocated to a CT
//! - `AgentQuota` - The total resource pool of an Agent
//! - `CostAttribution` - Tracks resource consumption and attribution
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 5.2 (CT Invariant #2: Budget Constraint)
use serde::{Deserialize, Serialize};
use super::*;


    #[test]

    fn test_resource_quota_new() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        assert_eq!(quota.max_tokens, 1000);

        assert_eq!(quota.gpu_ms, 100);

        assert_eq!(quota.wall_clock_ms, 5000);

        assert_eq!(quota.memory_bytes, 1024 * 1024);

        assert_eq!(quota.tool_calls, 50);

    }

    #[test]

    fn test_resource_quota_zero() {

        let quota = ResourceQuota::zero();

        assert_eq!(quota.max_tokens, 0);

        assert_eq!(quota.gpu_ms, 0);

    }

    #[test]

    fn test_resource_quota_unlimited() {

        let quota = ResourceQuota::unlimited();

        assert_eq!(quota.max_tokens, u64::MAX);

        assert_eq!(quota.tool_calls, u32::MAX);

    }

    #[test]

    fn test_can_accommodate() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 25);

        assert!(quota.can_accommodate(&cost));

    }

    #[test]

    fn test_cannot_accommodate() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let cost = CostAttribution::new(1001, 50, 2500, 512 * 1024, 25);

        assert!(!quota.can_accommodate(&cost));

    }

    #[test]

    fn test_subtract_cost() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let cost = CostAttribution::new(300, 30, 1000, 256 * 1024, 10);

        let remaining = quota.subtract(&cost);

        assert_eq!(remaining.max_tokens, 700);

        assert_eq!(remaining.gpu_ms, 70);

        assert_eq!(remaining.wall_clock_ms, 4000);

        assert_eq!(remaining.tool_calls, 40);

    }

    #[test]

    fn test_subtract_saturating() {

        let quota = ResourceQuota::new(100, 50, 1000, 1024, 5);

        let cost = CostAttribution::new(200, 75, 1500, 2048, 10);

        let remaining = quota.subtract(&cost);

        assert_eq!(remaining.max_tokens, 0);

        assert_eq!(remaining.gpu_ms, 0);

    }

    #[test]

    fn test_add_costs() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let cost = CostAttribution::new(300, 30, 1000, 256 * 1024, 10);

        let added = quota.add(&cost);

        assert_eq!(added.max_tokens, 1300);

        assert_eq!(added.gpu_ms, 130);

        assert_eq!(added.tool_calls, 60);

    }

    #[test]

    fn test_usage_fraction() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 100);

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 50);

        let (tokens, gpu, wall, mem, calls) = quota.usage_fraction(&cost).unwrap();

        assert_eq!(tokens, 0.5);

        assert_eq!(gpu, 0.5);

        assert_eq!(wall, 0.5);

        assert_eq!(mem, 0.5);

        assert_eq!(calls, 0.5);

    }

    #[test]

    fn test_cost_attribution_new() {

        let cost = CostAttribution::new(500, 50, 2500, 512 * 1024, 25);

        assert_eq!(cost.tokens, 500);

        assert_eq!(cost.gpu_ms, 50);

        assert_eq!(cost.tool_calls, 25);

    }

    #[test]

    fn test_cost_attribution_zero() {

        let cost = CostAttribution::zero();

        assert_eq!(cost.tokens, 0);

        assert_eq!(cost.gpu_ms, 0);

        assert_eq!(cost.memory_bytes, 0);

    }

    #[test]

    fn test_cost_add() {

        let cost1 = CostAttribution::new(100, 10, 1000, 100 * 1024, 5);

        let cost2 = CostAttribution::new(200, 20, 2000, 200 * 1024, 10);

        let sum = cost1.add(&cost2);

        assert_eq!(sum.tokens, 300);

        assert_eq!(sum.gpu_ms, 30);

        assert_eq!(sum.tool_calls, 15);

    }

    #[test]

    fn test_exceeds_quota() {

        let quota = ResourceQuota::new(1000, 100, 5000, 1024 * 1024, 50);

        let cost_ok = CostAttribution::new(500, 50, 2500, 512 * 1024, 25);

        let cost_bad = CostAttribution::new(1500, 50, 2500, 512 * 1024, 25);

        assert!(!cost_ok.exceeds_quota(&quota));

        assert!(cost_bad.exceeds_quota(&quota));

    }

    #[test]

    fn test_agent_quota_conversion() {

        let agent_quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let ct_quota = agent_quota.to_resource_quota();

        assert_eq!(ct_quota.max_tokens, 5000);

        assert_eq!(ct_quota.gpu_ms, 500);

    }


