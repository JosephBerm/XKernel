# GPU Accelerator Manager

> **Crate:** [`gpu_accelerator`](Cargo.toml)
> **Stream:** 2 — Kernel Services
> **Layer:** L1 (Kernel Services)
> **Owner:** Engineer 02
> **Status:** Active

---

## 1. Purpose & Scope

Manages GPU acceleration for reasoning workloads through CUDA (NVIDIA) and ROCm (AMD) backends. Handles GPU memory allocation, kernel scheduling, synchronization, and provides a unified interface to applications regardless of underlying GPU vendor. Critical for LLM reasoning workloads where GPU acceleration provides 10-100x speedup.

**Key Responsibilities:**
- GPU kernel submission and synchronization
- GPU memory (VRAM) allocation and management
- Multi-GPU load balancing across sockets/nodes
- CUDA/ROCm driver interface (with fallback to CPU)
- GPU resource quotas and fair scheduling
- GPU fault detection and recovery

**In Scope:**
- GPU device enumeration and capability detection
- Kernel launch and synchronization
- GPU memory allocation from semantic_memory L1 tier
- Multi-GPU coordination

**Out of Scope:**
- Reasoning algorithm implementation (handled by frameworks)
- GPU kernel development (handled by ML frameworks)
- System-wide power management (handled by system tools)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 4.6: GPU Integration Architecture
- Section 5.2: GPU Hardware Assumptions (NVIDIA/AMD preferred)

**Domain Model Entities Involved:**
- **CognitiveTask** — GPU resource requests during REASON phase
- **Capability** — ManageGpu capability required

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌──────────────────────────────┐
│  GPU Acceleration API        │
│  gpu_alloc(), gpu_exec()     │
└──────────────────────────────┘
             ↓
┌──────────────────────────────┐
│  GPU Driver Interface        │
├──────────┬──────────────────┤
│  CUDA    │  ROCm (HIP)      │
│  Driver  │  Driver          │
└──────────┴──────────────────┘
             ↓
┌──────────────────────────────┐
│  GPU Kernel Execution        │
│  (Launch, sync, error check) │
└──────────────────────────────┘
             ↓
┌──────────────────────────────┐
│  GPU Memory Management       │
│  (VRAM tier in semantic_mem) │
└──────────────────────────────┘
```

### 3.2 Key Invariants

1. **GPU Isolation**: Tasks cannot access each other's GPU memory
   - Enforced: Memory capability checks + GPU context isolation
   - Impact: Untrusted agents cannot spy on GPU state

2. **Quota Enforcement**: Task GPU time ≤ allocated quota
   - Enforced: Timeout monitoring + context preemption
   - Impact: Fair GPU sharing across tasks

3. **Deterministic Fallback**: If no GPU available, fall back to CPU (slower but works)
   - Enforced: CPU fallback routines always available
   - Impact: System works on non-GPU systems

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Query CT GPU quota |
| `capability_engine` | Internal | L0 | Verify ManageGpu capability |
| `semantic_memory` | Internal | L1 | Allocate GPU VRAM |

---

## 5. Public API Surface

```rust
/// GPU handle representing a GPU context
pub struct GpuHandle {
    pub device_id: u32,
    pub context: *mut CudaContext,  // Opaque CUDA context
}

/// Allocate GPU memory (from semantic_memory L1)
pub fn gpu_alloc(
    task_id: TaskId,
    size_bytes: usize,
) -> CsResult<GpuMemory>;

/// Execute kernel on GPU
pub fn gpu_exec(
    task_id: TaskId,
    kernel: &GpuKernel,
    args: GpuArgs,
) -> CsResult<()>;

/// Synchronize GPU execution
pub fn gpu_sync(task_id: TaskId) -> CsResult<()>;

/// Query GPU stats
pub fn gpu_stats(device_id: u32) -> CsResult<GpuStats>;

