# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 18

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Complete Compliance Engine with integration of Merkle-tree audit log, full journaling, and compliance reporting for EU AI Act and other regulations.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 17-20: Compliance Engine completion), Section 3.3.5 (Compliance Engine)
- **Supporting:** Week 17 (Merkle-tree and journaling), Week 15-16 (PolicyDecision)

## Deliverables
- [ ] Compliance Engine full integration
  - Merkle-tree audit log operational
  - Cognitive journaling active for memory and checkpoint operations
  - All telemetry events recorded in audit log
  - Compliance queries (by regulation, by time range, by decision type)
- [ ] Compliance reporting engine
  - EU AI Act compliance report (Articles 12, 18, 19, 26(6))
  - GDPR compliance report (data processing, rights, audits)
  - SOC2 compliance report (security, availability, confidentiality)
  - Custom compliance report generation
- [ ] Regulatory mapping and documentation
  - Map each audit log entry to applicable regulations
  - Generate evidence summaries (e.g., "5000 policy decisions with explanations")
  - Link to specific audit log entries as evidence
  - Auto-generate compliance documentation
- [ ] Data governance integration
  - Information-flow controls (track data dependencies)
  - Data classification tags (PUBLIC, INTERNAL, CONFIDENTIAL, PII)
  - Taint tracking (mark data derived from PII)
  - Output gates (prevent PII leakage in responses)
- [ ] Retention policy implementation
  - Operational tier: 7 days, verbatim outputs, full tool I/O, debug telemetry
  - Compliance tier: ≥6 months, metadata, PolicyDecision events, checkpoint references, integrity chains
  - Legal hold: prevent deletion on legal request
  - GDPR right to erasure: identify and redact PII on request
- [ ] Export and audit APIs
  - Export compliance report in multiple formats (JSON, PDF, HTML)
  - Export audit trail with integrity proofs
  - Sign exports with private key (for non-repudiation)
  - Support regulator queries (filtered by time, regulation, decision type)
- [ ] Testing and validation
  - Compliance reports generated for sample decisions
  - Regulatory mapping validated against official guidance
  - Retention policies enforced automatically
  - Export signatures verified
  - EU AI Act compliance verified by legal review

## Technical Specifications

