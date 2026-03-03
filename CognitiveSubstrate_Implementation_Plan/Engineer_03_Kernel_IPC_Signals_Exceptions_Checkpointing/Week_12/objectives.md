# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 12

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Implement Distributed IPC for cross-machine channels with capability re-verification, downgrade exactly_once_local to at_least_once with idempotency keys, and effect class declarations for determining compensation requirements.

## Document References
- **Primary:** Section 3.2.4 (Distributed IPC)
- **Supporting:** Section 2.6 (Semantic IPC), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] DistributedChannel struct with remote endpoint reference and network details
- [ ] Capability re-verification: before sending message, verify sender has access
- [ ] Idempotency key generation: unique per message to detect duplicates
- [ ] Message deduplication: remote side tracks seen idempotency keys
- [ ] Effect class system: READ_ONLY, WRITE_REVERSIBLE, WRITE_COMPENSABLE, WRITE_IRREVERSIBLE
- [ ] Compensation handler registration: for WRITE_COMPENSABLE and WRITE_REVERSIBLE effects
- [ ] Remote message format: include idempotency key, effect class, compensation handler ref
- [ ] Network transport integration: hook into lower-level network layer
- [ ] Unit tests for all effect classes, idempotency, capability verification
- [ ] Benchmark: cross-machine request-response latency

## Technical Specifications

### Distributed Channel Structure
```
pub struct DistributedChannel {
    pub channel_id: ChannelId,
    pub local_endpoint: ContextThreadRef,
    pub remote_endpoint: RemoteContextThreadRef,
    pub remote_machine_id: MachineId,
    pub remote_addr: SocketAddr,
    pub delivery_guarantee: DeliveryGuarantee,
    pub ipc_state: IpcStateSnapshot,
}

pub struct RemoteContextThreadRef {
    pub machine_id: MachineId,
    pub ct_id: ContextThreadId,
    pub capability_token: CapabilityToken,  // Verified on remote side
}
```

### Capability Re-Verification
```
pub struct CapabilityToken {
    pub capability_id: u64,
    pub ct_id: ContextThreadId,
    pub machine_id: MachineId,
    pub timestamp: Timestamp,
    pub signature: Vec<u8>,  // Cryptographic signature by issuing machine
}

fn verify_capability_for_distributed_send(
    sender: &ContextThread,
    receiver: &RemoteContextThreadRef,
) -> Result<(), CapabilityError> {
    // 1. Check sender has capability to access receiver CT
    if !sender.capabilities.contains(&receiver.capability_token) {
        return Err(CapabilityError::NoAccess);
    }

    // 2. Verify capability signature
    if !verify_signature(&receiver.capability_token) {
        return Err(CapabilityError::InvalidSignature);
    }

    // 3. Check capability not expired
    if receiver.capability_token.timestamp.elapsed() > CAPABILITY_TTL {
        return Err(CapabilityError::Expired);
    }

    Ok(())
}
```

### Effect Classes
```
pub enum EffectClass {
    ReadOnly,
    WriteReversible,          // Can undo via compensation
    WriteCompensable,         // Can partially undo via compensation
    WriteIrreversible,        // Cannot undo; must commit or abort
}

pub struct RemoteMessage {
    pub idempotency_key: IdempotencyKey,
    pub effect_class: EffectClass,
    pub compensation_handler: Option<CompensationHandler>,
    pub payload: Vec<u8>,
}

pub struct IdempotencyKey {
    pub machine_id: MachineId,
    pub sender_id: u64,
    pub sequence: u64,
}

impl IdempotencyKey {
    pub fn new(sender_id: u64) -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        IdempotencyKey {
            machine_id: get_local_machine_id(),
            sender_id,
            sequence: SEQUENCE.fetch_add(1, Ordering::SeqCst),
        }
    }
}

pub struct CompensationHandler {
    pub handler_id: HandlerId,
    pub undo_operation: Vec<u8>,  // Serialized undo operation
}
```

### Message Deduplication
```
pub struct DeduplicationCache {
    pub seen_keys: HashMap<IdempotencyKey, MessageResult>,
    pub max_size: usize,  // Default: 10000
}

pub enum MessageResult {
    Success(Vec<u8>),     // Response data
    Failed(String),       // Error message
}

impl DeduplicationCache {
    pub fn has_seen(&self, key: &IdempotencyKey) -> bool {
        self.seen_keys.contains_key(key)
    }

    pub fn get_cached_result(&self, key: &IdempotencyKey) -> Option<MessageResult> {
        self.seen_keys.get(key).cloned()
    }

    pub fn cache_result(&mut self, key: IdempotencyKey, result: MessageResult) {
        if self.seen_keys.len() >= self.max_size {
            // LRU eviction
            self.seen_keys.remove(&self.lru_key());
        }
        self.seen_keys.insert(key, result);
    }
}
```

