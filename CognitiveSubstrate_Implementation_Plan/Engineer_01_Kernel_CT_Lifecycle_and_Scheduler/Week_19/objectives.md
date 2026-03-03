# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 19

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Optimize context switch latency. Target sub-microsecond latency for co-located agent context switches. Focus on register save/restore and TLB management.

## Document References
- **Primary:** Section 7 (IPC Latency target: sub-microsecond for co-located agent scheduling), Section 3.2.2 (CPU Scheduling with context switching)
- **Supporting:** Section 3.2.1 (Boot Sequence context), Section 3.3.1 (Semantic Memory Manager impact on memory latency)

## Deliverables
- [ ] Context switch micro-benchmark — measure register save/restore latency
- [ ] Register save optimization — minimize registers saved (only callee-saved + return address)
- [ ] TLB optimization — selective TLB shootdown for multi-socket systems
- [ ] Stack switching — optimize kernel stack pointer switching
- [ ] Instruction prefetch — prefetch next CT's instruction stream before switch
- [ ] Preemption point selection — choose safe preemption points to minimize state flush
- [ ] Benchmark validation — measure context switch latency at varying CPU loads
- [ ] Target: <1µs context switch latency

## Technical Specifications
**Register Save Optimization:**
- Current: save all general-purpose registers (16 on x86-64 = 128 bytes)
- Optimization: only save callee-saved registers (6 on x86-64 = 48 bytes)
  - Callee-saved: RBX, RBP, R12-R15 (must save)
  - Caller-saved: RAX, RCX, RDX, RSI, RDI, R8-R11 (saved by function prologue)
- Implementation: update context switch code to save only 6 registers
- Expected improvement: 50% reduction in save/restore latency

**TLB Optimization (Section 3.2.2):**
- Current: full TLB flush (INVLPG all) on every context switch
- Optimization: use ASID (Address Space IDentifier) if available, or lazy TLB flush
  - x86-64 without ASID: tag page table with PCID (Process Context ID), avoid flush on switch to same PCID
  - ARM64 with ASID: hardware ensures ASID separation, no software TLB flush needed
- Implementation: configure PCID on x86-64 boot, avoid TLB shootdown for same-NUMA switches
- Expected improvement: 20-30% reduction in context switch latency

**Stack Switching:**
- Current: load kernel stack pointer from CPU's per-CPU area (2-3 memory accesses)
- Optimization: cache kernel stack pointer in CPU register or reduce indirection
- Implementation: use dedicated CPU segment register to cache stack (requires boot-time config)
- Expected improvement: 5-10% reduction in context switch latency

**Instruction Prefetch:**
- Current: next CT's code cold in L1/L2 cache after switch
- Optimization: before context switch, prefetch next CT's entrypoint (hint to CPU prefetcher)
- Implementation: use `prefetchnta` instruction on next CT's scheduler_entry function
- Expected improvement: eliminate first-instruction cache miss (5-10% reduction)

**Preemption Point Selection:**
- Current: preempt at any time (might catch CT in middle of complex operation)
- Optimization: choose preemption points after atomic sections, after TLB-affecting ops
- Implementation: mark safe preemption points in code, check preemption flag only there
- Expected improvement: reduce exception handler complexity, faster resumption

## Dependencies
- **Blocked by:** Week 18 (IPC optimization), profiling data from Week 17
- **Blocking:** Week 20-24 (overall performance targets depend on this)

## Acceptance Criteria
- [ ] Register save optimization implemented (<50 bytes saved)
- [ ] TLB optimization implemented (PCID on x86-64, leveraging ASID on ARM64)
- [ ] Stack switching optimized
- [ ] Instruction prefetch implemented
- [ ] Preemption points identified and documented
- [ ] Context switch latency micro-benchmark shows <1µs
- [ ] All optimizations tested on both single-socket and multi-socket systems
- [ ] No correctness issues

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Sub-microsecond context switching is production requirement
