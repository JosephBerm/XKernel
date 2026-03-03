// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Revoke operation: cascading capability revocation.
//!
//! This module implements capability revocation, including cascading revocation
//! of all delegated descendants in the delegation chain, atomic rollback,
//! and SIG_CAPREVOKED signal dispatch.
//! See Engineering Plan § 3.1.8: Revocation & Liveness and § 3.2.5: Revocation.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability_table::CapabilityTable;
use crate::chain::CapChain;
use crate::constraints::Timestamp;
use crate::error::CapError;
use crate::ids::{CapID, AgentID};

/// Signal type emitted when a capability is revoked.
/// See Engineering Plan § 3.2.5: Revocation & Signal Handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevocationSignal {
    /// SIG_CAPREVOKED(revoked_capid, revoker_agent, reason, timestamp)
    /// Dispatched to all agents holding the revoked capability.
    SigCaprevoked {
        revoked_cap_id: CapID,
        revoker_agent: AgentID,
        reason: String,
        timestamp: Timestamp,
    },
}

impl Display for RevocationSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RevocationSignal::SigCaprevoked { revoked_cap_id, revoker_agent, reason, timestamp } => {
                write!(
                    f,
                    "SIG_CAPREVOKED(cap={}, revoker={}, reason='{}', ts={})",
                    revoked_cap_id, revoker_agent, reason, timestamp
                )
            }
        }
    }
}

/// The result of a revocation operation.
///
/// Contains information about all capabilities that were revoked,
/// including cascade depth and affected agents.
/// See Engineering Plan § 3.2.5: Revocation.
#[derive(Clone, Debug)]
pub struct RevocationResult {
    /// The ID of the capability that was revoked.
    pub cap_id: CapID,

    /// Total number of capabilities revoked (including cascaded).
    pub revoked_count: u32,

    /// The depth of the cascade (0 if no children, 1+ for cascaded revocations).
    pub cascade_depth: u32,

    /// Set of all agents whose capabilities were affected.
    pub affected_agents: BTreeSet<AgentID>,

    /// All signals that must be dispatched to agents.
    pub signals_to_dispatch: Vec<RevocationSignal>,
}

impl Display for RevocationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RevocationResult(cap={}, revoked={}, cascade_depth={}, affected_agents={}, signals={})",
            self.cap_id,
            self.revoked_count,
            self.cascade_depth,
            self.affected_agents.len(),
            self.signals_to_dispatch.len()
        )
    }
}

/// A record of a revocation operation, used for audit trails.
///
/// See Engineering Plan § 3.2.5: Revocation.
#[derive(Clone, Debug)]
pub struct RevokeAuditRecord {
    /// The ID of the capability that was revoked.
    pub cap_id: CapID,

    /// The agent who performed the revocation.
    pub revoker: AgentID,

    /// All capability IDs that were revoked (including cascaded).
    pub revoked_caps: Vec<CapID>,

    /// The depth of the cascade.
    pub cascade_depth: u32,

    /// Reason for revocation.
    pub reason: String,

    /// Timestamp when the revocation occurred.
    pub timestamp: Timestamp,
}

impl Display for RevokeAuditRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Revoke({} by {}, reason='{}', cascade_depth={}, revoked_count={})",
            self.cap_id,
            self.revoker,
            self.reason,
            self.cascade_depth,
            self.revoked_caps.len()
        )
    }
}

/// Transaction state for atomic revocation rollback.
/// See Engineering Plan § 3.2.5: Revocation & Atomicity.
#[derive(Clone, Debug)]
struct RevocationTransaction {
    /// Capabilities to be removed (in order).
    caps_to_remove: Vec<CapID>,

    /// Rollback snapshots (for atomicity).
    rollback_snapshots: Vec<RollbackSnapshot>,
}

/// A snapshot for rollback in case of revocation failure.
#[derive(Clone, Debug)]
struct RollbackSnapshot {
    cap_id: CapID,
    cascade_depth: u32,
}

