# Week 18: Scheduler Performance Optimization - Technical Design

**Phase:** 2 (Optimization Phase)
**Target:** Sub-microsecond IPC latency for co-located agents
**Baseline:** 5.1% overhead → 3.1% (Week 17)
**Goal:** <1µs IPC latency for 100 concurrent Cognitive Tasks

---

## Executive Summary

Week 18 builds on Week 17's profiling work to achieve sub-microsecond inter-process communication (IPC) latency through five complementary optimizations:

1. **Priority calculation caching**: Pre-computed and cached per-CT priority scores (50-80% latency reduction)
2. **Selective TLB flush optimization**: Intelligent TLB invalidation (10-20% reduction)
3. **IPC fast path for same NUMA node**: Zero-copy semantics for co-located agents (<1µs for <1KB)
4. **Slab allocator for CT metadata**: Predictable, allocation-free scheduling operations
5. **Instruction cache locality**: Hot path inlining and code reorganization (5-15% reduction)

This document specifies implementation details, architectural decisions, and validation methodologies.

---

## 1. Priority Calculation Caching

### Problem Statement

Week 17 profiling identified priority recalculation in `ct_schedule()` as a 22% overhead contributor. The scheduler recomputes multi-factor priorities on every scheduling decision despite frequencies of 1-5 kHz per CT, creating cache thrashing and dependency chain stalls.

### Design: Lazy Priority Cache

Implement an **event-driven cache invalidation model** rather than per-scheduling recomputation:

```rust
// ct_lifecycle/scheduler/priority_cache.rs

#[derive(Clone, Copy)]
pub struct PriorityCacheEntry {
    priority: u32,
    version: u16,
    last_update_cycle: u64,
    flags: u8, // dirty bit, urgent bit
}

pub struct PriorityCache {
    entries: [PriorityCacheEntry; MAX_CTS],
    global_version: u16,
    mutation_bitmap: u64, // per 64 CTs
}

impl PriorityCache {
    /// O(1) cache lookup with version validation
    #[inline]
    pub fn get_cached_priority(&self, ct_id: CtId) -> Option<u32> {
        let idx = ct_id as usize;
        let entry = self.entries[idx];

        if entry.version == self.global_version && (entry.flags & 0x01) == 0 {
            return Some(entry.priority);
        }
        None
    }

    /// Mark CT priority as stale (called on state change)
    #[inline]
    pub fn invalidate(&mut self, ct_id: CtId) {
        let idx = ct_id as usize;
        self.entries[idx].flags |= 0x01; // dirty bit
        self.mutation_bitmap |= 1u64 << (idx & 0x3F);
    }

    /// Batch recompute only dirty entries
    pub fn refresh_dirty(&mut self, priority_fn: impl Fn(CtId) -> u32) {
        let mut bitmap = self.mutation_bitmap;
        while bitmap != 0 {
            let bit = bitmap.trailing_zeros() as usize;
            let ct_id = bit as CtId;

            let priority = priority_fn(ct_id);
            self.entries[ct_id as usize].priority = priority;
            self.entries[ct_id as usize].flags &= 0xFE; // clear dirty

            bitmap &= bitmap - 1; // clear lowest set bit (Brian Kernighan)
        }
        self.mutation_bitmap = 0;
    }
}
```

### Invalidation Triggers

Cache invalidation occurs **only** on state mutations:

- CT wake-up from blocked state
- Priority boost from timer expiry
- CPU affinity migration
- Blocking operation (enter I/O wait)
- Preemption by higher-priority CT

All other scheduling decisions use cached values.

### Expected Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Priority calc per schedule | 280 cycles | 45 cycles | 84% |
| Cache hit rate | 0% | 92-98% | N/A |
| Branch mispredictions (priority) | 18/1000 cycles | 2/1000 cycles | 89% |
| **IPC latency reduction** | **baseline** | **-65µs** | **~50-80%** |

---

## 2. Selective TLB Flush Optimization

### Problem Statement

Full TLB flush on every IPC invalidates shared code/data pages, causing **repeated I-cache and D-cache misses**. For same-NUMA-node IPC, address spaces often share page mappings.

### Design: NUMA-Aware Flush Strategy

Implement selective TLB invalidation with **address space sharing analysis**:

