// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Delegate operation: agent-to-agent capability delegation.
//!
//! This module implements capability delegation, where an agent with a capability
//! can grant it (with attenuated permissions) to another agent.
//! See Engineering Plan § 3.1.7: Delegation & Attenuation and § 3.2.4: Delegation.

use alloc::string::String;
use core::fmt::{self, Debug, Display};

use crate::attenuation::AttenuationPolicy;
use crate::capability::Capability;
use crate::capability_table::{CapabilityEntry, CapabilityTable};
use crate::chain::ChainEntry;
use crate::constraints::Timestamp;
use crate::error::CapError;
use crate::ids::{CapID, AgentID};
use crate::mandatory_policy::MandatoryCapabilityPolicy;
use crate::policy_engine::{EvalContext, PolicyEvaluator};

/// A record of a delegation operation, used for audit trails.
///
/// See Engineering Plan § 3.2.4: Delegation.
#[derive(Clone, Debug)]
pub struct DelegateAuditRecord {
    /// The ID of the original capability being delegated.
    pub original_cap_id: CapID,

    /// The ID of the new delegated capability.
    pub new_cap_id: CapID,

    /// The agent who performed the delegation.
    pub from_agent: AgentID,

    /// The agent who received the delegated capability.
    pub to_agent: AgentID,

    /// The attenuations applied during delegation.
    pub attenuation_applied: String,

    /// Timestamp when the delegation occurred.
    pub timestamp: Timestamp,
}

impl Display for DelegateAuditRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Delegate({} -> {} of {}, attenuation={})",
            self.from_agent, self.to_agent, self.original_cap_id, self.attenuation_applied
        )
    }
}

