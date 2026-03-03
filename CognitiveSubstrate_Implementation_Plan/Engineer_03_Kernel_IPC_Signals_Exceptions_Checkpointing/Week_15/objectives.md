# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 15

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Implement checkpoint migration support: enable CT migration between machines via checkpoint export, network transfer, and import on destination machine with full state restoration.

## Document References
- **Primary:** Section 3.2.7 (Checkpointing Engine)
- **Supporting:** Section 2.9 (Cognitive Checkpointing Engine), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] ExportableCheckpoint: serialize checkpoint to binary format with integrity hash
- [ ] CheckpointMigrationProtocol: network protocol for checkpoint transfer
- [ ] Checkpoint export: convert in-memory checkpoint to portable format
- [ ] Network transfer: send checkpoint via reliable channel to destination
- [ ] Checkpoint import: validate and restore checkpoint on destination machine
- [ ] CT migration: pause CT, export checkpoint, transfer, import, resume on destination
- [ ] Capability re-mapping: update capability references to destination machine
- [ ] IPC channel migration: migrate in-flight IPC channels to new CT location
- [ ] Integration tests: migrate CT between 2+ machines, verify state preservation
- [ ] Benchmark: measure checkpoint export/import time for varying checkpoint sizes

## Technical Specifications

### ExportableCheckpoint Format
```
pub struct ExportableCheckpoint {
    pub version: u32,                          // Format version for compatibility
    pub source_machine_id: MachineId,
    pub source_ct_id: ContextThreadId,
    pub timestamp: Timestamp,
    pub checkpoint: CognitiveCheckpoint,
    pub system_state: SystemStateSnapshot,    // Machine-specific state
    pub integrity_hash: Vec<u8>,               // SHA256 of all preceding fields
}

pub struct SystemStateSnapshot {
    pub os_info: String,                       // "Linux 6.8.0", etc.
    pub arch: String,                          // "x86_64", "aarch64", etc.
    pub memory_config: MemoryConfiguration,
    pub page_table_format: String,
}

impl ExportableCheckpoint {
    pub fn compute_integrity_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(bincode::serialize(&self.checkpoint).unwrap());
        hasher.update(self.timestamp.as_bytes());
        hasher.finalize().to_vec()
    }

    pub fn verify_integrity(&self) -> bool {
        self.integrity_hash == self.compute_integrity_hash()
    }
}
```

### Checkpoint Export
```
fn export_checkpoint(
    checkpoint: &CognitiveCheckpoint,
    source_machine_id: MachineId,
) -> Result<ExportableCheckpoint, ExportError> {
    // 1. Verify checkpoint integrity
    if !checkpoint.hash_chain_valid() {
        return Err(ExportError::CheckpointCorrupted);
    }

    // 2. Serialize checkpoint data
    let checkpoint_data = bincode::serialize(checkpoint)?;

    // 3. Gather system state
    let system_state = SystemStateSnapshot {
        os_info: os_info::os().to_string(),
        arch: std::env::consts::ARCH.to_string(),
        memory_config: get_memory_configuration(),
        page_table_format: "x86_64_4level".to_string(),
    };

    // 4. Create exportable checkpoint
    let mut exportable = ExportableCheckpoint {
        version: CHECKPOINT_FORMAT_VERSION,
        source_machine_id,
        source_ct_id: checkpoint.ct_ref.ct_id,
        timestamp: now(),
        checkpoint: checkpoint.clone(),
        system_state,
        integrity_hash: Vec::new(),
    };

    // 5. Compute integrity hash
    exportable.integrity_hash = exportable.compute_integrity_hash();

    Ok(exportable)
}
```

### Checkpoint Migration Protocol
```
pub enum MigrationMessage {
    MigrationRequest {
        ct_id: ContextThreadId,
        checkpoint_id: CheckpointId,
        dest_machine_id: MachineId,
    },
    CheckpointData {
        sequence: u32,
        chunk: Vec<u8>,
        total_chunks: u32,
    },
    MigrationAck {
        status: MigrationStatus,
        new_ct_id: Option<ContextThreadId>,
    },
    MigrationAbort {
        reason: String,
    },
}

pub enum MigrationStatus {
    InProgress,
    Complete,
    Failed(String),
}

pub struct CheckpointMigrationChannel {
    pub from_machine: MachineId,
    pub to_machine: MachineId,
    pub ct_id: ContextThreadId,
    pub checkpoint_id: CheckpointId,
    pub status: MigrationStatus,
    pub chunks_sent: u32,
    pub total_chunks: u32,
}
```

