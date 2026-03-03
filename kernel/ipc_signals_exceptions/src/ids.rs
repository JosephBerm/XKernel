// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Strongly Typed ID Types for IPC, Signals, Exceptions, and Checkpointing
//!
//! This module defines all identifier types used throughout the IPC subsystem.
//! Each type is a distinct newtype wrapper around ULID, providing compile-time
//! safety and preventing accidental ID misuse.
//!
//! ## References
//!
//! - Engineering Plan § 4.1 (Domain Model Specification)
//! - Engineering Plan § 4.2.1 (Typed ID System)

use alloc::fmt;
use alloc::string::ToString;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Channel ID - ULID-based identifier for an IPC channel.
///
/// Uniquely identifies a semantic channel instance in the system.
///
/// See Engineering Plan § 4.2.1 (Typed ID System)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ChannelID(Ulid);

impl ChannelID {
    /// Create a new random Channel ID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a Channel ID from a ULID value
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for ChannelID {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ChannelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ChannelID").field(&self.0.to_string()).finish()
    }
}

impl fmt::Display for ChannelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for ChannelID {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ChannelID {}

impl Hash for ChannelID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for ChannelID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChannelID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// Endpoint ID - ULID-based identifier for a channel endpoint (sender or receiver).
///
/// Each endpoint is uniquely identified within a channel pair.
///
/// See Engineering Plan § 4.2.1 (Typed ID System)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct EndpointID(Ulid);

impl EndpointID {
    /// Create a new random Endpoint ID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create an Endpoint ID from a ULID value
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for EndpointID {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EndpointID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EndpointID").field(&self.0.to_string()).finish()
    }
}

impl fmt::Display for EndpointID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for EndpointID {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for EndpointID {}

impl Hash for EndpointID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for EndpointID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EndpointID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// Signal ID - ULID-based identifier for a cognitive signal instance.
///
/// Uniquely identifies a signal sent through the system.
///
/// See Engineering Plan § 4.2.1 (Typed ID System)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SignalID(Ulid);

impl SignalID {
    /// Create a new random Signal ID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a Signal ID from a ULID value
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for SignalID {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SignalID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SignalID").field(&self.0.to_string()).finish()
    }
}

impl fmt::Display for SignalID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for SignalID {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SignalID {}

impl Hash for SignalID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Exception ID - ULID-based identifier for an exception instance.
///
/// Uniquely identifies an exception event in the system.
///
/// See Engineering Plan § 4.2.1 (Typed ID System)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ExceptionID(Ulid);

impl ExceptionID {
    /// Create a new random Exception ID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create an Exception ID from a ULID value
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for ExceptionID {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ExceptionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ExceptionID").field(&self.0.to_string()).finish()
    }
}

impl fmt::Display for ExceptionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for ExceptionID {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ExceptionID {}

impl Hash for ExceptionID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Checkpoint ID - ULID-based identifier for a cognitive checkpoint.
///
/// Uniquely identifies a checkpoint snapshot of CT state.
///
/// See Engineering Plan § 4.2.1 (Typed ID System)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct CheckpointID(Ulid);

impl CheckpointID {
    /// Create a new random Checkpoint ID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create a Checkpoint ID from a ULID value
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the underlying ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for CheckpointID {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CheckpointID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CheckpointID").field(&self.0.to_string()).finish()
    }
}

impl fmt::Display for CheckpointID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for CheckpointID {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for CheckpointID {}

impl Hash for CheckpointID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for CheckpointID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CheckpointID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id_creation() {
        let chan_id1 = ChannelID::new();
        let chan_id2 = ChannelID::new();
        assert_ne!(chan_id1, chan_id2);
    }

    #[test]
    fn test_channel_id_display() {
        let chan_id = ChannelID::new();
        let s = chan_id.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_endpoint_id_equality() {
        let ep_id = EndpointID::new();
        assert_eq!(ep_id, ep_id);
    }

    #[test]
    fn test_signal_id_ordering() {
        let sig1 = SignalID::new();
        let sig2 = SignalID::new();
        let _cmp = sig1.cmp(&sig2);
    }

    #[test]
    fn test_exception_id_hash() {
        use alloc::collections::BTreeSet;
use alloc::format;
        let exc1 = ExceptionID::new();
        let exc2 = ExceptionID::new();
        let mut set = BTreeSet::new();
        set.insert(exc1);
        set.insert(exc2);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_checkpoint_id_ulid_conversion() {
        let ulid = Ulid::new();
        let ckpt = CheckpointID::from_ulid(ulid);
        assert_eq!(ckpt.as_ulid(), ulid);
    }

    #[test]
    fn test_checkpoint_id_debug() {
        let ckpt = CheckpointID::new();
        let _d = alloc::format!("{:?}", ckpt);
    }

    #[test]
    fn test_all_ids_distinct_types() {
        let chan = ChannelID::new();
        let ep = EndpointID::new();
        let sig = SignalID::new();
        let exc = ExceptionID::new();
        let ckpt = CheckpointID::new();
        // Compile-time type safety ensures these cannot be confused
        assert_ne!(chan.as_ulid(), ep.as_ulid());
        assert_ne!(sig.as_ulid(), exc.as_ulid());
        assert_ne!(ckpt.as_ulid(), chan.as_ulid());
    }
}
