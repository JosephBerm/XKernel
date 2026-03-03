# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 31

## Phase: Phase 3

## Weekly Objective

Create migration guides for developers migrating from LangChain, Semantic Kernel, and CrewAI. Explain mapping between old and new patterns, and showcase advantages of CSCI-native approach.

## Document References

- **Primary:** Section 3.5.1 — CSCI; Section 3.5.2 — libcognitive; Section 3.5.5 — SDKs; Section 6.4 — Phase 3
- **Supporting:** Framework adapters; SDK v0.2; getting started guides (week 30)

## Deliverables

- [ ] Write LangChain migration guide: Agent → SDK ReAct, Memory → mem operations, Tools → tool_bind
- [ ] Write Semantic Kernel migration guide: Plans → libcognitive patterns, Functions → Tool SDK, Memory → mem operations
- [ ] Write CrewAI migration guide: Crews → crew_create/crew_join, Agents → ct_spawn, Tasks → task definitions
- [ ] Create side-by-side code comparisons (old framework vs. CSCI SDK)
- [ ] Explain advantages: lower latency (CSCI native), better resource control, unified semantics
- [ ] Provide adaptation examples: common patterns in old frameworks mapped to new SDK

## Technical Specifications

- LangChain guide: Agent loop → ReAct pattern, Memory manager → mem_read/mem_write, Tool use → tool_bind/tool_invoke
- Semantic Kernel guide: Plan execution → ct_spawn, Function invocation → tool_invoke, Memory plug-ins → mem layers
- CrewAI guide: Crew definition → crew_create, Agent tasks → ct_spawn with task specs, Execution → crew_join
- Code examples: before/after for common scenarios (multi-step reasoning, tool use, crew coordination)
- Advantages highlighted: 50% reduction in latency vs. LangChain, unified concurrency vs. SK, native crew support vs. CrewAI

## Dependencies

- **Blocked by:** Week 30
- **Blocking:** Week 32 (adoption promotion)

## Acceptance Criteria

Migration guides published; enables smooth adoption by existing framework users

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

