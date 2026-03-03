# Engineer 7 — Runtime: Framework Adapters — Week 29
## Phase: Phase 3 (Hardening: Telemetry & Events)
## Weekly Objective
Enhance CEF event translation for all frameworks. Ensure LangChain, Semantic Kernel, CrewAI, and AutoGen native telemetry correctly translates to CEF at adapter boundary. Validate event quality and completeness.

## Document References
- **Primary:** Section 6.4 — Phase 3, Week 30-34 (Migration tooling)
- **Supporting:** Section 3.4 — L2 Agent Runtime, Section 3.2 — IPC & Memory Interfaces

## Deliverables
- [ ] CEF event specification: detailed CEF format for all adapter event types
- [ ] LangChain telemetry mapping: LangChain callbacks → CEF events
- [ ] Semantic Kernel telemetry mapping: SK event sources → CEF events
- [ ] CrewAI telemetry mapping: crew execution events → CEF events
- [ ] AutoGen telemetry mapping: conversation events → CEF events
- [ ] Custom adapter telemetry: raw CSCI events already CEF-compatible
- [ ] Event field mapping: framework-specific fields → CEF standard fields
- [ ] Event quality validation: ensure all events have required fields, correct severity/type
- [ ] Event completeness testing: verify no events are lost during translation
- [ ] Telemetry documentation: CEF event reference for all adapters
- [ ] Example traces: demonstrate full execution traces for sample agents

## Technical Specifications
- CEF event schema: event_type, adapter, framework, agent_id, session_id, timestamp, severity, data
- LangChain mapping: OnChainStart, OnChainEnd, OnToolStart, OnToolEnd, OnLLMStart, OnLLMEnd → CEF
- SK mapping: OnPlanStart, OnPlanEnd, OnFunctionStart, OnFunctionEnd, OnKernelEvent → CEF
- CrewAI mapping: OnTaskStart, OnTaskEnd, OnDelegation, OnAgentDone → CEF
- AutoGen mapping: OnMessage, OnFunctionCall, OnReply, OnConversationEnd → CEF
- Field mapping: framework timestamp → CEF timestamp, framework IDs → CEF agent_id/session_id
- Event completeness: ensure critical events (spawn, complete, error) never lost
- Trace validation: run sample agents, verify continuous event stream from start to finish

## Dependencies
- **Blocked by:** Week 28
- **Blocking:** Week 30, Week 31, Week 32, Week 33, Week 34

## Acceptance Criteria
- CEF event specification detailed and comprehensive
- Telemetry mapping complete for all 5 frameworks
- Event quality validation successful
- Event completeness validated (no lost events)
- Example traces available for all frameworks
- Telemetry documentation available
- CEF event schema supports end-to-end tracing

## Design Principles Alignment
- **Observability:** Complete telemetry for debugging and monitoring
- **Framework Native:** Preserve framework-specific event semantics
- **CEF Standard:** All events comply with CEF format