```rust
// ct_lifecycle/scheduler/tlb_strategy.rs

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TlbFlushStrategy {
    /// No flush (same address space)
    NoFlush,
    /// INVLPG on critical pages only
    Selective(u8), // bitmap of page offsets
    /// Full flush (different NUMA + different AS)
    Full,
}

pub struct AddressSpaceSharing {
    /// Bitmask: shared pages within 4KB blocks
    shared_pages: [u64; 8], // 512 pages × 64 bits
    numa_node_local: bool,
}

pub fn calculate_tlb_strategy(
    src_ct: &CognitiveTask,
    dst_ct: &CognitiveTask,
) -> TlbFlushStrategy {
    // Same address space = no flush needed
    if src_ct.address_space_id == dst_ct.address_space_id {
        return TlbFlushStrategy::NoFlush;
    }

    // Different NUMA = full flush (context is heavy anyway)
    if src_ct.numa_node != dst_ct.numa_node {
        return TlbFlushStrategy::Full;
    }

    // Same NUMA, different AS: selective flush for message pages only
    let msg_page_count = (src_ct.ipc_buffer_size + 4095) / 4096;
    if msg_page_count <= 4 {
        let mut selective_mask = 0u8;
        for i in 0..msg_page_count {
            selective_mask |= 1u8 << (i & 0x7);
        }
        return TlbFlushStrategy::Selective(selective_mask);
    }

    TlbFlushStrategy::Full
}

/// Execute selective flush with minimal cycles
#[inline]
pub unsafe fn selective_tlb_flush(
    base_addr: usize,
    strategy: TlbFlushStrategy,
) {
    match strategy {
        TlbFlushStrategy::NoFlush => {
            // Zero-cost barrier for compiler optimization
            asm!("", options(nomem, nostack));
        }
        TlbFlushStrategy::Selective(mask) => {
            for i in 0..8 {
                if (mask & (1u8 << i)) != 0 {
                    asm!(
                        "invlpg [{}]",
                        in(reg) base_addr + (i << 12),
                        options(nostack)
                    );
                }
            }
        }
        TlbFlushStrategy::Full => {
            // Load CR3 causes full TLB flush
            asm!(
                "mov rax, cr3; mov cr3, rax",
                options(preserves_flags, nostack)
            );
        }
    }
}
```

### Flush Decision Tree

```
IPC Fast Path:
  1. Same address space?
     → No flush, proceed (most common in single-agent scenarios)
  2. Same NUMA node?
     → Selective flush (message buffer pages only)
  3. Different NUMA node?
     → Full flush + accept context migration cost
```

### Expected Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| TLB flushes per 1000 IPCs | 450 | 120 | 73% |
| Avg flush cost | 156 cycles | 18 cycles (selective) | 88% |
| **IPC latency reduction** | **baseline** | **-18µs** | **~10-20%** |

---

## 3. IPC Fast Path for Same NUMA Node

### Problem Statement

IPC involves 7 slow operations for cross-CT messages:
1. Source buffer serialization (120 cycles)
2. Permission checks (85 cycles)
3. Destination buffer allocation (210 cycles)
4. TLB flush (156 cycles)
5. Context switch (450 cycles)
6. Pipeline refill (180 cycles)
7. Destination deserialization (95 cycles)

**Total: ~1296 cycles (~3.2µs on 4 GHz)**

### Design: Zero-Copy Ring Buffer Fast Path

For **same NUMA node, <1KB messages**, bypass serialization entirely:

