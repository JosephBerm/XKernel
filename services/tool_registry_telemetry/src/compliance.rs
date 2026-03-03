// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Compliance and audit trail management.
//!
//! Provides data classification, retention policies, and audit logging
//! in compliance with GDPR, EU AI Act, and internal standards.
//!
//! See Engineering Plan § 2.12: CEF Compliance
//! and Addendum v2.5.1: GDPR and EU AI Act Requirements.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Retention policy for event data lifecycle management.
///
/// See Engineering Plan § 2.12: Retention Policy.
/// Defines how long events are retained across different compliance tiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Operational retention (days): How long to keep active operational events.
    /// Default: 7 days
    pub operational_days: u32,

    /// Compliance retention (months): How long to retain for audit/compliance.
    /// Default: 6+ months (per regulation)
    pub compliance_months: u32,

    /// Technical documentation retention (years): For documentation and incident investigation.
    /// Default: 10 years
    pub technical_doc_years: u32,
}

impl RetentionPolicy {
    /// Creates a new retention policy with specified periods.
    pub fn new(operational_days: u32, compliance_months: u32, technical_doc_years: u32) -> Self {
        RetentionPolicy {
            operational_days,
            compliance_months,
            technical_doc_years,
        }
    }

    /// Creates a default retention policy.
    /// Operational: 7 days, Compliance: 6 months, Technical: 10 years
    pub fn default() -> Self {
        RetentionPolicy {
            operational_days: 7,
            compliance_months: 6,
            technical_doc_years: 10,
        }
    }

    /// Creates a strict retention policy (shorter retention times).
    pub fn strict() -> Self {
        RetentionPolicy {
            operational_days: 3,
            compliance_months: 3,
            technical_doc_years: 5,
        }
    }

    /// Creates a long-term retention policy (extended retention).
    pub fn long_term() -> Self {
        RetentionPolicy {
            operational_days: 30,
            compliance_months: 24,
            technical_doc_years: 20,
        }
    }

    /// Returns true if this policy requires operational retention.
    pub fn requires_operational_retention(&self) -> bool {
        self.operational_days > 0
    }

    /// Returns true if this policy requires compliance retention.
    pub fn requires_compliance_retention(&self) -> bool {
        self.compliance_months > 0
    }

    /// Returns true if this policy requires technical documentation retention.
    pub fn requires_technical_retention(&self) -> bool {
        self.technical_doc_years > 0
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy::default()
    }
}

impl fmt::Display for RetentionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RetentionPolicy(op:{}d, comp:{}m, tech:{}y)",
            self.operational_days, self.compliance_months, self.technical_doc_years
        )
    }
}

/// Data classification for privacy and security handling.
///
/// See Engineering Plan § 2.12: Data Classification.
/// Determines how event data is protected, stored, and shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataClassification {
    /// Public data (no confidentiality concerns).
    /// Can be shared openly.
    Public,

    /// Internal data (restricted to organization).
    /// Access controlled within the organization.
    Internal,

    /// Confidential data (restricted to authorized personnel).
    /// Sensitive business information requiring protection.
    Confidential,

    /// PII (Personally Identifiable Information).
    /// Subject to GDPR Article 4 definition.
    /// Requires special protection and handling.
    PII,

    /// Regulated data (subject to specific regulations).
    /// E.g., healthcare data (HIPAA), financial data (PCI-DSS), etc.
    Regulated,
}

impl fmt::Display for DataClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataClassification::Public => write!(f, "Public"),
            DataClassification::Internal => write!(f, "Internal"),
            DataClassification::Confidential => write!(f, "Confidential"),
            DataClassification::PII => write!(f, "PII"),
            DataClassification::Regulated => write!(f, "Regulated"),
        }
    }
}

impl DataClassification {
    /// Returns true if this classification requires special handling (PII/Regulated).
    pub fn requires_special_handling(&self) -> bool {
        matches!(self, DataClassification::PII | DataClassification::Regulated)
    }

    /// Returns the minimum retention period in months required by compliance.
    pub fn minimum_retention_months(&self) -> u32 {
        match self {
            DataClassification::Public => 1,
            DataClassification::Internal => 3,
            DataClassification::Confidential => 6,
            DataClassification::PII => 12,      // GDPR minimum
            DataClassification::Regulated => 24, // Varies by regulation
        }
    }
}

/// Redaction policy for sensitive data.
///
/// See Engineering Plan § 2.12: Redaction.
/// Defines which fields to redact per classification level (GDPR Article 12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionPolicy {
    /// Fields to redact for Public classification.
    public_redact: Vec<String>,

    /// Fields to redact for Internal classification.
    internal_redact: Vec<String>,

    /// Fields to redact for Confidential classification.
    confidential_redact: Vec<String>,

    /// Fields to redact for PII classification.
    pii_redact: Vec<String>,

    /// Fields to redact for Regulated classification.
    regulated_redact: Vec<String>,
}

