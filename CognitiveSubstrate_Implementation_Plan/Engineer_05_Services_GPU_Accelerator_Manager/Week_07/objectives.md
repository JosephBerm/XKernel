# Engineer 5 — Services: GPU/Accelerator Manager — Week 07

## Phase: 1 (Advanced Scheduling & Multi-Model)
## Weekly Objective
Implement TPC-level spatial scheduling. Allocate specific TPCs/SMs to individual agents' inference kernels via cognitive scheduler directives. Establish LithOS-inspired spatial isolation to reduce tail latency by 13× vs. NVIDIA MPS.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, TPC-Level Spatial Scheduling subsection
- **Supporting:** Section 3.2.2 — GPU Scheduling, Section 6.2 — Phase 1, Weeks 11-14

## Deliverables
- [ ] TPC allocation data structure (TPC mask per agent/CT, allocation state machine, CUDA MPS/MIG config)
- [ ] Cognitive Scheduler ↔ GPU Manager TPC directive interface (allocate/deallocate TPCs via CUDA MPS or custom scheduling layer)
- [ ] TPC occupancy tracking (kernels active on each TPC group, utilization percentage via GPU performance counters)
- [ ] Spatial isolation enforcement: GPU hardware configuration via CUDA MPS context mapping or custom TPC scheduling layer
- [ ] Per-TPC performance monitoring (latency, throughput, memory bandwidth per group via GPU event counters)
- [ ] TPC reallocation mechanism (preempt low-priority agent, reallocate to high-priority via context switching)
- [ ] Latency measurement harness (tail latency profiling for multi-agent scenarios using GPU timestamps)
- [ ] Benchmark suite: Single-model multi-agent scenarios testing tail latency improvement (LithOS validation)

## Technical Specifications
- TPC/SM as basic schedulable unit (64-128 CUDA cores per TPC, hardware-dependent)
- Spatial scheduling: Each agent's kernels run on dedicated TPC group via CUDA MPS (Multi-Process Service) or custom scheduling layer
- LithOS reference: Achieve similar 13× tail latency reduction vs. NVIDIA MPS time-slice sharing using context-level TPC allocation
- Hardware support: GPU SM assignment via CUDA MPS process partitioning or custom kernel launch interception + TPC scheduling
- Cognitive Scheduler owns allocation decisions; GPU Manager executes via CUDA context control and kernel submission
- Monitoring: Per-TPC performance counters (execution time, memory stalls, divergence) via GPU performance API

## Dependencies
- **Blocked by:** Week 6 (Phase 0 completion, device driver ready)
- **Blocking:** Week 8 (TPC-level isolation validation), Week 9-10 (Kernel Atomization)

## Acceptance Criteria
- [ ] TPC allocation interface designed and approved by Cognitive Scheduler team
- [ ] Spatial isolation enforcement mechanism implemented and tested on real GPU
- [ ] Single-model multi-agent benchmark demonstrates tail latency improvement
- [ ] Latency profiling shows < 13µs p99 latency under 4-agent workload (target validation)
- [ ] Per-TPC monitoring infrastructure operational
- [ ] Design review: TPC scheduling logic approved by GPU architecture team

## Design Principles Alignment
- **Spatial Isolation:** TPCs dedicated to agents via CUDA MPS / custom scheduling; zero interference from competing kernels
- **Deterministic Performance:** Isolation removes scheduling variance, reducing tail latency (LithOS-validated)
- **LithOS Innovation:** Kernel-level spatial scheduling via CUDA context control outperforms userspace time-slicing

## Addendum v2.5.1 — Correction 1: GPU Driver Strategy
**Status:** Phase A (v1.0) using CUDA MPS / ROCm MIG + custom scheduling layer (LithOS-validated approach)
**Rationale:** LithOS demonstrates TPC-level scheduling via GPU context control and kernel launch queuing
**Implementation:** Use CUDA MPS context mapping or custom kernel launch interception layer to control TPC allocation (not raw MMIO register programming)
