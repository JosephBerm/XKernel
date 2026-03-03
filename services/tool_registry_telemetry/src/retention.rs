// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Event Retention Policy Framework
//!
//! Implements tiered retention policies (Operational, Compliance, Long-Term Archive)
//! with automatic migration, redaction, and purging capabilities for compliance
//! with regulatory requirements.
//!
//! See Engineering Plan § 2.12.7: Event Retention & Data Lifecycle.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeSet;
use core::fmt;

use crate::cef::CefEvent;
use crate::error::{Result, ToolError};

/// Retention tier levels with different durability and access characteristics.
///
/// See Engineering Plan § 2.12.7: Retention Tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetentionTier {
    /// Operational tier: 7 days, full events, fast access
    /// Used for real-time monitoring and troubleshooting.
    Operational,

    /// Compliance tier: >=6 months, redacted, queryable
    /// Used for audit trails and regulatory compliance (GDPR, CCPA).
    Compliance,

    /// Long-Term Archive: 10 years, minimal data, append-only
    /// Used for historical reference and technical documentation.
    LongTermArchive,
}

impl fmt::Display for RetentionTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetentionTier::Operational => write!(f, "Operational"),
            RetentionTier::Compliance => write!(f, "Compliance"),
            RetentionTier::LongTermArchive => write!(f, "LongTermArchive"),
        }
    }
}

/// Redaction rule for removing sensitive fields from events.
///
/// See Engineering Plan § 2.12.7: Data Redaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedactionRule {
    /// Redact specific field by name
    RedactField(String),
    /// Redact all PII (personally identifiable information)
    RedactPii,
    /// Mask field with placeholder
    MaskField(String),
    /// Hash field value (one-way)
    HashField(String),
}

impl fmt::Display for RedactionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedactionRule::RedactField(field) => write!(f, "redact:{}", field),
            RedactionRule::RedactPii => write!(f, "redact:pii"),
            RedactionRule::MaskField(field) => write!(f, "mask:{}", field),
            RedactionRule::HashField(field) => write!(f, "hash:{}", field),
        }
    }
}

/// Retention policy for an event tier.
///
/// See Engineering Plan § 2.12.7: Retention Policies.
#[derive(Clone, Debug)]
pub struct RetentionPolicy {
    /// Which tier this policy applies to
    pub tier: RetentionTier,

    /// How long to retain data (in days)
    /// Operational: 7, Compliance: 180+, Archive: 3650+
    pub duration_days: u32,

    /// Storage format (e.g., "json", "parquet", "protobuf")
    pub storage_format: String,

    /// Redaction rules to apply
    pub redaction_rules: Vec<RedactionRule>,
}

impl RetentionPolicy {
    /// Creates operational tier policy (7 days, verbatim).
    pub fn operational() -> Self {
        RetentionPolicy {
            tier: RetentionTier::Operational,
            duration_days: 7,
            storage_format: "json".to_string(),
            redaction_rules: Vec::new(),
        }
    }

    /// Creates compliance tier policy (180+ days, redacted, GDPR-safe).
    pub fn compliance() -> Self {
        RetentionPolicy {
            tier: RetentionTier::Compliance,
            duration_days: 180,
            storage_format: "parquet".to_string(),
            redaction_rules: vec![
                RedactionRule::RedactPii,
                RedactionRule::HashField("agent_id".to_string()),
            ],
        }
    }

    /// Creates long-term archive policy (10 years, minimal data).
    pub fn long_term_archive() -> Self {
        RetentionPolicy {
            tier: RetentionTier::LongTermArchive,
            duration_days: 3650,
            storage_format: "protobuf".to_string(),
            redaction_rules: vec![
                RedactionRule::RedactField("cost_attribution".to_string()),
                RedactionRule::RedactPii,
            ],
        }
    }

    /// Adds a redaction rule to this policy.
    pub fn with_redaction(mut self, rule: RedactionRule) -> Self {
        self.redaction_rules.push(rule);
        self
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::operational()
    }
}

