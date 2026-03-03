// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Strongly Typed ID Types
//!
//! This module defines all identifier types used throughout the CT Lifecycle subsystem.
//! Each type is a distinct newtype wrapper around Ulid, providing compile-time
//! safety and preventing accidental ID misuse.
//!
//! ## References
//!
//! - Engineering Plan S 4.1 (Domain Model Specification)
//! - Engineering Plan S 4.2.1 (Typed ID System)

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use alloc::string::{String, ToString};
use alloc::fmt;
use alloc::format;
use ulid::Ulid;

macro_rules! define_id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy)]
        pub struct $name(Ulid);

        impl $name {
            /// Create a new unique ID.
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Create from a Ulid value.
            pub fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
            }

            /// Get the underlying Ulid.
            pub fn as_ulid(&self) -> Ulid {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Cognitive Task ID.
define_id_type!(CTID);

/// Capability ID.
define_id_type!(CapID);

/// Checkpoint ID.
define_id_type!(CheckpointID);

/// Trace ID.
define_id_type!(TraceID);

/// Channel ID.
define_id_type!(ChannelID);

/// Agent ID.
define_id_type!(AgentID);

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;

    #[test]
    fn test_ctid_creation() {
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_ctid_btree_set() {
        let ct1 = CTID::new();
        let ct2 = CTID::new();
        let mut set = BTreeSet::new();
        set.insert(ct1);
        set.insert(ct2);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_cap_id_btree_set() {
        let cap1 = CapID::new();
        let cap2 = CapID::new();
        let mut set = BTreeSet::new();
        set.insert(cap1);
        set.insert(cap2);
        assert_eq!(set.len(), 2);
    }

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
}
