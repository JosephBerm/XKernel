# Week 17: Compliance Engine - Merkle-Tree Audit Log Implementation
## XKernal Cognitive Substrate OS | L1 Services Layer (Rust)

**Date:** 2026-03-02
**Phase:** Phase 2 Continuation
**Objective:** Establish cryptographic foundation for regulatory compliance through tamper-evident audit logging

---

## 1. Architecture Overview

The Merkle-tree audit log provides a cryptographic foundation for tamper-evident, append-only record keeping across all cognitive substrate operations. This implementation enables regulatory compliance by creating an immutable audit trail with cryptographic proof of integrity.

### 1.1 Core Components

- **Merkle Tree**: SHA-256 based binary tree for efficient proof generation
- **Audit Log Entry**: Typed schema capturing cognitive operations with contextual metadata
- **Block Structure**: Sealed containers with HMAC-SHA256 binding (N entries per block)
- **Cognitive Journal**: Memory operation tracking with reasoning context
- **Tamper Detection**: Periodic integrity verification with mismatch alerts
- **Query APIs**: Range queries, filtering, proof verification, export capabilities

---

## 2. Merkle Tree Data Structure

### 2.1 Tree Architecture

```rust
use sha2::{Sha256, Digest};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MerkleNode {
    Leaf {
        hash: [u8; 32],
        entry_index: usize,
    },
    Internal {
        hash: [u8; 32],
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },
}

impl MerkleNode {
    pub fn hash(&self) -> [u8; 32] {
        match self {
            MerkleNode::Leaf { hash, .. } => *hash,
            MerkleNode::Internal { hash, .. } => *hash,
        }
    }
}

pub struct MerkleTree {
    root: Option<Box<MerkleNode>>,
    leaf_count: usize,
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Creates a new empty Merkle tree
    pub fn new() -> Self {
        MerkleTree {
            root: None,
            leaf_count: 0,
            leaves: Vec::new(),
        }
    }

    /// Appends leaf hash to tree and rebuilds from leaves
    pub fn append_leaf(&mut self, leaf_hash: [u8; 32]) {
        self.leaves.push(leaf_hash);
        self.leaf_count += 1;
        self.rebuild_tree();
    }

    /// Rebuilds tree structure from current leaf hashes
    fn rebuild_tree(&mut self) {
        if self.leaves.is_empty() {
            self.root = None;
            return;
        }

        let mut current_level: Vec<MerkleNode> = self
            .leaves
            .iter()
            .enumerate()
            .map(|(idx, hash)| MerkleNode::Leaf {
                hash: *hash,
                entry_index: idx,
            })
            .collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i].clone();

                let right = if i + 1 < current_level.len() {
                    current_level[i + 1].clone()
                } else {
                    left.clone()
                };

                let combined = Self::hash_nodes(left.hash(), right.hash());

                next_level.push(MerkleNode::Internal {
                    hash: combined,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }

            current_level = next_level;
        }

        self.root = current_level.into_iter().next().map(Box::new);
    }

    /// Computes SHA-256 hash of two node hashes
    fn hash_nodes(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&left);
        hasher.update(&right);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }

    /// Generates proof path for leaf at given index
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaf_count || self.root.is_none() {
            return None;
        }

        let mut path = Vec::new();
        self.collect_proof_path(&self.root, leaf_index, 0, self.leaf_count, &mut path);

        Some(MerkleProof {
            leaf_index,
            leaf_hash: self.leaves[leaf_index],
            proof_path: path,
            tree_size: self.leaf_count,
        })
    }

    /// Recursively collects proof hashes along path to leaf
    fn collect_proof_path(
        &self,
        node: &Option<Box<MerkleNode>>,
        target_idx: usize,
        range_start: usize,
        range_size: usize,
        path: &mut Vec<ProofElement>,
    ) {
        let Some(n) = node else { return };

        match &**n {
            MerkleNode::Leaf { .. } => {}
            MerkleNode::Internal { left, right, .. } => {
                let left_size = Self::subtree_size(range_size / 2);
                let mid = range_start + left_size;

                if target_idx < mid {
                    path.push(ProofElement::Right(right.hash()));
                    self.collect_proof_path(&Some(left.clone()), target_idx, range_start, left_size, path);
                } else {
                    path.push(ProofElement::Left(left.hash()));
                    self.collect_proof_path(&Some(right.clone()), target_idx, mid, range_size - left_size, path);
                }
            }
        }
    }

    /// Calculates subtree node count for balanced tree
    fn subtree_size(range_size: usize) -> usize {
        if range_size <= 1 {
            return 1;
        }
        let next_power = (range_size as u64).next_power_of_two() as usize;
        next_power / 2
    }

    /// Verifies proof against known root
    pub fn verify_proof(proof: &MerkleProof, root_hash: [u8; 32]) -> bool {
        let mut computed = proof.leaf_hash;

        for element in &proof.proof_path {
            computed = match element {
                ProofElement::Left(sibling) => Self::hash_nodes(computed, *sibling),
                ProofElement::Right(sibling) => Self::hash_nodes(*sibling, computed),
            };
        }

        computed == root_hash
    }

    /// Returns current root hash
    pub fn root_hash(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|n| n.hash())
    }

    /// Returns total leaf count
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }
}

#[derive(Clone, Debug)]
pub enum ProofElement {
    Left([u8; 32]),
    Right([u8; 32]),
}

#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_hash: [u8; 32],
    pub proof_path: Vec<ProofElement>,
    pub tree_size: usize,
}

impl fmt::Display for MerkleProof {
    fn fmt(&self, f: &mut fmt::Fmt) -> fmt::Result {
        write!(
            f,
            "MerkleProof(index={}, hash={}, path_len={})",
            self.leaf_index,
            hex::encode(&self.leaf_hash[..8]),
            self.proof_path.len()
        )
    }
}
```

