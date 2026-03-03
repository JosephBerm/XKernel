# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 17

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Optimize fault recovery path to achieve < 100ms time from exception to resumed execution. Profile exception handling, reduce latency in context capture, checkpointing, and state restoration.

## Document References
- **Primary:** Section 7 (Fault Recovery Latency — Target: < 100ms)
- **Supporting:** Section 3.2.6 (Exception Engine), Section 3.2.7 (Checkpointing), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Exception handler latency profiling: measure each step of exception handling
- [ ] Context capture optimization: minimize register capture time
- [ ] Checkpoint creation optimization: reduce COW fork overhead
- [ ] State restoration optimization: minimize page table updates
- [ ] Exception dispatch fast path: direct handler invocation without lookups
- [ ] Memory pool for exception contexts: pre-allocate exception structures
- [ ] Batched page table updates: combine multiple updates into single TLB flush
- [ ] Async checkpoint offload: background checkpoint while CT resumes
- [ ] Microbenchmark exception handling path
- [ ] E2E benchmark: measure full exception -> resume cycle

## Technical Specifications

### Exception Handler Latency Breakdown
```
// Target: < 100ms total
Timeline Breakdown (example values):
  0ms:   Exception triggered (fault)
  0.1ms: Context capture (registers, memory snapshot)
  10ms:  Checkpoint creation (COW fork)
  50ms:  Exception handler execution
  100ms: CT resumed
  200ms: Background checkpoint completes (async)

Optimization focus areas:
  1. Context capture: target 0.1ms (currently may be 1-5ms)
  2. Checkpoint: target 10ms (currently may be 50-100ms)
  3. Handler invocation: target 1ms (currently may be 5-10ms)
```

### Context Capture Optimization
```
// Unoptimized: Captures all registers, all memory, all state
fn capture_context_full(ct: &ContextThread) -> ExceptionContext {
    ExceptionContext {
        registers: RegisterSnapshot {
            rax: ct.registers.rax,
            rbx: ct.registers.rbx,
            // ... capture all 16 registers
            // Allocates large snapshot struct
        },
        working_memory: ct.working_memory.clone(),  // Full copy
        tool_state: ct.tool_state.clone(),
        ipc_state: ct.ipc_state.snapshot(),
        // ... full deep copy
    }
}

// Optimized: Capture only exception-relevant state
fn capture_context_minimal(ct: &ContextThread, exception: &CognitiveException) -> ExceptionContext {
    // Pre-allocated context from pool
    let mut ctx = EXCEPTION_CONTEXT_POOL.acquire();

    // 1. Capture only faulting instruction context (rip, rsp)
    ctx.registers.rip = ct.registers.rip;
    ctx.registers.rsp = ct.registers.rsp;
    ctx.registers.rax = ct.registers.rax;  // Return value

    // 2. Exception-specific context only
    match exception {
        CognitiveException::ToolCallFailed(tool_ctx) => {
            ctx.tool_state = tool_ctx.clone();
            // Skip working memory capture for transient failures
        }
        CognitiveException::ContextOverflow => {
            ctx.working_memory = ct.working_memory_snapshot_fast();  // Only metadata, not full copy
        }
        _ => {
            ctx.working_memory = ct.working_memory_snapshot_fast();
        }
    }

    // 3. Lazy copy: don't copy unless handler accesses the field
    ctx.memory_refs = ct.memory_regions.refs();  // Just pointers, no copy

    ctx.timestamp = now();
    ctx
}

pub struct ExceptionContextPool {
    pub contexts: std::sync::mpsc::Channel<ExceptionContext>,
}

impl ExceptionContextPool {
    pub fn acquire(&self) -> ExceptionContext {
        self.contexts.recv()
            .unwrap_or_else(|_| ExceptionContext::new())
    }

    pub fn release(&self, ctx: ExceptionContext) {
        let _ = self.contexts.send(ctx);
    }
}
```

### Checkpoint Creation Optimization
```
// Unoptimized: Full page copy on every checkpoint
fn create_checkpoint_full(ct: &ContextThread) -> Result<CognitiveCheckpoint, CheckpointError> {
    // 1. Allocate checkpoint memory
    let mut checkpoint = CognitiveCheckpoint::new();

    // 2. Copy all memory pages
    for page in ct.memory_pages.iter() {
        checkpoint.memory.push(page.clone());  // Expensive memcpy for each page
    }

    // 3. Capture state
    checkpoint.context_snapshot = ct.working_memory.clone();
    checkpoint.tool_state = ct.tool_state.clone();

    // 4. Compute hash chain
    checkpoint.hash_chain = compute_hash(&checkpoint);

    Ok(checkpoint)
}

// Optimized: Lazy copy-on-write with background sync
fn create_checkpoint_lazy(ct: &mut ContextThread) -> Result<CheckpointId, CheckpointError> {
    // 1. Create checkpoint structure (no data copy yet)
    let checkpoint_id = CheckpointId::new();
    let mut checkpoint = CognitiveCheckpoint::new_empty(checkpoint_id);

    // 2. Set up COW: mark all pages as read-only in both original and checkpoint page tables
    for page in ct.memory_pages.iter_mut() {
        page.mark_copy_on_write()?;
    }

    // 3. Create page table fork (just metadata, no copy)
    let pt_fork = fork_page_table_for_checkpoint_lazy(&ct.memory_pages)?;
    checkpoint.page_table_fork = Some(pt_fork);

    // 4. Capture only metadata (fast)
    checkpoint.phase = ct.current_phase;
    checkpoint.timestamp = now();
    checkpoint.ct_ref = ct.reference();

    // 5. Queue background checkpoint to materialize pages
    CHECKPOINT_MANAGER.queue_materialization(checkpoint_id, ct.id)?;

    // 6. Store placeholder checkpoint
    ct.checkpoints.insert(checkpoint_id, checkpoint);

    Ok(checkpoint_id)
}

pub struct CheckpointMaterializationTask {
    pub checkpoint_id: CheckpointId,
    pub ct_id: ContextThreadId,
}

fn materialize_checkpoint_background(task: CheckpointMaterializationTask) {
    // Run in background thread; CT can resume immediately
    let ct = get_ct(task.ct_id);
    let checkpoint = ct.checkpoints.get_mut(task.checkpoint_id);

    // 1. Copy dirty pages from COW fork
    for page in ct.memory_pages.iter() {
        if page.is_dirty() {
            checkpoint.memory.push(page.clone());
            page.clear_dirty();
        }
    }

    // 2. Compute hash chain (CPU-intensive but background)
    checkpoint.hash_chain = compute_hash(checkpoint);

    // 3. Mark checkpoint complete
    checkpoint.is_materialized = true;
}
```