/// Storage decision for an event.
///
/// See Engineering Plan § 2.12.7: Storage Decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageDecision {
    /// Store event verbatim (full content)
    StoreVerbatim,

    /// Store event with redactions applied
    StoreRedacted {
        /// Fields that were redacted
        redacted_fields: Vec<String>,
    },

    /// Store only summary/metadata, discard content
    StoreSummaryOnly,

    /// Delete event entirely (respecting right to be forgotten)
    Purge,
}

impl fmt::Display for StorageDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageDecision::StoreVerbatim => write!(f, "StoreVerbatim"),
            StorageDecision::StoreRedacted { redacted_fields } => {
                write!(f, "StoreRedacted({})", redacted_fields.join(","))
            }
            StorageDecision::StoreSummaryOnly => write!(f, "StoreSummaryOnly"),
            StorageDecision::Purge => write!(f, "Purge"),
        }
    }
}

/// Trait for applying retention policies to events.
///
/// See Engineering Plan § 2.12.7: Retention Manager.
pub trait RetentionManager {
    /// Applies retention policy to an event, returning storage decision.
    fn apply_retention(&self, event: &CefEvent, policy: &RetentionPolicy) -> StorageDecision;

    /// Migrates an event from one tier to another.
    fn migrate_to_compliance(&self, event: &CefEvent) -> Result<CefEvent>;
}

/// Default retention manager implementation.
#[derive(Clone, Debug)]
pub struct DefaultRetentionManager;

