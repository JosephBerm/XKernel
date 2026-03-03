# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 17

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Begin scheduler performance profiling and optimization. Measure hot paths, identify bottlenecks, and start optimization work targeting production performance.

## Document References
- **Primary:** Section 6.3 (Phase 2 Week 20-24: Kernel: Performance tuning — optimize scheduler, IPC hot path, memory allocation, checkpoint efficiency)
- **Supporting:** Section 7 (Benchmark Strategy with targets for IPC Latency, Security Overhead, Cold Start, Fault Recovery)

## Deliverables
- [ ] Performance profiling setup — kernel profiler for context switch latency, scheduling decision time, IPC latency
- [ ] Profiling run 1: baseline measurements with 100 CTs at varying load levels (10%, 50%, 100% CPU)
- [ ] Profiling run 2: measure hot paths (scheduler priority calculation, context switch, IPC send/recv)
- [ ] Bottleneck identification — pinpoint top 5 slowest code paths in scheduler
- [ ] Optimization plan — document which bottlenecks to target in Weeks 18-20
- [ ] Initial optimizations — start with 2-3 quick wins (e.g., algorithm improvements)
- [ ] Benchmark results — measure performance before/after initial optimizations

## Technical Specifications
**Performance Metrics to Track (Section 7):**
- IPC Latency: request-response latency between co-located agents (target: sub-microsecond)
- Security Overhead: capability check latency per system call (target: <100ns per handle check)
- Cold Start: time from agent definition to first CT execution (target: <50ms)
- Fault Recovery: time from exception to resumed execution from checkpoint (target: <100ms)
- Scheduler Overhead: time spent in scheduling decisions (target: <1% of execution time)
- Context Switch Latency: time to switch from one CT to next (target: <10µs)

**Profiling Tools:**
- Linux perf (if running on Linux for testing)
- Hardware performance counters (CPU cycles, cache misses, branch misses)
- Custom kernel instrumentation (add timing markers to hot paths)
- Trace collection (save execution traces for detailed analysis)

**Hot Paths to Profile:**
1. Scheduler priority calculation (Week 07 onwards)
2. Context switch (save/restore registers, TLB flush)
3. IPC send/recv (capability check, page mapping, message copy)
4. Capability grant/revoke (page table updates, revocation list walk)
5. CT spawn/despawn (allocation, initialization, cleanup)

**Optimization Candidates:**
- Priority calculation: currently O(n) per CT, can cache/incrementally update?
- Context switch: can avoid TLB flush on same-NUMA-node switch?
- IPC: can eliminate even zero-copy if using shared memory regions?
- Capability checks: currently O(1) lookup, already optimized

## Dependencies
- **Blocked by:** Week 16 (adapters working), Phase 1 complete
- **Blocking:** Week 18-20 (performance optimization work), Week 24 (performance targets must be met)

## Acceptance Criteria
- [ ] Profiling setup complete and validated
- [ ] Baseline measurements collected (100 CTs at 3 load levels)
- [ ] Hot paths identified and ranked by latency
- [ ] Bottleneck analysis completed (top 5 slowest paths documented)
- [ ] Optimization plan written (which bottlenecks to fix, estimated impact)
- [ ] 2-3 initial optimizations implemented and tested
- [ ] Before/after benchmark results show improvement
- [ ] Profiling data saved for further analysis

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Performance profiling ensures production readiness