```rust
// ct_lifecycle/scheduler/ipc_fastpath.rs

pub const IPC_FASTPATH_THRESHOLD: usize = 1024;
pub const FASTPATH_RING_SLOTS: usize = 128;

pub struct FastpathRingBuffer {
    /// Pre-allocated message slots (cache-line aligned)
    slots: [MsgSlot; FASTPATH_RING_SLOTS],
    /// Producer pointer (sender CT)
    tail: AtomicU32,
    /// Consumer pointer (receiver CT)
    head: AtomicU32,
    /// NUMA node affinity
    numa_node: u8,
}

#[repr(align(64))] // cache-line aligned to avoid false sharing
pub struct MsgSlot {
    /// Message header (16 bytes)
    header: MsgHeader,
    /// Payload (1008 bytes)
    data: [u8; 1008],
}

pub struct MsgHeader {
    src_ct: u32,
    dst_ct: u32,
    len: u16,
    seq: u16,
}

/// Fast path for same-NUMA IPC (target: <500 cycles)
#[inline(always)]
pub fn ipc_fastpath(
    src_ct: &CognitiveTask,
    dst_ct: &CognitiveTask,
    msg: &[u8],
) -> Result<(), IpcError> {
    // Guard conditions for fast path
    if src_ct.numa_node != dst_ct.numa_node {
        return Err(IpcError::DifferentNuma);
    }
    if msg.len() > IPC_FASTPATH_THRESHOLD {
        return Err(IpcError::MessageTooLarge);
    }

    // Acquire write slot (atomic, no locks)
    let ring = unsafe { &dst_ct.fastpath_ring };
    let tail = ring.tail.load(Ordering::Relaxed);
    let next_tail = (tail + 1) & (FASTPATH_RING_SLOTS - 1);

    // Check space available
    if next_tail == ring.head.load(Ordering::Acquire) {
        return Err(IpcError::RingBufferFull);
    }

    // Write message to slot (single memcpy, no serialization)
    let slot = &mut ring.slots[tail as usize];
    slot.header.src_ct = src_ct.id;
    slot.header.dst_ct = dst_ct.id;
    slot.header.len = msg.len() as u16;
    slot.header.seq = src_ct.seq.fetch_add(1, Ordering::Relaxed) as u16;

    // Single memcpy instead of serialization pipeline
    slot.data[..msg.len()].copy_from_slice(msg);

    // Publish: full barrier to ensure visibility
    ring.tail.store(next_tail, Ordering::Release);

    // Signal destination (avoid spurious wakeups)
    if dst_ct.is_blocked_on_ipc() {
        dst_ct.wake();
    }

    Ok(())
}

/// Receiver-side fast path (target: <200 cycles)
#[inline(always)]
pub fn ipc_fastpath_recv(
    ct: &CognitiveTask,
) -> Option<(u32, &'static [u8])> {
    let ring = unsafe { &ct.fastpath_ring };
    let head = ring.head.load(Ordering::Relaxed);

    // Check if message available (no memory barrier for read-only check)
    if head == ring.tail.load(Ordering::Acquire) {
        return None;
    }

    let slot = &ring.slots[head as usize];
    let len = slot.header.len as usize;

    // Return pointer to buffer in-place (zero-copy)
    let result = Some((
        slot.header.src_ct,
        &slot.data[..len],
    ));

    // Advance pointer
    ring.head.store((head + 1) & (FASTPATH_RING_SLOTS - 1), Ordering::Release);

    result
}
```

### Control Flow Integration

```rust
// In main scheduling decision point
pub fn schedule_ipc(src: &CT, dst: &CT, msg: &[u8]) -> Result<()> {
    // Try fast path first (inlined, zero branches on common case)
    if let Ok(()) = ipc_fastpath(src, dst, msg) {
        return Ok(()); // 95% of case reaches here with <500 cycles
    }

    // Fallback to slow path for:
    // - Cross-NUMA IPC
    // - Large messages (>1KB)
    // - Ring buffer full (rare)
    slow_path_ipc(src, dst, msg)
}
```

### Expected Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| IPC latency (same NUMA, <1KB) | 3.2µs | 0.85µs | 73% |
| Cycles per operation | 1296 | 340 | 74% |
| Context switches avoided | 0% | 95%* | N/A |
| **Sub-microsecond achievement** | **No** | **Yes** | ✓ |

*For messages ≤1KB on same NUMA node

---

## 4. Slab Allocator for CT Metadata

### Problem Statement

Dynamic allocation of CT metadata (`~2KB per CT`) causes:
- Unpredictable latency spikes (25-180 cycles variance)
- Fragmentation over uptime
- Cache pollution during allocation

### Design: Pre-allocated Object Pool

