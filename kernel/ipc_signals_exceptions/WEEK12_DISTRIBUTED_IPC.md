# Week 12 — Distributed IPC: Cross-Machine Channels with Idempotency & Compensation

**Author:** Principal Software Engineer
**Date:** 2026-03-02
**Project:** XKernal Cognitive Substrate OS
**Status:** Technical Design Document (TDD)

---

## Executive Summary

This document specifies the implementation of distributed Inter-Process Communication (IPC) for XKernal, enabling reliable cross-machine message passing with capability re-verification, idempotency guarantees, and effect-based compensation semantics. The solution introduces DistributedChannel primitives, IdempotencyKey derivation from machine IDs and atomic sequence numbers, a bounded deduplication cache with LRU eviction, and four effect classes governing compensation behavior. By downgrading delivery guarantees from exactly-once (local) to at-least-once (distributed) and leveraging effect classes, we ensure correctness under network failures while maintaining transactional semantics for reversible and compensable operations.

---

## Problem Statement

Current XKernal IPC operates exclusively within a single machine, leveraging kernel-managed process memory and synchronous delivery semantics. Distributed systems require:

1. **Cross-machine reliability**: Network partitions, process crashes, and message loss demand idempotency and deduplication
2. **Capability verification under distribution**: Capabilities originating on machine A sent to machine B require re-verification of signatures, constraints, revocation status, and expiry
3. **Semantic guarantees degradation**: Local exactly-once delivery cannot be preserved across networks; at-least-once becomes the practical ceiling
4. **Failure recovery**: Operations with side effects require compensation strategies (rollback, idempotent replay, or acceptance of partial state)
5. **Scalability**: Deduplication caches must bound memory overhead via LRU eviction policies

---

## Architecture

### 2.1 DistributedChannel Struct

```rust
/// Remote endpoint descriptor for cross-machine channel communication
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RemoteEndpoint {
    pub machine_id: MachineId,
    pub socket_addr: std::net::SocketAddr,
    pub tls_fingerprint: [u8; 32], // SHA256(cert) for mutual auth
}

/// Distributed IPC channel with capability-aware message delivery
pub struct DistributedChannel {
    // Local reference
    pub local_channel_id: ChannelId,

    // Remote endpoint information
    pub remote_endpoint: RemoteEndpoint,

    // Delivery semantics
    pub delivery_guarantee: DeliveryGuarantee,

    // Connection management
    pub tls_stream: Option<TlsStream>,
    pub connect_timeout_ms: u32,

    // Idempotency infrastructure
    pub sender_machine_id: MachineId,
    pub sender_process_id: ProcessId,
    pub sequence_counter: Arc<AtomicU64>,

    // Capability cache for re-verification
    pub cap_cache: RwLock<HashMap<CapabilityId, CachedCapability>>,
}

#[derive(Clone, Copy, Debug)]
pub enum DeliveryGuarantee {
    AtLeastOnce,     // Distributed default
    ExactlyOnceLocal,// Local kernel channels only
}

#[derive(Clone, Debug)]
pub struct CachedCapability {
    pub signature_verified: bool,
    pub constraints_valid: bool,
    pub not_revoked: bool,
    pub not_expired: bool,
    pub cached_at: Instant,
    pub cache_ttl_ms: u32,
}
```

### 2.2 Idempotency Key Generation

```rust
/// Unique idempotency key derived from sender identity and atomic sequence
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct IdempotencyKey {
    pub sender_machine_id: MachineId,
    pub sender_process_id: ProcessId,
    pub sequence_number: u64,
}

impl IdempotencyKey {
    pub fn generate(
        sender_machine_id: MachineId,
        sender_process_id: ProcessId,
        sequence_counter: &Arc<AtomicU64>,
    ) -> Self {
        let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);
        IdempotencyKey {
            sender_machine_id,
            sender_process_id,
            sequence_number: seq,
        }
    }

    pub fn to_bytes(&self) -> [u8; 24] {
        let mut bytes = [0u8; 24];
        bytes[0..8].copy_from_slice(&self.sender_machine_id.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.sender_process_id.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.sequence_number.to_le_bytes());
        bytes
    }
}
```

