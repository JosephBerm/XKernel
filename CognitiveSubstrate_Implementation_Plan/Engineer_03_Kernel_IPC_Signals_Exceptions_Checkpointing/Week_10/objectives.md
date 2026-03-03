# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 10

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Extend Shared Context IPC from Week 9 with full concurrency testing, CRDT optimization, and preparation for protocol negotiation. Ensure CRDT merge is lock-free and handles all edge cases.

## Document References
- **Primary:** Section 3.2.4 (Shared Context IPC)
- **Supporting:** Section 9 (CRDT-Based Shared Memory), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Lock-free concurrent write detection using atomic operations
- [ ] Operation log persistence to checkpoint on shared context disable
- [ ] Query interface: agents can query current merged state without forcing merge
- [ ] Conflict statistics: per-key conflict count and resolution latency
- [ ] CRDT optimization: skip merge if no conflicts
- [ ] Integration with exception engine: conflicts trigger exception handler
- [ ] Comprehensive concurrency tests: 2-8 agents, 1000+ concurrent operations
- [ ] Performance profiling: identify bottlenecks in conflict detection and merge
- [ ] Benchmark: measure CRDT merge latency with varying conflict rates
- [ ] Documentation: CRDT merge algorithm and correctness proof

## Technical Specifications

### Lock-Free Concurrent Write Detection
```
pub struct AtomicSharedPage {
    pub base_addr: *mut u8,
    pub write_timestamps: Vec<AtomicU64>,  // Per-cache-line write timestamp
    pub writers: Vec<AtomicU32>,            // Bitmap of which CTs wrote to each cache line
}

fn detect_concurrent_write_lock_free(page: &AtomicSharedPage, offset: u64, writer_id: u32) -> bool {
    let line_index = offset / CACHE_LINE_SIZE;
    let prev_writer = page.writers[line_index].load(Ordering::Acquire);
    let current_time = page.write_timestamps[line_index].load(Ordering::Acquire);

    page.write_timestamps[line_index].store(now_u64(), Ordering::Release);
    page.writers[line_index].store(writer_id, Ordering::Release);

    // Conflict if different writer accessed within last cache-coherency window
    prev_writer != writer_id && prev_writer != 0 && (now_u64() - current_time) < COHERENCY_WINDOW_US
}
```

### Operation Log Persistence
```
fn persist_operation_log_to_checkpoint(channel: &SharedContextChannel) -> Result<(), PersistError> {
    // 1. Create checkpoint for shared context state
    // 2. Serialize entire operation_log to checkpoint
    // 3. Include vector clocks for recovery
    // 4. Use hash chain to ensure operation log integrity
    // 5. Clear operation_log in memory
    // 6. Store checkpoint reference in channel
}

fn restore_operation_log_from_checkpoint(cp: &CognitiveCheckpoint) -> Result<Vec<CrdtOp>, RestoreError> {
    // 1. Extract operation_log from checkpoint
    // 2. Verify hash chain
    // 3. Replay all operations to reconstruct state
    // 4. Return vector of CrdtOp
}
```

### Query Interface for Merged State
```
pub struct SharedContextQuery {
    pub key: CrdtKey,
    pub result: CrdtValue,
    pub is_merged: bool,     // Whether result is from merge or direct read
    pub conflict_count: u64,
}

fn query_shared_context(channel: &SharedContextChannel, key: CrdtKey) -> Result<SharedContextQuery, QueryError> {
    // 1. Look up key in operation log
    // 2. Find all conflicting versions
    // 3. If no conflicts, return direct value
    // 4. If conflicts, apply CRDT merge
    // 5. Return merged result with metadata
}
```

### Conflict Statistics
```
pub struct ConflictStats {
    pub key: CrdtKey,
    pub conflict_count: u64,
    pub last_conflict_timestamp: Option<Timestamp>,
    pub avg_merge_latency_us: u64,
    pub max_merge_latency_us: u64,
    pub conflicting_agents: Vec<ContextThreadId>,
}

pub struct SharedContextMetrics {
    pub stats: HashMap<CrdtKey, ConflictStats>,
    pub total_conflicts: u64,
    pub total_merges: u64,
    pub avg_conflict_rate: f64,  // Conflicts per second
}
```

### CRDT Optimization: Skip Merge if No Conflicts
```
fn apply_crdt_op_optimized(
    state: &mut HashMap<CrdtKey, CrdtValue>,
    op: &CrdtOp,
    conflict_count: &mut u64,
) -> bool {
    // Returns true if merge was performed, false if direct insert
    match op {
        CrdtOp::Set(key, value, clock, timestamp) => {
            if let Some(existing) = state.get_mut(key) {
                // Check if timestamp indicates conflict
                if existing.timestamp != timestamp && existing.timestamp > *timestamp {
                    // Potential conflict: merge
                    *conflict_count += 1;
                    *existing = CrdtValue::merge(existing, value);
                    return true;
                }
            }
            // No conflict: direct insert
            state.insert(key.clone(), value.clone());
            false
        }
        CrdtOp::Remove(key, _, _) => {
            state.remove(key);
            false
        }
        CrdtOp::Merge(sub_ops) => {
            let mut merged = false;
            for sub_op in sub_ops {
                if apply_crdt_op_optimized(state, sub_op, conflict_count) {
                    merged = true;
                }
            }
            merged
        }
    }
}
```

### Conflict-Triggered Exception Handler Integration
```
fn on_crdt_conflict_detected(channel: &SharedContextChannel, key: CrdtKey) {
    // 1. Increment conflict counter
    // 2. If conflict_count > THRESHOLD:
    //    a. Create ConflictException
    //    b. Invoke exception handlers on both agent CTs
    //    c. Allow handlers to escalate or request conflict resolution
}

pub enum ConflictException {
    CrdtMergeConflict {
        key: CrdtKey,
        agent_a_value: CrdtValue,
        agent_b_value: CrdtValue,
        merge_result: CrdtValue,
    },
}
```

### Concurrency Testing
```
#[test]
fn test_concurrent_writes_8_agents_1000_ops() {
    // Create shared context with 8 agents
    // Each agent performs 1000 concurrent writes
    // Verify:
    // 1. No data corruption
    // 2. Final merged state is consistent
    // 3. All conflicts detected and resolved
    // 4. Vector clocks correctly ordered operations
}

#[test]
fn test_crdt_merge_correctness_convergence() {
    // Generate random concurrent operations
    // Apply in different orders on different replicas
    // Verify all replicas converge to same final state
}
```

## Dependencies
- **Blocked by:** Week 9 (Shared Context baseline)
- **Blocking:** Week 11-12 Protocol Negotiation & Distributed IPC

## Acceptance Criteria
1. Lock-free detection prevents missed concurrent writes
2. Operation log persists correctly to checkpoint
3. Query interface allows reading merged state without forcing merge
4. Conflict statistics accurately track all conflicts
5. Optimization skips merge when no conflicts detected
6. Conflict-triggered exceptions work correctly
7. Concurrency tests with 8 agents pass without data corruption
8. CRDT merge deterministically converges all replicas
9. Benchmark: 1000 concurrent operations from 8 agents, merge latency < 1 millisecond
10. All tests pass with < 5% merge overhead vs. direct writes

## Design Principles Alignment
- **Performance:** Lock-free detection and merge optimization minimize overhead
- **Correctness:** CRDT guarantees all replicas converge despite concurrent operations
- **Observability:** Conflict statistics enable performance tuning and debugging
- **Fault Tolerance:** Operation log persistence enables recovery from failures
