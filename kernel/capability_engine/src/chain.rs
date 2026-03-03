// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability provenance tracking via delegation chains.
//!
//! This module defines the chain of delegation, which tracks the complete history
//! of a capability's creation and every sub-delegation.
//! See Engineering Plan § 3.1.2: Provenance & Chains.

use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::constraints::Timestamp;
use crate::error::CapError;
use crate::ids::AgentID;
use crate::constraints::CapConstraints;

/// A single entry in a capability's delegation chain.
///
/// See Engineering Plan § 3.1.2: Provenance & Chains.
/// Each entry records a delegation event: who granted the capability to whom,
/// what attenuations (restrictions) were applied, and when.
#[derive(Clone, Debug)]
pub struct ChainEntry {
    /// The agent who granted the capability.
    pub from: AgentID,

    /// The agent who received the capability.
    pub to: AgentID,

    /// Constraints applied during this delegation.
    pub attenuated_constraints: CapConstraints,

    /// When this delegation occurred.
    pub timestamp: Timestamp,
}

impl ChainEntry {
    /// Creates a new chain entry.
    pub fn new(
        from: AgentID,
        to: AgentID,
        constraints: CapConstraints,
        timestamp: Timestamp,
    ) -> Self {
        ChainEntry {
            from,
            to,
            attenuated_constraints: constraints,
            timestamp,
        }
    }
}

impl Display for ChainEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} -> {} @ {} ({})",
            self.from, self.to, self.timestamp, self.attenuated_constraints
        )
    }
}

/// A complete delegation chain for a capability.
///
/// See Engineering Plan § 3.1.2: Provenance & Chains.
/// The chain is a vector of delegation events in chronological order.
/// The first entry represents the initial grant (from kernel to an agent).
/// Subsequent entries represent delegations from one agent to another.
#[derive(Clone, Debug, Default)]
pub struct CapChain {
    entries: Vec<ChainEntry>,
}

impl CapChain {
    /// Creates an empty chain.
    pub fn new() -> Self {
        CapChain {
            entries: Vec::new(),
        }
    }

    /// Creates a chain from a vector of entries.
    pub fn from_entries(entries: Vec<ChainEntry>) -> Self {
        CapChain { entries }
    }

    /// Adds an entry to the chain (extends the chain via delegation).
    ///
    /// This is called when a capability is delegated from one agent to another.
    /// It validates that the delegation is allowed and returns an error if not.
    pub fn delegate(
        &mut self,
        from: AgentID,
        to: AgentID,
        attenuated_constraints: CapConstraints,
        now: Timestamp,
    ) -> Result<(), CapError> {
        // Check that the depth does not exceed any limits in existing constraints.
        // The depth is the current length of the chain.
        let current_depth = self.entries.len() as u32;

        // Check all constraints in the chain so far
        for entry in &self.entries {
            entry.attenuated_constraints.can_delegate(current_depth)?;
        }

        // If we reach here, the delegation is allowed
        let entry = ChainEntry::new(from, to, attenuated_constraints, now);
        self.entries.push(entry);
        Ok(())
    }

    /// Returns the current depth of the chain (number of delegations so far).
    pub fn depth(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Returns the root grantor of the capability (the first agent in the chain).
    pub fn root_grantor(&self) -> Option<&AgentID> {
        self.entries.first().map(|e| &e.from)
    }

    /// Returns the current holder of the capability (the last `to` in the chain, or None).
    pub fn current_holder(&self) -> Option<&AgentID> {
        self.entries.last().map(|e| &e.to)
    }

    /// Returns a reference to all entries in the chain.
    pub fn entries(&self) -> &[ChainEntry] {
        &self.entries
    }

    /// Returns the number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds an entry directly to the chain (used during root grant).
    ///
    /// This bypasses the delegation depth checks, used when creating
    /// a new chain entry for a root grant.
    pub fn add_entry(&mut self, entry: ChainEntry) -> Result<(), CapError> {
        self.entries.push(entry);
        Ok(())
    }

    /// Validates that the chain is well-formed.
    ///
    /// In a valid chain:
    /// - Each entry's `to` (receiver) should be the next entry's `from` (granter).
    /// - (This can be relaxed in later versions if parallel delegations are supported.)
    pub fn validate(&self) -> Result<(), CapError> {
        if self.entries.is_empty() {
            return Ok(());
        }

        for i in 1..self.entries.len() {
            let prev_to = &self.entries[i - 1].to;
            let curr_from = &self.entries[i].from;
            if prev_to != curr_from {
                return Err(CapError::InvalidChain(format!(
                    "chain broken at entry {}: {} != {}",
                    i, prev_to, curr_from
                )));
            }
        }

        Ok(())
    }
}

impl Display for CapChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CapChain[")?;
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(f, " -> ")?;
            }
            write!(f, "{}", entry)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_chain_entry_creation() {
        let from = AgentID::new("agent-a");
        let to = AgentID::new("agent-b");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        let entry = ChainEntry::new(from, to, constraints, timestamp);
        assert_eq!(entry.from.as_str(), "agent-a");
        assert_eq!(entry.to.as_str(), "agent-b");
    }