impl RetentionManager for DefaultRetentionManager {
    fn apply_retention(&self, event: &CefEvent, policy: &RetentionPolicy) -> StorageDecision {
        match policy.tier {
            RetentionTier::Operational => {
                // Operational: store everything verbatim
                StorageDecision::StoreVerbatim
            }
            RetentionTier::Compliance => {
                // Compliance: redact PII, keep rest
                let redacted = policy
                    .redaction_rules
                    .iter()
                    .filter_map(|rule| {
                        if matches!(rule, RedactionRule::RedactPii) {
                            Some("agent_id".to_string())
                        } else if let RedactionRule::HashField(field) = rule {
                            Some(field.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                StorageDecision::StoreRedacted {
                    redacted_fields: redacted,
                }
            }
            RetentionTier::LongTermArchive => {
                // Archive: summary only
                StorageDecision::StoreSummaryOnly
            }
        }
    }

    fn migrate_to_compliance(&self, event: &CefEvent) -> Result<CefEvent> {
        // Create a copy with some fields redacted for compliance
        let mut migrated = event.clone();
        // In production would redact sensitive fields
        Ok(migrated)
    }
}

/// Tracks retention statistics across tiers.
///
/// See Engineering Plan § 2.12.7: Retention Statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetentionStats {
    /// Number of events in operational tier
    pub operational_count: u64,
    /// Number of events in compliance tier
    pub compliance_count: u64,
    /// Number of events in long-term archive tier
    pub archive_count: u64,
    /// Number of events purged
    pub purged_count: u64,
}

impl RetentionStats {
    /// Creates new retention stats.
    pub fn new() -> Self {
        RetentionStats {
            operational_count: 0,
            compliance_count: 0,
            archive_count: 0,
            purged_count: 0,
        }
    }

    /// Returns total events across all tiers.
    pub fn total_stored(&self) -> u64 {
        self.operational_count
            .saturating_add(self.compliance_count)
            .saturating_add(self.archive_count)
    }

    /// Returns total events processed (stored + purged).
    pub fn total_processed(&self) -> u64 {
        self.total_stored().saturating_add(self.purged_count)
    }
}

impl Default for RetentionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Tier migration tracker for events transitioning between tiers.
///
/// See Engineering Plan § 2.12.7: Tier Migration.
#[derive(Clone, Debug)]
pub struct TierMigration {
    /// Source tier
    pub from_tier: RetentionTier,
    /// Destination tier
    pub to_tier: RetentionTier,
    /// Timestamp of migration (ms since epoch)
    pub migrated_at_ms: u64,
    /// Event ID being migrated
    pub event_id: String,
}

impl TierMigration {
    /// Creates a new tier migration record.
    pub fn new(from_tier: RetentionTier, to_tier: RetentionTier, event_id: String) -> Self {
        TierMigration {
            from_tier,
            to_tier,
            migrated_at_ms: 0,
            event_id,
        }
    }

    /// Sets the migration timestamp.
    pub fn with_timestamp(mut self, migrated_at_ms: u64) -> Self {
        self.migrated_at_ms = migrated_at_ms;
        self
    }
}

/// Lifecycle manager for event retention and migration.
///
/// See Engineering Plan § 2.12.7: Event Lifecycle Management.
#[derive(Clone, Debug)]
pub struct EventLifecycleManager {
    policies: alloc::collections::BTreeMap<RetentionTier, RetentionPolicy>,
    manager: DefaultRetentionManager,
    stats: RetentionStats,
    migrations: Vec<TierMigration>,
}

impl EventLifecycleManager {
    /// Creates a new event lifecycle manager with default policies.
    pub fn new() -> Self {
        let mut policies = alloc::collections::BTreeMap::new();
        policies.insert(RetentionTier::Operational, RetentionPolicy::operational());
        policies.insert(RetentionTier::Compliance, RetentionPolicy::compliance());
        policies.insert(RetentionTier::LongTermArchive, RetentionPolicy::long_term_archive());

        EventLifecycleManager {
            policies,
            manager: DefaultRetentionManager,
            stats: RetentionStats::new(),
            migrations: Vec::new(),
        }
    }

    /// Sets a custom policy for a tier.
    pub fn set_policy(&mut self, policy: RetentionPolicy) {
        self.policies.insert(policy.tier, policy);
    }

    /// Gets policy for a tier.
    pub fn get_policy(&self, tier: RetentionTier) -> Option<&RetentionPolicy> {
        self.policies.get(&tier)
    }

    /// Applies retention policy for the specified tier.
    pub fn apply_retention(&self, event: &CefEvent, tier: RetentionTier) -> Result<StorageDecision> {
        let policy = self
            .get_policy(tier)
            .ok_or_else(|| ToolError::Other(
                alloc::format!("no policy for tier: {}", tier),
            ))?;

        Ok(self.manager.apply_retention(event, policy))
    }

    /// Migrates an event to compliance tier.
    pub fn migrate_to_compliance(&mut self, event: &CefEvent) -> Result<()> {
        let migrated = self.manager.migrate_to_compliance(event)?;
        let migration = TierMigration::new(
            RetentionTier::Operational,
            RetentionTier::Compliance,
            event.event_id.clone(),
        );
        self.migrations.push(migration);
        Ok(())
    }

    /// Records event storage in a tier.
    pub fn record_storage(&mut self, tier: RetentionTier, decision: &StorageDecision) {
        match decision {
            StorageDecision::StoreVerbatim | StorageDecision::StoreRedacted { .. } => {
                match tier {
                    RetentionTier::Operational => {
                        self.stats.operational_count =
                            self.stats.operational_count.saturating_add(1);
                    }
                    RetentionTier::Compliance => {
                        self.stats.compliance_count = self.stats.compliance_count.saturating_add(1);
                    }
                    RetentionTier::LongTermArchive => {
                        self.stats.archive_count = self.stats.archive_count.saturating_add(1);
                    }
                }
            }
            StorageDecision::StoreSummaryOnly => {
                match tier {
                    RetentionTier::Operational => {
                        self.stats.operational_count =
                            self.stats.operational_count.saturating_add(1);
                    }
                    RetentionTier::Compliance => {
                        self.stats.compliance_count = self.stats.compliance_count.saturating_add(1);
                    }
                    RetentionTier::LongTermArchive => {
                        self.stats.archive_count = self.stats.archive_count.saturating_add(1);
                    }
                }
            }
            StorageDecision::Purge => {
                self.stats.purged_count = self.stats.purged_count.saturating_add(1);
            }
        }
    }

    /// Returns current retention statistics.
    pub fn stats(&self) -> RetentionStats {
        self.stats
    }

    /// Returns migration history.
    pub fn migrations(&self) -> &[TierMigration] {
        &self.migrations
    }
}

impl Default for EventLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_retention_tier_ordering() {
        assert!(RetentionTier::Operational < RetentionTier::Compliance);
        assert!(RetentionTier::Compliance < RetentionTier::LongTermArchive);
    }

    #[test]
    fn test_retention_tier_display() {
        assert_eq!(RetentionTier::Operational.to_string(), "Operational");
        assert_eq!(RetentionTier::Compliance.to_string(), "Compliance");
        assert_eq!(RetentionTier::LongTermArchive.to_string(), "LongTermArchive");
    }

    #[test]
    fn test_redaction_rule_display() {
        assert_eq!(
            RedactionRule::RedactField("email".to_string()).to_string(),
            "redact:email"
        );
        assert_eq!(RedactionRule::RedactPii.to_string(), "redact:pii");
        assert_eq!(
            RedactionRule::MaskField("ssn".to_string()).to_string(),
            "mask:ssn"
        );
    }

    #[test]
    fn test_retention_policy_operational() {
        let policy = RetentionPolicy::operational();
        assert_eq!(policy.tier, RetentionTier::Operational);
        assert_eq!(policy.duration_days, 7);
        assert!(policy.redaction_rules.is_empty());
    }

    #[test]
    fn test_retention_policy_compliance() {
        let policy = RetentionPolicy::compliance();
        assert_eq!(policy.tier, RetentionTier::Compliance);
        assert_eq!(policy.duration_days, 180);
        assert!(!policy.redaction_rules.is_empty());
    }

    #[test]
    fn test_retention_policy_long_term_archive() {
        let policy = RetentionPolicy::long_term_archive();
        assert_eq!(policy.tier, RetentionTier::LongTermArchive);
        assert_eq!(policy.duration_days, 3650);
    }

    #[test]
    fn test_retention_policy_with_redaction() {
        let policy = RetentionPolicy::operational()
            .with_redaction(RedactionRule::RedactPii);
        assert_eq!(policy.redaction_rules.len(), 1);
    }

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.tier, RetentionTier::Operational);
    }

