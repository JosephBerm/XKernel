# Engineer 2 — Kernel: Capability Engine & Security — Week 22

## Phase: PHASE 2 - Data Governance & Performance

## Weekly Objective
Complete KV-cache isolation with comprehensive performance validation, real-world LLM workload testing, and final hardening. Verify SELECTIVE mode achieves <10% p95 TTFT overhead on production models.

## Document References
- **Primary:** Section 3.3.2 (KV-Cache Isolation Performance SLO), Section 3.3.2 (PROMPTPEEK Defense)
- **Supporting:** Week 20-21 (KV-cache implementation), Engineer 7 (crew scheduling)

## Deliverables
- [ ] Production LLM workload testing (GPT-3-scale, LLaMA 30B models)
- [ ] Performance profiling and optimization for each isolation mode
- [ ] Cache hit rate analysis and improvement
- [ ] Latency breakdown analysis (cache lookup, coherency, eviction)
- [ ] Throughput validation under realistic load
- [ ] Memory efficiency report (per-crew overhead analysis)
- [ ] Integration with crew scheduling and load balancing
- [ ] PROMPTPEEK defense validation (cache side-channel mitigation)
- [ ] Final KV-cache isolation audit and sign-off

## Technical Specifications
- **Production LLM Workload Testing:**
  - Model 1: LLaMA 13B (representative mid-size)
    - Batch size: 32
    - Sequence length: 128 (prefill), 1 (decode)
    - Crew count: 3 (multitenancy)
    - Isolation mode: SELECTIVE
    - Target TTFT: <55ms (10% overhead)
    - Target TPS: >90
  - Model 2: GPT-3-scale (large model, high memory pressure)
    - Batch size: 64
    - Sequence length: 256 (prefill), 1-16 (decode)
    - Crew count: 5 (more contention)
    - Isolation mode: SELECTIVE
    - Target TTFT: <150ms (10% overhead)
    - Target TPS: >30
  - Model 3: LLaMA 30B (largest tested)
    - Batch size: 16
    - Sequence length: 256 (prefill), 1-8 (decode)
    - Crew count: 3
    - Isolation mode: STRICT + SELECTIVE (mixed)
    - Target TTFT: <200ms (15% overhead)
    - Target TPS: >15
- **Performance Profiling:**
  - Hot path: cache lookup in main loop
    - Baseline: 50ns (direct memory access)
    - SELECTIVE: +10ns (TLB check, permission validation)
    - STRICT: no overhead (no sharing)
    - Optimization: inline cache lookup in compiler
  - Coherency: TLB shootdown on crew switch
    - Baseline: <1000ns (IPI latency)
    - Optimization: batch multiple switches, amortize cost
  - Eviction: cache eviction on quota exceeded
    - Cost: <5000ns per eviction (LRU update + memory write)
    - Optimization: lazy eviction (defer until next miss)
- **Cache Hit Rate Analysis:**
  - Baseline (single crew): 90% hit rate (typical for LLM)
  - SELECTIVE (3 crews): 85% hit rate (5% degradation from sharing)
  - STRICT (3 crews): 90% hit rate (no sharing overhead)
  - Optimization 1: prefetching (predict next tokens)
  - Optimization 2: compression (store cache entries compressed)
  - Goal: >85% hit rate for SELECTIVE (acceptable tradeoff)
- **Latency Breakdown:**
  - Prefill phase (all tokens in initial input):
    - OPEN: 40ms (baseline)
    - SELECTIVE: 45ms (+12.5%, acceptable)
    - STRICT: 55ms (+37.5%, acceptable for high-security deployments)
  - Decode phase (one token at a time):
    - OPEN: 20ms (baseline)
    - SELECTIVE: 22ms (+10%, target met)
    - STRICT: 25ms (+25%, acceptable)
- **Throughput Validation:**
  - LLaMA 13B under load:
    - Single crew: 100 TPS (baseline)
    - 3 SELECTIVE crews: 300 TPS total (100 TPS per crew)
    - 3 STRICT crews: 280 TPS total (93 TPS per crew, 7% overhead)
  - GPT-3 scale under load:
    - Single crew: 30 TPS
    - 5 SELECTIVE crews: 140 TPS total (28 TPS per crew)
    - 5 STRICT crews: 130 TPS total (26 TPS per crew)
- **Memory Efficiency:**
  - LLaMA 13B cache footprint:
    - Baseline: 2GB per batch
    - OPEN mode (1 crew): 2GB
    - SELECTIVE (3 crews): 2GB (shared)
    - STRICT (3 crews): 6GB (3x duplication)
  - Analysis: SELECTIVE saves 4GB vs STRICT for 3 crews
  - Conclusion: SELECTIVE is memory-efficient for multi-crew
- **Crew Scheduling Integration:**
  - Scheduler: decides which crews run on which cores
  - Isolation impact: affects cache coherency traffic
  - Optimization: schedule crews with compatible isolation modes together
  - Example: STRICT crews on dedicated cores, SELECTIVE crews on shared cores
  - Metric: coherency traffic reduced by 30% with smart scheduling
- **PROMPTPEEK Defense Validation:**
  - Threat: adversary measures cache hit/miss timing to infer prompt content
  - Defense 1: constant-time cache access (padding with dummy accesses)
  - Defense 2: randomized eviction (unpredictable cache state)
  - Defense 3: noise injection (intentional cache misses)
  - Validation: timing measurements show <5% variance (indistinguishable from noise)
  - Test: adversary cannot distinguish "password" from "harmless" with >55% accuracy

## Dependencies
- **Blocked by:** Week 21 (KV-cache isolation advanced scenarios), Engineer 7 (crew scheduling)
- **Blocking:** Week 23-24 (general performance tuning), Phase 3 (weeks 25+)

## Acceptance Criteria
- LLaMA 13B with SELECTIVE achieves <55ms TTFT (10% overhead)
- GPT-3-scale with SELECTIVE achieves <150ms TTFT (10% overhead)
- LLaMA 30B with STRICT achieves <200ms TTFT (15% overhead)
- All throughput targets met (TPS measurements)
- Cache hit rate maintained >85% in SELECTIVE mode
- Memory efficiency meets expectations (SELECTIVE shares, STRICT isolates)
- PROMPTPEEK timing side-channel defeated (variance <5%)
- Crew scheduling integration optimizes cache coherency
- All performance benchmarks documented and validated
- Code review completed by performance and security teams

## Design Principles Alignment
- **P1 (Security-First):** PROMPTPEEK defense prevents timing side-channels
- **P3 (Granular Control):** Three isolation modes support different threat models
- **P4 (Performance):** <10% overhead for SELECTIVE meets production requirements
- **P7 (Multi-Agent Harmony):** Crew scheduling integration enables efficient multitenancy