    #[test]
    fn test_cap_chain_empty() {
        let chain = CapChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.depth(), 0);
        assert_eq!(chain.len(), 0);
        assert!(chain.root_grantor().is_none());
    }

    #[test]
    fn test_cap_chain_delegate() {
        let mut chain = CapChain::new();
        let from = AgentID::new("kernel");
        let to = AgentID::new("agent-a");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        let result = chain.delegate(from, to, constraints, timestamp);
        assert!(result.is_ok());
        assert_eq!(chain.depth(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_cap_chain_root_grantor() {
        let mut chain = CapChain::new();
        let kernel = AgentID::new("kernel");
        let agent_a = AgentID::new("agent-a");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        chain
            .delegate(kernel.clone(), agent_a, constraints, timestamp)
            .unwrap();

        assert_eq!(
            chain.root_grantor().map(|a| a.as_str()),
            Some("kernel")
        );
    }

    #[test]
    fn test_cap_chain_current_holder() {
        let mut chain = CapChain::new();
        let kernel = AgentID::new("kernel");
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        chain
            .delegate(kernel, agent_a, constraints.clone(), timestamp)
            .unwrap();
        chain
            .delegate(agent_a, agent_b, constraints, timestamp)
            .unwrap();

        assert_eq!(
            chain.current_holder().map(|a| a.as_str()),
            Some("agent-b")
        );
    }

    #[test]
    fn test_cap_chain_validate_valid() {
        let mut chain = CapChain::new();
        let kernel = AgentID::new("kernel");
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        chain
            .delegate(kernel, agent_a.clone(), constraints.clone(), timestamp)
            .unwrap();
        chain
            .delegate(agent_a, agent_b, constraints, timestamp)
            .unwrap();

        assert!(chain.validate().is_ok());
    }

    #[test]
    fn test_cap_chain_validate_broken() {
        let mut chain = CapChain::new();

        let entry1 = ChainEntry::new(
            AgentID::new("kernel"),
            AgentID::new("agent-a"),
            CapConstraints::new(),
            Timestamp::new(1000),
        );
        let entry2 = ChainEntry::new(
            AgentID::new("agent-c"), // This doesn't match agent-a
            AgentID::new("agent-b"),
            CapConstraints::new(),
            Timestamp::new(2000),
        );

        chain.entries.push(entry1);
        chain.entries.push(entry2);

        assert!(chain.validate().is_err());
    }

    #[test]
    fn test_cap_chain_depth_exceeded() {
        let mut chain = CapChain::new();
        let constraints = {
            let mut c = CapConstraints::new();
            c.chain_depth_limited = Some(crate::constraints::ChainDepthLimit::new(1));
            c
        };

        let timestamp = Timestamp::new(1000);
        let from1 = AgentID::new("kernel");
        let to1 = AgentID::new("agent-a");
        let to2 = AgentID::new("agent-b");

        // First delegation should succeed
        let result1 = chain.delegate(from1, to1.clone(), constraints.clone(), timestamp);
        assert!(result1.is_ok());

        // Second delegation should fail due to depth limit
        let result2 = chain.delegate(to1, to2, constraints, timestamp);
        assert!(result2.is_err());
    }

    #[test]
    fn test_cap_chain_display() {
        let mut chain = CapChain::new();
        let kernel = AgentID::new("kernel");
        let agent_a = AgentID::new("agent-a");
        let constraints = CapConstraints::new();
        let timestamp = Timestamp::new(1000);

        chain
            .delegate(kernel, agent_a, constraints, timestamp)
            .unwrap();

        let display = chain.to_string();
        assert!(display.contains("CapChain"));
        assert!(display.contains("kernel"));
    }

    #[test]
    fn test_cap_chain_from_entries() {
        let entry = ChainEntry::new(
            AgentID::new("kernel"),
            AgentID::new("agent-a"),
            CapConstraints::new(),
            Timestamp::new(1000),
        );

        let chain = CapChain::from_entries(vec![entry]);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.depth(), 1);
    }
}
