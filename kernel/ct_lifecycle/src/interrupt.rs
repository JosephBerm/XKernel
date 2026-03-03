// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Interrupt Controller and Handler Management
//!
//! This module implements interrupt handling for the Cognitive Substrate kernel.
//! It provides the Interrupt Descriptor Table (IDT), interrupt vector registration,
//! and exception/IRQ handling mechanisms.
//!
//! ## Interrupt Types
//!
//! - **Exceptions** (vectors 0-31): CPU-generated faults and traps
//! - **IRQs** (vectors 32-47): Device interrupts
//! - **Syscalls** (vector 128): System call interface
//! - **Timer** (vector 32): Preemption and scheduling
//!
//! ## Timer Interrupt
//!
//! The timer interrupt (default 100Hz, 10ms quantum) is used to trigger
//! preemptive context switches in the round-robin scheduler.
//!
//! ## References
//!
//! - Engineering Plan § 3.5 (Interrupt Handling)
//! - x86-64 AMD64 Architecture Programmer's Manual § 8.2 (IDT)
use core::fmt;
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_interrupt_vector_from_u8() {

        assert_eq!(

            InterruptVector::from_u8(0),

            Some(InterruptVector::DivideByZero)

        );

        assert_eq!(InterruptVector::from_u8(32), Some(InterruptVector::Timer));

        assert_eq!(InterruptVector::from_u8(128), Some(InterruptVector::Syscall));

    }

    #[test]

    fn test_interrupt_vector_as_u8() {

        assert_eq!(InterruptVector::DivideByZero.as_u8(), 0);

        assert_eq!(InterruptVector::Timer.as_u8(), 32);

        assert_eq!(InterruptVector::Syscall.as_u8(), 128);

    }

    #[test]

    fn test_interrupt_vector_is_exception() {

        assert!(InterruptVector::DivideByZero.is_exception());

        assert!(InterruptVector::PageFault.is_exception());

        assert!(!InterruptVector::Timer.is_exception());

    }

    #[test]

    fn test_interrupt_vector_is_irq() {

        assert!(InterruptVector::Timer.is_irq());

        assert!(InterruptVector::Keyboard.is_irq());

        assert!(!InterruptVector::DivideByZero.is_irq());

    }

    #[test]

    fn test_interrupt_vector_display() {

        let msg = InterruptVector::Timer.to_string();

        assert!(msg.contains("Timer"));

        let msg2 = InterruptVector::PageFault.to_string();

        assert!(msg2.contains("PageFault"));

    }

    #[test]

    fn test_timer_config_standard() {

        let config = TimerConfig::standard();

        assert_eq!(config.frequency_hz, 100);

        assert_eq!(config.quantum_ms, 10);

    }

    #[test]

    fn test_timer_config_custom() {

        let config = TimerConfig::with_frequency(200);

        assert_eq!(config.frequency_hz, 200);

        assert_eq!(config.quantum_ms, 5);

    }

    #[test]

    fn test_idt_creation() {

        let idt = InterruptDescriptorTable::new();

        assert_eq!(idt.handler_count(), 0);

    }

    #[test]

    fn test_idt_register_handler() {

        let mut idt = InterruptDescriptorTable::new();

        let result = idt.register_handler(32, 0x1000);

        assert!(result.is_ok());

        assert!(idt.is_registered(32));

        assert_eq!(idt.handler_count(), 1);

    }

    #[test]

    fn test_idt_register_duplicate() {

        let mut idt = InterruptDescriptorTable::new();

        let _ = idt.register_handler(32, 0x1000);

        let result = idt.register_handler(32, 0x2000);

        assert!(result.is_err());

    }

    #[test]

    fn test_idt_unregister_handler() {

        let mut idt = InterruptDescriptorTable::new();

        let _ = idt.register_handler(32, 0x1000);

        let result = idt.unregister_handler(32);

        assert!(result.is_ok());

        assert!(!idt.is_registered(32));

    }

    #[test]

    fn test_idt_unregister_not_registered() {

        let mut idt = InterruptDescriptorTable::new();

        let result = idt.unregister_handler(32);

        assert!(result.is_err());

    }

    #[test]

    fn test_idt_get_handler() {

        let mut idt = InterruptDescriptorTable::new();

        let _ = idt.register_handler(32, 0x1000);

        assert_eq!(idt.get_handler(32), Some(0x1000));

        assert_eq!(idt.get_handler(33), None);

    }

    #[test]

    fn test_idt_multiple_handlers() {

        let mut idt = InterruptDescriptorTable::new();

        for i in 0..10 {

            let _ = idt.register_handler(i, (0x1000 + i as u64) * 0x100);

        }

        assert_eq!(idt.handler_count(), 10);

    }

    #[test]

    fn test_idt_set_timer_config() {

        let mut idt = InterruptDescriptorTable::new();

        let config = TimerConfig::with_frequency(50);

        idt.set_timer_config(config);

        assert_eq!(idt.timer_config().frequency_hz, 50);

    }

    #[test]

    fn test_interrupt_error_display() {

        let err = InterruptError::InvalidVector { vector: 256 };

        let msg = err.to_string();

        assert!(msg.contains("Invalid"));

    }

    #[test]

    fn test_idt_load() {

        let idt = InterruptDescriptorTable::new();

        let result = idt.load();

        assert!(result.is_ok());

    }


