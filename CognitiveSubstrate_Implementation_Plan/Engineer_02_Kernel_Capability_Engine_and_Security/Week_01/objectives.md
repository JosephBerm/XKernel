# Engineer 2 — Kernel: Capability Engine & Security — Week 1

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Formalize the Capability entity core domain model with all essential attributes: unforgeable CapID handles, ResourceRef targets, Operation sets, CapConstraints, provenance tracking, revocation delegation, and attenuation policies. Complete formal specification document and review with all engineering streams.

## Document References
- **Primary:** Section 2.4 (Capability Formalization), Section 2.10 (MandatoryCapabilityPolicy)
- **Supporting:** Section 3.2.3 (Capability Enforcement Engine), Section 2.1 (Architecture Overview)

## Deliverables
- [ ] Capability entity formal specification document (CapID, ResourceRef, Operation, CapConstraints structures)
- [ ] CapConstraints type definition (time-bound, rate-limited, data-volume-limited, chain-depth-limited)
- [ ] CapChain provenance tracking structure and serialization format
- [ ] Capability attribute constraints and invariants document
- [ ] Cross-stream review session with all engineering teams (Streams 1, 3, 4, 5, 6)
- [ ] Review feedback integration and specification finalization

## Technical Specifications
- **CapID:** Unforgeable cryptographic handle (256-bit secure random, kernel-assigned only)
- **ResourceRef:** (agent_id, resource_type, resource_id) tuple with type validation
- **Operation Set:** Bit-field [read | write | execute | invoke | subscribe] with composition rules
- **CapConstraints:**
  - time_bound: (start_timestamp, expiry_timestamp)
  - rate_limited: (max_operations_per_period, period_duration_ns)
  - data_volume_limited: (max_bytes_per_period, period_duration_ns)
  - chain_depth_limited: (max_delegation_depth)
- **CapChain:** Linear provenance record [CapID → delegated_to → attenuated_constraints → timestamp]
- **revocable_by:** Set of AgentIDs with explicit revocation authority
- **attenuation:** AttenuationPolicy enum (reduce_ops, time_bound, rate_limit, data_limit)

## Dependencies
- **Blocked by:** Engineering Plan v2.5 finalization (should be complete)
- **Blocking:** Week 2 (continued formalization of MandatoryCapabilityPolicy), Weeks 3-5 (enforcement engine implementation)

## Acceptance Criteria
- Capability entity specification is mathematically precise and unambiguous
- All 7 core Capability attributes are formally defined with type signatures
- CapConstraints covers all identified constraint types with clear semantics
- CapChain provenance format supports full audit trail reconstruction
- All stream leads have reviewed and provided written acceptance
- No conflicts or ambiguities identified in cross-stream review
- Specification document includes at least 5 concrete examples with various constraint combinations

## Design Principles Alignment
- **P1 (Security-First):** Unforgeable CapID ensures no forgery attacks possible at domain level
- **P2 (Transparency):** Full CapChain provenance enables complete audit trails
- **P3 (Granular Control):** Operation set and constraint types enable fine-grained permission models
- **P5 (Formal Verification):** Precise formal specification enables future proofs
- **P7 (Multi-Agent Harmony):** revocable_by set and delegation chains support multi-agent governance
