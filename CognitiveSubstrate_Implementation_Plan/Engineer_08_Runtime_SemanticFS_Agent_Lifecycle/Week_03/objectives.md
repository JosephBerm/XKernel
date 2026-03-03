# Engineer 8 — Runtime: Semantic FS & Agent Lifecycle — Week 03

## Phase: Phase 0 (Foundation)

## Weekly Objective
Design Agent Unit File format. Define declarative YAML/TOML configuration structure specifying: framework, model requirements, capability requests, resource quotas, health check endpoint, restart policy, dependency ordering, and crew membership. Create format specification and example unit files.

## Document References
- **Primary:** Section 3.4.3 — Agent Lifecycle Manager (unit files as declarative config)
- **Supporting:** Section 3.4 — L2 Agent Runtime; Section 6.2 — Phase 1 Week 11-12

## Deliverables
- [ ] Agent Unit File format specification (YAML/TOML schema)
- [ ] Format specification document with field descriptions
- [ ] 5-10 example unit files demonstrating various scenarios
- [ ] JSON schema or validation grammar for format

## Technical Specifications
- YAML schema structure for declarative agent deployment
- Fields: name, framework (LangChain/SK/CrewAI), model requirements, capabilities, resource quotas
- Health check configuration: endpoint, probe_interval, failure_threshold, timeout
- Restart policy: strategy (always|on-failure|never), max_retries, backoff_multiplier
- Dependencies: after, before, requires (other agents)
- Crew membership: crew_name, role, ordering constraints

## Dependencies
- **Blocked by:** Week 01-02 domain model analysis
- **Blocking:** Week 05-06 Agent Lifecycle Manager prototype

## Acceptance Criteria
- [ ] Unit file format specification complete and documented
- [ ] Schema supports all lifecycle_config requirements
- [ ] Example files cover: simple agent, crew setup, health checks, restarts, dependencies
- [ ] Validation logic specified for format compliance

## Design Principles Alignment
- **Declarative Configuration:** Unit files express intent, not implementation
- **Explicitness:** All agent requirements and constraints declared upfront
- **Compatibility:** Supports LangChain, Semantic Kernel, and CrewAI frameworks
