// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Attenuation policies for capability delegation.
//!
//! This module defines how capabilities can be restricted (attenuated) during delegation.
//! See Engineering Plan § 3.1.7: Delegation & Attenuation.

use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::constraints::{CapConstraints, ChainDepthLimit, DataVolumeLimit, RateLimit, TimeBound};
use crate::error::CapError;
use crate::operations::OperationSet;

/// An attenuation policy describes how a capability can be restricted.
///
/// See Engineering Plan § 3.1.7: Delegation & Attenuation.
/// Attenuations are composable: a capability can have multiple restrictions applied.
#[derive(Clone, Debug)]
pub enum AttenuationPolicy {
    /// Restrict the capability to a subset of operations.
    ReduceOps(OperationSet),

    /// Add a time bound to the capability.
    TimeBound(TimeBound),

    /// Add a rate limit to the capability.
    RateLimit(RateLimit),

    /// Add a data volume limit to the capability.
    DataLimit(DataVolumeLimit),

    /// Add a delegation depth limit to the capability.
    DepthLimit(ChainDepthLimit),

    /// Compose multiple attenuation policies to be applied in sequence.
    Compose(Vec<AttenuationPolicy>),
}

impl AttenuationPolicy {
    /// Applies this attenuation policy to a capability, returning an attenuated copy.
    ///
    /// Per Engineering Plan § 3.1.7, attenuation is monotonic:
    /// you can only restrict, never expand, a capability's permissions.
    pub fn apply(&self, cap: &Capability) -> Result<Capability, CapError> {
        match self {
            AttenuationPolicy::ReduceOps(new_ops) => {
                // Restriction: the new operations must be a subset of the current operations
                if !new_ops.is_subset_of(cap.operations) {
                    return Err(CapError::InvalidAttenuation(
                        "cannot expand operations during attenuation".to_string(),
                    ));
                }
                let mut attenuated = cap.clone();
                attenuated.operations = *new_ops;
                Ok(attenuated)
            }

            AttenuationPolicy::TimeBound(new_bound) => {
                // Check that we're not expanding the time window
                if let Some(existing) = cap.constraints.time_bound {
                    // New start must be >= existing start
                    // New expiry must be <= existing expiry
                    if new_bound.start_timestamp < existing.start_timestamp
                        || new_bound.expiry_timestamp > existing.expiry_timestamp
                    {
                        return Err(CapError::InvalidAttenuation(
                            "cannot expand time window during attenuation".to_string(),
                        ));
                    }
                }

                let mut attenuated = cap.clone();
                attenuated.constraints.time_bound = Some(*new_bound);
                Ok(attenuated)
            }

            AttenuationPolicy::RateLimit(new_limit) => {
                // Check that we're not increasing the rate limit
                if let Some(existing) = cap.constraints.rate_limited {
                    if new_limit.max_operations_per_period > existing.max_operations_per_period {
                        return Err(CapError::InvalidAttenuation(
                            "cannot increase rate limit during attenuation".to_string(),
                        ));
                    }
                }

                let mut attenuated = cap.clone();
                attenuated.constraints.rate_limited = Some(*new_limit);
                Ok(attenuated)
            }

            AttenuationPolicy::DataLimit(new_limit) => {
                // Check that we're not increasing the data volume limit
                if let Some(existing) = cap.constraints.data_volume_limited {
                    if new_limit.max_bytes_per_period > existing.max_bytes_per_period {
                        return Err(CapError::InvalidAttenuation(
                            "cannot increase data volume limit during attenuation".to_string(),
                        ));
                    }
                }

                let mut attenuated = cap.clone();
                attenuated.constraints.data_volume_limited = Some(*new_limit);
                Ok(attenuated)
            }

            AttenuationPolicy::DepthLimit(new_limit) => {
                // Check that we're not increasing the depth limit
                if let Some(existing) = cap.constraints.chain_depth_limited {
                    if new_limit.max_delegation_depth > existing.max_delegation_depth {
                        return Err(CapError::InvalidAttenuation(
                            "cannot increase delegation depth limit during attenuation".to_string(),
                        ));
                    }
                }

                let mut attenuated = cap.clone();
                attenuated.constraints.chain_depth_limited = Some(*new_limit);
                Ok(attenuated)
            }

            AttenuationPolicy::Compose(policies) => {
                // Apply each policy in sequence
                let mut result = cap.clone();
                for policy in policies {
                    result = policy.apply(&result)?;
                }
                Ok(result)
            }
        }
    }

    /// Composes this policy with another, returning a policy that applies both.
    pub fn compose(self, other: AttenuationPolicy) -> AttenuationPolicy {
        match self {
            AttenuationPolicy::Compose(mut policies) => {
                policies.push(other);
                AttenuationPolicy::Compose(policies)
            }
            _ => AttenuationPolicy::Compose(vec![self, other]),
        }
    }
}

