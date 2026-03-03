# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 06

## Phase: PHASE 0 — Formalization & Synchronous IPC

## Weekly Objective

Implement the Cognitive Checkpointing Engine with COW page table forking, multiple trigger conditions, hash-linked chain for tamper evidence, and 5-checkpoint retention policy per CT.

## Document References
- **Primary:** Section 3.2.7 (Checkpointing Engine)
- **Supporting:** Section 2.9 (Cognitive Checkpointing Engine), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Copy-on-Write (COW) page table forking for CPU state snapshots
- [ ] Checkpoint trigger points: phase transitions, periodic 60s, pre-preemption, SIG_CHECKPOINT
- [ ] CognitiveCheckpoint struct implementation with all required fields
- [ ] Hash-linked chain: SHA256(previous_checkpoint) in each checkpoint for tamper evidence
- [ ] Checkpoint storage: memory-backed store with LRU eviction of checkpoints > 5 per CT
- [ ] ct_checkpoint syscall: explicit checkpoint creation
- [ ] ct_resume syscall: restore CT from checkpoint
- [ ] Retention policy: keep last 5 checkpoints per CT, evict oldest on overflow
- [ ] Unit tests for all checkpoint triggers and COW mechanics
- [ ] Benchmark: checkpoint creation overhead < 10ms

## Technical Specifications

### Copy-on-Write Page Table Forking
```
pub struct CoWPageTableFork {
    pub original_pt: *const PageTable,
    pub checkpoint_pt: *mut PageTable,
    pub shared_frames: Vec<FrameRef>,       // Frames shared between original and snapshot
    pub dirty_bitmap: Bitmap,               // Tracks which frames have been modified
}

fn fork_page_table_for_checkpoint(ct: &ContextThread) -> Result<CoWPageTableFork, CheckpointError> {
    // 1. Clone page table structure
    // 2. Mark all PTEs as read-only in BOTH original and snapshot
    // 3. Set up page fault handler for CoW
    // 4. Return new page table pointing to shared frames
}
```

### Checkpoint Triggers
1. **Phase Transitions:** When CT transitions between Observe -> Act -> Tool -> Observe
2. **Periodic:** Timer-based, every 60 seconds
3. **Pre-Preemption:** Before kernel preempts CT (scheduler switching to other CT)
4. **SIG_CHECKPOINT:** Explicit signal requesting checkpoint
5. **Exception:** Before invoking exception handler

### CognitiveCheckpoint with Hash Chain
```
pub struct CognitiveCheckpoint {
    pub id: CheckpointId,
    pub ct_ref: ContextThreadRef,
    pub timestamp: Timestamp,
    pub phase: ReasoningPhase,
    pub context_snapshot: ContextSnapshot,      // Working memory contents
    pub chain_position: usize,                  // Index in checkpoint chain
    pub memory_refs: Vec<MemoryRegion>,        // Mapped memory ranges
    pub tool_state: ToolStateSnapshot,         // In-flight tool calls
    pub capability_state: CapabilitySnapshot,  // Active capabilities
    pub ipc_state: IpcStateSnapshot,           // IPC channel state
    pub hash_chain: Vec<u8>,                   // SHA256 of previous checkpoint (32 bytes)
    pub metadata: CheckpointMetadata,
}

pub struct CheckpointMetadata {
    pub trigger_reason: TriggerReason,
    pub memory_size_bytes: u64,
    pub num_pages: usize,
}

pub enum TriggerReason {
    PhaseTransition,
    PeriodicTimer,
    PrePreemption,
    ExplicitSignal,
    ExceptionHandler,
}
```

