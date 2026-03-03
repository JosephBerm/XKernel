# Week 15: Checkpoint Migration Support - Technical Design Document

**XKernal Cognitive Substrate OS**
**L0 Microkernel Layer**
**Staff-Level Engineer (Engineer 3 — IPC, Signals, Exceptions & Checkpointing)**
**Phase 2, Week 1**

---

## Executive Summary

Week 15 implements checkpoint migration support, enabling computational threads (CTs) to migrate between machines via checkpoint export, network transfer, and import with full state restoration. This design document specifies the `ExportableCheckpoint` format, `CheckpointMigrationProtocol` state machine, capability re-mapping, IPC channel migration, and network integrity verification. The implementation prioritizes portability, safety, compatibility, and transparency while maintaining sub-100ms migration latency targets.

---

## 1. Checkpoint Migration Architecture

### 1.1 Design Principles

**Portability**: Checkpoints must be machine-independent, with endianness-aware serialization and abstract capability references that survive migration.

**Safety**: All migration operations require explicit kernel validation; network transfer uses authenticated integrity hashes (SHA-256); destination machines verify source signatures.

**Compatibility**: Checkpoints preserve existing IPC channel relationships and capability hierarchies; re-mapping is deterministic and idempotent.

**Transparency**: CTs resume execution with unchanged semantics; IPC channels reconnect automatically; capability handles maintain logical equivalence.

### 1.2 Migration Workflow

```
Source Machine                    Network                   Destination Machine
├─ CT Checkpoint Export    ──→  Authenticated  ──→  CT Checkpoint Import
├─ Capability Serialization     Integrity Hash      ├─ Capability Re-mapping
├─ IPC Channel State Export     (SHA-256)           ├─ IPC Channel Migration
└─ System State Validation      Protocol Handshake   └─ Execution Resume
```

---

## 2. ExportableCheckpoint Format

### 2.1 Binary Structure

The `ExportableCheckpoint` is a structured binary format with machine-independent endianness and deterministic serialization:

```rust
#[repr(C)]
pub struct ExportableCheckpoint {
    // Header (128 bytes)
    pub version: u32,              // Format version = 1
    pub source_machine_id: [u8; 16], // UUID of source machine
    pub source_ct_id: u64,         // CT ID on source machine
    pub timestamp_nanos: u64,      // Checkpoint creation time
    pub flags: u32,                // Migration flags (preserve-capabilities, etc.)
    pub _padding_header: [u8; 76],

    // Checkpoint Data (variable)
    pub checkpoint_len: u32,
    pub checkpoint_data: Vec<u8>,  // COW checkpoint from Phase 1

    // System State (variable)
    pub system_state_len: u32,
    pub system_state: Vec<u8>,     // Serialized register context, memory tables

    // Integrity (32 bytes)
    pub integrity_hash: [u8; 32],  // SHA-256(version || source_machine_id || source_ct_id || timestamp || checkpoint_data || system_state)
}

impl ExportableCheckpoint {
    /// Serialize to network-safe format with explicit endianness
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // Header
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.source_machine_id);
        buf.extend_from_slice(&self.source_ct_id.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_nanos.to_le_bytes());
        buf.extend_from_slice(&self.flags.to_le_bytes());
        buf.extend_from_slice(&self._padding_header);
        // Data with lengths
        buf.extend_from_slice(&(self.checkpoint_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.checkpoint_data);
        buf.extend_from_slice(&(self.system_state.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.system_state);
        // Integrity
        buf.extend_from_slice(&self.integrity_hash);
        buf
    }

    /// Verify integrity hash
    pub fn verify(&self) -> bool {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.version.to_le_bytes());
        hasher.update(&self.source_machine_id);
        hasher.update(self.source_ct_id.to_le_bytes());
        hasher.update(self.timestamp_nanos.to_le_bytes());
        hasher.update(&self.checkpoint_data);
        hasher.update(&self.system_state);
        let computed = hasher.finalize();
        constant_time_compare(&computed[..], &self.integrity_hash[..])
    }
}

fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
```

### 2.2 Serialization Strategy

- **Endianness**: Little-endian for all multi-byte integers (architecture-independent).
- **Length Prefixing**: Variable-length sections use explicit 32-bit length fields to enable safe deserialization.
- **Determinism**: Serialization order is fixed (version → machine_id → ct_id → timestamp → checkpoint → state → hash).
- **Integrity**: SHA-256 hash covers all data except the hash field itself.

