// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Physical Memory Manager
//!
//! This module implements physical memory management for the Cognitive Substrate kernel.
//! It provides bitmap-based page frame allocation and tracking, with support for
//! multiple page sizes and memory region types.
//!
//! ## Features
//!
//! - Bitmap-based page frame allocator for O(1) amortized allocation
//! - Support for multiple page sizes (4KiB, 2MiB, 1GiB)
//! - Memory region classification (Usable, Reserved, ACPI, Firmware)
//! - Free/allocation tracking with double-free detection
//! - Memory statistics collection
//!
//! ## References
//!
//! - Engineering Plan § 3.3 (Memory Management)
//! - Engineering Plan § 4.3 (Error Handling & Recovery)
use core::fmt;
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;


    #[test]

    fn test_page_size_bytes() {

        assert_eq!(PageSize::Small.bytes(), 4096);

        assert_eq!(PageSize::Large.bytes(), 2 * 1024 * 1024);

        assert_eq!(PageSize::Huge.bytes(), 1024 * 1024 * 1024);

    }

    #[test]

    fn test_page_size_bits() {

        assert_eq!(PageSize::Small.bits(), 12);

        assert_eq!(PageSize::Large.bits(), 21);

        assert_eq!(PageSize::Huge.bits(), 30);

    }

    #[test]

    fn test_memory_region_contains() {

        let region = MemoryRegion::new(0x1000, 0x1000, MemoryRegionType::Usable);

        assert!(region.contains(0x1000));

        assert!(region.contains(0x1500));

        assert!(!region.contains(0x0FFF));

        assert!(!region.contains(0x2000));

    }

    #[test]

    fn test_page_frame_contains() {

        let frame = PageFrame::new(0x1000, PageSize::Small, FrameState::Free);

        assert!(frame.contains(0x1000));

        assert!(frame.contains(0x1500));

        assert!(!frame.contains(0x0FFF));

    }

    #[test]

    fn test_memory_stats_free_percent() {

        let stats = MemoryStats {

            total_frames: 1000,

            free_frames: 500,

            allocated_frames: 500,

            reserved_frames: 0,

        };

        assert_eq!(stats.free_percent(), 50);

    }

    #[test]

    fn test_memory_stats_is_low() {

        let stats = MemoryStats {

            total_frames: 1000,

            free_frames: 50,

            allocated_frames: 950,

            reserved_frames: 0,

        };

        assert!(stats.is_low_memory(10));

        assert!(!stats.is_low_memory(1));

    }

    #[test]

    fn test_allocator_from_simple_map() {

        let regions = vec![MemoryRegion::new(0x0, 1024 * 1024, MemoryRegionType::Usable)];

        let allocator = PageFrameAllocator::from_memory_map(&regions);

        assert!(allocator.is_ok());

        let alloc = allocator.unwrap();

        assert!(alloc.total_frame_count() > 0);

        assert!(alloc.free_frame_count() > 0);

    }

    #[test]

    fn test_allocator_allocation() {

        let regions = vec![MemoryRegion::new(0x0, 1024 * 1024, MemoryRegionType::Usable)];

        let mut allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        let before_free = allocator.free_frame_count();

        let frame = allocator.allocate_frame(PageSize::Small);

        assert!(frame.is_ok());

        let frame = frame.unwrap();

        assert_eq!(frame.state, FrameState::Allocated);

        assert_eq!(allocator.free_frame_count(), before_free - 1);

        assert_eq!(allocator.allocated_frame_count(), 1);

    }

    #[test]

    fn test_allocator_free() {

        let regions = vec![MemoryRegion::new(0x0, 1024 * 1024, MemoryRegionType::Usable)];

        let mut allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        let frame = allocator.allocate_frame(PageSize::Small).unwrap();

        let before_free = allocator.free_frame_count();

        let result = allocator.free_frame(&frame);

        assert!(result.is_ok());

        assert_eq!(allocator.free_frame_count(), before_free + 1);

    }

    #[test]

    fn test_allocator_double_free() {

        let regions = vec![MemoryRegion::new(0x0, 1024 * 1024, MemoryRegionType::Usable)];

        let mut allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        let frame = allocator.allocate_frame(PageSize::Small).unwrap();

        let result1 = allocator.free_frame(&frame);

        assert!(result1.is_ok());

        let result2 = allocator.free_frame(&frame);

        assert!(result2.is_err());

    }

    #[test]

    fn test_allocator_oom() {

        let regions = vec![MemoryRegion::new(0x0, 4096, MemoryRegionType::Usable)];

        let mut allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        // Allocate all available frames

        let _ = allocator.allocate_frame(PageSize::Small);

        // Try to allocate when exhausted

        let result = allocator.allocate_frame(PageSize::Small);

        assert!(result.is_err());

    }

    #[test]

    fn test_allocator_multiple_regions() {

        let regions = vec![

            MemoryRegion::new(0x0, 512 * 1024, MemoryRegionType::Usable),

            MemoryRegion::new(0x80000, 512 * 1024, MemoryRegionType::Usable),

        ];

        let allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        assert!(allocator.total_frame_count() > 100);

    }

    #[test]

    fn test_allocator_stats() {

        let regions = vec![MemoryRegion::new(0x0, 1024 * 1024, MemoryRegionType::Usable)];

        let allocator = PageFrameAllocator::from_memory_map(&regions).unwrap();

        let stats = allocator.stats();

        assert_eq!(stats.total_frames, allocator.total_frame_count());

        assert_eq!(stats.free_frames, allocator.free_frame_count());

        assert_eq!(stats.allocated_frames, allocator.allocated_frame_count());

    }

    #[test]

    fn test_memory_error_display() {

        let err = MemoryError::OutOfMemory {

            requested_frames: 100,

            available_frames: 10,

        };

        let msg = err.to_string();

        assert!(msg.contains("Out of memory"));

        let err2 = MemoryError::DoubleFreed {

            address: 0x1000,

            current_state: FrameState::Free,

        };

        let msg2 = err2.to_string();

        assert!(msg2.contains("Double free"));

    }

    #[test]

    fn test_region_type_display() {

        assert_eq!(MemoryRegionType::Usable.to_string(), "Usable");

        assert_eq!(MemoryRegionType::Reserved.to_string(), "Reserved");

    }


