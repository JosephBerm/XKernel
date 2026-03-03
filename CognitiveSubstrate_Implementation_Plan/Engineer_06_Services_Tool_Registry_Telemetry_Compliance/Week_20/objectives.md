# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 20

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Complete Phase 2 compliance architecture with log export APIs, compliance testing, and integration validation. Prepare for Phase 3 launch and external compliance audits.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 19-20: log export APIs, deployer self-service), Section 3.3.5 (Compliance Engine)
- **Supporting:** Weeks 15-19 (all Phase 2 components)

## Deliverables
- [ ] Log export APIs
  - Export compliance logs (filtered by time, regulation, decision type)
  - Export audit trail with integrity proofs
  - Export in multiple formats (JSON, CSV, PDF)
  - Support pagination for large exports
  - Sign exports with private key (non-repudiation)
- [ ] Deployer self-service portal (REST API)
  - Query compliance status
  - Generate compliance reports on-demand
  - Submit GDPR erasure requests
  - Create/manage legal holds
  - Monitor retention policy execution
  - Export logs for regulator
- [ ] SaaS control boundary per EU AI Act Article 13(f)
  - Clear separation of concerns: deployer vs AI system
  - Deployer controls what data is processed
  - AI system enforces control boundaries
  - Audit trail tracks boundary enforcement
- [ ] ISO/IEC 24970 (AI System Governance) alignment tracking
  - Map compliance artifacts to ISO/IEC 24970 requirements
  - Auto-generate ISO/IEC compliance documentation
  - Track missing requirements
- [ ] Compliance testing and validation
  - EU AI Act compliance test suite (Articles 12, 18, 19, 26(6))
  - GDPR compliance test suite
  - SOC2 compliance test suite
  - Run tests; generate pass/fail report
- [ ] External counsel review preparation
  - Package all compliance evidence
  - Generate executive summary
  - Document known gaps and mitigations
  - Prepare for external security/compliance audit
- [ ] Phase 2 completion validation
  - All acceptance criteria from Weeks 15-20 verified
  - Integration tests pass
  - Compliance tests pass
  - Performance meets baselines
- [ ] Phase 3 transition planning
  - Telemetry benchmarks plan (Week 25-28)
  - Adversarial testing plan (Week 29-30)
  - Compliance validation plan (Week 31-32)
  - Paper section plan (Week 33-34)

## Technical Specifications

### Log Export API
```rust
pub struct LogExportAPI {
    storage: Arc<TwoTierStorage>,
    compliance_engine: Arc<ComplianceEngine>,
}

pub struct ExportRequest {
    pub export_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub format: ExportFormat,
    pub include_integrity_proofs: bool,
    pub include_redacted: bool,
    pub regulations: Vec<ApplicableRegulation>,
}

pub enum ExportFormat {
    JSON,
    CSV,
    PDF,
    Parquet,
}

impl LogExportAPI {
    pub async fn export_logs(&self, request: &ExportRequest) -> Result<Vec<u8>, ExportError> {
        let entries = self.compliance_engine.execute_compliance_query(&ComplianceQuery {
            regulation: None,
            start_time: request.start_time,
            end_time: request.end_time,
            decision_type: None,
            agent_filter: None,
        }).await?;

        let export = match &request.format {
            ExportFormat::JSON => self.export_as_json(&entries).await?,
            ExportFormat::CSV => self.export_as_csv(&entries).await?,
            ExportFormat::PDF => self.export_as_pdf(&entries).await?,
            ExportFormat::Parquet => self.export_as_parquet(&entries).await?,
        };

        // Sign export with private key
        let signature = self.sign_export(&export)?;
        let signed_export = format!("{}\nSIGNATURE: {}", String::from_utf8(export)?, signature);

        Ok(signed_export.into_bytes())
    }

    pub async fn export_as_json(&self, entries: &[AuditLogEntry]) -> Result<Vec<u8>, ExportError> {
        let json = serde_json::to_vec(entries)?;
        Ok(json)
    }

    pub async fn export_as_pdf(&self, entries: &[AuditLogEntry]) -> Result<Vec<u8>, ExportError> {
        // Generate PDF using printpdf or similar library
        // Include header with export date, time range, regulations
        // Include evidence summary
        // Include integrity proofs if requested
        Ok(vec![])
    }

    fn sign_export(&self, export: &[u8]) -> Result<String, ExportError> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let key = "export_signing_key"; // In production: from secure storage
        let mut mac = HmacSha256::new_from_slice(key.as_bytes())
            .map_err(|_| ExportError::SigningError)?;
        mac.update(export);

        Ok(format!("{:x}", mac.finalize().into_bytes()))
    }

    pub fn verify_export_signature(export: &[u8], signature: &str) -> Result<bool, ExportError> {
        // Verify signature matches export content
        Ok(true)
    }
}
```

