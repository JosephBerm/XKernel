# Engineer 4 — Services: Semantic Memory Manager — Week 2

## Phase: 0 — Foundation & Formalization
## Weekly Objective
Continue SemanticMemory entity formalization with detailed semantics for operation atomicity, ordering guarantees, and concurrency models. Define the embedded semantic indexing system for L2 and establish the capability control model for tier interactions.

## Document References
- **Primary:** Section 2.5 — SemanticMemory (three-tier model with kernel operations per tier)
- **Supporting:** Section 3.3.1 — Semantic Memory Manager (detailed implementation)

## Deliverables
- [ ] Concurrency and atomicity specification for tier-specific operations
- [ ] Embedded vector indexing system design (no external pgvector server)
- [ ] Capability control model specification for cross-tier access
- [ ] Memory layout diagrams for L1, L2, L3 with size bounds
- [ ] Eviction policy formalization (Spill-First, Compact-Later model)
- [ ] Ordering and consistency guarantees documentation

## Technical Specifications
- Formalize semantics of L1 allocate/resize with concurrent CT access
- Define L2 semantic search via embedded vectors (dimension, quantization, distance metrics)
- Specify L2 merge and expire operations with ACID properties
- Define L3 persistent storage layout and replication model
- Establish memory pressure thresholds triggering inter-tier migration
- Specify page-level granularity for remapping operations (O(1) physical page remapping)

## Dependencies
- **Blocked by:** Week 1 (basic entity definitions)
- **Blocking:** Week 3 (kernel architecture review), Week 4 (stub implementation)

## Acceptance Criteria
- [ ] Concurrency model defined and reviewed for race-free operation
- [ ] Embedded semantic indexing approach validated for performance
- [ ] Capability control model supports isolation and delegation
- [ ] Memory layout fits target architecture (HBM capacity, DRAM budget, NVMe access patterns)
- [ ] Eviction policies mathematically specified with invariants

## Design Principles Alignment
- **Determinism:** Atomic operation semantics enable reproducible memory behavior
- **Efficiency:** Spill-First, Compact-Later minimizes copying overhead
- **Isolation:** Capability model prevents unauthorized tier access
- **Simplicity:** No external dependency on pgvector server (embedded indexing)
