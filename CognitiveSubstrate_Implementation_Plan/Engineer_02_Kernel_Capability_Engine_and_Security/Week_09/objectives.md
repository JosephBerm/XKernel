# Engineer 2 — Kernel: Capability Engine & Security — Week 9

## Phase: PHASE 1 - Core Services + Multi-Agent

## Weekly Objective
Implement Membrane pattern for transparent sandbox boundaries. Enable bulk attenuation and revocation of all capabilities when agent crosses sandbox boundary. Integrate with AgentCrew shared memory regions.

## Document References
- **Primary:** Section 2.4 (Membrane Pattern), Section 3.2.3 (Capability Enforcement Engine)
- **Supporting:** Section 2.1 (Architecture Overview), Engineer 7 specification on AgentCrew shared memory

## Deliverables
- [ ] Membrane abstraction layer (transparent wrapper for capability sets)
- [ ] Bulk attenuation logic (apply single constraint to all wrapped capabilities)
- [ ] Bulk revocation logic (atomic revocation of all wrapped capabilities)
- [ ] Membrane policy language (rules for attenuation based on sandbox boundary)
- [ ] Integration with AgentCrew shared memory regions (shared_memory → wrapped capabilities)
- [ ] Transparent capability invocation through membrane (agents unaware of wrapping)
- [ ] Membrane lifecycle management (create on sandbox entry, destroy on exit)
- [ ] Comprehensive test suite (120+ tests for all membrane scenarios)
- [ ] Performance impact assessment (<5% overhead for wrapped capability invocation)

## Technical Specifications
- **Membrane Abstraction:**
  - Wraps set of capabilities: Membrane<Cap_0, Cap_1, ..., Cap_n>
  - Transparent wrapper: agents reference capabilities by original handle
  - Kernel intercepts capability uses through membrane
  - All invocations routed through membrane policy enforcement
  - Membrane ID: unique identifier for each membrane instance
  - Created on agent sandbox entry, destroyed on sandbox exit
- **Bulk Attenuation Logic:**
  - Membrane.attenuate(constraint) applies to ALL wrapped capabilities
  - Constraint types: reduce_ops, time_bound, rate_limit, data_volume_limit
  - Example: entering restricted sandbox → add 1-hour time bound to all caps
  - Each attenuation creates new derived capabilities within membrane
  - Derived capabilities inherit original CapChain provenance
  - No effect on capabilities outside membrane (Agent B's caps remain unchanged)
- **Bulk Revocation Logic:**
  - Membrane.revoke() invalidates ALL wrapped capabilities in single atomic operation
  - Triggered on sandbox exit or security violation
  - Each revocation logged with membrane_id and reason
  - Dispatch SIG_CAPREVOKED to agent with membrane_id
  - Cascade revocation: all delegations of membrane capabilities also revoked
  - Latency target: <10000ns for revocation of 100 capabilities
- **Membrane Policy Language:**
  - DSL for defining attenuation rules: "sandbox(name) → constraints"
  - Rule syntax: sandbox(gpt4_inference) → {time_bound(1h), read_only}
  - Policy evaluation: when agent enters sandbox, matching rules applied
  - Multiple rules can apply: conjunctive (AND) composition
  - Override mechanism: admin can temporarily disable membrane attenuation (audit logged)
- **AgentCrew Shared Memory Integration:**
  - Shared memory region: Map<shared_key, value> accessible by crew agents
  - Capability to shared memory wrapped in membrane with specific policies
  - Example: shared_memory["context"] accessible by Agent A (read), Agent B (read-write), Agent C (none)
  - Membrane policies prevent Agent A from writing, prevent Agent C from reading
  - Revoke shared memory capability → prevents all crew access (mutual isolation)
- **Transparent Capability Invocation:**
  - Agent code: capability_invoke(cap_handle, operation, args)
  - Kernel checks: is cap_handle within a membrane?
  - Yes: apply membrane policies, then invoke original capability
  - No: invoke capability directly (no membrane overhead)
  - Agents are unaware of membrane wrapping
  - Transparent error handling: membrane constraint violation → error returned to agent
- **Membrane Lifecycle:**
  - Create: on sandbox entry, wrap all capabilities accessible in sandbox context
  - Update: on shared memory grant, add new capability to membrane
  - Destroy: on sandbox exit, revoke all wrapped capabilities
  - Persistence: membrane state logged for audit, reconstructed on restart
  - Memory overhead: <100 bytes per wrapped capability

## Dependencies
- **Blocked by:** Week 7-8 (delegation chains, cascade revocation), Engineer 7 (shared memory specification)
- **Blocking:** Week 10 (distributed IPC), Week 13-14 (multi-agent demo)

## Acceptance Criteria
- Membrane wrapper is truly transparent to agent code
- Bulk attenuation correctly applies to all wrapped capabilities
- Bulk revocation atomically invalidates all wrapped capabilities
- Membrane policies correctly compose with agent's original capabilities
- AgentCrew shared memory integration prevents unauthorized access
- All 120+ tests pass with >95% code coverage
- Wrapped capability invocation has <5% performance overhead vs direct invocation
- Membrane create/destroy completes in <1000ns
- Code review completed by security team

## Design Principles Alignment
- **P1 (Security-First):** Membrane enforces capability attenuation transparently
- **P2 (Transparency):** Agents unaware of membrane, but full auditability
- **P3 (Granular Control):** Per-sandbox attenuation rules enable fine-grained policies
- **P4 (Performance):** <5% overhead for high-throughput scenarios
- **P7 (Multi-Agent Harmony):** Membrane enables safe capability sharing in agent crews
- **P8 (Robustness):** Atomic revocation ensures no partial state
