# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 09

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Implement crew-aware scheduling affinity. Ensure CTs in the same AgentCrew receive scheduling affinity, pinned to the same NUMA node to maximize shared memory locality and crew synchronization.

## Document References
- **Primary:** Section 3.2.2 (Crew-Aware Scheduling: CTs in the same AgentCrew receive scheduling affinity — pinned to the same NUMA node to maximize shared memory locality)
- **Supporting:** Section 2.3 (AgentCrew entity with scheduling_affinity: AffinityPolicy), Section 2.13 (CT → AgentCrew relationship)

## Deliverables
- [ ] Rust module `crew_scheduler.rs` — crew-aware scheduling with NUMA affinity
- [ ] Crew affinity tracking — map crew ID to NUMA node assignment
- [ ] NUMA topology discovery — detect NUMA nodes on boot, map to CPU cores
- [ ] Affinity binding — when spawning CT with crew membership, assign to crew's NUMA node
- [ ] Crew migration — if crew grows dynamically, rebalance crew members across NUMA nodes
- [ ] Shared memory optimization — crew CTs on same NUMA node access shared L3 memory with lower latency
- [ ] Scheduler affinity enforcement — runqueue per NUMA node, schedule crew CTs to their assigned node
- [ ] Test suite — 15+ test cases covering crew creation, affinity binding, dynamic crew growth, NUMA locality verification

## Technical Specifications
**Crew-Aware Scheduling (Section 3.2.2):**
- Every AgentCrew has scheduling_affinity: AffinityPolicy (prefer co-scheduling members to same NUMA node)
- On CT spawn with crew, query crew's assigned NUMA node
- Allocate context_window (L1 memory) from crew's NUMA node HBM
- Allocate shared_memory (L3 memory) accessible to all crew members on that node
- Schedule all crew CTs on CPU cores within same NUMA node when possible

**NUMA Topology:**
- Example: 2-socket system, 16 cores per socket
  - NUMA node 0: cores 0-15, local HBM @ 0x0-0x7FFFFFFF
  - NUMA node 1: cores 16-31, local HBM @ 0x100000000-0x17FFFFFFF
- Kernel discovers topology at boot via ACPI SRAT table (x86-64) or device tree (ARM64)

**Affinity Policy:**
- STRICT: all crew members must run on same node (may block if oversubscribed)
- PREFER: crew members prefer same node, but can overflow to adjacent node if overloaded
- RELAXED: no affinity requirement (default for loose crews)

**Shared Memory Benefits (Section 3.3.1):**
- Same NUMA node = same L3 cache (typically 20MB per node)
- Shared memory region L3MemoryRef mapped read-write into all crew CTs on that node
- Cache coherency traffic stays local; no cross-node coherency traffic
- Expected benefit: 10-30% latency reduction for crew coordination (requires profiling)

## Dependencies
- **Blocked by:** Week 08 (basic 4-dimensional priority scheduler), Engineer 4 (Memory Manager must allocate from NUMA-aware pools)
- **Blocking:** Week 10 (deadlock detection with crew context), Week 13 (demo preparation with 3-agent crew)

## Acceptance Criteria
- [ ] NUMA topology discovered and logged at boot
- [ ] Crew affinity policy enforced at CT spawn
- [ ] All crew CTs allocated to same NUMA node's HBM
- [ ] Shared L3 memory region accessible to all crew members
- [ ] Scheduler respects NUMA locality when scheduling crew CTs
- [ ] All 15+ test cases pass
- [ ] Performance test: 3-agent crew on same NUMA node vs different nodes, measure memory latency difference
- [ ] Integration test: spawn 3-agent crew, verify all CTs on same NUMA node

## Design Principles Alignment
- **P2 — Cognitive Primitives as Kernel Abstractions:** AgentCrew scheduling is fundamental kernel abstraction
- **P7 — Production-Grade from Phase 1:** NUMA awareness is production requirement for multi-socket systems
