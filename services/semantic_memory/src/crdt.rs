// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Conflict-free Replicated Data Types (CRDTs) for crew-wide shared memory.
//!
//! This module provides CRDT types for maintaining consistency of L3 shared memory
//! across multiple crew members without requiring global coordination.
//!
//! See Engineering Plan § 4.1.4: CRDT Consistency & Replication.

use alloc::string::String;

/// Enumeration of supported CRDT types for crew-wide memory sharing.
///
/// Each CRDT type has different semantics for conflict resolution:
/// - GCounter: Growing counter (only increments)
/// - PNCounter: PN-Counter (can increment and decrement)
/// - LWWRegister: Last-Writer-Wins Register
/// - ORSet: Observed-Remove Set
/// - MVRegister: Multi-Value Register (tracks all concurrent values)
///
/// See Engineering Plan § 4.1.4: CRDT Types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrdtType {
    /// Grow-Only Counter - values only increase
    /// Used for monotonic counters (message counts, visits, etc.)
    GCounter,

    /// PN-Counter (Positive-Negative Counter) - can increment or decrement
    /// Implementation: GCounter for increments + GCounter for decrements
    PNCounter,

    /// Last-Writer-Wins Register - last write wins on concurrent updates
    /// Requires timestamp or version vector for ordering
    LWWRegister,

    /// Observed-Remove Set (ORSet) - allows concurrent add/remove
    /// Add wins over remove; tracks unique tags per element
    ORSet,

    /// Multi-Value Register - tracks all concurrent values
    /// Application must reconcile which value to use
    MVRegister,
}

impl CrdtType {
    /// Returns a human-readable name for this CRDT type.
    pub fn name(&self) -> &'static str {
        match self {
            CrdtType::GCounter => "g_counter",
            CrdtType::PNCounter => "pn_counter",
            CrdtType::LWWRegister => "lww_register",
            CrdtType::ORSet => "or_set",
            CrdtType::MVRegister => "mv_register",
        }
    }

    /// Returns a description of the CRDT's conflict resolution strategy.
    pub fn conflict_resolution(&self) -> &'static str {
        match self {
            CrdtType::GCounter => "monotonic increase (commutative)",
            CrdtType::PNCounter => "separate increment/decrement counters",
            CrdtType::LWWRegister => "timestamp-based: last writer wins",
            CrdtType::ORSet => "add wins over remove",
            CrdtType::MVRegister => "track all concurrent values",
        }
    }

    /// Returns whether this CRDT requires external timestamp/version tracking.
    pub fn requires_versioning(&self) -> bool {
        matches!(
            self,
            CrdtType::LWWRegister | CrdtType::MVRegister
        )
    }

    /// Returns whether this CRDT is idempotent (order-independent).
    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            CrdtType::GCounter
                | CrdtType::PNCounter
                | CrdtType::ORSet
        )
    }
}

/// Conflict resolution strategy for CRDT updates.
///
/// Defines how to resolve conflicts when concurrent updates occur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictResolution {
    /// No conflict - use the new value unconditionally
    Overwrite,

    /// Use the value with the highest timestamp
    /// For timestamp, lower values = earlier (Unix epoch format)
    HighestTimestamp,

    /// Use the value with the lowest timestamp
    LowestTimestamp,

    /// Merge/combine values (application-defined semantics)
    Merge,

    /// Keep all concurrent values for application resolution
    MultiValue,

    /// Use version vector causality for ordering
    VersionVector,
}

impl ConflictResolution {
    /// Returns a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            ConflictResolution::Overwrite => "new value overwrites old",
            ConflictResolution::HighestTimestamp => "value with highest timestamp wins",
            ConflictResolution::LowestTimestamp => "value with lowest timestamp wins",
            ConflictResolution::Merge => "application-defined merge",
            ConflictResolution::MultiValue => "keep all values for application resolution",
            ConflictResolution::VersionVector => "use version vector for causality",
        }
    }
}

