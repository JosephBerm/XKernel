# Engineer 7 — Runtime: Framework Adapters — Week 11
## Phase: Phase 1 (Implementation: LangChain Adapter)
## Weekly Objective
Begin LangChain adapter implementation. Implement chain step translation to CT graph with full dependency DAG support. Implement LangChain Memory translation to L2 Episodic Memory. Support all chain types (Sequential, Router, Map-Reduce).

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 12-14 (Begin LangChain adapter, Agent Lifecycle)
- **Supporting:** Section 3.4.1 — Framework Adapters, Section 3.4 — L2 Agent Runtime

## Deliverables
- [ ] LangChain adapter implementation (50%): core chain translation engine
- [ ] Chain step parser: decompose LangChain chains into steps
- [ ] CT graph builder: convert chain steps to CT dependency DAG
- [ ] Sequential chain translator: translate sequential execution to linear CT DAG
- [ ] Router chain translator: translate routing logic to CT conditional spawning (if supported)
- [ ] Map-Reduce chain translator: translate parallel processing to CT parallel batch spawn
- [ ] LangChain memory mapper: translate ConversationBufferMemory, SummaryMemory, ConversationKGMemory to L2 episodic writes
- [ ] Tool binding integration: LangChain tools → ToolBindings with argument schema
- [ ] Unit tests (20+): chain parsing, DAG construction, memory mapping, tool binding
- [ ] Integration test: simple 3-step LangChain chain on kernel

## Technical Specifications
- Chain parser: extract steps from chain._steps, build step dependency map
- CT graph builder: step → CT with input/output mapping, dependencies from step.input_keys/output_keys
- DAG construction: DAG node = CT, DAG edge = data dependency, serialize as CT spawn batch
- Sequential chain: step[i] deps = [step[i-1]]
- Router chain: condition step deps = [], routing step deps = [condition], option steps deps = [routing]
- Map-Reduce chain: mapper step deps = [base], reduce step deps = [all mapper CTs]
- Memory mapper: on step result, write to L2 via mem_write(agent_id+step_id, result_data, TTL=session)
- Tool binding: create_tool_binding(langchain_tool.name, langchain_tool.func_signature, required_caps=[])
- Error handling: invalid chain structure → TranslationError, memory write failures → log and continue
- Integration test: load ReAct agent, translate chain, spawn on kernel, observe CT execution traces

## Dependencies
- **Blocked by:** Week 10
- **Blocking:** Week 12, Week 13, Week 14, Week 15, Week 16

## Acceptance Criteria
- LangChain adapter 50% implementation with core translators functional
- All chain types (Sequential, Router, Map-Reduce) translatable to CT DAGs
- Memory mapping from LangChain to L2 episodic working
- Tool binding for LangChain tools functional
- 20+ unit tests passing, integration test successful

## Design Principles Alignment
- **Framework Native:** Leverage LangChain abstractions (Chain, Memory, Tool) directly
- **Kernel Native:** Direct translation to CT DAG, episodic memory, tool bindings
- **Dependency Preserving:** Maintain chain dependencies as CT dependencies
