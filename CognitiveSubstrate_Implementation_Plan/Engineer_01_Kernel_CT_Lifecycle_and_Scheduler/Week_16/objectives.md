# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 16

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Complete framework adapter integration for LangChain and Semantic Kernel. Verify end-to-end execution of real-world agents from both frameworks on Cognitive Substrate.

## Document References
- **Primary:** Section 3.4.1 (Framework Adapters P0: LangChain and Semantic Kernel), Section 3.5.1 (CSCI system calls for adapter use)
- **Supporting:** Section 6.3 (Phase 2 Exit Criteria), Section 3.4.2 (Semantic File System with Knowledge Source mounting)

## Deliverables
- [ ] LangChain adapter complete (all chain types: SimpleChain, ReActChain, MapReduceChain, etc.)
- [ ] Semantic Kernel adapter complete (plugins, planners, memory)
- [ ] End-to-end test: real LangChain agent runs on Cognitive Substrate
- [ ] End-to-end test: real Semantic Kernel agent runs on Cognitive Substrate
- [ ] Tool integration — adapters correctly bind external tools via ToolBindings
- [ ] Memory integration — adapters correctly use L2/L3 semantic memory
- [ ] Error handling — adapters handle CT exceptions and signal correctly
- [ ] Integration test suite — 20+ test cases for both adapters

## Technical Specifications
**LangChain Adapter (Section 3.4.1):**
- Support Chain types:
  - SimpleChain: sequential steps
  - ReActChain: reasoning loop with tools
  - MapReduceChain: parallel steps with reduce
  - RouterChain: branching logic
- Translation:
  - Step → CT with priority based on position
  - Tool → ToolBinding via capability grant
  - Memory → L2 episodic memory reads/writes
  - Callbacks → signal handlers and exception handlers

**Semantic Kernel Adapter (Section 3.4.1):**
- Support:
  - Plugins → ToolBindings (one per plugin)
  - Planners → CT spawners (generate CT DAGs from plans)
  - Functions → CT entrypoints
  - Memory → L2/L3 memory access
  - Skills → reusable CT graph templates

**Tool Integration:**
- Tool.invoke() from adapter calls CSCI tool_invoke(binding)
- Binding includes:
  - target: ToolRef
  - capability: CapID (capability-gated)
  - input: typed parameters
  - sandbox_config: isolation policy
- Kernel invokes tool in isolated process with minimal capabilities

**Memory Integration:**
- Adapter calls mem_write(l2_memory_ref, key, value) after step completion
- Adapter calls mem_read(l2_memory_ref, key) before step execution
- L2 memory per-agent, semantically indexed
- All writes logged to trace for replay

**Error Handling:**
- Adapter registers exception handler via exc_register
- Handler can: Retry, Rollback(checkpoint), Escalate, Terminate
- Tool failures (ToolCallFailed) handled by adapter with backoff
- Budget exhaustion (BudgetExhausted) pauses agent gracefully

## Dependencies
- **Blocked by:** Week 15 (scheduler APIs exposed), Engineer 7,8 (runtime framework)
- **Blocking:** Week 20-24 (SDK work depends on working adapters), Phase 2 exit criteria

## Acceptance Criteria
- [ ] LangChain adapter supports all major chain types
- [ ] Semantic Kernel adapter supports plugins, planners, memory
- [ ] Real LangChain agent (e.g., ReAct research agent) runs end-to-end successfully
- [ ] Real Semantic Kernel agent (e.g., plugin orchestration) runs end-to-end successfully
- [ ] Tool invocations work correctly with capabilities and sandboxing
- [ ] Memory reads/writes work correctly
- [ ] Exception handling works (retry, rollback, escalation)
- [ ] All 20+ adapter test cases pass
- [ ] Performance acceptable (adapter overhead <10% vs native framework)

## Design Principles Alignment
- **P6 — Framework-Agnostic Agent Runtime:** Two major frameworks work on Cognitive Substrate
- **P1 — Agent-First:** Agents from both frameworks run as first-class kernel entities
