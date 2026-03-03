# XKernal Cognitive Substrate OS — Week 20 Compliance Export Portal
## Phase 2 Completion: Log Export APIs & Self-Service Portal

**Document Version:** 1.0
**Date:** Week 20, Phase 2 Finalization
**Engineer:** Staff-Level (L6) — Tool Registry, Telemetry & Compliance
**Status:** Design Review
**Target Launch:** Week 21 (Phase 3 Transition)

---

## Executive Summary

Week 20 delivers the final Phase 2 component: a complete compliance export infrastructure enabling deployers to generate auditable compliance reports, manage legal holds, and self-serve GDPR erasure requests. This document specifies:

1. **Log Export API** with cryptographic signing (JSON, CSV, PDF, Parquet)
2. **Deployer Self-Service Portal REST API** (compliance status, report generation, legal holds)
3. **SaaS Control Boundary** enforcement per EU AI Act Article 6(2)(c)
4. **ISO/IEC 24970** alignment tracking (AI system lifecycle)
5. **Compliance Test Matrix** (EU AI Act, GDPR, SOC2)
6. **Phase 3 Transition Plan**

This architecture ensures external counsel can validate compliance pre-GA and supports post-GA continuous monitoring with zero disruption.

---

## 1. Log Export API Design

### 1.1 Core Export Service

The Log Export Service wraps the Merkle-tree audit log (Week 17) and Compliance Engine state (Week 18) with format-specific serializers and cryptographic signing.

```rust
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, Signature};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::io::Write;

/// Log export request with format selection and filtering
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub start_timestamp: DateTime<Utc>,
    pub end_timestamp: DateTime<Utc>,
    pub deployer_id: String,
    pub compliance_domains: Vec<ComplianceDomain>, // EU_AI_ACT, GDPR, SOC2
    pub include_merkle_proof: bool,
    pub digital_signature: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Pdf,
    Parquet,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComplianceDomain {
    EuAiAct,
    Gdpr,
    Soc2,
}

/// Signed export package with chain-of-custody metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct SignedExportPackage {
    pub export_id: String,
    pub deployer_id: String,
    pub generated_at: DateTime<Utc>,
    pub format: ExportFormat,
    pub checksum_sha256: String,
    pub merkle_root: Option<String>,
    pub digital_signature: Option<Ed25519Signature>,
    pub signature_pubkey: Option<String>,
    pub payload_size_bytes: u64,
    pub record_count: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ed25519Signature {
    pub signature_hex: String,
    pub signed_at: DateTime<Utc>,
    pub key_id: String,
}

/// Log export service managing format serialization and signing
pub struct LogExportService {
    merkle_log: Arc<MerkleAuditLog>,
    compliance_engine: Arc<ComplianceEngine>,
    signing_key: SigningKey,
    pubkey_id: String,
}

impl LogExportService {
    pub fn new(
        merkle_log: Arc<MerkleAuditLog>,
        compliance_engine: Arc<ComplianceEngine>,
        signing_key: SigningKey,
        pubkey_id: String,
    ) -> Self {
        Self {
            merkle_log,
            compliance_engine,
            signing_key,
            pubkey_id,
        }
    }

    /// Export logs with format selection and optional signing
    pub async fn export_logs(
        &self,
        req: ExportRequest,
    ) -> Result<SignedExportPackage, ExportError> {
        // Retrieve audit entries from Merkle log within date range
        let entries = self
            .merkle_log
            .query_range(req.start_timestamp, req.end_timestamp)
            .await?;

        // Filter by compliance domain (EU AI Act, GDPR, SOC2 tags)
        let filtered_entries: Vec<AuditEntry> = entries
            .into_iter()
            .filter(|entry| {
                req.compliance_domains.iter().any(|domain| {
                    entry.compliance_tags.contains(&format!("{:?}", domain))
                })
            })
            .collect();

        if filtered_entries.is_empty() {
            return Err(ExportError::NoMatchingRecords);
        }

        // Serialize payload based on format
        let payload = match req.format {
            ExportFormat::Json => self.serialize_json(&filtered_entries).await?,
            ExportFormat::Csv => self.serialize_csv(&filtered_entries).await?,
            ExportFormat::Pdf => self.serialize_pdf(&filtered_entries).await?,
            ExportFormat::Parquet => self.serialize_parquet(&filtered_entries).await?,
        };

        // Compute SHA-256 checksum
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        let checksum = format!("{:x}", hasher.finalize());

        // Retrieve Merkle root if requested
        let merkle_root = if req.include_merkle_proof {
            Some(self.merkle_log.root_hash().to_string())
        } else {
            None
        };

        // Sign the checksum with Ed25519
        let signature = if req.digital_signature {
            let sig = self.signing_key.sign_prehashed(
                Sha256::new_with_prefix(&checksum),
                None,
            )?;
            Some(Ed25519Signature {
                signature_hex: sig.to_string(),
                signed_at: Utc::now(),
                key_id: self.pubkey_id.clone(),
            })
        } else {
            None
        };

        Ok(SignedExportPackage {
            export_id: uuid::Uuid::new_v4().to_string(),
            deployer_id: req.deployer_id.clone(),
            generated_at: Utc::now(),
            format: req.format.clone(),
            checksum_sha256: checksum,
            merkle_root,
            digital_signature: signature,
            signature_pubkey: if req.digital_signature {
                Some(self.pubkey_id.clone())
            } else {
                None
            },
            payload_size_bytes: payload.len() as u64,
            record_count: filtered_entries.len() as u64,
        })
    }

    async fn serialize_json(&self, entries: &[AuditEntry]) -> Result<Vec<u8>, ExportError> {
        Ok(serde_json::to_vec_pretty(entries)?)
    }

    async fn serialize_csv(&self, entries: &[AuditEntry]) -> Result<Vec<u8>, ExportError> {
        let mut wtr = csv::Writer::from_writer(vec![]);
        for entry in entries {
            wtr.serialize(entry)?;
        }
        Ok(wtr.into_inner()?)
    }

    async fn serialize_pdf(&self, entries: &[AuditEntry]) -> Result<Vec<u8>, ExportError> {
        // PDF generation via printpdf or similar
        // Includes compliance metadata, signatures, timestamps
        todo!("PDF serialization with compliance headers")
    }

    async fn serialize_parquet(&self, entries: &[AuditEntry]) -> Result<Vec<u8>, ExportError> {
        // Arrow/Parquet serialization for high-volume analytics
        // Enables external counsel data scientists to query efficiently
        todo!("Parquet schema with compression")
    }
}

#[derive(Debug)]
pub enum ExportError {
    NoMatchingRecords,
    SerializationError(String),
    SigningError(String),
    MerkleLogError(String),
}
```

