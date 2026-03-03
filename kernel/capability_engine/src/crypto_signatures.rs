// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Ed25519 cryptographic signatures for distributed trust boundaries.
//!
//! Signatures are ONLY used for cross-kernel IPC at network ingress/egress.
//! Local kernel operations use CapID handles (non-cryptographic, <100ns).
//! Signature over: (CapID, delegation_chain, constraints).
//! See Engineering Plan § 3.2.0 and Week 6 § 5.

use alloc::string::String;

use core::fmt::Write;

#![forbid(unsafe_code)]

use alloc::vec::Vec;
use core::fmt::{self, Debug};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::CapID;

/// A 64-byte Ed25519 signature.
/// Authenticates a capability for cross-kernel IPC.
/// See Week 6 § 5: Cryptographic Signatures.
#[derive(Clone, PartialEq, Eq)]
pub struct Ed25519Signature([u8; 64]);

impl Ed25519Signature {
    /// Creates a signature from a 64-byte array.
    pub const fn from_bytes(bytes: [u8; 64]) -> Self {
        Ed25519Signature(bytes)
    }

    /// Returns the underlying 64-byte array.
    pub const fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Exposes the signature as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ed25519Signature")
            .field(&hex_encode(&self.0[..16]))
            .finish()
    }
}

impl fmt::Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519Signature({}...)", hex_encode(&self.0[..16]))
    }
}

/// A 32-byte Ed25519 public key.
/// Identifies the signing entity (kernel, agent, service).
/// See Week 6 § 5.
#[derive(Clone, PartialEq, Eq)]
pub struct Ed25519PublicKey([u8; 32]);

impl Ed25519PublicKey {
    /// Creates a public key from a 32-byte array.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Ed25519PublicKey(bytes)
    }

    /// Returns the underlying 32-byte array.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Exposes the key as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ed25519PublicKey")
            .field(&hex_encode(&self.0[..16]))
            .finish()
    }
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PublicKey({}...)", hex_encode(&self.0[..16]))
    }
}

/// A 32-byte Ed25519 secret key (for signing only).
/// MUST be kept confidential (kernel-internal only).
/// See Week 6 § 5.
#[derive(Clone)]
pub struct Ed25519SecretKey([u8; 32]);

impl Ed25519SecretKey {
    /// Creates a secret key from a 32-byte array.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Ed25519SecretKey(bytes)
    }

    /// Returns the underlying 32-byte array.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Exposes the key as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for Ed25519SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ed25519SecretKey")
            .field(&"[REDACTED]")
            .finish()
    }
}

/// Signed capability for cross-kernel IPC.
/// Contains the capability and its cryptographic signature.
/// See Week 6 § 5: Cryptographic Signatures.
#[derive(Clone, Debug)]
pub struct SignedCapability {
    /// The capability being signed
    pub capability: Capability,

    /// Ed25519 signature over the capability
    pub signature: Ed25519Signature,

    /// Public key of the signer (for verification)
    pub signer_key: Ed25519PublicKey,

    /// Timestamp when signature was created (nanoseconds)
    pub signed_at_ns: u64,
}

impl SignedCapability {
    /// Creates a new signed capability.
    /// Note: In real implementation, signature would be computed.
    /// This is a placeholder for prototyping.
    pub fn new(
        capability: Capability,
        signature: Ed25519Signature,
        signer_key: Ed25519PublicKey,
        signed_at_ns: u64,
    ) -> Self {
        SignedCapability {
            capability,
            signature,
            signer_key,
            signed_at_ns,
        }
    }
}

/// Capability signature message: the data that is signed.
/// Contains all security-relevant fields of a capability.
/// See Week 6 § 5.
#[derive(Clone, Debug)]
pub struct CapabilitySignatureMessage {
    /// Capability ID being signed
    pub cap_id: CapID,

    /// Hash of the delegation chain
    pub chain_hash: [u8; 32],

    /// Serialized constraints (for signature coverage)
    pub constraints_bytes: Vec<u8>,

    /// Timestamp of message creation (nanoseconds)
    pub created_at_ns: u64,

    /// Nonce (for replay protection)
    pub nonce: u64,
}

impl CapabilitySignatureMessage {
    /// Creates a new signature message from a capability.
    /// Note: In real implementation, chain_hash and constraints_bytes would be
    /// computed from the actual capability data.
    pub fn from_capability(cap: &Capability, nonce: u64, created_at_ns: u64) -> Result<Self, CapError> {
        // Placeholder implementation
        let chain_hash = [0u8; 32]; // Real implementation: hash the chain
        let constraints_bytes = Vec::new(); // Real: serialize constraints

        Ok(CapabilitySignatureMessage {
            cap_id: cap.id.clone(),
            chain_hash,
            constraints_bytes,
            created_at_ns,
            nonce,
        })
    }