### 2.3 Effect Classification System

```rust
/// Effect classification governs compensation behavior on failure
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectClass {
    /// No side effects; safe to replay indefinitely
    ReadOnly,

    /// Side effects can be reversed via registered compensation handler
    WriteReversible,

    /// Side effects can be compensated (partial rollback) via handler
    WriteCompensable,

    /// Side effects are irreversible; best-effort delivery required
    WriteIrreversible,
}

/// Serialized compensation action for remote invocation
#[derive(Clone, Debug)]
pub struct CompensationHandler {
    pub handler_id: HandlerId,
    pub compensation_args: Vec<u8>,
    pub timeout_ms: u32,
}

/// Effect metadata attached to distributed messages
#[derive(Clone, Debug)]
pub struct EffectMetadata {
    pub effect_class: EffectClass,
    pub compensation: Option<CompensationHandler>,
    pub idempotency_key: IdempotencyKey,
}
```

### 2.4 Deduplication Cache with LRU Eviction

```rust
/// LRU deduplication cache for at-least-once semantics
pub struct DeduplicationCache {
    // Idempotency key → (result, effect_metadata)
    cache: Arc<Mutex<LinkedHashMap<IdempotencyKey, CachedResult>>>,
    max_size: usize,
}

#[derive(Clone, Debug)]
pub struct CachedResult {
    pub message_result: Vec<u8>,
    pub processed_at: Instant,
    pub effect_metadata: EffectMetadata,
}

impl DeduplicationCache {
    pub fn new(max_size: usize) -> Self {
        DeduplicationCache {
            cache: Arc::new(Mutex::new(LinkedHashMap::new())),
            max_size,
        }
    }

    pub fn lookup(&self, key: &IdempotencyKey) -> Option<Vec<u8>> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).map(|r| r.message_result.clone())
    }

    pub fn insert(&self, key: IdempotencyKey, result: Vec<u8>, metadata: EffectMetadata) {
        let mut cache = self.cache.lock().unwrap();

        // Insert and evict LRU entry if at capacity
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            if let Some((evicted_key, _)) = cache.pop_front() {
                // Optionally: log eviction metric
            }
        }

        cache.insert(key, CachedResult {
            message_result: result,
            processed_at: Instant::now(),
            effect_metadata: metadata,
        });
    }

    pub fn evict_stale(&self, ttl_ms: u32) {
        let mut cache = self.cache.lock().unwrap();
        let now = Instant::now();
        cache.retain(|_, v| {
            now.duration_since(v.processed_at).as_millis() < ttl_ms as u128
        });
    }
}
```

### 2.5 Capability Re-Verification Protocol

```rust
/// Pre-flight verification before cross-machine capability transmission
pub struct CapabilityVerifier {
    pub revocation_list: Arc<RwLock<HashSet<CapabilityId>>>,
    pub revocation_check_interval_ms: u32,
}

impl CapabilityVerifier {
    pub async fn verify_before_send(
        &self,
        cap: &Capability,
        remote_endpoint: &RemoteEndpoint,
    ) -> Result<VerificationResult, CapError> {
        let mut verified = VerificationResult {
            signature_valid: false,
            constraints_satisfied: false,
            not_revoked: false,
            not_expired: false,
        };

        // 1. Cryptographic signature verification
        verified.signature_valid = self.verify_signature(cap)?;

        // 2. Constraint validation (e.g., target_machine)
        verified.constraints_satisfied = self.verify_constraints(cap, remote_endpoint)?;

        // 3. Revocation status check against distributed ledger
        let revoked = self.revocation_list.read().unwrap().contains(&cap.id);
        verified.not_revoked = !revoked;

        // 4. Expiry validation
        verified.not_expired = cap.expiry > SystemTime::now();

        if verified.is_valid() {
            Ok(verified)
        } else {
            Err(CapError::VerificationFailed)
        }
    }
}

#[derive(Debug)]
pub struct VerificationResult {
    pub signature_valid: bool,
    pub constraints_satisfied: bool,
    pub not_revoked: bool,
    pub not_expired: bool,
}

impl VerificationResult {
    pub fn is_valid(&self) -> bool {
        self.signature_valid && self.constraints_satisfied &&
        self.not_revoked && self.not_expired
    }
}
```

