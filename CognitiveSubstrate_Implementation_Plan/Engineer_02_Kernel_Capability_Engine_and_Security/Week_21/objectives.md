# Engineer 2 — Kernel: Capability Engine & Security — Week 21

## Phase: PHASE 2 - Data Governance & Performance

## Weekly Objective
Complete KV-cache isolation implementation with advanced scenarios including cache eviction policies, cross-team information flow, and sophisticated isolation breaches testing.

## Document References
- **Primary:** Section 3.3.2 (KV-Cache Isolation - Advanced Scenarios), Section 3.3.2 (PROMPTPEEK Defense)
- **Supporting:** Week 20 (KV-cache isolation modes), Section 3.3.2 (KV-Cache Isolation Threats)

## Deliverables
- [ ] Cache eviction policies for SELECTIVE mode (LRU, LFU, adaptive)
- [ ] Cross-team information flow control (prevent leakage via cache side-effects)
- [ ] Preemption handling (cache invalidation on crew switch)
- [ ] Cache warmup and priming (initial state isolation)
- [ ] Comprehensive test suite (200+ tests for isolation scenarios)
- [ ] Cache isolation bypass attempts (adversarial testing)
- [ ] Performance optimization for cache coherency
- [ ] Integration with crew scheduling (Engineer 7)
- [ ] Cache isolation documentation and best practices

## Technical Specifications
- **Cache Eviction Policies for SELECTIVE Mode:**
  - Challenge: SELECTIVE mode shares physical cache pages
  - LRU policy: evict least-recently-used entry across all crews
    - Issue: one crew can cause another's data to be evicted
    - Mitigation: per-crew LRU sub-queues, proportional eviction
  - LFU policy: evict least-frequently-used entry
    - Issue: frequency can be manipulated by noisy crew
    - Mitigation: access frequency tracked separately per crew
  - Adaptive policy: hybrid LRU + priority based on isolation_mode
    - Rule 1: STRICT-mode data never evicted by SELECTIVE crew
    - Rule 2: SELECTIVE crew data evicts other SELECTIVE data preferentially
    - Rule 3: OPEN crew uses residual cache space (best-effort)
  - Eviction guard: cache never shrinks below minimum per crew
- **Cross-Team Information Flow Control:**
  - Threat 1: Crew_A fills cache, Crew_B evicts A's data → infers A's patterns
  - Defense: quota-based eviction (Crew_A controls its own eviction)
  - Threat 2: Cache hit rate varies based on Crew_B's data → timing side-channel
  - Defense: constant-time cache access patterns (padding if needed)
  - Threat 3: Memory bandwidth contention reveals computation patterns
  - Defense: rate limiting (bandwidth allocated per crew)
  - Threat 4: TLB misses reveal cache organization
  - Defense: deterministic TLB miss patterns (no information leakage)
- **Preemption Handling:**
  - Scenario: Crew_A using cache, preempted, Crew_B resumes
  - Safe approach: flush entire cache on preemption (expensive)
  - Optimized approach: TLB shootdown + selective cache flush
    - Crew_B can see Crew_A's cache entries (in physical memory)
    - But: page table unmaps prevent Crew_B from accessing (TLB miss)
    - On re-preemption: Crew_A cache entries still in physical cache
    - Clean: reuse cache if ownership verified via page tables
  - Security: verified at every cache hit (capability required)
- **Cache Warmup and Priming:**
  - Challenge: cache cold on first inference of crew (worse performance)
  - Warmup: pre-fill cache with dummy data for initialization
  - Privacy: dummy data is crew-specific and random (no leakage)
  - Performance: amortized across multiple inferences
  - Timing: warmup latency added to first inference only
- **Comprehensive Test Suite (200+ Tests):**
  - Category 1: Basic isolation (50 tests)
    - Test: Crew_A cannot read Crew_B cache data
    - Test: Cache coherency maintained
    - Test: Cache eviction doesn't leak information
  - Category 2: Preemption scenarios (40 tests)
    - Test: Cache state isolation after preemption
    - Test: Concurrent preemption of multiple crews
    - Test: Preemption during cache miss
  - Category 3: Information flow (50 tests)
    - Test: Hit rate doesn't reveal computation
    - Test: Timing side-channels mitigated
    - Test: Bandwidth contention doesn't leak patterns
  - Category 4: Performance (40 tests)
    - Test: Latency within target for each mode
    - Test: Throughput under load
    - Test: Cache efficiency (hit rate)
  - Category 5: Edge cases (20 tests)
    - Test: Full cache (eviction policy)
    - Test: Rapid preemption
    - Test: Mixed isolation modes
- **Adversarial Isolation Breach Attempts:**
  - Attack 1: Access cache via unintended side-channel → blocked by capability checks
  - Attack 2: Timing attack on cache hits/misses → mitigated by constant-time access
  - Attack 3: Eviction attack (fill cache to force eviction) → rate limiting prevents
  - Attack 4: Bandwidth attack (contend for bandwidth) → bandwidth allocation prevents
  - Attack 5: Preemption race (preempt during cache update) → atomic updates prevent
  - Attack 6: TLB poisoning (map wrong cache pages) → verified at each access
  - Attack 7: Collusion (multiple crews cooperate) → isolation enforced at capability level
  - Attack 8: Speculative execution (speculate into Crew_B cache) → CPU speculation barriers

## Dependencies
- **Blocked by:** Week 20 (KV-cache isolation modes), Engineer 7 (crew scheduling)
- **Blocking:** Week 22 (completion and tuning), Week 23-24 (performance optimization)

## Acceptance Criteria
- Cache eviction policies prevent cross-crew information leakage
- Cross-team information flow control defeats all identified attacks
- Preemption handling maintains isolation across context switches
- Cache warmup enables efficient initialization
- All 200+ tests pass with >95% code coverage
- All adversarial attacks prevented or mitigated
- Performance targets met (within 10% p95 TTFT overhead for SELECTIVE)
- Cache coherency maintained even under preemption
- Code review completed by security and systems teams

## Design Principles Alignment
- **P1 (Security-First):** Eviction policies prevent information leakage
- **P2 (Transparency):** Cache isolation strategies are documented
- **P3 (Granular Control):** Per-crew cache quotas enable fine-grained control
- **P4 (Performance):** Optimized eviction maintains performance
- **P7 (Multi-Agent Harmony):** Crew scheduling integration enables crew-aware isolation
