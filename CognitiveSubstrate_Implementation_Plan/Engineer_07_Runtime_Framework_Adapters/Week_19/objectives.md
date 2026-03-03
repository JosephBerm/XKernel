# Engineer 7 — Runtime: Framework Adapters — Week 19
## Phase: Phase 2 (Multi-Framework: CrewAI Adapter)
## Weekly Objective
Implement CrewAI adapter completely. Crew structure translates to AgentCrew. Tasks translate to CTs with dependencies. Roles translate to capability sets. Validate CrewAI agents on kernel with multi-agent orchestration.

## Document References
- **Primary:** Section 6.3 — Phase 2, Week 15-18 (Complete LangChain + SK adapters, CrewAI adapter)
- **Supporting:** Section 3.4.1 — Framework Adapters

## Deliverables
- [ ] CrewAI adapter implementation (80%): crew, task, role translation complete
- [ ] Crew-to-AgentCrew translation: fully functional with multi-agent coordination
- [ ] Task-to-CT translation: task dependencies correctly mapped to CT dependencies
- [ ] Role-to-Capability mapping: role definitions converted to agent capability sets
- [ ] Multi-agent communication: CrewAI messaging through SemanticChannels
- [ ] Task execution orchestration: manage task dependencies, handle failures, collect results
- [ ] CrewAI memory integration: shared crew memory → L2/L3 kernel memory model
- [ ] Delegation support: agent-to-agent task delegation through CT spawning
- [ ] Validation tests (15+): single agent tasks, multi-agent coordination, delegation, error handling
- [ ] CrewAI MVP scenario: 3-agent crew with delegated tasks on Cognitive Substrate

## Technical Specifications
- Crew translation: each agent → CT context (agent_id, capabilities), crew_id tracks crew membership
- Task translation: Task.agent_required=true → must run on specific agent CT, dependencies → CT deps
- Role translation: Role permissions and skills → capability grants to agent
- SemanticChannel usage: crew messaging (agent-to-agent) through SemanticChannel with agent IDs
- Task execution: spawn CT per task in dependency order, collect results into crew memory
- Memory integration: crew.memory → shared L2 episodic memory across agents
- Delegation: task.delegate() → spawn CT on different agent (based on role expertise)
- Error handling: missing role capabilities, circular task dependencies, agent failures
- MVP scenario: 3 agents (researcher, analyst, writer), delegated tasks, shared memory

## Dependencies
- **Blocked by:** Week 18
- **Blocking:** Week 20, Week 21, Week 22, Week 23

## Acceptance Criteria
- CrewAI adapter 80% complete with all core features functional
- Crew → AgentCrew translation working correctly
- Task dependencies properly mapped to CT dependencies
- Role capabilities correctly granted
- 15+ validation tests passing
- Multi-agent orchestration working through SemanticChannels
- CrewAI MVP scenario successfully executing on kernel
- Delegation mechanism functional

## Design Principles Alignment
- **Natural Mapping:** Crew → AgentCrew, Task → CT, Role → Capabilities are 1:1
- **Multi-Agent:** SemanticChannels enable agent-to-agent communication
- **Orchestration:** Task dependencies automatically translate to CT orchestration