### 2.6 Distributed Message Format

```rust
/// Wire format for distributed IPC messages
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteMessage {
    pub idempotency_key: IdempotencyKey,
    pub sender_endpoint: RemoteEndpoint,
    pub recipient_channel_id: ChannelId,
    pub payload: Vec<u8>,
    pub effect_metadata: EffectMetadata,
    pub timestamp_us: u64,
    pub signature: [u8; 64], // Ed25519 signature over preceding fields
}

/// Acknowledgment from receiver confirming processing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteAck {
    pub idempotency_key: IdempotencyKey,
    pub status: AckStatus,
    pub result: Vec<u8>,
    pub timestamp_us: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckStatus {
    Processed,
    Duplicate,
    VerificationFailed,
    ProcessingError,
}
```

---

## Implementation

### 3.1 Sender Path: chan_send_distributed Syscall

```rust
/// Distributed channel send with capability re-verification and idempotency
pub async fn chan_send_distributed(
    channel: &DistributedChannel,
    capability: &Capability,
    payload: &[u8],
    effect_class: EffectClass,
    compensation: Option<CompensationHandler>,
) -> Result<RemoteAck, IpcError> {
    // Step 1: Verify capability is valid for remote transmission
    let verifier = CapabilityVerifier {
        revocation_list: Arc::new(RwLock::new(HashSet::new())),
        revocation_check_interval_ms: 5000,
    };
    let verification = verifier.verify_before_send(capability, &channel.remote_endpoint).await?;

    // Step 2: Generate unique idempotency key
    let idempotency_key = IdempotencyKey::generate(
        channel.sender_machine_id,
        channel.sender_process_id,
        &channel.sequence_counter,
    );

    // Step 3: Construct remote message with effect metadata
    let effect_metadata = EffectMetadata {
        effect_class,
        compensation,
        idempotency_key: idempotency_key.clone(),
    };

    let remote_msg = RemoteMessage {
        idempotency_key: idempotency_key.clone(),
        sender_endpoint: RemoteEndpoint {
            machine_id: channel.sender_machine_id,
            socket_addr: get_local_socket_addr()?,
            tls_fingerprint: get_local_tls_fingerprint(),
        },
        recipient_channel_id: channel.local_channel_id,
        payload: payload.to_vec(),
        effect_metadata,
        timestamp_us: get_timestamp_us(),
        signature: [0u8; 64], // Signed below
    };

    // Step 4: Sign message with sender's private key
    let msg_bytes = bincode::serialize(&remote_msg)?;
    let signature = sign_message(&msg_bytes)?;
    let mut signed_msg = remote_msg;
    signed_msg.signature = signature;

    // Step 5: Establish TLS connection and send message
    let tls_stream = establish_tls_connection(&channel.remote_endpoint).await?;
    send_over_tls(&tls_stream, &signed_msg).await?;

    // Step 6: Wait for acknowledgment with timeout
    let ack = wait_for_ack(&tls_stream, &idempotency_key, 5000).await?;

    match ack.status {
        AckStatus::Processed => Ok(ack),
        AckStatus::Duplicate => {
            // Idempotent: return cached result
            Ok(ack)
        }
        AckStatus::VerificationFailed => {
            Err(IpcError::RemoteVerificationFailed)
        }
        AckStatus::ProcessingError => {
            // Invoke compensation if effect is reversible/compensable
            if let Some(handler) = &effect_metadata.compensation {
                invoke_compensation(handler, &channel.remote_endpoint).await?;
            }
            Err(IpcError::RemoteProcessingError)
        }
    }
}
```

