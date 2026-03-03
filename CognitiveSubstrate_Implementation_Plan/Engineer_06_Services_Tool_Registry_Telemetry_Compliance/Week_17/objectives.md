# Engineer 6 — Services: Tool Registry, Telemetry & Compliance — Week 17

## Phase: Phase 2 (Weeks 15-24)

## Weekly Objective
Begin Compliance Engine implementation with Merkle-tree audit log for tamper-evident, append-only record keeping. Establish cryptographic foundation for regulatory compliance.

## Document References
- **Primary:** Section 6.3 (Phase 2, Week 17-20: Compliance Engine with journaling, Merkle-tree audit log), Section 3.3.5 (Compliance Engine, Merkle-tree audit trail, tamper-evident)
- **Supporting:** Week 15-16 (PolicyDecision events), Week 11 (Telemetry)

## Deliverables
- [ ] Merkle-tree data structure implementation
  - Hash nodes: SHA-256 of leaf data
  - Tree traversal and proof generation
  - Proof verification (detect tampering)
  - Tree serialization and deserialization
- [ ] Audit log entry schema
  - Entry type (PolicyDecision, ToolInvocation, MemoryAccess, Checkpoint, etc.)
  - Timestamp and sequence number
  - Content hash (SHA-256)
  - Previous entry hash (for chaining)
  - Merkle tree reference (which block contains this entry)
- [ ] Merkle-tree block and rotation
  - New block created every N entries (e.g., 10k entries per block)
  - Block sealed with root hash signature (HMAC-SHA256)
  - Chain of blocks: each block contains hash of previous block
  - Rotation event logged and audited
- [ ] Cognitive journaling
  - Record all memory writes with reasoning context
  - Record all checkpoint operations (create, restore, delete)
  - Record context (which request/decision triggered the write)
  - Query interface: fetch journal entries by time range, type, agent
- [ ] Tamper detection system
  - Periodically verify Merkle-tree integrity
  - Re-hash all entries; verify against stored hashes
  - Detect and log any mismatches
  - Alert on tampering detection
- [ ] Query and audit APIs
  - Get entry by sequence number or timestamp
  - Get entries by type (filter)
  - Verify proof for specific entry (prove entry is in tree)
  - Export time-range of entries with integrity proof
- [ ] Integration with telemetry
  - Emit AuditLogEntry events on entry creation
  - Link to PolicyDecision, Tool calls, memory operations
  - Cost attribution for audit logging
- [ ] Unit and integration tests
  - Merkle-tree correctness (hash verification, proof generation)
  - Tamper detection (catch hash modifications)
  - Block rotation and sealing
  - Query functionality and filtering
  - Cognitive journaling completeness

## Technical Specifications

