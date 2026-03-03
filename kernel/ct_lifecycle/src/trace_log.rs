// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Kernel Trace Ring Buffer
//!
//! This module implements a fixed-size circular ring buffer for kernel tracing.
//! It records CT phase transitions with timestamps for debugging and performance analysis.
//!
//! ## Ring Buffer Characteristics
//!
//! - **Size**: Fixed at ~1 MB (approximately 65,000 entries at 16 bytes/entry)
//! - **Overflow**: Circular: oldest entries overwritten when full
//! - **Performance**: O(1) append, read, and iteration
//! - **Ordering**: Entries iterable from oldest to newest
//!
//! ## Trace Entry Format
//!
//! Each entry records:
//! - CT ID (Ulid)
//! - Source phase
//! - Destination phase
//! - Timestamp (nanoseconds)
//! - Transition reason
//!
//! ## References
//!
//! - Engineering Plan § 4.4 (Tracing and Diagnostics)
//! - Engineering Plan § 5.3 (Performance Metrics)
use core::fmt;
use crate::ids::CTID;
use crate::phase::CTPhase;
use super::*;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use ulid::Ulid;


    #[test]

    fn test_ring_buffer_creation() {

        let buf = KernelRingBuffer::new(100);

        assert_eq!(buf.capacity(), 100);

        assert!(buf.is_empty());

        assert_eq!(buf.len(), 0);

    }

    #[test]

    fn test_ring_buffer_push() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        let entry = TraceEntry::new(

            ct_id,

            CTPhase::Spawn,

            CTPhase::Plan,

            1000,

            "Test transition",

        );

        buf.push(entry);

        assert_eq!(buf.len(), 1);

        assert!(!buf.is_empty());

    }

    #[test]

    fn test_ring_buffer_iteration() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        for i in 0..10 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let entries: Vec<_> = buf.iter().collect();

        assert_eq!(entries.len(), 10);

        // Check ordering (oldest to newest)

        for i in 0..10 {

            assert_eq!(entries[i].timestamp_ns, (i * 100) as u64);

        }

    }

    #[test]

    fn test_ring_buffer_wrap_around() {

        let mut buf = KernelRingBuffer::new(10);

        let ct_id = CTID::new();

        // Fill the buffer

        for i in 0..10 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        assert_eq!(buf.len(), 10);

        assert_eq!(buf.wrap_count, 0);

        // Add one more to trigger wrap

        let entry = TraceEntry::new(ct_id, CTPhase::Plan, CTPhase::Reason, 1000, "Test");

        buf.push(entry);

        assert_eq!(buf.len(), 10); // Still 10 (wrapped around)

        assert_eq!(buf.wrap_count, 1); // One wrap detected

    }

    #[test]

    fn test_trace_entry_reason_str() {

        let ct_id = CTID::new();

        let entry = TraceEntry::new(

            ct_id,

            CTPhase::Spawn,

            CTPhase::Plan,

            1000,

            "Initialization",

        );

        assert_eq!(entry.reason_str(), "Initialization");

    }

    #[test]

    fn test_ring_buffer_stats() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        for i in 0..5 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let stats = buf.stats();

        assert_eq!(stats.total_entries, 5);

        assert_eq!(stats.buffer_size, 5);

        assert_eq!(stats.oldest_timestamp_ns, 0);

        assert_eq!(stats.newest_timestamp_ns, 400);

    }

    #[test]

    fn test_ring_buffer_last_entries() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        for i in 0..10 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let last = buf.last_entries(3);

        assert_eq!(last.len(), 3);

        // Most recent first

        assert_eq!(last[0].timestamp_ns, 900);

        assert_eq!(last[1].timestamp_ns, 800);

        assert_eq!(last[2].timestamp_ns, 700);

    }

    #[test]

    fn test_ring_buffer_entries_for_ct() {

        let mut buf = KernelRingBuffer::new(100);

        let ct1 = CTID::new();

        let ct2 = CTID::new();

        for i in 0..5 {

            let entry = TraceEntry::new(

                ct1,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        for i in 0..3 {

            let entry = TraceEntry::new(

                ct2,

                CTPhase::Plan,

                CTPhase::Reason,

                ((i + 5) * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let ct1_entries = buf.entries_for_ct(ct1);

        assert_eq!(ct1_entries.len(), 5);

        let ct2_entries = buf.entries_for_ct(ct2);

        assert_eq!(ct2_entries.len(), 3);

    }

    #[test]

    fn test_ring_buffer_entries_in_range() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        for i in 0..10 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let range_entries = buf.entries_in_range(200, 600);

        assert_eq!(range_entries.len(), 5); // entries at 200, 300, 400, 500, 600

    }

    #[test]

    fn test_ring_buffer_clear() {

        let mut buf = KernelRingBuffer::new(100);

        let ct_id = CTID::new();

        let entry = TraceEntry::new(ct_id, CTPhase::Spawn, CTPhase::Plan, 1000, "Test");

        buf.push(entry);

        assert!(!buf.is_empty());

        buf.clear();

        assert!(buf.is_empty());

        assert_eq!(buf.len(), 0);

    }

    #[test]

    fn test_ring_buffer_default_size() {

        let buf = KernelRingBuffer::new_default_size();

        assert_eq!(buf.capacity(), 65536);

    }

    #[test]

    fn test_trace_entry_long_reason() {

        let ct_id = CTID::new();

        let long_reason = "This is a very long reason that exceeds the fixed size";

        let entry = TraceEntry::new(ct_id, CTPhase::Spawn, CTPhase::Plan, 1000, long_reason);

        // Should be truncated to 31 bytes

        let reason = entry.reason_str();

        assert!(reason.len() <= 31);

    }

    #[test]

    fn test_ring_buffer_total_entries_count() {

        let mut buf = KernelRingBuffer::new(5);

        let ct_id = CTID::new();

        for i in 0..15 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let stats = buf.stats();

        assert_eq!(stats.total_entries, 15); // Counts all pushed, not just current

        assert!(stats.overwrite_count > 0);

    }

    #[test]

    fn test_ring_buffer_large_dataset() {

        let mut buf = KernelRingBuffer::new(1000);

        let ct_id = CTID::new();

        for i in 0..10000 {

            let entry = TraceEntry::new(

                ct_id,

                CTPhase::Spawn,

                CTPhase::Plan,

                (i * 100) as u64,

                "Test",

            );

            buf.push(entry);

        }

        let stats = buf.stats();

        assert_eq!(stats.total_entries, 10000);

        assert!(stats.overwrite_count > 0);

        assert_eq!(stats.buffer_size, 1000); // Limited by capacity

    }


