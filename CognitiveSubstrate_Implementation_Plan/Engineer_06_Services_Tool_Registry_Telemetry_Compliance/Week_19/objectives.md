# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 19

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Implement two-tier data retention (Operational: 7 days; Compliance: ≥6 months) with automatic enforcement, legal holds, and GDPR compliance.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 19-20: Two-tier retention, GDPR, data governance), Section 3.3.5 (two-tier retention metadata, compliance)
- **Supporting:** Week 17-18 (Compliance Engine, journaling)

## Deliverables
- [ ] Two-tier storage backend
  - Operational tier: fast SSD storage, 7-day retention, automatic cleanup
  - Compliance tier: slower archive storage, ≥6 months, immutable (append-only)
  - Technical docs tier: ultra-long-term (10 years), audit trail only
- [ ] Data movement and tiering
  - Operational events auto-move to compliance tier after 7 days
  - Compression during move (gzip or zstd)
  - Verification checksum after move
  - Retention time logged and tracked
- [ ] Legal hold system
  - Mark data range as held; prevent deletion
  - Multiple holds can apply to same data
  - Export hold registry
  - Automatic notification when hold expires
- [ ] GDPR right to erasure
  - Identify all PII data for specific data subject
  - Redact PII in-place; mark as erased
  - Generate erasure certificate
  - Verify no PII remains (scan operation)
- [ ] Automatic retention enforcement
  - Daily job: move operational data to compliance tier
  - Weekly job: verify integrity of compliance tier
  - Monthly job: verify no unintended deletions
  - Alert on any retention policy violations
- [ ] Metadata tier
  - Store only metadata (size, hash, timestamps) for archived data
  - Metadata remains on fast storage for quick queries
  - Content stored on slower archive
  - Restore from archive on-demand
- [ ] Testing and validation
  - Data correctly moves between tiers
  - Integrity maintained during archival
  - Legal holds prevent deletion
  - GDPR erasure works correctly
  - Retention policies enforced automatically

## Technical Specifications

### Two-Tier Storage Backend
```rust
pub struct TwoTierStorage {
    operational_backend: Arc<OperationalTier>,
    compliance_backend: Arc<ComplianceTier>,
    archive_backend: Arc<ArchiveTier>,
}

pub struct OperationalTier {
    storage_path: PathBuf,
    retention_days: u64,
    max_size_gb: u64,
}

pub struct ComplianceTier {
    storage_path: PathBuf,
    retention_months: u64,
    immutable: bool,
}

pub struct ArchiveTier {
    storage_path: PathBuf,
    retention_years: u64,
}

impl TwoTierStorage {
    pub async fn write_event(&self, event: &CEFEvent, tier: StorageTier)
        -> Result<String, StorageError>
    {
        let event_id = &event.event_id;
        match tier {
            StorageTier::Operational => {
                self.operational_backend.write(event_id, event).await?;
            }
            StorageTier::Compliance => {
                self.compliance_backend.write(event_id, event).await?;
            }
            StorageTier::Archive => {
                self.archive_backend.write(event_id, event).await?;
            }
        }
        Ok(event_id.clone())
    }

    pub async fn move_to_compliance_tier(&self) -> Result<u64, StorageError> {
        let now = now();
        let cutoff = now - (7 * 24 * 3600);

        let events = self.operational_backend.get_before(cutoff).await?;
        let mut moved = 0;

        for event in events {
            // Compress
            let compressed = self.compress_event(&event)?;

            // Move to compliance
            self.compliance_backend.write(&event.event_id, &event).await?;

            // Verify
            let checksum_original = self.compute_checksum(&event)?;
            let checksum_moved = self.compliance_backend.get_checksum(&event.event_id).await?;
            if checksum_original == checksum_moved {
                // Delete from operational
                self.operational_backend.delete(&event.event_id).await.ok();
                moved += 1;
            }
        }

        Ok(moved)
    }

    fn compress_event(&self, event: &CEFEvent) -> Result<Vec<u8>, StorageError> {
        use flate2::write::GzEncoder;
        use std::io::Write;

        let json = serde_json::to_vec(event)?;
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&json)?;
        encoder.finish().map_err(|e| StorageError::CompressionError(e.to_string()))
    }

    fn compute_checksum(&self, event: &CEFEvent) -> Result<String, StorageError> {
        use sha2::{Sha256, Digest};
        let json = serde_json::to_vec(event)?;
        let mut hasher = Sha256::new();
        hasher.update(&json);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

pub enum StorageTier {
    Operational,
    Compliance,
    Archive,
}
```

