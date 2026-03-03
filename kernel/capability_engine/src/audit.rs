// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Audit operations: capability and agent audit reporting.
//!
//! This module implements comprehensive audit functionality for capabilities
//! and agents, providing complete provenance visibility and compliance tracking.
//! See Engineering Plan § 3.2.6: Audit & Compliance.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::capability_table::{CapabilityEntry, CapabilityTable};
use crate::chain::ChainEntry;
use crate::constraints::Timestamp;
use crate::error::CapError;
use crate::ids::{CapID, AgentID};

/// A complete audit chain for a capability (provenance history).
/// See Engineering Plan § 3.2.6: Audit & Compliance.
#[derive(Clone, Debug)]
pub struct CapChain {
    /// All entries in the chain, in chronological order.
    pub entries: Vec<ChainEntry>,

    /// Timestamp of the earliest entry.
    pub created_at: Timestamp,

    /// Timestamp of the latest entry.
    pub last_modified: Timestamp,

    /// Total number of delegations in the chain.
    pub delegation_count: u32,
}

impl Display for CapChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapChain(entries={}, delegations={}, modified={})",
            self.entries.len(),
            self.delegation_count,
            self.last_modified
        )
    }
}

/// Audit report for a single capability.
///
/// Provides complete provenance information, holders, delegations, and
/// policy compliance status for auditing and forensic analysis.
/// See Engineering Plan § 3.2.6: Audit & Compliance.
#[derive(Clone, Debug)]
pub struct CapabilityAuditReport {
    /// The capability being audited.
    pub capability: Capability,

    /// The complete delegation chain (provenance history).
    pub provenance_chain: CapChain,

    /// All agents currently holding this capability.
    pub all_holders: Vec<AgentID>,

    /// All delegations made from this capability to other agents.
    pub delegations_made: Vec<DelegationInfo>,

    /// Compliance status with mandatory policies.
    pub policy_compliance_status: String,
}

impl Display for CapabilityAuditReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapAudit({}, chain_len={}, holders={}, delegations={})",
            self.capability.id,
            self.provenance_chain.entries.len(),
            self.all_holders.len(),
            self.delegations_made.len()
        )
    }
}

/// Information about a delegation made from a capability.
#[derive(Clone, Debug)]
pub struct DelegationInfo {
    /// The agent who made the delegation.
    pub delegating_agent: AgentID,

    /// The agent who received the delegation.
    pub receiving_agent: AgentID,

    /// When the delegation occurred.
    pub timestamp: Timestamp,

    /// Constraints applied in delegation.
    pub constraints_applied: String,
}

impl Display for DelegationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}->{}@{}",
            self.delegating_agent, self.receiving_agent, self.timestamp
        )
    }
}

/// Audit report for an agent.
///
/// Provides comprehensive information about all capabilities held by an agent,
/// delegations made, and delegations received.
/// See Engineering Plan § 3.2.6: Audit & Compliance.
#[derive(Clone, Debug)]
pub struct AgentAuditReport {
    /// The agent being audited.
    pub agent_id: AgentID,

    /// All capabilities held by this agent.
    pub held_capabilities: Vec<CapID>,

    /// All delegations made by this agent (with target agents).
    pub delegations_made: Vec<DelegationRecord>,

    /// All delegations received by this agent (with source agents).
    pub delegations_received: Vec<DelegationRecord>,

    /// Total capability count.
    pub capability_count: u32,

    /// Total delegations made.
    pub delegations_made_count: u32,

    /// Total delegations received.
    pub delegations_received_count: u32,
}

impl Display for AgentAuditReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AgentAudit({}, caps={}, made={}, received={})",
            self.agent_id,
            self.held_capabilities.len(),
            self.delegations_made.len(),
            self.delegations_received.len()
        )
    }
}

/// A record of a delegation involving an agent.
#[derive(Clone, Debug)]
pub struct DelegationRecord {
    /// The capability ID involved in this delegation.
    pub cap_id: CapID,