### Merkle-Tree Audit Log
```rust
pub struct MerkleAuditLog {
    blocks: Arc<RwLock<Vec<MerkleBlock>>>,
    current_entries: Arc<Mutex<VecDeque<AuditLogEntry>>>,
    entries_per_block: usize,
    block_seal_key: String, // HMAC-SHA256 key
}

pub struct MerkleBlock {
    pub block_id: String,
    pub block_number: u64,
    pub entries: Vec<AuditLogEntry>,
    pub merkle_root: String,
    pub previous_block_hash: String,
    pub block_hash: String, // HMAC(merkle_root + previous_block_hash)
    pub sealed_at: i64,
    pub seal_signature: String,
}

pub struct AuditLogEntry {
    pub sequence_number: u64,
    pub timestamp: i64,
    pub entry_type: AuditLogEntryType,
    pub content: serde_json::Value,
    pub content_hash: String,
    pub previous_entry_hash: String,
    pub merkle_proof: Option<Vec<String>>, // Path to root
}

pub enum AuditLogEntryType {
    PolicyDecision,
    ToolInvocation,
    MemoryWrite,
    CheckpointCreate,
    CheckpointRestore,
    IPCMessage,
    Checkpoint,
    ExceptionRaised,
}

pub struct MerkleNode {
    pub hash: String,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
    pub is_leaf: bool,
}

impl MerkleAuditLog {
    pub fn new(entries_per_block: usize, seal_key: String) -> Self {
        Self {
            blocks: Arc::new(RwLock::new(Vec::new())),
            current_entries: Arc::new(Mutex::new(VecDeque::new())),
            entries_per_block,
            block_seal_key: seal_key,
        }
    }

    pub async fn append_entry(&self, entry_type: AuditLogEntryType,
                             content: serde_json::Value) -> Result<u64, LogError>
    {
        let mut entries = self.current_entries.lock().await;
        let sequence_number = (self.blocks.read().await.len() as u64 * self.entries_per_block as u64)
            + (entries.len() as u64);

        let content_hash = self.compute_hash(&content)?;
        let previous_hash = entries.back()
            .map(|e| e.content_hash.clone())
            .unwrap_or_default();

        let entry = AuditLogEntry {
            sequence_number,
            timestamp: now(),
            entry_type,
            content,
            content_hash,
            previous_entry_hash: previous_hash,
            merkle_proof: None,
        };

        entries.push_back(entry.clone());

        // Check if we need to seal a block
        if entries.len() >= self.entries_per_block {
            self.seal_block(entries.drain(..).collect()).await?;
        }

        Ok(sequence_number)
    }

    async fn seal_block(&self, entries: Vec<AuditLogEntry>) -> Result<(), LogError> {
        let merkle_root = self.build_merkle_tree(&entries)?;

        let mut blocks = self.blocks.write().await;
        let block_number = blocks.len() as u64;
        let previous_block_hash = blocks.last()
            .map(|b| b.block_hash.clone())
            .unwrap_or_default();

        let block_hash = self.compute_block_hash(&merkle_root, &previous_block_hash)?;

        let seal_signature = self.sign_block_seal(&merkle_root)?;

        let block = MerkleBlock {
            block_id: format!("block-{}", block_number),
            block_number,
            entries,
            merkle_root,
            previous_block_hash,
            block_hash,
            sealed_at: now(),
            seal_signature,
        };

        blocks.push(block);
        Ok(())
    }

    fn build_merkle_tree(&self, entries: &[AuditLogEntry]) -> Result<String, LogError> {
        if entries.is_empty() {
            return Ok(String::new());
        }

        let mut leaf_nodes: Vec<MerkleNode> = entries.iter()
            .map(|e| MerkleNode {
                hash: e.content_hash.clone(),
                left: None,
                right: None,
                is_leaf: true,
            })
            .collect();

        while leaf_nodes.len() > 1 {
            let mut parent_nodes = Vec::new();

            for i in (0..leaf_nodes.len()).step_by(2) {
                let left = leaf_nodes[i].clone();
                let right = if i + 1 < leaf_nodes.len() {
                    leaf_nodes[i + 1].clone()
                } else {
                    left.clone() // Duplicate if odd number
                };

                let combined_hash = format!("{}{}", left.hash, right.hash);
                let parent_hash = self.hash_string(&combined_hash).unwrap_or_default();

                parent_nodes.push(MerkleNode {
                    hash: parent_hash,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                    is_leaf: false,
                });
            }

            leaf_nodes = parent_nodes;
        }

        Ok(leaf_nodes.first().map(|n| n.hash.clone()).unwrap_or_default())
    }

    fn compute_hash(&self, content: &serde_json::Value) -> Result<String, LogError> {
        self.hash_string(&content.to_string())
    }

    fn hash_string(&self, data: &str) -> Result<String, LogError> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn compute_block_hash(&self, merkle_root: &str, previous_hash: &str)
        -> Result<String, LogError>
    {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.block_seal_key.as_bytes())
            .map_err(|_| LogError::SigningError)?;
        mac.update(format!("{}{}", merkle_root, previous_hash).as_bytes());
        Ok(format!("{:x}", mac.finalize().into_bytes()))
    }

    fn sign_block_seal(&self, merkle_root: &str) -> Result<String, LogError> {
        // HMAC signature for block seal
        self.compute_block_hash(merkle_root, "SEAL")
    }

    pub async fn verify_integrity(&self) -> Result<bool, LogError> {
        let blocks = self.blocks.read().await;

        for (idx, block) in blocks.iter().enumerate() {
            // Recompute merkle root
            let recomputed_root = self.build_merkle_tree(&block.entries)?;
            if recomputed_root != block.merkle_root {
                eprintln!("Tamper detected in block {}: merkle root mismatch", idx);
                return Ok(false);
            }

            // Verify block hash
            let recomputed_block_hash = self.compute_block_hash(&block.merkle_root, &block.previous_block_hash)?;
            if recomputed_block_hash != block.block_hash {
                eprintln!("Tamper detected in block {}: block hash mismatch", idx);
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn get_entry(&self, sequence_number: u64) -> Result<AuditLogEntry, LogError> {
        let blocks = self.blocks.read().await;

        for block in blocks.iter() {
            for entry in &block.entries {
                if entry.sequence_number == sequence_number {
                    return Ok(entry.clone());
                }
            }
        }

        Err(LogError::EntryNotFound)
    }

    pub async fn get_entries_by_type(&self, entry_type: AuditLogEntryType,
                                    start_time: i64, end_time: i64)
        -> Result<Vec<AuditLogEntry>, LogError>
    {
        let blocks = self.blocks.read().await;
        let mut results = Vec::new();

        for block in blocks.iter() {
            for entry in &block.entries {
                if std::mem::discriminant(&entry.entry_type) == std::mem::discriminant(&entry_type)
                    && entry.timestamp >= start_time && entry.timestamp <= end_time
                {
                    results.push(entry.clone());
                }
            }
        }

        Ok(results)
    }

    pub async fn export_with_integrity_proof(&self, output_path: &Path) -> Result<(), LogError> {
        let blocks = self.blocks.read().await;
        let export = serde_json::json!({
            "blocks": *blocks,
            "integrity_verified": self.verify_integrity().await?,
        });

        tokio::fs::write(output_path, export.to_string()).await
            .map_err(|e| LogError::IoError(e.to_string()))?;
        Ok(())
    }
}
```

