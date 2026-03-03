# Engineer 8 — Runtime: Semantic FS & Agent Lifecycle — Week 12

## Phase: Phase 1 (Health Checks & Knowledge Sources)

## Weekly Objective
Complete Agent Lifecycle Manager implementation with restart policies and dependency-aware ordering. Implement all restart strategy types (always, on-failure, never), dependency resolution for crew members, and integration tests. Agent Lifecycle Manager reaches Phase 1 completion.

## Document References
- **Primary:** Section 3.4.3 — Agent Lifecycle Manager (restart policies, dependency ordering, unit files); Section 6.2 — Phase 1 Week 11-12
- **Supporting:** Section 3.4 — L2 Agent Runtime

## Deliverables
- [ ] Restart policy implementation (always, on-failure:N, never)
- [ ] Backoff strategy: exponential backoff with jitter for retries
- [ ] Dependency resolution algorithm for crew members
- [ ] Ordered startup/shutdown based on dependencies (DAG traversal)
- [ ] Unit file integration: parse and apply all lifecycle config
- [ ] Comprehensive test suite: restart scenarios, dependency ordering, crews

## Technical Specifications
- Restart strategies: always (immediate), on-failure (with N max attempts), never
- Backoff: exponential multiplier, jitter, max delay cap
- Dependency DAG: topological sort for startup ordering, reverse for shutdown
- Crew orchestration: start members in order, stop in reverse order
- Health check integration: unhealthy status triggers restart decision
- State management: track restart counts, last restart time, restart history

## Dependencies
- **Blocked by:** Week 11 health check probe implementation; Week 06 Agent Lifecycle Manager prototype
- **Blocking:** Week 13-14 hot-reload capability and cs-agentctl CLI completion

## Acceptance Criteria
- [ ] All restart strategy types implemented and working
- [ ] Backoff logic prevents restart storms
- [ ] Dependency resolution handles complex crew topologies
- [ ] Ordered startup/shutdown verified for crews
- [ ] 20+ integration tests passing (restart scenarios, crews, dependencies)
- [ ] Agent Lifecycle Manager feature-complete for Phase 1

## Design Principles Alignment
- **Reliability:** Automatic recovery from transient failures
- **Composability:** Crews properly ordered and managed
- **Observability:** Restart decisions and history logged
