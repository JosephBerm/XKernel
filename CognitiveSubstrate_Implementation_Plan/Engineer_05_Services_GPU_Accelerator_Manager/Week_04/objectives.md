# Engineer 5 — Services: GPU/Accelerator Manager — Week 04

## Phase: 0 (GPU Manager Skeleton & Single-Model VRAM)
## Weekly Objective
Implement GPU Manager skeleton as L1 kernel service using CUDA Driver API / ROCm HIP context management. Establish basic VRAM management for single-model scenarios. Model registry tracks loaded models, VRAM footprint, and bound CTs. CUDA/ROCm integration begins testing.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager (VRAM Management), Section 6.1 — Phase 0, Weeks 4-6
- **Supporting:** Section 3.3.2 — Model Registry

## Deliverables
- [ ] GPU Manager kernel module implementation (initialization via CUDA Driver API, service registration)
- [ ] Model registry data structure (model ID, VRAM footprint, bound CTs, load timestamp, CUDA device handle)
- [ ] Single-model VRAM management implementation (cuMemAlloc/hipMalloc, tracking, bound model lifecycle)
- [ ] Model loading path: Model file → CUDA/ROCm memory allocation → registry entry → ready for inference
- [ ] Model unloading path: Registry removal → cuMemFree/hipMemFree → GPU memory coherency verification
- [ ] Integration test suite: Load/unload single model via CUDA Driver API, verify VRAM state transitions
- [ ] CUDA/ROCm integration: GPU Manager context management and device capability detection

## Technical Specifications
- GPU Manager operates as kernel service (not daemon process); uses CUDA Driver API / ROCm HIP
- Model registry: struct containing model_id, vram_footprint_bytes, bound_ct_list, load_state, cuda_device_handle
- Single-model scenario: One model loaded in VRAM at a time (simple case for Phase 0)
- VRAM allocation: cuMemAlloc (CUDA) or hipMalloc (ROCm) for model weights; automatic GPU memory management
- Model binding: Cognitive Scheduler allocates CTs to GPU; GPU Manager records CT→model association
- VRAM bounds: Assume fixed single-model partition (e.g., first 16GB of VRAM reserved for model)

## Dependencies
- **Blocked by:** Week 3 (Device Driver Interface design complete)
- **Blocking:** Week 5-6 (GPU command submission queue), Week 11-12 (Multi-model VRAM)

## Acceptance Criteria
- [ ] GPU Manager module loads/unloads cleanly in kernel
- [ ] Model registry correctly tracks loaded models and VRAM allocation
- [ ] VRAM allocation/deallocation tested with CUDA Driver API / ROCm HIP
- [ ] Single-model load/unload cycle passes integration tests
- [ ] CUDA/ROCm integration checkpoint tested (basic cuMemAlloc/hipMalloc working)
- [ ] Code review: Kernel team validates architecture and safety; CUDA/ROCm integration verified

## Design Principles Alignment
- **Kernel Service Pattern:** GPU Manager as L1 service, not daemon (kernel-native control via CUDA/ROCm APIs)
- **Simple First:** Single-model case establishes foundation for Phase 1 multi-model
- **Registry-Driven:** Model tracking via registry enables future dynamic loading/eviction

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) implementation using CUDA Driver API / ROCm HIP context management
**Rationale:** LithOS and PhoenixOS validate CUDA/ROCm abstraction layer approach for kernel context control and memory isolation
**Implementation:** Use cuDeviceCreate, cuCtxCreate (CUDA) or hipDeviceGet, hipCtxCreate (ROCm); avoid custom MMIO register access