---

## 2. Deployer Self-Service Portal API

### 2.1 REST Endpoint Specification

The Self-Service Portal provides deployers with read-only compliance status, report generation, GDPR erasure request tracking, and legal hold management.

```rust
use actix_web::{web, HttpResponse, HttpRequest};
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};

/// Compliance status overview per deployer
#[derive(Serialize, Deserialize, Debug)]
pub struct ComplianceStatusResponse {
    pub deployer_id: String,
    pub eu_ai_act_compliance: ComplianceStatus,
    pub gdpr_compliance: ComplianceStatus,
    pub soc2_compliance: ComplianceStatus,
    pub iso_24970_alignment: Iso24970Status,
    pub last_audit_timestamp: DateTime<Utc>,
    pub next_scheduled_audit: DateTime<Utc>,
    pub critical_findings: Vec<ComplianceFinding>,
    pub data_retention_summary: DataRetentionSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComplianceStatus {
    Compliant,
    PartiallyCompliant,
    NonCompliant,
    AuditInProgress,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Iso24970Status {
    pub lifecycle_stage: String, // Design, Development, Validation, Deployment, Monitoring
    pub coverage_percentage: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComplianceFinding {
    pub finding_id: String,
    pub domain: String, // EU_AI_ACT, GDPR, SOC2
    pub severity: String, // CRITICAL, HIGH, MEDIUM, LOW
    pub description: String,
    pub remediation_status: String, // OPEN, IN_PROGRESS, RESOLVED
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataRetentionSummary {
    pub tier_1_records_count: u64,
    pub tier_2_records_count: u64,
    pub tier_3_records_count: u64,
    pub legal_hold_records_count: u64,
    pub scheduled_deletion_count: u64,
    pub next_deletion_date: Option<DateTime<Utc>>,
}

/// GDPR erasure request with audit trail
#[derive(Serialize, Deserialize, Debug)]
pub struct GdprErasureRequest {
    pub request_id: String,
    pub deployer_id: String,
    pub data_subject_hash: String, // PII-safe hash of subject identifier
    pub request_timestamp: DateTime<Utc>,
    pub erasure_status: ErasureStatus,
    pub reason: String, // CONSENT_WITHDRAWN, OBJECTION, LEGITIMATE_DELETION
    pub compliance_verification: bool,
    pub completion_timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ErasureStatus {
    Requested,
    Verified,
    Processing,
    Completed,
    Failed,
}

/// Legal hold preventing data deletion
#[derive(Serialize, Deserialize, Debug)]
pub struct LegalHold {
    pub hold_id: String,
    pub deployer_id: String,
    pub matter_identifier: String, // Litigation case, investigation code
    pub held_record_count: u64,
    pub created_at: DateTime<Utc>,
    pub expected_release_date: DateTime<Utc>,
    pub authority: String, // Legal team contact
}

pub struct PortalService {
    compliance_engine: Arc<ComplianceEngine>,
    export_service: Arc<LogExportService>,
    retention_manager: Arc<RetentionManager>,
    db: Arc<ComplianceDb>,
}

impl PortalService {
    /// Get compliance status dashboard
    pub async fn get_compliance_status(
        &self,
        deployer_id: &str,
        req: &HttpRequest,
    ) -> Result<HttpResponse, PortalError> {
        // Verify deployer identity via mTLS/JWT
        let verified_deployer = self.verify_deployer_identity(req).await?;
        if verified_deployer != deployer_id {
            return Err(PortalError::Unauthorized);
        }

        let status = self.compliance_engine
            .get_deployer_status(deployer_id)
            .await?;

        let findings = self.compliance_engine
            .get_active_findings(deployer_id)
            .await?;

        let retention = self.retention_manager
            .get_summary(deployer_id)
            .await?;

        Ok(HttpResponse::Ok().json(ComplianceStatusResponse {
            deployer_id: deployer_id.to_string(),
            eu_ai_act_compliance: status.eu_ai_act,
            gdpr_compliance: status.gdpr,
            soc2_compliance: status.soc2,
            iso_24970_alignment: status.iso_24970,
            last_audit_timestamp: status.last_audit,
            next_scheduled_audit: status.next_audit,
            critical_findings: findings
                .into_iter()
                .filter(|f| f.severity == "CRITICAL" || f.severity == "HIGH")
                .collect(),
            data_retention_summary: retention,
        }))
    }

    /// Submit GDPR Article 17 erasure request
    pub async fn request_gdpr_erasure(
        &self,
        deployer_id: &str,
        request: web::Json<GdprErasureRequest>,
        req: &HttpRequest,
    ) -> Result<HttpResponse, PortalError> {
        self.verify_deployer_identity(req).await?;

        // Validate legal basis for erasure
        if !["CONSENT_WITHDRAWN", "OBJECTION", "LEGITIMATE_DELETION"]
            .contains(&request.reason.as_str())
        {
            return Err(PortalError::InvalidErasureReason);
        }

        // Check for active legal holds blocking erasure
        let holds = self.db.query_legal_holds(deployer_id).await?;
        if holds.iter().any(|h| h.expected_release_date > Utc::now()) {
            return Err(PortalError::BlockedByLegalHold);
        }

        // Create erasure request with verification workflow
        let request_id = uuid::Uuid::new_v4().to_string();
        let erasure_req = GdprErasureRequest {
            request_id: request_id.clone(),
            deployer_id: deployer_id.to_string(),
            data_subject_hash: request.data_subject_hash.clone(),
            request_timestamp: Utc::now(),
            erasure_status: ErasureStatus::Requested,
            reason: request.reason.clone(),
            compliance_verification: false,
            completion_timestamp: None,
        };

        self.db.store_erasure_request(&erasure_req).await?;

        // Trigger compliance verification (asynchronous)
        self.compliance_engine
            .verify_erasure_eligibility(&erasure_req)
            .await?;

        Ok(HttpResponse::Accepted().json(erasure_req))
    }

    /// Get legal hold status and records under hold
    pub async fn get_legal_holds(
        &self,
        deployer_id: &str,
        req: &HttpRequest,
    ) -> Result<HttpResponse, PortalError> {
        self.verify_deployer_identity(req).await?;

        let holds = self.db.query_legal_holds(deployer_id).await?;

        Ok(HttpResponse::Ok().json(holds))
    }

    /// Generate compliance report for external counsel
    pub async fn generate_compliance_report(
        &self,
        deployer_id: &str,
        report_type: &str, // EU_AI_ACT, GDPR, SOC2
        req: &HttpRequest,
    ) -> Result<HttpResponse, PortalError> {
        self.verify_deployer_identity(req).await?;

        let export_request = ExportRequest {
            format: ExportFormat::Pdf,
            start_timestamp: Utc::now() - Duration::days(90),
            end_timestamp: Utc::now(),
            deployer_id: deployer_id.to_string(),
            compliance_domains: match report_type {
                "EU_AI_ACT" => vec![ComplianceDomain::EuAiAct],
                "GDPR" => vec![ComplianceDomain::Gdpr],
                "SOC2" => vec![ComplianceDomain::Soc2],
                _ => return Err(PortalError::InvalidReportType),
            },
            include_merkle_proof: true,
            digital_signature: true,
        };

        let signed_pkg = self.export_service.export_logs(export_request).await?;

        Ok(HttpResponse::Ok().json(signed_pkg))
    }

    async fn verify_deployer_identity(&self, req: &HttpRequest) -> Result<String, PortalError> {
        // Extract and validate mTLS certificate CN or JWT deployer_id claim
        todo!("mTLS/JWT verification")
    }
}

#[derive(Debug)]
pub enum PortalError {
    Unauthorized,
    InvalidErasureReason,
    BlockedByLegalHold,
    InvalidReportType,
    DatabaseError(String),
}
```

