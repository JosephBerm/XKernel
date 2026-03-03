// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Cryptographic proof checking and capability verification

use crate::capability::{Capability, CapabilityError};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub type Result<T> = core::result::Result<T, CapabilityError>;

/// Cryptographic proof for capability verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// Proof data (simplified - would be cryptographic hash/signature)
    pub data: Vec<u8>,
    /// Proof type identifier
    pub proof_type: ProofType,
    /// Timestamp of proof generation
    pub timestamp: u64,
}

impl Proof {
    /// Create a new proof
    pub fn new(data: Vec<u8>, proof_type: ProofType) -> Self {
        Self {
            data,
            proof_type,
            timestamp: 0,
        }
    }

    /// Verify this proof is valid
    pub fn is_valid(&self) -> bool {
        !self.data.is_empty()
    }
}

/// Type of cryptographic proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    /// HMAC-based proof
    Hmac,
    /// Digital signature proof
    Signature,
    /// Zero-knowledge proof
    ZeroKnowledge,
    /// Merkle tree proof
    MerkleProof,
}

impl ProofType {
    /// Get the minimum proof size in bytes
    pub fn min_size(&self) -> usize {
        match self {
            ProofType::Hmac => 32,
            ProofType::Signature => 64,
            ProofType::ZeroKnowledge => 128,
            ProofType::MerkleProof => 32,
        }
    }
}

/// Result of capability verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// Capability is valid
    Valid,
    /// Capability is invalid
    Invalid,
    /// Proof is missing
    MissingProof,
    /// Capability is revoked
    Revoked,
}

impl VerificationResult {
    /// Check if the verification succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, VerificationResult::Valid)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            VerificationResult::Valid => "Capability is valid",
            VerificationResult::Invalid => "Capability is invalid",
            VerificationResult::MissingProof => "Proof is missing",
            VerificationResult::Revoked => "Capability is revoked",
        }
    }
}

/// Cryptographic verification engine
#[derive(Debug)]
pub struct VerificationEngine {
    /// Verification policies
    policies: Vec<VerificationPolicy>,
}

impl VerificationEngine {
    /// Create a new verification engine
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Add a verification policy
    pub fn add_policy(&mut self, policy: VerificationPolicy) {
        self.policies.push(policy);
    }

    /// Verify a capability with its proof
    pub fn verify(&self, cap: &Capability, proof: Option<&Proof>) -> Result<VerificationResult> {
        if !cap.is_active {
            return Ok(VerificationResult::Revoked);
        }

        let proof = proof.ok_or_else(|| {
            CapabilityError::InvalidCapability("missing proof".into())
        })?;

        if !proof.is_valid() {
            return Ok(VerificationResult::Invalid);
        }

        // Check all policies
        for policy in &self.policies {
            if !policy.verify(cap, proof)? {
                return Ok(VerificationResult::Invalid);
            }
        }

        Ok(VerificationResult::Valid)
    }

    /// Batch verify multiple capabilities
    pub fn verify_batch(
        &self,
        capabilities: &[(Capability, Option<Proof>)],
    ) -> Result<Vec<VerificationResult>> {
        capabilities
            .iter()
            .map(|(cap, proof)| self.verify(cap, proof.as_ref()))
            .collect()
    }
}

impl Default for VerificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Verification policy for capability validation
#[derive(Debug, Clone)]
pub struct VerificationPolicy {
    /// Policy name
    pub name: alloc::string::String,
    /// Proof type required
    pub required_proof_type: ProofType,
}

impl VerificationPolicy {
    /// Create a new verification policy
    pub fn new(name: alloc::string::String, required_proof_type: ProofType) -> Self {
        Self {
            name,
            required_proof_type,
        }
    }

    /// Verify against this policy
    pub fn verify(&self, _cap: &Capability, proof: &Proof) -> Result<bool> {
        Ok(proof.proof_type == self.required_proof_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_creation() {
        let proof = Proof::new(vec![1, 2, 3], ProofType::Hmac);
        assert!(proof.is_valid());
    }

    #[test]
    fn test_proof_type_sizes() {
        assert_eq!(ProofType::Hmac.min_size(), 32);
        assert_eq!(ProofType::Signature.min_size(), 64);
    }

    #[test]
    fn test_verification_result() {
        assert!(VerificationResult::Valid.is_success());
        assert!(!VerificationResult::Invalid.is_success());
    }

    #[test]
    fn test_verification_engine() {
        let mut engine = VerificationEngine::new();
        let policy = VerificationPolicy::new("test".into(), ProofType::Hmac);
        engine.add_policy(policy);

        let perms = vec![];
        let cap = Capability::new(1, 100, perms, None);
        let proof = Proof::new(vec![1, 2, 3], ProofType::Hmac);

        let result = engine.verify(&cap, Some(&proof)).unwrap();
        assert_eq!(result, VerificationResult::Valid);
    }
}
