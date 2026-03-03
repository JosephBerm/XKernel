# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 20

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Complete distributed channel hardening: final testing, performance optimization for distributed paths, and integration with SDK layer. Ensure all CSCI syscalls work correctly through SDK.

## Document References
- **Primary:** Section 3.2.4 (Distributed IPC)
- **Supporting:** Section 7 (IPC Latency), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Distributed IPC performance optimization: target < 100ms cross-machine latency
- [ ] Network codec optimization: reduce serialization overhead
- [ ] Batch message transmission: send multiple messages per network packet
- [ ] Connection pooling: reuse connections between machines
- [ ] Distributed channel stress tests: 1000+ concurrent cross-machine messages
- [ ] SDK integration: test all CSCI syscalls work through SDK layer
- [ ] Documentation: distributed IPC API and best practices
- [ ] Performance report: latency breakdown by component
- [ ] Integration test suite: multi-machine scenarios
- [ ] Final validation: all Weeks 1-20 work integrated and tested

## Technical Specifications

### Distributed IPC Optimization
```
// Unoptimized: Serialize per message, send immediately
fn send_distributed_unoptimized(message: &[u8]) -> Result<(), SendError> {
    // 1. Serialize message (expensive)
    let serialized = serde_json::to_vec(message)?;

    // 2. Establish connection
    let mut conn = connect_to_remote_machine(REMOTE_MACHINE)?;

    // 3. Send single message
    conn.send(&serialized)?;
    conn.flush()?;

    Ok(())
}

// Optimized: Batch serialization, connection pooling
fn send_distributed_optimized(message: &[u8]) -> Result<(), SendError> {
    // 1. Use thread-local serialization buffer (reusable)
    let serialized = {
        thread_local!(static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(4096)));
        BUFFER.with(|buf| {
            let mut b = buf.borrow_mut();
            b.clear();
            bincode::serialize_into(&mut *b, message)?;
            b.clone()
        })
    };

    // 2. Get pooled connection (no new connection)
    let mut conn = CONNECTION_POOL.get_connection(REMOTE_MACHINE)?;

    // 3. Enqueue message (batched)
    conn.enqueue(&serialized)?;

    // 4. Auto-flush if batch full or timeout exceeded
    if conn.batch_size() >= BATCH_SIZE_LIMIT || conn.last_flush.elapsed() > Duration::from_millis(10) {
        conn.flush()?;
    }

    Ok(())
}
```

### Network Codec Optimization
```
// Cap'n Proto schema for distributed IPC (zero-copy friendly)
pub struct DistributedMessage {
    pub idempotency_key_machine: u32,
    pub idempotency_key_sender: u64,
    pub idempotency_key_sequence: u64,
    pub effect_class: u8,  // Packed: no enum overhead
    pub payload: Vec<u8>,
}

// Packed binary format (no serde overhead)
impl DistributedMessage {
    pub fn encode(&self, buf: &mut Vec<u8>) {
        // Hand-optimized binary encoding
        buf.extend_from_slice(&self.idempotency_key_machine.to_le_bytes());
        buf.extend_from_slice(&self.idempotency_key_sender.to_le_bytes());
        buf.extend_from_slice(&self.idempotency_key_sequence.to_le_bytes());
        buf.push(self.effect_class);
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);
    }

    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let machine = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let sender = u64::from_le_bytes(buf[4..12].try_into()?);
        let sequence = u64::from_le_bytes(buf[12..20].try_into()?);
        let effect_class = buf[20];
        let payload_len = u32::from_le_bytes(buf[21..25].try_into()?) as usize;
        let payload = buf[25..25+payload_len].to_vec();

        Ok(DistributedMessage {
            idempotency_key_machine: machine,
            idempotency_key_sender: sender,
            idempotency_key_sequence: sequence,
            effect_class,
            payload,
        })
    }
}
```

### Connection Pooling
```
pub struct ConnectionPool {
    pub pools: HashMap<MachineId, Vec<RemoteConnection>>,
    pub max_per_machine: usize,  // Default: 4
}

pub struct RemoteConnection {
    pub machine_id: MachineId,
    pub socket: TcpStream,
    pub write_buffer: Vec<u8>,
    pub last_flush: Instant,
    pub in_use: bool,
}

impl ConnectionPool {
    pub fn get_connection(&mut self, machine_id: MachineId) -> Result<RemoteConnection, PoolError> {
        let pool = self.pools.entry(machine_id).or_insert_with(Vec::new);

        // Try to get idle connection
        if let Some(pos) = pool.iter().position(|c| !c.in_use) {
            let mut conn = pool.remove(pos);
            conn.in_use = true;
            return Ok(conn);
        }

        // Create new connection if under limit
        if pool.len() < self.max_per_machine {
            let socket = TcpStream::connect(get_machine_addr(machine_id))?;
            let conn = RemoteConnection {
                machine_id,
                socket,
                write_buffer: Vec::with_capacity(64 * 1024),  // 64KB buffer
                last_flush: Instant::now(),
                in_use: true,
            };
            Ok(conn)
        } else {
            Err(PoolError::ExhaustedConnections)
        }
    }

    pub fn return_connection(&mut self, mut conn: RemoteConnection) -> Result<(), PoolError> {
        conn.in_use = false;
        conn.write_buffer.clear();
        self.pools.entry(conn.machine_id).or_insert_with(Vec::new).push(conn);
        Ok(())
    }
}
```