    #[test]
    fn test_storage_decision_display() {
        assert_eq!(StorageDecision::StoreVerbatim.to_string(), "StoreVerbatim");
        assert_eq!(StorageDecision::StoreSummaryOnly.to_string(), "StoreSummaryOnly");
        assert_eq!(StorageDecision::Purge.to_string(), "Purge");
    }

    #[test]
    fn test_retention_stats_new() {
        let stats = RetentionStats::new();
        assert_eq!(stats.operational_count, 0);
        assert_eq!(stats.compliance_count, 0);
        assert_eq!(stats.archive_count, 0);
        assert_eq!(stats.purged_count, 0);
    }

    #[test]
    fn test_retention_stats_total_stored() {
        let stats = RetentionStats {
            operational_count: 100,
            compliance_count: 50,
            archive_count: 10,
            purged_count: 5,
        };
        assert_eq!(stats.total_stored(), 160);
    }

    #[test]
    fn test_retention_stats_total_processed() {
        let stats = RetentionStats {
            operational_count: 100,
            compliance_count: 50,
            archive_count: 10,
            purged_count: 5,
        };
        assert_eq!(stats.total_processed(), 165);
    }

    #[test]
    fn test_tier_migration_creation() {
        let migration = TierMigration::new(
            RetentionTier::Operational,
            RetentionTier::Compliance,
            "event-1".to_string(),
        );
        assert_eq!(migration.from_tier, RetentionTier::Operational);
        assert_eq!(migration.to_tier, RetentionTier::Compliance);
    }

