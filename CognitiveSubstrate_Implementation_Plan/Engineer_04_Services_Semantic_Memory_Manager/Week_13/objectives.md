# Engineer 4 — Services: Semantic Memory Manager — Week 13

## Phase: 1 — Three-Tier Implementation
## Weekly Objective
Implement Out-of-Context (OOC) Handler for extreme memory pressure scenarios. Establish emergency escalation protocol: spill L1→L2, compress L2, checkpoint CTs as last resort. Ensure no reasoning state is lost during OOC recovery.

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 7-10 (Three-tier with prefetch, CRDT, OOC handler)
- **Supporting:** Section 2.5 — SemanticMemory (eviction policies)

## Deliverables
- [ ] OOC trigger detection (when memory pressure exceeds critical threshold)
- [ ] Emergency eviction pipeline (L1→L2 at accelerated rate)
- [ ] L2 compression invocation (invoke compactor in accelerated mode)
- [ ] CT checkpointing mechanism (snapshot current state to L3)
- [ ] CT suspension infrastructure (pause execution, release resources)
- [ ] OOC recovery path (resume from checkpoint)
- [ ] Unit tests for OOC scenarios (allocation beyond 3x L1 capacity)
- [ ] Integration test: trigger OOC, checkpoint, resume, verify correctness

## Technical Specifications
- Define critical memory threshold (e.g., 95% total memory utilization)
- Implement three-tier OOC response:
  1. Spill L1→L2 at maximum rate (100% of available I/O)
  2. Invoke compactor with max budget (emergency mode)
  3. Checkpoint and suspend CTs (write current execution state to L3)
- Checkpoint format: CT code pointer, register state, stack, L1 snapshot
- Suspension: unmap CT address space, freeze execution
- Recovery: remap CT from checkpoint, resume execution
- Implement abort semantics: if cannot fit even with OOC, report error
- Track OOC events and recovery time for diagnostics

## Dependencies
- **Blocked by:** Week 12 (L3 enables checkpointing)
- **Blocking:** Week 14 (CRDT for shared memory)

## Acceptance Criteria
- [ ] OOC detection latency <100ms
- [ ] Spill rate during OOC >100MB/s
- [ ] Checkpointing time proportional to L1 snapshot size
- [ ] CT successfully resumes after OOC recovery
- [ ] No reasoning state lost during OOC/recovery cycle
- [ ] Integration test: force 10x memory allocation, recover gracefully

## Design Principles Alignment
- **Robustness:** Emergency escalation prevents hard failures
- **Safety:** Checkpointing preserves reasoning state
- **Fairness:** OOC cost distributed across competing CTs
- **Transparency:** OOC handling invisible to CT code