/// Configuration for CRDT behavior in a shared memory region.
///
/// Controls how CRDTs handle replication, versioning, and conflict resolution.
///
/// See Engineering Plan § 4.1.4: CRDT Configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrdtConfig {
    /// Type of CRDT to use
    crdt_type: CrdtType,

    /// Conflict resolution strategy
    conflict_resolution: ConflictResolution,

    /// Maximum concurrent versions to track (for MVRegister)
    max_concurrent_versions: u32,

    /// Whether to automatically compact old versions
    auto_compact: bool,

    /// Replication timeout in milliseconds
    replication_timeout_ms: u64,

    /// Whether to require quorum for writes
    require_quorum: bool,
}

impl CrdtConfig {
    /// Creates a new CRDT configuration.
    ///
    /// # Arguments
    ///
    /// * `crdt_type` - Type of CRDT to use
    /// * `conflict_resolution` - Conflict resolution strategy
    pub fn new(crdt_type: CrdtType, conflict_resolution: ConflictResolution) -> Self {
        CrdtConfig {
            crdt_type,
            conflict_resolution,
            max_concurrent_versions: 10,
            auto_compact: true,
            replication_timeout_ms: 5000,
            require_quorum: true,
        }
    }

    /// Creates a config for a grow-only counter.
    pub fn g_counter() -> Self {
        CrdtConfig::new(CrdtType::GCounter, ConflictResolution::Overwrite)
    }

    /// Creates a config for a PN-counter.
    pub fn pn_counter() -> Self {
        CrdtConfig::new(CrdtType::PNCounter, ConflictResolution::Merge)
    }

    /// Creates a config for Last-Writer-Wins.
    pub fn lww_register() -> Self {
        CrdtConfig::new(CrdtType::LWWRegister, ConflictResolution::HighestTimestamp)
    }

    /// Creates a config for an Observed-Remove Set.
    pub fn or_set() -> Self {
        CrdtConfig::new(CrdtType::ORSet, ConflictResolution::Merge)
    }

    /// Creates a config for Multi-Value register.
    pub fn mv_register() -> Self {
        CrdtConfig::new(CrdtType::MVRegister, ConflictResolution::MultiValue)
    }

    /// Returns the CRDT type.
    pub fn crdt_type(&self) -> &CrdtType {
        &self.crdt_type
    }

    /// Returns the conflict resolution strategy.
    pub fn conflict_resolution(&self) -> &ConflictResolution {
        &self.conflict_resolution
    }

    /// Sets the maximum concurrent versions.
    pub fn set_max_concurrent_versions(&mut self, max_versions: u32) {
        self.max_concurrent_versions = max_versions;
    }

    /// Returns the maximum concurrent versions.
    pub fn max_concurrent_versions(&self) -> u32 {
        self.max_concurrent_versions
    }

    /// Sets whether to auto-compact.
    pub fn set_auto_compact(&mut self, enabled: bool) {
        self.auto_compact = enabled;
    }

    /// Returns whether auto-compact is enabled.
    pub fn auto_compact(&self) -> bool {
        self.auto_compact
    }

    /// Sets the replication timeout.
    pub fn set_replication_timeout_ms(&mut self, timeout_ms: u64) {
        self.replication_timeout_ms = timeout_ms;
    }

    /// Returns the replication timeout.
    pub fn replication_timeout_ms(&self) -> u64 {
        self.replication_timeout_ms
    }

    /// Sets whether quorum is required.
    pub fn set_require_quorum(&mut self, required: bool) {
        self.require_quorum = required;
    }

    /// Returns whether quorum is required.
    pub fn require_quorum(&self) -> bool {
        self.require_quorum
    }

