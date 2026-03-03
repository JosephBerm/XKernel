// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Security (Capability) Family Syscalls
//!
//! Security family syscalls manage capability-based access control:
//! - **cap_grant**: Grant capability to agent
//! - **cap_delegate**: Delegate capability with attenuation
//! - **cap_revoke**: Revoke capability and all descendants
//!
//! # Engineering Plan Reference
//! Section 10: Security Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{
    AgentID, AttenuationSpec, CapabilityID, CapConstraints, CapabilitySet, CapabilitySpec,
};

/// Security family syscall numbers.
pub mod number {
    /// cap_grant syscall number within Security family.
    pub const CAP_GRANT: u8 = 0;
    /// cap_delegate syscall number within Security family.
    pub const CAP_DELEGATE: u8 = 1;
    /// cap_revoke syscall number within Security family.
    pub const CAP_REVOKE: u8 = 2;
}

/// Get the definition of the cap_grant syscall.
///
/// **cap_grant**: Grant capability to agent.
///
/// Grants a capability to a target agent with specified constraints.
/// The capability specifies what resource access is granted and under what
/// conditions (expiration, use limits, delegability, etc.).
///
/// # Parameters
/// - `target_agent`: (AgentID) Agent receiving the capability
/// - `capability`: (Config) Capability specification (name, resource, access level)
/// - `constraints`: (Config) Constraints on the capability (expiration, max uses, etc.)
///
/// # Returns
/// - Success: CapabilityID of the granted capability
/// - Error: CS_EINVAL (invalid spec or constraints), CS_EPERM (no grant capability),
///          CS_EPOLICY (denied by policy)
///
/// # Preconditions
/// - Caller must have capability to grant capabilities
/// - `target_agent` must exist and be valid
/// - `capability` spec must be valid and complete
/// - `constraints` must be consistent (e.g., max_uses < max_uses_parent)
/// - Caller must have the capability being granted
///
/// # Postconditions
/// - Capability is created with immutable CapabilityID
/// - Capability is linked to target agent
/// - Capability has specified constraints
/// - Capability is recorded in capability chain/tree
///
/// # Engineering Plan Reference
/// Section 10.1: cap_grant specification.
pub fn cap_grant_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "cap_grant",
        SyscallFamily::Capability,
        number::CAP_GRANT,
        ReturnType::Identifier,
        CapabilitySet::CAP_CAPABILITY_FAMILY,
        "Grant capability to agent",
    )
    .with_param(SyscallParam::new(
        "target_agent",
        ParamType::Identifier,
        "Agent receiving the capability",
        false,
    ))
    .with_param(SyscallParam::new(
        "capability",
        ParamType::Config,
        "Capability specification",
        false,
    ))
    .with_param(SyscallParam::new(
        "constraints",
        ParamType::Config,
        "Constraints on the capability",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEpolicy)
    .with_preconditions(
        "Caller has Capability capability; target_agent valid; capability spec valid; constraints consistent",
    )
    .with_postconditions(
        "Capability created with immutable CapabilityID; linked to target agent; constraints recorded",
    )
}

/// Get the definition of the cap_delegate syscall.
///
/// **cap_delegate**: Delegate capability with attenuation.
///
/// Delegates an owned capability to another agent with reduced scope (attenuation).
/// Attenuation can reduce access level, expiration time, usage limit, and
/// further delegability. The result is a new capability that is a child
/// of the delegated capability in the capability tree.
///
/// # Parameters
/// - `cap_id`: (Identifier) Capability ID to delegate
/// - `target_agent`: (AgentID) Agent receiving the delegated capability
/// - `attenuation`: (Config) Attenuation specification (new access level, etc.)
///
/// # Returns
/// - Success: CapabilityID of the new delegated capability
/// - Error: CS_EINVAL (invalid capability or attenuation), CS_EPERM (not owner),
///          CS_ENOATTN (invalid attenuation)
///
/// # Preconditions
/// - `cap_id` must reference an existing, valid capability
/// - Caller must own or have capability to delegate this capability
/// - `target_agent` must exist and be valid
/// - `attenuation` must be valid and not increase scope vs. parent capability
/// - Parent capability must have `delegatable` flag set
///
/// # Postconditions
/// - New capability created with attenuated scope
/// - New capability is child of delegated capability in tree
/// - Original capability unchanged
/// - Caller retains original capability
///
/// # Engineering Plan Reference
/// Section 10.2: cap_delegate specification.
pub fn cap_delegate_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "cap_delegate",
        SyscallFamily::Capability,
        number::CAP_DELEGATE,
        ReturnType::Identifier,
        CapabilitySet::CAP_CAPABILITY_FAMILY,
        "Delegate capability with attenuation",
    )
    .with_param(SyscallParam::new(
        "cap_id",
        ParamType::Identifier,
        "Capability ID to delegate",
        false,
    ))
    .with_param(SyscallParam::new(
        "target_agent",
        ParamType::Identifier,
        "Agent receiving the delegated capability",
        false,
    ))
    .with_param(SyscallParam::new(
        "attenuation",
        ParamType::Config,
        "Attenuation specification",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoattn)
    .with_preconditions(
        "Capability valid and owned; target_agent valid; attenuation valid; not increasing scope; parent delegatable",
    )
    .with_postconditions(
        "New capability created as child; attenuated scope; original unchanged; caller retains original",
    )
}

