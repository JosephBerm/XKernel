// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Discrete capability operations and composition.
//!
//! This module defines the operations that capabilities can grant, and how they compose.
//! See Engineering Plan § 3.1.1: Discrete Operations & Composition.

use core::fmt::{self, Debug, Display};

/// A set of operations that a capability can grant.
///
/// See Engineering Plan § 3.1.1: Discrete Operations & Composition.
/// Operations compose via bitwise union; capabilities can be attenuated by
/// restricting them to a subset of operations.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationSet(u8);

impl OperationSet {
    /// Read operation: permission to observe/retrieve resource state.
    pub const READ: u8 = 0b00001;

    /// Write operation: permission to modify/mutate resource state.
    pub const WRITE: u8 = 0b00010;

    /// Execute operation: permission to run/invoke executable code.
    pub const EXECUTE: u8 = 0b00100;

    /// Invoke operation: permission to call service methods or RPC endpoints.
    pub const INVOKE: u8 = 0b01000;

    /// Subscribe operation: permission to listen to resource event streams.
    pub const SUBSCRIBE: u8 = 0b10000;

    /// Creates an empty operation set (no operations).
    pub const fn empty() -> Self {
        OperationSet(0)
    }

    /// Creates an operation set from raw bits.
    pub const fn from_bits(bits: u8) -> Self {
        OperationSet(bits)
    }

    /// Creates an operation set containing only READ.
    pub const fn read() -> Self {
        OperationSet(Self::READ)
    }

    /// Creates an operation set containing only WRITE.
    pub const fn write() -> Self {
        OperationSet(Self::WRITE)
    }

    /// Creates an operation set containing only EXECUTE.
    pub const fn execute() -> Self {
        OperationSet(Self::EXECUTE)
    }

    /// Creates an operation set containing only INVOKE.
    pub const fn invoke() -> Self {
        OperationSet(Self::INVOKE)
    }

    /// Creates an operation set containing only SUBSCRIBE.
    pub const fn subscribe() -> Self {
        OperationSet(Self::SUBSCRIBE)
    }

    /// Creates an operation set containing all operations.
    pub const fn all() -> Self {
        OperationSet(Self::READ | Self::WRITE | Self::EXECUTE | Self::INVOKE | Self::SUBSCRIBE)
    }

    /// Returns the underlying bitfield.
    pub const fn bits(&self) -> u8 {
        self.0
    }

    /// Checks if this set contains the specified operation.
    pub const fn contains(&self, op: u8) -> bool {
        (self.0 & op) == op
    }

    /// Returns true if READ is contained in this set.
    pub const fn contains_read(&self) -> bool {
        self.contains(Self::READ)
    }

    /// Returns true if WRITE is contained in this set.
    pub const fn contains_write(&self) -> bool {
        self.contains(Self::WRITE)
    }

    /// Returns true if EXECUTE is contained in this set.
    pub const fn contains_execute(&self) -> bool {
        self.contains(Self::EXECUTE)
    }

    /// Returns true if INVOKE is contained in this set.
    pub const fn contains_invoke(&self) -> bool {
        self.contains(Self::INVOKE)
    }

    /// Returns true if SUBSCRIBE is contained in this set.
    pub const fn contains_subscribe(&self) -> bool {
        self.contains(Self::SUBSCRIBE)
    }

    /// Returns true if this set is empty.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns the union of this set with another.
    pub const fn union(&self, other: OperationSet) -> OperationSet {
        OperationSet(self.0 | other.0)
    }

    /// Returns the intersection of this set with another.
    pub const fn intersection(&self, other: OperationSet) -> OperationSet {
        OperationSet(self.0 & other.0)
    }

    /// Returns the bitwise difference: operations in self but not in other.
    pub const fn difference(&self, other: OperationSet) -> OperationSet {
        OperationSet(self.0 & !other.0)
    }

    /// Checks if this set is a subset of another (all operations in self are in other).
    pub const fn is_subset_of(&self, other: OperationSet) -> bool {
        (self.0 & other.0) == self.0
    }

    /// Checks if this set is a superset of another (all operations in other are in self).
    pub const fn is_superset_of(&self, other: OperationSet) -> bool {
        other.is_subset_of(*self)
    }

    /// Returns the number of distinct operations in this set.
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Creates a new set by adding a single operation.
    pub const fn with(&self, op: u8) -> OperationSet {
        OperationSet(self.0 | op)
    }