### Cognitive Journaling
```rust
pub struct CognitiveJournal {
    audit_log: Arc<MerkleAuditLog>,
}

pub struct MemoryWriteJournalEntry {
    pub address: String,
    pub size_bytes: u64,
    pub data_hash: String,
    pub requesting_agent: String,
    pub requesting_decision: Option<String>, // Decision ID that triggered write
    pub checkpoint_ref: Option<String>,
    pub reasoning_context: String, // Why was this write necessary?
}

pub struct CheckpointJournalEntry {
    pub checkpoint_id: String,
    pub operation: CheckpointOperation,
    pub cpu_state_committed: bool,
    pub gpu_state_committed: bool,
    pub memory_committed: u64,
    pub triggered_by: Option<String>, // Request/decision ID
}

pub enum CheckpointOperation {
    Create,
    Restore,
    Delete,
}

impl CognitiveJournal {
    pub async fn journal_memory_write(&self, entry: &MemoryWriteJournalEntry)
        -> Result<u64, JournalError>
    {
        let content = serde_json::to_value(entry)?;
        self.audit_log.append_entry(
            AuditLogEntryType::MemoryWrite,
            content
        ).await.map_err(|e| JournalError::LogError(e))
    }

    pub async fn journal_checkpoint_operation(&self, entry: &CheckpointJournalEntry)
        -> Result<u64, JournalError>
    {
        let entry_type = match entry.operation {
            CheckpointOperation::Create => AuditLogEntryType::CheckpointCreate,
            CheckpointOperation::Restore => AuditLogEntryType::CheckpointRestore,
            CheckpointOperation::Delete => AuditLogEntryType::Checkpoint,
        };

        let content = serde_json::to_value(entry)?;
        self.audit_log.append_entry(entry_type, content)
            .await
            .map_err(|e| JournalError::LogError(e))
    }

    pub async fn get_journal_entries_for_decision(&self, decision_id: &str,
                                                  time_window_secs: i64)
        -> Result<Vec<AuditLogEntry>, JournalError>
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Get entries within time window for this decision
        self.audit_log.get_entries_by_type(
            AuditLogEntryType::MemoryWrite,
            now - time_window_secs,
            now
        ).await.map_err(|e| JournalError::LogError(e))
    }
}
```

## Dependencies
- **Blocked by:** Week 15-16 (PolicyDecision foundation)
- **Blocking:** Week 18-19 (Compliance Engine completion, retention policies)

## Acceptance Criteria
- [ ] Merkle-tree implementation functional; hash verification working
- [ ] Audit log entries with chain hashing (previous_entry_hash)
- [ ] Block sealing and HMAC-SHA256 signatures
- [ ] Merkle-tree proof generation for entries
- [ ] Tamper detection algorithm detects hash modifications
- [ ] Cognitive journaling for memory writes and checkpoint operations
- [ ] Query interface: get by sequence, by type, by time range
- [ ] Integrity verification periodic check
- [ ] Export with integrity proof functional
- [ ] Unit tests: Merkle-tree correctness, tamper detection, query API
- [ ] Integration tests: append, seal, verify, query

## Design Principles Alignment
- **Immutability:** Entries append-only; no retroactive modification
- **Tamper-evident:** Merkle-tree detects any data corruption
- **Auditability:** Chain structure preserves causality and ordering
- **Traceability:** Cognitive journaling links memory changes to decisions
- **Compliance:** Cryptographic proofs support regulatory audits
