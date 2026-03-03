# Engineer 5 — Services: GPU/Accelerator Manager — Week 03

## Phase: 0 (Device Driver Interface Design)
## Weekly Objective
Design GPU abstraction layer interface leveraging CUDA Driver API and ROCm HIP for device communication. Specify GPU context management, command submission queue protocol via standard APIs, and kernel-level abstraction layer. Establish bridge between kernel and GPU hardware using standard driver stacks (Phase A v1.0 approach).

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, Device Driver Interface subsection
- **Supporting:** Section 5 — Technology Decisions (GPU hardware abstraction)

## Deliverables
- [ ] CUDA Driver API / ROCm HIP abstraction layer specification (context, stream, memory management)
- [ ] Command submission protocol design via CUDA streams and kernel launch queuing
- [ ] GPU event handling specification (kernel completion signals, error handling, device synchronization)
- [ ] Memory management interface (allocation via cuMemAlloc/hipMalloc, coherency management)
- [ ] Driver abstraction layer API (C interface for kernel to control GPU via CUDA Driver API)
- [ ] Device driver integration skeleton (CUDA device discovery, context initialization, capability detection)

## Technical Specifications
- CUDA Driver API control: Kernel abstracts GPU operations through standard CUDA Driver API (cuMemAlloc, cuLaunchKernel, etc.)
- Command submission: CUDA streams and kernel launch queues managed by driver stack (not raw MMIO)
- Event handling: GPU events via cuEventCreate/hipEventCreate; kernel completion via stream synchronization
- Memory access: Kernel-controlled GPU memory via CUDA/ROCm APIs; automatic cache coherency via drivers
- API protection: Kernel encapsulates GPU calls; userspace cannot directly invoke GPU commands
- Hardware compatibility: NVIDIA H100/H200/B200 (CUDA 12.x) as P0, AMD MI300X (ROCm HIP) as P1 stretch
- Vendor abstraction: Phase A (v1.0) uses CUDA/ROCm; Phase B (v2.0) native driver as future roadmap

## Dependencies
- **Blocked by:** Week 2 (Domain model and state machine)
- **Blocking:** Week 4-5 (GPU Manager skeleton implementation)

## Acceptance Criteria
- [ ] CUDA Driver API / ROCm HIP abstraction validated against target hardware (NVIDIA H100 / AMD MI300X)
- [ ] Command submission protocol proven with CUDA stream/kernel launch APIs
- [ ] GPU event handling design reviewed for safety and correctness
- [ ] Memory interface compatible with VRAM management requirements (Week 4-5)
- [ ] Device driver integration skeleton compiles and links with kernel codebase
- [ ] Design document approved by kernel architecture team; Phase A vs Phase B roadmap documented

## Design Principles Alignment
- **Kernel-Native GPU Management:** L1 service uses CUDA Driver API / ROCm HIP; no userspace intermediary
- **Hardware Abstraction:** Standard APIs abstract vendor-specific control mechanisms from upper layers
- **Safety & Isolation:** Kernel encapsulation protects from malicious or buggy userspace GPU access
- **Phase A (v1.0) Focus:** Leverage existing CUDA/ROCm driver maturity; Phase B native driver (future roadmap)

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) implementation using CUDA Driver API / ROCm HIP abstraction layer
**Rationale:** LithOS (SOSP 2025) and PhoenixOS (SOSP 2025) validated approaches for TPC scheduling and concurrent C/R run atop CUDA/ROCm stacks, not bare-metal MMIO.
**Phase B (v2.0):** Post-GA roadmap includes exploration of native GPU driver interface with direct MMIO register access (long-term ambition; AMD open ISA more feasible than NVIDIA proprietary stack).
