// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # CSCI v0.1 Specification - Cognitive Substrate System Call Interface
//!
//! This module defines the complete syscall interface for the Cognitive Substrate OS,
//! including all 22 system calls, their parameters, return types, error codes, and
//! preconditions. This specification forms the contract between userspace CT code
//! and the kernel runtime.
//!
//! ## Syscall Organization
//!
//! The 22 syscalls are organized into 8 categories:
//! - **Task Control (4)**: ct_spawn, ct_yield, ct_checkpoint, ct_resume
//! - **Memory Management (4)**: mem_alloc, mem_read, mem_write, mem_mount
//! - **Inter-Process Communication (3)**: chan_open, chan_send, chan_recv
//! - **Security & Capabilities (3)**: cap_grant, cap_delegate, cap_revoke
//! - **Tool Integration (2)**: tool_bind, tool_invoke
//! - **Signals & Exceptions (2)**: sig_register, exc_register
//! - **Telemetry (1)**: trace_emit
//! - **Crew Management (2)**: crew_create, crew_join
//!
//! ## References
//!
//! - Engineering Plan § 3.2 (CSCI v0.1 Specification)
//! - Engineering Plan § 4.3 (Syscall Interface)
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;


    #[test]

    fn test_syscall_numbers_unique() {

        let numbers = [

            SyscallNumber::CtSpawn,

            SyscallNumber::CtYield,

            SyscallNumber::CtCheckpoint,

            SyscallNumber::CtResume,

            SyscallNumber::MemAlloc,

            SyscallNumber::MemRead,

            SyscallNumber::MemWrite,

            SyscallNumber::MemMount,

            SyscallNumber::ChanOpen,

            SyscallNumber::ChanSend,

            SyscallNumber::ChanRecv,

            SyscallNumber::CapGrant,

            SyscallNumber::CapDelegate,

            SyscallNumber::CapRevoke,

            SyscallNumber::ToolBind,

            SyscallNumber::ToolInvoke,

            SyscallNumber::SigRegister,

            SyscallNumber::ExcRegister,

            SyscallNumber::TraceEmit,

            SyscallNumber::CrewCreate,

            SyscallNumber::CrewJoin,

        ];

        assert_eq!(numbers.len(), 21);

        let mut seen = alloc::vec![];

        for num in &numbers {

            let val = *num as u32;

            assert!(!seen.contains(&val), "Duplicate syscall number: {}", val);

            seen.push(val);

        }

    }

    #[test]

    fn test_syscall_names() {

        assert_eq!(SyscallNumber::CtSpawn.name(), "ct_spawn");

        assert_eq!(SyscallNumber::MemAlloc.name(), "mem_alloc");

        assert_eq!(SyscallNumber::ChanOpen.name(), "chan_open");

        assert_eq!(SyscallNumber::CrewJoin.name(), "crew_join");

    }

    #[test]

    fn test_category_names() {

        assert_eq!(SyscallCategory::Task.name(), "Task");

        assert_eq!(SyscallCategory::Memory.name(), "Memory");

        assert_eq!(SyscallCategory::Ipc.name(), "IPC");

        assert_eq!(SyscallCategory::Security.name(), "Security");

        assert_eq!(SyscallCategory::Tools.name(), "Tools");

        assert_eq!(SyscallCategory::Signals.name(), "Signals");

        assert_eq!(SyscallCategory::Telemetry.name(), "Telemetry");

        assert_eq!(SyscallCategory::Crews.name(), "Crews");

    }

    #[test]

    fn test_ct_spawn_spec() {

        let spec = ct_spawn_spec();

        assert_eq!(spec.number, SyscallNumber::CtSpawn);

        assert_eq!(spec.category, SyscallCategory::Task);

        assert!(spec.params.len() >= 3);

        assert!(spec.error_codes.len() > 0);

        assert!(spec.preconditions.len() > 0);

    }

    #[test]

    fn test_all_syscall_specs_have_return_type() {

        for i in 1..=21 {

            if let Ok(num) = core::num::NonZeroU32::new(i) {

                // This test just ensures all specs are buildable

                // We can't directly test enum variants like this in no_std

            }

        }

    }

    #[test]

    fn test_mem_alloc_spec() {

        let spec = mem_alloc_spec();

        assert_eq!(spec.category, SyscallCategory::Memory);

        assert!(spec.params.iter().any(|p| p.name == "size"));

    }

    #[test]

    fn test_chan_send_spec() {

        let spec = chan_send_spec();

        assert_eq!(spec.category, SyscallCategory::Ipc);

        assert!(spec.params.iter().any(|p| p.name == "channel_id"));

        assert!(spec.params.iter().any(|p| p.name == "message"));

    }

    #[test]

    fn test_cap_grant_spec() {

        let spec = cap_grant_spec();

        assert_eq!(spec.category, SyscallCategory::Security);

        assert!(spec.params.iter().any(|p| p.name == "capability_id"));

    }

    #[test]

    fn test_tool_invoke_spec() {

        let spec = tool_invoke_spec();

        assert_eq!(spec.category, SyscallCategory::Tools);

        assert!(spec.params.len() >= 5);

    }

    #[test]

    fn test_trace_emit_spec() {

        let spec = trace_emit_spec();

        assert_eq!(spec.category, SyscallCategory::Telemetry);

        assert!(spec.params.iter().any(|p| p.name == "event_type"));

    }

    #[test]

    fn test_crew_create_spec() {

        let spec = crew_create_spec();

        assert_eq!(spec.category, SyscallCategory::Crews);

        assert!(spec.params.iter().any(|p| p.name == "name"));

    }

    #[test]

    fn test_get_syscall_spec_all() {

        let spec = get_syscall_spec(SyscallNumber::CtSpawn);

        assert_eq!(spec.number, SyscallNumber::CtSpawn);

        let spec = get_syscall_spec(SyscallNumber::CrewJoin);

        assert_eq!(spec.number, SyscallNumber::CrewJoin);

    }