### 3.2 Receiver Path: Remote Reception & Deduplication

```rust
/// Remote reception handler: dedup → process → cache → ack
pub async fn handle_remote_message(
    msg: RemoteMessage,
    dedup_cache: &DeduplicationCache,
    receiver_handler: impl Fn(&[u8]) -> Result<Vec<u8>, String>,
) -> Result<RemoteAck, String> {
    // Step 1: Verify sender signature
    if !verify_message_signature(&msg).await? {
        return Ok(RemoteAck {
            idempotency_key: msg.idempotency_key.clone(),
            status: AckStatus::VerificationFailed,
            result: vec![],
            timestamp_us: get_timestamp_us(),
        });
    }

    // Step 2: Deduplication check
    if let Some(cached_result) = dedup_cache.lookup(&msg.idempotency_key) {
        return Ok(RemoteAck {
            idempotency_key: msg.idempotency_key.clone(),
            status: AckStatus::Duplicate,
            result: cached_result,
            timestamp_us: get_timestamp_us(),
        });
    }

    // Step 3: Process message via application handler
    let result = match receiver_handler(&msg.payload) {
        Ok(output) => {
            // Step 4: Cache result for deduplication
            dedup_cache.insert(
                msg.idempotency_key.clone(),
                output.clone(),
                msg.effect_metadata.clone(),
            );
            output
        }
        Err(e) => {
            // Handle effect-specific failure semantics
            match msg.effect_metadata.effect_class {
                EffectClass::ReadOnly => {
                    // Safe to retry indefinitely
                    vec![]
                }
                EffectClass::WriteReversible | EffectClass::WriteCompensable => {
                    // Trigger compensation on sender
                    if let Some(handler) = &msg.effect_metadata.compensation {
                        // Queue compensation task
                        trigger_compensation(handler).await.ok();
                    }
                    vec![]
                }
                EffectClass::WriteIrreversible => {
                    // Best-effort: log and fail open
                    eprintln!("Irreversible operation failed: {}", e);
                    vec![]
                }
            }
        }
    };

    Ok(RemoteAck {
        idempotency_key: msg.idempotency_key.clone(),
        status: if result.is_empty() {
            AckStatus::ProcessingError
        } else {
            AckStatus::Processed
        },
        result,
        timestamp_us: get_timestamp_us(),
    })
}
```

### 3.3 Compensation Invocation

```rust
/// Invoke registered compensation handler on failure
pub async fn invoke_compensation(
    handler: &CompensationHandler,
    remote_endpoint: &RemoteEndpoint,
) -> Result<(), String> {
    // Dispatch compensation handler with timeout
    let compensation_future = execute_compensation_handler(
        handler.handler_id,
        &handler.compensation_args,
    );

    match tokio::time::timeout(
        Duration::from_millis(handler.timeout_ms as u64),
        compensation_future,
    ).await {
        Ok(Ok(_)) => {
            // Successfully compensated
            Ok(())
        }
        Ok(Err(e)) => {
            Err(format!("Compensation handler error: {}", e))
        }
        Err(_) => {
            Err(format!("Compensation handler timed out after {}ms", handler.timeout_ms))
        }
    }
}
```

---

## Testing