pub struct GpuStats {
    pub total_memory: u64,
    pub free_memory: u64,
    pub utilization: f32,
    pub temperature: f32,
}
```

---

## 6. Building & Testing

```bash
cargo build -p gpu_accelerator
cargo test -p gpu_accelerator
```

**Build Requirements:**
- CUDA Toolkit 12+ (for NVIDIA support) OR ROCm 5+ (for AMD)
- OR: CPU-only build (feature: cpu-fallback)

**Key Test Scenarios:**
1. GPU memory allocation — Success on available GPU
2. GPU kernel execution — Correct results
3. CPU fallback — Works without GPU (slower)
4. GPU quota enforcement — Task gets only assigned quota
5. Multi-GPU coordination — Round-robin scheduling

---

## 7. Design Decisions Log

### 7.1 "Abstracted vs. Direct GPU API?"

**Decision:** Abstracted unified GPU API (gpu_alloc, gpu_exec) instead of exposing CUDA/ROCm directly.

**Alternatives:**
1. CUDA-only — Depend directly on CUDA Driver API
2. Expose vendor APIs — Let applications choose CUDA vs. ROCm

**Rationale:**
- Abstraction allows vendor-independent code (NVIDIA/AMD agnostic)
- Easier to add new GPU vendors (Intel Arc, etc.)
- Simplifies CPU fallback (auto-translate GPU ops to CPU)
- Single testing/validation path regardless of vendor

**Date:** 2026-03-01
**Author:** Engineer 02

### 7.2 "GPU Preemption vs. Running to Completion?"

**Decision:** Preemptive GPU scheduling with context saves (if GPU supports).

**Alternatives:**
1. Non-preemptive — Tasks run to completion, even if timeout
2. Kill-on-timeout — Abruptly terminate overshooting task

**Rationale:**
- Preemption prevents starvation of other tasks
- Context save allows resumption after timeout
- Modern NVIDIA/AMD GPUs support preemption
- CPU fallback also preemptive (via OS scheduler)

**Date:** 2026-03-01
**Author:** Engineer 02

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `gpu_alloc` | O(log n) | Memory allocator overhead |
| `gpu_exec` | O(1) | Kernel submission (not execution) |
| `gpu_sync` | O(n) | n = kernel execution time |
| Device enumeration | O(d) | d = number of GPUs |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Forgetting gpu_sync()**
```rust
// ✗ WRONG: Not waiting for GPU completion
gpu_exec(task_id, &kernel, args)?;
// GPU is still running, but we read results immediately
let results = gpu_read(mem)?;  // RACE CONDITION

// ✓ RIGHT: Sync before reading
gpu_exec(task_id, &kernel, args)?;
gpu_sync(task_id)?;  // Wait for completion
let results = gpu_read(mem)?;
```

**Mistake 2: Exceeding GPU quota**
```rust
// ✗ WRONG: Kernel takes longer than quota
gpu_exec(task_id, &long_kernel, args)?;  // 100ms kernel
// Watchdog timeout fires after 50ms quota → killed

// ✓ RIGHT: Check kernel time estimate
let kernel_time = estimate_gpu_time(&kernel)?;
if kernel_time > gpu_quota {
    return Err(CsError::ResourceExceeded);
}
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Check GPU quota in CT config | Direct call |
| `semantic_memory` | Allocate VRAM from L1 tier | Direct call |
| `framework_adapters` (L2) | Submit GPU workloads during REASON | CSCI wrapper |

---

## 11. Future Roadmap

**Planned Improvements:**
- Dynamic GPU kernel compilation — Compile kernels at runtime for flexibility
- GPU memory sharing — Share read-only GPU memory between agents
- Heterogeneous scheduling — Mix GPU + CPU execution for hybrid workloads

**Technical Debt:**
- ROCm support incomplete (CUDA-only in v0.1)
- Multi-GPU synchronization overhead (O(n) broadcast on sync)

---

## 12. References

- **CUDA Driver API:** https://docs.nvidia.com/cuda/cuda-driver-api/
- **ROCm HIP:** https://rocmdocs.amd.com/
- **GPU Preemption:** https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html#compute-capability-6-0

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 02
