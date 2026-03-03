# Engineer 8 — Runtime: Semantic FS & Agent Lifecycle — Week 11

## Phase: Phase 1 (Health Checks & Knowledge Sources)

## Weekly Objective
Begin full Agent Lifecycle Manager implementation. Focus on health check probe mechanisms: endpoint configuration, periodic probe scheduling, and N-consecutive-failure restart trigger logic. Lay groundwork for restart policy implementation in Week 12.

## Document References
- **Primary:** Section 3.4.3 — Agent Lifecycle Manager (health checks, restart policies); Section 6.2 — Phase 1 Week 11-12
- **Supporting:** Section 3.4 — L2 Agent Runtime

## Deliverables
- [ ] Health check probe implementation (HTTP/gRPC endpoint monitoring)
- [ ] Probe scheduling mechanism with configurable intervals
- [ ] Failure counting and N-consecutive-failure detection logic
- [ ] Health status state machine (healthy, degraded, unhealthy)
- [ ] Integration with Agent unit files for health check config
- [ ] Comprehensive test suite for health check scenarios
- [ ] Formal Agent Unit File Schema specification in TOML format with JSON Schema validation (Addendum v2.5.1 — Correction 6: Format Specifications)

## Technical Specifications
- Probe types: HTTP GET/HEAD, gRPC health check, custom scripts
- Scheduling: periodic probes with configurable interval (e.g., every 10s)
- Failure detection: track N consecutive failures, threshold-based transition
- Timeout handling: probe timeout triggers failure count increment
- State transitions: healthy→degraded→unhealthy with clear semantics
- Logging: all probe results and state transitions logged

### Agent Unit File Format Specification

The Agent Unit File defines all 8 properties required by Section 3.4.3:

```toml
[agent]
name = "research-agent-alpha"
framework = "langchain"
crew = "research-team-01"

[model]
name = "gpt-4-turbo"
context_window = 128000
max_tokens_per_ct = 4096

[capabilities]
requests = ["tool:web_search:invoke", "memory:l2:read_write", "memory:l3:read_only"]

[resources]
max_tokens = 50000
max_gpu_ms = 10000
max_wall_clock_s = 300
max_tool_calls = 100

[health]
endpoint = "/health"
interval_s = 30
failure_threshold = 3

[restart]
policy = "on_failure"  # always | on_failure | never
max_retries = 5
backoff_ms = 1000

[dependencies]
requires = ["coordinator-agent"]
start_after = ["knowledge-base-agent"]
```

**Eight Required Properties:**
1. **framework** — LLM framework (langchain, semantic-kernel, etc.)
2. **model_requirements** — Model name, context window, token limits per CT
3. **capability_requests** — List of required capabilities with access levels
4. **resource_quotas** — Max tokens, GPU ms, wall-clock time, tool calls
5. **health_check_endpoint** — HTTP/gRPC endpoint path for probes
6. **restart_policy** — always | on_failure | never
7. **dependency_ordering** — Agents that must start before this agent
8. **crew_membership** — Which crew/team this agent belongs to

## Dependencies
- **Blocked by:** Week 06 Agent Lifecycle Manager prototype; Week 03-04 unit file format
- **Blocking:** Week 12 restart policy and dependency ordering implementation

## Acceptance Criteria
- [ ] All probe types implemented and tested
- [ ] Probe scheduling working correctly across multiple agents
- [ ] Failure detection accurate and configurable
- [ ] Health status state machine operational
- [ ] 15+ unit tests covering probe scenarios
- [ ] Integration tests with real agent endpoints

## Design Principles Alignment
- **Observability:** Health status always visible and queryable
- **Reliability:** Failure detection enables automatic recovery
- **Configurability:** Probe intervals and thresholds adjustable per agent