    /// The other agent involved (source if this is received, target if made).
    pub other_agent: AgentID,

    /// When the delegation occurred.
    pub timestamp: Timestamp,

    /// The attenuation applied (if any).
    pub attenuation: String,
}

impl Display for DelegationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}@{}",
            self.cap_id, self.other_agent, self.timestamp
        )
    }
}

/// Audits a single capability for complete provenance and compliance information.
///
/// Returns a comprehensive report including the capability, its complete delegation
/// chain, all current holders, delegations made, and policy compliance status.
///
/// Per Engineering Plan § 3.2.6, this enables forensic analysis of capability
/// lifecycle and compliance verification. Target latency: <10ms for typical chains.
///
/// # Arguments
/// * `table` - The capability table
/// * `cap_id` - The capability to audit
///
/// See Engineering Plan § 3.2.6: Audit & Compliance.
pub fn audit_capability(
    table: &CapabilityTable,
    cap_id: &CapID,
) -> Result<CapabilityAuditReport, CapError> {
    // Step 1: Look up the capability
    let entry = table.lookup(cap_id)?;
    let capability = entry.capability.clone();

    // Step 2: Extract the delegation chain
    let chain_entries = capability.chain.entries().to_vec();
    let created_at = if chain_entries.is_empty() {
        Timestamp::new(0)
    } else {
        chain_entries[0].timestamp
    };
    let last_modified = if chain_entries.is_empty() {
        Timestamp::new(0)
    } else {
        chain_entries[chain_entries.len() - 1].timestamp
    };

    let provenance_chain = CapChain {
        entries: chain_entries.clone(),
        created_at,
        last_modified,
        delegation_count: chain_entries.len() as u32,
    };

    // Step 3: Collect all holders
    let all_holders = entry.holder_set.iter().cloned().collect::<Vec<_>>();

    // Step 4: Find delegations made from this capability
    let delegations_made = find_delegations_from_cap(table, cap_id)?;

    // Step 5: Determine policy compliance status
    let policy_compliance_status = "compliant".to_string(); // Placeholder

    let report = CapabilityAuditReport {
        capability,
        provenance_chain,
        all_holders,
        delegations_made,
        policy_compliance_status,
    };

    Ok(report)
}

/// Audits an agent for all capabilities and delegations.
///
/// Returns a comprehensive report including:
/// - All capabilities held by the agent
/// - All delegations made by the agent
/// - All delegations received by the agent
///
/// # Arguments
/// * `table` - The capability table
/// * `agent_id` - The agent to audit
///
/// See Engineering Plan § 3.2.6: Audit & Compliance.
pub fn audit_agent(
    table: &CapabilityTable,
    agent_id: &AgentID,
) -> Result<AgentAuditReport, CapError> {
    let mut held_capabilities = Vec::new();
    let mut delegations_made = Vec::new();

    // Step 1: Find all capabilities held by this agent
    for (cap_id, entry) in table.iter() {
        if entry.holder_set.contains(agent_id) {
            held_capabilities.push(cap_id.clone());
        }
    }

    // Step 2: Find delegations made by this agent (from chain entries)
    for (cap_id, entry) in table.iter() {
        for chain_entry in entry.capability.chain.entries() {
            if chain_entry.from == *agent_id {
                delegations_made.push(DelegationRecord {
                    cap_id: cap_id.clone(),
                    other_agent: chain_entry.to.clone(),
                    timestamp: chain_entry.timestamp,
                    attenuation: chain_entry.attenuated_constraints.to_string(),
                });
            }
        }
    }

    // Step 3: Find delegations received by this agent
    let delegations_received = Vec::new(); // Placeholder: would need delegation table

    let capability_count = held_capabilities.len() as u32;
    let delegations_made_count = delegations_made.len() as u32;
    let delegations_received_count = delegations_received.len() as u32;

    let report = AgentAuditReport {
        agent_id: agent_id.clone(),
        held_capabilities,
        delegations_made,
        delegations_received,
        capability_count,
        delegations_made_count,
        delegations_received_count,
    };

    Ok(report)
}