    /// Creates a new set by removing a single operation.
    pub const fn without(&self, op: u8) -> OperationSet {
        OperationSet(self.0 & !op)
    }
}

impl Debug for OperationSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ops = Vec::new();
        if self.contains_read() {
            ops.push("Read");
        }
        if self.contains_write() {
            ops.push("Write");
        }
        if self.contains_execute() {
            ops.push("Execute");
        }
        if self.contains_invoke() {
            ops.push("Invoke");
        }
        if self.contains_subscribe() {
            ops.push("Subscribe");
        }
        f.debug_set().entries(ops).finish()
    }
}

impl Display for OperationSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        if self.contains_read() {
            if !first {
                write!(f, "|")?;
            }
            write!(f, "Read")?;
            first = false;
        }
        if self.contains_write() {
            if !first {
                write!(f, "|")?;
            }
            write!(f, "Write")?;
            first = false;
        }
        if self.contains_execute() {
            if !first {
                write!(f, "|")?;
            }
            write!(f, "Execute")?;
            first = false;
        }
        if self.contains_invoke() {
            if !first {
                write!(f, "|")?;
            }
            write!(f, "Invoke")?;
            first = false;
        }
        if self.contains_subscribe() {
            if !first {
                write!(f, "|")?;
            }
            write!(f, "Subscribe")?;
            first = false;
        }
        if first {
            write!(f, "(empty)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_operation_set_empty() {
        let ops = OperationSet::empty();
        assert!(ops.is_empty());
        assert_eq!(ops.count(), 0);
    }

    #[test]
    fn test_operation_set_read() {
        let ops = OperationSet::read();
        assert!(!ops.is_empty());
        assert!(ops.contains_read());
        assert!(!ops.contains_write());
    }

    #[test]
    fn test_operation_set_all() {
        let ops = OperationSet::all();
        assert!(ops.contains_read());
        assert!(ops.contains_write());
        assert!(ops.contains_execute());
        assert!(ops.contains_invoke());
        assert!(ops.contains_subscribe());
        assert_eq!(ops.count(), 5);
    }

    #[test]
    fn test_operation_set_union() {
        let read = OperationSet::read();
        let write = OperationSet::write();
        let combined = read.union(write);
        assert!(combined.contains_read());
        assert!(combined.contains_write());
        assert_eq!(combined.count(), 2);
    }

    #[test]
    fn test_operation_set_intersection() {
        let all = OperationSet::all();
        let read_write = OperationSet::read().union(OperationSet::write());
        let result = all.intersection(read_write);
        assert_eq!(result, read_write);
    }

    #[test]
    fn test_operation_set_difference() {
        let all = OperationSet::all();
        let read = OperationSet::read();
        let result = all.difference(read);
        assert!(!result.contains_read());
        assert!(result.contains_write());
        assert!(result.contains_execute());
    }

    #[test]
    fn test_operation_set_is_subset_of() {
        let read = OperationSet::read();
        let read_write = OperationSet::read().union(OperationSet::write());
        let all = OperationSet::all();

        assert!(read.is_subset_of(read_write));
        assert!(read.is_subset_of(all));
        assert!(!read_write.is_subset_of(read));
    }

    #[test]
    fn test_operation_set_is_superset_of() {
        let read = OperationSet::read();
        let all = OperationSet::all();

        assert!(all.is_superset_of(read));
        assert!(!read.is_superset_of(all));
    }

    #[test]
    fn test_operation_set_with_without() {
        let ops = OperationSet::read();
        let with_write = ops.with(OperationSet::WRITE);
        assert!(with_write.contains_read());
        assert!(with_write.contains_write());

        let without_read = with_write.without(OperationSet::READ);
        assert!(!without_read.contains_read());
        assert!(without_read.contains_write());
    }

    #[test]
    fn test_operation_set_debug() {
        let ops = OperationSet::read().union(OperationSet::write());
        let debug_str = format!("{:?}", ops);
        assert!(debug_str.contains("Read"));
        assert!(debug_str.contains("Write"));
    }

    #[test]
    fn test_operation_set_display() {
        let ops = OperationSet::all();
        let display_str = ops.to_string();
        assert!(display_str.contains("Read"));
        assert!(display_str.contains("Write"));
    }

    #[test]
    fn test_operation_set_from_bits() {
        let ops = OperationSet::from_bits(0b00011); // READ | WRITE
        assert!(ops.contains_read());
        assert!(ops.contains_write());
        assert!(!ops.contains_execute());
    }
}
