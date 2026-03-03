# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 10

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Implement runtime wait-for graph for dynamic deadlock detection. Detect circular wait-for cycles at runtime and resolve by preempting lowest-priority CT in the cycle.

## Document References
- **Primary:** Section 3.2.2 (Deadlock Prevention: Runtime wait-for graph detects dynamic circular waits and resolves by preempting the lowest-priority CT in the cycle)
- **Supporting:** Section 2.1 (Dependency DAG for static cycle detection context)

## Deliverables
- [ ] Rust module `wait_for_graph.rs` — runtime wait-for graph for dynamic deadlock detection
- [ ] Wait-for edge tracking — CT A waits for CT B creates edge A→B in graph
- [ ] Graph traversal for cycle detection — DFS or SCC to detect cycles at runtime
- [ ] Cycle resolution — identify lowest-priority CT in cycle, preempt it
- [ ] Preemption mechanism — checkpoint CT, move to blocked queue, trigger exception
- [ ] SIG_DEADLINE_WARN for affected CTs — notify CTs being preempted
- [ ] Test suite — 20+ test cases covering various deadlock scenarios and resolutions
- [ ] Integration test — create dynamic deadlock scenario (e.g., two crews waiting on each other), verify detection and resolution

## Technical Specifications
**Wait-For Relationship (Section 3.2.2):**
- CT_A waits for CT_B if: CT_A is blocked on resource held by CT_B, or CT_A cannot proceed until CT_B completes
- Create graph edge: CT_A → CT_B
- Check for cycles: if path exists from CT_A back to CT_A through wait-for edges, cycle detected
- False positive mitigation: only add wait-for edges when CT explicitly blocks (e.g., on lock, on dependency, on resource allocation)

**Cycle Detection Algorithm:**
- Run at each scheduling decision point or periodically (every 10 context switches)
- DFS from each CT: mark visited, if revisit CT in current path = cycle found
- SCC-based approach: Tarjan's algorithm to find all strongly connected components (cycles)
- Target: cycle detection <1ms for 1000 CTs

**Resolution (Lowest-Priority Preemption):**
- For each cycle detected, calculate priority score of all CTs in cycle
- Preempt CT with lowest priority score
- Checkpoint preempted CT, save to checkpoint_refs
- Move CT to blocked queue, issue exception/signal
- Preempted CT can request resume from checkpoint

**Example Deadlock Scenario:**
```
Agent A spawns CT_A and CT_B (both with high priority)
Agent B spawns CT_C and CT_D (both with high priority)
CT_A needs resource held by CT_C
CT_C needs resource held by CT_A
Wait-for graph: CT_A→CT_C→CT_A (cycle!)
Deadlock detected. Priorities: CT_A=0.8, CT_C=0.7
Preempt CT_C (lower priority). Checkpoint CT_C, unblock CT_A.
```

## Dependencies
- **Blocked by:** Week 09 (crew-aware scheduling for context), Week 08 (priority scoring for resolution)
- **Blocking:** Week 13-14 (multi-agent demo with potential deadlock scenarios)

## Acceptance Criteria
- [ ] Wait-for graph correctly tracks CT dependencies and resource waits
- [ ] Cycle detection identifies all deadlock scenarios in test cases
- [ ] Lowest-priority CT correctly identified in cycles
- [ ] Preemption mechanism checkpoints CT and moves it to blocked queue
- [ ] SIG_DEADLINE_WARN or custom signal sent to preempted CT
- [ ] All 20+ test cases pass
- [ ] Performance: cycle detection <1ms for 1000 CTs
- [ ] Integration test: dynamic deadlock scenario detected and resolved without system hang
- [ ] No false positives or missed deadlocks

## Design Principles Alignment
- **P8 — Fault-Tolerant by Design:** Deadlock detection + resolution ensures system remains responsive
- **P7 — Production-Grade from Phase 1:** Deadlock prevention is production requirement