### Legal Hold System
```rust
pub struct LegalHoldManager {
    holds: Arc<Mutex<HashMap<String, LegalHold>>>,
}

pub struct LegalHold {
    pub hold_id: String,
    pub created_at: i64,
    pub created_by: String,
    pub reason: String,
    pub time_range: (i64, i64),
    pub expires_at: Option<i64>,
    pub data_identifiers: Vec<String>,
}

impl LegalHoldManager {
    pub async fn create_hold(&self, reason: &str, time_range: (i64, i64),
                            created_by: &str) -> Result<String, HoldError>
    {
        let hold_id = uuid::Uuid::new_v4().to_string();
        let hold = LegalHold {
            hold_id: hold_id.clone(),
            created_at: now(),
            created_by: created_by.to_string(),
            reason: reason.to_string(),
            time_range,
            expires_at: Some(now() + (30 * 24 * 3600)), // 30-day default
            data_identifiers: vec![],
        };

        self.holds.lock().await.insert(hold_id.clone(), hold);
        Ok(hold_id)
    }

    pub async fn is_data_held(&self, data_id: &str) -> Result<bool, HoldError> {
        let holds = self.holds.lock().await;
        Ok(holds.values().any(|h| h.data_identifiers.contains(&data_id.to_string())))
    }

    pub async fn release_expired_holds(&self) -> Result<u32, HoldError> {
        let mut holds = self.holds.lock().await;
        let now = now();
        let expired_count = holds.values().filter(|h| {
            h.expires_at.map(|exp| exp < now).unwrap_or(false)
        }).count();

        holds.retain(|_, h| !h.expires_at.map(|exp| exp < now).unwrap_or(false));
        Ok(expired_count as u32)
    }
}
```

### GDPR Right to Erasure
```rust
pub struct GDPREngine {
    storage: Arc<TwoTierStorage>,
}

pub struct ErasureRequest {
    pub request_id: String,
    pub data_subject_id: String,
    pub requested_at: i64,
    pub status: ErasureStatus,
    pub pii_found_count: u64,
    pub pii_redacted_count: u64,
}

pub enum ErasureStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

impl GDPREngine {
    pub async fn submit_erasure_request(&self, data_subject_id: &str)
        -> Result<String, ErasureError>
    {
        let request = ErasureRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            data_subject_id: data_subject_id.to_string(),
            requested_at: now(),
            status: ErasureStatus::Pending,
            pii_found_count: 0,
            pii_redacted_count: 0,
        };

        Ok(request.request_id)
    }

    pub async fn process_erasure_request(&self, request_id: &str)
        -> Result<ErasureRequest, ErasureError>
    {
        // Find all data referencing data_subject
        // Redact PII in-place
        // Mark as erased

        Ok(ErasureRequest {
            request_id: request_id.to_string(),
            data_subject_id: "redacted".to_string(),
            requested_at: now(),
            status: ErasureStatus::Completed,
            pii_found_count: 100,
            pii_redacted_count: 100,
        })
    }

    pub async fn generate_erasure_certificate(&self, request_id: &str) -> Result<String, ErasureError> {
        // Generate proof that erasure completed
        Ok(format!("ERASURE_CERTIFICATE_{}_{}", request_id, now()))
    }

    pub async fn verify_no_pii_remains(&self, data_subject_id: &str) -> Result<bool, ErasureError> {
        // Scan all storage for any PII referencing subject
        // Return true if none found
        Ok(true)
    }
}
```

## Dependencies
- **Blocked by:** Week 18 (Compliance Engine)
- **Blocking:** Week 20 (final testing and compliance validation)

## Acceptance Criteria
- [ ] Operational tier auto-cleanup after 7 days
- [ ] Compliance tier immutable; data preserved ≥6 months
- [ ] Data integrity verified during tier movement
- [ ] Legal holds prevent deletion; expiration automatic
- [ ] GDPR erasure requests processed; PII redacted
- [ ] Erasure certificates generated and verified
- [ ] Automated retention job runs daily without errors
- [ ] Metadata-only queries work without restoring from archive
- [ ] Unit tests cover tier movement, legal holds, erasure
- [ ] Integration tests verify end-to-end compliance

## Design Principles Alignment
- **Data safety:** Legal holds protect data during litigation
- **Privacy:** GDPR erasure removes PII on request
- **Compliance:** Automatic enforcement prevents policy violations
- **Efficiency:** Compression and tiering reduce storage costs
- **Auditability:** All retention actions logged and traceable
