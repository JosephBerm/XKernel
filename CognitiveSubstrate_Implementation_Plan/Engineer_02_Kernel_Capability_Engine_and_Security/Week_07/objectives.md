# Engineer 2 — Kernel: Capability Engine & Security — Week 7

## Phase: PHASE 1 - Core Services + Multi-Agent

## Weekly Objective
Implement capability delegation chains with full attenuation support. Enable reduction of permissions, time-bound constraints, read-only restrictions, and complete provenance tracking across delegation hierarchy.

## Document References
- **Primary:** Section 2.4 (Capability Delegation & Attenuation), Section 3.2.3 (Capability Enforcement Engine - Delegate Operation)
- **Supporting:** Section 2.4 (CapChain Provenance), Section 2.10 (MandatoryCapabilityPolicy)

## Deliverables
- [ ] Attenuation policy implementation (reduce_ops, time_bound, read_only, rate_limit, data_volume_limit)
- [ ] Delegation chain construction algorithm
- [ ] Attenuation validation (ensures delegated cap is subset of original)
- [ ] CapChain provenance recording (full history of all delegations with timestamps)
- [ ] Constraint composition rules (combining multiple attenuations on one delegation)
- [ ] Delegation depth tracking and chain-depth-limited constraint enforcement
- [ ] Comprehensive test suite (100+ tests for all attenuation scenarios)
- [ ] Performance profiling (delegation chain lookup, constraint validation latency)

## Technical Specifications
- **Attenuation Policies:**
  - reduce_ops: Set<Operation> subset (e.g., {read} ⊂ {read, write})
  - time_bound: (start_ns, expiry_ns) intersection with original time bounds
  - read_only: restrict to read operation only (special case of reduce_ops)
  - rate_limit: (max_ops_per_period, period_ns) composition via minimum
  - data_volume_limit: (max_bytes_per_period, period_ns) composition via minimum
  - Composition: all attenuations AND together (most restrictive wins)
- **Delegation Chain Construction:**
  - Immutable linear chain: CapID[0] → CapID[1] → CapID[2] → ... → CapID[n]
  - Each entry: (capid, holder_agent, attenuation_applied, timestamp, delegated_by_agent)
  - Stored in kernel persistent store via capability service
  - Forward pointers enable delegation (parent CapID points to children)
  - Backward pointers enable revocation (child can identify parent for cascade)
- **Attenuation Validation:**
  - On each delegation step, verify delegated capability ⊆ original capability
  - Operations: delegated_ops ⊆ original_ops (bitwise AND check)
  - Time bounds: delegated_expiry ≤ original_expiry (unsigned comparison)
  - Rate limits: delegated_rate ≤ original_rate (unsigned comparison)
  - Data limits: delegated_volume ≤ original_volume (unsigned comparison)
  - All validations performed before creating new CapID
- **CapChain Provenance Recording:**
  - Every delegation step appended to immutable provenance log
  - Provenance entry: (delegating_capid, new_capid, attenuation, delegating_agent, delegated_to_agent, timestamp)
  - Total ordering via lamport timestamps or kernel clock
  - Persistent storage: capability service persists each entry
  - Audit interface: full chain retrievable via audit(capid)
- **Delegation Depth Tracking:**
  - Capability.chain_depth = number of delegations from root
  - constraint.chain_depth_limited(max_depth) prevents excessive delegation
  - On each delegation: new_depth = parent_depth + 1
  - If new_depth > constraint.max_depth → delegation rejected
  - Prevents delegation loops and excessive indirection
- **Constraint Composition:**
  - Combining time bounds: new_expiry = min(parent_expiry, delegation_expiry)
  - Combining operation sets: new_ops = parent_ops ∩ delegation_ops
  - Combining rate limits: new_rate = min(parent_rate, delegation_rate)
  - Each layer restricts further, no expansion allowed

## Dependencies
- **Blocked by:** Week 3-6 (Capability Enforcement Engine, MMU integration, capability table optimization)
- **Blocking:** Week 8 (continuation of delegation chains), Week 9 (Membrane pattern for sandboxes)

## Acceptance Criteria
- Attenuation policies implement all 5 constraint types with correct semantics
- Delegation chains are immutable and fully auditable
- Attenuation validation prevents any capability expansion
- CapChain provenance records every delegation with full context
- Delegation depth limiting prevents infinite delegation chains
- Constraint composition produces correct results for all combinations
- All 100+ tests pass with >95% code coverage
- Delegation latency <1500ns (p50), <3000ns (p99)
- Code review completed by security team

## Design Principles Alignment
- **P1 (Security-First):** Attenuation validation ensures no privilege elevation
- **P2 (Transparency):** CapChain provenance enables complete audit trails
- **P3 (Granular Control):** Fine-grained attenuation supports least-privilege delegation
- **P5 (Formal Verification):** Attenuation rules can be formally verified as monotonic reduction
- **P6 (Compliance & Audit):** CapChain supports complete delegation history queries
- **P7 (Multi-Agent Harmony):** Delegation chains enable safe multi-agent capability sharing
