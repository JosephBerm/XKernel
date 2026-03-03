# Engineer 2 — Kernel: Capability Engine & Security — Week 20

## Phase: PHASE 2 - Data Governance & Performance

## Weekly Objective
Begin KV-Cache isolation implementation via page tables with three modes: STRICT (separate pages per crew), SELECTIVE (isolation-by-default), and OPEN (single-tenant). Establish isolation architecture and baseline performance.

## Document References
- **Primary:** Section 3.3.2 (KV-Cache Isolation via Page Tables - 3 Modes), Section 3.3.2 (KV-Cache Isolation Performance SLO)
- **Supporting:** Section 3.2.3 (MMU Integration), Section 2.4 (Capability Constraints)

## Deliverables
- [ ] STRICT isolation mode implementation (separate cache pages per crew)
- [ ] SELECTIVE isolation mode implementation (shared with permission checks)
- [ ] OPEN isolation mode implementation (no isolation, single-tenant)
- [ ] Mode selection mechanism (policy configuration per model/crew)
- [ ] KV-cache page table mapping strategy
- [ ] Cache coherency protocol for multi-core systems
- [ ] Baseline performance measurement for all three modes
- [ ] Integration with capability system (cache access requires capability)
- [ ] Documentation of isolation modes and tradeoffs

## Technical Specifications
- **STRICT Isolation Mode:**
  - Behavior: each crew has completely separate KV-cache pages
  - Cache structure: M-model 13B → 2*batch_size*seq_len*hidden_dim bytes
  - Per-crew allocation: separate physical pages for each crew
  - Page table mapping: Crew_A pages not mapped in Crew_B page tables
  - Hardware enforcement: Crew_B cannot access Crew_A pages (TLB miss)
  - Guarantee: zero information leakage between crews
  - Memory overhead: 3x cache size (one per crew) for 3 crews
  - Latency overhead: none (no cache sharing, no invalidation)
  - Use case: strict isolation required (multi-tenant production)
- **SELECTIVE Isolation Mode:**
  - Behavior: shared cache pages with permission-based access control
  - Cache structure: single shared KV-cache for all crews
  - Page table mapping: all crews mapped to cache pages
  - Permission bits: read-only for crew in other sequence positions
  - Capability requirement: READ_KV_CACHE capability for access
  - Invalidation: when crew switches, update PTE permissions atomically
  - Information leakage: bounded (only within same batch)
  - Memory overhead: 1x cache size (shared)
  - Latency overhead: <10% p95 TTFT overhead (target)
  - Use case: balanced isolation and performance (most deployments)
- **OPEN Isolation Mode:**
  - Behavior: no isolation, single logical cache for all crews
  - Cache structure: single KV-cache serving all crews
  - Page table mapping: all crews can read-write (no restrictions)
  - Capability requirement: none (cache read/write is implicit)
  - Information leakage: full KV-cache visible to all crews
  - Memory overhead: 1x cache size
  - Latency overhead: none (most efficient)
  - Use case: single-tenant inference (development, testing)
  - Security: not suitable for multi-tenant with sensitive data
- **Mode Selection Mechanism:**
  - Policy entry: model_id → isolation_mode
  - Example 1: gpt4_production → SELECTIVE (balanced)
  - Example 2: llama13b_research → OPEN (efficiency)
  - Example 3: medical_llm → STRICT (high security)
  - Dynamic switch: supported with cache flush (brief latency impact)
  - Override: admin can override for specific inference runs
- **KV-Cache Page Table Mapping:**
  - Mapping structure: KV-cache pages → (crew_id, batch_position, seq_position)
  - Entry metadata: (isolation_mode, permission_bits, generation_id)
  - Update strategy: TLB shootdown on mode change (IPI to all cores)
  - Caching: K-L3 cache efficient (locality of reference in SELECTIVE mode)
  - Eviction: LRU replacement policy for cache entries
- **Cache Coherency Protocol:**
  - Challenge: multiple cores reading/writing same cache pages
  - SELECTIVE mode: readers don't conflict (read-only permission)
  - STRICT mode: no sharing (no coherency issue)
  - OPEN mode: requires coherency protocol
  - Protocol: write-through with immediate invalidation
    - Writer: invalidates peer caches (IPI)
    - Reader: reads from main memory if peer TLB-shootdown received
  - Latency: <1000ns for cache coherency maintenance
- **Baseline Performance Measurement:**
  - Model: LLaMA 13B (baseline)
  - Batch size: 32
  - Sequence length: 128 (prefill), 1 (decode)
  - Metrics: Time-To-First-Token (TTFT), tokens per second (TPS)
  - Baseline (OPEN): 50ms TTFT, 100 TPS
  - SELECTIVE target: <55ms TTFT (10% overhead), 90+ TPS
  - STRICT target: <70ms TTFT (40% overhead), 70+ TPS

## Dependencies
- **Blocked by:** Week 1-19 (capability engine, data governance, output gates)
- **Blocking:** Week 21-22 (advanced KV-cache scenarios), Week 23-24 (performance tuning)

## Acceptance Criteria
- STRICT mode provides complete isolation with zero leakage
- SELECTIVE mode achieves <10% p95 TTFT overhead
- OPEN mode has no performance overhead
- Mode selection mechanism is flexible and easy to configure
- KV-cache page table mapping is efficient and correct
- Cache coherency protocol maintains consistency
- All three modes have correct isolation properties
- Baseline performance meets or exceeds targets
- Code review completed by systems and performance teams

## Design Principles Alignment
- **P1 (Security-First):** STRICT mode guarantees no information leakage
- **P3 (Granular Control):** Three modes enable fine-grained isolation selection
- **P4 (Performance):** SELECTIVE achieves performance-security balance
- **P8 (Robustness):** Cache coherency ensures correctness even under contention
