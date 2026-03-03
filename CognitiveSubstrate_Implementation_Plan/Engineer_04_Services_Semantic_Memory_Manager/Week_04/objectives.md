# Engineer 4 — Services: Semantic Memory Manager — Week 4

## Phase: 0 — Foundation & Formalization
## Weekly Objective
Implement stub Memory Manager — L1 allocation at CT spawn, sized to model context window. Set up basic page allocation and deallocation infrastructure. Establish foundational build and test infrastructure for Phase 0.

## Document References
- **Primary:** Section 6.1 — Phase 0, Week 4-6 (Stub Memory Manager)
- **Supporting:** Section 3.3 — L1 Kernel Services, Section 2.5 — SemanticMemory

## Deliverables
- [ ] Stub Memory Manager process skeleton with IPC handler loop
- [ ] L1 Working Memory allocator (page-granule allocation, deallocation)
- [ ] Memory sizing calculation based on model context window
- [ ] MMU page table setup for CT address space mapping
- [ ] Basic heap allocator for Memory Manager's own data structures
- [ ] Unit tests for allocation/deallocation at scale
- [ ] Integration test: CT spawn with L1 memory mapped into address space

## Technical Specifications
- Implement L1 allocator supporting allocate/deallocate/resize at page granularity
- Size L1 Working Memory to model context window (establish baseline size based on architecture)
- Map allocated pages into CT address space via MMU using kernel helper functions
- Implement ref counting for shared pages in crew scenarios
- Create simple page pool management with free list
- Define memory layout: guard pages, page metadata, allocation bitmap
- No eviction or migration logic yet (stub phase)

## Dependencies
- **Blocked by:** Week 3 (kernel architecture review)
- **Blocking:** Week 5 (L1 interface definitions), Week 7 (L1 full implementation)

## Acceptance Criteria
- [ ] Stub Memory Manager compiles and links successfully
- [ ] L1 allocation works for 1K-1M page allocations
- [ ] Pages correctly mapped into CT user space
- [ ] Allocation/deallocation performance acceptable (<1ms per page)
- [ ] Basic integration test passes: spawn CT with L1 memory

## Design Principles Alignment
- **Simplicity:** Stub implementation focuses on core allocation without eviction complexity
- **Isolation:** Memory Manager process properly isolated from CT
- **Performance:** Page-granular mapping enables efficient multi-CT sharing
- **Determinism:** Allocation order deterministic for reproducible tests
