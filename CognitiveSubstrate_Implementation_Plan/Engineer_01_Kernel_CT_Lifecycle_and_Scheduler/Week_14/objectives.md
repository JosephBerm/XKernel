# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 14

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Complete Phase 1 exit criteria verification. Run multi-agent demo. Test fault tolerance scenarios (failures and recovery) in realistic context.

## Document References
- **Primary:** Section 6.2 (Phase 1 Exit Criteria: AgentCrew of 3 agents with capability-gated, policy-checked, fully traced operation; simulate failures)
- **Supporting:** Section 2.7-2.8 (CognitiveException, CognitiveSignal), Section 3.2.6-3.2.7 (Exception Engine, Checkpointing)

## Deliverables
- [ ] Phase 1 exit criteria checklist — all items verified
- [ ] Live demo execution — 3-agent crew scenario with audience
- [ ] Failure scenario 1: tool call triggers retry — verify exponential backoff, max retries
- [ ] Failure scenario 2: context overflow triggers eviction — verify lowest-relevance eviction to L2
- [ ] Failure scenario 3: budget exhaustion triggers checkpoint+suspend — verify suspension and resume
- [ ] Failure scenario 4: deadlock detection and resolution — create deadlock, verify detection and preemption
- [ ] Trace log review — full execution trace with all events, exceptions, signals
- [ ] Phase 1 retrospective — what went well, what needs improvement for Phase 2

## Technical Specifications
**Phase 1 Exit Criteria (Section 6.2):**
- [ ] Demo: AgentCrew of 3
- [ ] Agent A researches (web search), shares via SemanticChannel with Agent B (analysis), which writes summary via Agent C
- [ ] All capability-gated
- [ ] All policy-checked
- [ ] Fully traced
- [ ] Simulate failures:
  - [ ] Tool call triggers retry
  - [ ] Context overflow triggers eviction
  - [ ] Budget exhaustion triggers checkpoint+suspend
  - [ ] Deadlock detected and resolved

**Failure Scenarios:**

1. **Tool Call Retry (Section 2.7: ToolCallFailed):**
   - Setup: Agent A's web search tool fails (mock network timeout)
   - Expected: Exponential backoff retry (1ms → 2ms → 4ms → 8ms)
   - Max retries: 3 (from watchdog_config.tool_retry_limit)
   - After 3 failures: escalate to exception handler
   - Handler: escalate to parent Agent with ToolCallFailed exception
   - Verify: retry attempts logged, max retries respected, escalation happens

2. **Context Overflow Eviction (Section 2.7: ContextOverflow):**
   - Setup: Agent C's context window fills beyond capacity
   - Expected: Kernel handler evicts lowest-relevance context to L2
   - Verify: evicted context accessible in L2, CT continues reasoning without interruption

3. **Budget Exhaustion (Section 2.7: BudgetExhausted):**
   - Setup: Agent B's resource_budget reaches 100% (tokens, GPU-ms, wall-clock, or memory)
   - Expected: Kernel checkpoints CT, suspends (moves to blocked queue)
   - Notify: parent Agent receives BudgetExhausted exception
   - Verify: checkpoint created, CT suspended, can be resumed later

4. **Deadlock Detection and Resolution (Section 3.2.2):**
   - Setup: Create scenario where Agent A and Agent B wait on each other
   - Expected: Wait-for graph detects cycle
   - Resolution: preempt lowest-priority CT
   - Verify: deadlock detected within 100ms, preempted CT checkpointed, system continues

**Trace Log Review:**
- Extract full trace from kernel ring buffer
- Verify events in order: phase transitions, exceptions, signals, checkpoints, capability checks, IPC messages
- Verify timestamps are monotonic
- Verify cost attribution (tokens, GPU-ms, wall-clock tracked per CT)

**Demo Presentation:**
- Show microkernel booting
- Spawn 3-agent crew
- Run research→analysis→writing pipeline
- Inject failures and show recovery
- Display scheduler trace with priority scores
- Show SemanticChannel communication

## Dependencies
- **Blocked by:** All Week 01-13 work
- **Blocking:** Phase 2 begins Week 15

## Acceptance Criteria
- [ ] All Phase 1 exit criteria verified (not just in test, in live demo)
- [ ] Live demo completes successfully (3-agent crew runs, agents communicate, output correct)
- [ ] Tool call retry scenario works (3 retries, then exception)
- [ ] Context overflow scenario works (eviction to L2, CT continues)
- [ ] Budget exhaustion scenario works (checkpoint, suspend, escalate)
- [ ] Deadlock scenario works (detection, preemption, resolution)
- [ ] Trace log complete and correct (all events recorded in order)
- [ ] Cost attribution accurate (tokens/GPU-ms/wall-clock attributed correctly)
- [ ] No crashes, no memory leaks, no data corruption
- [ ] Demo video/logs for posterity

## Design Principles Alignment
- **P8 — Fault-Tolerant by Design:** All failure scenarios handled gracefully with recovery
- **P5 — Observable by Default:** Complete trace visibility into all operations
- **P7 — Production-Grade from Phase 1:** Phase 1 exit represents production-ready foundation
