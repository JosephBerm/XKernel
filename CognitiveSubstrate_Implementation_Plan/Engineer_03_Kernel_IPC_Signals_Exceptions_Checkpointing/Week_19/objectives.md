# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 19

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Harden distributed IPC channels: implement robust idempotency, ensure exactly-once semantics despite network failures, and implement comprehensive compensation handlers for all effect classes.

## Document References
- **Primary:** Section 3.2.4 (Distributed IPC)
- **Supporting:** Section 2.6 (Semantic IPC), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Idempotency key durability: persist seen keys to local store
- [ ] Deduplication cache persistence: survive process crashes
- [ ] Exactly-once guarantee: prevent duplicate delivery despite retries
- [ ] Compensation handler implementation: reversible, compensable, irreversible
- [ ] Rollback protocol: coordinated rollback between source and destination
- [ ] Network failure recovery: handle connection loss, timeouts, packet loss
- [ ] Two-phase commit simulation: prepare + commit for distributed operations
- [ ] Chaos testing: inject failures and verify recovery
- [ ] Distributed integration tests: multi-machine scenarios
- [ ] Documentation: distributed semantics and failure scenarios

## Technical Specifications

### Persistent Idempotency Key Store
```
pub struct IdempotencyKeyStore {
    pub db: RocksDB,  // Persistent key-value store
    pub table_name: &'static str,
}

pub struct IdempotencyKeyRecord {
    pub key: IdempotencyKey,
    pub timestamp: Timestamp,
    pub result: MessageResult,
    pub status: ProcessingStatus,
}

pub enum ProcessingStatus {
    InProgress,
    Complete,
    RolledBack,
}

impl IdempotencyKeyStore {
    pub fn record_received_key(
        &self,
        key: &IdempotencyKey,
    ) -> Result<(), StoreError> {
        let record = IdempotencyKeyRecord {
            key: key.clone(),
            timestamp: now(),
            result: MessageResult::InProgress,
            status: ProcessingStatus::InProgress,
        };

        // Persist to disk immediately
        let serialized = bincode::serialize(&record)?;
        self.db.put(key.to_bytes(), serialized)?;

        Ok(())
    }

    pub fn has_seen_key(&self, key: &IdempotencyKey) -> Result<bool, StoreError> {
        self.db.get(key.to_bytes()).map(|opt| opt.is_some())
    }

    pub fn get_cached_result(
        &self,
        key: &IdempotencyKey,
    ) -> Result<Option<MessageResult>, StoreError> {
        match self.db.get(key.to_bytes())? {
            Some(serialized) => {
                let record: IdempotencyKeyRecord = bincode::deserialize(&serialized)?;
                Ok(Some(record.result))
            }
            None => Ok(None),
        }
    }

    pub fn mark_complete(
        &self,
        key: &IdempotencyKey,
        result: MessageResult,
    ) -> Result<(), StoreError> {
        let mut record = self.db.get(key.to_bytes())?
            .ok_or(StoreError::KeyNotFound)?;

        record.status = ProcessingStatus::Complete;
        record.result = result;

        let serialized = bincode::serialize(&record)?;
        self.db.put(key.to_bytes(), serialized)?;

        Ok(())
    }
}
```

