// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Agent Domain Model
//!
//! This module defines the Agent type, which represents an autonomous entity
//! capable of spawning and managing Cognitive Tasks.
//!
//! ## Agent Properties
//!
//! An Agent encapsulates 12 key properties including identity, capabilities,
//! memory state, resource management, and lifecycle configuration.
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 4.1.2 (Agent Properties)
use crate::ids::{AgentID, CapID, ChannelID, CTID};
use crate::resource::AgentQuota;
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::vec::Vec;
use ulid::Ulid;
use alloc::string::String;


    #[test]

    fn test_agent_new() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let agent = Agent::new(quota);

        assert_eq!(agent.resource_quota, quota);

        assert!(agent.capabilities.is_empty());

        assert!(agent.active_tasks.is_empty());

    }

    #[test]

    fn test_add_capability() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        let cap = CapID::new();

        assert!(agent.add_capability(cap));

        assert!(agent.has_capability(cap));

        // Adding again returns false

        assert!(!agent.add_capability(cap));

    }

    #[test]

    fn test_has_all_capabilities() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        let cap1 = CapID::new();

        let cap2 = CapID::new();

        let cap3 = CapID::new();

        agent.add_capability(cap1);

        agent.add_capability(cap2);

        let mut required = BTreeSet::new();

        required.insert(cap1);

        required.insert(cap2);

        assert!(agent.has_all_capabilities(&required));

        required.insert(cap3);

        assert!(!agent.has_all_capabilities(&required));

    }

    #[test]

    fn test_add_capability_dependency() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        let cap = CapID::new();

        let prereq = CapID::new();

        agent.add_capability_dependency(cap, prereq);

        let deps = agent.capability_graph.get(&cap);

        assert!(deps.is_some());

        assert!(deps.unwrap().contains(&prereq));

    }

    #[test]

    fn test_active_task_management() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        let ct = CTID::new();

        assert!(agent.add_active_task(ct).is_ok());

        assert!(agent.is_task_active(ct));

        assert_eq!(agent.active_task_count(), 1);

        assert!(agent.remove_active_task(ct));

        assert!(!agent.is_task_active(ct));

        assert_eq!(agent.active_task_count(), 0);

    }

    #[test]

    fn test_active_task_limit() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        agent.lifecycle_config.max_concurrent_cts = 2;

        let ct1 = CTID::new();

        let ct2 = CTID::new();

        let ct3 = CTID::new();

        assert!(agent.add_active_task(ct1).is_ok());

        assert!(agent.add_active_task(ct2).is_ok());

        let result = agent.add_active_task(ct3);

        assert!(result.is_err());

    }

    #[test]

    fn test_communication_protocol() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        let protocol = CommunicationProtocol {

            protocol_type: alloc::string::String::from("ipc"),

            channel_id: ChannelID::new(),

        };

        agent.add_protocol(protocol.clone());

        assert_eq!(agent.communication_protocols.len(), 1);

    }

    #[test]

    fn test_update_activity() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let mut agent = Agent::new(quota);

        agent.update_activity(1000);

        assert_eq!(agent.last_activity_ms, 1000);

        agent.update_activity(2000);

        assert_eq!(agent.last_activity_ms, 2000);

    }

    #[test]

    fn test_total_memory() {

        let quota = AgentQuota::new(5000, 500, 25000, 5 * 1024 * 1024, 250);

        let agent = Agent::new(quota);

        let total = agent.total_memory();

        assert_eq!(total, agent.memory_state.total_bytes());

    }

    #[test]

    fn test_memory_state_default() {

        let memory = MemoryState::default();

        assert_eq!(

            memory.total_bytes(),

            memory.long_term_memory_bytes

                + memory.working_memory_bytes

                + memory.experience_buffer_bytes

        );

    }

    #[test]

    fn test_framework_adapter() {

        let adapter = FrameworkAdapter::new(42, 1);

        assert_eq!(adapter.adapter_id, 42);

        assert_eq!(adapter.version, 1);

    }

    #[test]

    fn test_lifecycle_config_default() {

        let config = LifecycleConfig::default();

        assert!(config.max_concurrent_cts > 0);

        assert!(config.checkpoint_enabled);

        assert!(config.trace_enabled);

    }