### Batch Message Transmission
```
pub struct BatchTransmitter {
    pub queue: VecDeque<DistributedMessage>,
    pub batch_size_limit: usize,  // Default: 100 messages or 64KB
    pub batch_time_limit: Duration,  // Default: 10ms
    pub last_flush: Instant,
}

impl BatchTransmitter {
    pub fn enqueue(&mut self, msg: DistributedMessage) -> Result<(), EnqueueError> {
        self.queue.push_back(msg);

        if self.should_flush() {
            self.flush()?;
        }

        Ok(())
    }

    fn should_flush(&self) -> bool {
        if self.queue.len() >= self.batch_size_limit {
            return true;
        }

        if self.last_flush.elapsed() > self.batch_time_limit {
            return true;
        }

        false
    }

    fn flush(&mut self) -> Result<(), FlushError> {
        let batch_size = self.queue.len();
        let mut buf = Vec::with_capacity(1024);

        // Encode batch header
        buf.extend_from_slice(&(batch_size as u32).to_le_bytes());

        // Encode all messages
        for msg in self.queue.drain(..) {
            msg.encode(&mut buf);
        }

        // Send batch via single network transmission
        NETWORK_LAYER.send_packet(&buf)?;
        self.last_flush = Instant::now();

        Ok(())
    }
}
```

### Distributed Channel Stress Tests
```
#[test]
fn test_distributed_stress_1000_concurrent_messages() {
    // Setup: 3 machines, 10 agents per machine, 100 messages from each
    let machines = setup_distributed_environment(3);
    let mut handles = Vec::new();

    for machine_id in 0..3 {
        for agent_id in 0..10 {
            let h = std::thread::spawn(move || {
                for _ in 0..100 {
                    let remote_machine = (machine_id + 1) % 3;
                    let message = format!("Message from agent {}", agent_id);
                    chan_send_distributed(remote_machine, &message)?;
                }
                Ok::<(), SendError>(())
            });
            handles.push(h);
        }
    }

    // Wait for all to complete
    let mut success_count = 0;
    let mut failure_count = 0;
    for h in handles {
        match h.join() {
            Ok(Ok(())) => success_count += 1,
            _ => failure_count += 1,
        }
    }

    println!("Stress test: {} succeeded, {} failed", success_count, failure_count);
    assert!(success_count > 2500, "At least 2500 messages should succeed");
    assert!(failure_count < 500, "At most 500 messages should fail");
}

#[test]
fn test_distributed_latency_multi_machine() {
    let mut latencies = Vec::new();

    for _ in 0..1000 {
        let remote_machine = REMOTE_MACHINE_ID;
        let start = Instant::now();

        chan_send_distributed(remote_machine, b"test message")?;
        let elapsed = start.elapsed();

        latencies.push(elapsed.as_millis());
    }

    let p50 = percentile(&latencies, 50);
    let p99 = percentile(&latencies, 99);
    let max = latencies.iter().max();

    println!("Distributed latency: P50: {}ms, P99: {}ms, Max: {}ms", p50, p99, max.unwrap_or(&0));
    assert!(p50 < 50, "P50 latency must be < 50ms");
    assert!(p99 < 100, "P99 latency must be < 100ms");
}
```

### SDK Integration Testing
```
#[test]
fn test_sdk_csci_syscalls_integration() {
    // Test all CSCI syscalls work through SDK layer
    let sdk = CognitiveSubstrateSDK::new()?;

    // 1. chan_open syscall
    let channel_id = sdk.chan_open(
        ProtocolHint::Some(Protocol::ReAct),
        REMOTE_CT_REF,
    )?;

    // 2. chan_send syscall
    let request = serde_json::json!({
        "thought": "Testing SDK integration",
        "action": "send_message",
    });
    sdk.chan_send(channel_id, &request.to_string().into_bytes())?;

    // 3. chan_recv syscall
    let response = sdk.chan_recv(channel_id, TIMEOUT_MS)?;

    // 4. sig_register syscall
    sdk.sig_register(CognitiveSignal::SigCheckpoint, test_signal_handler)?;

    // 5. exc_register syscall
    sdk.exc_register(test_exception_handler)?;

    // 6. ct_checkpoint syscall
    let cp_id = sdk.ct_checkpoint()?;

    // 7. ct_resume syscall
    sdk.ct_resume(cp_id)?;

    println!("All CSCI syscalls work through SDK layer");
    Ok(())
}
```

## Dependencies
- **Blocked by:** Week 12-19 (All distributed work)
- **Blocking:** Week 21-22 (SDK Integration), Week 23-24 (Benchmarking & Launch)

## Acceptance Criteria
1. Cross-machine request-response latency < 100ms (P99)
2. Network codec serialization overhead < 5% of message size
3. Connection pooling reduces connection overhead by > 90%
4. Batch transmission reduces per-message overhead by > 50%
5. Stress test: 1000+ concurrent distributed messages with >95% success
6. All CSCI syscalls (chan_open, chan_send, chan_recv, sig_register, exc_register, ct_checkpoint, ct_resume) work
7. SDK layer transparent to application code
8. Multi-machine integration tests all pass
9. Performance report documents latency breakdown
10. No regressions from Weeks 1-19

## Design Principles Alignment
- **Performance:** Batching and pooling minimize per-message overhead
- **Efficiency:** Packed binary format reduces serialization cost
- **Transparency:** SDK provides unified interface across local and distributed
- **Scalability:** Connection pooling handles multiple machines efficiently
