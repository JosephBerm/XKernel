# Engineer 2 — Kernel: Capability Engine & Security — Week 3

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Begin implementation of Capability Enforcement Engine's core 6 kernel operations: Grant, Delegate, Revoke, Audit, Membrane, and Policy Check. Start with Grant (kernel-only) and Delegate (transfer with optional attenuation) operations with comprehensive unit test coverage.

## Document References
- **Primary:** Section 3.2.3 (Capability Enforcement Engine - 6 Operations), Section 3.2.3 (MMU-backed Enforcement)
- **Supporting:** Section 2.4 (Capability Formalization), Section 2.10 (MandatoryCapabilityPolicy)

## Deliverables
- [ ] Grant operation implementation (kernel-only, page table mapping creation)
- [ ] Delegate operation implementation (capability transfer with optional attenuation)
- [ ] Capability table data structure (in-kernel storage for all active capabilities)
- [ ] Page table mapping integration layer (Grant creates mappings, Delegate updates)
- [ ] Unit test suite for Grant operation (50+ tests covering edge cases)
- [ ] Unit test suite for Delegate operation (50+ tests covering attenuation scenarios)
- [ ] Integration tests for Grant+Delegate sequences
- [ ] Performance profiling baseline for both operations

## Technical Specifications
- **Grant Operation:**
  - Kernel-only, cannot be invoked by agents
  - Creates new CapID in kernel capability table
  - Validates Capability attributes against MandatoryCapabilityPolicies
  - Creates page table mappings for granted resource
  - Atomic: either fully succeeds or fully fails, no partial state
  - Returns CapID to requesting kernel entity
  - Latency target: <500ns (warm cache)
- **Delegate Operation:**
  - AgentA can delegate its capability to AgentB
  - Optional attenuation: reduce operations, shorten time bounds, add rate limits
  - New delegation chain entry recorded in CapChain
  - Optional revocation callback registration
  - Updates page table mappings for delegated resource
  - Validates resulting capability against MandatoryCapabilityPolicies
  - Latency target: <1000ns (warm cache)
- **Capability Table:**
  - Hash map: CapID → (Capability, holder_set, page_table_mappings)
  - O(1) lookup performance
  - Lock-free reads (RCU or seqlock pattern), atomic updates
  - Persistent storage via microkernel IPC to persistence service

## Dependencies
- **Blocked by:** Week 1-2 (formal specifications), Week 2 (MandatoryCapabilityPolicy)
- **Blocking:** Week 4 (Revoke, Audit, Membrane, Policy Check), Week 5 (MMU integration completion)

## Acceptance Criteria
- Grant operation fully implements kernel-only semantics with page table mapping
- Delegate operation supports attenuation and maintains full provenance chain
- All 50+ tests for Grant pass with >95% code coverage
- All 50+ tests for Delegate pass with >95% code coverage
- Grant latency <500ns (p50), <1000ns (p99) on reference hardware
- Delegate latency <1000ns (p50), <2000ns (p99) on reference hardware
- Integration tests demonstrate correct Grant→Delegate sequences
- Code review completed by at least 2 other kernel engineers

## Design Principles Alignment
- **P1 (Security-First):** Grant is kernel-only, preventing agent-level capability forgery
- **P2 (Transparency):** All Grant/Delegate operations logged to provenance chain
- **P3 (Granular Control):** Delegate supports fine-grained attenuation of permissions
- **P4 (Performance):** Sub-microsecond operation latency for hot path
- **P5 (Formal Verification):** Precise specification enables proof of atomicity guarantees
- **P7 (Multi-Agent Harmony):** Delegation chains support multi-agent capability sharing
