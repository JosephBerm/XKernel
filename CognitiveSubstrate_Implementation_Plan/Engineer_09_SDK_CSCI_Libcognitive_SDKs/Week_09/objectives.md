# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 09

## Phase: Phase 1

## Weekly Objective

Begin libcognitive standard library implementation with ReAct (Reasoning + Acting) pattern as composable CT graph template. Implement using ct_spawn and dependency chains, enabling agents to decompose tasks into thought-action cycles.

## Document References

- **Primary:** Section 3.5.2 — libcognitive: Standard Library; Section 6.2 — Phase 1
- **Supporting:** CSCI v0.1 (ct_spawn, ct_yield); cognitive task execution semantics; graph composition patterns

## Deliverables

- [ ] Design ReAct pattern as composable CT graph: Thought → Action → Observation → repeat
- [ ] Implement ct.ReAct({agentPrompt, tools, maxSteps}) pattern using ct_spawn
- [ ] Implement thought cycle: agent_ct_spawn({prompt: thought_prompt, deps: {agentState}})
- [ ] Implement action dispatch: tool_invoke(toolName, args) within thought cycle
- [ ] Implement observation capture: mem_write(observationSlot, toolResult)
- [ ] Create typed agent state passing and memory slots for reasoning artifacts
- [ ] Write unit tests and integration tests with mock tools

## Technical Specifications

- ReAct pattern decomposes task into thoughts and actions; cycles continue until agent decides done
- Agent state includes: currentTask, conversationHistory, toolResults, reflection
- Each thought spawns as CT with access to current state via mem_read
- Actions trigger tool_invoke syscalls; results written back to memory
- Handles max step limits, cyclic dependencies, and tool errors gracefully

## Dependencies

- **Blocked by:** Weeks 7-8
- **Blocking:** Week 10 (ReAct refinement); Week 11-12 (Chain-of-Thought, Reflection)

## Acceptance Criteria

ReAct pattern fully implemented and tested; can be used as library function

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