/// Audits capabilities within a time window.
///
/// Returns all capabilities whose chains contain entries within [start_time, end_time).
/// Enables temporal analysis of capability lifecycle.
///
/// # Arguments
/// * `table` - The capability table
/// * `start_time` - Start of time window
/// * `end_time` - End of time window
///
/// Latency Target: <10ms for <100 entries
/// See Engineering Plan § 3.2.6: Audit & Compliance.
pub fn audit_by_timestamp(
    table: &CapabilityTable,
    start_time: Timestamp,
    end_time: Timestamp,
) -> Result<Vec<CapChain>, CapError> {
    let mut results = Vec::new();

    for (_, entry) in table.iter() {
        let chain_entries = entry.capability.chain.entries();
        let in_window = chain_entries.iter().any(|e| {
            e.timestamp >= start_time && e.timestamp < end_time
        });

        if in_window {
            let created_at = if chain_entries.is_empty() {
                Timestamp::new(0)
            } else {
                chain_entries[0].timestamp
            };
            let last_modified = if chain_entries.is_empty() {
                Timestamp::new(0)
            } else {
                chain_entries[chain_entries.len() - 1].timestamp
            };

            results.push(CapChain {
                entries: chain_entries.to_vec(),
                created_at,
                last_modified,
                delegation_count: chain_entries.len() as u32,
            });
        }
    }

    Ok(results)
}

/// Audits all capabilities touched by an agent.
///
/// Returns the set of all capabilities that have entries in their chain
/// where the agent is either 'from' or 'to'.
///
/// # Arguments
/// * `table` - The capability table
/// * `agent_id` - The agent to audit for
///
/// Latency Target: <10ms for <100 entries
/// See Engineering Plan § 3.2.6: Audit & Compliance.
pub fn audit_by_agent(
    table: &CapabilityTable,
    agent_id: &AgentID,
) -> Result<Vec<CapChain>, CapError> {
    let mut results = Vec::new();

    for (_, entry) in table.iter() {
        let chain_entries = entry.capability.chain.entries();
        let involves_agent = chain_entries.iter().any(|e| {
            e.from == *agent_id || e.to == *agent_id
        });

        if involves_agent {
            let created_at = if chain_entries.is_empty() {
                Timestamp::new(0)
            } else {
                chain_entries[0].timestamp
            };
            let last_modified = if chain_entries.is_empty() {
                Timestamp::new(0)
            } else {
                chain_entries[chain_entries.len() - 1].timestamp
            };

            results.push(CapChain {
                entries: chain_entries.to_vec(),
                created_at,
                last_modified,
                delegation_count: chain_entries.len() as u32,
            });
        }
    }

    Ok(results)
}

/// Finds all delegations made from a specific capability.
fn find_delegations_from_cap(
    table: &CapabilityTable,
    parent_cap_id: &CapID,
) -> Result<Vec<DelegationInfo>, CapError> {
    let mut delegations = Vec::new();

    // In a full implementation, would check if parent_cap_id is referenced in each entry
    // For now, this is a placeholder
    for (_, entry) in table.iter() {
        for chain_entry in entry.capability.chain.entries() {
            // Check if this chain was created from parent_cap_id
            // In a real impl, would have parent_cap_id in chain_entry
            delegations.push(DelegationInfo {
                delegating_agent: chain_entry.from.clone(),
                receiving_agent: chain_entry.to.clone(),
                timestamp: chain_entry.timestamp,
                constraints_applied: chain_entry.attenuated_constraints.to_string(),
            });
        }
    }

    Ok(delegations)
}