### Exactly-Once Guarantee Protocol
```
pub enum ExactlyOncePhase {
    Prepare,     // Destination acknowledged message receipt
    Commit,      // Source confirmed processing complete
    Abort,       // Rollback decision
}

pub struct ExactlyOnceMessage {
    pub idempotency_key: IdempotencyKey,
    pub phase: ExactlyOncePhase,
    pub payload: Option<Vec<u8>>,
    pub compensation: Option<CompensationHandler>,
}

fn send_exactly_once(
    message: &[u8],
    dest_machine: MachineId,
) -> Result<(), SendError> {
    // 1. Generate idempotency key
    let key = IdempotencyKey::new(get_ct_id());

    // 2. Prepare phase: send message, wait for ack
    let prepare_msg = ExactlyOnceMessage {
        idempotency_key: key.clone(),
        phase: ExactlyOncePhase::Prepare,
        payload: Some(message.to_vec()),
        compensation: None,
    };

    let mut retry_count = 0;
    loop {
        match send_remote_message(dest_machine, &prepare_msg) {
            Ok(ack) => {
                // Received ack; move to commit phase
                break;
            }
            Err(e) if retry_count < MAX_RETRIES => {
                retry_count += 1;
                thread::sleep(Duration::from_millis(100 * 2_u64.pow(retry_count)));
                continue;
            }
            Err(e) => return Err(SendError::Undeliverable(e)),
        }
    }

    // 3. Commit phase: confirm processing
    let commit_msg = ExactlyOnceMessage {
        idempotency_key: key.clone(),
        phase: ExactlyOncePhase::Commit,
        payload: None,
        compensation: None,
    };

    send_remote_message(dest_machine, &commit_msg)?;

    Ok(())
}

fn receive_exactly_once(
    msg: &ExactlyOnceMessage,
) -> Result<Vec<u8>, ReceiveError> {
    // 1. Check if we've seen this key
    let idempotency_store = get_idempotency_store();

    match msg.phase {
        ExactlyOncePhase::Prepare => {
            if idempotency_store.has_seen_key(&msg.idempotency_key)? {
                // Already processed; return cached result
                return Ok(idempotency_store
                    .get_cached_result(&msg.idempotency_key)?
                    .unwrap()
                    .to_bytes());
            }

            // Record receipt (transactional)
            idempotency_store.record_received_key(&msg.idempotency_key)?;

            // Process message
            let payload = msg.payload.as_ref().ok_or(ReceiveError::NoPayload)?;
            let result = process_message(payload)?;

            // Store result for potential replay
            idempotency_store.mark_complete(
                &msg.idempotency_key,
                MessageResult::Success(result.clone()),
            )?;

            Ok(result)
        }
        ExactlyOncePhase::Commit => {
            // Confirm processing is durable
            let mut record = idempotency_store.get_record(&msg.idempotency_key)?;
            record.status = ProcessingStatus::Complete;
            idempotency_store.update_record(&record)?;
            Ok(vec![])
        }
        ExactlyOncePhase::Abort => {
            // Rollback decision
            idempotency_store.mark_rolled_back(&msg.idempotency_key)?;
            Ok(vec![])
        }
    }
}
```

### Compensation Handlers for All Effect Classes
```
pub struct CompensationHandler {
    pub handler_id: HandlerId,
    pub effect_class: EffectClass,
    pub handler_fn: fn(&RemoteMessage) -> Result<(), CompensationError>,
}

pub enum EffectClass {
    ReadOnly,
    WriteReversible,      // Can undo: transaction rollback
    WriteCompensable,     // Can compensate: inverse operation
    WriteIrreversible,    // Cannot undo: accept loss
}

impl CompensationHandler {
    pub fn invoke(
        &self,
        message: &RemoteMessage,
    ) -> Result<(), CompensationError> {
        match self.effect_class {
            EffectClass::ReadOnly => Ok(()),  // No compensation needed

            EffectClass::WriteReversible => {
                // Rollback transaction
                // Extract transaction ID from message
                let txn_id = extract_txn_id(message)?;
                rollback_transaction(txn_id)?;
                Ok(())
            }

            EffectClass::WriteCompensable => {
                // Invoke inverse operation
                (self.handler_fn)(message)?;
                Ok(())
            }

            EffectClass::WriteIrreversible => {
                // Cannot undo; escalate
                Err(CompensationError::IrreversibleEffect)
            }
        }
    }
}

// Examples of compensation handlers

fn compensate_database_update(msg: &RemoteMessage) -> Result<(), CompensationError> {
    // Inverse operation: if original was INSERT, this is DELETE
    let original_op = extract_operation(msg)?;
    let inverse_op = invert_operation(&original_op)?;
    execute_database_operation(&inverse_op)?;
    Ok(())
}

fn compensate_counter_increment(msg: &RemoteMessage) -> Result<(), CompensationError> {
    // Inverse: decrement by same amount
    let increment_amount = extract_amount(msg)?;
    decrement_counter(increment_amount)?;
    Ok(())
}

fn compensate_money_transfer(msg: &RemoteMessage) -> Result<(), CompensationError> {
    // Inverse: transfer back
    let (from, to, amount) = extract_transfer_details(msg)?;
    transfer_money(to, from, amount)?;  // Reversed direction
    Ok(())
}
```

