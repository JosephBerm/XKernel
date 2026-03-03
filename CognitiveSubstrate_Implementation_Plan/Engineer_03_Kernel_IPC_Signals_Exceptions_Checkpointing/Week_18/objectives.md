# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 18

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Complete fault recovery optimization: further reduce checkpoint overhead, optimize exception path, and achieve cumulative > 5x improvement over baseline. Validate all optimizations on reference hardware.

## Document References
- **Primary:** Section 7 (Fault Recovery Latency)
- **Supporting:** Section 3.2.6-3.2.8 (Exception, Checkpointing, Watchdog), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] Checkpoint delta optimization: only snapshot changes since last checkpoint
- [ ] Exception context sharing: reuse exception context structures
- [ ] Handler invocation inlining: reduce function call overhead
- [ ] Preemption point caching: track safe preemption points for faster delivery
- [ ] Signal coalescing: combine duplicate signals before delivery
- [ ] Rollback path optimization: fast restore from latest checkpoint
- [ ] Combined optimization benchmark: measure overall system improvement
- [ ] Regression test suite: verify optimizations don't break functionality
- [ ] Performance report: before/after comparison and analysis
- [ ] Documentation: optimization summary and architectural decisions

## Technical Specifications

### Checkpoint Delta Optimization
```
pub struct CheckpointDelta {
    pub base_checkpoint_id: CheckpointId,
    pub changed_pages: Vec<(PageIndex, PageData)>,  // Only dirty pages
    pub timestamp: Timestamp,
}

pub struct DeltaCheckpointManager {
    pub base_checkpoint: Option<CognitiveCheckpoint>,
    pub deltas: Vec<CheckpointDelta>,
}

impl DeltaCheckpointManager {
    pub fn create_delta_checkpoint(
        &mut self,
        ct: &ContextThread,
    ) -> Result<CheckpointId, CheckpointError> {
        // 1. Identify dirty pages since last checkpoint
        let dirty_pages = ct.memory_pages
            .iter()
            .enumerate()
            .filter(|(_, page)| page.is_dirty())
            .collect::<Vec<_>>();

        // 2. Copy only dirty pages (typically 5-10% of total)
        let delta = CheckpointDelta {
            base_checkpoint_id: self.base_checkpoint.as_ref().map(|cp| cp.id),
            changed_pages: dirty_pages
                .iter()
                .map(|(idx, page)| (*idx, page.clone()))
                .collect(),
            timestamp: now(),
        };

        // 3. Store delta (much smaller than full checkpoint)
        let delta_id = CheckpointId::new();
        self.deltas.push(delta);

        // 4. Periodically compact deltas into new base checkpoint
        if self.deltas.len() > 5 {
            self.compact_deltas_to_base()?;
        }

        Ok(delta_id)
    }

    fn compact_deltas_to_base(&mut self) -> Result<(), CheckpointError> {
        // Merge all deltas back into base checkpoint
        // Keeps full checkpoint history compact
        Ok(())
    }

    pub fn restore_from_delta(
        &self,
        ct: &mut ContextThread,
        delta_id: CheckpointId,
    ) -> Result<(), RestoreError> {
        // 1. Start with base checkpoint
        ct.restore_from_checkpoint(self.base_checkpoint.as_ref()?)?;

        // 2. Apply all deltas up to delta_id in order
        for delta in &self.deltas {
            if delta.timestamp <= self.deltas[delta_id.index()].timestamp {
                for (page_idx, page_data) in &delta.changed_pages {
                    ct.memory_pages[*page_idx] = page_data.clone();
                }
            }
        }

        Ok(())
    }
}
```

