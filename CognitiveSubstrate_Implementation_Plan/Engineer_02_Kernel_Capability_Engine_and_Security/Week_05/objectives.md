# Engineer 2 — Kernel: Capability Engine & Security — Week 5

## Phase: PHASE 0 - Domain Model + Kernel Skeleton

## Weekly Objective
Complete MMU-backed capability enforcement integration. Ensure page table mappings are created ONLY when capabilities are held, preventing unauthorized memory access at hardware level. Implement capability-to-page-table binding with comprehensive testing.

## Document References
- **Primary:** Section 3.2.3 (MMU-backed Capability Enforcement - Hardware Enforcement), Section 3.2.3 (Capability Enforcement Engine)
- **Supporting:** Section 2.4 (Capability Formalization), Architecture documentation on MMU integration

## Deliverables
- [ ] MMU abstraction layer for bare-metal architecture (TLB, page table operations)
- [ ] Capability-to-page-table binding mechanism (1:1 mapping of capabilities to page entries)
- [ ] Page table entry lifecycle management (create on Grant, update on Delegate, invalidate on Revoke)
- [ ] TLB invalidation strategy for revoked capabilities (local + IPI for multi-core)
- [ ] Hardware permission enforcement validation (read/write/execute at hardware level)
- [ ] Cross-agent isolation validation (Agent A cannot access Agent B's unmapped pages)
- [ ] Performance profiling (page table lookup, TLB hit rates, invalidation overhead)
- [ ] Integration tests with all 6 capability operations (200+ tests)
- [ ] Architectural documentation of MMU integration

## Technical Specifications
- **MMU Abstraction Layer:**
  - Platform-independent interface to MMU operations
  - Supported architectures: x86_64, ARM64 (with platform-specific backends)
  - Operations: allocate_pagetable(), map_page(), unmap_page(), invalidate_tlb()
  - Support for multi-level page tables (typical 4-level on x86_64, 4-level on ARM64)
  - Atomic page table updates (via hardware-assisted CAS or lock-based serialization)
- **Capability-Page Table Binding:**
  - Each page table entry includes: physical_address, permission_bits, capability_id, owner_agent
  - permission_bits derived from Capability.operations: [read → 0x1, write → 0x2, execute → 0x4]
  - Hardware enforces permission bits: fault on unauthorized access
  - No capability → no page table entry → no hardware access (fail-safe default)
- **Page Table Entry Lifecycle:**
  - Grant: allocate physical page, create PTE with capability_id and operations→permissions mapping
  - Delegate: update PTE owner_agent field, add delegation chain reference
  - Revoke: invalidate PTE, flush TLB on all cores, return page to free pool
  - Attenuation: update PTE permission_bits to subset of original operations
- **TLB Invalidation:**
  - Local invalidation (invlpg x86, tlbi ARM64) for single-core systems
  - Inter-processor interrupt (IPI) strategy for multi-core: broadcast REVOKE to all cores, each issues local invalidation
  - Latency target: <5000ns on 8-core system for full TLB invalidation
- **Hardware Permission Enforcement:**
  - Read fault if page marked unreadable (PTE.readable == 0) and read attempted
  - Write fault if page marked unwritable (PTE.writable == 0) and write attempted
  - Execute fault if page marked non-executable (PTE.executable == 0) and fetch attempted
  - All faults trapped to kernel exception handler, logged, and signal dispatched to agent
- **Cross-Agent Isolation Proof:**
  - Agent A's page tables contain no entries for Agent B's memory regions
  - Grant operations validated to only map Agent B's memory if Agent B holds capability
  - Delegate operations cannot map memory outside delegated resource

## Dependencies
- **Blocked by:** Week 3-4 (Capability Enforcement Engine operations), Week 2 (MandatoryCapabilityPolicy)
- **Blocking:** Week 6 (local capability check optimization), Week 7 (Phase 1)

## Acceptance Criteria
- MMU abstraction layer supports both x86_64 and ARM64 platforms
- Page table entries created ONLY for held capabilities
- Hardware permission enforcement prevents unauthorized memory access
- All 200+ integration tests pass (Grant, Delegate, Revoke, Audit, Membrane, Policy Check with MMU)
- Cross-agent isolation verified: Agent A cannot read, write, or execute Agent B's unmapped memory
- TLB invalidation completes in <5000ns on 8-core system
- Performance profiling shows <5% overhead from capability-to-page mapping mechanism
- Code review completed by kernel architecture team

## Design Principles Alignment
- **P1 (Security-First):** Hardware enforces capability checks, not software policy
- **P2 (Transparency):** Page table entries provide complete visibility into agent memory access
- **P3 (Granular Control):** Per-page permissions enable fine-grained access control
- **P4 (Performance):** Hardware permission checks have zero software overhead
- **P5 (Formal Verification):** MMU behavior can be formally verified against threat model
- **P8 (Robustness):** Hardware enforcement is fail-safe: no access by default
