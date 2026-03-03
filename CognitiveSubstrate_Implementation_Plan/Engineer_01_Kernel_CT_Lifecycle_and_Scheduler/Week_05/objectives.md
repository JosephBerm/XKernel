# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 05

## Phase: PHASE 0 — Domain Model + Kernel Skeleton (Weeks 1-6)

## Weekly Objective
Integrate with capability engine (Engineer 2). Implement capability validation on CT spawn ensuring CT's capabilities are always a subset of parent Agent's capabilities. Establish capability grant/revoke hooks for kernel enforcement.

## Document References
- **Primary:** Section 2.1 (Invariant 1 — Capabilities always subset of parent Agent), Section 2.4 (Capability entity), Section 3.2.3 (Capability Enforcement Engine — MMU-backed enforcement)
- **Supporting:** Section 2.10 (MandatoryCapabilityPolicy), Section 3.2.2 (CPU Scheduling with Capability Cost dimension)

## Deliverables
- [ ] Rust module `capability_validation.rs` — CT spawn checks capability subset invariant
- [ ] Capability subset checking algorithm — O(n) scan of parent Agent's CapabilityGraph
- [ ] Integration point with Engineer 2's capability engine — define interface for cap_grant, cap_revoke calls
- [ ] CT spawn syscall enhanced — reject spawn if child capabilities exceed parent
- [ ] Mandatory policy check hook — call into MandatoryCapabilityPolicy engine before page table mapping
- [ ] MMU-backed page table mapping — create page table entries only for capabilities held
- [ ] Signal dispatch for SIG_CAPREVOKED — when capability revoked, notify all CTs holding that capability
- [ ] Test suite — capability subset validation, policy checks, page table mapping verification

## Technical Specifications
**Capability Invariant 1 (Section 2.1):**
- Let parent_agent.capabilities = C_parent
- Let ct.capabilities = C_ct
- Invariant: C_ct ⊆ C_parent (set subset)
- Enforce at spawn time: for each cap in C_ct, verify it exists in C_parent with same or greater permissions

**Capability Enforcement (Section 3.2.3):**
- Capabilities are MMU-backed: every memory region is mapped into an agent's address space only if that agent holds the corresponding capability
- Agent A cannot read Agent B's context window because Agent A's page table does not contain a mapping for those physical pages
- Six kernel operations: Grant (creates page mapping), Delegate (transfer with optional attenuation), Revoke (invalidate + unmap), Audit (provenance query), Membrane (bulk attenuation), Policy Check (validate against MandatoryCapabilityPolicies)
- Target: capability checks <100ns per system call (O(1) handle lookups into kernel capability table)

**Mandatory Policy Validation (Section 3.2.3):**
- Before kernel creates page table mapping, consult MandatoryCapabilityPolicy engine
- Policy types: deny, audit, warn, or require_approval
- Example: "No agent may access production DB without human-approved capability"

**SIG_CAPREVOKED Signal (Section 2.8, 3.2.5):**
- Trigger: a held capability is revoked
- Default action: terminate CT using revoked capability
- Delivery: via interrupt handler, signal frame injected at safe preemption point

## Dependencies
- **Blocked by:** Week 04 (DAG implementation), and Engineer 2 must define capability engine interface by Week 05
- **Blocking:** Week 06 (full integration testing), Phase 1+ (all future work depends on capability enforcement)

## Acceptance Criteria
- [ ] Capability subset invariant enforced at spawn time
- [ ] ct_spawn rejects spawn if child cap C_ct ⊄ C_parent
- [ ] Page table mappings created only for capabilities held
- [ ] Capability revocation unmaps pages from all derived holders
- [ ] SIG_CAPREVOKED delivered and logged
- [ ] MandatoryCapabilityPolicy checks executed before every page mapping
- [ ] Test suite: 15+ cases covering subset validation, policy checks, revocation scenarios
- [ ] Coordination meeting with Engineer 2 to align interfaces

## Design Principles Alignment
- **P3 — Capability-Based Security from Day Zero:** Every agent starts with zero authority, inherits from parent
- **P2 — Cognitive Primitives as Kernel Abstractions:** Capability enforcement is kernel responsibility, not library
- **P5 — Observable by Default:** All capability operations audited and traceable