### Downgrade Strategy: exactly_once_local -> at_least_once
```
fn compute_effective_delivery_guarantee(
    original: DeliveryGuarantee,
    is_distributed: bool,
) -> DeliveryGuarantee {
    match (original, is_distributed) {
        (DeliveryGuarantee::ExactlyOnceLocal, true) => {
            // Distributed: can only provide at_least_once with idempotency
            DeliveryGuarantee::AtLeastOnce
        }
        (other, _) => other,
    }
}
```

### Effect Class Determination & Compensation
```
fn compute_effect_class(message: &RemoteMessage) -> EffectClass {
    // Analyze message payload and operation
    // Determine if operation can be reversed, compensated, or is irreversible

    // Examples:
    // - Read from data store: ReadOnly
    // - Update database record: WriteReversible (can rollback transaction)
    // - Increment counter: WriteCompensable (can apply inverse)
    // - Send external email: WriteIrreversible (cannot undo)

    EffectClass::WriteCompensable  // Default conservative
}

pub fn register_compensation_handler(
    handler_id: HandlerId,
    effect_class: EffectClass,
    compensation_fn: fn(&RemoteMessage) -> Result<(), CompensationError>,
) {
    // Store compensation function in kernel registry
    // Callable on remote side if message fails
}

fn invoke_compensation_on_failure(
    message: &RemoteMessage,
    error: &str,
) -> Result<(), CompensationError> {
    match message.effect_class {
        EffectClass::WriteReversible => {
            // Invoke reversal handler (e.g., rollback transaction)
            Ok(())
        }
        EffectClass::WriteCompensable => {
            // Invoke compensation handler (e.g., apply inverse operation)
            if let Some(handler) = &message.compensation_handler {
                invoke_handler(handler)?;
            }
            Ok(())
        }
        EffectClass::WriteIrreversible => {
            // No compensation possible; operation must be accepted as-is
            Err(CompensationError::IrreversibleEffect)
        }
        EffectClass::ReadOnly => Ok(()),  // No side effects
    }
}
```

### Distributed Message Sending
```
syscall fn chan_send_distributed(
    channel_id: ChannelId,
    message: &[u8],
    effect_class: EffectClass,
) -> Result<(), SendError> {
    // 1. Verify capability re-verification
    let channel = get_distributed_channel(channel_id)?;
    verify_capability_for_distributed_send(&current_ct(), &channel.remote_endpoint)?;

    // 2. Generate idempotency key
    let idempotency_key = IdempotencyKey::new(current_ct().id);

    // 3. Create RemoteMessage
    let remote_msg = RemoteMessage {
        idempotency_key: idempotency_key.clone(),
        effect_class,
        compensation_handler: None,
        payload: message.to_vec(),
    };

    // 4. Send via network transport
    send_over_network(&channel.remote_addr, &remote_msg)?;

    // 5. Wait for delivery/ack based on delivery_guarantee
    match channel.delivery_guarantee {
        DeliveryGuarantee::AtMostOnce => Ok(()),
        DeliveryGuarantee::AtLeastOnce | DeliveryGuarantee::ExactlyOnceLocal => {
            wait_for_ack(&idempotency_key, DELIVERY_TIMEOUT_MS)
        }
    }
}
```

### Remote Message Reception & Deduplication
```
fn receive_distributed_message(msg: RemoteMessage) -> Result<Vec<u8>, ReceiveError> {
    // 1. Check deduplication cache
    if dedup_cache.has_seen(&msg.idempotency_key) {
        // Return cached result
        return Ok(dedup_cache.get_cached_result(&msg.idempotency_key)?);
    }

    // 2. Process message
    let result = process_message(&msg.payload)?;

    // 3. Cache result
    dedup_cache.cache_result(msg.idempotency_key.clone(), MessageResult::Success(result.clone()));

    // 4. Send ack
    send_ack(&msg.idempotency_key)?;

    Ok(result)
}
```

## Dependencies
- **Blocked by:** Week 1-11 (All Phase 0 & Phase 1 prior work)
- **Blocking:** Week 13-14 GPU Checkpointing & Full Fault Tolerance Demo

## Acceptance Criteria
1. Capability re-verification prevents unauthorized cross-machine access
2. Idempotency keys uniquely identify messages
3. Deduplication cache prevents duplicate processing
4. All four effect classes handled correctly
5. exactly_once_local downgraded to at_least_once for distributed
6. Compensation handlers invoked on failure for reversible/compensable effects
7. WriteIrreversible effects properly escalate on error
8. No silent data loss on network failures
9. Unit tests cover: capability verification, idempotency, all effect classes, compensation
10. Benchmark: cross-machine request-response latency < 10ms (accounting for network RTT)

## Design Principles Alignment
- **Security:** Capability re-verification prevents privilege escalation
- **Reliability:** Idempotency prevents duplicate processing on network retries
- **Fault Tolerance:** Compensation handlers enable graceful error recovery
- **Transparency:** Distributed channels work like local channels with automatic protocol adaptation