### Exception Context Pool with Reuse
```
pub struct ExceptionContextPool {
    pub available: std::sync::mpsc::Channel<Box<ExceptionContext>>,
    pub max_pool_size: usize,
    pub current_size: std::sync::atomic::AtomicUsize,
}

impl ExceptionContextPool {
    pub fn acquire(&self) -> Box<ExceptionContext> {
        match self.available.recv() {
            Ok(ctx) => {
                // Reuse pooled context
                ctx
            }
            Err(_) => {
                // Allocate new if pool exhausted
                if self.current_size.load(Ordering::Relaxed) < self.max_pool_size {
                    self.current_size.fetch_add(1, Ordering::Relaxed);
                    Box::new(ExceptionContext::new())
                } else {
                    // Wait for available context (backpressure)
                    self.available.recv().unwrap()
                }
            }
        }
    }

    pub fn release(&self, mut ctx: Box<ExceptionContext>) {
        // Clear for reuse
        ctx.reset();
        let _ = self.available.send(ctx);
    }
}
```

### Handler Invocation Inlining
```
// Unoptimized: Function pointer call with indirection
fn dispatch_handler_indirect(handler: *const (), ctx: &ExceptionContext) {
    unsafe {
        let handler_fn: fn(&ExceptionContext) = std::mem::transmute(handler);
        handler_fn(ctx);  // Indirect call, CPU branch prediction misses
    }
}

// Optimized: Inline critical handlers
#[inline(always)]
fn dispatch_handler_inline(
    handler_type: HandlerType,
    ctx: &ExceptionContext,
) -> ExceptionHandlerResult {
    match handler_type {
        HandlerType::RetryDefaultPolicy => {
            // Inline retry handler (no function call)
            ExceptionHandlerResult::Retry(RetryPolicy::default())
        }
        HandlerType::RollbackLatestCheckpoint => {
            // Inline rollback (no function call)
            ExceptionHandlerResult::Rollback(ctx.checkpoint_available.unwrap())
        }
        HandlerType::Custom(handler_ptr) => {
            // Only call custom handlers through pointer
            unsafe {
                let handler_fn: fn(&ExceptionContext) = std::mem::transmute(handler_ptr);
                handler_fn(ctx)
            }
        }
    }
}

pub enum HandlerType {
    RetryDefaultPolicy,
    RollbackLatestCheckpoint,
    Custom(*const ()),
}
```

### Preemption Point Caching
```
pub struct PreemptionPointCache {
    pub cached_points: Vec<u64>,  // Instruction pointers at safe preemption points
    pub phase_transitions: Vec<u64>,  // Phase transition instructions
    pub syscall_returns: Vec<u64>,   // syscall return instructions
}

impl PreemptionPointCache {
    pub fn register_preemption_point(&mut self, instr_ptr: u64) {
        if !self.cached_points.contains(&instr_ptr) {
            self.cached_points.push(instr_ptr);
            self.cached_points.sort();  // Binary search support
        }
    }

    pub fn is_preemption_point(&self, instr_ptr: u64) -> bool {
        self.cached_points.binary_search(&instr_ptr).is_ok()
    }

    pub fn next_preemption_point(&self, from_instr: u64) -> Option<u64> {
        self.cached_points
            .iter()
            .find(|&&pt| pt > from_instr)
            .copied()
    }
}

// Build cache during CT execution
fn on_phase_transition(ct: &mut ContextThread) {
    ct.preemption_cache.register_preemption_point(ct.registers.rip);
}

fn on_syscall_exit(ct: &mut ContextThread) {
    ct.preemption_cache.register_preemption_point(ct.registers.rip);
}
```

### Signal Coalescing
```
pub struct SignalCoalescer {
    pub pending_signals: VecDeque<CognitiveSignal>,
    pub last_coalesce_time: Timestamp,
}

impl SignalCoalescer {
    pub fn enqueue_signal(&mut self, signal: CognitiveSignal) {
        // Coalesce duplicate signals
        match signal {
            CognitiveSignal::SigBudgetWarn => {
                // Only deliver one SigBudgetWarn per second
                if self.last_coalesce_time.elapsed() < Duration::from_secs(1) {
                    return;  // Skip duplicate
                }
            }
            CognitiveSignal::SigContextLow => {
                // Coalesce multiple context low signals
                if self.pending_signals.contains(&CognitiveSignal::SigContextLow) {
                    return;  // Already queued
                }
            }
            _ => {}
        }

        self.pending_signals.push_back(signal);
        self.last_coalesce_time = now();
    }

    pub fn get_pending_signals(&mut self) -> Vec<CognitiveSignal> {
        self.pending_signals.drain(..).collect()
    }
}
```