### 4.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_key_uniqueness() {
        let seq_counter = Arc::new(AtomicU64::new(0));
        let key1 = IdempotencyKey::generate(MachineId(1), ProcessId(100), &seq_counter);
        let key2 = IdempotencyKey::generate(MachineId(1), ProcessId(100), &seq_counter);
        assert_ne!(key1.sequence_number, key2.sequence_number);
    }

    #[test]
    fn test_dedup_cache_lru_eviction() {
        let cache = DeduplicationCache::new(3);

        cache.insert(IdempotencyKey {
            sender_machine_id: MachineId(1),
            sender_process_id: ProcessId(1),
            sequence_number: 1
        }, vec![1, 2, 3], EffectMetadata {
            effect_class: EffectClass::ReadOnly,
            compensation: None,
            idempotency_key: todo!()
        });

        // Insert 2 more to reach capacity
        for i in 2..=3 {
            cache.insert(IdempotencyKey {
                sender_machine_id: MachineId(1),
                sender_process_id: ProcessId(1),
                sequence_number: i as u64
            }, vec![i as u8], EffectMetadata {
                effect_class: EffectClass::ReadOnly,
                compensation: None,
                idempotency_key: todo!()
            });
        }

        // Insert 4th to trigger eviction of oldest
        cache.insert(IdempotencyKey {
            sender_machine_id: MachineId(1),
            sender_process_id: ProcessId(1),
            sequence_number: 4
        }, vec![4], EffectMetadata {
            effect_class: EffectClass::ReadOnly,
            compensation: None,
            idempotency_key: todo!()
        });

        assert!(cache.lookup(&IdempotencyKey {
            sender_machine_id: MachineId(1),
            sender_process_id: ProcessId(1),
            sequence_number: 1
        }).is_none());
    }

    #[tokio::test]
    async fn test_capability_verification() {
        let verifier = CapabilityVerifier {
            revocation_list: Arc::new(RwLock::new(HashSet::new())),
            revocation_check_interval_ms: 5000,
        };

        let cap = Capability {
            id: CapabilityId(1),
            expiry: SystemTime::now() + Duration::from_secs(3600),
            // ...
        };

        let endpoint = RemoteEndpoint {
            machine_id: MachineId(2),
            socket_addr: "127.0.0.1:9000".parse().unwrap(),
            tls_fingerprint: [0u8; 32],
        };

        let result = verifier.verify_before_send(&cap, &endpoint).await.unwrap();
        assert!(result.not_expired);
    }
}
```

### 4.2 Integration Tests

- Verify cross-machine message delivery with network latency simulation
- Test idempotency under duplicate message injection
- Validate compensation invocation on processing failures
- Measure round-trip latency including TLS handshake

---

## Acceptance Criteria

1. **Idempotency**: Duplicate messages return cached results without re-processing
2. **Capability Verification**: Capabilities verified for signature, constraints, revocation, and expiry before cross-machine transmission
3. **Deduplication Cache**: Bounded to 10,000 entries with LRU eviction; memory overhead <50MB
4. **Effect Class Semantics**: Compensation handlers invoked per EffectClass on sender/receiver failures
5. **Performance**: Cross-machine latency <10ms (including network RTT, TLS, signing)
6. **Delivery Guarantee**: Downgraded from exactly-once_local to at_least_once_distributed with transparent retry
7. **Robustness**: Handle network partitions, process crashes, and TLS certificate validation failures

---

## Design Principles

1. **Capability-Centric Security**: All distributed operations require pre-flight capability re-verification; no implicit delegation
2. **Idempotency by Default**: Atomic sequence numbers and deduplication guarantee safe retry semantics
3. **Effect-Driven Compensation**: Four effect classes enable precise failure recovery strategies (rollback, compensation, or best-effort)
4. **Bounded Resource Consumption**: LRU deduplication cache prevents unbounded memory growth
5. **Transparent Delivery Degradation**: Exactly-once semantics degrade to at-least-once gracefully; application logic remains unchanged
6. **Fail-Safe Defaults**: Unverified capabilities rejected; irreversible operations logged and never silently dropped

---

## Implementation Timeline

- **Week 12a**: DistributedChannel, IdempotencyKey, and DeduplicationCache
- **Week 12b**: Capability re-verification and sender path (chan_send_distributed)
- **Week 12c**: Receiver path, deduplication, and compensation invocation
- **Week 12d**: Integration testing, benchmarking, and documentation

---

## References

- XKernal Week 11: Local IPC & Signal Delivery
- XKernal Week 13: Consensus Protocols for Distributed Transactions
- Herlihy & Shavit: "The Art of Multiprocessor Programming" (Ch. 3: Concurrent Objects)
- Google Protocol Buffers v3: Serialization Format
- TLS 1.3 RFC 8446: Mutual Authentication