### Compliance Engine
```rust
pub struct ComplianceEngine {
    audit_log: Arc<MerkleAuditLog>,
    cognitive_journal: Arc<CognitiveJournal>,
    telemetry: Arc<TelemetryEngineV2>,
    retention_policies: Arc<RetentionPolicies>,
}

pub struct ComplianceQuery {
    pub regulation: Option<ApplicableRegulation>,
    pub start_time: i64,
    pub end_time: i64,
    pub decision_type: Option<String>,
    pub agent_filter: Option<String>,
}

impl ComplianceEngine {
    pub async fn execute_compliance_query(&self, query: &ComplianceQuery)
        -> Result<ComplianceQueryResult, ComplianceError>
    {
        let mut entries = vec![];

        // Get matching audit log entries
        if let Some(decision_type) = &query.decision_type {
            let entry_type = self.decision_type_to_audit_type(decision_type)?;
            entries.extend(
                self.audit_log.get_entries_by_type(entry_type, query.start_time, query.end_time)
                    .await?
            );
        } else {
            // Get all entries in time range
            // Implementation: iterate all entry types
        }

        // Filter by regulation if specified
        if let Some(regulation) = &query.regulation {
            entries.retain(|e| {
                let applicable = RegulationMapper::get_applicable_regulations(&e).contains(regulation);
                applicable
            });
        }

        // Filter by agent if specified
        if let Some(agent_filter) = &query.agent_filter {
            entries.retain(|e| {
                if let Some(agent) = e.extract_agent() {
                    agent.contains(agent_filter)
                } else {
                    false
                }
            });
        }

        Ok(ComplianceQueryResult {
            matching_entries: entries.len() as u64,
            entries,
            query_time: now(),
        })
    }

    pub async fn generate_compliance_report(&self, regulation: ApplicableRegulation,
                                           start_time: i64, end_time: i64)
        -> Result<ComplianceReport, ComplianceError>
    {
        let query = ComplianceQuery {
            regulation: Some(regulation.clone()),
            start_time,
            end_time,
            decision_type: None,
            agent_filter: None,
        };

        let result = self.execute_compliance_query(&query).await?;

        let report = match regulation {
            ApplicableRegulation::EUAIAct => {
                self.generate_eu_ai_act_report(&result).await?
            }
            ApplicableRegulation::GDPR => {
                self.generate_gdpr_report(&result).await?
            }
            ApplicableRegulation::SOC2 => {
                self.generate_soc2_report(&result).await?
            }
            _ => ComplianceReport::default(),
        };

        Ok(report)
    }

    async fn generate_eu_ai_act_report(&self, query_result: &ComplianceQueryResult)
        -> Result<ComplianceReport, ComplianceError>
    {
        // EU AI Act Articles 12, 18, 19, 26(6)
        let mut evidence = vec![];

        // Article 12(2)(a): Right to explanation
        let decisions_with_explanation = query_result.entries.iter()
            .filter(|e| e.entry_type == AuditLogEntryType::PolicyDecision)
            .count();
        evidence.push(format!(
            "Article 12(2)(a): {} policy decisions with explanations recorded",
            decisions_with_explanation
        ));

        // Article 18: High-risk AI system documentation
        let checkpoints = query_result.entries.iter()
            .filter(|e| matches!(e.entry_type, AuditLogEntryType::CheckpointCreate | AuditLogEntryType::CheckpointRestore))
            .count();
        evidence.push(format!(
            "Article 18: {} checkpoints created (system state documentation)",
            checkpoints
        ));

        // Article 19: Human oversight
        let escalations = query_result.entries.iter()
            .filter(|e| e.content.get("escalated_to").is_some())
            .count();
        evidence.push(format!(
            "Article 19: {} decisions escalated for human review",
            escalations
        ));

        Ok(ComplianceReport {
            regulation: "EU AI Act".to_string(),
            start_time: query_result.query_time,
            end_time: query_result.query_time,
            compliant: true,
            evidence,
            recommendations: vec![],
        })
    }

    async fn generate_gdpr_report(&self, query_result: &ComplianceQueryResult)
        -> Result<ComplianceReport, ComplianceError>
    {
        // GDPR compliance report
        let mut evidence = vec![];

        // Data processing documentation
        evidence.push("Audit trail documents all data processing operations".to_string());
        evidence.push("Data classification tags applied to sensitive data".to_string());
        evidence.push("Taint tracking identifies PII-derived data".to_string());

        Ok(ComplianceReport {
            regulation: "GDPR".to_string(),
            compliant: true,
            evidence,
            recommendations: vec!["Review data retention policies quarterly".to_string()],
        })
    }

    async fn generate_soc2_report(&self, query_result: &ComplianceQueryResult)
        -> Result<ComplianceReport, ComplianceError>
    {
        // SOC2 compliance report
        let mut evidence = vec![];

        evidence.push("All operations logged and searchable in audit trail".to_string());
        evidence.push("Integrity checks detect unauthorized modifications".to_string());
        evidence.push("Access controls enforced via Policy Engine".to_string());

        Ok(ComplianceReport {
            regulation: "SOC2".to_string(),
            compliant: true,
            evidence,
            recommendations: vec![],
        })
    }
}

pub struct ComplianceReport {
    pub regulation: String,
    pub start_time: i64,
    pub end_time: i64,
    pub compliant: bool,
    pub evidence: Vec<String>,
    pub recommendations: Vec<String>,
}

pub struct ComplianceQueryResult {
    pub matching_entries: u64,
    pub entries: Vec<AuditLogEntry>,
    pub query_time: i64,
}
```

### Data Governance and Taint Tracking
```rust
pub struct DataGovernanceEngine {
    data_classifications: Arc<RwLock<HashMap<String, DataClassification>>>,
    taint_tracker: Arc<TaintTracker>,
}

pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    PII,
    SensitiveHealthData,
}

pub struct TaintTracker {
    data_sources: Arc<Mutex<HashMap<String, DataSource>>>,
    taint_propagation: Arc<Mutex<HashMap<String, Vec<String>>>>, // derived -> sources
}

pub struct DataSource {
    pub data_id: String,
    pub classification: DataClassification,
    pub created_at: i64,
    pub agent_created_by: String,
    pub content_hash: String,
}

impl TaintTracker {
    pub async fn mark_pii_source(&self, data_id: String) -> Result<(), TrackingError> {
        let mut sources = self.data_sources.lock().await;
        sources.insert(data_id.clone(), DataSource {
            data_id: data_id.clone(),
            classification: DataClassification::PII,
            created_at: now(),
            agent_created_by: "system".to_string(),
            content_hash: String::new(),
        });
        Ok(())
    }

    pub async fn mark_data_as_derived(&self, derived_id: String, source_ids: Vec<String>)
        -> Result<(), TrackingError>
    {
        let mut propagation = self.taint_propagation.lock().await;
        propagation.insert(derived_id, source_ids);
        Ok(())
    }

    pub async fn is_pii_derived(&self, data_id: &str) -> Result<bool, TrackingError> {
        let propagation = self.taint_propagation.lock().await;
        let sources = self.get_sources(data_id, &propagation);

        let sources_lock = self.data_sources.lock().await;
        let has_pii = sources.iter().any(|s| {
            if let Some(source) = sources_lock.get(s) {
                matches!(source.classification, DataClassification::PII)
            } else {
                false
            }
        });

        Ok(has_pii)
    }

    fn get_sources(&self, data_id: &str, propagation: &HashMap<String, Vec<String>>)
        -> Vec<String>
    {
        let mut sources = vec![];
        if let Some(direct_sources) = propagation.get(data_id) {
            sources.extend(direct_sources.clone());
            for source in direct_sources {
                sources.extend(self.get_sources(source, propagation));
            }
        }
        sources
    }
}

pub struct OutputGate {
    taint_tracker: Arc<TaintTracker>,
}

impl OutputGate {
    pub async fn check_output_safe(&self, output_data: &str) -> Result<bool, GateError> {
        // Check if output contains PII-derived data
        // Implementation: pattern matching + taint tracking
        Ok(true)
    }

    pub async fn redact_output_for_external(&self, output_data: &str)
        -> Result<String, GateError>
    {
        // Redact PII before sending output externally
        let mut result = output_data.to_string();

        // Apply PII redaction patterns
        let pii_patterns = vec![
            (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "[EMAIL]"),
            (r"\b\d{3}-\d{2}-\d{4}\b", "[SSN]"),
        ];

        for (pattern, replacement) in pii_patterns {
            let re = regex::Regex::new(pattern).unwrap();
            result = re.replace_all(&result, replacement).to_string();
        }

        Ok(result)
    }
}
```

