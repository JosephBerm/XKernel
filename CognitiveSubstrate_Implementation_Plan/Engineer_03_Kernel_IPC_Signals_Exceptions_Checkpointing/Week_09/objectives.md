# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 09

## Phase: PHASE 1 — Advanced IPC & Distributed Communication

## Weekly Objective

Implement Shared Context IPC with CRDT (Conflict-free Replicated Data Type) conflict resolution. Multiple agents map same physical pages as read-write, kernel manages concurrent access and merges conflicting updates.

## Document References
- **Primary:** Section 3.2.4 (Shared Context IPC)
- **Supporting:** Section 9 (CRDT-Based Shared Memory), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] SharedContextChannel struct with CRDT operation log
- [ ] CRDT implementation: Last-Write-Wins (LWW) with vector clocks for causal ordering
- [ ] Physical page sharing: both agents map same physical pages in read-write mode
- [ ] Conflict detection: kernel monitors concurrent writes to same location
- [ ] Conflict resolution: CRDT merge function applied to resolve conflicts
- [ ] Vector clock management: timestamp concurrent operations
- [ ] Operation log: append-only log of all CRDT operations
- [ ] ctx_share_memory syscall: enable shared context on channel
- [ ] Unit tests for CRDT merge, concurrent writes, causal ordering
- [ ] Benchmark: concurrent writes from 2 agents, measure merge latency

## Technical Specifications

### SharedContextChannel Structure
```
pub struct SharedContextChannel {
    pub channel_id: ChannelId,
    pub agent_a: ContextThreadRef,
    pub agent_b: ContextThreadRef,
    pub shared_pages: Vec<PhysicalPage>,
    pub operation_log: Vec<CrdtOperation>,
    pub vector_clocks: HashMap<ContextThreadId, VectorClock>,
    pub conflict_count: u64,
}

pub struct VectorClock {
    pub clock: HashMap<ContextThreadId, u64>,
}

impl VectorClock {
    pub fn increment(&mut self, actor: ContextThreadId) {
        *self.clock.entry(actor).or_insert(0) += 1;
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for (ct_id, clock_val) in &self.clock {
            let other_val = other.clock.get(ct_id).unwrap_or(&0);
            if clock_val > other_val {
                return false;
            }
            if clock_val < other_val {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }
}
```

### CRDT Operation Type
```
pub enum CrdtOp {
    Set(CrdtKey, CrdtValue, VectorClock, Timestamp), // Last-Write-Wins: newer timestamp wins
    Remove(CrdtKey, VectorClock, Timestamp),
    Merge(Vec<CrdtOp>),                              // Merge of concurrent operations
}

pub struct CrdtKey {
    pub offset: u64,        // Offset in shared context
    pub size: u32,
}

pub struct CrdtValue {
    pub data: Vec<u8>,
    pub timestamp: Timestamp,  // Use timestamp to break ties in LWW
}

impl CrdtValue {
    pub fn merge(a: &CrdtValue, b: &CrdtValue) -> CrdtValue {
        // Last-Write-Wins: value with newer timestamp wins
        if a.timestamp > b.timestamp {
            a.clone()
        } else {
            b.clone()
        }
    }
}
```

### Physical Page Mapping
```
fn enable_shared_context(channel: &SharedContextChannel) -> Result<(), ShareError> {
    // 1. Get page tables for both agents
    // 2. Allocate physical pages for shared context (default 4MB)
    // 3. Map pages into both agents' address spaces at same virtual address
    // 4. Set PTE flags: readable, writable, copy-on-write for conflict detection
    // 5. Install page fault handler to detect concurrent writes
    // 6. Return shared context base address to both agents
}
```