impl RedactionPolicy {
    /// Creates a new redaction policy.
    pub fn new() -> Self {
        RedactionPolicy {
            public_redact: Vec::new(),
            internal_redact: Vec::new(),
            confidential_redact: Vec::new(),
            pii_redact: Vec::new(),
            regulated_redact: Vec::new(),
        }
    }

    /// Creates a default GDPR-compliant redaction policy.
    pub fn gdpr_default() -> Self {
        let mut policy = RedactionPolicy::new();

        // PII fields to redact
        policy.pii_redact = alloc::vec![
            "user_id".to_string(),
            "email".to_string(),
            "phone".to_string(),
            "name".to_string(),
            "ip_address".to_string(),
        ];

        // Regulated fields
        policy.regulated_redact = alloc::vec![
            "credit_card".to_string(),
            "ssn".to_string(),
            "medical_record".to_string(),
        ];

        policy
    }

    /// Adds a field to redact for a given classification.
    pub fn add_redaction(mut self, classification: DataClassification, field: impl Into<String>) -> Self {
        let field_str = field.into();
        match classification {
            DataClassification::Public => self.public_redact.push(field_str),
            DataClassification::Internal => self.internal_redact.push(field_str),
            DataClassification::Confidential => self.confidential_redact.push(field_str),
            DataClassification::PII => self.pii_redact.push(field_str),
            DataClassification::Regulated => self.regulated_redact.push(field_str),
        }
        self
    }

    /// Gets redaction list for a classification level.
    pub fn get_redactions(&self, classification: DataClassification) -> &[String] {
        match classification {
            DataClassification::Public => &self.public_redact,
            DataClassification::Internal => &self.internal_redact,
            DataClassification::Confidential => &self.confidential_redact,
            DataClassification::PII => &self.pii_redact,
            DataClassification::Regulated => &self.regulated_redact,
        }
    }

    /// Returns true if a field should be redacted for a given classification.
    pub fn should_redact(&self, classification: DataClassification, field: &str) -> bool {
        self.get_redactions(classification)
            .iter()
            .any(|f| f == field)
    }
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        RedactionPolicy::gdpr_default()
    }
}

/// Audit record for compliance and verification.
///
/// See Engineering Plan § 2.12: Audit Trail.
/// Records immutable audit entries for compliance verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    /// Reference to the event being audited.
    pub event_ref: String,

    /// Merkle hash of the event for tamper detection.
    pub merkle_hash: String,

    /// Data classification level.
    pub data_classification: DataClassification,

    /// Retention tier applied.
    pub retention_tier: String,

    /// Whether redaction was applied to this record.
    pub redaction_applied: bool,

    /// Timestamp when this audit record was created.
    pub created_timestamp_ns: u64,
}

impl AuditRecord {
    /// Creates a new audit record.
    pub fn new(
        event_ref: impl Into<String>,
        merkle_hash: impl Into<String>,
        data_classification: DataClassification,
        retention_tier: impl Into<String>,
        redaction_applied: bool,
        created_timestamp_ns: u64,
    ) -> Self {
        AuditRecord {
            event_ref: event_ref.into(),
            merkle_hash: merkle_hash.into(),
            data_classification,
            retention_tier: retention_tier.into(),
            redaction_applied,
            created_timestamp_ns,
        }
    }

    /// Returns true if this record contains PII or Regulated data.
    pub fn contains_sensitive_data(&self) -> bool {
        self.data_classification.requires_special_handling()
    }

    /// Returns the minimum retention period for this record in months.
    pub fn minimum_retention_months(&self) -> u32 {
        self.data_classification.minimum_retention_months()
    }
}

