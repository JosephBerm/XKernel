# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 30

## Phase: Phase 3

## Weekly Objective

Create comprehensive Getting Started guide and tutorials. Enable developers to build Hello World agent in 15 minutes, understand patterns, and integrate with frameworks.

## Document References

- **Primary:** Section 3.5.2 — libcognitive v0.1; Section 3.5.5 — SDKs v0.2; Section 6.4 — Phase 3
- **Supporting:** API playground (week 29); SDK v0.2; documentation portal; framework adapters

## Deliverables

- [ ] Write Getting Started guide: installation, Hello World agent (15 min)
- [ ] Create pattern tutorials: ReAct, Chain-of-Thought, Reflection, error handling, crews
- [ ] Write tool binding tutorial: web search, code execution, custom tools
- [ ] Write memory management tutorial: allocate, read, write, structured data
- [ ] Write IPC tutorial: channel creation, multi-agent coordination
- [ ] Create framework integration tutorials: LangChain, Semantic Kernel, CrewAI
- [ ] Video walkthroughs for key tutorials

## Technical Specifications

- Getting Started covers: SDK installation, first agent, memory, tools, troubleshooting
- Pattern tutorials show: pattern composition, error handling, real-world use cases
- Tool tutorials include: built-in tools (web, code), custom tool definition, tool composition
- Memory tutorials explain: memory slots, serialization, async access patterns
- Framework tutorials show: adapter instantiation, bridging SDK to LangChain/SK/CrewAI
- Videos provide visual guidance; code playgrounds provide editable examples

## Dependencies

- **Blocked by:** Week 29
- **Blocking:** Week 31-32 (migration guides)

## Acceptance Criteria

Comprehensive tutorials and guides published; enables rapid developer onboarding

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

