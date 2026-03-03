// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Virtual Memory and Page Table Management
//!
//! This module implements virtual memory management for the Cognitive Substrate kernel.
//! It provides x86-64 4-level page table support with page mapping, unmapping,
//! and address space management.
//!
//! ## Page Table Hierarchy (x86-64)
//!
//! - **PML4** (Page Map Level 4): Maps 512 GiB regions
//! - **PDPT** (Page Directory Pointer Table): Maps 1 GiB regions
//! - **PD** (Page Directory): Maps 2 MiB regions
//! - **PT** (Page Table): Maps 4 KiB regions
//!
//! ## Kernel Address Space
//!
//! - **0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF**: User space (47-bit)
//! - **0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF**: Kernel space (higher-half)
//!
//! ## References
//!
//! - Engineering Plan § 3.4 (Virtual Memory)
//! - x86-64 AMD64 Architecture Programmer's Manual
use core::fmt;
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_pte_flags_encode_decode() {

        let flags = PageTableEntryFlags::kernel();

        let encoded = flags.encode();

        let decoded = PageTableEntryFlags::decode(encoded);

        assert_eq!(decoded.present, flags.present);

        assert_eq!(decoded.writable, flags.writable);

        assert_eq!(decoded.global, flags.global);

    }

    #[test]

    fn test_pte_flags_user() {

        let flags = PageTableEntryFlags::user();

        assert!(flags.present);

        assert!(flags.writable);

        assert!(flags.user_accessible);

        assert!(!flags.global);

    }

    #[test]

    fn test_pte_flags_read_only() {

        let flags = PageTableEntryFlags::read_only();

        assert!(flags.present);

        assert!(!flags.writable);

    }

    #[test]

    fn test_page_table_level_operations() {

        let mut level = PageTableLevel::new();

        assert!(!level.is_present(0));

        level.set_entry(0, 0x1000 | 1); // Address | present bit

        assert!(level.is_present(0));

        assert_eq!(level.get_entry(0), 0x1000 | 1);

        level.clear_entry(0);

        assert!(!level.is_present(0));

    }

    #[test]

    fn test_page_table_create() {

        let pt = PageTable::new();

        // Should create without panicking

    }

    #[test]

    fn test_page_table_map() {

        let mut pt = PageTable::new();

        let result = pt.map_page(0x1000, 0x2000, PageTableEntryFlags::kernel());

        assert!(result.is_ok());

    }

    #[test]

    fn test_page_table_map_misaligned() {

        let mut pt = PageTable::new();

        let result = pt.map_page(0x1001, 0x2000, PageTableEntryFlags::kernel());

        assert!(result.is_err());

    }

    #[test]

    fn test_page_table_double_map() {

        let mut pt = PageTable::new();

        let result1 = pt.map_page(0x1000, 0x2000, PageTableEntryFlags::kernel());

        assert!(result1.is_ok());

        let result2 = pt.map_page(0x1000, 0x3000, PageTableEntryFlags::kernel());

        assert!(result2.is_err());

    }

    #[test]

    fn test_page_table_translate() {

        let mut pt = PageTable::new();

        pt.map_page(0x1000, 0x2000, PageTableEntryFlags::kernel())

            .unwrap();

        let result = pt.translate(0x1000);

        assert!(result.is_ok());

        let phys = result.unwrap();

        assert_eq!(phys, 0x2000);

    }

    #[test]

    fn test_page_table_unmap() {

        let mut pt = PageTable::new();

        pt.map_page(0x1000, 0x2000, PageTableEntryFlags::kernel())

            .unwrap();

        let result = pt.unmap_page(0x1000);

        assert!(result.is_ok());

        assert_eq!(result.unwrap(), 0x2000);

        let translate_result = pt.translate(0x1000);

        assert!(translate_result.is_err());

    }

    #[test]

    fn test_page_table_unmap_unmapped() {

        let mut pt = PageTable::new();

        let result = pt.unmap_page(0x1000);

        assert!(result.is_err());

    }

    #[test]

    fn test_page_table_multiple_mappings() {

        let mut pt = PageTable::new();

        for i in 0..10 {

            let vaddr = 0x1000 * (i + 1);

            let paddr = 0x2000 * (i + 1);

            let result = pt.map_page(vaddr, paddr, PageTableEntryFlags::kernel());

            assert!(result.is_ok());

        }

        for i in 0..10 {

            let vaddr = 0x1000 * (i + 1);

            let paddr = 0x2000 * (i + 1);

            let result = pt.translate(vaddr);

            assert!(result.is_ok());

            assert_eq!(result.unwrap(), paddr);

        }

    }

    #[test]

    fn test_mmu_error_display() {

        let err = MmuError::AlignmentError {

            address: 0x1001,

            required_alignment: 4096,

        };

        let msg = err.to_string();

        assert!(msg.contains("Alignment"));

    }


