# Engineer 7 — Runtime: Framework Adapters — Week 23
## Phase: Phase 2 (Multi-Framework: Custom/Raw Adapter)
## Weekly Objective
Implement Custom/Raw adapter for framework-agnostic agent code. Ensure direct CSCI mapping via SDK. Validate all 22 syscalls accessible from adapter path. Test with 10+ real-world agent scenarios from LangChain/SK/AutoGen/CrewAI benchmarks.

## Document References
- **Primary:** Section 3.4.1 — Framework Adapters (custom/raw mapping)
- **Supporting:** Section 3.4 — L2 Agent Runtime, Section 1.2 — P6: Framework-Agnostic Agent Runtime

## Deliverables
- [ ] Custom/Raw adapter implementation (80%): direct CSCI SDK usage, minimal translation
- [ ] SDK interface documentation: how to write raw CSCI agents using Python SDK
- [ ] Syscall validation: verify all 22 syscalls (mem_*, task_*, tool_*, channel_*, cap_*) callable from adapter path
- [ ] Direct task spawning: raw CT creation without framework translation
- [ ] Direct memory operations: episodic and semantic memory writes without mapping layer
- [ ] Direct tool binding: raw ToolBinding creation and invocation
- [ ] Direct channel operations: SemanticChannel creation for multi-agent patterns
- [ ] 10+ real-world agent scenarios: benchmark agents from LangChain, SK, AutoGen, CrewAI collections
- [ ] Performance comparison: raw adapter vs framework adapters (translation overhead measurement)
- [ ] Scenario validation (20+ tests): various agent patterns, syscall coverage, performance

## Technical Specifications
- Raw adapter: minimal translation, direct SDK calls to kernel
- SDK interface: simple agent class with execute(input) → output
- Task spawning: task_spawn syscall directly with CT graph, no translation layer
- Memory operations: mem_write/mem_read without mapping to framework memory model
- Tool binding: tool_bind syscall directly with function signatures
- Channel operations: channel_create, channel_send, channel_receive for IPC
- Syscall verification: checklist of all 22 syscalls, test code path for each
- Benchmark scenarios: select 10+ public agents from framework benchmarks, convert to raw adapter
- Performance metrics: translation latency (raw ~0ms), syscall count, memory overhead
- Test categories: simple tasks, complex workflows, multi-agent patterns, tool usage, error handling

## Dependencies
- **Blocked by:** Week 22
- **Blocking:** Week 24

## Acceptance Criteria
- Custom/Raw adapter 80% complete with direct SDK mapping functional
- All 22 syscalls verified accessible from adapter execution path
- 10+ real-world agent scenarios implemented as raw adapters
- 20+ validation tests passing across various patterns
- Performance data collected showing translation overhead (or lack thereof)
- Raw adapter documentation available
- SDK interface suitable for end-user agent development

## Design Principles Alignment
- **Zero Translation:** Custom adapter eliminates translation overhead
- **Kernel Native:** Direct syscall usage for maximum efficiency
- **Extensible:** Provides foundation for users to write custom agents