### Distributed Rollback Protocol
```
pub enum RollbackPhase {
    Prepare,   // Ask all affected parties if they can rollback
    Commit,    // Confirm rollback
    Abort,     // Cancel rollback
}

pub struct RollbackRequest {
    pub request_id: u64,
    pub idempotency_keys: Vec<IdempotencyKey>,
    pub phase: RollbackPhase,
}

fn initiate_distributed_rollback(
    affected_machines: Vec<MachineId>,
    keys: Vec<IdempotencyKey>,
) -> Result<(), RollbackError> {
    let request_id = rand::random();

    // Phase 1: Prepare - ask if rollback is possible
    for machine_id in &affected_machines {
        let prepare_req = RollbackRequest {
            request_id,
            idempotency_keys: keys.clone(),
            phase: RollbackPhase::Prepare,
        };

        match send_rollback_request(machine_id, &prepare_req) {
            Ok(RollbackResponse::CanRollback) => {
                // Good; move to commit phase
            }
            Ok(RollbackResponse::CannotRollback(reason)) => {
                // At least one machine cannot rollback; abort
                for m in &affected_machines {
                    let abort_req = RollbackRequest {
                        request_id,
                        idempotency_keys: keys.clone(),
                        phase: RollbackPhase::Abort,
                    };
                    let _ = send_rollback_request(m, &abort_req);
                }
                return Err(RollbackError::IrreversibleEffect);
            }
            Err(e) => {
                // Network failure; abort
                for m in &affected_machines {
                    let abort_req = RollbackRequest {
                        request_id,
                        idempotency_keys: keys.clone(),
                        phase: RollbackPhase::Abort,
                    };
                    let _ = send_rollback_request(m, &abort_req);
                }
                return Err(RollbackError::NetworkFailure(e));
            }
        }
    }

    // Phase 2: Commit - confirm rollback on all machines
    for machine_id in &affected_machines {
        let commit_req = RollbackRequest {
            request_id,
            idempotency_keys: keys.clone(),
            phase: RollbackPhase::Commit,
        };
        send_rollback_request(machine_id, &commit_req)?;
    }

    Ok(())
}
```

### Chaos Testing
```
#[cfg(test)]
mod chaos_tests {
    use proptest::prelude::*;

    #[test]
    fn test_chaos_network_failures() {
        // Inject random network failures
        for _ in 0..100 {
            let scenario = generate_random_scenario();

            // Randomly drop packets
            let loss_rate = rand::random::<f32>();

            match run_distributed_operation(&scenario, loss_rate) {
                Ok(result) => {
                    // Verify exactly-once: no duplicates
                    assert_no_duplicates(&result);
                }
                Err(e) => {
                    // Verify rollback succeeded
                    assert_rolled_back(&scenario)?;
                }
            }
        }
    }

    #[test]
    fn test_chaos_machine_crashes() {
        // Inject random machine crashes during message processing
        for _ in 0..50 {
            let scenario = generate_random_scenario();
            let crash_phase = rand::random::<CrashPhase>();

            match run_with_crash(&scenario, crash_phase) {
                Ok(_) => {
                    // Verify idempotency: retrying gives same result
                    let retry_result = retry_operation(&scenario)?;
                    assert_same_result(original, retry_result);
                }
                Err(e) => {
                    // Verify consistency: state is recoverable
                    assert_recoverable(&scenario)?;
                }
            }
        }
    }

    #[test]
    fn test_chaos_byzantine_failures() {
        // One machine returns conflicting results
        for _ in 0..30 {
            let scenario = generate_random_scenario();

            match run_with_byzantine(&scenario) {
                Ok(_) => panic!("Should detect Byzantine failure"),
                Err(ByzantineError::DetectedConflict) => {
                    // Good; rollback initiated
                    assert_rolled_back(&scenario)?;
                }
                _ => panic!("Unexpected error"),
            }
        }
    }
}
```

## Dependencies
- **Blocked by:** Week 12 (Distributed IPC baseline)
- **Blocking:** Week 20-21 (Remaining Phase 2 work)

## Acceptance Criteria
1. Idempotency keys persisted and survive process restarts
2. Exactly-once delivery guaranteed despite network retries
3. Compensation handlers work for all effect classes
4. Distributed rollback protocol coordinates multi-machine rollback
5. Chaos tests inject failures and verify recovery
6. No message duplication across crashes
7. All distributed scenarios converge to consistent state
8. Deduplication cache uses persistent storage
9. Integration tests pass for all failure scenarios
10. Documentation covers all distributed semantics and failure cases

## Design Principles Alignment
- **Reliability:** Persistence prevents loss of idempotency tracking
- **Atomicity:** Exactly-once semantics ensure no duplicates
- **Recoverability:** Compensation handlers enable graceful failure handling
- **Resilience:** Chaos tests ensure system survives Byzantine failures