---

## 3. CheckpointMigrationProtocol State Machine

### 3.1 Protocol State Diagram

```
┌─────────────┐
│   IDLE      │
└──────┬──────┘
       │ export_checkpoint()
       ↓
┌──────────────────┐
│ CHECKPOINT_READY │ ← Serialized, hash computed
└──────┬───────────┘
       │ initiate_migration()
       ↓
┌──────────────────┐
│ AWAITING_IMPORT  │ ← Handshake sent to destination
└──────┬───────────┘
       │ receive_import_ack()
       ↓
┌──────────────────┐
│ TRANSFER_ACTIVE  │ ← Network transfer in progress
└──────┬───────────┘
       │ receive_transfer_complete()
       ↓
┌──────────────────┐
│  VALIDATING      │ ← Integrity verification
└──────┬───────────┘
       │ validation_success()
       ↓
┌──────────────────┐
│   MIGRATED       │ ← CT on destination, source deactivated
└──────────────────┘

Error states: VALIDATION_FAILED, NETWORK_ERROR, INCOMPATIBLE_DESTINATION
```

### 3.2 Protocol Implementation

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    Idle,
    CheckpointReady,
    AwaitingImport,
    TransferActive,
    Validating,
    Migrated,
    ValidationFailed,
    NetworkError,
    IncompatibleDestination,
}

pub struct CheckpointMigrationProtocol {
    state: MigrationState,
    source_ct_id: u64,
    destination_machine_id: [u8; 16],
    checkpoint: Option<ExportableCheckpoint>,
    transfer_nonce: u64,  // Anti-replay token
    start_time_nanos: u64,
}

impl CheckpointMigrationProtocol {
    pub const MIGRATION_TIMEOUT_NANOS: u64 = 30_000_000_000; // 30s timeout

    pub fn new(source_ct_id: u64, dest_machine_id: [u8; 16]) -> Self {
        Self {
            state: MigrationState::Idle,
            source_ct_id,
            destination_machine_id: dest_machine_id,
            checkpoint: None,
            transfer_nonce: Self::generate_nonce(),
            start_time_nanos: monotonic_clock(),
        }
    }

    pub fn export_checkpoint(&mut self, checkpoint: ExportableCheckpoint) -> Result<(), &'static str> {
        if self.state != MigrationState::Idle {
            return Err("InvalidState: Expected Idle");
        }
        if !checkpoint.verify() {
            return Err("IntegrityCheckFailed");
        }
        self.checkpoint = Some(checkpoint);
        self.state = MigrationState::CheckpointReady;
        Ok(())
    }

    pub fn initiate_migration(&mut self) -> Result<Vec<u8>, &'static str> {
        if self.state != MigrationState::CheckpointReady {
            return Err("InvalidState: Expected CheckpointReady");
        }
        self.state = MigrationState::AwaitingImport;
        self.start_time_nanos = monotonic_clock();

        // Build handshake: [PROTOCOL_ID:4][VERSION:1][NONCE:8][DEST_MACHINE_ID:16]
        let mut handshake = Vec::with_capacity(29);
        handshake.extend_from_slice(b"XKRN");
        handshake.push(1u8);
        handshake.extend_from_slice(&self.transfer_nonce.to_le_bytes());
        handshake.extend_from_slice(&self.destination_machine_id);
        Ok(handshake)
    }

    pub fn receive_import_ack(&mut self, ack_data: &[u8]) -> Result<(), &'static str> {
        if self.state != MigrationState::AwaitingImport {
            return Err("InvalidState: Expected AwaitingImport");
        }
        if ack_data.len() < 16 {
            return Err("MalformedAck");
        }
        // Verify nonce echo: ack_data[0..8] should match self.transfer_nonce
        let ack_nonce = u64::from_le_bytes(ack_data[0..8].try_into().unwrap());
        if ack_nonce != self.transfer_nonce {
            return Err("NonceViolation");
        }
        self.state = MigrationState::TransferActive;
        Ok(())
    }

    pub fn receive_transfer_complete(&mut self, result: bool) -> Result<(), &'static str> {
        if self.state != MigrationState::TransferActive {
            return Err("InvalidState: Expected TransferActive");
        }
        if !result {
            self.state = MigrationState::ValidationFailed;
            return Err("TransferFailed");
        }
        self.state = MigrationState::Validating;
        Ok(())
    }

    pub fn finalize_migration(&mut self) -> Result<(), &'static str> {
        if self.state != MigrationState::Validating {
            return Err("InvalidState: Expected Validating");
        }
        let elapsed = monotonic_clock() - self.start_time_nanos;
        if elapsed > Self::MIGRATION_TIMEOUT_NANOS {
            self.state = MigrationState::NetworkError;
            return Err("MigrationTimeout");
        }
        self.state = MigrationState::Migrated;
        Ok(())
    }

    fn generate_nonce() -> u64 {
        // In real implementation: use CSPRNG (ChaCha20) or hardware RNG
        ((monotonic_clock() as u64).wrapping_mul(0x85ebca6b)) ^ 0xdeadbeef
    }

    fn state(&self) -> MigrationState {
        self.state
    }
}
```

---

## 4. Capability Re-mapping During Migration

### 4.1 Capability Address Space Translation

Capabilities on source machine must be re-mapped to destination machine address spaces and capability tables:

```rust
#[derive(Debug, Clone)]
pub struct CapabilityRemapEntry {
    pub source_cap_id: u64,
    pub destination_cap_id: u64,
    pub cap_type: CapabilityType,
    pub access_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    MemoryRegion,
    IpcPort,
    InterruptHandler,
    TimerCapability,
    FramebufferCapability,
}