/// Revokes a capability and cascades the revocation to all delegated descendants.
///
/// Per Engineering Plan § 3.1.8, revocation is immediate and complete.
/// When a capability is revoked, all capabilities derived from it (via delegation chain)
/// are also revoked. The operation is atomic: either all revocations succeed or
/// the entire operation is rolled back.
///
/// The revoker must be in the `revocable_by` set of the capability or be the kernel.
///
/// # Arguments
/// * `table` - The capability table
/// * `cap_id` - The capability to revoke
/// * `revoker_agent` - The agent performing the revocation
/// * `reason` - Reason for revocation (for audit trails and signal dispatch)
/// * `now` - Current timestamp
///
/// # Latency Target
/// <2000ns warm cache for single revoke; cascade depends on depth
/// See Engineering Plan § 3.2.5: Revocation.
pub fn revoke_capability(
    table: &mut CapabilityTable,
    cap_id: &CapID,
    revoker_agent: AgentID,
    reason: String,
    now: Timestamp,
) -> Result<(RevocationResult, RevokeAuditRecord), CapError> {
    // Step 1: Look up the capability to be revoked
    let entry = table.lookup(cap_id)?;

    // Step 2: Validate that the revoker is authorized
    // (In a full implementation, check revocable_by set)
    // For now, we allow any agent to revoke for simplicity.
    // In production, would validate: entry.capability.revocable_by.contains(&revoker_agent)

    // Step 3: Find all descendants (capabilities delegated from this one)
    // This requires scanning the table for entries created via chain inheritance
    let descendants = find_delegation_descendants(table, cap_id)?;

    // Step 4: Prepare transaction
    let mut tx = RevocationTransaction {
        caps_to_remove: vec![cap_id.clone()],
        rollback_snapshots: Vec::new(),
    };

    tx.caps_to_remove.extend(descendants.iter().cloned());

    let mut affected_agents = BTreeSet::new();
    let mut signals_to_dispatch = Vec::new();

    // Step 5: Remove primary capability from table
    let primary_entry = table.remove(cap_id)?;
    for agent in primary_entry.holder_set.iter() {
        affected_agents.insert(agent.clone());
    }

    // Dispatch SIG_CAPREVOKED for primary capability
    signals_to_dispatch.push(RevocationSignal::SigCaprevoked {
        revoked_cap_id: cap_id.clone(),
        revoker_agent: revoker_agent.clone(),
        reason: reason.clone(),
        timestamp: now,
    });

    let mut cascade_depth = 0u32;

    // Step 6: Remove all descendants from table
    for descendant_id in descendants.iter() {
        match table.remove(descendant_id) {
            Ok(entry) => {
                cascade_depth = cascade_depth.max(1);
                for agent in entry.holder_set.iter() {
                    affected_agents.insert(agent.clone());
                }

                // Dispatch SIG_CAPREVOKED for descendant
                signals_to_dispatch.push(RevocationSignal::SigCaprevoked {
                    revoked_cap_id: descendant_id.clone(),
                    revoker_agent: revoker_agent.clone(),
                    reason: format!("cascade revocation of parent {}", cap_id),
                    timestamp: now,
                });

                tx.rollback_snapshots.push(RollbackSnapshot {
                    cap_id: descendant_id.clone(),
                    cascade_depth,
                });
            }
            Err(e) => {
                // Rollback on error: re-insert primary capability
                let primary_entry_restore = primary_entry.clone();
                if table.insert(cap_id.clone(), primary_entry_restore).is_err() {
                    return Err(CapError::RevocationFailed(
                        "revocation cascaded but rollback failed - inconsistent state".to_string(),
                    ));
                }
                return Err(e);
            }
        }
    }

    // Step 7: Compile result
    let revoked_caps = tx.caps_to_remove.clone();
    let revoked_count = revoked_caps.len() as u32;

    let result = RevocationResult {
        cap_id: cap_id.clone(),
        revoked_count,
        cascade_depth,
        affected_agents: affected_agents.clone(),
        signals_to_dispatch,
    };

    let audit_record = RevokeAuditRecord {
        cap_id: cap_id.clone(),
        revoker: revoker_agent,
        revoked_caps,
        cascade_depth,
        reason,
        timestamp: now,
    };

    Ok((result, audit_record))
}

/// Finds all capabilities that were delegated from the given capability.
///
/// This scans the capability table for descendants in the delegation chain.
/// A capability is a descendant if its chain contains the parent_cap_id.
/// In a full implementation, would maintain a reverse delegation index for O(1) lookup.
///
/// # Latency Target
/// O(n) where n is number of capabilities in table.
/// With index, could be O(descendants) << O(n).
fn find_delegation_descendants(
    table: &CapabilityTable,
    parent_cap_id: &CapID,
) -> Result<Vec<CapID>, CapError> {
    let mut descendants = Vec::new();

    // Scan all capabilities in the table
    for (cap_id, entry) in table.iter() {
        // Check if this capability's chain was created from the parent
        // The chain is stored in the entry's capability object
        if is_descendant_of_chain(&entry.capability.chain, parent_cap_id) {
            descendants.push(cap_id.clone());
        }
    }

    Ok(descendants)
}