### Conflict Detection & CRDT Merge
```
fn on_page_fault_concurrent_write(addr: u64, faulting_ct: ContextThreadId, channel: &mut SharedContextChannel) {
    // 1. Identify which bytes were written
    // 2. Get vector clock for faulting CT
    // 3. Check if other agent also wrote to overlapping region
    // 4. If conflict:
    //    a. Extract both versions from page cache
    //    b. Create CrdtOp::Set for both versions
    //    c. Apply merge function
    //    d. Write merged result back to physical page
    //    e. Record conflict in operation log
    // 5. Re-execute faulting instruction

    let offset_in_page = addr - channel.shared_pages[0].base_addr;
    let mut clock = channel.vector_clocks.entry(faulting_ct).or_insert_with(VectorClock::new);
    clock.increment(faulting_ct);

    channel.conflict_count += 1;

    // Fetch current value from other agent's version
    let key = CrdtKey { offset: offset_in_page as u64, size: 8 };
    let my_value = CrdtValue {
        data: read_from_shared_page(addr, 8),
        timestamp: now(),
    };

    // Check if other agent wrote to same location
    if has_concurrent_write(channel, key) {
        let other_value = get_other_agent_value(channel, key);
        let merged = CrdtValue::merge(&my_value, &other_value);
        write_to_shared_page(addr, &merged.data);
        channel.operation_log.push(CrdtOp::Merge(vec![
            CrdtOp::Set(key.clone(), my_value, clock.clone(), now()),
            CrdtOp::Set(key.clone(), other_value, channel.vector_clocks[&other_agent_id()].clone(), now()),
        ]));
    } else {
        channel.operation_log.push(CrdtOp::Set(key, my_value, clock.clone(), now()));
    }
}
```

### ctx_share_memory Syscall
```
syscall fn ctx_share_memory(channel_id: ChannelId, enable: bool) -> Result<SharedContextHandle, ShareError> {
    // 1. Verify both endpoint CTs of channel exist
    // 2. If enable == true:
    //    a. Call enable_shared_context()
    //    b. Return SharedContextHandle with base address and size
    // 3. If enable == false:
    //    a. Disable page table mapping
    //    b. Flush operation log to checkpoint
    //    c. Return Ok(())
}

pub struct SharedContextHandle {
    pub base_address: u64,
    pub size: usize,
    pub vector_clock: VectorClock,
}
```

### CRDT Merge Algorithm
```
fn crdt_merge_all_operations(ops: &[CrdtOp]) -> HashMap<CrdtKey, CrdtValue> {
    let mut state: HashMap<CrdtKey, CrdtValue> = HashMap::new();

    for op in ops {
        match op {
            CrdtOp::Set(key, value, clock, _) => {
                state.entry(key.clone())
                    .and_modify(|existing| {
                        *existing = CrdtValue::merge(existing, value);
                    })
                    .or_insert_with(|| value.clone());
            }
            CrdtOp::Remove(key, _, _) => {
                state.remove(key);
            }
            CrdtOp::Merge(sub_ops) => {
                let sub_state = crdt_merge_all_operations(sub_ops);
                for (key, value) in sub_state {
                    state.entry(key)
                        .and_modify(|existing| {
                            *existing = CrdtValue::merge(existing, &value);
                        })
                        .or_insert(value);
                }
            }
        }
    }

    state
}
```

## Dependencies
- **Blocked by:** Week 1-8 (Phase 0 & Pub/Sub)
- **Blocking:** Week 10-11 Protocol Negotiation, Week 13-14 Full Fault Tolerance Demo

## Acceptance Criteria
1. Shared context enables both agents to read/write same physical pages
2. Vector clocks correctly track causality of concurrent operations
3. CRDT merge resolves conflicts deterministically
4. Last-Write-Wins correctly breaks ties using timestamp
5. Operation log records all CRDT operations
6. Page fault handler detects concurrent writes
7. No data corruption with concurrent writes from both agents
8. Unit tests cover: basic sharing, concurrent writes, CRDT merge, vector clock ordering
9. Benchmark: 1000 concurrent writes from 2 agents, merge latency < 100 microseconds

## Design Principles Alignment
- **Consistency:** CRDT merge ensures all replicas converge to same state
- **Concurrency:** Both agents can write simultaneously without locks
- **Causality:** Vector clocks track causal relationships between operations
- **Observability:** Operation log enables replay and debugging