### Network Transfer
```
fn migrate_checkpoint_to_machine(
    ct: &mut ContextThread,
    checkpoint_id: CheckpointId,
    dest_machine_id: MachineId,
) -> Result<(), MigrationError> {
    // 1. Pause CT on source machine
    ct.pause_for_migration()?;

    // 2. Get checkpoint
    let checkpoint = ct.get_checkpoint(checkpoint_id)?;

    // 3. Export checkpoint
    let exportable = export_checkpoint(&checkpoint, get_local_machine_id())?;

    // 4. Connect to destination machine
    let dest_addr = lookup_machine_addr(dest_machine_id)?;
    let mut remote_conn = connect_to_machine(dest_addr)?;

    // 5. Send migration request
    let request = MigrationMessage::MigrationRequest {
        ct_id: ct.id,
        checkpoint_id,
        dest_machine_id,
    };
    remote_conn.send_message(&request)?;

    // 6. Serialize checkpoint to chunks
    let serialized = bincode::serialize(&exportable)?;
    let chunk_size = 1_000_000;  // 1MB chunks
    let total_chunks = (serialized.len() + chunk_size - 1) / chunk_size;

    for (i, chunk) in serialized.chunks(chunk_size).enumerate() {
        let msg = MigrationMessage::CheckpointData {
            sequence: i as u32,
            chunk: chunk.to_vec(),
            total_chunks: total_chunks as u32,
        };
        remote_conn.send_message(&msg)?;

        // Receive ack for each chunk
        let ack = remote_conn.receive_message::<MigrationMessage>()?;
        if let MigrationMessage::MigrationAck { status: MigrationStatus::Failed(e), .. } = ack {
            return Err(MigrationError::RemoteRejected(e));
        }
    }

    // 7. Wait for destination confirmation
    let final_ack = remote_conn.receive_message::<MigrationMessage>()?;
    match final_ack {
        MigrationMessage::MigrationAck { status: MigrationStatus::Complete, new_ct_id } => {
            ct.set_migrated_to(dest_machine_id, new_ct_id)?;
            Ok(())
        }
        _ => Err(MigrationError::DestinationFailed),
    }
}
```

### Checkpoint Import
```
fn import_checkpoint(
    exportable: &ExportableCheckpoint,
    dest_machine_id: MachineId,
) -> Result<ContextThreadId, ImportError> {
    // 1. Verify integrity
    if !exportable.verify_integrity() {
        return Err(ImportError::CorruptedCheckpoint);
    }

    // 2. Verify system compatibility
    verify_system_compatibility(&exportable.system_state)?;

    // 3. Validate checkpoint
    if !exportable.checkpoint.hash_chain_valid() {
        return Err(ImportError::InvalidCheckpoint);
    }

    // 4. Create new CT on destination machine
    let new_ct = ContextThread::new_from_checkpoint(&exportable.checkpoint)?;
    let new_ct_id = new_ct.id;

    // 5. Remap capabilities
    let remapped_caps = remap_capabilities_to_destination(&new_ct)?;
    new_ct.set_capabilities(remapped_caps)?;

    // 6. Update machine references
    new_ct.update_machine_id(dest_machine_id)?;

    // 7. Restore checkpoint state
    new_ct.restore_from_checkpoint(&exportable.checkpoint)?;

    // 8. Register new CT in kernel
    kernel_register_ct(new_ct)?;

    Ok(new_ct_id)
}

fn verify_system_compatibility(state: &SystemStateSnapshot) -> Result<(), CompatibilityError> {
    // Verify architecture matches
    if state.arch != std::env::consts::ARCH {
        return Err(CompatibilityError::ArchitectureMismatch);
    }

    // Verify OS is compatible
    // (allow some flexibility for kernel versions)

    Ok(())
}
```

### IPC Channel Migration
```
fn migrate_ipc_channels(
    ct: &ContextThread,
    old_ct_id: ContextThreadId,
    new_ct_id: ContextThreadId,
    dest_machine_id: MachineId,
) -> Result<(), MigrationError> {
    // Find all channels involving old CT
    let channels = find_channels_for_ct(old_ct_id);

    for channel in channels {
        match channel {
            SemanticChannel::RequestResponse(req_resp) => {
                // Update endpoint references
                if req_resp.endpoints.requestor.ct_id == old_ct_id {
                    req_resp.endpoints.requestor = ContextThreadRef {
                        machine_id: dest_machine_id,
                        ct_id: new_ct_id,
                    };
                } else {
                    req_resp.endpoints.requestee = ContextThreadRef {
                        machine_id: dest_machine_id,
                        ct_id: new_ct_id,
                    };
                }
            }
            SemanticChannel::PubSub(pubsub) => {
                // Update publisher or subscriber reference
                if pubsub.publisher.ct_id == old_ct_id {
                    pubsub.publisher.machine_id = dest_machine_id;
                    pubsub.publisher.ct_id = new_ct_id;
                }
                for sub in &mut pubsub.subscribers {
                    if sub.subscriber_id.ct_id == old_ct_id {
                        sub.subscriber_id.machine_id = dest_machine_id;
                        sub.subscriber_id.ct_id = new_ct_id;
                    }
                }
            }
            _ => {}  // Handle other channel types
        }
    }

    Ok(())
}
```

## Dependencies
- **Blocked by:** Week 1-14 (All Phase 0 & Phase 1 work)
- **Blocking:** Week 16 onwards (remaining Phase 2)

## Acceptance Criteria
1. ExportableCheckpoint includes all required fields and integrity hash
2. Checkpoint export preserves all state correctly
3. Network transfer handles large checkpoints (>1GB) via chunking
4. Checkpoint import validates integrity before restoration
5. System compatibility check prevents incompatible migrations
6. Capabilities remapped correctly to destination machine
7. IPC channels updated with new CT references
8. Multi-machine migration test passes
9. Checkpoint size accurately preserved (< 5% overhead)
10. Benchmark: export 100MB checkpoint in < 10ms, import in < 50ms

## Design Principles Alignment
- **Portability:** Checkpoint format is machine-independent
- **Safety:** Integrity hash prevents corruption detection
- **Compatibility:** System state verification prevents incompatible migrations
- **Transparency:** IPC channels automatically updated for new location