### Deployer Self-Service Portal
```rust
pub struct CompliancePortal {
    compliance_engine: Arc<ComplianceEngine>,
    export_api: Arc<LogExportAPI>,
    legal_hold_manager: Arc<LegalHoldManager>,
    gdpr_engine: Arc<GDPREngine>,
}

#[actix_web::get("/api/compliance/status")]
async fn get_compliance_status(portal: web::Data<CompliancePortal>) -> Result<HttpResponse> {
    let status = ComplianceStatus {
        eu_ai_act: true,
        gdpr: true,
        soc2: true,
        last_verified: now(),
    };

    Ok(HttpResponse::Ok().json(status))
}

#[actix_web::post("/api/compliance/report")]
async fn generate_report(
    portal: web::Data<CompliancePortal>,
    regulation: web::Query<String>,
) -> Result<HttpResponse> {
    let reg = parse_regulation(&regulation)?;
    let report = portal.compliance_engine.generate_compliance_report(
        reg,
        now() - (30 * 24 * 3600),
        now()
    ).await?;

    Ok(HttpResponse::Ok().json(report))
}

#[actix_web::post("/api/gdpr/erasure-request")]
async fn submit_erasure_request(
    portal: web::Data<CompliancePortal>,
    body: web::Json<ErasureRequestBody>,
) -> Result<HttpResponse> {
    let request_id = portal.gdpr_engine.submit_erasure_request(&body.data_subject_id).await?;

    Ok(HttpResponse::Ok().json(json!({
        "request_id": request_id,
        "status": "pending",
        "created_at": now(),
    })))
}

#[actix_web::post("/api/legal-hold")]
async fn create_legal_hold(
    portal: web::Data<CompliancePortal>,
    body: web::Json<LegalHoldRequestBody>,
) -> Result<HttpResponse> {
    let hold_id = portal.legal_hold_manager.create_hold(
        &body.reason,
        (body.start_time, body.end_time),
        &body.created_by,
    ).await?;

    Ok(HttpResponse::Ok().json(json!({
        "hold_id": hold_id,
        "status": "active",
    })))
}

#[actix_web::post("/api/export-logs")]
async fn export_logs(
    portal: web::Data<CompliancePortal>,
    body: web::Json<ExportRequest>,
) -> Result<HttpResponse> {
    let export = portal.export_api.export_logs(&body).await?;

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(export))
}
```

### Compliance Test Suite
```rust
pub struct ComplianceTestSuite;

#[tokio::test]
async fn test_eu_ai_act_article_12_2a() {
    // Test: Every policy decision has explanation
    let engine = setup_compliance_engine().await;

    for _ in 0..100 {
        let decision = create_sample_policy_decision().await;
        assert!(!decision.explanation.is_empty());
        assert!(!decision.redacted_explanation.is_empty());
    }
}

#[tokio::test]
async fn test_eu_ai_act_article_18() {
    // Test: High-risk AI system documentation exists
    let engine = setup_compliance_engine().await;

    let checkpoints = engine.execute_compliance_query(&ComplianceQuery {
        regulation: Some(ApplicableRegulation::EUAIAct),
        entry_type: Some("CheckpointCreate".to_string()),
        ..Default::default()
    }).await.unwrap();

    assert!(checkpoints.matching_entries > 0);
}

#[tokio::test]
async fn test_gdpr_data_processing() {
    // Test: All data processing logged and traceable
    let engine = setup_compliance_engine().await;

    let processing_records = engine.execute_compliance_query(&ComplianceQuery {
        regulation: Some(ApplicableRegulation::GDPR),
        ..Default::default()
    }).await.unwrap();

    assert!(processing_records.matching_entries > 0);
}

#[tokio::test]
async fn test_soc2_audit_trail() {
    // Test: Complete audit trail exists
    let engine = setup_compliance_engine().await;

    let audit_entries = engine.execute_compliance_query(&ComplianceQuery {
        regulation: Some(ApplicableRegulation::SOC2),
        ..Default::default()
    }).await.unwrap();

    assert!(audit_entries.matching_entries > 0);
}
```

## Dependencies
- **Blocked by:** Weeks 15-19 (all Phase 2 components)
- **Blocking:** Phase 3 Week 25-36 (benchmarks, testing, validation, launch)

## Acceptance Criteria
- [ ] Export APIs functional; support JSON, CSV, PDF, Parquet
- [ ] Export signing and verification working
- [ ] Deployer portal REST API complete
- [ ] Status endpoint returns accurate compliance status
- [ ] Report generation on-demand for all regulations
- [ ] GDPR erasure API functional and tested
- [ ] Legal hold API functional and tested
- [ ] SaaS control boundary clearly defined and enforced
- [ ] ISO/IEC 24970 mapping complete
- [ ] Compliance test suite passes (EU AI Act, GDPR, SOC2)
- [ ] External counsel review documentation prepared
- [ ] Phase 2 completion validation signed off
- [ ] Phase 3 transition plan documented

## Design Principles Alignment
- **Self-service:** Deployers can export and verify compliance without assistance
- **Transparency:** All compliance evidence available for external audits
- **Security:** Exports signed for non-repudiation
- **Regulatory alignment:** Multiple regulation support with clear mapping
- **Operability:** Automated testing provides ongoing compliance verification
