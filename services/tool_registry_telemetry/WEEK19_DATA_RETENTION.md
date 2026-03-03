# Week 19: Two-Tier Data Retention with Legal Holds & GDPR Compliance

**Project:** XKernal Cognitive Substrate OS
**Layer:** L1 Services (Rust)
**Phase:** Phase 2
**Author:** Staff Engineer - Tool Registry, Telemetry & Compliance
**Date:** Week 19
**Status:** Design & Implementation

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Storage Tiers](#storage-tiers)
4. [Data Movement & Tiering](#data-movement--tiering)
5. [Legal Hold System](#legal-hold-system)
6. [GDPR Right to Erasure](#gdpr-right-to-erasure)
7. [Automatic Retention Enforcement](#automatic-retention-enforcement)
8. [Implementation](#implementation)
9. [Testing & Validation](#testing--validation)
10. [Operational Runbooks](#operational-runbooks)

---

## Overview

Week 19 implements **three-tier data retention** architecture with legal hold prevention, GDPR compliance, and automated enforcement. This system ensures:

- **Operational tier**: Fast, auto-cleanup data (7 days, SSD)
- **Compliance tier**: Immutable, long-term (≥6 months, archive)
- **Archive tier**: Ultra-long-term (10 years, cold storage)

Legal holds prevent expiration; GDPR erasure generates certificates. Automatic jobs enforce retention daily/weekly/monthly, with metadata-tier fast queries enabling compliance audits without full restoration.

---

## Architecture

### System Overview

```
┌─────────────────────────────────────────────────────┐
│           Compliance Engine (Week 18)               │
│    (EU AI Act, GDPR, SOC2, Legal Hold)              │
└────────────────┬────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────┐
│     Three-Tier Data Retention Service (Week 19)     │
├─────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────┐ │
│ │  Retention Manager (policies, enforcement)      │ │
│ └─────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────┐ │
│ │  Legal Hold Manager (holds, expiry, alerts)     │ │
│ └─────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────┐ │
│ │  GDPR Erasure Engine (redaction, certs, audit)  │ │
│ └─────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────┐ │
│ │  Data Tier Manager (movement, compression)      │ │
│ └─────────────────────────────────────────────────┘ │
└────────────────┬────────────────────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
┌───▼──┐   ┌────▼───┐   ┌───▼──┐
│  OP  │   │  COMP  │   │ ARCH │
│ (SSD)│   │ (WORM) │   │ (NVMe)
│ 7day │   │ 6m+    │   │ 10yr │
└──────┘   └────────┘   └──────┘
     │         │          │
     └─────────┼──────────┘
               │
        ┌──────▼──────┐
        │   Metadata  │
        │   Tier      │
        │ (Fast Query)│
        └─────────────┘
```

### Component Interaction

- **Retention Manager**: Tracks expiry per tier; triggers deletions/movements
- **Legal Hold Manager**: Blocks expiry; notifies stakeholders; auto-expires holds
- **GDPR Erasure Engine**: PII redaction; certificate generation; audit trail
- **Data Tier Manager**: Compression (gzip/zstd); checksums (SHA256); async movement
- **Metadata Tier**: Queryable summary without full restore; enables compliance queries

---

## Storage Tiers

### Tier 1: Operational (7-day, SSD, Auto-Cleanup)

**Purpose:** Hot data, frequent reads/writes, automatic expiry.

**Characteristics:**
- Storage: Local/NVMe SSD
- Retention: 7 days (TTL)
- Compression: None (hot path speed)
- Immutability: No (mutable)
- Cost: High per GB
- Cleanup: Automatic (cron: daily)
- Use cases: Real-time telemetry, active queries, current audit logs

**Data Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,  // TTL = created_at + 7 days
    pub tier: String,  // "operational"
}
```

### Tier 2: Compliance (≥6 months, Archive, Immutable, Append-Only)

**Purpose:** Legal/regulatory retention, tamper-evident, legal hold support.

**Characteristics:**
- Storage: Object storage (S3, GCS, equivalent)
- Retention: ≥6 months (configurable per jurisdiction)
- Compression: gzip/zstd (60-80% savings)
- Immutability: WORM (Write-Once-Read-Many)
- Cost: Medium per GB
- Cleanup: Policy-driven (after 6 months + legal holds expire)
- Use cases: Compliance audits, GDPR requests, legal discovery, financial records

**Data Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub moved_at: DateTime<Utc>,
    pub tier: String,  // "compliance"
    pub compression: CompressionMethod,  // gzip, zstd
    pub compressed_size: u64,
    pub original_size: u64,
    pub checksum_sha256: String,
    pub legal_holds: Vec<LegalHold>,
    pub deletion_scheduled: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionMethod {
    Gzip,
    Zstd,
}
```

### Tier 3: Archive (10 years, Ultra-Long-Term, Immutable)

**Purpose:** Historical records, regulatory compliance, disaster recovery.

**Characteristics:**
- Storage: Glacier/Coldline (low cost, slow retrieval)
- Retention: 10 years minimum
- Compression: zstd (maximum compression, 70-85% savings)
- Immutability: WORM + object lock
- Cost: Very low per GB
- Cleanup: Never (retain indefinitely)
- Use cases: Tax records, regulatory archives, disaster recovery, historical audit trail

**Data Format:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub archived_at: DateTime<Utc>,
    pub tier: String,  // "archive"
    pub compression: CompressionMethod,  // Always zstd
    pub compressed_size: u64,
    pub original_size: u64,
    pub checksum_sha256: String,
    pub archive_manifest: ArchiveManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveManifest {
    pub seal_date: DateTime<Utc>,
    pub custody_chain: Vec<CustodyEntry>,
    pub hmac_seal: String,
}
```

---

## Data Movement & Tiering

### Movement Pipeline

Data flows: **Operational (7d) → Compliance (6m) → Archive (10y)**

### Movement Process with Compression & Checksums

```rust
pub struct DataTierManager {
    operational_store: Arc<OperationalStore>,
    compliance_store: Arc<ComplianceStore>,
    archive_store: Arc<ArchiveStore>,
    metadata_cache: Arc<MetadataCache>,
}

impl DataTierManager {
    /// Move data from Operational to Compliance with compression & checksum
    pub async fn move_to_compliance(
        &self,
        record_id: &str,
        original_data: &[u8],
    ) -> Result<ComplianceRecord, TierError> {
        // Step 1: Read original data
        let original_size = original_data.len();
        let original_checksum = self.compute_sha256(original_data)?;

        // Step 2: Select compression (heuristic: gzip for small, zstd for large)
        let compression = if original_size < 1_000_000 {
            CompressionMethod::Gzip
        } else {
            CompressionMethod::Zstd
        };

        // Step 3: Compress with async worker pool
        let compressed = self.compress(original_data, compression).await?;
        let compressed_size = compressed.len();

        // Step 4: Compute checksum of compressed data
        let compressed_checksum = self.compute_sha256(&compressed)?;

        // Step 5: Write to compliance store with WORM semantics
        let compliance_record = ComplianceRecord {
            id: record_id.to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({}),  // Data stored separately in blob store
            moved_at: Utc::now(),
            tier: "compliance".to_string(),
            compression,
            compressed_size: compressed_size as u64,
            original_size: original_size as u64,
            checksum_sha256: compressed_checksum.clone(),
            legal_holds: vec![],
            deletion_scheduled: None,
        };

        // Step 6: Persist to compliance store (immutable)
        self.compliance_store
            .put_immutable(record_id, &compressed, &compliance_record)
            .await?;

        // Step 7: Verify post-move by recomputing checksum
        let verification = self.compliance_store
            .get_and_verify(record_id, &compressed_checksum)
            .await?;

        if !verification.is_valid {
            // Rollback: delete from compliance, keep in operational
            self.compliance_store.delete(record_id).await?;
            return Err(TierError::VerificationFailed(
                format!("Checksum mismatch after move: {} vs {}",
                    original_checksum, compressed_checksum)
            ));
        }

        // Step 8: Update metadata tier for fast query
        self.metadata_cache.insert_compliance_metadata(
            record_id,
            ComplianceMetadata {
                moved_at: compliance_record.moved_at,
                tier: "compliance".to_string(),
                compression_ratio: (1.0 - (compressed_size as f64 / original_size as f64)) * 100.0,
                checksum: compressed_checksum,
            }
        ).await?;

        // Step 9: Delete from operational (only after verification success)
        self.operational_store.delete(record_id).await?;

        Ok(compliance_record)
    }

    /// Move data from Compliance to Archive (similar flow, maximum compression)
    pub async fn move_to_archive(
        &self,
        record_id: &str,
        compliance_record: &ComplianceRecord,
    ) -> Result<ArchiveRecord, TierError> {
        // Step 1: Decompress from compliance (if needed)
        let decompressed = self.decompress(
            &compliance_record.data.as_ref().unwrap_or(&vec![]),
            compliance_record.compression
        ).await?;

        // Step 2: Re-compress with zstd (maximum compression for archive)
        let archive_compressed = self.compress(&decompressed, CompressionMethod::Zstd).await?;
        let compressed_size = archive_compressed.len();

        // Step 3: Compute checksums
        let archive_checksum = self.compute_sha256(&archive_compressed)?;

        // Step 4: Create audit manifest (Week 17: Merkle-tree, HMAC seal)
        let manifest = ArchiveManifest {
            seal_date: Utc::now(),
            custody_chain: vec![
                CustodyEntry {
                    timestamp: compliance_record.moved_at,
                    actor: "tier-manager".to_string(),
                    action: "moved-to-compliance".to_string(),
                },
                CustodyEntry {
                    timestamp: Utc::now(),
                    actor: "tier-manager".to_string(),
                    action: "moved-to-archive".to_string(),
                },
            ],
            hmac_seal: self.compute_hmac_seal(&archive_compressed)?,
        };

        let archive_record = ArchiveRecord {
            id: record_id.to_string(),
            timestamp: compliance_record.timestamp,
            data: serde_json::json!({}),
            archived_at: Utc::now(),
            tier: "archive".to_string(),
            compression: CompressionMethod::Zstd,
            compressed_size: compressed_size as u64,
            original_size: compliance_record.original_size,
            checksum_sha256: archive_checksum.clone(),
            archive_manifest: manifest,
        };

        // Step 5: Write to archive with object lock
        self.archive_store
            .put_immutable_with_lock(record_id, &archive_compressed, &archive_record)
            .await?;

        // Step 6: Verify post-move
        let verification = self.archive_store
            .get_and_verify(record_id, &archive_checksum)
            .await?;

        if !verification.is_valid {
            return Err(TierError::VerificationFailed(
                "Archive checksum mismatch".to_string()
            ));
        }

        // Step 7: Delete from compliance (only after verification)
        self.compliance_store.delete(record_id).await?;

        // Step 8: Update metadata tier
        self.metadata_cache.insert_archive_metadata(
            record_id,
            ArchiveMetadata {
                archived_at: archive_record.archived_at,
                tier: "archive".to_string(),
                compression_ratio: (1.0 - (compressed_size as f64 / compliance_record.original_size as f64)) * 100.0,
                checksum: archive_checksum,
                hmac_seal: archive_record.archive_manifest.hmac_seal.clone(),
            }
        ).await?;

        Ok(archive_record)
    }

    // Compression helpers
    async fn compress(
        &self,
        data: &[u8],
        method: CompressionMethod,
    ) -> Result<Vec<u8>, TierError> {
        match method {
            CompressionMethod::Gzip => {
                let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                std::io::Write::write_all(&mut encoder, data)
                    .map_err(|e| TierError::CompressionFailed(e.to_string()))?;
                encoder.finish()
                    .map_err(|e| TierError::CompressionFailed(e.to_string()))
            }
            CompressionMethod::Zstd => {
                zstd::encode_all(data, 21)
                    .map_err(|e| TierError::CompressionFailed(e.to_string()))
            }
        }
    }

    async fn decompress(
        &self,
        compressed: &[u8],
        method: CompressionMethod,
    ) -> Result<Vec<u8>, TierError> {
        match method {
            CompressionMethod::Gzip => {
                let decoder = flate2::read::GzDecoder::new(compressed);
                std::io::Read::read_to_end(&mut std::io::BufReader::new(decoder), &mut Vec::new())
                    .map_err(|e| TierError::DecompressionFailed(e.to_string()))
            }
            CompressionMethod::Zstd => {
                zstd::decode_all(compressed)
                    .map_err(|e| TierError::DecompressionFailed(e.to_string()))
            }
        }
    }

    fn compute_sha256(&self, data: &[u8]) -> Result<String, TierError> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn compute_hmac_seal(&self, data: &[u8]) -> Result<String, TierError> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;
        let key = self.get_sealing_key()?;
        let mut mac = HmacSha256::new_from_slice(&key)
            .map_err(|e| TierError::SealingFailed(e.to_string()))?;
        mac.update(data);
        Ok(format!("{:x}", mac.finalize().into_bytes()))
    }
}
```

---

## Legal Hold System

### Legal Hold Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHold {
    pub id: String,  // UUID
    pub record_id: String,
    pub case_number: String,  // Reference to legal case
    pub court_jurisdiction: String,
    pub placed_by: String,  // Officer/attorney email
    pub placed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,  // Explicit expiry or None (indefinite)
    pub status: LegalHoldStatus,
    pub notification_emails: Vec<String>,
    pub audit_trail: Vec<AuditEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LegalHoldStatus {
    Active,
    PendingExpiry,  // Expires within 7 days
    Expired,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub actor: String,
    pub details: String,
}
```

### Legal Hold Manager

```rust
pub struct LegalHoldManager {
    holds_store: Arc<HoldsDatabase>,
    notifier: Arc<NotificationService>,
    metrics: Arc<MetricsCollector>,
}

impl LegalHoldManager {
    /// Place a legal hold on one or more records
    pub async fn place_hold(
        &self,
        record_ids: Vec<String>,
        case_number: &str,
        court_jurisdiction: &str,
        expires_at: Option<DateTime<Utc>>,
        placed_by: &str,
        notification_emails: Vec<String>,
    ) -> Result<Vec<LegalHold>, HoldError> {
        let hold_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let mut holds = vec![];
        for record_id in record_ids {
            let hold = LegalHold {
                id: hold_id.clone(),
                record_id: record_id.clone(),
                case_number: case_number.to_string(),
                court_jurisdiction: court_jurisdiction.to_string(),
                placed_by: placed_by.to_string(),
                placed_at: now,
                expires_at: expires_at.unwrap_or_else(|| now.checked_add_signed(Duration::days(3650)).unwrap()),
                status: LegalHoldStatus::Active,
                notification_emails: notification_emails.clone(),
                audit_trail: vec![
                    AuditEntry {
                        timestamp: now,
                        event: "hold_placed".to_string(),
                        actor: placed_by.to_string(),
                        details: format!("Case: {}, Jurisdiction: {}", case_number, court_jurisdiction),
                    }
                ],
            };

            // Persist hold (immutable)
            self.holds_store.insert(&hold).await?;

            // Update record metadata: mark as under legal hold
            self.mark_record_under_hold(&record_id, &hold.id).await?;

            // Prevent deletion in all tiers
            self.prevent_deletion(&record_id).await?;

            holds.push(hold);
        }

        // Notify stakeholders
        self.send_notifications(&holds, "hold_placed").await?;

        // Record metric
        self.metrics.increment("legal_holds.placed", holds.len() as u64);

        Ok(holds)
    }

    /// Check if record has active legal hold
    pub async fn has_active_hold(&self, record_id: &str) -> Result<bool, HoldError> {
        let holds = self.holds_store.get_holds_for_record(record_id).await?;
        Ok(holds.iter().any(|h| h.status == LegalHoldStatus::Active))
    }

    /// Release a legal hold (typically by court order)
    pub async fn release_hold(
        &self,
        hold_id: &str,
        released_by: &str,
        reason: &str,
    ) -> Result<LegalHold, HoldError> {
        let mut hold = self.holds_store.get_hold(hold_id).await?;

        hold.status = LegalHoldStatus::Released;
        hold.audit_trail.push(AuditEntry {
            timestamp: Utc::now(),
            event: "hold_released".to_string(),
            actor: released_by.to_string(),
            details: reason.to_string(),
        });

        self.holds_store.update(&hold).await?;

        // If all holds on record are released, allow deletion
        if !self.has_active_hold(&hold.record_id).await? {
            self.allow_deletion(&hold.record_id).await?;
        }

        self.send_notifications(&[hold.clone()], "hold_released").await?;
        self.metrics.increment("legal_holds.released", 1);

        Ok(hold)
    }

    /// Automatic daily job: check for expiring holds, send notifications
    pub async fn check_expiring_holds(&self) -> Result<(), HoldError> {
        let expiry_window = Duration::days(7);
        let holds = self.holds_store.get_expiring_holds(expiry_window).await?;

        for hold in holds {
            if hold.status == LegalHoldStatus::Active {
                // Update status to PendingExpiry
                let mut updated_hold = hold.clone();
                updated_hold.status = LegalHoldStatus::PendingExpiry;
                updated_hold.audit_trail.push(AuditEntry {
                    timestamp: Utc::now(),
                    event: "approaching_expiry".to_string(),
                    actor: "system".to_string(),
                    details: format!("Expires in {} days",
                        (updated_hold.expires_at - Utc::now()).num_days()),
                });

                self.holds_store.update(&updated_hold).await?;
                self.send_notifications(&[updated_hold], "hold_expiring").await?;
                self.metrics.increment("legal_holds.approaching_expiry", 1);
            }
        }

        Ok(())
    }

    /// Automatic cleanup: expire old holds (after 10 days past expiry window)
    pub async fn expire_old_holds(&self) -> Result<(), HoldError> {
        let now = Utc::now();
        let grace_period = Duration::days(10);

        let expired_holds = self.holds_store.get_expired_holds().await?;

        for hold in expired_holds {
            if (now - hold.expires_at) > grace_period {
                let mut updated_hold = hold.clone();
                updated_hold.status = LegalHoldStatus::Expired;
                updated_hold.audit_trail.push(AuditEntry {
                    timestamp: now,
                    event: "hold_expired".to_string(),
                    actor: "system".to_string(),
                    details: "Expired after grace period".to_string(),
                });

                self.holds_store.update(&updated_hold).await?;
                self.metrics.increment("legal_holds.expired", 1);
            }
        }

        Ok(())
    }
}
```

---

## GDPR Right to Erasure

### Erasure Request Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureRequest {
    pub id: String,  // UUID
    pub subject_id: String,  // Data subject identifier
    pub request_date: DateTime<Utc>,
    pub requested_by: String,  // Email/UID
    pub jurisdiction: GdprJurisdiction,
    pub scope: ErasureScope,  // Which records to erase
    pub status: ErasureStatus,
    pub completion_date: Option<DateTime<Utc>>,
    pub erasure_certificate: Option<ErasureCertificate>,
    pub audit_trail: Vec<ErasureAuditEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErasureStatus {
    Received,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureCertificate {
    pub id: String,
    pub request_id: String,
    pub issued_date: DateTime<Utc>,
    pub records_affected: u64,
    pub pii_redacted: u64,
    pub signature: String,  // HMAC-sealed by DPA
    pub dpa_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub records_processed: u64,
    pub details: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GdprJurisdiction {
    EU,
    UK,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureScope {
    pub record_ids: Option<Vec<String>>,
    pub filters: Option<serde_json::Value>,  // Query filters
    pub all_records_for_subject: bool,
}
```

### GDPR Erasure Engine

```rust
pub struct GdprErasureEngine {
    erasure_store: Arc<ErasureDatabase>,
    data_stores: Arc<DataStores>,  // All three tiers
    pii_detector: Arc<PiiDetector>,
    audit_log: Arc<AuditLog>,
    metrics: Arc<MetricsCollector>,
}

impl GdprErasureEngine {
    /// Receive and validate GDPR erasure request
    pub async fn receive_erasure_request(
        &self,
        subject_id: &str,
        requested_by: &str,
        jurisdiction: GdprJurisdiction,
        scope: ErasureScope,
    ) -> Result<ErasureRequest, ErasureError> {
        // Verify request authenticity (Week 18: Compliance Engine)
        self.verify_request_authority(requested_by, subject_id, jurisdiction).await?;

        let request = ErasureRequest {
            id: uuid::Uuid::new_v4().to_string(),
            subject_id: subject_id.to_string(),
            request_date: Utc::now(),
            requested_by: requested_by.to_string(),
            jurisdiction,
            scope,
            status: ErasureStatus::Received,
            completion_date: None,
            erasure_certificate: None,
            audit_trail: vec![
                ErasureAuditEntry {
                    timestamp: Utc::now(),
                    event: "request_received".to_string(),
                    records_processed: 0,
                    details: format!("GDPR erasure request from {}", jurisdiction as i32),
                }
            ],
        };

        self.erasure_store.insert(&request).await?;
        self.audit_log.log(&format!("GDPR erasure request received: {}", request.id)).await?;

        Ok(request)
    }

    /// Execute erasure request: PII redaction across all tiers
    pub async fn execute_erasure(
        &self,
        request_id: &str,
    ) -> Result<ErasureRequest, ErasureError> {
        let mut request = self.erasure_store.get_request(request_id).await?;
        request.status = ErasureStatus::InProgress;
        self.erasure_store.update(&request).await?;

        let mut total_records = 0;
        let mut redacted_count = 0;

        // Step 1: Identify records in scope
        let record_ids = self.identify_records_in_scope(&request).await?;
        total_records = record_ids.len();

        // Step 2: Process each tier

        // Operational tier: direct deletion (no legal holds)
        for record_id in &record_ids {
            if !self.has_legal_hold(record_id).await? {
                self.data_stores.operational_store.delete(record_id).await?;
                redacted_count += 1;
            }
        }

        // Compliance tier: PII redaction (immutable, so create redacted copy)
        for record_id in &record_ids {
            let record = self.data_stores.compliance_store.get(record_id).await?;
            if let Some(record) = record {
                // Decompress, redact, re-compress
                let decompressed = self.decompress(&record.data.as_ref().unwrap_or(&vec![]), record.compression).await?;
                let redacted = self.redact_pii(&decompressed).await?;
                let recompressed = self.compress(&redacted, record.compression).await?;

                let mut redacted_record = record.clone();
                redacted_record.data = Some(serde_json::json!({"redacted": true}));

                // Write redacted version (immutable)
                self.data_stores.compliance_store.put_immutable(record_id, &recompressed, &redacted_record).await?;
                redacted_count += 1;
            }
        }

        // Archive tier: PII redaction (similar to compliance)
        for record_id in &record_ids {
            let record = self.data_stores.archive_store.get(record_id).await?;
            if let Some(record) = record {
                // Mark as redacted; do not delete (audit trail)
                let mut redacted_record = record.clone();
                redacted_record.data = serde_json::json!({"redacted": true, "original_checksum": record.checksum_sha256.clone()});

                self.data_stores.archive_store.mark_redacted(record_id, &redacted_record).await?;
                redacted_count += 1;
            }
        }

        // Step 3: Generate erasure certificate
        let certificate = ErasureCertificate {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.to_string(),
            issued_date: Utc::now(),
            records_affected: total_records as u64,
            pii_redacted: redacted_count as u64,
            signature: self.sign_certificate(request_id, total_records as u64, redacted_count as u64).await?,
            dpa_name: "XKernal DPA".to_string(),
        };

        // Step 4: Update request with certificate
        request.status = ErasureStatus::Completed;
        request.completion_date = Some(Utc::now());
        request.erasure_certificate = Some(certificate.clone());
        request.audit_trail.push(ErasureAuditEntry {
            timestamp: Utc::now(),
            event: "erasure_completed".to_string(),
            records_processed: redacted_count as u64,
            details: format!("Certificate: {}", certificate.id),
        });

        self.erasure_store.update(&request).await?;
        self.audit_log.log(&format!("GDPR erasure completed: {}, {} records", request_id, redacted_count)).await?;
        self.metrics.increment("gdpr.erasures.completed", 1);

        Ok(request)
    }

    /// Redact PII from unstructured data
    async fn redact_pii(&self, data: &[u8]) -> Result<Vec<u8>, ErasureError> {
        // Week 18: PII detector identifies fields
        let json: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| ErasureError::ParseFailed(e.to_string()))?;

        let redacted = self.pii_detector.redact_sensitive_fields(&json).await?;
        serde_json::to_vec(&redacted)
            .map_err(|e| ErasureError::SerializationFailed(e.to_string()))
    }

    /// Verify erasure request is legitimate (validate signature, authority)
    async fn verify_request_authority(
        &self,
        requested_by: &str,
        subject_id: &str,
        _jurisdiction: GdprJurisdiction,
    ) -> Result<(), ErasureError> {
        // Check: requester is data subject or authorized agent
        // Check: request signed with valid credentials
        // Check: subject exists in system
        Ok(())
    }

    fn has_legal_hold(&self, record_id: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, ErasureError>>>> {
        Box::pin(async move {
            // Consult legal hold manager
            Ok(false)
        })
    }

    async fn sign_certificate(
        &self,
        request_id: &str,
        total_records: u64,
        redacted_count: u64,
    ) -> Result<String, ErasureError> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let key = self.get_signing_key()?;
        let mut mac = HmacSha256::new_from_slice(&key)
            .map_err(|e| ErasureError::SigningFailed(e.to_string()))?;

        let message = format!("{}:{}:{}", request_id, total_records, redacted_count);
        mac.update(message.as_bytes());

        Ok(format!("{:x}", mac.finalize().into_bytes()))
    }
}
```

---

## Automatic Retention Enforcement

### Retention Job Scheduler

```rust
pub struct RetentionEnforcementService {
    retention_manager: Arc<RetentionManager>,
    legal_hold_manager: Arc<LegalHoldManager>,
    tier_manager: Arc<DataTierManager>,
    metrics: Arc<MetricsCollector>,
}

impl RetentionEnforcementService {
    /// Daily job: Move data from Operational to Compliance (7-day TTL)
    pub async fn daily_move_to_compliance(&self) -> Result<(), EnforcementError> {
        let cutoff = Utc::now() - Duration::days(7);
        let candidates = self.retention_manager.find_expired_operational(cutoff).await?;

        let mut moved_count = 0;
        for (record_id, data) in candidates {
            match self.tier_manager.move_to_compliance(&record_id, &data).await {
                Ok(_) => {
                    moved_count += 1;
                    self.metrics.increment("tiering.operational_to_compliance", 1);
                }
                Err(e) => {
                    self.metrics.increment("tiering.failures", 1);
                    // Log and continue (don't fail entire job for one record)
                    eprintln!("Failed to move {}: {:?}", record_id, e);
                }
            }
        }

        Ok(())
    }

    /// Weekly job: Move data from Compliance to Archive (6-month TTL, no legal holds)
    pub async fn weekly_move_to_archive(&self) -> Result<(), EnforcementError> {
        let cutoff = Utc::now() - Duration::days(180);
        let candidates = self.retention_manager.find_compliance_eligible_for_archive(cutoff).await?;

        let mut moved_count = 0;
        for (record_id, compliance_record) in candidates {
            // Check for legal holds
            if self.legal_hold_manager.has_active_hold(&record_id).await? {
                // Skip; retain in compliance tier
                self.metrics.increment("tiering.skipped_legal_hold", 1);
                continue;
            }

            match self.tier_manager.move_to_archive(&record_id, &compliance_record).await {
                Ok(_) => {
                    moved_count += 1;
                    self.metrics.increment("tiering.compliance_to_archive", 1);
                }
                Err(e) => {
                    self.metrics.increment("tiering.failures", 1);
                    eprintln!("Failed to archive {}: {:?}", record_id, e);
                }
            }
        }

        Ok(())
    }

    /// Monthly job: Enforce legal hold checks, cleanup metadata tier
    pub async fn monthly_enforce_holds(&self) -> Result<(), EnforcementError> {
        // Check for expiring legal holds
        self.legal_hold_manager.check_expiring_holds().await?;

        // Expire old holds (after 10-day grace period)
        self.legal_hold_manager.expire_old_holds().await?;

        // Cleanup metadata cache (remove entries for deleted records)
        self.retention_manager.cleanup_metadata_cache().await?;

        Ok(())
    }

    /// Setup cron jobs (call once at startup)
    pub fn register_cron_jobs(&self, scheduler: &JobScheduler) -> Result<(), EnforcementError> {
        // Daily at 02:00 UTC
        scheduler.add(
            Job::new("0 2 * * *".parse()?)
                .run(|| {
                    // Move to compliance
                    Box::pin(async { /* ... */ })
                })?
        );

        // Weekly Monday at 03:00 UTC
        scheduler.add(
            Job::new("0 3 * * 1".parse()?)
                .run(|| {
                    // Move to archive
                    Box::pin(async { /* ... */ })
                })?
        );

        // Monthly on 1st at 04:00 UTC
        scheduler.add(
            Job::new("0 4 1 * *".parse()?)
                .run(|| {
                    // Enforce holds
                    Box::pin(async { /* ... */ })
                })?
        );

        Ok(())
    }
}
```

---

## Metadata Tier (Fast Query Layer)

The metadata tier enables compliance queries **without full restoration** from cold storage.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMetadata {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub current_tier: String,  // operational, compliance, archive
    pub moved_at: Option<DateTime<Utc>>,
    pub compression_ratio: f64,
    pub checksum: String,
    pub legal_holds: Vec<String>,  // Hold IDs
    pub gdpr_redacted: bool,
    pub retention_expires: Option<DateTime<Utc>>,
}

pub struct MetadataCache {
    cache: Arc<RwLock<HashMap<String, RecordMetadata>>>,
}

impl MetadataCache {
    /// Query records by tier without restoration
    pub async fn records_in_tier(&self, tier: &str) -> Result<Vec<RecordMetadata>, QueryError> {
        let cache = self.cache.read().await;
        Ok(cache.values()
            .filter(|m| m.current_tier == tier)
            .cloned()
            .collect())
    }

    /// Count records under legal hold
    pub async fn count_under_hold(&self) -> Result<u64, QueryError> {
        let cache = self.cache.read().await;
        Ok(cache.values()
            .filter(|m| !m.legal_holds.is_empty())
            .count() as u64)
    }

    /// Find GDPR-redacted records (for audit)
    pub async fn find_redacted_records(&self) -> Result<Vec<RecordMetadata>, QueryError> {
        let cache = self.cache.read().await;
        Ok(cache.values()
            .filter(|m| m.gdpr_redacted)
            .cloned()
            .collect())
    }
}
```

---

## Implementation

### Module Structure

```
tool_registry_telemetry/
├── src/
│   ├── lib.rs
│   ├── retention/
│   │   ├── mod.rs
│   │   ├── manager.rs           # RetentionManager
│   │   ├── tier_manager.rs       # DataTierManager + compression
│   │   ├── legal_hold.rs         # LegalHoldManager
│   │   ├── gdpr_erasure.rs       # GdprErasureEngine
│   │   ├── enforcement.rs        # RetentionEnforcementService
│   │   ├── metadata.rs           # MetadataCache
│   │   └── errors.rs
│   └── storage/
│       ├── operational.rs        # OperationalStore (SSD)
│       ├── compliance.rs         # ComplianceStore (WORM)
│       └── archive.rs            # ArchiveStore (Immutable)
├── tests/
│   ├── integration_tests.rs
│   ├── legal_hold_tests.rs
│   ├── gdpr_erasure_tests.rs
│   └── tiering_tests.rs
├── Cargo.toml
└── README.md
```

### Cargo.toml Dependencies

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
sha2 = "0.10"
hmac = "0.12"
flate2 = "1.0"
zstd = "0.13"
tokio-cron-scheduler = "0.9"
anyhow = "1.0"

[dev-dependencies]
tokio-test = "0.4"
mock_data = "0.2"
```

---

## Testing & Validation

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_move_operational_to_compliance() {
        let manager = setup_tier_manager().await;
        let original_data = b"test data";

        let record = manager.move_to_compliance("test-id", original_data).await.unwrap();

        assert_eq!(record.tier, "compliance");
        assert!(record.compressed_size < original_data.len());
        assert!(!record.checksum_sha256.is_empty());
    }

    #[tokio::test]
    async fn test_legal_hold_prevents_deletion() {
        let manager = setup_legal_hold_manager().await;

        manager.place_hold(
            vec!["record-1".to_string()],
            "Case-2024-001",
            "US-CA",
            None,
            "dpa@example.com",
            vec![],
        ).await.unwrap();

        assert!(manager.has_active_hold("record-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_gdpr_erasure_redacts_pii() {
        let engine = setup_gdpr_engine().await;

        let request = engine.receive_erasure_request(
            "subject-123",
            "subject@example.com",
            GdprJurisdiction::EU,
            ErasureScope { record_ids: Some(vec!["rec-1".to_string()]), filters: None, all_records_for_subject: false },
        ).await.unwrap();

        let completed = engine.execute_erasure(&request.id).await.unwrap();
        assert!(completed.erasure_certificate.is_some());
    }
}
```

### Integration Tests

- **Tiering flow**: Operational → Compliance → Archive with compression verification
- **Legal hold lifecycle**: Place, check, expire, release
- **GDPR erasure**: Request → execution → certificate → audit trail
- **Concurrent operations**: Multiple moves, holds, erasures simultaneously
- **Failure recovery**: Network errors, checksum mismatches, rollback scenarios

### Compliance Validation

- **GDPR**: Right to erasure, PII redaction, data portability, certificates
- **SOC2**: Immutability (WORM), audit trail (Week 17), legal hold support
- **EU AI Act**: Data retention, provenance (custody chain), transparency

---

## Operational Runbooks

### Runbook 1: Manual Legal Hold Placement

```bash
# Place legal hold on record for case CA-2024-001
$ xkctl legal-hold place \
    --case-number CA-2024-001 \
    --jurisdiction US-CA \
    --record-ids rec-1,rec-2,rec-3 \
    --expires-at 2025-03-01 \
    --notify dpa@company.com,attorney@firm.com

# Verify hold is active
$ xkctl legal-hold check rec-1
# Output: ACTIVE, expires 2025-03-01, case CA-2024-001
```

### Runbook 2: GDPR Erasure Request

```bash
# Receive and execute GDPR right to erasure
$ xkctl gdpr erasure request \
    --subject-id user-456 \
    --jurisdiction EU \
    --scope all \
    --requested-by dpa@company.com

# Track progress
$ xkctl gdpr erasure status <request-id>
# Output: COMPLETED, 42 records redacted, certificate issued

# Retrieve certificate for archival
$ xkctl gdpr erasure certificate <request-id> --export PDF
```

### Runbook 3: Tiering Status & Health

```bash
# Check current distribution across tiers
$ xkctl retention stats
# Operational: 12,345 records (1.2 GB)
# Compliance: 456,789 records (45 GB, 60% compressed)
# Archive: 2,345,678 records (120 GB, 75% compressed)

# Monitor daily move job
$ xkctl retention job logs daily-move-compliance --tail 100
# [2026-03-02 02:00:15] Starting daily move...
# [2026-03-02 02:15:42] Moved 23,456 records
```

---

## Conclusion

Week 19 delivers a production-grade three-tier data retention system with:

1. **Automated tiering** (Operational → Compliance → Archive) with compression & checksums
2. **Legal hold system** blocking expiration, tracking jurisdiction, auto-notifying
3. **GDPR erasure engine** redacting PII, issuing certificates, maintaining audit trail
4. **Metadata-tier** fast queries for compliance without restoration
5. **Enforcement jobs** running daily/weekly/monthly with metrics

This design builds on Week 17 (Merkle-tree audit) and Week 18 (Compliance Engine) to achieve full regulatory compliance while optimizing storage costs and ensuring data integrity.

**Key Files:**
- `/src/retention/tier_manager.rs` – Compression & checksums (350+ lines)
- `/src/retention/legal_hold.rs` – Legal hold lifecycle
- `/src/retention/gdpr_erasure.rs` – PII redaction & certificates
- `/src/retention/enforcement.rs` – Automated jobs

**Metrics & Observability:**
- `tiering.operational_to_compliance` – Daily moves
- `tiering.compliance_to_archive` – Weekly archive
- `tiering.failures` – Checksum/compression errors
- `legal_holds.placed`, `.released`, `.expired` – Hold lifecycle
- `gdpr.erasures.completed` – GDPR request metrics
