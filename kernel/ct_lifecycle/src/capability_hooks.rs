// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Capability Hooks for Kernel Enforcement
//!
//! This module integrates the capability engine with CT lifecycle management
//! through hook points that are invoked during capability grant and revocation.
//!
//! ## Hook Architecture
//!
//! - **CapGrant Hook**: Called when a capability is granted to a CT or Agent
//! - **CapRevoke Hook**: Called when a capability is revoked from a CT or Agent
//! - **SigCapRevoked Signal**: Dispatched to all CTs holding a revoked capability
//!
//! ## Mandatory Policy Integration
//!
//! Before any capability is mapped to page tables, the MandatoryCapabilityPolicy
//! is consulted to ensure kernel security policies are enforced.
//!
//! ## References
//!
//! - Engineering Plan § 5.3: Capability Hooks & Enforcement
//! - Engineering Plan § 3.1.4: Mandatory & Stateless
//! - Engineering Plan § 3.2.3: Grant Operations
//! - Week 5 Deliverable: capability_hooks.rs
use core::fmt::{self, Debug, Display};
use crate::error::CsError;
use crate::ids::{AgentID, CapID, ChannelID, CTID};
use crate::{Result, CTPhase};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_cap_grant_hook() {

        let mut hooks = CapabilityHooks::new();

        let cap_id = CapID::new("cap-001");

        let recipient = "agent-001".to_string();

        let result = hooks.on_cap_grant(cap_id.clone(), recipient.clone(), 1000).unwrap();

        assert!(result.allowed);

        assert_eq!(result.signals_to_dispatch.len(), 1);

        assert!(matches!(

            &result.signals_to_dispatch[0],

            CapabilityHookEvent::CapGranted { .. }

        ));

    }

    #[test]

    fn test_cap_revoke_hook() {

        let mut hooks = CapabilityHooks::new();

        let cap_id = CapID::new("cap-001");

        let holder = "agent-001".to_string();

        let affected_cts = vec![CTID::new("ct-001"), CTID::new("ct-002")];

        let result = hooks

            .on_cap_revoke(

                cap_id.clone(),

                holder.clone(),

                affected_cts.clone(),

                "security policy".to_string(),

                2000,

            )

            .unwrap();

        assert!(result.allowed);

        // Should have 2 signals: revoke + signal dispatch

        assert_eq!(result.signals_to_dispatch.len(), 2);

        assert!(result.requires_audit());

        assert_eq!(result.audit_entries.len(), 1);

    }

    #[test]

    fn test_cap_revoke_hook_no_affected_cts() {

        let mut hooks = CapabilityHooks::new();

        let cap_id = CapID::new("cap-001");

        let holder = "agent-001".to_string();

        let result = hooks

            .on_cap_revoke(cap_id.clone(), holder.clone(), vec![], "cleanup".to_string(), 3000)

            .unwrap();

        assert!(result.allowed);

        // Should have only 1 signal (the revoke event, no signal dispatch)

        assert_eq!(result.signals_to_dispatch.len(), 1);

        assert_eq!(result.audit_entries.len(), 1);

    }

    #[test]

    fn test_hook_result_allow() {

        let result = CapabilityHookResult::allow();

        assert!(result.allowed);

        assert!(result.decision.allows());

    }

    #[test]

    fn test_hook_result_deny() {

        let result = CapabilityHookResult::deny("test reason");

        assert!(!result.allowed);

        assert!(result.decision.denies());

    }

    #[test]

    fn test_hook_result_audit() {

        let result = CapabilityHookResult::audit("audit message");

        assert!(result.allowed);

        assert!(result.requires_audit());

        assert_eq!(result.audit_entries.len(), 1);

    }

    #[test]

    fn test_hook_stats() {

        let mut hooks = CapabilityHooks::new();

        hooks

            .on_cap_grant(

                CapID::new("cap-001"),

                "agent-001".to_string(),

                1000,

            )

            .unwrap();

        hooks

            .on_cap_grant(

                CapID::new("cap-002"),

                "agent-001".to_string(),

                1100,

            )

            .unwrap();

        hooks

            .on_cap_revoke(

                CapID::new("cap-003"),

                "agent-001".to_string(),

                vec![],

                "test".to_string(),

                1200,

            )

            .unwrap();

        let (total, grants) = hooks.stats();

        assert_eq!(total, 3);

        assert_eq!(grants, 2);

    }

    #[test]

    fn test_capability_policy_decision_approve() {

        let decision = CapabilityPolicyDecision::Approve;

        assert!(decision.allows());

        assert!(!decision.denies());

    }

    #[test]

    fn test_capability_policy_decision_deny() {

        let decision = CapabilityPolicyDecision::Deny {

            reason: "policy violation".to_string(),

        };

        assert!(!decision.allows());

        assert!(decision.denies());

    }

    #[test]

    fn test_capability_policy_decision_require_approval() {

        let decision = CapabilityPolicyDecision::RequireApproval {

            reason: "admin approval needed".to_string(),

        };

        assert!(!decision.denies());

        assert!(decision.requires_approval());

    }

    #[test]

    fn test_signal_constants() {

        assert_eq!(SIG_CAPREVOKED, 52);

        assert_eq!(SIG_CAPGRANT, 53);

    }

    #[test]

    fn test_multiple_signals_in_result() {

        let mut result = CapabilityHookResult::allow();

        result.add_signal(CapabilityHookEvent::CapGranted {

            cap_id: CapID::new("cap-001"),

            recipient: "agent-001".to_string(),

            timestamp_ns: 1000,

        });

        result.add_signal(CapabilityHookEvent::CapRevoked {

            cap_id: CapID::new("cap-002"),

            holder: "agent-002".to_string(),

            timestamp_ns: 2000,

            reason: "test".to_string(),

        });

        assert_eq!(result.signals_to_dispatch.len(), 2);

    }

    #[test]

    fn test_hook_event_display() {

        let event = CapabilityHookEvent::CapGranted {

            cap_id: CapID::new("cap-001"),

            recipient: "agent-001".to_string(),

            timestamp_ns: 1000,

        };

        let display = format!("{}", event);

        assert!(display.contains("CapGranted"));

        assert!(display.contains("cap-001"));

    }