### Checkpoint Storage & Retention
```
pub struct CheckpointStore {
    pub ct_id: ContextThreadId,
    pub checkpoints: VecDeque<CognitiveCheckpoint>,  // Ordered by creation time
    pub checkpoint_map: HashMap<CheckpointId, usize>, // ID -> index in deque
    pub max_checkpoints: usize,                      // Default: 5
}

impl CheckpointStore {
    pub fn add_checkpoint(&mut self, cp: CognitiveCheckpoint) -> Result<CheckpointId, StoreError> {
        if self.checkpoints.len() >= self.max_checkpoints {
            // Evict oldest checkpoint before adding new one
            if let Some(oldest) = self.checkpoints.pop_front() {
                self.checkpoint_map.remove(&oldest.id);
            }
        }
        self.checkpoints.push_back(cp.clone());
        self.checkpoint_map.insert(cp.id, self.checkpoints.len() - 1);
        Ok(cp.id)
    }

    pub fn get_checkpoint(&self, id: CheckpointId) -> Option<&CognitiveCheckpoint> {
        self.checkpoint_map
            .get(&id)
            .and_then(|&idx| self.checkpoints.get(idx))
    }
}
```

### Tamper Detection via Hash Chain
```
fn verify_checkpoint_chain(store: &CheckpointStore) -> bool {
    for (i, cp) in store.checkpoints.iter().enumerate() {
        if i == 0 {
            // First checkpoint: hash_chain must be empty
            if !cp.hash_chain.is_empty() {
                return false;
            }
        } else if let Some(prev_cp) = store.checkpoints.get(i - 1) {
            // Subsequent checkpoint: hash_chain must match SHA256(previous)
            let expected_hash = sha256(&bincode::encode(prev_cp).unwrap());
            if cp.hash_chain != expected_hash {
                return false;  // Tampering detected
            }
        }
    }
    true
}
```

### ct_checkpoint Syscall
```
syscall fn ct_checkpoint() -> Result<CheckpointId, CheckpointError> {
    // 1. Get current CT
    // 2. Verify CT is in safe state for checkpointing
    // 3. Call fork_page_table_for_checkpoint()
    // 4. Capture context_snapshot, tool_state, capability_state, ipc_state
    // 5. Calculate hash_chain from previous checkpoint
    // 6. Create CognitiveCheckpoint struct
    // 7. Add to CheckpointStore (evict oldest if needed)
    // 8. Return checkpoint ID to caller
}
```

### ct_resume Syscall
```
syscall fn ct_resume(checkpoint_id: CheckpointId) -> Result<(), ResumeError> {
    // 1. Get current CT and checkpoint store
    // 2. Find checkpoint by ID
    // 3. Verify checkpoint belongs to current CT
    // 4. Verify checkpoint hash chain (detect tampering)
    // 5. Restore CT state:
    //    - Restore page tables
    //    - Restore registers
    //    - Restore working memory
    //    - Restore tool state, capabilities, IPC state
    // 6. Resume CT execution from checkpoint
}
```

## Dependencies
- **Blocked by:** Week 1-5 (Formalization, Signals, Exception Engine)
- **Blocking:** Week 12-13 GPU Checkpointing, Week 13-14 Full Fault Tolerance Demo

## Acceptance Criteria
1. COW page table fork works correctly without data corruption
2. All five checkpoint triggers (phase, periodic, preemption, signal, exception) work
3. Hash chain correctly detects tampering with checkpoints
4. Retention policy maintains exactly 5 checkpoints per CT
5. ct_checkpoint syscall completes in < 10ms on 1GB working memory
6. ct_resume correctly restores all CT state
7. Hash chain verification prevents restore from corrupted checkpoints
8. Unit tests cover: all trigger conditions, COW mechanics, hash chain, retention policy
9. Stress test: 100+ checkpoints/second without data loss

## Design Principles Alignment
- **Fault Tolerance:** Multiple triggers ensure checkpoints capture state at critical points
- **Tamper Evidence:** Hash chain provides cryptographic proof of checkpoint integrity
- **Resource Efficiency:** COW avoids copying unchanged memory, LRU eviction limits storage
- **Safety:** Retention of last 5 checkpoints allows recovery from cascading failures
