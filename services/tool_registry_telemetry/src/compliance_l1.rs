// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! L1 Compliance extensions: policy engine, Merkle audit, and data retention.

use alloc::vec::Vec;
use core::fmt;

/// Policy decision for compliance evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    RequireApproval,
    LogOnly,
}

impl fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Deny => write!(f, "Deny"),
            Self::RequireApproval => write!(f, "RequireApproval"),
            Self::LogOnly => write!(f, "LogOnly"),
        }
    }
}

/// Policy evaluator trait for extensible compliance checks
pub trait PolicyEvaluator: Send + Sync {
    fn evaluate(&self, context: &PolicyContext) -> PolicyDecision;
}

/// Policy evaluation context
#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub user_id: u64,
    pub tool_id: u64,
    pub action: u32,
    pub resource: u64,
    pub timestamp: u64,
}

impl PolicyContext {
    pub fn new(user_id: u64, tool_id: u64, action: u32, resource: u64) -> Self {
        Self {
            user_id,
            tool_id,
            action,
            resource,
            timestamp: 0,
        }
    }
}

/// Merkle node for audit tree
#[derive(Debug, Clone, Copy)]
pub struct MerkleNode {
    pub hash: u64,
    pub left_hash: u64,
    pub right_hash: u64,
    pub is_leaf: bool,
}

impl MerkleNode {
    pub fn new_leaf(data_hash: u64) -> Self {
        Self {
            hash: data_hash,
            left_hash: 0,
            right_hash: 0,
            is_leaf: true,
        }
    }

    pub fn new_parent(left_hash: u64, right_hash: u64) -> Self {
        // Simple hash combination
        let hash = (left_hash ^ right_hash).wrapping_mul(31);

        Self {
            hash,
            left_hash,
            right_hash,
            is_leaf: false,
        }
    }

    /// Verify integrity by checking hash consistency
    pub fn verify(&self) -> bool {
        if self.is_leaf {
            self.hash != 0
        } else {
            let recomputed = Self::new_parent(self.left_hash, self.right_hash).hash;
            self.hash == recomputed
        }
    }
}

/// Audit entry in compliance log
#[derive(Debug, Clone, Copy)]
pub struct AuditEntry {
    pub entry_id: u64,
    pub timestamp: u64,
    pub user_id: u64,
    pub action: u32,
    pub decision: u8, // PolicyDecision as u8
    pub merkle_hash: u64,
}

impl AuditEntry {
    pub fn new(entry_id: u64, user_id: u64, action: u32, decision: PolicyDecision) -> Self {
        Self {
            entry_id,
            timestamp: 0,
            user_id,
            action,
            decision: decision as u8,
            merkle_hash: 0,
        }
    }
}

/// Audit provider trait
pub trait AuditProvider: Send + Sync {
    fn log_entry(&mut self, entry: AuditEntry) -> Result<(), ComplianceError>;
    fn get_entry(&self, entry_id: u64) -> Option<AuditEntry>;
    fn verify_chain(&self) -> bool;
}

/// Data retention policy
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    pub retention_days: u16,
    pub min_retention_days: u16,
    pub encryption_required: bool,
    pub backup_required: bool,
}

impl RetentionPolicy {
    pub fn new(retention_days: u16) -> Self {
        Self {
            retention_days,
            min_retention_days: 7,
            encryption_required: true,
            backup_required: true,
        }
    }

    pub fn is_retention_valid(&self) -> bool {
        self.retention_days >= self.min_retention_days
    }

    /// Check if data should be purged based on age
    pub fn should_purge(&self, age_days: u16) -> bool {
        age_days > self.retention_days
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::new(90) // 90-day retention by default
    }
}

/// Compliance engine
#[derive(Debug, Clone)]
pub struct ComplianceEngine {
    audit_log: Vec<AuditEntry>,
    merkle_tree: Vec<MerkleNode>,
    retention_policy: RetentionPolicy,
    max_audit_entries: usize,
}

impl ComplianceEngine {
    pub fn new(retention_policy: RetentionPolicy, max_entries: usize) -> Self {
        Self {
            audit_log: Vec::new(),
            merkle_tree: Vec::new(),
            retention_policy,
            max_audit_entries: max_entries,
        }
    }

    /// Log an audit entry
    pub fn log_entry(&mut self, entry: AuditEntry) -> Result<(), ComplianceError> {
        if self.audit_log.len() >= self.max_audit_entries {
            return Err(ComplianceError::AuditLogFull);
        }

        // Create Merkle node for this entry
        let leaf = MerkleNode::new_leaf(entry.merkle_hash);
        self.merkle_tree.push(leaf);

        self.audit_log.push(entry);
        Ok(())
    }

    /// Verify audit chain integrity
    pub fn verify_integrity(&self) -> bool {
        if self.merkle_tree.is_empty() {
            return true;
        }

        // Verify all leaf nodes
        for node in &self.merkle_tree {
            if !node.verify() {
                return false;
            }
        }

        true
    }

    /// Check which data should be purged
    pub fn check_retention(&self, age_days: u16) -> bool {
        self.retention_policy.should_purge(age_days)
    }

    pub fn audit_count(&self) -> usize {
        self.audit_log.len()
    }

    pub fn get_audit_entry(&self, entry_id: u64) -> Option<AuditEntry> {
        self.audit_log.iter().find(|e| e.entry_id == entry_id).copied()
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new(RetentionPolicy::default(), 10000)
    }
}

/// Compliance errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceError {
    AuditLogFull,
    IntegrityViolation,
    PolicyViolation,
    RetentionViolation,
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuditLogFull => write!(f, "audit log full"),
            Self::IntegrityViolation => write!(f, "integrity violation"),
            Self::PolicyViolation => write!(f, "policy violation"),
            Self::RetentionViolation => write!(f, "retention violation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_context() {
        let ctx = PolicyContext::new(1, 100, 5, 1000);
        assert_eq!(ctx.user_id, 1);
        assert_eq!(ctx.tool_id, 100);
    }

    #[test]
    fn test_merkle_node() {
        let leaf = MerkleNode::new_leaf(12345);
        assert!(leaf.is_leaf);
        assert!(leaf.verify());

        let parent = MerkleNode::new_parent(leaf.hash, leaf.hash);
        assert!(!parent.is_leaf);
        assert!(parent.verify());
    }

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::new(1, 1, 5, PolicyDecision::Allow);
        assert_eq!(entry.entry_id, 1);
        assert_eq!(entry.user_id, 1);
    }

    #[test]
    fn test_retention_policy() {
        let policy = RetentionPolicy::new(90);
        assert!(policy.is_retention_valid());
        assert!(!policy.should_purge(80));
        assert!(policy.should_purge(100));
    }

    #[test]
    fn test_compliance_engine() {
        let mut engine = ComplianceEngine::default();

        let entry = AuditEntry::new(1, 1, 5, PolicyDecision::Allow);
        assert!(engine.log_entry(entry).is_ok());

        assert_eq!(engine.audit_count(), 1);
        assert!(engine.verify_integrity());
    }

    #[test]
    fn test_audit_retrieval() {
        let mut engine = ComplianceEngine::default();

        let entry = AuditEntry::new(1, 1, 5, PolicyDecision::Deny);
        engine.log_entry(entry).unwrap();

        let retrieved = engine.get_audit_entry(1).unwrap();
        assert_eq!(retrieved.user_id, 1);
    }
}