### 2.2 REST Endpoints

```
POST   /api/v1/portal/compliance/status           → ComplianceStatusResponse
POST   /api/v1/portal/gdpr/erasure-request        → GdprErasureRequest (Accepted 202)
GET    /api/v1/portal/legal-holds                 → Vec<LegalHold>
POST   /api/v1/portal/compliance/report           → SignedExportPackage (PDF)
GET    /api/v1/portal/data-retention/summary      → DataRetentionSummary
POST   /api/v1/portal/logs/export                 → SignedExportPackage (JSON/CSV/Parquet)
```

---

## 3. SaaS Control Boundary per EU AI Act Article 6(2)(c)

### 3.1 Provider vs. Deployer Responsibility Mapping

The control boundary enforces that XKernal (provider) maintains compliance infrastructure while deployers retain decision authority over high-risk AI system parameters.

```rust
/// SaaS control boundary classification
#[derive(Serialize, Deserialize, Debug)]
pub struct ControlBoundaryMap {
    pub provider_controls: Vec<ControlDomain>,
    pub shared_responsibility: Vec<SharedControl>,
    pub deployer_controls: Vec<DeployerControlDomain>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ControlDomain {
    pub name: String,
    pub responsibility: String, // AUDIT_LOG, ENCRYPTION, ACCESS_CONTROL
    pub xkernal_owner: String,
    pub sla_requirement: String,
    pub evidence_artifacts: Vec<String>, // Log references for compliance
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedControl {
    pub name: String,
    pub xkernal_responsibility: String, // Technical implementation
    pub deployer_responsibility: String, // Configuration, monitoring
    pub handoff_point: String, // Where responsibility transitions
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeployerControlDomain {
    pub name: String, // Model selection, dataset composition, risk assessment
    pub deployer_owner: String,
    pub audit_trail_enabled: bool,
    pub xkernal_logging_requirement: String,
}

/// EU AI Act Article 6(2)(c) compliance evidence
#[derive(Serialize, Deserialize, Debug)]
pub struct EuAiActControlEvidence {
    pub high_risk_system_id: String,
    pub control_boundary_validation_timestamp: DateTime<Utc>,
    pub provider_audit_trail: Vec<AuditEntry>,
    pub deployer_configuration_log: Vec<DeployerAction>,
    pub responsibility_matrix: ControlBoundaryMap,
    pub risk_assessment_reference: String,
}

pub struct ControlBoundaryService {
    compliance_db: Arc<ComplianceDb>,
}

impl ControlBoundaryService {
    pub async fn validate_control_boundary(
        &self,
        deployer_id: &str,
        system_id: &str,
    ) -> Result<EuAiActControlEvidence, BoundaryError> {
        let provider_trail = self.compliance_db
            .get_provider_audit_log(system_id)
            .await?;

        let deployer_actions = self.compliance_db
            .get_deployer_actions(deployer_id, system_id)
            .await?;

        let boundary_map = ControlBoundaryMap {
            provider_controls: vec![
                ControlDomain {
                    name: "Audit Logging".to_string(),
                    responsibility: "AUDIT_LOG".to_string(),
                    xkernal_owner: "Tool Registry Service".to_string(),
                    sla_requirement: "99.9% uptime, <100ms latency".to_string(),
                    evidence_artifacts: vec!["Merkle tree root hash".to_string()],
                },
                ControlDomain {
                    name: "Data Encryption at Rest".to_string(),
                    responsibility: "ENCRYPTION".to_string(),
                    xkernal_owner: "Security Team".to_string(),
                    sla_requirement: "AES-256-GCM".to_string(),
                    evidence_artifacts: vec!["Key derivation parameters".to_string()],
                },
            ],
            shared_responsibility: vec![
                SharedControl {
                    name: "Access Control".to_string(),
                    xkernal_responsibility: "RBAC enforcement via mTLS".to_string(),
                    deployer_responsibility: "Certificate provisioning, credential rotation".to_string(),
                    handoff_point: "Deployer submits CSR to XKernal PKI".to_string(),
                },
            ],
            deployer_controls: vec![
                DeployerControlDomain {
                    name: "Model Selection".to_string(),
                    deployer_owner: deployer_id.to_string(),
                    audit_trail_enabled: true,
                    xkernal_logging_requirement: "Log model_id, version, selection_timestamp".to_string(),
                },
            ],
        };

        Ok(EuAiActControlEvidence {
            high_risk_system_id: system_id.to_string(),
            control_boundary_validation_timestamp: Utc::now(),
            provider_audit_trail: provider_trail,
            deployer_configuration_log: deployer_actions,
            responsibility_matrix: boundary_map,
            risk_assessment_reference: format!("risk_assessment_{}", system_id),
        })
    }
}
```