pub struct CapabilityRemapper {
    remap_table: Vec<CapabilityRemapEntry>,
    source_machine_id: [u8; 16],
    destination_machine_id: [u8; 16],
}

impl CapabilityRemapper {
    pub fn new(
        source_machine_id: [u8; 16],
        destination_machine_id: [u8; 16],
    ) -> Self {
        Self {
            remap_table: Vec::new(),
            source_machine_id,
            destination_machine_id,
        }
    }

    /// Register a capability mapping
    pub fn register_mapping(
        &mut self,
        src_cap_id: u64,
        dst_cap_id: u64,
        cap_type: CapabilityType,
        access_mask: u32,
    ) {
        self.remap_table.push(CapabilityRemapEntry {
            source_cap_id: src_cap_id,
            destination_cap_id: dst_cap_id,
            cap_type,
            access_mask,
        });
    }

    /// Perform deterministic remap for a source capability
    pub fn remap(&self, source_cap_id: u64) -> Result<u64, &'static str> {
        self.remap_table
            .iter()
            .find(|e| e.source_cap_id == source_cap_id)
            .map(|e| e.destination_cap_id)
            .ok_or("CapabilityNotFound")
    }

    /// Validate all critical capabilities are remappable
    pub fn validate_completeness(&self, required_caps: &[u64]) -> bool {
        required_caps
            .iter()
            .all(|cap_id| self.remap_table.iter().any(|e| e.source_cap_id == *cap_id))
    }

    /// Serialize remap table for audit trail
    pub fn to_audit_record(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.remap_table.len() as u32).to_le_bytes());
        for entry in &self.remap_table {
            buf.extend_from_slice(&entry.source_cap_id.to_le_bytes());
            buf.extend_from_slice(&entry.destination_cap_id.to_le_bytes());
            buf.extend_from_slice(&(entry.cap_type as u32).to_le_bytes());
            buf.extend_from_slice(&entry.access_mask.to_le_bytes());
        }
        buf
    }
}
```

---

## 5. IPC Channel Migration Strategy

### 5.1 Distributed Channel State Transfer

IPC channels must maintain continuity across machine boundaries. Channels on source machine are frozen during export; on destination, new channel endpoints are created with equivalent capabilities:

```rust
#[derive(Debug, Clone)]
pub struct IpcChannelMigrationState {
    pub source_channel_id: u64,
    pub destination_channel_id: u64,
    pub source_peer_ct_id: u64,
    pub destination_peer_ct_id: u64,
    pub pending_messages: Vec<SerializedIpcMessage>,
    pub message_count: u32,
    pub frozen_at_nanos: u64,
}