---

## 3. Audit Log Entry Schema

### 3.1 Type-Safe Entry Design

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum AuditLogEntryType {
    PolicyDecision,
    ToolInvocation,
    MemoryWrite,
    CheckpointCreate,
    CheckpointRestore,
    IPCMessage,
    ExceptionRaised,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Monotonically increasing sequence number within block
    pub sequence_number: u64,

    /// Entry type classification
    pub entry_type: AuditLogEntryType,

    /// UTC timestamp of operation
    pub timestamp: DateTime<Utc>,

    /// SHA-256 hash of entry content
    pub content_hash: [u8; 32],

    /// SHA-256 hash of previous entry (chain binding)
    pub previous_entry_hash: [u8; 32],

    /// Merkle proof for this entry in block tree
    pub merkle_proof: Option<MerkleProof>,

    /// Serialized operation context
    pub payload: serde_json::Value,

    /// Cognitive reasoning attached to operation
    pub reasoning_context: Option<String>,

    /// Compliance metadata
    pub regulatory_refs: Vec<String>,
}

impl AuditLogEntry {
    /// Creates new audit log entry with computed hashes
    pub fn new(
        sequence_number: u64,
        entry_type: AuditLogEntryType,
        payload: serde_json::Value,
        previous_entry_hash: [u8; 32],
    ) -> Self {
        let content_hash = Self::compute_content_hash(&payload);

        AuditLogEntry {
            sequence_number,
            entry_type,
            timestamp: Utc::now(),
            content_hash,
            previous_entry_hash,
            merkle_proof: None,
            payload,
            reasoning_context: None,
            regulatory_refs: Vec::new(),
        }
    }

    /// Computes SHA-256 hash of entry content
    fn compute_content_hash(payload: &serde_json::Value) -> [u8; 32] {
        let serialized = serde_json::to_vec(payload).expect("JSON serialization");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }

    /// Validates entry integrity
    pub fn validate(&self) -> Result<(), String> {
        let recomputed = Self::compute_content_hash(&self.payload);
        if recomputed != self.content_hash {
            return Err("Content hash mismatch".to_string());
        }
        Ok(())
    }
}
```

---

## 4. Block Rotation and HMAC Sealing

### 4.1 Block Structure

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditBlock {
    /// Block sequence number
    pub block_number: u64,

    /// Entries within this block
    pub entries: Vec<AuditLogEntry>,

    /// Merkle root of all entries in block
    pub merkle_root: [u8; 32],

    /// HMAC-SHA256 seal using block secret key
    pub block_seal: [u8; 32],

    /// Hash of previous block (chain binding)
    pub previous_block_hash: [u8; 32],

    /// Timestamp when block was sealed
    pub sealed_at: DateTime<Utc>,

    /// Block rotation parameters
    pub max_entries: usize,
}

pub struct AuditBlockManager {
    current_block: AuditBlock,
    block_secret_key: [u8; 32],
    entry_count: u64,
    blocks: Vec<AuditBlock>,
}

impl AuditBlockManager {
    const DEFAULT_MAX_ENTRIES: usize = 1000;

    /// Initializes block manager with cryptographic key
    pub fn new(block_secret_key: [u8; 32]) -> Self {
        AuditBlockManager {
            current_block: AuditBlock {
                block_number: 0,
                entries: Vec::new(),
                merkle_root: [0u8; 32],
                block_seal: [0u8; 32],
                previous_block_hash: [0u8; 32],
                sealed_at: Utc::now(),
                max_entries: Self::DEFAULT_MAX_ENTRIES,
            },
            block_secret_key,
            entry_count: 0,
            blocks: Vec::new(),
        }
    }

    /// Appends entry to current block, rotating if necessary
    pub fn append_entry(&mut self, mut entry: AuditLogEntry) -> Result<(), String> {
        entry.sequence_number = self.entry_count;
        self.entry_count += 1;

        if self.current_block.entries.len() >= self.current_block.max_entries {
            self.rotate_block()?;
        }

        self.current_block.entries.push(entry);
        Ok(())
    }

    /// Rotates to new block, sealing current block
    fn rotate_block(&mut self) -> Result<(), String> {
        if self.current_block.entries.is_empty() {
            return Ok(());
        }

        // Rebuild Merkle tree for current block
        let mut tree = MerkleTree::new();
        for entry in &self.current_block.entries {
            tree.append_leaf(entry.content_hash);
        }

        self.current_block.merkle_root = tree.root_hash().ok_or("Empty tree")?;

        // Compute HMAC-SHA256 block seal
        let block_seal = self.compute_block_seal(&self.current_block)?;
        self.current_block.block_seal = block_seal;
        self.current_block.sealed_at = Utc::now();

        // Chain to previous block
        if !self.blocks.is_empty() {
            let prev = &self.blocks[self.blocks.len() - 1];
            self.current_block.previous_block_hash = Self::hash_block(prev);
        }

        self.blocks.push(self.current_block.clone());

        // Initialize new block
        self.current_block = AuditBlock {
            block_number: self.current_block.block_number + 1,
            entries: Vec::new(),
            merkle_root: [0u8; 32],
            block_seal: [0u8; 32],
            previous_block_hash: [0u8; 32],
            sealed_at: Utc::now(),
            max_entries: Self::DEFAULT_MAX_ENTRIES,
        };

        Ok(())
    }

    /// Computes HMAC-SHA256 seal for block
    fn compute_block_seal(&self, block: &AuditBlock) -> Result<[u8; 32], String> {
        let mut mac = HmacSha256::new_from_slice(&self.block_secret_key)
            .map_err(|_| "Invalid key size")?;

        mac.update(&block.block_number.to_le_bytes());
        mac.update(&block.merkle_root);
        mac.update(&block.previous_block_hash);

        let result = mac.finalize().into_bytes();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        Ok(output)
    }

    /// Computes SHA-256 hash of entire block
    fn hash_block(block: &AuditBlock) -> [u8; 32] {
        let serialized = serde_json::to_vec(block).expect("JSON serialization");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }

    /// Verifies block seal integrity
    pub fn verify_block_seal(&self, block: &AuditBlock) -> Result<bool, String> {
        let expected_seal = self.compute_block_seal(block)?;
        Ok(block.block_seal == expected_seal)
    }
}
```

---

## 5. Cognitive Journaling