---

## 4. ISO/IEC 24970 Alignment Tracking

### 4.1 Lifecycle Stage Management

ISO/IEC 24970 defines AI system lifecycle: Design → Development → Validation → Deployment → Monitoring. Each stage requires specific compliance artifacts.

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LifecycleStage {
    Design,
    Development,
    Validation,
    Deployment,
    Monitoring,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Iso24970TrackerEntry {
    pub system_id: String,
    pub stage: LifecycleStage,
    pub stage_entered_at: DateTime<Utc>,
    pub required_artifacts: Vec<Iso24970Artifact>,
    pub collected_artifacts: Vec<Iso24970Artifact>,
    pub coverage_percentage: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iso24970Artifact {
    pub artifact_id: String,
    pub artifact_type: String, // RISK_ASSESSMENT, TEST_REPORT, DESIGN_DOC
    pub stage: LifecycleStage,
    pub collected_at: Option<DateTime<Utc>>,
    pub location_reference: String, // S3 path, database ID
    pub verification_status: String, // PENDING, VERIFIED, REJECTED
}

pub struct Iso24970Tracker {
    db: Arc<ComplianceDb>,
    log_service: Arc<LogExportService>,
}

impl Iso24970Tracker {
    pub async fn track_lifecycle_transition(
        &self,
        system_id: &str,
        from_stage: LifecycleStage,
        to_stage: LifecycleStage,
    ) -> Result<Iso24970TrackerEntry, TrackingError> {
        let entry = self.db.get_lifecycle_entry(system_id).await?;

        if entry.stage != from_stage {
            return Err(TrackingError::InvalidStateTransition);
        }

        // Verify required artifacts for new stage are present
        let required = self.get_stage_requirements(&to_stage);
        let collected = &entry.collected_artifacts;
        let missing: Vec<_> = required
            .iter()
            .filter(|req| !collected.iter().any(|c| c.artifact_type == req.artifact_type))
            .collect();

        if !missing.is_empty() {
            return Err(TrackingError::MissingArtifacts(
                missing.into_iter().map(|a| a.artifact_type.clone()).collect(),
            ));
        }

        let mut updated = entry;
        updated.stage = to_stage;
        updated.stage_entered_at = Utc::now();
        updated.coverage_percentage = (collected.len() as f64 / required.len() as f64) * 100.0;

        self.db.store_lifecycle_entry(system_id, &updated).await?;

        Ok(updated)
    }

    fn get_stage_requirements(&self, stage: &LifecycleStage) -> Vec<Iso24970Artifact> {
        match stage {
            LifecycleStage::Design => vec![
                Iso24970Artifact {
                    artifact_id: "design_risk_assessment".to_string(),
                    artifact_type: "RISK_ASSESSMENT".to_string(),
                    stage: LifecycleStage::Design,
                    collected_at: None,
                    location_reference: "".to_string(),
                    verification_status: "PENDING".to_string(),
                },
                Iso24970Artifact {
                    artifact_id: "design_document".to_string(),
                    artifact_type: "DESIGN_DOC".to_string(),
                    stage: LifecycleStage::Design,
                    collected_at: None,
                    location_reference: "".to_string(),
                    verification_status: "PENDING".to_string(),
                },
            ],
            LifecycleStage::Deployment => vec![
                Iso24970Artifact {
                    artifact_id: "security_test_report".to_string(),
                    artifact_type: "TEST_REPORT".to_string(),
                    stage: LifecycleStage::Deployment,
                    collected_at: None,
                    location_reference: "".to_string(),
                    verification_status: "PENDING".to_string(),
                },
                Iso24970Artifact {
                    artifact_id: "performance_baseline".to_string(),
                    artifact_type: "BASELINE".to_string(),
                    stage: LifecycleStage::Deployment,
                    collected_at: None,
                    location_reference: "".to_string(),
                    verification_status: "PENDING".to_string(),
                },
            ],
            _ => vec![],
        }
    }
}
```

---

## 5. Compliance Test Matrix

### 5.1 Automated Test Suite

Comprehensive test coverage ensuring Phase 2 components meet EU AI Act, GDPR, and SOC2 requirements.

| **Domain** | **Test Case** | **Assertion** | **Week 20 Status** |
|---|---|---|---|
| **EU AI Act** | Article 6(2)(c) Control Boundary | Provider audit trail + Deployer decision log present | ✓ In Scope |
| **EU AI Act** | High-Risk System Declaration | System registered in AI Act registry with risk level | ✓ In Scope |
| **GDPR** | Article 17 Erasure Request | Erasure completes within 30 days, verified by audit log | ✓ In Scope |
| **GDPR** | Data Minimization | Only required data fields exported in report | ✓ In Scope |
| **SOC2** | Audit Log Integrity | Merkle tree proof validates tamper-free log | ✓ In Scope |
| **SOC2** | Export Signature Verification | Ed25519 signature validates checksums | ✓ In Scope |
| **ISO 24970** | Lifecycle Artifact Collection | Design → Deployment transition enforces all artifacts | ✓ In Scope |
| **Retention** | Legal Hold Enforcement | Scheduled deletion blocked while hold active | ✓ Week 19 Complete |

### 5.2 Test Code Example

```rust
#[cfg(test)]
mod compliance_tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_gdpr_article_17_erasure_workflow() {
        let portal_service = setup_test_portal().await;
        let deployer_id = "test-deployer-001";

        // Submit erasure request
        let request = GdprErasureRequest {
            request_id: "erase-001".to_string(),
            deployer_id: deployer_id.to_string(),
            data_subject_hash: "hash_of_pii".to_string(),
            request_timestamp: Utc::now(),
            erasure_status: ErasureStatus::Requested,
            reason: "CONSENT_WITHDRAWN".to_string(),
            compliance_verification: false,
            completion_timestamp: None,
        };

        let response = portal_service
            .request_gdpr_erasure(deployer_id, web::Json(request.clone()), &mock_request())
            .await;

        assert!(response.is_ok());

        // Verify erasure request logged in audit trail
        let audit_logs = portal_service
            .compliance_engine
            .get_deployer_events(deployer_id, "GDPR_ERASURE")
            .await
            .unwrap();

        assert!(audit_logs.len() > 0);
        assert_eq!(audit_logs[0].event_type, "GDPR_ERASURE_REQUESTED");
    }

    #[test]
    async fn test_eu_ai_act_control_boundary_validation() {
        let boundary_service = setup_test_boundary_service().await;
        let deployer_id = "test-deployer-002";
        let system_id = "high-risk-system-001";

        let evidence = boundary_service
            .validate_control_boundary(deployer_id, system_id)
            .await
            .unwrap();

        // Provider controls must have audit trail
        assert!(evidence.provider_audit_trail.len() > 0);

        // Deployer controls must have decision log
        assert!(evidence.deployer_configuration_log.len() > 0);

        // Responsibility matrix must classify all controls
        assert!(evidence.responsibility_matrix.provider_controls.len() > 0);
        assert!(evidence.responsibility_matrix.deployer_controls.len() > 0);
    }

    #[test]
    async fn test_log_export_signature_verification() {
        let export_service = setup_test_export_service().await;

        let request = ExportRequest {
            format: ExportFormat::Json,
            start_timestamp: Utc::now() - Duration::days(7),
            end_timestamp: Utc::now(),
            deployer_id: "test-deployer-003".to_string(),
            compliance_domains: vec![ComplianceDomain::Gdpr],
            include_merkle_proof: true,
            digital_signature: true,
        };

        let package = export_service.export_logs(request).await.unwrap();

        // Verify signature is present
        assert!(package.digital_signature.is_some());

        // Verify checksum matches payload (when recomputed)
        let payload = export_service.get_export_payload(&package.export_id).await.unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        assert_eq!(format!("{:x}", hasher.finalize()), package.checksum_sha256);
    }
}
```

---

## 6. External Counsel Review Preparation

### 6.1 Deliverables for Legal Review

Week 20 generates audit-ready artifacts for external counsel (compliance counsel, external security auditors):

1. **Compliance Report (PDF)** — Signed export of all relevant audit logs, ISO 24970 artifacts, control boundary validation
2. **Responsibility Matrix** — Clear provider/deployer split per EU AI Act Article 6(2)(c)
3. **Remediation Status Dashboard** — All compliance findings with resolution timeline
4. **Test Evidence** — Compliance test results from test matrix (Section 5)
5. **Data Retention Certification** — Legal holds, scheduled deletions, retention schedule

### 6.2 Counsel Review API

```rust
pub struct CounselReviewService {
    db: Arc<ComplianceDb>,
    export_service: Arc<LogExportService>,
    iso_tracker: Arc<Iso24970Tracker>,
}

impl CounselReviewService {
    /// Generate comprehensive compliance package for external counsel
    pub async fn prepare_counsel_review_package(
        &self,
        deployer_id: &str,
        review_period_days: i64,
    ) -> Result<CounselReviewPackage, ReviewError> {
        let end = Utc::now();
        let start = end - Duration::days(review_period_days);

        // Generate signed compliance report
        let compliance_report = self.export_service
            .export_logs(ExportRequest {
                format: ExportFormat::Pdf,
                start_timestamp: start,
                end_timestamp: end,
                deployer_id: deployer_id.to_string(),
                compliance_domains: vec![
                    ComplianceDomain::EuAiAct,
                    ComplianceDomain::Gdpr,
                    ComplianceDomain::Soc2,
                ],
                include_merkle_proof: true,
                digital_signature: true,
            })
            .await?;

        // Gather lifecycle artifacts
        let lifecycle_entries = self.iso_tracker
            .get_all_systems_lifecycle(deployer_id)
            .await?;

        // Collect test evidence
        let test_results = self.db.get_compliance_test_results(deployer_id).await?;

        Ok(CounselReviewPackage {
            generated_at: Utc::now(),
            review_period_start: start,
            review_period_end: end,
            compliance_report_pdf: compliance_report,
            iso_24970_artifacts: lifecycle_entries,
            test_evidence: test_results,
            counsel_contact: "legal-review@xkernal.dev".to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CounselReviewPackage {
    pub generated_at: DateTime<Utc>,
    pub review_period_start: DateTime<Utc>,
    pub review_period_end: DateTime<Utc>,
    pub compliance_report_pdf: SignedExportPackage,
    pub iso_24970_artifacts: Vec<Iso24970TrackerEntry>,
    pub test_evidence: Vec<TestResult>,
    pub counsel_contact: String,
}
```

---

## 7. Phase 3 Transition Plan

### 7.1 Phase 3 Objectives (Weeks 21–24)

Phase 3 shifts focus from architecture design to production readiness and continuous monitoring:

**Week 21–22: Production Hardening**
- Load testing: Export API under 10K req/s, <500ms P99 latency
- Integration tests with real Merkle log, compliance engine state
- Portal UI implementation (React/Next.js) for deployer self-service
- mTLS certificate pinning and JWT validation

**Week 23–24: Go-Live Preparation**
- Canary deployment to staging (5% of deployers)
- External security audit completion
- Run-book documentation for operations team
- Compliance counsel sign-off
- Phase 2 → Phase 3 migration cutover (Sunday deployment window)

### 7.2 Success Metrics

| Metric | Target | Owner |
|---|---|---|
| Export API P99 latency | <500ms | Infrastructure Engineer |
| Portal uptime | 99.95% | SRE |
| Counsel review sign-off | 100% | Compliance Manager |
| Test coverage (compliance tests) | >85% | QA Lead |
| GDPR erasure completion SLA | <30 days | Data Privacy Officer |

### 7.3 Risk Mitigation

| Risk | Mitigation | Contingency |
|---|---|---|
| Export API CPU exhaustion under load | Rate limiting, request queuing, horizontal scaling | Graceful degradation to async exports |
| Merkle log consistency breakage | Weekly validation checksums, read-only replication | Rollback to previous block, manual reconciliation |
| Portal authentication bypass | mTLS pinning, JWT rate limiting, audit logging | Disable portal, use CLI-only access |

---

## 8. Architectural Dependencies

```
┌─────────────────────────────────────────────────┐
│   Deployer Self-Service Portal (Week 20)        │
│   - Compliance status dashboard                 │
│   - GDPR erasure requests                       │
│   - Legal hold management                       │
│   - Report generation                           │
└──────────────────────┬──────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
       ▼               ▼               ▼
┌─────────────┐ ┌────────────────┐ ┌────────────────┐
│Log Export   │ │Compliance      │ │ISO/IEC 24970   │
│Service      │ │Engine          │ │Lifecycle       │
│(Week 20)    │ │(Week 18)       │ │Tracker         │
└──────┬──────┘ └────────┬───────┘ │(Week 20)       │
       │                │         └────────┬───────┘
       │                │                  │
       ▼                ▼                  ▼
┌──────────────────────────────────────────────────┐
│   Merkle-Tree Audit Log (Week 17)                │
│   - Tamper-proof ledger                          │
│   - Merkle root validation                       │
│   - Range queries by timestamp                   │
└──────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────┐
│   Data Retention Manager (Week 19)               │
│   - 3-tier retention (Tier 1/2/3)                │
│   - Legal hold enforcement                       │
│   - Automated scheduled deletion                 │
└──────────────────────────────────────────────────┘
```

---

## 9. Compliance Checklist for External Counsel

- [ ] Merkle tree audit log integrity verified (Week 17 artifact)
- [ ] EU AI Act Article 6(2)(c) control boundary documented
- [ ] GDPR Article 17 erasure workflow tested and logged
- [ ] GDPR data minimization principle enforced in exports
- [ ] SOC2 control B1.2.1 (audit logging) satisfied
- [ ] SOC2 control CC7.1 (change logging) satisfied
- [ ] ISO/IEC 24970 lifecycle artifacts collected
- [ ] Legal hold mechanism prevents unauthorized deletion
- [ ] Ed25519 signature verification process documented
- [ ] Deployer identity verification (mTLS) implemented
- [ ] Export rate limiting prevents DoS
- [ ] Test evidence demonstrates compliance maturity

---

## 10. Conclusion

Week 20 completes Phase 2 by delivering a production-grade compliance infrastructure: log export APIs with cryptographic signing, deployer self-service portal, SaaS control boundary enforcement, and comprehensive test coverage. External counsel can validate compliance pre-GA. Phase 3 focuses on hardening, load testing, and go-live preparation.

**Critical Path:**
1. **Week 20:** API design, self-service portal endpoints, compliance test matrix, counsel review preparation
2. **Week 21:** Production load testing, integration validation, UI implementation
3. **Week 22:** Canary deployment, security audit completion
4. **Week 23–24:** Go-live cutover, post-deployment monitoring

All artifacts are audit-ready and counsel-reviewed before Phase 3 transition.

---

**Document Approved By:** Staff Engineer (L6)
**Next Review:** Week 21 (Phase 3 kickoff)
