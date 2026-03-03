# Engineer 2 — Kernel: Capability Engine & Security — Week 6

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Optimize local capability checks as O(1) handle lookups into kernel capability table achieving <100ns per check. Implement caching layer for hot-path capability checks. Cryptographic signatures reserved for distributed trust boundaries only.

## Document References
- **Primary:** Section 3.2.3 (Local Capability Checks - O(1) Handle Lookups), Section 3.2.4 (Distributed IPC - Cryptographic Signatures)
- **Supporting:** Section 3.2.3 (Capability Enforcement Engine), Section 2.4 (Capability Formalization)

## Deliverables
- [ ] Kernel capability table hash map optimization (O(1) lookup guarantees)
- [ ] L1 cache-friendly data layout for capability lookups (cache-line aligned, prefetch-optimized)
- [ ] Hot-path capability check fast path implementation (<50ns)
- [ ] Slow-path capability check fallback (>100ns, handles edge cases)
- [ ] Per-core capability check caching layer (thread-local caches with invalidation)
- [ ] Cache invalidation protocol for Revoke operation
- [ ] Benchmarking suite (latency distribution, cache hit rates, contention profiles)
- [ ] Cryptographic signature infrastructure reserved for distributed boundaries (no local usage)
- [ ] Performance validation achieving <100ns p99 latency
- [ ] Documentation of cache coherency and invalidation protocol

## Technical Specifications
- **Kernel Capability Table:**
  - Hash map: CapID (256-bit hash) → (Capability struct, derived_caps, page_tables)
  - Hash function: BLAKE3 (cryptographically secure, fast)
  - Collision handling: chaining with minimal overhead
  - Lock-free reads via seqlock or RCU (Read-Copy-Update) pattern
  - Per-bucket locks for updates (fine-grained locking, not global lock)
  - Memory layout: align capability entries to cache line boundary (64 bytes)
- **Fast Path (<50ns):**
  - Input: agent_id, capid, operation
  - Lookup: hash(capid) → cached_capability struct
  - Validation: capability.holder includes agent_id AND operation in capability.operations
  - Return: success or error (no allocations, no system calls)
  - Typical code path: <10 instructions (hash lookup + bounds checks)
- **Slow Path (fallback for >100ns cases):**
  - Handles revocation chain traversal (for derived capabilities)
  - Handles complex constraint evaluation (time bounds, rate limits, data volume)
  - Handles policy checks if not cached
  - May allocate temporary buffers for constraint evaluation
  - Called <1% of the time in typical workloads
- **Per-Core Caching:**
  - Thread-local L1 cache: 256 entries (recent capability checks)
  - Cache key: (agent_id, capid, operation)
  - Cache value: (result, validation_epoch)
  - Validation epoch: incremented on each Revoke → automatic cache invalidation
  - Cache hit rate target: >95% in steady state
- **Cache Invalidation Protocol:**
  - Revoke operation increments global validation_epoch
  - All thread-local caches see epoch change via memory barrier
  - Thread-local cache entries compare cached_epoch vs current_epoch
  - Mismatch → cache miss → slow path lookup
  - No IPI or explicit invalidation needed (passive detection)
- **Cryptographic Signatures:**
  - Reserved exclusively for distributed IPC at network ingress/egress
  - Local kernel operations use CapID handles (non-cryptographic)
  - Inter-agent IPC within same kernel: CapID handles only
  - Cross-kernel IPC: cryptographic signature over (CapID, delegation_chain, constraints)
  - Signature algorithm: Ed25519 (deterministic, fast verification)

## Dependencies
- **Blocked by:** Week 3-5 (Capability Enforcement Engine, MMU integration)
- **Blocking:** Week 7-14 (Phase 1 - multi-agent capability delegation and IPC), Week 11 (Distributed IPC)

## Acceptance Criteria
- Kernel capability table lookup is O(1) with typical case <5 instructions
- Fast path achieves <50ns p50, <100ns p99 latency
- Per-core caching achieves >95% hit rate in steady state
- Slow path fallback handles all complex scenarios correctly
- Cache invalidation protocol causes zero explicit IPI overhead
- Benchmarking suite demonstrates <100ns p99 across all core counts (1-16 cores)
- Cryptographic signatures are used ONLY at distributed boundaries
- All capability checks maintain <100ns latency guarantee
- Code review completed by performance engineering team

## Design Principles Alignment
- **P1 (Security-First):** O(1) checksecurity-critical path prevents timing-based side channels
- **P2 (Transparency):** Cache invalidation protocol is deterministic and auditable
- **P4 (Performance):** <100ns capability checks enable high-throughput AI workloads
- **P5 (Formal Verification):** O(1) hash table lookup can be formally verified
- **P7 (Multi-Agent Harmony):** Per-core caching avoids cross-core contention
- **P8 (Robustness):** Slow path fallback ensures correctness even under cache pressure