### State Restoration Optimization
```
// Unoptimized: Update page tables one by one
fn restore_state_unoptimized(ct: &mut ContextThread, checkpoint: &CognitiveCheckpoint) -> Result<(), RestoreError> {
    // For each page: update PT entry, flush TLB
    for (i, page) in checkpoint.memory.iter().enumerate() {
        ct.page_table.update_entry(i, page.physical_addr)?;
        // TLB flush for each update (expensive)
        cpu::flush_tlb_single(i * PAGE_SIZE)?;
    }
    Ok(())
}

// Optimized: Batch updates, single TLB flush
fn restore_state_batch(ct: &mut ContextThread, checkpoint: &CognitiveCheckpoint) -> Result<(), RestoreError> {
    // 1. Prepare all page table updates (no TLB flush yet)
    let mut updates = Vec::new();
    for (i, page) in checkpoint.memory.iter().enumerate() {
        updates.push((i, page.physical_addr));
    }

    // 2. Atomic swap: old page table -> checkpoint page table
    //    (if page tables are pre-forked, this is just one pointer update)
    ct.page_table = checkpoint.page_table_fork.take()?;

    // 3. Single TLB invalidation for entire range
    cpu::flush_tlb_range(0, checkpoint.memory.len() * PAGE_SIZE)?;

    // 4. Restore registers
    ct.registers = checkpoint.context_snapshot.registers.clone();

    Ok(())
}
```

### Exception Dispatch Fast Path
```
// Unoptimized: Exception lookup via HashMap
fn dispatch_exception_slow(ct_id: ContextThreadId, exception: &CognitiveException) {
    let ct = CONTEXT_THREADS.get(&ct_id)?;  // HashMap lookup
    let handler = ct.exception_engine.handler?;  // Option lookup
    unsafe { handler(exception) }
}

// Optimized: Direct handler pointer in register
#[inline(always)]
fn dispatch_exception_fast(ct: &ContextThread, exception: &CognitiveException) {
    // Handler pointer stored in CT, already loaded in register
    if let Some(handler) = ct.exception_engine.handler {
        // Direct call, no lookups
        unsafe { handler(exception) }
    }
}

// Inline assembly for minimal overhead
#[inline(always)]
fn dispatch_exception_asm(handler_ptr: *const (), exception: &CognitiveException) {
    unsafe {
        core::arch::asm!(
            "call [{}]",
            in(reg) handler_ptr,
            in("rdi") exception as *const CognitiveException,
            clobber_abi("C"),
        );
    }
}
```

### E2E Fault Recovery Benchmark
```
#[test]
fn bench_exception_to_resume_full_path() {
    // Setup: CT with exception handler and checkpoint
    let mut ct = setup_test_ct();
    ct.register_exception_handler(test_exception_handler);

    // Benchmark: trigger exception -> capture -> checkpoint -> handler -> resume
    let iterations = 1000;
    let mut latencies = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();

        // Trigger exception
        ct.trigger_exception(CognitiveException::ToolCallFailed(
            ToolFailureContext { /* ... */ }
        ))?;

        // Wait for CT to resume
        ct.wait_until_resumed()?;

        let elapsed = start.elapsed();
        latencies.push(elapsed.as_millis());
    }

    // Analyze results
    let p50 = percentile(&latencies, 50);
    let p99 = percentile(&latencies, 99);
    let max = latencies.iter().max();

    println!("Exception -> Resume: P50: {}ms, P99: {}ms, Max: {}ms", p50, p99, max.unwrap_or(&0));

    // Target: < 100ms for all percentiles
    assert!(p50 < 50, "P50 must be < 50ms");
    assert!(p99 < 100, "P99 must be < 100ms");
    assert!(max.unwrap_or(&0) < 200, "Max must be < 200ms");
}
```

## Dependencies
- **Blocked by:** Week 5-6 (Exception Engine, Checkpointing)
- **Blocking:** Week 19-20 (Remaining optimizations)

## Acceptance Criteria
1. Context capture < 0.1ms (10x improvement over baseline)
2. Checkpoint creation < 10ms (even with materialization deferred)
3. State restoration < 50ms (batch updates reduce TLB flushes)
4. Exception dispatch < 1ms (direct handler pointer)
5. Full exception -> resume cycle < 100ms
6. P99 latency < 100ms; max latency < 200ms
7. Background checkpoint materialization doesn't delay CT
8. Memory pool reduces allocation overhead
9. All tests pass; no correctness regressions
10. Profiler confirms optimizations are effective

## Design Principles Alignment
- **Performance:** Sub-100ms recovery enables interactive responsiveness
- **Efficiency:** Lazy COW and background materialization reduce blocking time
- **Predictability:** Batch operations and memory pools reduce latency variance
- **Responsiveness:** CT resumes quickly even as background work completes
