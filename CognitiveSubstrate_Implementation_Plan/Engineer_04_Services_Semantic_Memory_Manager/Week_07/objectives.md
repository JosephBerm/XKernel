# Engineer 4 — Services: Semantic Memory Manager — Week 7

## Phase: 1 — Three-Tier Implementation
## Weekly Objective
Implement L1 Working Memory (HBM) at production scale. Build full allocation, sizing, and multi-CT crew shared memory support with proper reference counting. Establish memory protection and isolation for read-only vs. read-write sharing.

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 7-10 (Three-tier with prefetch, CRDT, OOC handler)
- **Supporting:** Section 2.5 — SemanticMemory, Section 3.3.1 — Semantic Memory Manager

## Deliverables
- [ ] L1 Working Memory allocator with crew shared memory support
- [ ] Reference counting system for shared pages
- [ ] MMU configuration for read-only and read-write sharing
- [ ] L1 memory sizing logic based on model context and HBM capacity
- [ ] Crew memory coherence tracking (version vectors or timestamps)
- [ ] Unit tests for single-CT and multi-CT allocation scenarios
- [ ] Memory protection validation tests

## Technical Specifications
- Implement L1 allocation supporting per-CT isolation and crew sharing
- Support capability-based access control: CT can create read-only view of its pages
- Multiple CTs map same physical pages with selective r/o or r/w permissions
- Implement reference counting for physical pages (garbage collect when rc=0)
- Support resize operations for L1 allocation (remap pages, update MMU tables)
- Optimize for HBM access latency (microsecond-scale operations)
- Implement coherence protocol for shared regions (basic: invalidate on write)

## Dependencies
- **Blocked by:** Week 6 (Phase 0 foundation complete)
- **Blocking:** Week 8 (continue L1 implementation), Week 9 (L2 implementation)

## Acceptance Criteria
- [ ] L1 allocator handles 100+ concurrent allocations correctly
- [ ] Crew shared memory correctly mapped to multiple address spaces
- [ ] Reference counting prevents use-after-free
- [ ] Memory protection domain isolation verified
- [ ] Integration test: two CTs allocate, one creates read-only view of other's memory
- [ ] Microsecond-scale access latency demonstrated

## Design Principles Alignment
- **Isolation:** Per-CT address spaces with capability-gated sharing
- **Performance:** HBM placement and MMU mapping enable microsecond access
- **Correctness:** Reference counting and coherence tracking prevent memory safety bugs
- **Efficiency:** Shared pages reduce working set for crew scenarios
