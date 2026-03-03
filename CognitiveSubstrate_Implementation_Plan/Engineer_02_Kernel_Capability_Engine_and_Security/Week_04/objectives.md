# Engineer 2 — Kernel: Capability Engine & Security — Week 4

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Complete remaining Capability Enforcement Engine operations: Revoke (invalidate + unmap pages + SIG_CAPREVOKED signal), Audit (full provenance chain queries), Membrane (transparent wrapper for bulk attenuation/revocation), and Policy Check (validate against MandatoryCapabilityPolicies before page mapping).

## Document References
- **Primary:** Section 3.2.3 (Capability Enforcement Engine - 6 Operations), Section 3.3.6 (Mandatory Policy Check Integration)
- **Supporting:** Section 2.4 (Capability Formalization), Section 2.10 (MandatoryCapabilityPolicy)

## Deliverables
- [ ] Revoke operation implementation with page table unmapping cascade
- [ ] SIG_CAPREVOKED signal definition and dispatch system
- [ ] Revocation propagation to all derived capability holders
- [ ] Audit operation implementation (full CapChain query interface)
- [ ] Audit query performance optimization (<10ms for typical chains)
- [ ] Membrane pattern implementation (transparent wrapper for bulk operations)
- [ ] Policy Check operation (pre-mapping validation against MandatoryCapabilityPolicies)
- [ ] Unit tests for all 4 operations (70+ tests total)
- [ ] Integration tests for complete operation set

## Technical Specifications
- **Revoke Operation:**
  - Invalidates CapID in kernel capability table
  - Cascades unmap to all page table entries created by Grant
  - Finds all delegated capabilities derived from revoked capability
  - Recursively revokes all derived capabilities
  - Dispatches SIG_CAPREVOKED signal to all affected agents
  - Signal includes: (revoked_capid, revoker_agent, revocation_reason, timestamp)
  - Atomic: either fully succeeds or rolls back, no partial revocations
  - Latency target: <2000ns (per capability in derivation tree)
- **Audit Operation:**
  - Query interface: audit(capid) → CapChain
  - Returns complete linear history of grants, delegations, attenuations
  - Includes all policy checks performed at each step
  - Supports range queries: audit_by_timestamp(start, end) → Set<CapChain>
  - Supports provenance queries: audit_by_agent(agent_id) → Set<CapChain>
  - Returns immutable view (copy) of audit data
  - Latency target: <10ms for typical chains (<100 entries)
- **Membrane Pattern:**
  - Wraps set of capabilities for agents entering sandbox boundary
  - Bulk attenuation: apply constraint to all wrapped capabilities
  - Bulk revocation: revoke all wrapped capabilities in one atomic operation
  - Transparent wrapper: agents use capabilities transparently, wrapper applies rules
  - Integration point with AgentCrew shared memory regions
- **Policy Check Operation:**
  - Invoked before Grant creates page table mapping
  - Validates resulting capability against all applicable MandatoryCapabilityPolicies
  - Checks scope (system_wide, agent_scoped, crew_scoped)
  - Evaluates policy rules (operation set, constraint, resource type, agent identity)
  - Checks exceptions (whitelisted exemptions)
  - Enforces mode (deny → error, audit → log + allow, warn → notify + allow)
  - Latency target: <100ns (amortized with caching)

## Dependencies
- **Blocked by:** Week 3 (Grant and Delegate), Week 2 (MandatoryCapabilityPolicy)
- **Blocking:** Week 5 (MMU integration completion), Week 6 (local capability check optimization)

## Acceptance Criteria
- Revoke operation fully implements cascade unmapping and signal dispatch
- All page table entries for revoked capability and derivatives are unmapped
- SIG_CAPREVOKED correctly delivered to all affected agents
- Revoke latency <2000ns per capability in derivation tree
- Audit operation supports all 3 query types with consistent results
- Audit latency <10ms for typical chains
- Membrane pattern integrates cleanly with AgentCrew shared memory
- Policy Check prevents all policy-violating capability grants
- All 70+ tests pass with >95% code coverage
- Code review completed by security team lead

## Design Principles Alignment
- **P1 (Security-First):** Policy Check prevents policy-violating grants at enforcement point
- **P2 (Transparency):** Audit operation provides complete visibility into capability provenance
- **P3 (Granular Control):** Membrane enables bulk attenuation without individual agent intervention
- **P4 (Performance):** Sub-microsecond checks with amortized caching
- **P5 (Formal Verification):** Revocation cascade can be proved complete
- **P6 (Compliance & Audit):** Audit operation supports regulatory compliance needs
- **P7 (Multi-Agent Harmony):** Revocation cascade handles multi-agent derivation trees