impl Display for AttenuationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttenuationPolicy::ReduceOps(ops) => write!(f, "ReduceOps({})", ops),
            AttenuationPolicy::TimeBound(tb) => write!(f, "TimeBound({})", tb),
            AttenuationPolicy::RateLimit(rl) => write!(f, "RateLimit({})", rl),
            AttenuationPolicy::DataLimit(dvl) => write!(f, "DataLimit({})", dvl),
            AttenuationPolicy::DepthLimit(cdl) => write!(f, "DepthLimit({})", cdl),
            AttenuationPolicy::Compose(policies) => {
                write!(f, "Compose[")?;
                for (i, p) in policies.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "]")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AgentID, CapID, ResourceID, ResourceType};
use alloc::string::ToString;
use alloc::vec;

    fn create_test_cap() -> Capability {
        Capability {
            id: CapID::from_bytes([1u8; 32]),
            target_agent: AgentID::new("agent-a"),
            target_resource_type: ResourceType::new("file"),
            target_resource_id: ResourceID::new("file-001"),
            operations: OperationSet::all(),
            constraints: CapConstraints::new(),
            chain: crate::chain::CapChain::new(),
            revocable_by: Default::default(),
            created_at: crate::constraints::Timestamp::new(1000),
            expires_at: None,
        }
    }

    #[test]
    fn test_reduce_ops_valid() {
        let cap = create_test_cap();
        let policy = AttenuationPolicy::ReduceOps(OperationSet::read());
        let result = policy.apply(&cap);

        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.operations.contains_read());
        assert!(!attenuated.operations.contains_write());
    }

    #[test]
    fn test_reduce_ops_invalid_expansion() {
        let mut cap = create_test_cap();
        cap.operations = OperationSet::read(); // Start with just read

        // Try to expand to write - should fail
        let policy = AttenuationPolicy::ReduceOps(OperationSet::read().union(OperationSet::write()));
        let result = policy.apply(&cap);

        assert!(result.is_err());
    }

    #[test]
    fn test_time_bound_valid() {
        let cap = create_test_cap();
        let start = crate::constraints::Timestamp::new(1000);
        let expiry = crate::constraints::Timestamp::new(2000);
        let policy = AttenuationPolicy::TimeBound(TimeBound::new(start, expiry));
        let result = policy.apply(&cap);

        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.constraints.time_bound.is_some());
    }

    #[test]
    fn test_time_bound_invalid_expansion() {
        let mut cap = create_test_cap();
        let start = crate::constraints::Timestamp::new(2000);
        let expiry = crate::constraints::Timestamp::new(3000);
        cap.constraints.time_bound = Some(TimeBound::new(start, expiry));

        // Try to expand to earlier start - should fail
        let new_start = crate::constraints::Timestamp::new(1000);
        let policy = AttenuationPolicy::TimeBound(TimeBound::new(new_start, expiry));
        let result = policy.apply(&cap);

        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_valid() {
        let cap = create_test_cap();
        let policy = AttenuationPolicy::RateLimit(RateLimit::new(100, 1_000_000_000));
        let result = policy.apply(&cap);

        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.constraints.rate_limited.is_some());
    }

    #[test]
    fn test_rate_limit_invalid_expansion() {
        let mut cap = create_test_cap();
        cap.constraints.rate_limited = Some(RateLimit::new(100, 1_000_000_000));

        // Try to increase limit - should fail
        let policy = AttenuationPolicy::RateLimit(RateLimit::new(200, 1_000_000_000));
        let result = policy.apply(&cap);

        assert!(result.is_err());
    }

    #[test]
    fn test_data_limit_valid() {
        let cap = create_test_cap();
        let policy = AttenuationPolicy::DataLimit(DataVolumeLimit::new(1_000_000, 1_000_000_000));
        let result = policy.apply(&cap);

        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.constraints.data_volume_limited.is_some());
    }

    #[test]
    fn test_depth_limit_valid() {
        let cap = create_test_cap();
        let policy = AttenuationPolicy::DepthLimit(ChainDepthLimit::new(2));
        let result = policy.apply(&cap);

        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.constraints.chain_depth_limited.is_some());
    }

    #[test]
    fn test_compose_policies() {
        let cap = create_test_cap();
        let policy = AttenuationPolicy::ReduceOps(OperationSet::read())
            .compose(AttenuationPolicy::RateLimit(RateLimit::new(100, 1_000_000_000)));

        let result = policy.apply(&cap);
        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.operations.contains_read());
        assert!(attenuated.constraints.rate_limited.is_some());
    }

    #[test]
    fn test_compose_multiple() {
        let cap = create_test_cap();
        let start = crate::constraints::Timestamp::new(1000);
        let expiry = crate::constraints::Timestamp::new(2000);

        let policy = AttenuationPolicy::Compose(vec![
            AttenuationPolicy::ReduceOps(OperationSet::read()),
            AttenuationPolicy::TimeBound(TimeBound::new(start, expiry)),
            AttenuationPolicy::RateLimit(RateLimit::new(100, 1_000_000_000)),
        ]);

        let result = policy.apply(&cap);
        assert!(result.is_ok());
        let attenuated = result.unwrap();
        assert!(attenuated.operations.contains_read());
        assert!(attenuated.constraints.time_bound.is_some());
        assert!(attenuated.constraints.rate_limited.is_some());
    }

    #[test]
    fn test_attenuation_policy_display() {
        let policy = AttenuationPolicy::ReduceOps(OperationSet::read());
        let display = policy.to_string();
        assert!(display.contains("ReduceOps"));
    }
}
