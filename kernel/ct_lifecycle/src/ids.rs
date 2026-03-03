// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Strongly Typed ID Types
//!
//! This module defines all identifier types used throughout the CT Lifecycle subsystem.
//! Each type is a distinct newtype wrapper around Ulid or u64, providing compile-time
//! safety and preventing accidental ID misuse.
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 4.2.1 (Typed ID System)
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use super::*;

use alloc::string::{String, ToString};
use alloc::collections::BTreeSet;
use alloc::fmt;
use alloc::format;
use ulid::Ulid;


        let cap1 = CapID::new();

        let cap2 = CapID::new();

        let mut set = BTreeSet::new();

        set.insert(cap1);

        set.insert(cap2);

        assert_eq!(set.len(), 2);

    

    #[test]

    fn test_checkpoint_id_ulid_conversion() {

        let ulid = Ulid::new();

        let ckpt = CheckpointID::from_ulid(ulid);

        assert_eq!(ckpt.as_ulid(), ulid);

    }

    #[test]

    fn test_trace_id_debug() {

        let trace = TraceID::new();

        let _d = alloc::format!("{:?}", trace);

    }

    #[test]

    fn test_channel_id_distinct_type() {

        let chan = ChannelID::new();

        let agent = AgentID::new();

        // Compile-time type safety ensures these cannot be confused

        assert_ne!(chan.as_ulid(), agent.as_ulid());

    }


