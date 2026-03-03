# Engineer 5 — Services: GPU/Accelerator Manager — Week 09

## Phase: 1 (Kernel Atomization)
## Weekly Objective
Implement kernel atomization: transparently split long-running GPU kernels into schedulable atoms (thread block subsets) via API-level kernel launch interception, without modifying application code or PTX. Eliminate head-of-line blocking and enable mid-execution TPC reallocation.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, Kernel Atomization subsection
- **Supporting:** Section 5 — Technology Decisions, Section 3.2.2 — GPU Scheduling

## Deliverables
- [ ] Kernel atom definition and specification (thread block subset, execution scope)
- [ ] API-level kernel launch interception: Intercept cuLaunchKernel / hipLaunchKernel calls to split into atoms
- [ ] Atom boundary identification framework (determine thread block ranges for atoms based on kernel semantics)
- [ ] Atom descriptor generation (atom ID, thread block range, shared state, kernel arguments)
- [ ] Mid-execution preemption mechanism (save atom state via GPU memory snapshots, resume on different TPCs)
- [ ] Atom execution scheduler (sequence atoms via kernel launch queuing, handle dependencies, reallocation events)
- [ ] Memory coherency handling (ensure atom results visible to subsequent atoms via GPU synchronization)
- [ ] Testing suite: Long-running kernels (10M+ thread blocks) with realtime reallocation

## Technical Specifications
- Atom granularity: Thread block subset (e.g., 256-1024 blocks per atom)
- Instrumentation: API-level kernel launch interception (cuLaunchKernel / hipLaunchKernel hook)
- Preemption: Save block-local state (via GPU memory staging, shared memory snapshots); restore on resume
- Reallocation: Atom can resume on different TPC group after preemption via new CUDA context / HIP stream
- No PTX changes: Application uses standard CUDA kernels; kernel atomizes at API launch level (not PTX modification)
- Compiler independence: Works with any CUDA architecture (Turing, Ampere, Hopper) via API-level interception

## Dependencies
- **Blocked by:** Week 8 (TPC scheduling validation)
- **Blocking:** Week 10-11 (Dynamic hardware right-sizing), Week 12-13 (Multi-model VRAM)

## Acceptance Criteria
- [ ] Atomization engine produces correct atom boundaries for standard CUDA kernels
- [ ] Binary instrumentation adds < 5% overhead in kernel execution time
- [ ] Mid-execution preemption tested: atom saves state, resumes correctly on new TPC
- [ ] Long-running kernel test (10M+ blocks) with realtime reallocation works correctly
- [ ] No memory corruption or race conditions under concurrent atoms
- [ ] Code review: Atomization logic approved; compiler interaction validated

## Design Principles Alignment
- **Transparent Optimization:** Application code unchanged; kernel handles atomization via API-level interception
- **Fine-Grained Control:** Atoms enable mid-execution scheduling for responsive allocation
- **Head-of-Line Elimination:** Long kernels no longer block concurrent agents

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) using API-level kernel launch interception (not PTX modification)
**Rationale:** LithOS validates API-level kernel launch splitting as effective approach for kernel atomization
**Implementation:** Intercept cuLaunchKernel / hipLaunchKernel to split kernel launches into atoms; avoid PTX binary instrumentation