impl fmt::Display for AuditRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AuditRecord(event={}, hash={}, classification={}, tier={}, redacted={})",
            self.event_ref,
            &self.merkle_hash[0..8.min(self.merkle_hash.len())],
            self.data_classification,
            self.retention_tier,
            self.redaction_applied
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec;

    // Tests for RetentionPolicy
    #[test]
    fn test_retention_policy_creation() {
        let policy = RetentionPolicy::new(7, 6, 10);
        assert_eq!(policy.operational_days, 7);
        assert_eq!(policy.compliance_months, 6);
        assert_eq!(policy.technical_doc_years, 10);
    }

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.operational_days, 7);
        assert_eq!(policy.compliance_months, 6);
        assert_eq!(policy.technical_doc_years, 10);
    }

    #[test]
    fn test_retention_policy_strict() {
        let policy = RetentionPolicy::strict();
        assert_eq!(policy.operational_days, 3);
        assert_eq!(policy.compliance_months, 3);
        assert_eq!(policy.technical_doc_years, 5);
    }

    #[test]
    fn test_retention_policy_long_term() {
        let policy = RetentionPolicy::long_term();
        assert_eq!(policy.operational_days, 30);
        assert_eq!(policy.compliance_months, 24);
        assert_eq!(policy.technical_doc_years, 20);
    }

    #[test]
    fn test_retention_policy_requires_operational() {
        let policy = RetentionPolicy::new(7, 6, 10);
        assert!(policy.requires_operational_retention());

        let zero_policy = RetentionPolicy::new(0, 6, 10);
        assert!(!zero_policy.requires_operational_retention());
    }

    #[test]
    fn test_retention_policy_requires_compliance() {
        let policy = RetentionPolicy::new(7, 6, 10);
        assert!(policy.requires_compliance_retention());

        let zero_policy = RetentionPolicy::new(7, 0, 10);
        assert!(!zero_policy.requires_compliance_retention());
    }

    #[test]
    fn test_retention_policy_requires_technical() {
        let policy = RetentionPolicy::new(7, 6, 10);
        assert!(policy.requires_technical_retention());

        let zero_policy = RetentionPolicy::new(7, 6, 0);
        assert!(!zero_policy.requires_technical_retention());
    }

    #[test]
    fn test_retention_policy_display() {
        let policy = RetentionPolicy::new(7, 6, 10);
        let display = policy.to_string();
        assert!(display.contains("7d"));
        assert!(display.contains("6m"));
        assert!(display.contains("10y"));
    }

    #[test]
    fn test_retention_policy_equality() {
        let policy1 = RetentionPolicy::new(7, 6, 10);
        let policy2 = RetentionPolicy::new(7, 6, 10);
        assert_eq!(policy1, policy2);
    }

    #[test]
    fn test_retention_policy_clone() {
        let policy = RetentionPolicy::new(7, 6, 10);
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    // Tests for DataClassification
    #[test]
    fn test_data_classification_display() {
        assert_eq!(DataClassification::Public.to_string(), "Public");
        assert_eq!(DataClassification::Internal.to_string(), "Internal");
        assert_eq!(DataClassification::Confidential.to_string(), "Confidential");
        assert_eq!(DataClassification::PII.to_string(), "PII");
        assert_eq!(DataClassification::Regulated.to_string(), "Regulated");
    }

    #[test]
    fn test_data_classification_requires_special_handling() {
        assert!(!DataClassification::Public.requires_special_handling());
        assert!(!DataClassification::Internal.requires_special_handling());
        assert!(!DataClassification::Confidential.requires_special_handling());
        assert!(DataClassification::PII.requires_special_handling());
        assert!(DataClassification::Regulated.requires_special_handling());
    }

    #[test]
    fn test_data_classification_minimum_retention() {
        assert_eq!(DataClassification::Public.minimum_retention_months(), 1);
        assert_eq!(DataClassification::Internal.minimum_retention_months(), 3);
        assert_eq!(DataClassification::Confidential.minimum_retention_months(), 6);
        assert_eq!(DataClassification::PII.minimum_retention_months(), 12);
        assert_eq!(DataClassification::Regulated.minimum_retention_months(), 24);
    }

    #[test]
    fn test_data_classification_equality() {
        assert_eq!(DataClassification::PII, DataClassification::PII);
        assert_ne!(DataClassification::PII, DataClassification::Public);
    }

    // Tests for RedactionPolicy
    #[test]
    fn test_redaction_policy_new() {
        let policy = RedactionPolicy::new();
        assert!(policy.public_redact.is_empty());
        assert!(policy.pii_redact.is_empty());
    }

    #[test]
    fn test_redaction_policy_gdpr_default() {
        let policy = RedactionPolicy::gdpr_default();
        assert!(!policy.pii_redact.is_empty());
        assert!(!policy.regulated_redact.is_empty());
    }

    #[test]
    fn test_redaction_policy_add_redaction() {
        let policy = RedactionPolicy::new()
            .add_redaction(DataClassification::PII, "user_id")
            .add_redaction(DataClassification::PII, "email");

        assert_eq!(policy.pii_redact.len(), 2);
    }

    #[test]
    fn test_redaction_policy_should_redact() {
        let policy = RedactionPolicy::new().add_redaction(DataClassification::PII, "user_id");

        assert!(policy.should_redact(DataClassification::PII, "user_id"));
        assert!(!policy.should_redact(DataClassification::PII, "other_field"));
    }

    #[test]
    fn test_redaction_policy_get_redactions() {
        let policy = RedactionPolicy::new()
            .add_redaction(DataClassification::PII, "user_id")
            .add_redaction(DataClassification::PII, "email");

        let redactions = policy.get_redactions(DataClassification::PII);
        assert_eq!(redactions.len(), 2);
    }

    #[test]
    fn test_redaction_policy_equality() {
        let policy1 = RedactionPolicy::gdpr_default();
        let policy2 = RedactionPolicy::gdpr_default();
        assert_eq!(policy1, policy2);
    }

    #[test]
    fn test_redaction_policy_clone() {
        let policy = RedactionPolicy::gdpr_default();
        let cloned = policy.clone();
        assert_eq!(policy, cloned);
    }

    #[test]
    fn test_redaction_policy_default() {
        let policy = RedactionPolicy::default();
        assert!(!policy.pii_redact.is_empty());
    }

    // Tests for AuditRecord
    #[test]
    fn test_audit_record_creation() {
        let record = AuditRecord::new(
            "event-001",
            "hash-abc123",
            DataClassification::Confidential,
            "standard",
            false,
            1000,
        );

        assert_eq!(record.event_ref, "event-001");
        assert_eq!(record.merkle_hash, "hash-abc123");
        assert_eq!(record.data_classification, DataClassification::Confidential);
        assert_eq!(record.retention_tier, "standard");
        assert!(!record.redaction_applied);
        assert_eq!(record.created_timestamp_ns, 1000);
    }

    #[test]
    fn test_audit_record_contains_sensitive_data() {
        let public_record = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Public,
            "tier",
            false,
            1000,
        );
        assert!(!public_record.contains_sensitive_data());

        let pii_record = AuditRecord::new(
            "event-002",
            "hash",
            DataClassification::PII,
            "tier",
            false,
            1000,
        );
        assert!(pii_record.contains_sensitive_data());

        let regulated_record = AuditRecord::new(
            "event-003",
            "hash",
            DataClassification::Regulated,
            "tier",
            false,
            1000,
        );
        assert!(regulated_record.contains_sensitive_data());
    }

    #[test]
    fn test_audit_record_minimum_retention() {
        let record = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::PII,
            "tier",
            false,
            1000,
        );
        assert_eq!(record.minimum_retention_months(), 12);
    }

    #[test]
    fn test_audit_record_equality() {
        let record1 = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Confidential,
            "tier",
            false,
            1000,
        );
        let record2 = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Confidential,
            "tier",
            false,
            1000,
        );
        assert_eq!(record1, record2);
    }

    #[test]
    fn test_audit_record_clone() {
        let record = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Regulated,
            "tier",
            true,
            1000,
        );
        let cloned = record.clone();
        assert_eq!(record, cloned);
    }

    #[test]
    fn test_audit_record_display() {
        let record = AuditRecord::new(
            "event-001",
            "hash-long-value",
            DataClassification::PII,
            "compliance",
            true,
            1000,
        );
        let display = record.to_string();
        assert!(display.contains("event-001"));
        assert!(display.contains("PII"));
        assert!(display.contains("true"));
    }

    #[test]
    fn test_audit_record_redaction_applied() {
        let record_not_redacted = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Internal,
            "tier",
            false,
            1000,
        );
        assert!(!record_not_redacted.redaction_applied);

        let record_redacted = AuditRecord::new(
            "event-002",
            "hash",
            DataClassification::PII,
            "tier",
            true,
            1000,
        );
        assert!(record_redacted.redaction_applied);
    }

    #[test]
    fn test_retention_policy_combined_checks() {
        let policy = RetentionPolicy::new(7, 6, 10);
        assert!(policy.requires_operational_retention());
        assert!(policy.requires_compliance_retention());
        assert!(policy.requires_technical_retention());

        let partial = RetentionPolicy::new(0, 6, 0);
        assert!(!partial.requires_operational_retention());
        assert!(partial.requires_compliance_retention());
        assert!(!partial.requires_technical_retention());
    }

    #[test]
    fn test_redaction_policy_multiple_classifications() {
        let policy = RedactionPolicy::new()
            .add_redaction(DataClassification::Public, "timestamp")
            .add_redaction(DataClassification::PII, "user_id")
            .add_redaction(DataClassification::PII, "email")
            .add_redaction(DataClassification::Regulated, "ssn");

        assert_eq!(policy.get_redactions(DataClassification::Public).len(), 1);
        assert_eq!(policy.get_redactions(DataClassification::PII).len(), 2);
        assert_eq!(policy.get_redactions(DataClassification::Regulated).len(), 1);
    }

    #[test]
    fn test_audit_record_timestamps() {
        let record1 = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Internal,
            "tier",
            false,
            1000,
        );
        let record2 = AuditRecord::new(
            "event-001",
            "hash",
            DataClassification::Internal,
            "tier",
            false,
            2000,
        );
        assert_ne!(record1, record2);
    }
}
