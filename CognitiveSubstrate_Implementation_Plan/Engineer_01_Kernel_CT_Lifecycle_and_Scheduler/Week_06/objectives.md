# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 06

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Complete Phase 0 integration testing. Spawn 100 CTs with cognitive priority, verify all phase transitions are logged, test exception handling (ContextOverflow), signal dispatch (SIG_DEADLINE_WARN), checkpointing and restore, and cycle detection.

## Document References
- **Primary:** Section 6.1 (Phase 0 Exit Criteria), Section 3.2.6 (Cognitive Exception Engine), Section 3.2.7 (Cognitive Checkpointing Engine), Section 3.2.8 (Reasoning Watchdog)
- **Supporting:** Section 2.7 (CognitiveException entity), Section 2.8 (CognitiveSignal), Section 2.9 (CognitiveCheckpoint)

## Deliverables
- [ ] Integration test suite `phase_0_integration_tests.rs` — comprehensive end-to-end tests
- [ ] Scenario 1: Spawn 100 CTs, schedule with cognitive priority, verify transitions logged
- [ ] Scenario 2: ContextOverflow exception — trigger L1 context overflow, verify exception handling, verify eviction to L2
- [ ] Scenario 3: SIG_DEADLINE_WARN — set deadline, verify signal delivered at 80% deadline elapsed
- [ ] Scenario 4: Checkpoint and restore — checkpoint a CT in reason phase, restore from checkpoint, verify state consistency
- [ ] Scenario 5: Dependency cycle rejection — attempt to spawn CT with circular dependency, verify rejected with clear error
- [ ] Exception handler registration (exc_register syscall) — custom exception handler receives context, can retry/rollback/escalate
- [ ] Phase 0 exit criteria checklist — all items verified

## Technical Specifications
**Phase 0 Exit Criteria (Section 6.1):**
- [ ] Boot bare-metal kernel in QEMU (no Linux, no POSIX)
- [ ] Spawn 100 CTs
- [ ] Schedule with cognitive priority (even if round-robin, establish priority scoring structure)
- [ ] Enforce capabilities with mandatory policies
- [ ] Handle a ContextOverflow exception
- [ ] Dispatch SIG_DEADLINE_WARN
- [ ] Checkpoint a CT and restore from checkpoint
- [ ] Detect a dependency cycle and reject the spawn

**ContextOverflow Exception (Section 2.7):**
- Severity: Recoverable
- Default kernel handler: Evict lowest-relevance context to L2, notify agent, retry
- Test: allocate large context window, trigger overflow, verify eviction, verify CT continues

**SIG_DEADLINE_WARN Signal (Section 2.8):**
- Trigger: Approaching wall-clock deadline (default 80% of deadline_ms elapsed)
- Default action: advisory; agent may adjust strategy
- Test: set deadline, await signal, log signal reception

**Checkpoint/Restore (Section 3.2.7):**
- CPU state: snapshots full address space using copy-on-write page table fork
- Triggers: phase transitions, periodic intervals (60s), pre-preemption, SIG_CHECKPOINT
- Test: checkpoint CT, verify checkpoint size <2MB for typical context, restore, verify execution resumes correctly

**Cognitive Priority Scoring (Section 3.2.2):**
- Four-dimensional: Chain Criticality (0.4), Resource Efficiency (0.25), Deadline Pressure (0.2), Capability Cost (0.15)
- Test: score 100 CTs with varying dependencies, verify priority scores calculated correctly

## Dependencies
- **Blocked by:** Weeks 01-05 (all previous Phase 0 work)
- **Blocking:** Phase 1 begins Week 07

## Acceptance Criteria
- [ ] All Phase 0 exit criteria verified
- [ ] Integration test suite passes 100% (all 5 scenarios)
- [ ] Microkernel boots, spawns, schedules 100 CTs with no crashes
- [ ] All exception types (at least ContextOverflow) handle correctly
- [ ] All signal types (at least SIG_DEADLINE_WARN) dispatch correctly
- [ ] Checkpoint/restore cycle completes with state consistency verified
- [ ] Dependency cycle detection works; circular dependency spawn rejected
- [ ] Full trace log generated and reviewed (every phase transition recorded)
- [ ] Code review sign-off from kernel stream leads

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Phase 0 exit represents a stable, functional, production-ready microkernel
- **P8 — Fault-Tolerant by Design:** Exception handling, signals, checkpointing prove fault tolerance capability
- **P5 — Observable by Default:** Full trace logging from first boot
