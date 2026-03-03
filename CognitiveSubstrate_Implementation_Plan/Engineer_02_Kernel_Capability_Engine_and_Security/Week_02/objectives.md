# Engineer 2 — Kernel: Capability Engine & Security — Week 2

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Complete formalization of MandatoryCapabilityPolicy entity with all policy attributes, enforcement modes, exception handling, and scope semantics. Integrate with Capability formalization from Week 1 and validate consistency across both models.

## Document References
- **Primary:** Section 2.10 (MandatoryCapabilityPolicy), Section 2.4 (Capability Formalization)
- **Supporting:** Section 3.3.6 (Mandatory Policy Check Integration), Section 2.1 (Architecture Overview)

## Deliverables
- [ ] MandatoryCapabilityPolicy entity formal specification document
- [ ] Policy enforcement mode definitions (deny, audit, warn) with semantics
- [ ] Exception handling framework for policy overrides
- [ ] Scope semantics formalization (agent-scoped, system-wide, crew-scoped)
- [ ] Policy-Capability interaction consistency proofs
- [ ] Integration test plan for policy checks against capability operations
- [ ] Formal specification review completion with all streams

## Technical Specifications
- **MandatoryCapabilityPolicy:**
  - id: PolicyID (unique policy identifier)
  - rule: PolicyRule (predicate on Capability attributes)
  - scope: PolicyScope (system_wide | agent_scoped(AgentID) | crew_scoped(CrewID))
  - enforcement: EnforcementMode (deny | audit | warn)
  - exceptions: Set<ExceptionPath> (explicit whitelist of exemptions)
  - created_timestamp: u64
  - audit_retention_period: u64 (nanoseconds)
- **PolicyRule:** Compositional predicates on:
  - target resource type and classification
  - operation set restrictions
  - constraint combinations (time bounds, rate limits)
  - agent identity and role
- **EnforcementMode:**
  - deny: Block operation + log security event + return error
  - audit: Allow operation + log full audit record + no performance penalty
  - warn: Allow operation + log warning + return notification to invoker
- **ExceptionPath:** (policy_id, capid_pattern, exemption_reason, authorized_by, expiry_timestamp)

## Dependencies
- **Blocked by:** Week 1 (Capability formalization) completion
- **Blocking:** Week 3 (Capability Enforcement Engine implementation), Week 4 (mandatory policy check integration)

## Acceptance Criteria
- MandatoryCapabilityPolicy specification is formally complete and unambiguous
- All 6 core attributes precisely defined with type signatures and invariants
- Policy scope semantics enable agent isolation, crew boundaries, and system-wide rules
- Exception framework supports legitimate exemptions without policy bypass
- Policy-Capability interaction proofs demonstrate no capability grants violate policies
- Integration test plan covers at least 12 policy-capability interaction scenarios
- All stream leads provide written acceptance

## Design Principles Alignment
- **P1 (Security-First):** Mandatory policies cannot be circumvented by agents
- **P2 (Transparency):** Audit mode provides complete visibility into policy enforcement decisions
- **P3 (Granular Control):** Policy scope enables fine-grained application across system hierarchy
- **P5 (Formal Verification):** Formal specification enables proofs of policy non-bypassability
- **P6 (Compliance & Audit):** audit mode and retention periods support regulatory requirements
- **P7 (Multi-Agent Harmony):** scope semantics enable crew-level and system-level policies