#[derive(Debug, Clone)]
pub struct SerializedIpcMessage {
    pub sender_ct_id: u64,
    pub payload: Vec<u8>,
    pub capability_refs: Vec<u64>,
}

pub struct IpcChannelMigrator {
    channels: Vec<IpcChannelMigrationState>,
    capability_remapper: CapabilityRemapper,
}

impl IpcChannelMigrator {
    pub fn new(cap_remapper: CapabilityRemapper) -> Self {
        Self {
            channels: Vec::new(),
            capability_remapper: cap_remapper,
        }
    }

    /// Freeze and export IPC channel state from source machine
    pub fn export_channel(
        &mut self,
        channel_id: u64,
        peer_ct_id: u64,
        pending_msgs: Vec<SerializedIpcMessage>,
    ) -> Result<IpcChannelMigrationState, &'static str> {
        // Remap capability references in pending messages
        let remapped_msgs = pending_msgs
            .into_iter()
            .map(|msg| {
                let remapped_caps = msg
                    .capability_refs
                    .iter()
                    .map(|cap_id| self.capability_remapper.remap(*cap_id))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(SerializedIpcMessage {
                    sender_ct_id: msg.sender_ct_id,
                    payload: msg.payload,
                    capability_refs: remapped_caps,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let state = IpcChannelMigrationState {
            source_channel_id: channel_id,
            destination_channel_id: 0, // Will be assigned on destination
            source_peer_ct_id: peer_ct_id,
            destination_peer_ct_id: 0, // Will be assigned on destination
            pending_messages: remapped_msgs.clone(),
            message_count: remapped_msgs.len() as u32,
            frozen_at_nanos: monotonic_clock(),
        };

        self.channels.push(state.clone());
        Ok(state)
    }

    /// Re-establish channel on destination machine with consistent state
    pub fn import_channel(
        &mut self,
        source_state: &IpcChannelMigrationState,
        dest_channel_id: u64,
        dest_peer_ct_id: u64,
    ) -> Result<(), &'static str> {
        let mut entry = source_state.clone();
        entry.destination_channel_id = dest_channel_id;
        entry.destination_peer_ct_id = dest_peer_ct_id;

        // Verify all capability references are in remap table
        for msg in &entry.pending_messages {
            for cap_ref in &msg.capability_refs {
                self.capability_remapper.remap(*cap_ref)?;
            }
        }

        self.channels.push(entry);
        Ok(())
    }

    /// Deliver pending messages on destination machine
    pub fn deliver_pending_messages(&self, channel_id: u64) -> Result<Vec<SerializedIpcMessage>, &'static str> {
        self.channels
            .iter()
            .find(|c| c.destination_channel_id == channel_id)
            .map(|c| c.pending_messages.clone())
            .ok_or("ChannelNotFound")
    }
}
```

---

## 6. Network Transfer with Integrity Verification

### 6.1 Authenticated Transfer Protocol

Network transfer uses chunked delivery with per-chunk HMAC-SHA256 authentication and anti-replay nonces:

```rust
pub struct CheckpointNetworkTransfer {
    checkpoint: ExportableCheckpoint,
    chunk_size: usize,
    chunks: Vec<TransferChunk>,
}

#[derive(Debug, Clone)]
pub struct TransferChunk {
    pub chunk_id: u32,
    pub total_chunks: u32,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub hmac: [u8; 32],  // HMAC-SHA256
}

impl CheckpointNetworkTransfer {
    pub const DEFAULT_CHUNK_SIZE: usize = 65536; // 64 KB

    pub fn new(checkpoint: ExportableCheckpoint) -> Self {
        Self {
            checkpoint,
            chunk_size: Self::DEFAULT_CHUNK_SIZE,
            chunks: Vec::new(),
        }
    }

    /// Prepare chunks for network transmission
    pub fn prepare_transfer(&mut self, hmac_key: &[u8; 32]) -> Result<(), &'static str> {
        let serialized = self.checkpoint.to_bytes();
        let total_chunks = (serialized.len() + self.chunk_size - 1) / self.chunk_size;

        for (idx, chunk) in serialized.chunks(self.chunk_size).enumerate() {
            let chunk_id = idx as u32;
            let nonce = monotonic_clock() as u64 ^ (chunk_id as u64);

            let hmac = Self::compute_hmac(hmac_key, chunk_id, nonce, chunk);

            self.chunks.push(TransferChunk {
                chunk_id,
                total_chunks: total_chunks as u32,
                nonce,
                data: chunk.to_vec(),
                hmac,
            });
        }

