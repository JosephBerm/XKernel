# Engineer 4 — Services: Semantic Memory Manager — Week 10

## Phase: 1 — Three-Tier Implementation
## Weekly Objective
Implement Spill-First, Compact-Later eviction from L1→L2 with O(1) physical page remapping. Build the eviction trigger logic and page migration pipeline that minimizes copying overhead and latency impact.

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 7-10 (Three-tier with prefetch, CRDT, OOC handler)
- **Supporting:** Section 2.5 — SemanticMemory (eviction policies)

## Deliverables
- [ ] Memory pressure monitoring system with configurable thresholds
- [ ] L1→L2 eviction trigger logic and scheduling
- [ ] Page remapping pipeline (remap HBM pages to DRAM via MMU)
- [ ] O(1) physical page remapping implementation (update page tables only)
- [ ] Eviction policy: select lowest-priority L1 pages for spill
- [ ] Unit tests for eviction under sustained memory pressure
- [ ] Integration test: allocate beyond L1 capacity, verify spill to L2

## Technical Specifications
- Define L1 pressure threshold (e.g., 85% utilization triggers eviction)
- Implement priority scoring for L1 pages (recency, frequency, semantic relevance)
- Use CLOCK or aging algorithm for priority tracking (low overhead)
- Remapping mechanism: update CT page tables to point to L2 DRAM pages
- No data copying during remapping (physical page pointer update only)
- Track page migration metadata (source, destination, timestamp)
- Support prefetch hints: eagerly spill low-priority pages before pressure
- Implement rate limiting on concurrent evictions to avoid thrashing

## Dependencies
- **Blocked by:** Week 9 (L2 implementation)
- **Blocking:** Week 11 (background compactor), Week 12 (L3 integration)

## Acceptance Criteria
- [ ] Eviction latency <1ms per page (remapping only, no copying)
- [ ] Pages correctly remapped to L2 and accessible via L1 handle
- [ ] Priority algorithm selects appropriate eviction candidates
- [ ] System remains responsive under sustained memory pressure
- [ ] Integration test: 2x L1 allocation → spill → access from L2
- [ ] Performance metrics demonstrate O(1) remapping overhead

## Design Principles Alignment
- **Efficiency:** O(1) remapping eliminates copying cost
- **Performance:** Spill-First policy triggers before emergency
- **Determinism:** Eviction order predictable via priority algorithm
- **Safety:** Remapped pages remain accessible to CT without intervention
