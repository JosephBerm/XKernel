# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 13

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Prepare Phase 1 demo with AgentCrew of 3 agents. Verify scheduling affinity, priority scoring, and deadlock resolution work together in realistic multi-agent scenario.

## Document References
- **Primary:** Section 6.2 (Phase 1 Exit Criteria: AgentCrew of 3 agents collaborating with full fault tolerance demonstrated)
- **Supporting:** Section 3.2.2 (all scheduling features), Section 2.3 (AgentCrew entity)

## Deliverables
- [ ] Demo scenario specification — 3-agent crew with realistic workload
- [ ] Test workload: Agent A researches (web search), shares via SemanticChannel with Agent B (analysis), which writes summary via Agent C
- [ ] Scheduling validation — verify crew CTs pinned to same NUMA node
- [ ] Priority verification — verify priority scores correctly influence scheduling order
- [ ] Crew coordination test — verify SemanticChannel communication works between crew CTs
- [ ] Demo preparation — practice run-through, timing, success criteria
- [ ] Demo presentation — show scheduler behavior, trace logs, performance metrics

## Technical Specifications
**3-Agent Crew Scenario (Section 6.2):**

1. **Agent A (Researcher):**
   - Task: research topic X using web search tool
   - Spawns 3 CTs: search_query, analyze_results, summarize_findings
   - Dependencies: search_query → analyze_results → summarize_findings
   - Expected: runs reason→act→reflect→yield cycle 3 times

2. **Agent B (Analyst):**
   - Task: analyze findings from Agent A
   - Spawns 2 CTs: receive_summary, process_analysis
   - Depends on: Agent A's summarize_findings CT
   - Expected: waits for Agent A, then processes asynchronously

3. **Agent C (Writer):**
   - Task: write final report from Agent B's analysis
   - Spawns 2 CTs: compile_report, format_output
   - Depends on: Agent B's process_analysis CT
   - Expected: final output after all dependencies

**Scheduling Assertions:**
- All 7 CTs (3+2+2) allocated to same NUMA node (crew affinity)
- Priority scores: Agent A's early CTs high (unblock others), Agent B/C lower until dependencies ready
- Deadline escalation: if crew has deadline, all CTs escalate proportionally
- Deadlock prevention: no cycles (dependencies form DAG)
- GPU resources: if reason phase uses inference, verify GPU TPCs allocated

**SemanticChannel Communication:**
- Agent A → Agent B: send research summary via typed SemanticChannel
- Agent B → Agent C: send analysis via typed SemanticChannel
- Verify: zero-copy IPC for co-located agents, capability-gated access

## Dependencies
- **Blocked by:** Week 12 (GPU Manager integration complete), all prior Phase 1 weeks
- **Blocking:** Phase 1 exit criteria verification, Week 14 demo review

## Acceptance Criteria
- [ ] 3-agent crew scenario runs start-to-finish without errors
- [ ] All 7 CTs spawn and schedule successfully
- [ ] NUMA affinity verified (all CTs on same node)
- [ ] Priority scores correctly calculated and influence scheduling
- [ ] SemanticChannel communication works between agents
- [ ] No deadlocks detected (or if detected, correctly resolved)
- [ ] GPU resources allocated and released correctly
- [ ] Trace logs show complete execution flow with phase transitions
- [ ] Demo passes rehearsal without issues
- [ ] Timing: full scenario completes in <5 minutes

## Design Principles Alignment
- **P1 — Agent-First:** Demo shows agents as first-class kernel entities
- **P6 — Framework-Agnostic Agent Runtime:** Agents run independently, not in framework processes
