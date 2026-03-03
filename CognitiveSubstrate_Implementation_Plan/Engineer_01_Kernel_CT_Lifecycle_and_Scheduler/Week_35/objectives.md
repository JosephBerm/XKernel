# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 35

## Phase: PHASE 3 — Production Hardening + Launch (Weeks 25-36)

## Weekly Objective
Final security audit and launch preparation. Conduct OS completeness re-audit. Ensure all scheduler components meet production standards. Prepare for open-source release.

## Document References
- **Primary:** Section 6.4 (Weeks 34-36: OS completeness re-audit for scheduler subsystem, launch preparation)
- **Supporting:** Section 10 (Success Criteria), Section 9 (Open Source and Go-to-Market)

## Deliverables
- [ ] Final security audit — external or internal review of all critical paths
- [ ] OS completeness re-audit — verify 100% coverage of scheduler subsystem
- [ ] Scheduler subsystem audit checklist — all components covered
- [ ] Code quality review — production-grade standards
- [ ] Documentation audit — all code documented, all decisions recorded
- [ ] Open-source preparation — license, CONTRIBUTING.md, build instructions
- [ ] Release notes — what's included, what's not, known limitations
- [ ] Launch communication — blog post, announcement, developer relations prep

## Technical Specifications
**Final Security Audit Scope:**

1. **Capability Enforcement:**
   - [ ] Page table mappings verified for all capability operations
   - [ ] Capability subset invariant enforced
   - [ ] Revocation properly unmaps pages from all holders
   - [ ] No kernel memory accessible without capability

2. **Scheduler Priority:**
   - [ ] Priority calculation correct for all 4 dimensions
   - [ ] No priority inversion lasting >1 second
   - [ ] Scheduler starvation impossible (aging or reserve scheduling)
   - [ ] Deadline enforcement reliable

3. **Dependency DAG & Deadlock:**
   - [ ] All cycles detected at spawn time
   - [ ] Runtime wait-for graph correctly identifies all cycles
   - [ ] Preemption mechanism works correctly
   - [ ] No false deadlock detections

4. **Signal & Exception Handling:**
   - [ ] All signals delivered safely (no race conditions)
   - [ ] All exceptions handled with correct context
   - [ ] No signal spoofing possible (user CTs can't send signals)
   - [ ] Exception handlers have access to correct state

5. **Checkpointing & Recovery:**
   - [ ] Checkpoints are consistent and verifiable
   - [ ] Checkpoint signatures prevent tampering
   - [ ] Recovery from checkpoint exact (bit-for-bit)
   - [ ] GPU state checkpoint concurrent and correct

**OS Completeness Re-Audit Checklist:**

Scheduler Subsystem:
- [ ] CT lifecycle state machine: spawn→plan→reason→act→reflect→yield→complete
- [ ] CT spawn: all fields initialized, all invariants checked
- [ ] CT scheduling: priority calculation, context switching, fairness
- [ ] CT yield: proper state transitions, trace logging
- [ ] CT checkpoint: copy-on-write, GPU state, consistency
- [ ] CT resume: exact recovery from checkpoint
- [ ] Phase transitions: all logged, all traced
- [ ] Dependency DAG: cycle checking, wait-for tracking
- [ ] Priority calculation: 4 dimensions, weighted sum, normalization
- [ ] CPU scheduling: priority heap, O(log n) operations
- [ ] GPU scheduling: TPC allocation, latency modeling, right-sizing
- [ ] Crew scheduling: NUMA affinity, member co-scheduling
- [ ] Deadlock prevention: static + runtime detection
- [ ] Deadlock resolution: preemption, checkpoint, recovery
- [ ] IPC: zero-copy for co-located agents, capability-gated
- [ ] Signals: all 8 types, safe delivery, handlers
- [ ] Exceptions: all 7 types, default + custom handlers, escalation
- [ ] Watchdogs: deadline enforcement, iteration limits, loop detection

**Code Quality Review:**
- [ ] All code reviewed and approved
- [ ] No compiler warnings (Rust/LLVM)
- [ ] No security warnings (cargo audit, clippy)
- [ ] No memory leaks (verified with valgrind/ASAN)
- [ ] No race conditions (verified with ThreadSanitizer)
- [ ] Test coverage >80% (measured with cargo tarpaulin)

**Documentation Audit:**
- [ ] Every function documented (doc comments)
- [ ] Every algorithm documented (pseudocode or reference)
- [ ] Every design decision documented (ADR or comments)
- [ ] All invariants documented
- [ ] All assumptions documented
- [ ] Integration points with other systems documented

## Dependencies
- **Blocked by:** Week 34 (OS completeness audit, paper submission)
- **Blocking:** Week 36 (final launch)

## Acceptance Criteria
- [ ] Final security audit completed (no critical findings)
- [ ] OS completeness audit 100% coverage
- [ ] Scheduler subsystem verified production-ready
- [ ] Code quality at production standards
- [ ] All documentation complete and accurate
- [ ] Open-source release preparation complete
- [ ] Ready for Week 36 launch

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Final audit validates production readiness