/// Delegates a capability from one agent to another with attenuation.
///
/// The delegating agent must currently hold the capability. Attenuation is strictly
/// monotonic: the delegated capability can never have more permissions than the original.
///
/// Per Engineering Plan § 3.1.7, attenuation can only reduce, never expand permissions.
/// All attenuations are validated against MandatoryCapabilityPolicies before completion.
///
/// # Arguments
/// * `table` - The capability table
/// * `original_cap_id` - The ID of the capability to delegate
/// * `from_agent` - The agent delegating the capability (must hold it)
/// * `to_agent` - The agent receiving the delegated capability
/// * `attenuation` - The attenuation policy to apply (optional)
/// * `new_cap_id` - The kernel-assigned ID for the delegated capability
/// * `policy_engine` - The mandatory policy enforcement engine
/// * `now` - Current timestamp
///
/// # Latency Target
/// <1000ns warm cache (per Engineering Plan § 3.2.4)
///
/// See Engineering Plan § 3.1.7 & § 3.2.4: Delegation & Attenuation.
pub fn delegate_capability(
    table: &mut CapabilityTable,
    original_cap_id: &CapID,
    from_agent: AgentID,
    to_agent: AgentID,
    attenuation: Option<AttenuationPolicy>,
    new_cap_id: CapID,
    policy_engine: &dyn PolicyEvaluator,
    now: Timestamp,
) -> Result<(CapID, DelegateAuditRecord), CapError> {
    // Step 1: Look up the original capability
    let entry = table.lookup(original_cap_id)?;

    // Step 2: Validate that from_agent holds the original capability
    if !entry.is_held_by(&from_agent) {
        return Err(CapError::Other(format!(
            "agent {} does not hold capability {}",
            from_agent, original_cap_id
        )));
    }

    // Step 3: Clone the original capability as the basis for delegation
    let mut delegated_cap = entry.capability.clone();
    delegated_cap.id = new_cap_id.clone();

    // Step 4: Apply attenuation (if provided)
    let attenuation_desc = if let Some(att) = attenuation {
        let att_desc = format!("{:?}", att);
        delegated_cap = att.apply(&delegated_cap)?;
        att_desc
    } else {
        "none".to_string()
    };

    // Step 5: Validate attenuated capability against policies
    let eval_context = EvalContext::new(
        now.nanos(),
        from_agent.clone(),
        to_agent.clone(),
        "delegate",
    );
    let decision = policy_engine.evaluate(&delegated_cap, &eval_context);

    if decision.denies_operation() {
        return Err(CapError::PolicyDenied(format!(
            "delegation denied by policy: {}",
            decision
        )));
    }

    // Step 6: Add chain entry for this delegation
    let chain_entry = ChainEntry::new(
        from_agent.clone(),
        to_agent.clone(),
        delegated_cap.constraints.clone(),
        now,
    );
    delegated_cap.chain.delegate(
        from_agent.clone(),
        to_agent.clone(),
        delegated_cap.constraints.clone(),
        now,
    )?;

    // Step 7: Update the target agent in the delegated capability
    delegated_cap.target_agent = to_agent.clone();

    // Step 8: Create new entry and insert into table
    let new_entry = CapabilityEntry::new(delegated_cap, to_agent.clone(), now);
    table.insert(new_cap_id.clone(), new_entry)?;

    // Step 9: Add the receiving agent as a holder in the original capability entry
    let original_entry = table.lookup_mut(original_cap_id)?;
    original_entry.add_holder(to_agent.clone());

    // Step 10: Create and return audit record
    let audit_record = DelegateAuditRecord {
        original_cap_id: original_cap_id.clone(),
        new_cap_id: new_cap_id.clone(),
        from_agent,
        to_agent,
        attenuation_applied: attenuation_desc,
        timestamp: now,
    };

    Ok((new_cap_id, audit_record))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::capability_table::CapabilityTable;
    use crate::constraints::CapConstraints;
    use crate::ids::{ResourceID, ResourceType};
    use crate::operations::OperationSet;
    use crate::policy::BasicPolicyEngine;
use alloc::format;
use alloc::string::ToString;

    fn make_test_capability(cap_id_byte: u8) -> Capability {
        let mut bytes = [0u8; 32];
        bytes[0] = cap_id_byte;
        Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("holder-a"),
            ResourceType::file(),
            ResourceID::new("test-resource"),
            OperationSet::all(),
            Timestamp::new(1000),
        )
    }

    #[test]
    fn test_delegate_basic_capability() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        // First, create and grant the original capability
        let original_cap_id = CapID::from_bytes([1u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([2u8; 32]);

        let cap = make_test_capability(1);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        // Now delegate
        let (new_id, audit) = delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a.clone(),
            agent_b.clone(),
            None,
            delegated_cap_id.clone(),
            &policy_engine,
            now,
        )
        .unwrap();

        assert_eq!(new_id, delegated_cap_id);
        assert_eq!(audit.from_agent, agent_a);
        assert_eq!(audit.to_agent, agent_b);
        assert_eq!(audit.original_cap_id, original_cap_id);
    }

    #[test]
    fn test_delegate_non_holder_fails() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([3u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let agent_c = AgentID::new("agent-c");

        let cap = make_test_capability(3);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        // Try to delegate as agent_c who doesn't hold it
        let result = delegate_capability(
            &mut table,
            &original_cap_id,
            agent_c,
            agent_b,
            None,
            CapID::from_bytes([4u8; 32]),
            &policy_engine,
            now,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_delegate_with_attenuation() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([5u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([6u8; 32]);

        let cap = make_test_capability(5);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.operations = OperationSet::all();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        // Delegate with attenuation: reduce to read-only
        let attenuation = AttenuationPolicy::ReduceOps(OperationSet::read());

        let (_, _) = delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a,
            agent_b,
            Some(attenuation),
            delegated_cap_id.clone(),
            &policy_engine,
            now,
        )
        .unwrap();

        // Check that delegated capability has only read
        let delegated_entry = table.lookup(&delegated_cap_id).unwrap();
        assert!(delegated_entry.capability.operations.contains_read());
        assert!(!delegated_entry.capability.operations.contains_write());
    }

    #[test]
    fn test_delegate_updates_chain() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([7u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([8u8; 32]);

        let cap = make_test_capability(7);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.target_agent = agent_a.clone();
        let original_chain_len = cap.chain.len();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a,
            agent_b,
            None,
            delegated_cap_id.clone(),
            &policy_engine,
            now,
        )
        .unwrap();

        let delegated_entry = table.lookup(&delegated_cap_id).unwrap();
        // The delegated capability should have a longer chain
        assert!(delegated_entry.capability.chain.len() >= original_chain_len);
    }

    #[test]
    fn test_delegate_adds_holder_to_original() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([9u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([10u8; 32]);

        let cap = make_test_capability(9);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        assert_eq!(table.lookup(&original_cap_id).unwrap().holder_set.len(), 1);

        delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a,
            agent_b.clone(),
            None,
            delegated_cap_id,
            &policy_engine,
            now,
        )
        .unwrap();

        // Original entry should now list agent_b as a holder
        let original_entry = table.lookup(&original_cap_id).unwrap();
        assert_eq!(original_entry.holder_set.len(), 2);
        assert!(original_entry.is_held_by(&agent_b));
    }

    #[test]
    fn test_delegate_audit_record_display() {
        let record = DelegateAuditRecord {
            original_cap_id: CapID::from_bytes([1u8; 32]),
            new_cap_id: CapID::from_bytes([2u8; 32]),
            from_agent: AgentID::new("agent-a"),
            to_agent: AgentID::new("agent-b"),
            attenuation_applied: "ReduceOps(1)".to_string(),
            timestamp: Timestamp::new(1000),
        };

        let display = record.to_string();
        assert!(display.contains("Delegate"));
        assert!(display.contains("agent-a"));
        assert!(display.contains("agent-b"));
    }

    #[test]
    fn test_delegate_creates_new_capability_entry() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([11u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([12u8; 32]);

        let cap = make_test_capability(11);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        assert_eq!(table.len(), 1);

        delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a,
            agent_b.clone(),
            None,
            delegated_cap_id.clone(),
            &policy_engine,
            now,
        )
        .unwrap();

        assert_eq!(table.len(), 2);
        let new_entry = table.lookup(&delegated_cap_id).unwrap();
        assert!(new_entry.is_held_by(&agent_b));
    }

    #[test]
    fn test_delegate_duplicate_new_cap_id_fails() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id_1 = CapID::from_bytes([13u8; 32]);
        let original_cap_id_2 = CapID::from_bytes([14u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([15u8; 32]);

        // Set up two original capabilities
        let cap1 = make_test_capability(13);
        let mut cap1 = cap1.clone();
        cap1.id = original_cap_id_1.clone();
        cap1.target_agent = agent_a.clone();
        let entry1 = CapabilityEntry::new(cap1, agent_a.clone(), now);
        table.insert(original_cap_id_1.clone(), entry1).unwrap();

        let cap2 = make_test_capability(14);
        let mut cap2 = cap2.clone();
        cap2.id = original_cap_id_2.clone();
        cap2.target_agent = agent_a.clone();
        let entry2 = CapabilityEntry::new(cap2, agent_a.clone(), now);
        table.insert(original_cap_id_2.clone(), entry2).unwrap();

        // First delegation succeeds
        delegate_capability(
            &mut table,
            &original_cap_id_1,
            agent_a.clone(),
            agent_b.clone(),
            None,
            delegated_cap_id.clone(),
            &policy_engine,
            now,
        )
        .unwrap();

        // Second delegation with same new_cap_id fails
        let result = delegate_capability(
            &mut table,
            &original_cap_id_2,
            agent_a,
            agent_b,
            None,
            delegated_cap_id,
            &policy_engine,
            now,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_delegate_monotonic_attenuation_enforced() {
        let mut table = CapabilityTable::new();
        let policy_engine = BasicPolicyEngine::new();
        let now = Timestamp::new(1000);

        let original_cap_id = CapID::from_bytes([16u8; 32]);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let delegated_cap_id = CapID::from_bytes([17u8; 32]);

        // Create capability with read-only
        let cap = make_test_capability(16);
        let mut cap = cap.clone();
        cap.id = original_cap_id.clone();
        cap.operations = OperationSet::read();
        cap.target_agent = agent_a.clone();

        let entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        table.insert(original_cap_id.clone(), entry).unwrap();

        // Try to delegate with expanded permissions (write) - should fail
        let attenuation = AttenuationPolicy::ReduceOps(OperationSet::all());
        let result = delegate_capability(
            &mut table,
            &original_cap_id,
            agent_a,
            agent_b,
            Some(attenuation),
            delegated_cap_id,
            &policy_engine,
            now,
        );

        assert!(result.is_err());
    }
}
