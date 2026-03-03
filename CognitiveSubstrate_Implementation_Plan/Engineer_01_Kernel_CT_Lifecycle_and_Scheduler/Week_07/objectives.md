# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 07

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Begin implementation of Cognitive Priority Scheduler with 4-dimensional scoring. Implement Chain Criticality (0.4 weight) and Resource Efficiency (0.25 weight) scoring components. Establish scheduler scoring infrastructure.

## Document References
- **Primary:** Section 3.2.2 (Cognitive Priority Scheduler — Four-dimensional priority scoring with all four dimensions: Chain Criticality, Resource Efficiency, Deadline Pressure, Capability Cost)
- **Supporting:** Section 2.1 (CognitivePriority struct), Section 6.2 (Phase 1 exit criteria)

## Deliverables
- [ ] Rust module `scheduler_scoring.rs` — priority scoring implementation
- [ ] Chain Criticality scorer (0.4 weight) — analyze dependency DAG, score CTs unblocking most downstream work highest
- [ ] Resource Efficiency scorer (0.25 weight) — identify batch-ready CTs, score co-scheduling affinity
- [ ] Priority score calculation function — weighted sum of all dimension scores [0, 1]
- [ ] Scheduler runqueue refactored — replace round-robin with priority-based ordering
- [ ] CognitivePriority struct population — every CT spawn computes initial priority score
- [ ] Test suite — 25+ test cases covering DAG analysis, batch detection, score calculations

## Technical Specifications
**Chain Criticality (0.4 weight) — Section 3.2.2:**
- From dependency DAG, calculate how many downstream CTs each CT unblocks
- Use transitive closure: if CT_A is a dependency for CT_B which is a dependency for CT_C, CT_A's criticality includes contribution from CT_C
- Score = (downstream_count) / (total_ct_count), normalized to [0, 1]
- CTs with no dependents = 0.0; CTs blocking large chains = close to 1.0

**Resource Efficiency (0.25 weight) — Section 3.2.2:**
- Identify CTs that are ready for inference batching (same model, compatible batch sizes)
- Track co-scheduling patterns: which CTs have been scheduled together successfully
- Score = batch_readiness_factor ∈ [0, 1]
- CTs ready for batching with others = higher score

**Priority Scheduling Data Structure:**
- Change from FIFO runqueue to priority heap (min-heap by priority score)
- Pop highest-priority CT at each scheduling event
- Maintain per-CT priority score, update on phase transitions

**Example Scenario:**
```
CT_A (no deps, batch-ready): Chain_Criticality=0.2, Resource_Efficiency=0.8, score = 0.4*0.2 + 0.25*0.8 + ... = ...
CT_B (depends on A, not batch-ready): Chain_Criticality=0.8, Resource_Efficiency=0.1, score = 0.4*0.8 + 0.25*0.1 + ... = ...
CT_C (depends on A,B): Chain_Criticality=0.1, Resource_Efficiency=0.0, score = ...
```

## Dependencies
- **Blocked by:** Week 06 (Phase 0 complete), Engineer 5 (GPU Manager interface for batching hints)
- **Blocking:** Week 08 (Deadline Pressure dimension), Week 09 (Capability Cost dimension)

## Acceptance Criteria
- [ ] Chain Criticality scorer implemented and tested with 15+ DAG topologies
- [ ] Resource Efficiency scorer identifies batch-ready CTs correctly
- [ ] Priority score correctly calculated as weighted sum (0.4*chain + 0.25*efficiency + ...)
- [ ] Scheduler runqueue uses priority heap, always schedules highest-score CT next
- [ ] All 25+ test cases pass
- [ ] Backward compatibility: Phase 0 round-robin tests still pass (priority-based scheduling is superset)
- [ ] Integration test: 100 CTs with dependencies, verify critical path CTs get higher priority

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Priority scheduling is production requirement, not optimization
- **P2 — Cognitive Primitives as Kernel Abstractions:** Cognitive priority scheduling is kernel responsibility