### 5.1 Memory Write Tracking

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CognitiveJournalEntry {
    /// Memory location or reference
    pub memory_ref: String,

    /// Previous value before write
    pub previous_value: serde_json::Value,

    /// New value after write
    pub new_value: serde_json::Value,

    /// Reasoning context explaining the write
    pub reasoning: String,

    /// Checkpoint this write belongs to
    pub checkpoint_id: Option<String>,

    /// Causal context (what triggered this write)
    pub causal_context: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointRecord {
    /// Unique checkpoint identifier
    pub checkpoint_id: String,

    /// Memory snapshot at checkpoint
    pub memory_snapshot: serde_json::Value,

    /// All journal entries since previous checkpoint
    pub journal_entries: Vec<CognitiveJournalEntry>,

    /// Merkle root of journal entries
    pub journal_merkle_root: [u8; 32],

    /// Timestamp of checkpoint
    pub created_at: DateTime<Utc>,

    /// Parent checkpoint for restore chain
    pub parent_checkpoint_id: Option<String>,
}

pub struct CognitiveJournal {
    current_entries: Vec<CognitiveJournalEntry>,
    checkpoints: Vec<CheckpointRecord>,
    journal_tree: MerkleTree,
}

impl CognitiveJournal {
    pub fn new() -> Self {
        CognitiveJournal {
            current_entries: Vec::new(),
            checkpoints: Vec::new(),
            journal_tree: MerkleTree::new(),
        }
    }

    /// Records memory write with reasoning
    pub fn record_write(
        &mut self,
        memory_ref: String,
        previous: serde_json::Value,
        new_value: serde_json::Value,
        reasoning: String,
    ) -> Result<(), String> {
        let entry = CognitiveJournalEntry {
            memory_ref,
            previous_value: previous,
            new_value,
            reasoning,
            checkpoint_id: None,
            causal_context: None,
        };

        let entry_hash = Self::hash_entry(&entry);
        self.journal_tree.append_leaf(entry_hash);
        self.current_entries.push(entry);

        Ok(())
    }

    /// Creates checkpoint of current state
    pub fn create_checkpoint(
        &mut self,
        checkpoint_id: String,
        memory_snapshot: serde_json::Value,
    ) -> Result<CheckpointRecord, String> {
        let journal_merkle_root = self
            .journal_tree
            .root_hash()
            .ok_or("Empty journal tree")?;

        let parent_id = self.checkpoints.last().map(|c| c.checkpoint_id.clone());

        let checkpoint = CheckpointRecord {
            checkpoint_id,
            memory_snapshot,
            journal_entries: self.current_entries.clone(),
            journal_merkle_root,
            created_at: Utc::now(),
            parent_checkpoint_id: parent_id,
        };

        self.checkpoints.push(checkpoint.clone());
        self.current_entries.clear();
        self.journal_tree = MerkleTree::new();

        Ok(checkpoint)
    }

    /// Restores to previous checkpoint
    pub fn restore_checkpoint(&mut self, checkpoint_id: &str) -> Result<serde_json::Value, String> {
        let checkpoint = self
            .checkpoints
            .iter()
            .find(|c| c.checkpoint_id == checkpoint_id)
            .ok_or("Checkpoint not found")?;

        self.current_entries.clear();
        self.journal_tree = MerkleTree::new();

        Ok(checkpoint.memory_snapshot.clone())
    }

    fn hash_entry(entry: &CognitiveJournalEntry) -> [u8; 32] {
        let serialized = serde_json::to_vec(entry).expect("JSON serialization");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }
}
```

---

## 6. Tamper Detection and Integrity Verification

### 6.1 Verification Engine

```rust
#[derive(Clone, Debug)]
pub struct TamperDetectionReport {
    pub verified: bool,
    pub total_entries: usize,
    pub failed_entries: Vec<(usize, String)>,
    pub block_chain_valid: bool,
    pub seal_verification_status: Vec<(u64, bool)>,
    pub timestamp: DateTime<Utc>,
}

pub struct TamperDetectionEngine {
    block_manager: AuditBlockManager,
    last_verification: Option<DateTime<Utc>>,
}

impl TamperDetectionEngine {
    pub fn new(block_manager: AuditBlockManager) -> Self {
        TamperDetectionEngine {
            block_manager,
            last_verification: None,
        }
    }

    /// Performs complete integrity verification on audit log
    pub fn verify_integrity(&mut self) -> Result<TamperDetectionReport, String> {
        let mut failed_entries = Vec::new();
        let mut seal_verification_status = Vec::new();

        // Verify all entry hashes
        for block in &self.block_manager.blocks {
            for entry in &block.entries {
                if let Err(e) = entry.validate() {
                    failed_entries.push((entry.sequence_number as usize, e));
                }
            }

            // Verify block seal
            match self.block_manager.verify_block_seal(block) {
                Ok(valid) => seal_verification_status.push((block.block_number, valid)),
                Err(e) => seal_verification_status.push((block.block_number, false)),
            }
        }

        // Verify block chain integrity
        let block_chain_valid = self.verify_block_chain();

        let total_entries: usize = self
            .block_manager
            .blocks
            .iter()
            .map(|b| b.entries.len())
            .sum();

        let verified = failed_entries.is_empty() && block_chain_valid && seal_verification_status.iter().all(|(_, v)| *v);

        self.last_verification = Some(Utc::now());

        Ok(TamperDetectionReport {
            verified,
            total_entries,
            failed_entries,
            block_chain_valid,
            seal_verification_status,
            timestamp: Utc::now(),
        })
    }

    /// Verifies block chain hash bindings
    fn verify_block_chain(&self) -> bool {
        for i in 1..self.block_manager.blocks.len() {
            let current = &self.block_manager.blocks[i];
            let previous = &self.block_manager.blocks[i - 1];
            let expected_prev_hash = AuditBlockManager::hash_block(previous);

            if current.previous_block_hash != expected_prev_hash {
                return false;
            }
        }
        true
    }

    /// Detects and reports hash mismatches
    pub fn detect_mismatches(&self) -> Result<Vec<(usize, String)>, String> {
        let mut mismatches = Vec::new();

        for block in &self.block_manager.blocks {
            for entry in &block.entries {
                if let Err(e) = entry.validate() {
                    mismatches.push((entry.sequence_number as usize, e));
                }
            }
        }

        Ok(mismatches)
    }
}
```

---

## 7. Query and Audit APIs

### 7.1 Range Queries and Export

```rust
#[derive(Clone, Debug)]
pub struct AuditQuery {
    pub from_sequence: Option<u64>,
    pub to_sequence: Option<u64>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub entry_types: Vec<AuditLogEntryType>,
}

pub struct AuditLogQueryEngine {
    block_manager: AuditBlockManager,
}

impl AuditLogQueryEngine {
    pub fn new(block_manager: AuditBlockManager) -> Self {
        AuditLogQueryEngine { block_manager }
    }

    /// Executes range query across blocks
    pub fn query_range(&self, query: &AuditQuery) -> Result<Vec<AuditLogEntry>, String> {
        let mut results = Vec::new();

        for block in &self.block_manager.blocks {
            for entry in &block.entries {
                if self.matches_query(entry, query) {
                    results.push(entry.clone());
                }
            }
        }

        Ok(results)
    }

    /// Filters entries by type
    pub fn filter_by_type(
        &self,
        entry_type: AuditLogEntryType,
    ) -> Result<Vec<AuditLogEntry>, String> {
        let query = AuditQuery {
            from_sequence: None,
            to_sequence: None,
            from_time: None,
            to_time: None,
            entry_types: vec![entry_type],
        };
        self.query_range(&query)
    }

    /// Retrieves entry by sequence number
    pub fn get_by_sequence(&self, seq: u64) -> Result<Option<AuditLogEntry>, String> {
        for block in &self.block_manager.blocks {
            for entry in &block.entries {
                if entry.sequence_number == seq {
                    return Ok(Some(entry.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Retrieves entries within time window
    pub fn get_by_timestamp(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<AuditLogEntry>, String> {
        let query = AuditQuery {
            from_sequence: None,
            to_sequence: None,
            from_time: Some(from),
            to_time: Some(to),
            entry_types: Vec::new(),
        };
        self.query_range(&query)
    }

    /// Verifies Merkle proof for entry
    pub fn verify_proof(&self, entry: &AuditLogEntry, block_number: u64) -> Result<bool, String> {
        let block = self
            .block_manager
            .blocks
            .iter()
            .find(|b| b.block_number == block_number)
            .ok_or("Block not found")?;

        if let Some(proof) = &entry.merkle_proof {
            Ok(MerkleTree::verify_proof(proof, block.merkle_root))
        } else {
            Ok(false)
        }
    }

    /// Exports audit log with integrity proofs
    pub fn export_with_integrity(
        &self,
        format: ExportFormat,
    ) -> Result<Vec<u8>, String> {
        let export = AuditLogExport {
            blocks: self.block_manager.blocks.clone(),
            export_time: Utc::now(),
            total_entries: self.total_entry_count(),
        };

        match format {
            ExportFormat::Json => Ok(serde_json::to_vec_pretty(&export)
                .map_err(|e| format!("JSON serialization error: {}", e))?),
            ExportFormat::Binary => {
                bincode::serialize(&export).map_err(|e| format!("Binary serialization error: {}", e))
            }
        }
    }

    fn matches_query(&self, entry: &AuditLogEntry, query: &AuditQuery) -> bool {
        if let Some(from) = query.from_sequence {
            if entry.sequence_number < from {
                return false;
            }
        }
        if let Some(to) = query.to_sequence {
            if entry.sequence_number > to {
                return false;
            }
        }
        if let Some(from) = query.from_time {
            if entry.timestamp < from {
                return false;
            }
        }
        if let Some(to) = query.to_time {
            if entry.timestamp > to {
                return false;
            }
        }
        if !query.entry_types.is_empty() && !query.entry_types.contains(&entry.entry_type) {
            return false;
        }
        true
    }

    fn total_entry_count(&self) -> usize {
        self.block_manager.blocks.iter().map(|b| b.entries.len()).sum()
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuditLogExport {
    pub blocks: Vec<AuditBlock>,
    pub export_time: DateTime<Utc>,
    pub total_entries: usize,
}

pub enum ExportFormat {
    Json,
    Binary,
}
```

---

## 8. Integration with Telemetry

### 8.1 Policy Decision and Tool Invocation Logging

```rust
pub struct TelemetryAuditBridge {
    log_manager: AuditBlockManager,
    journal: CognitiveJournal,
}

impl TelemetryAuditBridge {
    pub fn new(log_manager: AuditBlockManager, journal: CognitiveJournal) -> Self {
        TelemetryAuditBridge { log_manager, journal }
    }

    /// Records policy decision to audit log
    pub fn audit_policy_decision(
        &mut self,
        policy_id: String,
        decision: String,
        reasoning: String,
    ) -> Result<(), String> {
        let payload = json!({
            "policy_id": policy_id,
            "decision": decision,
            "reasoning": reasoning,
        });

        let entry = AuditLogEntry::new(
            0,
            AuditLogEntryType::PolicyDecision,
            payload,
            [0u8; 32],
        );

        self.log_manager.append_entry(entry)?;
        Ok(())
    }

    /// Records tool invocation with context
    pub fn audit_tool_invocation(
        &mut self,
        tool_name: String,
        args: serde_json::Value,
        result: serde_json::Value,
    ) -> Result<(), String> {
        let payload = json!({
            "tool_name": tool_name,
            "arguments": args,
            "result": result,
        });

        let entry = AuditLogEntry::new(
            0,
            AuditLogEntryType::ToolInvocation,
            payload,
            [0u8; 32],
        );

        self.log_manager.append_entry(entry)?;
        Ok(())
    }

    /// Records memory write operation
    pub fn audit_memory_write(
        &mut self,
        memory_ref: String,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
        reasoning: String,
    ) -> Result<(), String> {
        self.journal.record_write(memory_ref, old_value, new_value, reasoning)?;
        Ok(())
    }

    /// Records exception with context
    pub fn audit_exception(
        &mut self,
        exception_type: String,
        message: String,
        backtrace: Option<String>,
    ) -> Result<(), String> {
        let payload = json!({
            "exception_type": exception_type,
            "message": message,
            "backtrace": backtrace,
        });

        let entry = AuditLogEntry::new(
            0,
            AuditLogEntryType::ExceptionRaised,
            payload,
            [0u8; 32],
        );

        self.log_manager.append_entry(entry)?;
        Ok(())
    }
}
```

---

## 9. Conclusion and Next Steps

The Merkle-tree audit log provides the cryptographic foundation for regulatory compliance in the XKernal Cognitive Substrate OS. This implementation delivers:

- **Tamper-evident records** through cryptographic hashing and chaining
- **Efficient proof generation** via Merkle trees (O(log n) proof size)
- **Block rotation** with HMAC-SHA256 sealing for audit integrity
- **Cognitive journaling** with checkpoint/restore capabilities
- **Comprehensive querying** and export with proof verification
- **Integration points** for policy decisions, tool invocations, and memory operations

Week 18 will focus on persistent storage backend integration and compliance reporting workflows.

