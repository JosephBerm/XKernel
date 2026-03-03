# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 15

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Support Runtime stream (Engineers 7, 8) by exposing scheduler APIs for framework adapters. Ensure CT spawn from adapter path works correctly and integrates with LangChain/Semantic Kernel.

## Document References
- **Primary:** Section 3.4.1 (Framework Adapters: LangChain, Semantic Kernel, AutoGen, CrewAI translate to CT graph), Section 3.5.1 (CSCI syscalls available to adapters)
- **Supporting:** Section 3.2.2 (Cognitive Priority Scheduler), Section 2.1 (CognitivePriority struct)

## Deliverables
- [ ] Scheduler API documentation — for framework adapters to call ct_spawn, ct_yield, ct_checkpoint
- [ ] LangChain adapter integration — chain steps map to CT graph, verify spawn works
- [ ] Semantic Kernel adapter integration — planners spawn CTs correctly
- [ ] CT spawn from adapter context — ensure memory, capabilities, priorities set correctly
- [ ] Test suite — spawn CTs via adapter pathways for LangChain and Semantic Kernel
- [ ] Performance baseline — measure overhead of adapter layer vs direct CSCI calls

## Technical Specifications
**Framework Adapter Integration (Section 3.4.1):**

1. **LangChain Adapter:**
   - Chain steps → CT graph (one CT per step)
   - Memory → L2 episodic memory (shared across steps)
   - Tools → ToolBindings (via capability grant from parent Agent)
   - Example: ReAct chain with 5 steps becomes 5 CTs with dependencies

2. **Semantic Kernel Adapter:**
   - Plugins → ToolBindings
   - Planners → CT spawners (planner generates CT dependency DAG)
   - Memory → L2/L3 semantic memory
   - Example: Orchestration plan with 3 plugins becomes 3 CTs

**CT Spawn from Adapter:**
- Adapter calls ct_spawn(parent_agent_ref, ct_spec) where ct_spec includes:
  - phase: initially spawn
  - priority: CognitivePriority (adapter sets based on framework semantics)
  - capabilities: subset of parent_agent.capabilities
  - context_window_size: from adapter config
  - dependencies: list of prerequisite CTs (DAG edges)
  - resource_budget: from adapter config or parent quota
  - watchdog_config: from adapter config (deadline, max iterations)
  - signal_handlers: from adapter or kernel defaults

**Example LangChain Integration:**
```rust
// LangChain ReAct chain
let chain = ReActChain::new(llm, tools);
// Adapter creates CT graph:
let ct_research = ct_spawn(agent, CTSpec {
  phase: spawn,
  priority: CognitivePriority { chain_criticality: 0.9, ... },
  dependencies: vec![],
  ...
});
let ct_analyze = ct_spawn(agent, CTSpec {
  dependencies: vec![ct_research],
  ...
});
```

**Adapter API (CSCI extension for adapters):**
- ct_spawn(parent_agent: AgentRef, spec: CTSpec) → Result<CTRef>
- ct_yield() → () (yield to scheduler)
- mem_read(ref: MemoryRef, key: String) → Result<Value>
- mem_write(ref: MemoryRef, key: String, value: Value) → Result<()>
- chan_send(channel: ChannelRef, message: Message) → Result<()>
- All existing CSCI syscalls (22 from Section 3.5.1)

## Dependencies
- **Blocked by:** Phase 1 complete (Week 14)
- **Blocking:** Week 16 (continue adapter work), Week 18-22 (SDK integration)

## Acceptance Criteria
- [ ] Scheduler APIs documented for adapter use
- [ ] LangChain adapter successfully spawns CTs from chain steps
- [ ] Semantic Kernel adapter successfully spawns CTs from planner output
- [ ] CT spawn from adapter context correctly sets memory, capabilities, priority
- [ ] All 15+ adapter integration test cases pass
- [ ] Adapter overhead measured (target: <5% vs native CT spawn)
- [ ] Coordination with Engineers 7, 8 to align on adapter interfaces

## Design Principles Alignment
- **P6 — Framework-Agnostic Agent Runtime:** Adapters enable multiple frameworks to use Cognitive Substrate
- **P2 — Cognitive Primitives as Kernel Abstractions:** Framework concepts map cleanly to CT primitives