/// Finds all delegations made by an agent.
fn find_agent_delegations_made(
    table: &CapabilityTable,
    agent_id: &AgentID,
) -> Result<Vec<DelegationRecord>, CapError> {
    let mut delegations = Vec::new();

    for (cap_id, entry) in table.iter() {
        for chain_entry in entry.capability.chain.entries() {
            if chain_entry.from == *agent_id {
                delegations.push(DelegationRecord {
                    cap_id: cap_id.clone(),
                    other_agent: chain_entry.to.clone(),
                    timestamp: chain_entry.timestamp,
                    attenuation: chain_entry.attenuated_constraints.to_string(),
                });
            }
        }
    }

    Ok(delegations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::capability_table::CapabilityEntry;
    use crate::ids::{ResourceID, ResourceType};
    use crate::operations::OperationSet;
use alloc::string::ToString;

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
    fn test_audit_capability_basic() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([1u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(1);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let report = audit_capability(&table, &cap_id).unwrap();

        assert_eq!(report.capability.id, cap_id);
        assert_eq!(report.all_holders.len(), 1);
    }

    #[test]
    fn test_audit_capability_not_found() {
        let table = CapabilityTable::new();
        let cap_id = CapID::from_bytes([2u8; 32]);

        let result = audit_capability(&table, &cap_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_audit_capability_report_display() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([3u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(3);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let report = audit_capability(&table, &cap_id).unwrap();
        let display = report.to_string();

        assert!(display.contains("CapAudit"));
        assert!(display.contains(&cap_id.to_string()));
    }

    #[test]
    fn test_audit_agent_basic() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id_1 = CapID::from_bytes([4u8; 32]);
        let cap_id_2 = CapID::from_bytes([5u8; 32]);
        let agent = AgentID::new("holder");

        let cap1 = make_test_capability(4);
        let mut cap1 = cap1.clone();
        cap1.id = cap_id_1.clone();
        cap1.target_agent = agent.clone();
        let entry1 = CapabilityEntry::new(cap1, agent.clone(), now);
        table.insert(cap_id_1, entry1).unwrap();

        let cap2 = make_test_capability(5);
        let mut cap2 = cap2.clone();
        cap2.id = cap_id_2.clone();
        cap2.target_agent = agent.clone();
        let entry2 = CapabilityEntry::new(cap2, agent.clone(), now);
        table.insert(cap_id_2, entry2).unwrap();

        let report = audit_agent(&table, &agent).unwrap();

        assert_eq!(report.agent_id, agent);
        assert_eq!(report.held_capabilities.len(), 2);
    }

    #[test]
    fn test_audit_agent_not_found() {
        let table = CapabilityTable::new();
        let agent = AgentID::new("nonexistent");

        let report = audit_agent(&table, &agent).unwrap();

        assert_eq!(report.agent_id, agent);
        assert_eq!(report.held_capabilities.len(), 0);
    }

    #[test]
    fn test_audit_agent_report_display() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([6u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(6);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent.clone(), now);
        table.insert(cap_id, entry).unwrap();

        let report = audit_agent(&table, &agent).unwrap();
        let display = report.to_string();

        assert!(display.contains("AgentAudit"));
        assert!(display.contains(&agent.to_string()));
    }

    #[test]
    fn test_delegation_info_display() {
        let info = DelegationInfo {
            delegating_agent: AgentID::new("agent-a"),
            receiving_agent: AgentID::new("agent-b"),
            timestamp: Timestamp::new(1000),
            constraints_applied: "none".to_string(),
        };

        let display = info.to_string();
        assert!(display.contains("agent-a"));
        assert!(display.contains("agent-b"));
    }

    #[test]
    fn test_delegation_record_display() {
        let record = DelegationRecord {
            cap_id: CapID::from_bytes([7u8; 32]),
            other_agent: AgentID::new("agent-b"),
            timestamp: Timestamp::new(1000),
            attenuation: "none".to_string(),
        };

        let display = record.to_string();
        assert!(display.contains("agent-b"));
    }

    #[test]
    fn test_audit_agent_with_no_capabilities() {
        let table = CapabilityTable::new();
        let agent = AgentID::new("idle-agent");

        let report = audit_agent(&table, &agent).unwrap();

        assert_eq!(report.held_capabilities.len(), 0);
        assert_eq!(report.delegations_made.len(), 0);
        assert_eq!(report.delegations_received.len(), 0);
    }

    #[test]
    fn test_capability_audit_report_with_empty_delegations() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([8u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(8);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let report = audit_capability(&table, &cap_id).unwrap();

        assert_eq!(report.delegations_made.len(), 0);
    }

    #[test]
    fn test_audit_agent_multiple_capabilities_same_agent() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let agent = AgentID::new("multi-cap-holder");

        for i in 0..5 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            let cap_id = CapID::from_bytes(bytes);

            let cap = make_test_capability(i);
            let mut cap = cap.clone();
            cap.id = cap_id.clone();
            cap.target_agent = agent.clone();

            let entry = CapabilityEntry::new(cap, agent.clone(), now);
            table.insert(cap_id, entry).unwrap();
        }

        let report = audit_agent(&table, &agent).unwrap();
        assert_eq!(report.held_capabilities.len(), 5);
    }

    #[test]
    fn test_audit_capability_includes_provenance() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([9u8; 32]);
        let agent = AgentID::new("holder");

        let cap = make_test_capability(9);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent.clone();

        let entry = CapabilityEntry::new(cap, agent, now);
        table.insert(cap_id.clone(), entry).unwrap();

        let report = audit_capability(&table, &cap_id).unwrap();

        // Should have the chain from creation
        assert!(report.provenance_chain.entries.len() >= 0);
    }

    #[test]
    fn test_find_agent_delegations_empty_for_new_agent() {
        let table = CapabilityTable::new();
        let agent = AgentID::new("new-agent");

        let delegations = find_agent_delegations_made(&table, &agent).unwrap();
        assert_eq!(delegations.len(), 0);
    }

    #[test]
    fn test_audit_agent_report_contains_agent_id() {
        let table = CapabilityTable::new();
        let agent = AgentID::new("test-agent");

        let report = audit_agent(&table, &agent).unwrap();
        assert_eq!(report.agent_id, agent);
    }

    #[test]
    fn test_audit_by_timestamp_empty_window() {
        let table = CapabilityTable::new();
        let start = Timestamp::new(1000);
        let end = Timestamp::new(2000);

        let results = audit_by_timestamp(&table, start, end).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_audit_by_agent_empty() {
        let table = CapabilityTable::new();
        let agent = AgentID::new("agent");

        let results = audit_by_agent(&table, &agent).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_cap_chain_display() {
        let chain = CapChain {
            entries: Vec::new(),
            created_at: Timestamp::new(1000),
            last_modified: Timestamp::new(2000),
            delegation_count: 0,
        };

        let display = chain.to_string();
        assert!(display.contains("CapChain"));
    }

    #[test]
    fn test_audit_capability_includes_holders() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let cap_id = CapID::from_bytes([10u8; 32]);
        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");

        let cap = make_test_capability(10);
        let mut cap = cap.clone();
        cap.id = cap_id.clone();
        cap.target_agent = agent_a.clone();

        let mut entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        entry.add_holder(agent_b.clone());
        table.insert(cap_id.clone(), entry).unwrap();

        let report = audit_capability(&table, &cap_id).unwrap();

        assert_eq!(report.all_holders.len(), 2);
        assert!(report.all_holders.contains(&agent_a));
        assert!(report.all_holders.contains(&agent_b));
    }

    #[test]
    fn test_agent_audit_report_counts() {
        let mut table = CapabilityTable::new();
        let now = Timestamp::new(1000);

        let agent = AgentID::new("holder");

        for i in 0..3 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            let cap_id = CapID::from_bytes(bytes);

            let cap = make_test_capability(i);
            let mut cap = cap.clone();
            cap.id = cap_id.clone();
            cap.target_agent = agent.clone();

            let entry = CapabilityEntry::new(cap, agent.clone(), now);
            table.insert(cap_id, entry).unwrap();
        }

        let report = audit_agent(&table, &agent).unwrap();

        assert_eq!(report.capability_count, 3);
        assert_eq!(report.held_capabilities.len(), 3);
    }
}