### Rollback Path Optimization
```
// Unoptimized: Full state restoration
fn rollback_slow(ct: &mut ContextThread, checkpoint_id: CheckpointId) -> Result<(), RollbackError> {
    let checkpoint = ct.get_checkpoint(checkpoint_id)?;

    // 1. Restore all page tables
    for page in &checkpoint.memory_refs {
        restore_page_table_entry(&page)?;
    }

    // 2. Restore registers
    ct.registers = checkpoint.context_snapshot.registers.clone();

    // 3. Restore working memory
    ct.working_memory = checkpoint.context_snapshot.working_memory.clone();

    Ok(())
}

// Optimized: Fast path for most recent checkpoint
#[inline(always)]
fn rollback_fast(ct: &mut ContextThread) -> Result<(), RollbackError> {
    // Assume latest checkpoint is most common case
    let checkpoint = ct.get_latest_checkpoint()?;

    // 1. Atomic page table swap (pre-forked, just pointer)
    ct.page_table = checkpoint.page_table_fork.take()?;

    // 2. Single TLB flush
    cpu::flush_tlb_all()?;

    // 3. Restore only CPU state (registers in fast path, working memory in background)
    ct.registers = checkpoint.context_snapshot.registers.clone();

    // 4. Background restore for working memory
    BACKGROUND_RESTORE_QUEUE.enqueue(ct.id, checkpoint_id)?;

    Ok(())
}
```

### Combined Optimization Benchmark
```
#[test]
fn test_combined_optimizations_benchmark() {
    // Measure cumulative improvement
    let baseline_latency = bench_baseline_exception_to_resume();  // ~500ms
    let optimized_latency = bench_optimized_exception_to_resume();  // target ~100ms

    let improvement_factor = baseline_latency / optimized_latency;
    println!("Cumulative improvement: {}x", improvement_factor);

    // Target: > 5x improvement
    assert!(improvement_factor > 5.0, "Must achieve > 5x improvement");

    // Breakdown by component
    let delta_improvement = bench_delta_checkpoint_vs_full();
    let context_pool_improvement = bench_context_pool_vs_alloc();
    let handler_inline_improvement = bench_handler_inline_vs_indirect();
    let preemption_cache_improvement = bench_preemption_cache_vs_lookup();
    let rollback_improvement = bench_rollback_fast_vs_slow();

    println!("Delta checkpoint: {}x", delta_improvement);
    println!("Context pool: {}x", context_pool_improvement);
    println!("Handler inline: {}x", handler_inline_improvement);
    println!("Preemption cache: {}x", preemption_cache_improvement);
    println!("Rollback fast: {}x", rollback_improvement);
}
```

## Dependencies
- **Blocked by:** Week 17 (Fault Recovery Optimization baseline)
- **Blocking:** Week 19-20 (Distributed Channel Hardening)

## Acceptance Criteria
1. Checkpoint delta reduces size by > 90% for typical workloads
2. Context pool eliminates allocation overhead for exceptions
3. Handler inlining reduces dispatch latency by > 50%
4. Preemption cache enables binary search for safe points
5. Signal coalescing reduces signal delivery overhead
6. Rollback fast path achieves < 50ms restore latency
7. Combined optimization achieves > 5x improvement
8. No regressions; all functionality preserved
9. Performance report documents all improvements
10. Regression test suite passes completely

## Design Principles Alignment
- **Performance:** Cumulative optimizations achieve aggressive latency targets
- **Efficiency:** Delta checkpoints and context pooling reduce memory overhead
- **Predictability:** Preemption cache and signal coalescing reduce variance
- **Scalability:** Optimization techniques scale to 1000+ agents
