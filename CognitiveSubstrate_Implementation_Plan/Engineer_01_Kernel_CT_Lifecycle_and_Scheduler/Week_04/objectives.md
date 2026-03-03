# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 04

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Implement CT dependency DAG with cycle detection at spawn time. Reject circular dependencies with clear error messages. Ensure CT cannot enter reason phase until all dependencies complete.

## Document References
- **Primary:** Section 2.1 (Invariant 5 — Dependency DAG is cycle-checked at spawn; circular deps rejected), Section 3.2.2 (Deadlock Prevention — static cycle detection on CT dependency DAG at spawn time)
- **Supporting:** Section 2.13 (CT → dependencies: DAG<CTRef>), Section 6.1 (Phase 0 Exit Criteria)

## Deliverables
- [ ] Rust module `dependency_dag.rs` — DAG<CTRef> type and cycle detection algorithm
- [ ] Tarjan's strongly connected components algorithm for cycle detection
- [ ] Spawn-time validation — ct_spawn rejects spawn if dependencies form cycle
- [ ] Dependency wait list — track which CTs block each CT from entering reason phase
- [ ] Notification mechanism — when a dependency completes, notify dependent CTs
- [ ] Test suite — 20+ test cases covering linear chains, trees, DAGs, and cycle attempts
- [ ] Error messages — clear feedback when cycle detected (list all CTs in cycle)

## Technical Specifications
**Cycle Detection (Section 3.2.2):**
- Static detection at spawn time using DFS or Tarjan's SCC algorithm
- O(V + E) where V = number of CTs, E = number of dependency edges
- On cycle detection, reject spawn and return error: SpawnCycleDetected { ct_ids: Vec<ULID> }

**Dependency Constraints (Section 2.1):**
- Invariant 3: All dependencies must complete before CT enters reason phase
- Invariant 5: Dependency DAG cycle-checked at spawn time; circular dependencies rejected
- Implementation: before phase transition spawn→plan→reason, verify all CT IDs in dependencies set are in complete state

**Example Test Cases:**
- Linear chain: CT1 → CT2 → CT3 (valid, should spawn)
- Tree: CT1 → {CT2, CT3} → CT4 (valid)
- Simple cycle: CT1 → CT2 → CT1 (invalid, reject at spawn)
- Multi-level cycle: CT1 → CT2 → CT3 → CT1 (invalid)
- Self-loop: CT1 → CT1 (invalid)
- Diamond: CT1 → {CT2, CT3} → CT4, CT2 also depends on CT3 (valid, CT4 waits for CT2 and CT3 completion)

## Dependencies
- **Blocked by:** Week 03 (round-robin scheduler to test with)
- **Blocking:** Week 05 (capability engine), Week 04-06 integration testing

## Acceptance Criteria
- [ ] Cycle detection algorithm implemented and tested with 20+ cases
- [ ] ct_spawn rejects circular dependencies with error
- [ ] CT cannot transition to reason phase if any dependency incomplete
- [ ] Dependency completion notifies all dependent CTs waiting on that CT
- [ ] Performance: cycle detection completes in <1ms for 1000 CT dependency graph
- [ ] All test cases pass; no false positives or false negatives
- [ ] Integration test: spawn 100 CTs with random dependencies, verify no deadlocks

## Design Principles Alignment
- **P8 — Fault-Tolerant by Design:** Cycle detection prevents infinite wait deadlocks at spawn time
- **P5 — Observable by Default:** Clear error messages when cycles detected aid debugging
- **P7 — Production-Grade from Phase 1:** Formal deadlock prevention is production requirement
