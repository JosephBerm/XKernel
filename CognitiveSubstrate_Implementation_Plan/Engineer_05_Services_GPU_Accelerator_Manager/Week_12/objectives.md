# Engineer 5 — Services: GPU/Accelerator Manager — Week 12

## Phase: 1 (KV-Cache Isolation via Page Tables)
## Weekly Objective
Implement KV-cache isolation via GPU memory allocation pools: three security modes (STRICT, SELECTIVE, OPEN) control data sharing between agents. Validate performance: SELECTIVE mode ≤ 10% p95 TTFT overhead for 13B-30B models.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, KV-Cache Isolation subsection
- **Supporting:** Section 5 — Technology Decisions (MMU + embedded vector index)

## Deliverables
- [ ] KV-cache isolation modes specification (STRICT, SELECTIVE, OPEN semantics)
- [ ] GPU memory allocation pool management: Separate allocations per crew (cuMemAlloc / hipMalloc per mode)
- [ ] Mode enforcement mechanism: Memory allocation control prevents unauthorized KV access between crews
- [ ] STRICT mode: Separate GPU memory pools per crew; zero sharing
- [ ] SELECTIVE mode: Isolation by default; upgrade-to-shareable pools for non-sensitive data
- [ ] OPEN mode: Global KV-cache pool; single-tenant per GPU (degraded security)
- [ ] Pool-level access control: Memory allocation tracking on mode changes, pool enforcement
- [ ] Performance instrumentation: TTFT (Time-to-First-Token) measurement per mode
- [ ] Validation testing: STRICT, SELECTIVE, OPEN modes; single & multi-crew scenarios
- [ ] Security audit: Verify memory isolation prevents unauthorized KV access

## Technical Specifications
- KV-cache size: 13B-30B models, 2-8 layers of cache per inference request
- Memory allocation granularity: GPU memory pool per crew (via cuMemAlloc / hipMalloc)
- Mode semantics:
  - STRICT: Separate GPU memory allocations per crew; maximum isolation (highest memory overhead)
  - SELECTIVE: Shared allocations marked SENSITIVE (isolated pools) vs. SHAREABLE (reused pools); dynamic upgrade
  - OPEN: Global KV pool via single allocation; fastest; minimal isolation (risky for hostile crews)
- Memory enforcement: Kernel tracks allocations per crew; GPU Manager prevents cross-crew access via allocation control
- SELECTIVE target: p95 TTFT overhead < 10% vs. STRICT (measure time-to-first-output token)

## Dependencies
- **Blocked by:** Week 11 (Multi-model VRAM management)
- **Blocking:** Week 13-14 (Multi-GPU support), Week 29-30 (KV-cache side-channel testing)

## Acceptance Criteria
- [ ] STRICT mode isolation verified: No cross-crew KV access possible via memory allocation control
- [ ] SELECTIVE mode p95 TTFT overhead < 10% vs. STRICT baseline
- [ ] OPEN mode performance confirmed (baseline for comparison)
- [ ] Mode transitions (e.g., STRICT ↔ SELECTIVE) work without crashes
- [ ] Pool-level access control tests pass: Unauthorized access attempts blocked
- [ ] Security audit completed; no KV isolation bypass vectors identified

## Design Principles Alignment
- **Flexible Security:** Three modes trade off security vs. performance for different threat models
- **Performance-Conscious:** SELECTIVE mode proves isolation cost is minimal
- **API-Based Isolation:** GPU memory allocation pools provide efficient isolation via CUDA/ROCm APIs

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) using GPU memory allocation pools per crew (not GPU page tables)
**Rationale:** LithOS/PhoenixOS validate memory allocation control as effective isolation mechanism
**Implementation:** Use separate cuMemAlloc / hipMalloc pools per crew; avoid custom GPU page table management