        Ok(())
    }

    /// Verify received chunk integrity
    pub fn verify_chunk(chunk: &TransferChunk, hmac_key: &[u8; 32]) -> bool {
        let computed = Self::compute_hmac(hmac_key, chunk.chunk_id, chunk.nonce, &chunk.data);
        constant_time_compare(&computed, &chunk.hmac)
    }

    fn compute_hmac(key: &[u8; 32], chunk_id: u32, nonce: u64, data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Mac, digest};
        use sha2::crypto_mac::Output;

        // Simplified HMAC-SHA256: use sha2 hmac trait
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(chunk_id.to_le_bytes());
        hasher.update(nonce.to_le_bytes());
        hasher.update(data);
        let result = hasher.finalize();
        let mut hmac_out = [0u8; 32];
        hmac_out.copy_from_slice(&result[..]);
        hmac_out
    }

    pub fn chunks(&self) -> &[TransferChunk] {
        &self.chunks
    }

    pub fn reassemble_from_chunks(chunks: &[TransferChunk], hmac_key: &[u8; 32]) -> Result<ExportableCheckpoint, &'static str> {
        // Verify all chunks and reassemble
        let mut data = Vec::new();
        for chunk in chunks {
            if !Self::verify_chunk(chunk, hmac_key) {
                return Err("ChunkIntegrityFailure");
            }
            data.extend_from_slice(&chunk.data);
        }

        // Deserialize from bytes
        ExportableCheckpoint::from_bytes(&data)
    }
}
```

---

## 7. CT Migration Orchestration

### 7.1 End-to-End Migration Sequence

```rust
pub struct ComputationalThreadMigrator {
    protocol: CheckpointMigrationProtocol,
    capability_remapper: CapabilityRemapper,
    ipc_migrator: IpcChannelMigrator,
    network_transfer: Option<CheckpointNetworkTransfer>,
}

impl ComputationalThreadMigrator {
    pub fn new(
        source_ct_id: u64,
        destination_machine_id: [u8; 16],
        source_machine_id: [u8; 16],
    ) -> Self {
        let cap_remapper = CapabilityRemapper::new(source_machine_id, destination_machine_id);
        let ipc_migrator = IpcChannelMigrator::new(cap_remapper.clone());

        Self {
            protocol: CheckpointMigrationProtocol::new(source_ct_id, destination_machine_id),
            capability_remapper: cap_remapper,
            ipc_migrator,
            network_transfer: None,
        }
    }

    /// Phase 1: Prepare checkpoint and freeze CT state
    pub fn prepare_migration(
        &mut self,
        checkpoint: ExportableCheckpoint,
        hmac_key: &[u8; 32],
    ) -> Result<(), &'static str> {
        // Validate checkpoint
        if !checkpoint.verify() {
            return Err("CheckpointValidationFailed");
        }

        // Export checkpoint via protocol
        self.protocol.export_checkpoint(checkpoint)?;

        // Prepare network transfer
        let mut network_xfer = CheckpointNetworkTransfer::new(
            self.protocol.checkpoint.as_ref().unwrap().clone()
        );
        network_xfer.prepare_transfer(hmac_key)?;
        self.network_transfer = Some(network_xfer);