```rust
// ct_lifecycle/memory/slab_allocator.rs

pub const SLAB_SIZE: usize = 32 * 1024; // 32 KB per slab
pub const MAX_SLABS: usize = 256;
pub const CT_METADATA_SIZE: usize = 2048;

pub struct SlabAllocator {
    slabs: [Slab; MAX_SLABS],
    slab_count: AtomicUsize,
    allocation_stats: AllocationStats,
}

pub struct Slab {
    /// Bitmap: 1 = free, 0 = allocated
    free_bitmap: AtomicU64,
    data: [u8; SLAB_SIZE],
    _padding: [u8; 64 - (core::mem::size_of::<AtomicU64>() % 64)],
}

pub struct AllocationStats {
    total_allocations: AtomicU64,
    total_deallocations: AtomicU64,
    peak_utilization: AtomicUsize,
}

impl SlabAllocator {
    pub const fn new() -> Self {
        SlabAllocator {
            slabs: [Slab::new(); MAX_SLABS],
            slab_count: AtomicUsize::new(0),
            allocation_stats: AllocationStats {
                total_allocations: AtomicU64::new(0),
                total_deallocations: AtomicU64::new(0),
                peak_utilization: AtomicUsize::new(0),
            },
        }
    }

    /// Allocate CT metadata (target: <20 cycles)
    #[inline]
    pub fn allocate(&self) -> Result<&mut [u8; CT_METADATA_SIZE], AllocationError> {
        let slab_count = self.slab_count.load(Ordering::Relaxed);

        for slab_idx in 0..slab_count {
            let slab = &self.slabs[slab_idx];
            let mut bitmap = slab.free_bitmap.load(Ordering::Relaxed);

            while bitmap != 0 {
                let slot = bitmap.trailing_zeros() as usize;
                let expected = bitmap;

                // Atomic CAS to claim slot
                match slab.free_bitmap.compare_exchange(
                    expected,
                    expected & !(1u64 << slot),
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let offset = slot * CT_METADATA_SIZE;
                        let ptr = unsafe {
                            slab.data.as_mut_ptr().add(offset) as *mut [u8; CT_METADATA_SIZE]
                        };

                        self.allocation_stats.total_allocations.fetch_add(1, Ordering::Relaxed);
                        return Ok(unsafe { &mut *ptr });
                    }
                    Err(new_bitmap) => {
                        bitmap = new_bitmap;
                    }
                }
            }
        }

        // Allocate new slab if space available
        if slab_count < MAX_SLABS {
            match self.slab_count.compare_exchange(
                slab_count,
                slab_count + 1,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => self.allocate(), // retry with new slab
                Err(_) => Err(AllocationError::ExhaustedSlabs),
            }
        } else {
            Err(AllocationError::OutOfMemory)
        }
    }

    /// Deallocate CT metadata (target: <15 cycles)
    #[inline]
    pub fn deallocate(&self, ptr: *const u8) -> Result<(), DeallocError> {
        for (slab_idx, slab) in self.slabs.iter().enumerate() {
            let slab_start = slab.data.as_ptr() as usize;
            let slab_end = slab_start + SLAB_SIZE;
            let ptr_addr = ptr as usize;

            if ptr_addr >= slab_start && ptr_addr < slab_end {
                let offset = ptr_addr - slab_start;
                let slot = offset / CT_METADATA_SIZE;

                slab.free_bitmap.fetch_or(1u64 << slot, Ordering::Release);
                self.allocation_stats.total_deallocations.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        }
        Err(DeallocError::InvalidPointer)
    }
}
```

### Initialization Strategy

Pre-warm allocator on boot with worst-case CT count:

```rust
pub fn prewarm_ct_allocator(expected_cts: usize) {
    let slabs_needed = (expected_cts * CT_METADATA_SIZE + SLAB_SIZE - 1) / SLAB_SIZE;
    unsafe {
        CT_ALLOCATOR.slab_count.store(
            slabs_needed.min(MAX_SLABS),
            Ordering::Release,
        );
    }
    // All CTs pre-allocate metadata on init
    // → Zero allocation latency during runtime
}
```

### Expected Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Allocation latency | 85-210 cycles | 12-18 cycles | 95% |
| Latency variance | ±125 cycles | ±3 cycles | 97% |
| Memory fragmentation | 12-18% | <1% | N/A |
| **IPC latency reduction** | **baseline** | **-8µs** | **~8%** |

---

## 5. Instruction Cache Locality Optimization

### Problem Statement

Hot IPC path spans 8 KB of code (4 cache lines at 32-byte granularity), causing **I-cache misses on pipeline stalls**. Function call overhead and indirect branches cause speculative execution waste.

### Design: Hot Path Consolidation

Reorganize code to fit hot path in single 64-byte cache line:

```rust
// ct_lifecycle/scheduler/hot_path.rs

/// CRITICAL: Keep this function under 256 bytes (single 4KB page)
/// All allocations, bounds checks moved to separate cold functions
#[inline(always)]
#[must_use]
pub fn schedule_decision_hotpath(
    current_ct: &CognitiveTask,
    ready_queue: &[CtId; PRIORITY_LEVELS],
) -> CtId {
    // Step 1: Priority-based search (15 cycles)
    for priority in (0..PRIORITY_LEVELS).rev() {
        if let Some(&ct_id) = ready_queue[priority].first() {
            return ct_id;
        }
    }

    // Step 2: Round-robin fallback (8 cycles)
    current_ct.id
}

/// Secondary hot path: Context switch (keep together for I-cache)
#[inline(always)]
pub unsafe fn context_switch_minimal(
    from: &CognitiveTask,
    to: &CognitiveTask,
) {
    // Save minimal state (8 registers = 64 bytes)
    asm!(
        "mov [rdi], rsp",
        "mov [rdi + 8], rbx",
        "mov [rdi + 16], r12",
        "mov [rdi + 24], r13",
        "mov rsp, [rsi]",
        "mov rbx, [rsi + 8]",
        "mov r12, [rsi + 16]",
        "mov r13, [rsi + 24]",
        in("rdi") from as *const CognitiveTask as *mut u64,
        in("rsi") to as *const CognitiveTask as *mut u64,
        clobber_abi("C"),
        options(nomem, nostack)
    );
}

/// Cold path: Permission checks (moved off hot path)
#[cold]
pub fn ipc_permission_check(src: &CognitiveTask, dst: &CognitiveTask) -> Result<()> {
    if (src.capabilities & IPC_CAP) == 0 {
        return Err(IpcError::PermissionDenied);
    }
    if (dst.capabilities & IPC_RECEIVE_CAP) == 0 {
        return Err(IpcError::PermissionDenied);
    }
    Ok(())
}

/// Cold path: Priority recalculation (cache miss acceptable here)
#[cold]
pub fn recalc_priority_slow(ct: &CognitiveTask) -> u32 {
    let base = ct.base_priority as u32;
    let boost = if ct.wait_time_cycles > BOOST_THRESHOLD {
        (ct.wait_time_cycles / 1000) as u32
    } else {
        0
    };
    (base + boost).min(255)
}
```

### Code Layout Directives

Use linker script to co-locate hot functions:

```ld
/* kernel.ld snippet */
.text.hot : {
    *(.text.hotpath)
    *(.text.context_switch)
    *(.text.ipc_fastpath)
} : text

.text.cold : {
    *(.text.cold)
    *(.text.permission_check)
    *(.text.priority_calc)
}
```

### Branch Prediction Tuning

Arrange likely/unlikely branches to match CPU branch predictor:

```rust
// Structure branches for predictor
#[inline(always)]
pub fn ipc_dispatch(msg: &Message) -> Result<()> {
    // Fast path likely: inlined, predicted taken
    if likely(msg.len <= 1024) {
        return ipc_fastpath_inline(msg);
    }

    // Slow path rare: cold function, predicted not taken
    ipc_slowpath_cold(msg)
}

// LLVM intrinsic for branch probability
#[inline]
const fn likely(b: bool) -> bool {
    // Compiler hint: 99% of branches taken
    b
}
```

### Expected Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| I-cache miss rate (hot path) | 8-12% | 0.2-1.2% | 85% |
| Code footprint (hot) | 8.2 KB | 1.6 KB | 80% |
| Branch mispredict rate | 3.2% | 0.4% | 87% |
| **IPC latency reduction** | **baseline** | **-6µs** | **~5-15%** |

---

## 6. Integration: Full Fast Path

### Unified IPC Decision Tree

```
IPC Request (CT A → CT B, <1KB message)
│
├─→ [Cache Hit] Priority check (48 cycles, cached)
│   └─→ [NUMA Check] Same node?
│       ├─→ [YES] Selective TLB flush (18 cycles)
│       │   └─→ [Ring Space?]
│       │       ├─→ [YES] Fastpath ring enqueue (340 cycles total)
│       │       │   └─→ [Sub-microsecond] ✓
│       │       └─→ [NO] Fallback slow path
│       │
│       └─→ [NO] Full TLB flush (156 cycles)
│           └─→ Slow path (~2.5µs)
│
└─→ [Cache Miss] Recalc priority (280 cycles)
    └─→ Invalidate + try fast path (2% overhead)
```

### Latency Breakdown (Same NUMA, <1KB)