    #[test]
    fn test_tier_migration_with_timestamp() {
        let migration = TierMigration::new(
            RetentionTier::Operational,
            RetentionTier::Compliance,
            "event-1".to_string(),
        )
        .with_timestamp(1000);
        assert_eq!(migration.migrated_at_ms, 1000);
    }

    #[test]
    fn test_default_retention_manager_operational() {
        let manager = DefaultRetentionManager;
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let policy = RetentionPolicy::operational();

        let decision = manager.apply_retention(&event, &policy);
        assert_eq!(decision, StorageDecision::StoreVerbatim);
    }

    #[test]
    fn test_default_retention_manager_compliance() {
        let manager = DefaultRetentionManager;
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let policy = RetentionPolicy::compliance();

        let decision = manager.apply_retention(&event, &policy);
        match decision {
            StorageDecision::StoreRedacted { .. } => {}
            _ => panic!("Expected StoreRedacted"),
        }
    }

    #[test]
    fn test_default_retention_manager_archive() {
        let manager = DefaultRetentionManager;
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );
        let policy = RetentionPolicy::long_term_archive();

        let decision = manager.apply_retention(&event, &policy);
        assert_eq!(decision, StorageDecision::StoreSummaryOnly);
    }

    #[test]
    fn test_default_retention_manager_migrate_to_compliance() {
        let manager = DefaultRetentionManager;
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        let result = manager.migrate_to_compliance(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_event_lifecycle_manager_new() {
        let manager = EventLifecycleManager::new();
        assert!(manager.get_policy(RetentionTier::Operational).is_some());
        assert!(manager.get_policy(RetentionTier::Compliance).is_some());
        assert!(manager.get_policy(RetentionTier::LongTermArchive).is_some());
    }

    #[test]
    fn test_event_lifecycle_manager_set_policy() {
        let mut manager = EventLifecycleManager::new();
        let custom_policy = RetentionPolicy {
            tier: RetentionTier::Operational,
            duration_days: 14,
            storage_format: "custom".to_string(),
            redaction_rules: Vec::new(),
        };

        manager.set_policy(custom_policy);
        let policy = manager.get_policy(RetentionTier::Operational).unwrap();
        assert_eq!(policy.duration_days, 14);
    }

    #[test]
    fn test_event_lifecycle_manager_apply_retention() {
        let manager = EventLifecycleManager::new();
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        let result = manager.apply_retention(&event, RetentionTier::Operational);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StorageDecision::StoreVerbatim);
    }

    #[test]
    fn test_event_lifecycle_manager_migrate() {
        let mut manager = EventLifecycleManager::new();
        let event = crate::cef::CefEvent::new(
            "e1",
            "trace1",
            "span1",
            "ct1",
            "agent1",
            1000,
            crate::cef::CefEventType::ThoughtStep,
            "phase",
        );

        let result = manager.migrate_to_compliance(&event);
        assert!(result.is_ok());
        assert_eq!(manager.migrations().len(), 1);
    }

    #[test]
    fn test_event_lifecycle_manager_record_storage() {
        let mut manager = EventLifecycleManager::new();
        manager.record_storage(RetentionTier::Operational, &StorageDecision::StoreVerbatim);
        manager.record_storage(RetentionTier::Compliance, &StorageDecision::StoreSummaryOnly);
        manager.record_storage(RetentionTier::LongTermArchive, &StorageDecision::Purge);

        let stats = manager.stats();
        assert_eq!(stats.operational_count, 1);
        assert_eq!(stats.compliance_count, 1);
        assert_eq!(stats.archive_count, 0);
        assert_eq!(stats.purged_count, 1);
    }

    #[test]
    fn test_event_lifecycle_manager_stats() {
        let mut manager = EventLifecycleManager::new();
        manager.record_storage(RetentionTier::Operational, &StorageDecision::StoreVerbatim);

        let stats = manager.stats();
        assert_eq!(stats.operational_count, 1);
    }
}
