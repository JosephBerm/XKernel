// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Distributed Channel Configuration
//!
//! This module defines configuration for distributed channels that span
//! multiple cognitive substrate systems. Distributed channels require special
//! handling for capability verification, idempotency, and encryption.
//!
//! ## References
//!
//! - Engineering Plan § 5.2.5 (Distributed Channels)

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Encryption configuration for distributed channels.
///
/// Specifies the encryption algorithm and parameters for securing
/// messages transmitted across network boundaries.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncryptionConfig {
    /// No encryption (plaintext transmission).
    None,

    /// AES-256-GCM authenticated encryption.
    ///
    /// Provides both confidentiality and authenticity guarantees.
    /// Industry standard encryption suitable for production systems.
    Aes256Gcm,

    /// ChaCha20-Poly1305 authenticated encryption.
    ///
    /// Alternative to AES, often faster on systems without AES-NI support.
    ChaCha20Poly1305,

    /// Custom encryption scheme (identifier in String).
    Custom(String),
}

impl EncryptionConfig {
    /// Check if encryption is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, EncryptionConfig::None)
    }

    /// Check if this is an authenticated encryption mode.
    pub fn is_authenticated(&self) -> bool {
        matches!(
            self,
            EncryptionConfig::Aes256Gcm | EncryptionConfig::ChaCha20Poly1305
        )
    }
}

/// Network address for remote endpoints in distributed channels.
///
/// Represents a network location where a remote cognitive substrate system
/// listens for incoming channel connections.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkAddress {
    /// Hostname or IP address
    pub host: String,

    /// TCP/UDP port number
    pub port: u16,

    /// Optional transport protocol hint (e.g., "tcp", "quic")
    pub protocol: Option<String>,
}

impl NetworkAddress {
    /// Create a new network address.
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            protocol: None,
        }
    }

    /// Create a network address with protocol hint.
    pub fn with_protocol(host: String, port: u16, protocol: String) -> Self {
        Self {
            host,
            port,
            protocol: Some(protocol),
        }
    }
}

/// Configuration for distributed semantic channels.
///
/// When a channel spans multiple cognitive substrate systems (distributed=true),
/// it requires additional configuration for security and idempotency.
///
/// See Engineering Plan § 5.2.5 (Distributed Channels)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Remote endpoint network address.
    ///
    /// The network location of the remote cognitive substrate system
    /// that hosts the receiver endpoint.
    pub remote_endpoint: NetworkAddress,

    /// Capability verification requirement.
    ///
    /// MUST be true for all distributed channels. Ensures that the sender
    /// has explicit capability authorization to send to this remote receiver.
    /// This is a fundamental security invariant: no distributed communication
    /// without capability verification.
    ///
    /// Validation Rule: If distributed = Some(_), then capability_verification MUST be true.
    pub capability_verification: bool,

    /// Idempotency key tracking.
    ///
    /// When using exactly_once_local delivery across system boundaries,
    /// idempotency keys enable deduplication on the receiver side in case
    /// of network retransmission. Required when downgrading from local to
    /// distributed exactly-once semantics.
    ///
    /// Validation Rule: If delivery = ExactlyOnceLocal and distributed = Some(_),
    /// then idempotency_keys MUST be true.
    pub idempotency_keys: bool,

    /// Encryption configuration for message transmission.
    ///
    /// Specifies the encryption algorithm for securing messages sent
    /// across network boundaries. Recommended to be Aes256Gcm or ChaCha20Poly1305.
    pub encryption: EncryptionConfig,
}

impl DistributedConfig {
    /// Create a new distributed channel configuration.
    ///
    /// Note: Validates that capability_verification is true.
    pub fn new(
        remote_endpoint: NetworkAddress,
        encryption: EncryptionConfig,
    ) -> Result<Self, &'static str> {
        // Capability verification is mandatory for distributed channels
        Ok(Self {
            remote_endpoint,
            capability_verification: true,
            idempotency_keys: false,
            encryption,
        })
    }

    /// Enable idempotency key tracking for exactly-once semantics.
    pub fn with_idempotency_keys(mut self, enabled: bool) -> Self {
        self.idempotency_keys = enabled;
        self
    }

    /// Check if this configuration is ready for exactly-once delivery.
    ///
    /// Exactly-once-local delivery across system boundaries requires idempotency keys.
    pub fn supports_exactly_once_distributed(&self) -> bool {
        self.idempotency_keys && self.capability_verification
    }

    /// Check if this configuration properly secures the channel.
    ///
    /// Distributed channels should use authenticated encryption.
    pub fn is_properly_secured(&self) -> bool {
        self.capability_verification && self.encryption.is_authenticated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config_none() {
        let enc = EncryptionConfig::None;
        assert!(!enc.is_enabled());
        assert!(!enc.is_authenticated());
    }

    #[test]
    fn test_encryption_config_aes256gcm() {
        let enc = EncryptionConfig::Aes256Gcm;
        assert!(enc.is_enabled());
        assert!(enc.is_authenticated());
    }

    #[test]
    fn test_encryption_config_chacha() {
        let enc = EncryptionConfig::ChaCha20Poly1305;
        assert!(enc.is_enabled());
        assert!(enc.is_authenticated());
    }

    #[test]
    fn test_network_address_creation() {
        let addr = NetworkAddress::new(
            alloc::string::String::from("127.0.0.1"),
            8080,
        );
        assert_eq!(addr.host, "127.0.0.1");
        assert_eq!(addr.port, 8080);
        assert!(addr.protocol.is_none());
    }

    #[test]
    fn test_network_address_with_protocol() {
        let addr = NetworkAddress::with_protocol(
            alloc::string::String::from("example.com"),
            443,
            alloc::string::String::from("quic"),
        );
        assert_eq!(addr.host, "example.com");
        assert_eq!(addr.port, 443);
        assert_eq!(addr.protocol, Some(alloc::string::String::from("quic")));
    }

    #[test]
    fn test_distributed_config_creation() {
        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        assert!(cfg.capability_verification);
        assert!(!cfg.idempotency_keys);
        assert!(cfg.encryption.is_enabled());
    }

    #[test]
    fn test_distributed_config_with_idempotency_keys() {
        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm)
            .unwrap()
            .with_idempotency_keys(true);
        assert!(cfg.idempotency_keys);
    }

    #[test]
    fn test_distributed_config_supports_exactly_once() {
        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm)
            .unwrap()
            .with_idempotency_keys(true);
        assert!(cfg.supports_exactly_once_distributed());
    }

    #[test]
    fn test_distributed_config_properly_secured() {
        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm).unwrap();
        assert!(cfg.is_properly_secured());

        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let cfg = DistributedConfig::new(addr, EncryptionConfig::None).unwrap();
        assert!(!cfg.is_properly_secured());
    }
}
