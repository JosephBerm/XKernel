// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # MMU-Backed Capability-Aware Page Mapping
//!
//! This module enforces capability-aware memory mapping: **Only memory pages
//! corresponding to held capabilities are mapped in page tables.**
//!
//! ## Core Invariant
//!
//! For any agent or CT with capability set C and resource set R:
//! - A page P corresponding to resource r ∈ R is mapped in the page table
//!   **if and only if** the agent/CT holds a capability for r
//! - Absence of capability → no page mapping (fail-safe default)
//! - Capability revocation → immediate page unmapping
//!
//! ## Design Principles
//!
//! - **Fail-Safe Default**: No mapping without capability
//! - **Mandatory Enforcement**: Cannot bypass via direct page table writes
//! - **Hardware Enforcement**: x86-64 page tables provide memory isolation
//! - **Atomic Operations**: Mapping and capability checks are atomic
//!
//! ## Page Mapping Workflow
//!
//! 1. Capability check: Does agent hold capability for resource?
//! 2. Policy check: Do mandatory policies allow mapping?
//! 3. Page table entry creation: Map virtual address → physical page
//! 4. TLB invalidation: Flush translation buffer
//!
//! ## References
//!
//! - Engineering Plan § 5.4: MMU-Backed Capability Mapping
//! - Engineering Plan § 3.4: Virtual Memory
//! - Week 5 Deliverable: mmu_capability_mapping.rs
use core::fmt::{self, Debug, Display};
use crate::ids::{AgentID, CapID, CTID};
use crate::virtual_memory::PageTableEntryFlags;
use crate::{Result, MmuError};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec::Vec;
use alloc::string::ToString;


    fn make_test_flags() -> PageTableEntryFlags {

        PageTableEntryFlags {

            present: true,

            writable: true,

            user_accessible: true,

            write_through: false,

            cache_disable: false,

            accessed: false,

            dirty: false,

            huge_page: false,

            global: false,

        }

    }

    #[test]

    fn test_resource_ref_creation() {

        let res = ResourceRef::new("memory", "mem-001");

        assert_eq!(res.resource_type, "memory");

        assert_eq!(res.resource_id, "mem-001");

    }

    #[test]

    fn test_capability_page_mapping_creation() {

        let cap_id = CapID::new("cap-001");

        let resource = ResourceRef::new("memory", "mem-001");

        let flags = make_test_flags();

        let mapping = CapabilityPageMapping::new(

            cap_id.clone(),

            resource.clone(),

            0x1000,

            0x2000,

            0x100,

            flags,

            1000,

        );

        assert_eq!(mapping.cap_id, cap_id);

        assert_eq!(mapping.resource, resource);

        assert_eq!(mapping.virtual_start, 0x1000);

        assert_eq!(mapping.virtual_end, 0x2000);

        assert_eq!(mapping.size_bytes(), 0x1000);

        assert!(mapping.is_active);

    }

    #[test]

    fn test_mapping_covers_virtual_address() {

        let mapping = CapabilityPageMapping::new(

            CapID::new("cap-001"),

            ResourceRef::new("memory", "mem-001"),

            0x1000,

            0x2000,

            0x100,

            make_test_flags(),

            1000,

        );

        assert!(mapping.covers_virtual_address(0x1000));

        assert!(mapping.covers_virtual_address(0x1500));

        assert!(mapping.covers_virtual_address(0x1FFF));

        assert!(!mapping.covers_virtual_address(0x0FFF));

        assert!(!mapping.covers_virtual_address(0x2000));

    }

    #[test]

    fn test_mapping_deactivate() {

        let mut mapping = CapabilityPageMapping::new(

            CapID::new("cap-001"),

            ResourceRef::new("memory", "mem-001"),

            0x1000,

            0x2000,

            0x100,

            make_test_flags(),

            1000,

        );

        assert!(mapping.is_active);

        mapping.deactivate();

        assert!(!mapping.is_active);

    }

    #[test]

    fn test_create_mapping() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        let resource = ResourceRef::new("memory", "mem-001");

        let result = mapper.create_mapping(

            "agent-001".to_string(),

            cap_id.clone(),

            resource,

            0x1000,

            0x2000,

            0x100,

            make_test_flags(),

            1000,

        );

        assert!(result.is_ok());

        let stats = mapper.stats();

        assert_eq!(stats.total_mappings_created, 1);

        assert_eq!(stats.active_mappings, 1);

    }

    #[test]

    fn test_create_mapping_invalid_range() {

        let mut mapper = MmuCapabilityMapper::new();

        let result = mapper.create_mapping(

            "agent-001".to_string(),

            CapID::new("cap-001"),

            ResourceRef::new("memory", "mem-001"),

            0x2000,

            0x1000, // end < start

            0x100,

            make_test_flags(),

            1000,

        );

        assert!(result.is_err());

    }

    #[test]

    fn test_multiple_mappings_same_capability() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        // Create multiple mappings for same capability

        for i in 0..3 {

            let start = 0x1000 + (i * 0x1000);

            let end = start + 0x1000;

            mapper

                .create_mapping(

                    "agent-001".to_string(),

                    cap_id.clone(),

                    ResourceRef::new("memory", &format!("mem-{}", i)),

                    start,

                    end,

                    0x100 + i as u64,

                    make_test_flags(),

                    1000 + i as u64,

                )

                .unwrap();

        }

        let stats = mapper.stats();

        assert_eq!(stats.total_mappings_created, 3);

        assert_eq!(stats.active_mappings, 3);

    }

    #[test]

    fn test_revoke_capability_mappings() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        // Create 3 mappings

        for i in 0..3 {

            mapper

                .create_mapping(

                    "agent-001".to_string(),

                    cap_id.clone(),

                    ResourceRef::new("memory", &format!("mem-{}", i)),

                    0x1000 + (i * 0x1000),

                    0x2000 + (i * 0x1000),

                    0x100 + i as u64,

                    make_test_flags(),

                    1000,

                )

                .unwrap();

        }

        // Revoke all mappings for this capability

        let removed = mapper.revoke_capability_mappings(&cap_id);

        assert_eq!(removed, 3);

        let stats = mapper.stats();

        assert_eq!(stats.total_mappings_removed, 3);

        assert_eq!(stats.active_mappings, 0);

    }

    #[test]

    fn test_deactivate_mapping() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        mapper

            .create_mapping(

                "agent-001".to_string(),

                cap_id.clone(),

                ResourceRef::new("memory", "mem-001"),

                0x1000,

                0x2000,

                0x100,

                make_test_flags(),

                1000,

            )

            .unwrap();

        let result = mapper.deactivate_mapping("agent-001", &cap_id, 0x1500);

        assert!(result.is_ok());

        let stats = mapper.stats();

        assert_eq!(stats.active_mappings, 0);

    }

    #[test]

    fn test_deactivate_mapping_not_found() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        let result = mapper.deactivate_mapping("agent-001", &cap_id, 0x1500);

        assert!(result.is_err());

    }

    #[test]

    fn test_get_entity_mappings() {

        let mut mapper = MmuCapabilityMapper::new();

        // Create mappings for multiple entities

        for entity in &["agent-001", "agent-002"] {

            for i in 0..2 {

                mapper

                    .create_mapping(

                        entity.to_string(),

                        CapID::new(&format!("cap-{}", i)),

                        ResourceRef::new("memory", &format!("mem-{}", i)),

                        0x1000 + (i * 0x1000),

                        0x2000 + (i * 0x1000),

                        0x100 + i as u64,

                        make_test_flags(),

                        1000,

                    )

                    .unwrap();

            }

        }

        let agent1_mappings = mapper.get_entity_mappings("agent-001");

        assert_eq!(agent1_mappings.len(), 2);

        let agent2_mappings = mapper.get_entity_mappings("agent-002");

        assert_eq!(agent2_mappings.len(), 2);

    }

    #[test]

    fn test_get_capability_mappings() {

        let mut mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        // Create 3 mappings with same capability, different entities

        for i in 0..3 {

            mapper

                .create_mapping(

                    format!("entity-{}", i),

                    cap_id.clone(),

                    ResourceRef::new("memory", &format!("mem-{}", i)),

                    0x1000 + (i * 0x1000),

                    0x2000 + (i * 0x1000),

                    0x100 + i as u64,

                    make_test_flags(),

                    1000,

                )

                .unwrap();

        }

        let cap_mappings = mapper.get_capability_mappings(&cap_id);

        assert_eq!(cap_mappings.len(), 3);

    }

    #[test]

    fn test_mapping_stats() {

        let mut mapper = MmuCapabilityMapper::new();

        for i in 0..5 {

            mapper

                .create_mapping(

                    "agent-001".to_string(),

                    CapID::new(&format!("cap-{}", i)),

                    ResourceRef::new("memory", &format!("mem-{}", i)),

                    0x1000,

                    0x2000,

                    0x100 + i as u64,

                    make_test_flags(),

                    1000,

                )

                .unwrap();

        }

        let stats = mapper.stats();

        assert_eq!(stats.total_mappings_created, 5);

        assert_eq!(stats.active_mappings, 5);

        assert!(stats.total_bytes_mapped > 0);

    }

    #[test]

    fn test_fail_safe_default_no_capability() {

        // This test demonstrates the fail-safe default:

        // Without calling create_mapping(), there are no mappings

        let mapper = MmuCapabilityMapper::new();

        let cap_id = CapID::new("cap-001");

        let mappings = mapper.get_capability_mappings(&cap_id);

        assert_eq!(mappings.len(), 0); // No mapping without capability

    }