    /// Returns a human-readable description of the configuration.
    pub fn description(&self) -> String {
        format!(
            "CrdtConfig {{ type: {}, resolution: {}, max_versions: {}, auto_compact: {}, quorum: {} }}",
            self.crdt_type.name(),
            self.conflict_resolution.description(),
            self.max_concurrent_versions,
            self.auto_compact,
            self.require_quorum
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_crdt_type_name() {
        assert_eq!(CrdtType::GCounter.name(), "g_counter");
        assert_eq!(CrdtType::PNCounter.name(), "pn_counter");
        assert_eq!(CrdtType::LWWRegister.name(), "lww_register");
        assert_eq!(CrdtType::ORSet.name(), "or_set");
        assert_eq!(CrdtType::MVRegister.name(), "mv_register");
    }

    #[test]
    fn test_crdt_type_conflict_resolution() {
        assert_eq!(
            CrdtType::GCounter.conflict_resolution(),
            "monotonic increase (commutative)"
        );
        assert_eq!(
            CrdtType::LWWRegister.conflict_resolution(),
            "timestamp-based: last writer wins"
        );
    }

    #[test]
    fn test_crdt_type_requires_versioning() {
        assert!(!CrdtType::GCounter.requires_versioning());
        assert!(CrdtType::LWWRegister.requires_versioning());
        assert!(CrdtType::MVRegister.requires_versioning());
    }

    #[test]
    fn test_crdt_type_is_idempotent() {
        assert!(CrdtType::GCounter.is_idempotent());
        assert!(CrdtType::ORSet.is_idempotent());
        assert!(!CrdtType::LWWRegister.is_idempotent());
    }

    #[test]
    fn test_conflict_resolution_description() {
        assert_eq!(
            ConflictResolution::Overwrite.description(),
            "new value overwrites old"
        );
        assert_eq!(
            ConflictResolution::HighestTimestamp.description(),
            "value with highest timestamp wins"
        );
        assert!(ConflictResolution::MultiValue
            .description()
            .contains("values for application"));
    }

    #[test]
    fn test_crdt_config_creation() {
        let config =
            CrdtConfig::new(CrdtType::ORSet, ConflictResolution::Merge);

        assert_eq!(config.crdt_type(), &CrdtType::ORSet);
        assert_eq!(config.conflict_resolution(), &ConflictResolution::Merge);
        assert!(config.auto_compact());
        assert!(config.require_quorum());
    }

    #[test]
    fn test_crdt_config_g_counter() {
        let config = CrdtConfig::g_counter();
        assert_eq!(config.crdt_type(), &CrdtType::GCounter);
        assert_eq!(
            config.conflict_resolution(),
            &ConflictResolution::Overwrite
        );
    }

    #[test]
    fn test_crdt_config_pn_counter() {
        let config = CrdtConfig::pn_counter();
        assert_eq!(config.crdt_type(), &CrdtType::PNCounter);
        assert_eq!(config.conflict_resolution(), &ConflictResolution::Merge);
    }

    #[test]
    fn test_crdt_config_lww_register() {
        let config = CrdtConfig::lww_register();
        assert_eq!(config.crdt_type(), &CrdtType::LWWRegister);
        assert_eq!(
            config.conflict_resolution(),
            &ConflictResolution::HighestTimestamp
        );
    }

    #[test]
    fn test_crdt_config_or_set() {
        let config = CrdtConfig::or_set();
        assert_eq!(config.crdt_type(), &CrdtType::ORSet);
        assert_eq!(config.conflict_resolution(), &ConflictResolution::Merge);
    }

    #[test]
    fn test_crdt_config_mv_register() {
        let config = CrdtConfig::mv_register();
        assert_eq!(config.crdt_type(), &CrdtType::MVRegister);
        assert_eq!(
            config.conflict_resolution(),
            &ConflictResolution::MultiValue
        );
    }

    #[test]
    fn test_crdt_config_set_max_versions() {
        let mut config = CrdtConfig::g_counter();
        config.set_max_concurrent_versions(20);
        assert_eq!(config.max_concurrent_versions(), 20);
    }

    #[test]
    fn test_crdt_config_auto_compact() {
        let mut config = CrdtConfig::g_counter();
        config.set_auto_compact(false);
        assert!(!config.auto_compact());
    }

    #[test]
    fn test_crdt_config_replication_timeout() {
        let mut config = CrdtConfig::g_counter();
        config.set_replication_timeout_ms(10000);
        assert_eq!(config.replication_timeout_ms(), 10000);
    }

    #[test]
    fn test_crdt_config_require_quorum() {
        let mut config = CrdtConfig::g_counter();
        config.set_require_quorum(false);
        assert!(!config.require_quorum());
    }

    #[test]
    fn test_crdt_config_description() {
        let config = CrdtConfig::lww_register();
        let desc = config.description();
        assert!(desc.contains("lww_register"));
        assert!(desc.contains("CrdtConfig"));
    }
}
