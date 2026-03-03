# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 11

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Integrate with GPU Manager (Engineer 5). Coordinate CPU scheduling with GPU TPC allocation. Implement dual-resource scheduling for inference phases that require both CPU and GPU.

## Document References
- **Primary:** Section 3.2.2 (This scheduler controls two hardware resources simultaneously: CPU cores and GPU compute units (TPCs/SMs); GPU Scheduling (LithOS-inspired))
- **Supporting:** Section 3.3.2 (GPU / Accelerator Manager with TPC-Level Spatial Scheduling and Kernel Atomization), Section 3.2.2 (Capability Cost dimension for GPU-heavy phases)

## Deliverables
- [ ] Rust module `dual_resource_scheduler.rs` — coordinate CPU and GPU scheduling
- [ ] GPU Manager interface definition — collaborate with Engineer 5 to define contract
- [ ] TPC allocation request/grant mechanism — CT can request TPCs for inference phase
- [ ] GPU-CPU co-scheduling — when CT enters reason phase (inference), reserve CPU cores + GPU TPCs
- [ ] Inference latency modeling — lightweight model to predict TPC allocation needed for target latency
- [ ] Dynamic right-sizing — allocate minimal TPCs to meet latency SLO, reclaim excess for other agents
- [ ] Scheduler integration — runqueue considers both CPU and GPU availability before scheduling
- [ ] Test suite — 15+ test cases covering dual-resource allocation, latency modeling, right-sizing

## Technical Specifications
**Dual-Resource Scheduling (Section 3.2.2):**
- Unlike CPU scheduling alone, this scheduler manages two resources simultaneously
- When CT enters reason phase, scheduler must allocate:
  1. CPU cores (typically 1-4 for batching control, remaining cores for system)
  2. GPU TPCs/SMs (for inference kernel execution)
- CPU cores run LLM inference framework (vLLM, SGLang, TensorRT-LLM), submit kernels to GPU
- GPU TPCs execute kernels for multiple CTs in parallel

**TPC Allocation (Section 3.3.2):**
- LithOS-inspired: each agent's inference kernels allocated specific TPCs, managed like CPU cores
- Example: 128 TPCs available, Agent A gets 32 (25%), Agent B gets 32 (25%), Agent C gets 32 (25%), system reserved 32
- Kernel atomization: long-running kernels split into atoms (subsets of thread blocks) without app changes
- Allocation dynamically adjusted based on deadline pressure

**Inference Latency Modeling (Section 3.3.2):**
- Lightweight predictor: given model size, sequence length, TPC count → predict kernel latency
- Used to determine minimal TPC allocation to meet SLO
- Example: 13B model with 2048 tokens
  - 64 TPCs: 50ms (p99)
  - 32 TPCs: 95ms (p99)
  - 16 TPCs: 180ms (p99)
- If SLO is <100ms, allocate 32 TPCs; if <200ms, allocate 16 TPCs

**Scheduler Decision Logic:**
```rust
// When CT ready for reason phase
fn allocate_gpu_resources(ct: &CognitiveTask, slo_ms: u32) -> GpuAllocation {
  let tpc_count = latency_model.predict_tpc_count(ct.model, ct.context_len, slo_ms);
  let gpu_manager = get_gpu_manager();
  match gpu_manager.request_tpc(tpc_count, ct.priority) {
    Available(tpcs) => GpuAllocation { tpcs, ... },
    Unavailable => CT waits in GPU queue
  }
}
```

## Dependencies
- **Blocked by:** Week 10 (deadlock detection for resource contention context), Engineer 5 must define GPU Manager interface by Week 11
- **Blocking:** Week 12 (full GPU integration), Week 20-24 (performance tuning)

## Acceptance Criteria
- [ ] GPU Manager interface defined in collaboration with Engineer 5
- [ ] CT can request TPC allocation for inference phase
- [ ] Dual-resource scheduler correctly allocates CPU + GPU
- [ ] Inference latency model implemented and validated
- [ ] Dynamic right-sizing correctly allocates minimal TPCs for SLO
- [ ] Scheduler respects TPC availability (doesn't over-allocate)
- [ ] All 15+ test cases pass
- [ ] Integration test with Engineer 5: spawn 5 CTs with inference, verify co-scheduling

## Design Principles Alignment
- **P2 — Cognitive Primitives as Kernel Abstractions:** GPU scheduling is kernel-level, not userspace framework
- **P7 — Production-Grade from Phase 1:** Dual-resource scheduling is essential for efficient inference workloads