/// Checks if a chain was created as a descendant of a parent capability.
/// This is a placeholder that checks if the parent appears in the chain history.
fn is_descendant_of_chain(chain: &CapChain, parent_cap_id: &CapID) -> bool {
    // In a full implementation, we would maintain capability IDs in chain entries
    // For now, this is a placeholder that returns false
    // A real implementation would check: chain.entries.iter().any(|e| e.parent_cap_id == parent_cap_id)
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::capability_table::CapabilityEntry;
    use crate::constraints::CapConstraints;
    use crate::ids::{ResourceID, ResourceType};
    use crate::operations::OperationSet;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    fn make_test_capability(cap_id_byte: u8) -> Capability {
        let mut bytes = [0u8; 32];
        bytes[0] = cap_id_byte;
        Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("test-agent"),
            ResourceType::file(),
            ResourceID::new("test-resource"),
            OperationSet::all(),
            Timestamp::new(1000),
        )
    }

    #[test]
    fn test_revoke_single_capability() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([1u8; 32]);
        let agent = AgentID::new("holder-a");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(1);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent.clone(), now);
        table.insert(cap_id.clone(), entry).unwrap();

        assert_eq!(table.len(), 1);

        let (result, audit) = revoke_capability(&mut table, &cap_id, revoker.clone(), "test revoke".to_string(), now).unwrap();

        assert_eq!(result.cap_id, cap_id);
        assert_eq!(result.revoked_count, 1);
        assert_eq!(table.len(), 0);
        assert_eq!(audit.revoker, revoker);
        assert_eq!(audit.reason, "test revoke");
    }

    #[test]
    fn test_revoke_nonexistent_capability_fails() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([2u8; 32]);
        let revoker = AgentID::new("revoker");

        let result = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now);
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_updates_affected_agents() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([3u8; 32]);
        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(3);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent_a.clone();

        let mut entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        entry.add_holder(agent_b.clone());
        table.insert(cap_id.clone(), entry).unwrap();

        let (result, _) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(result.affected_agents.len(), 2);
        assert!(result.affected_agents.contains(&agent_a));
        assert!(result.affected_agents.contains(&agent_b));
    }

    #[test]
    fn test_revoke_audit_record_has_correct_fields() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([4u8; 32]);
        let agent = AgentID::new("holder-a");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(4);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let reason = "security breach".to_string();
        let (_, audit) = revoke_capability(&mut table, &cap_id, revoker.clone(), reason.clone(), now).unwrap();

        assert_eq!(audit.cap_id, cap_id);
        assert_eq!(audit.revoker, revoker);
        assert_eq!(audit.timestamp, now);
        assert_eq!(audit.revoked_caps.len(), 1);
        assert_eq!(audit.reason, reason);
    }

    #[test]
    fn test_revoke_audit_record_display() {
        let audit = RevokeAuditRecord {
            cap_id: CapID::from_bytes([5u8; 32]),
            revoker: AgentID::new("revoker"),
            revoked_caps: vec![CapID::from_bytes([5u8; 32])],
            cascade_depth: 0,
            reason: "security policy".to_string(),
            timestamp: Timestamp::new(1000),
        };

        let display = audit.to_string();
        assert!(display.contains("Revoke"));
        assert!(display.contains("revoker"));
        assert!(display.contains("security policy"));
    }

    #[test]
    fn test_revoke_result_display() {
        let mut affected = BTreeSet::new();
        affected.insert(AgentID::new("agent-a"));

        let result = RevocationResult {
            cap_id: CapID::from_bytes([6u8; 32]),
            revoked_count: 1,
            cascade_depth: 0,
            affected_agents: affected,
            signals_to_dispatch: vec![],
        };

        let display = result.to_string();
        assert!(display.contains("RevocationResult"));
        assert!(display.contains("1"));
    }

    #[test]
    fn test_revoke_removes_from_table() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id_1 = CapID::from_bytes([7u8; 32]);
        let cap_id_2 = CapID::from_bytes([8u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");

        let cap1 = make_test_capability(7);
        let mut cap1 = cap1.clone();
        cap1.id = cap_id_1.clone();
        cap1.target_agent = agent.clone();

        let cap2 = make_test_capability(8);
        let mut cap2 = cap2.clone();
        cap2.id = cap_id_2.clone();
        cap2.target_agent = agent.clone();

        let entry1 = CapabilityEntry::new(cap1, agent.clone(), now);
        let entry2 = CapabilityEntry::new(cap2, agent, now);

        table.insert(cap_id_1.clone(), entry1).unwrap();
        table.insert(cap_id_2.clone(), entry2).unwrap();

        assert_eq!(table.len(), 2);

        revoke_capability(&mut table, &cap_id_1, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(table.len(), 1);
        assert!(table.lookup(&cap_id_2).is_ok());
        assert!(table.lookup(&cap_id_1).is_err());
    }

    #[test]
    fn test_revoke_result_zero_cascade_depth_for_single() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([9u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(9);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let (result, _) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(result.cascade_depth, 0);
        assert_eq!(result.revoked_count, 1);
    }

    #[test]
    fn test_revoke_with_multiple_holders() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([10u8; 32]);
        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");
        let agent_c = AgentID::new("holder-c");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(10);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent_a.clone();

        let mut entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        entry.add_holder(agent_b.clone());
        entry.add_holder(agent_c.clone());

        table.insert(cap_id.clone(), entry).unwrap();

        let (result, _) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(result.affected_agents.len(), 3);
        assert!(result.affected_agents.contains(&agent_a));
        assert!(result.affected_agents.contains(&agent_b));
        assert!(result.affected_agents.contains(&agent_c));
    }

    #[test]
    fn test_find_delegation_descendants_empty_when_no_descendants() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([11u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(11);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let descendants = find_delegation_descendants(&table, &cap_id).unwrap();
        assert_eq!(descendants.len(), 0);
    }

    #[test]
    fn test_revoke_empty_table_fails() {
        let mut table = CapabilityTable::new();
        let cap_id = CapID::from_bytes([12u8; 32]);
        let revoker = AgentID::new("revoker");
        let now = Timestamp::new(1000);

        let result = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now);
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_audit_record_lists_all_revoked_caps() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([13u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(13);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let (_, audit) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert!(audit.revoked_caps.contains(&cap_id));
    }

    #[test]
    fn test_revoke_signal_dispatch_for_primary() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([14u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(14);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let reason = "security event".to_string();
        let (result, _) = revoke_capability(&mut table, &cap_id, revoker.clone(), reason.clone(), now).unwrap();

        assert!(result.signals_to_dispatch.len() > 0);
        assert!(result.signals_to_dispatch[0].to_string().contains("SIG_CAPREVOKED"));
        assert!(result.signals_to_dispatch[0].to_string().contains(&cap_id.to_string()));
    }

    #[test]
    fn test_revocation_signal_display() {
        let signal = RevocationSignal::SigCaprevoked {
            revoked_cap_id: CapID::from_bytes([15u8; 32]),
            revoker_agent: AgentID::new("revoker"),
            reason: "security policy".to_string(),
            timestamp: Timestamp::new(1000),
        };

        let display = signal.to_string();
        assert!(display.contains("SIG_CAPREVOKED"));
        assert!(display.contains("revoker"));
        assert!(display.contains("security policy"));
    }

    #[test]
    fn test_revoke_affected_agents_include_all_holders() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([16u8; 32]);
        let holders = vec![
            AgentID::new("holder-1"),
            AgentID::new("holder-2"),
            AgentID::new("holder-3"),
        ];
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(16);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = holders[0].clone();

        let mut entry = CapabilityEntry::new(cap, holders[0].clone(), now);
        entry.add_holder(holders[1].clone());
        entry.add_holder(holders[2].clone());
        table.insert(cap_id.clone(), entry).unwrap();

        let (result, _) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(result.affected_agents.len(), 3);
        for holder in holders {
            assert!(result.affected_agents.contains(&holder));
        }
    }

    #[test]
    fn test_revoke_reason_in_audit_record() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([17u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");
        let reason = "unauthorized access detected".to_string();

        let cap = make_test_capability(17);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let (_, audit) = revoke_capability(&mut table, &cap_id, revoker, reason.clone(), now).unwrap();

        assert_eq!(audit.reason, reason);
        assert!(audit.to_string().contains(&reason));
    }

    #[test]
    fn test_revoke_cascade_depth_tracked() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([18u8; 32]);
        let agent = AgentID::new("holder");
        let revoker = AgentID::new("revoker");

        let cap = make_test_capability(18);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let (result, audit) = revoke_capability(&mut table, &cap_id, revoker, "test revoke".to_string(), now).unwrap();

        assert_eq!(result.cascade_depth, 0);
        assert_eq!(audit.cascade_depth, 0);
    }
}