    /// Serializes the message to bytes for signing.
    /// Order: cap_id(32) | chain_hash(32) | constraints_len(8) | constraints | created_at(8) | nonce(8)
    pub fn to_bytes(&self) -> Result<Vec<u8>, CapError> {
        let mut bytes = Vec::new();

        // Capability ID (32 bytes)
        bytes.extend_from_slice(self.cap_id.as_slice());

        // Chain hash (32 bytes)
        bytes.extend_from_slice(&self.chain_hash);

        // Constraints length (8 bytes, little-endian)
        bytes.extend_from_slice(&(self.constraints_bytes.len() as u64).to_le_bytes());

        // Constraints
        bytes.extend_from_slice(&self.constraints_bytes);

        // Created at (8 bytes)
        bytes.extend_from_slice(&self.created_at_ns.to_le_bytes());

        // Nonce (8 bytes)
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        Ok(bytes)
    }
}

/// Cryptographic signer/verifier for capabilities.
/// Provides placeholder implementations for Ed25519 operations.
/// Real implementation would use a cryptographic library.
/// See Week 6 § 5.
pub struct CryptographicSigner;

impl CryptographicSigner {
    /// Signs a capability message with a secret key.
    /// Returns a signature (placeholder implementation).
    pub fn sign(
        message: &CapabilitySignatureMessage,
        secret_key: &Ed25519SecretKey,
    ) -> Result<Ed25519Signature, CapError> {
        // Real implementation: use ed25519-dalek or similar
        // For now: return placeholder signature
        let msg_bytes = message.to_bytes()?;
        let mut sig_bytes = [0u8; 64];

        // Placeholder: mix message and secret key bytes
        for (i, &byte) in msg_bytes.iter().enumerate() {
            sig_bytes[i % 64] ^= byte ^ secret_key.as_bytes()[i % 32];
        }

        Ok(Ed25519Signature::from_bytes(sig_bytes))
    }

    /// Verifies a signature against a message and public key.
    /// Returns Ok(()) if valid, Err if invalid (placeholder implementation).
    pub fn verify(
        message: &CapabilitySignatureMessage,
        signature: &Ed25519Signature,
        public_key: &Ed25519PublicKey,
    ) -> Result<(), CapError> {
        // Real implementation: use ed25519-dalek or similar
        // For now: accept all signatures as valid (placeholder)
        let _msg_bytes = message.to_bytes()?;
        let _sig = signature.as_slice();
        let _key = public_key.as_slice();

        // Placeholder: always succeeds
        Ok(())
    }
}

/// Helper function to encode bytes as hex string.
fn hex_encode(bytes: &[u8]) -> alloc::string::String {
    let mut s = String::new();
    for &byte in bytes {
        let _ = write!(s, "{:02x}", byte);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Timestamp;
    use crate::ids::{ResourceID, ResourceType, AgentID};
    use crate::operations::OperationSet;
use alloc::string::ToString;

    #[test]
    fn test_ed25519_signature_creation() {
        let sig = Ed25519Signature::from_bytes([42u8; 64]);
        assert_eq!(sig.as_bytes(), &[42u8; 64]);
    }

    #[test]
    fn test_ed25519_public_key_creation() {
        let key = Ed25519PublicKey::from_bytes([99u8; 32]);
        assert_eq!(key.as_bytes(), &[99u8; 32]);
    }

    #[test]
    fn test_ed25519_secret_key_creation() {
        let key = Ed25519SecretKey::from_bytes([77u8; 32]);
        assert_eq!(key.as_bytes(), &[77u8; 32]);
    }

    #[test]
    fn test_signed_capability_creation() {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let sig = Ed25519Signature::from_bytes([42u8; 64]);
        let key = Ed25519PublicKey::from_bytes([99u8; 32]);

        let signed = SignedCapability::new(cap, sig, key, 5000);
        assert_eq!(signed.signed_at_ns, 5000);
    }

    #[test]
    fn test_capability_signature_message_creation() {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let msg = CapabilitySignatureMessage::from_capability(&cap, 12345, 5000)
            .expect("message creation");
        assert_eq!(msg.nonce, 12345);
        assert_eq!(msg.created_at_ns, 5000);
    }

    #[test]
    fn test_capability_signature_message_to_bytes() {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let msg = CapabilitySignatureMessage::from_capability(&cap, 12345, 5000)
            .expect("message creation");
        let bytes = msg.to_bytes().expect("serialization");

        // Verify structure
        assert!(bytes.len() >= 32 + 32 + 8 + 8 + 8); // cap_id + chain_hash + len + created + nonce
    }

    #[test]
    fn test_sign_and_verify() {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let secret_key = Ed25519SecretKey::from_bytes([55u8; 32]);
        let public_key = Ed25519PublicKey::from_bytes([66u8; 32]);

        let msg = CapabilitySignatureMessage::from_capability(&cap, 12345, 5000)
            .expect("message creation");

        let signature = CryptographicSigner::sign(&msg, &secret_key).expect("sign");

        // Placeholder: verify always succeeds
        let result = CryptographicSigner::verify(&msg, &signature, &public_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_display() {
        let sig = Ed25519Signature::from_bytes([42u8; 64]);
        let display = sig.to_string();
        assert!(display.contains("Ed25519Signature"));
    }

    #[test]
    fn test_public_key_display() {
        let key = Ed25519PublicKey::from_bytes([99u8; 32]);
        let display = key.to_string();
        assert!(display.contains("Ed25519PublicKey"));
    }
}