/// Get the definition of the cap_revoke syscall.
///
/// **cap_revoke**: Revoke capability and all descendants.
///
/// Revokes a capability and all capabilities that were derived from it
/// (descendants in the capability tree). This provides a way to efficiently
/// revoke an entire capability subtree. The revocation is cascading and atomic.
///
/// # Parameters
/// - `cap_id`: (Identifier) Capability ID to revoke
///
/// # Returns
/// - Success: Numeric count of revoked capabilities (including descendants)
/// - Error: CS_EINVAL (invalid capability ID), CS_EPERM (not owner or revoker)
///
/// # Preconditions
/// - `cap_id` must reference an existing, valid capability
/// - Caller must own the capability or have revoke permission
/// - Capability's `revocable` flag must be set
///
/// # Postconditions
/// - Capability is revoked (invalidated)
/// - All descendant capabilities in tree are revoked
/// - Agents holding revoked capabilities can no longer use them
/// - Revocation is logged for audit trail
/// - Returns count of revoked capabilities
///
/// # Engineering Plan Reference
/// Section 10.3: cap_revoke specification.
pub fn cap_revoke_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "cap_revoke",
        SyscallFamily::Capability,
        number::CAP_REVOKE,
        ReturnType::Numeric,
        CapabilitySet::CAP_CAPABILITY_FAMILY,
        "Revoke capability and all descendants",
    )
    .with_param(SyscallParam::new(
        "cap_id",
        ParamType::Identifier,
        "Capability ID to revoke",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEperm)
    .with_preconditions(
        "Capability valid; caller is owner or has revoke permission; revocable flag set",
    )
    .with_postconditions(
        "Capability revoked; descendants revoked; agents cannot use revoked caps; count returned",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_grant_definition() {
        let def = cap_grant_definition();
        assert_eq!(def.name, "cap_grant");
        assert_eq!(def.family, SyscallFamily::Capability);
        assert_eq!(def.number, number::CAP_GRANT);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_cap_grant_parameters() {
        let def = cap_grant_definition();
        assert_eq!(def.parameters[0].name, "target_agent");
        assert_eq!(def.parameters[1].name, "capability");
        assert_eq!(def.parameters[2].name, "constraints");
    }

    #[test]
    fn test_cap_grant_errors() {
        let def = cap_grant_definition();
        assert!(def.error_codes.len() >= 4);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEinval));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEperm));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEpolicy));
    }

    #[test]
    fn test_cap_delegate_definition() {
        let def = cap_delegate_definition();
        assert_eq!(def.name, "cap_delegate");
        assert_eq!(def.family, SyscallFamily::Capability);
        assert_eq!(def.number, number::CAP_DELEGATE);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_cap_delegate_parameters() {
        let def = cap_delegate_definition();
        assert_eq!(def.parameters[0].name, "cap_id");
        assert_eq!(def.parameters[1].name, "target_agent");
        assert_eq!(def.parameters[2].name, "attenuation");
    }

    #[test]
    fn test_cap_delegate_errors() {
        let def = cap_delegate_definition();
        assert!(def.error_codes.len() >= 4);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEnoattn));
    }

    #[test]
    fn test_cap_revoke_definition() {
        let def = cap_revoke_definition();
        assert_eq!(def.name, "cap_revoke");
        assert_eq!(def.family, SyscallFamily::Capability);
        assert_eq!(def.number, number::CAP_REVOKE);
        assert_eq!(def.return_type, ReturnType::Numeric);
        assert_eq!(def.parameters.len(), 1);
    }

    #[test]
    fn test_cap_revoke_parameters() {
        let def = cap_revoke_definition();
        assert_eq!(def.parameters[0].name, "cap_id");
    }

    #[test]
    fn test_cap_revoke_errors() {
        let def = cap_revoke_definition();
        assert!(def.error_codes.len() >= 3);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEinval));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEperm));
    }

    #[test]
    fn test_security_family_syscall_numbers_unique() {
        assert_ne!(number::CAP_GRANT, number::CAP_DELEGATE);
        assert_ne!(number::CAP_DELEGATE, number::CAP_REVOKE);
        assert_ne!(number::CAP_GRANT, number::CAP_REVOKE);
    }

    #[test]
    fn test_all_definitions_have_preconditions() {
        assert!(!cap_grant_definition().preconditions.is_empty());
        assert!(!cap_delegate_definition().preconditions.is_empty());
        assert!(!cap_revoke_definition().preconditions.is_empty());
    }

    #[test]
    fn test_all_definitions_have_postconditions() {
        assert!(!cap_grant_definition().postconditions.is_empty());
        assert!(!cap_delegate_definition().postconditions.is_empty());
        assert!(!cap_revoke_definition().postconditions.is_empty());
    }
}