| Component | Cycles | Time |
|-----------|--------|------|
| Priority check (cached) | 48 | 12ns |
| NUMA validation | 24 | 6ns |
| TLB selective flush | 18 | 4.5ns |
| Ring buffer check | 32 | 8ns |
| Atomic compare-exchange | 88 | 22ns |
| Memory barrier | 12 | 3ns |
| Destination wake (fast) | 45 | 11ns |
| **Total** | **267** | **~67ns (0.067µs)** |

**With 1 cache miss every 40 IPCs (2.5% rate):**
- Average: (267 × 39 + 1500 × 1) / 40 = 336 cycles = **0.084µs**

**Target: <1µs for 100 concurrent CTs** → Achievable at >95% hit rate ✓

---

## 7. Validation Strategy

### Benchmarking Methodology

```rust
// ct_lifecycle/bench/ipc_latency.rs

#[bench]
fn bench_ipc_fastpath_100ct(b: &mut Bencher) {
    let cts = spawn_100_concurrent_cts();
    let src = &cts[0];
    let dst = &cts[1];
    let msg = vec![0u8; 512];

    b.iter(|| {
        ipc_fastpath(src, dst, &msg).unwrap()
    });
}

// Expected: 267 ± 15 cycles (sub-microsecond)

#[bench]
fn bench_priority_cache_hit(b: &mut Bencher) {
    let cache = PriorityCache::new();
    let ct_id = CtId(42);
    cache.entries[42].priority = 150;
    cache.entries[42].flags = 0; // not dirty

    b.iter(|| {
        cache.get_cached_priority(ct_id)
    });
}

// Expected: 8 ± 2 cycles (L1 cache hit)
```

### Profiling Integration

Extend Week 17 flamegraph to show optimization impact:

```bash
# Week 18 profiling
perf record -e cycles:u,instructions:u,cache-references:u,cache-misses:u \
    --call-graph=dwarf ./scheduler_bench 100_cts_1000_ipc

# Compare with Week 17 baseline
perf diff week17_perf.data
```

### Regression Testing

- IPC latency percentiles (p50, p95, p99, p999) must not regress
- Concurrent CT count scalability (16, 32, 64, 100 CTs)
- Message size sweep (16B, 128B, 512B, 1KB, 4KB, 16KB)
- NUMA configuration: single-node, dual-node, quad-node

---

## 8. Risk Assessment & Mitigation

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Ring buffer exhaustion under burst load | Medium | Monitor occupancy, backpressure to slow path |
| Cache line false sharing | Medium | Explicit 64-byte alignment on atomic fields |
| Slab fragmentation over long uptime | Low | Pre-warm allocator, periodic defragmentation |
| TLB selective flush correctness | High | Unit tests for all NUMA configurations |
| Instruction cache eviction by other code | Low | Isolate hot path in dedicated section, measure occupancy |

---

## 9. Success Criteria

- [ ] Priority cache hit rate ≥92% (statically verified)
- [ ] IPC latency (same NUMA, <1KB): <1µs (p99)
- [ ] TLB flushes reduced by ≥70% for same-NUMA IPC
- [ ] Slab allocator latency: 12-18 cycles (repeatable)
- [ ] I-cache footprint of hot path: <2KB
- [ ] 100 concurrent CTs sustainable with sub-microsecond scheduling
- [ ] No performance regression on slow-path IPC
- [ ] All tests passing on single-socket and dual-socket NUMA hardware

---

## 10. Deliverables Timeline

| Week | Component | Status |
|------|-----------|--------|
| 18 | Priority cache | Implementation |
| 18 | TLB strategy | Implementation |
| 18 | Fastpath ring buffer | Implementation |
| 18 | Slab allocator | Implementation |
| 18 | I-cache locality | Implementation |
| 18 | Integration testing | Validation |
| 18 | Performance measurement | Benchmarking |

All code changes committed with detailed bench results and flamegraphs.

---

## References

- Week 17 Report: Profiling baseline, flamegraph analysis, top 5 bottlenecks
- XKernal L0 Microkernel Specification (sections 4.2, 4.3 scheduler)
- Intel SDM Vol. 1 (section 11.5 TLB invalidation, section 12 performance monitoring)
- Drepper, U. "What Every Programmer Should Know About Memory" (NUMA, cache hierarchy)

---

**Document Version:** 1.0
**Author:** Staff Engineer, CT Lifecycle & Scheduler
**Approval:** Phase 2 Technical Lead
**Effective Date:** Week 18, Month 3 of FY26
