# Engineer 4 — Services: Semantic Memory Manager — Week 12

## Phase: 1 — Three-Tier Implementation
## Weekly Objective
Implement L3 Long-Term Memory (NVMe) as persistent, capability-controlled shared storage. Add memory-mapped I/O with kernel-managed prefetch. Establish distributed replication support and persistent knowledge base querying.

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 7-10 (Three-tier with prefetch, CRDT, OOC handler)
- **Supporting:** Section 2.5 — SemanticMemory, Section 3.3.1 — Semantic Memory Manager

## Deliverables
- [ ] L3 persistent storage implementation on NVMe
- [ ] Memory-mapped I/O layer for L3 access
- [ ] Capability-based access control for shared L3 regions
- [ ] Kernel-managed prefetch system (MSched-style prediction)
- [ ] Replication protocol for distributed L3 consistency
- [ ] Query interface for L3 knowledge base (capability-gated)
- [ ] Unit tests for persistent storage, replication, querying
- [ ] Integration test: store knowledge in L3, access via prefetch

## Technical Specifications
- L3 storage: append-only semantic log on NVMe (immutable history)
- Memory-mapped interface: mmap L3 pages into memory address space with lazy loading
- Capability model: CT gets read or read-write capability on shared L3 regions
- Prefetch predictor: predict what knowledge CT will need based on phase/task description
- Implement prefetch queue: pre-warm pages into L2 before CT requests them
- Replication: sync L3 updates to replica nodes (eventual consistency)
- Query API: search L3 by semantic key, vector similarity, metadata filters
- Support time-travel queries (access L3 at earlier timestamp via snapshots)

## Dependencies
- **Blocked by:** Week 10 (eviction establishes L2), Week 11 (compactor provides efficient L2)
- **Blocking:** Week 13 (OOC handler), Week 15 (knowledge source mounting)

## Acceptance Criteria
- [ ] L3 stores and retrieves data persistently across process restarts
- [ ] Prefetch latency acceptable (pages available within 10ms)
- [ ] Capability control prevents unauthorized L3 access
- [ ] Replication sync completes within 100ms
- [ ] k-NN search on L3 completes in <1s (acceptable for cold storage)
- [ ] Integration test: checkpoint L1 to L3, crash, restart, verify recovery

## Design Principles Alignment
- **Durability:** NVMe persistence enables knowledge retention across sessions
- **Performance:** Prefetch hiding reduces access latency
- **Isolation:** Capability control enables shared L3 with privacy
- **Scalability:** Replication supports distributed knowledge base