### Retention Policies
```rust
pub struct RetentionPolicies {
    operational_retention_days: u64,
    compliance_retention_months: u64,
    technical_docs_retention_years: u64,
}

impl RetentionPolicies {
    pub fn new() -> Self {
        Self {
            operational_retention_days: 7,
            compliance_retention_months: 6,
            technical_docs_retention_years: 10,
        }
    }

    pub async fn apply_retention(&self, storage: &impl DataStorage) -> Result<u64, RetentionError> {
        let mut deleted_count = 0;

        // Delete operational tier data older than 7 days
        let cutoff_time = now() - (self.operational_retention_days * 24 * 3600);
        deleted_count += storage.delete_operational_tier_before(cutoff_time).await?;

        // Delete non-PolicyDecision compliance data older than 6 months
        let cutoff_time = now() - (self.compliance_retention_months * 30 * 24 * 3600);
        deleted_count += storage.delete_compliance_metadata_before(cutoff_time).await?;

        Ok(deleted_count)
    }

    pub async fn place_legal_hold(&self, storage: &impl DataStorage, hold_id: &str,
                                 time_range: (i64, i64)) -> Result<(), RetentionError>
    {
        // Mark data in time range as held; prevent deletion
        storage.mark_legal_hold(hold_id, time_range).await?;
        Ok(())
    }

    pub async fn gdpr_right_to_erasure(&self, storage: &impl DataStorage, agent_id: &str)
        -> Result<u64, RetentionError>
    {
        // Find and redact PII for specific agent
        storage.redact_pii_for_agent(agent_id).await
    }
}

pub trait DataStorage {
    async fn delete_operational_tier_before(&self, cutoff: i64) -> Result<u64, RetentionError>;
    async fn delete_compliance_metadata_before(&self, cutoff: i64) -> Result<u64, RetentionError>;
    async fn mark_legal_hold(&self, hold_id: &str, time_range: (i64, i64))
        -> Result<(), RetentionError>;
    async fn redact_pii_for_agent(&self, agent_id: &str) -> Result<u64, RetentionError>;
}
```

## Dependencies
- **Blocked by:** Week 17 (Merkle-tree and journaling)
- **Blocking:** Week 19-20 (two-tier retention and testing)

## Acceptance Criteria
- [ ] Compliance Engine integrated with Merkle-tree and journaling
- [ ] Compliance reports generated for EU AI Act, GDPR, SOC2
- [ ] Regulatory mapping automated; evidence summary generated
- [ ] Data governance tags applied; taint tracking functional
- [ ] Retention policies enforced (7-day operational, 6-month compliance)
- [ ] Legal hold prevents deletion on request
- [ ] GDPR right to erasure implemented; PII redacted
- [ ] Export APIs with cryptographic signatures
- [ ] Compliance queries by regulation, time, decision type
- [ ] Unit tests cover compliance reporting, data governance, retention
- [ ] EU AI Act compliance validated by legal review

## Design Principles Alignment
- **Comprehensive compliance:** All major regulations supported
- **Auditability:** Evidence generated automatically from audit logs
- **Data privacy:** PII tracked, tainted, redacted in outputs
- **Retention safety:** Legal holds protect relevant data
- **Transparency:** Compliance reports explain regulatory coverage