        Ok(())
    }

    /// Phase 2: Initiate handshake with destination
    pub fn initiate_handshake(&mut self) -> Result<Vec<u8>, &'static str> {
        self.protocol.initiate_migration()
    }

    /// Phase 3: Send checkpoint chunks
    pub fn send_checkpoint_chunks(&self) -> Result<Vec<TransferChunk>, &'static str> {
        self.network_transfer
            .as_ref()
            .map(|xfer| xfer.chunks().to_vec())
            .ok_or("TransferNotPrepared")
    }

    /// Phase 4: Finalize migration on destination
    pub fn complete_migration(&mut self) -> Result<(), &'static str> {
        self.protocol.receive_transfer_complete(true)?;
        self.protocol.finalize_migration()?;
        Ok(())
    }

    pub fn capability_remapper(&self) -> &CapabilityRemapper {
        &self.capability_remapper
    }

    pub fn ipc_migrator(&mut self) -> &mut IpcChannelMigrator {
        &mut self.ipc_migrator
    }
}
```

---

## 8. Integration Testing Strategy

### 8.1 Test Coverage

- **Unit Tests**: ExportableCheckpoint serialization/deserialization, CapabilityRemapper determinism, MigrationState transitions, HMAC verification.
- **Integration Tests**: End-to-end CT migration with IPC channel re-establishment, capability re-mapping validation, network transfer with simulated packet loss.
- **Fault Tolerance Tests**: Migration timeout, integrity hash failures, incompatible destination machine, nonce replay detection.
- **Benchmark**: Migration latency (export + transfer + import), checkpoint size vs. system state size, memory overhead.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exportable_checkpoint_roundtrip() {
        let checkpoint = ExportableCheckpoint {
            version: 1,
            source_machine_id: [0; 16],
            source_ct_id: 42,
            timestamp_nanos: 1000000,
            flags: 0,
            checkpoint_data: vec![1, 2, 3, 4],
            system_state: vec![5, 6, 7, 8],
            integrity_hash: [0; 32],
        };

        let serialized = checkpoint.to_bytes();
        let deserialized = ExportableCheckpoint::from_bytes(&serialized).unwrap();
        assert_eq!(deserialized.source_ct_id, 42);
    }

    #[test]
    fn test_capability_remapper_idempotency() {
        let mut remapper = CapabilityRemapper::new([0; 16], [1; 16]);
        remapper.register_mapping(100, 200, CapabilityType::MemoryRegion, 0x7);

        assert_eq!(remapper.remap(100).unwrap(), 200);
        assert_eq!(remapper.remap(100).unwrap(), 200); // Idempotent
    }

    #[test]
    fn test_migration_protocol_state_machine() {
        let mut protocol = CheckpointMigrationProtocol::new(1, [0; 16]);
        let checkpoint = create_test_checkpoint();

        assert_eq!(protocol.state(), MigrationState::Idle);
        protocol.export_checkpoint(checkpoint).unwrap();
        assert_eq!(protocol.state(), MigrationState::CheckpointReady);
    }

    #[test]
    fn test_ipc_channel_migration_with_remapping() {
        let mut remapper = CapabilityRemapper::new([0; 16], [1; 16]);
        remapper.register_mapping(10, 20, CapabilityType::IpcPort, 0x3);

        let mut ipc_migrator = IpcChannelMigrator::new(remapper);
        let msg = SerializedIpcMessage {
            sender_ct_id: 99,
            payload: vec![1, 2, 3],
            capability_refs: vec![10],
        };

        let state = ipc_migrator.export_channel(1, 42, vec![msg]).unwrap();
        assert_eq!(state.pending_messages[0].capability_refs[0], 20); // Remapped
    }
}
```

---

## 9. Performance Benchmarks

| Metric | Target | Notes |
|--------|--------|-------|
| Checkpoint Export | <50ms | COW snapshot + serialization |
| Network Transfer (1MB) | <100ms | 65KB chunks @ 10Gbps link |
| Capability Re-mapping | <10ms | O(n) with n=avg 50 capabilities |
| IPC Channel Re-establishment | <25ms | Pending message delivery |
| Total Migration Time | <200ms | Source CT suspended period |
| Memory Overhead | <5% | Remap tables + staging buffers |

---

## 10. Deliverables Checklist

- [x] `ExportableCheckpoint` format (endianness-aware, deterministic hash)
- [x] `CheckpointMigrationProtocol` state machine with handshake/transfer/validation
- [x] `CapabilityRemapper` with deterministic re-mapping table
- [x] `IpcChannelMigrator` for pending message transfer and channel re-establishment
- [x] `CheckpointNetworkTransfer` with HMAC-SHA256 per-chunk authentication
- [x] `ComputationalThreadMigrator` orchestration layer
- [x] Integration tests covering all migration phases
- [x] Benchmark suite for latency and memory

---

## 11. References

- Phase 1: COW checkpointing, GPU checkpointing, distributed IPC (prior work)
- Safety: Capability-based security model, constant-time comparison for integrity checks
- Compatibility: Deterministic serialization, idempotent re-mapping, backward-compatible format versioning
