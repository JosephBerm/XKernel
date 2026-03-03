// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Boot Sequence Implementation
//!
//! This module implements the Cognitive Substrate OS boot sequence, orchestrating
//! the transition from firmware handoff through kernel initialization and first CT spawn.
//! The boot process progresses through strictly-ordered stages, each with detailed
//! error handling and timing instrumentation.
//!
//! ## Boot Stages
//!
//! 1. **FirmwareHandoff**: Kernel receives control from bootloader
//! 2. **MemoryInit**: Physical memory mapping from firmware
//! 3. **MmuEnable**: Virtual memory system activation
//! 4. **InterruptInit**: Interrupt controller setup
//! 5. **GpuEnumerate**: GPU device discovery and enumeration
//! 6. **SchedulerInit**: Task scheduler initialization
//! 7. **InitCtSpawn**: Creation and startup of init CT
//! 8. **BootComplete**: All stages done, system ready
//!
//! ## Performance Target
//!
//! Total boot time to first CT execution: <500ms
//!
//! ## References
//!
//! - Engineering Plan § 3.2 (Boot Sequence Design)
//! - Engineering Plan § 4.3 (Error Handling & Recovery)
use core::fmt;
use super::*;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_boot_stage_display() {

        assert_eq!(BootStage::FirmwareHandoff.to_string(), "FirmwareHandoff");

        assert_eq!(BootStage::BootComplete.to_string(), "BootComplete");

    }

    #[test]

    fn test_valid_transitions() {

        assert!(BootStage::FirmwareHandoff.is_valid_transition(BootStage::MemoryInit));

        assert!(BootStage::MemoryInit.is_valid_transition(BootStage::MmuEnable));

        assert!(BootStage::MmuEnable.is_valid_transition(BootStage::InterruptInit));

        assert!(BootStage::InterruptInit.is_valid_transition(BootStage::GpuEnumerate));

        assert!(BootStage::GpuEnumerate.is_valid_transition(BootStage::SchedulerInit));

        assert!(BootStage::SchedulerInit.is_valid_transition(BootStage::InitCtSpawn));

        assert!(BootStage::InitCtSpawn.is_valid_transition(BootStage::BootComplete));

    }

    #[test]

    fn test_invalid_transitions() {

        assert!(!BootStage::FirmwareHandoff.is_valid_transition(BootStage::MmuEnable));

        assert!(!BootStage::MemoryInit.is_valid_transition(BootStage::InterruptInit));

        assert!(!BootStage::BootComplete.is_valid_transition(BootStage::InitCtSpawn));

    }

    #[test]

    fn test_boot_context_creation() {

        let ctx = BootContext::new(1000);

        assert_eq!(ctx.current_stage(), BootStage::FirmwareHandoff);

        assert!(!ctx.is_complete());

        assert_eq!(ctx.boot_log().len(), 1);

    }

    #[test]

    fn test_boot_context_transition_valid() {

        let mut ctx = BootContext::new(1000);

        let result = ctx.transition_stage(BootStage::MemoryInit, 2000, "Memory init");

        assert!(result.is_ok());

        assert_eq!(ctx.current_stage(), BootStage::MemoryInit);

        assert_eq!(ctx.boot_log().len(), 2);

    }

    #[test]

    fn test_boot_context_transition_invalid() {

        let mut ctx = BootContext::new(1000);

        let result = ctx.transition_stage(BootStage::InterruptInit, 2000, "Invalid");

        assert!(result.is_err());

        assert_eq!(ctx.current_stage(), BootStage::FirmwareHandoff);

    }

    #[test]

    fn test_boot_log_entries() {

        let mut log = BootLog::new(1000);

        log.record_stage(BootStage::FirmwareHandoff, 1000, "Start");

        log.record_stage(BootStage::MemoryInit, 2000, "Memory");

        assert_eq!(log.len(), 2);

        assert_eq!(log.entries()[0].stage, BootStage::FirmwareHandoff);

        assert_eq!(log.entries()[0].timestamp_ns, 0);

        assert_eq!(log.entries()[1].timestamp_ns, 1000);

    }

    #[test]

    fn test_boot_log_elapsed() {

        let log = BootLog::new(5000);

        assert_eq!(log.elapsed_ns(5000), 0);

        assert_eq!(log.elapsed_ns(10000), 5000);

        assert_eq!(log.elapsed_ns(15000), 10000);

    }

    #[test]

    fn test_boot_context_memory_tracking() {

        let mut ctx = BootContext::new(1000);

        ctx.set_total_memory(1024 * 1024 * 1024); // 1 GiB

        ctx.set_free_memory(512 * 1024 * 1024); // 512 MiB

        assert_eq!(ctx.total_memory_bytes(), 1024 * 1024 * 1024);

        assert_eq!(ctx.free_memory_bytes(), 512 * 1024 * 1024);

    }

    #[test]

    fn test_boot_error_display() {

        let err = BootError::MemoryMapInvalid {

            details: "Invalid entry".to_string(),

        };

        let msg = err.to_string();

        assert!(msg.contains("Invalid memory map"));

        let err2 = BootError::BootTimeout {

            stage: BootStage::MemoryInit,

            timeout_ms: 1000,

        };

        let msg2 = err2.to_string();

        assert!(msg2.contains("timeout"));

    }

    #[test]

    fn test_full_boot_sequence() {

        let mut ctx = BootContext::new(1000);

        ctx.set_total_memory(1024 * 1024);

        ctx.set_free_memory(512 * 1024);

        assert!(ctx.transition_stage(BootStage::MemoryInit, 2000, "Init").is_ok());

        assert!(ctx.transition_stage(BootStage::MmuEnable, 3000, "MMU").is_ok());

        assert!(ctx.transition_stage(BootStage::InterruptInit, 4000, "Int").is_ok());

        assert!(ctx.transition_stage(BootStage::GpuEnumerate, 5000, "GPU").is_ok());

        assert!(ctx.transition_stage(BootStage::SchedulerInit, 6000, "Sched").is_ok());

        assert!(ctx.transition_stage(BootStage::InitCtSpawn, 7000, "CT").is_ok());

        assert!(ctx.transition_stage(BootStage::BootComplete, 8000, "Done").is_ok());

        assert!(ctx.is_complete());

        assert_eq!(ctx.boot_log().len(), 8);

        assert_eq!(ctx.elapsed_ns(8000), 7000);

    }

    #[test]

    fn test_boot_context_elapsed() {

        let ctx = BootContext::new(5000);

        assert_eq!(ctx.elapsed_ns(5000), 0);

        assert_eq!(ctx.elapsed_ns(10000), 5000);

    }


