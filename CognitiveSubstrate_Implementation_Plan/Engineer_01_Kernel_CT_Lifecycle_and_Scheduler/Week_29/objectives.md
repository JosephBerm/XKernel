# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 29

## Phase: PHASE 3 — Production Hardening + Launch (Weeks 25-36)

## Weekly Objective
Begin fuzz testing of scheduler edge cases. Test CT spawn with adversarial dependency graphs, priority inversion scenarios, and resource exhaustion conditions.

## Document References
- **Primary:** Section 6.4 (Weeks 28-32: Fuzz testing — scheduler edge cases, CT spawn with adversarial dependency graphs, priority inversion scenarios)
- **Supporting:** Section 3.2.2 (Scheduler algorithms and edge cases)

## Deliverables
- [ ] Fuzz testing framework — automated test generator for adversarial inputs
- [ ] Dependency graph fuzzing — generate random DAGs (and invalid cycles), verify handling
- [ ] Priority inversion testing — scenarios where priority ordering is violated
- [ ] Resource exhaustion fuzzing — exhaust memory, CPU, GPU, capabilities
- [ ] Signal/exception fuzzing — send signals/exceptions to CTs in all phase combinations
- [ ] Concurrency fuzzing — race conditions, concurrent CT spawns, concurrent capability changes
- [ ] Crash and recovery — kill CTs, verify no system crashes
- [ ] Documentation — all fuzz tests, findings, fixes

## Technical Specifications
**Dependency Graph Fuzzing:**
- Random DAG generation: 10-100 CTs, 5-20% edge density
- Invalid cycle generation: intentionally create cycles, verify rejection
- Edge cases: self-loops, disconnected graphs, linear chains, wide DAGs, deep DAGs
- Expected: ct_spawn rejects all cycles, accepts all valid DAGs

**Priority Inversion Testing:**
- Priority inversion: low-priority CT blocks high-priority CT
- Test scenario: CT A (high priority, depends on CT C), CT B (low priority), CT C (medium priority) blocks B
- Expected: scheduler should identify dependency and elevate CT C's priority temporarily
- Verify: CT C completes before CT A (or CT A waits, doesn't spin)

**Resource Exhaustion Fuzzing:**
- Memory exhaustion: spawn CTs until allocation fails, verify graceful handling
- CPU exhaustion: spawn CTs until runqueue overloaded, verify fair scheduling
- GPU exhaustion: request more TPCs than available, verify queuing/backoff
- Capability exhaustion: grant capabilities until grant fails, verify error handling

**Signal/Exception Fuzzing:**
- Send SIG_TERMINATE to CT in all phases (spawn, plan, reason, act, reflect, yield)
- Send SIG_DEADLINE_WARN at varying times (10%, 50%, 80%, 90%, 95% of deadline)
- Send exceptions (ContextOverflow, ToolCallFailed, BudgetExhausted) at all phases
- Expected: all handled gracefully, no crashes

**Concurrency Fuzzing:**
- Concurrent ct_spawn: 100 threads spawning CTs simultaneously
- Concurrent dependency changes: create/modify/delete dependencies while scheduling
- Concurrent signal delivery: multiple signals to same CT
- Expected: no race conditions, deterministic behavior

## Dependencies
- **Blocked by:** Week 28 (benchmarking complete, baseline established)
- **Blocking:** Week 30-32 (adversarial testing, security hardening)

## Acceptance Criteria
- [ ] Fuzz testing framework implemented and validated
- [ ] Dependency graph fuzzing: 100+ random DAGs tested
- [ ] Invalid cycles: 50+ cycles correctly rejected
- [ ] Priority inversion: 20+ scenarios tested
- [ ] Resource exhaustion: all resources (memory, CPU, GPU, caps) fuzzed
- [ ] Signal/exception fuzzing: all combinations tested
- [ ] Concurrency fuzzing: 10+ concurrent operation scenarios
- [ ] Crash and recovery: no crashes under any fuzz test
- [ ] All findings documented

## Design Principles Alignment
- **P8 — Fault-Tolerant by Design:** Fuzz testing ensures robustness
- **P7 — Production-Grade from Phase 1:** Production-grade means fuzzing validation
