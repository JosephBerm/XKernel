# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 18

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Continue scheduler performance optimization. Implement algorithmic improvements and low-level optimizations targeting sub-microsecond IPC latency for co-located agents.

## Document References
- **Primary:** Section 7 (IPC Latency target: sub-microsecond for request-response between co-located agents), Section 3.2.4 (Semantic IPC Subsystem)
- **Supporting:** Section 17 (from profiling, identify hot paths to optimize)

## Deliverables
- [ ] Optimization 1: Priority calculation caching — avoid recalculating priority for all CTs every context switch
- [ ] Optimization 2: Selective TLB flush — avoid TLB flush on same-NUMA-node context switches
- [ ] Optimization 3: IPC fast path — zero-copy for request-response on same NUMA node
- [ ] Optimization 4: Memory allocation pool — pre-allocate scheduler data structures to avoid allocation latency
- [ ] Optimization 5: Instruction cache locality — rearrange hot scheduler code paths for better instruction cache hit rate
- [ ] Benchmark validation — measure before/after for each optimization
- [ ] Target: IPC latency <1µs for 100 concurrent CTs

## Technical Specifications
**Optimization 1: Priority Calculation Caching**
- Current: recalculate all CT priorities O(n) at each context switch
- Optimization: cache priority scores, update only when CT phase changes or deadline changes
- Implementation: invalidate cache entry on: phase transition, deadline escalation, dependency completion
- Expected improvement: 50-80% reduction in scheduler decision latency

**Optimization 2: Selective TLB Flush**
- Current: flush entire TLB on context switch (expensive on modern CPUs)
- Optimization: if switching between CTs on same NUMA node, skip TLB flush (shared CPU cache)
- Implementation: track current NUMA node per CPU core, compare with next CT's node
- Expected improvement: 10-20% reduction in context switch latency

**Optimization 3: IPC Fast Path**
- Current: request-response copies data through kernel message buffer
- Optimization: if sender and receiver on same NUMA node, pass physical page reference only
- Implementation: map same physical pages into both address spaces, copy happens in shared memory
- Expected improvement: sub-microsecond latency for small messages (<1KB)

**Optimization 4: Memory Allocation Pool**
- Current: allocate scheduler data structures (runqueue entries, priority scores) on demand
- Optimization: pre-allocate pools at boot (1000 CTs worth of scheduler structures)
- Implementation: slab allocator for scheduler objects
- Expected improvement: eliminate allocation latency (target: <1µs), reduce fragmentation

**Optimization 5: Instruction Cache Locality**
- Current: scheduler hot paths scattered across multiple cache lines
- Optimization: use `#[inline]` and careful code layout to fit hot path in single L1 cache line (64 bytes)
- Implementation: profile instruction cache misses, rearrange functions
- Expected improvement: 5-15% reduction in latency

## Dependencies
- **Blocked by:** Week 17 (profiling and bottleneck identification)
- **Blocking:** Week 19-20 (context switch latency optimization), Week 24 (final performance targets)

## Acceptance Criteria
- [ ] Priority caching implemented and validated (50% latency reduction target)
- [ ] Selective TLB flush implemented and validated (10% latency reduction target)
- [ ] IPC fast path implemented and validated (sub-microsecond target)
- [ ] Memory allocation pool implemented and validated (eliminate allocation latency)
- [ ] Instruction cache locality optimized (5% latency reduction target)
- [ ] IPC latency measured: <1µs for 100 concurrent CTs
- [ ] All optimizations backward compatible (no correctness issues)
- [ ] Benchmark results documented

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Sub-microsecond IPC is production requirement for interactive agents
