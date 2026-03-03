# Semantic Memory Manager

> **Crate:** [`semantic_memory`](Cargo.toml)
> **Stream:** 2 — Kernel Services
> **Layer:** L1 (Kernel Services)
> **Owner:** Engineer 02
> **Status:** Active

---

## 1. Purpose & Scope

Manages the three-tier semantic memory hierarchy (L1/L2/L3) for agents and CTs. Unlike traditional virtual memory which is byte-addressed, semantic memory is instruction-addressed and integrates with GPU acceleration. Handles memory allocation, NUMA-aware placement, and garbage collection while enforcing capability-based access control.

**Key Responsibilities:**
- L1 fast memory (SRAM, GPU VRAM) allocation and management
- L2 standard memory (DRAM) management
- L3 slow memory (NVMe, S3) management
- NUMA-aware placement for multi-socket systems
- Garbage collection and eviction policies
- Capability-based memory access enforcement

**In Scope:**
- Memory tier allocation algorithms
- NUMA topology awareness
- GC policy (mark-and-sweep, generational, etc.)
- Memory pressure and OOM handling

**Out of Scope:**
- GPU compute scheduling (handled by gpu_accelerator)
- Virtual memory translation (handled by hardware MMU)
- File system semantics (handled by semantic_fs)

---

## 2. Engineering Plan Reference

**Relevant Sections:**
- Section 2.2: SemanticMemory domain entity
- Section 4.5: Memory Management Architecture

**Domain Model Entities Involved:**
- **SemanticMemory** — Three-tier memory management
- **CognitiveTask** — Consumers of memory
- **Agent** — Memory quota owners

---

## 3. Architecture & Design

### 3.1 High-Level Architecture

```
┌────────────────────────────────┐
│  Memory Allocation API         │
│  alloc(), free(), gc()         │
└────────────────────────────────┘
             ↓
┌────────────────────────────────┐
│  Three-Tier Memory Manager     │
├────────┬────────────┬──────────┤
│  L1    │     L2    │    L3    │
│ (Fast) │  (Normal) │  (Slow)  │
└────────┴────────────┴──────────┘
             ↓
┌────────────────────────────────┐
│  NUMA & GPU Placement          │
│  (Topology-aware allocation)   │
└────────────────────────────────┘
             ↓
    ┌──────────────────────────┐
    │ Garbage Collector        │
    │ (Mark-and-sweep, GC/GC0) │
    └──────────────────────────┘
```

### 3.2 Key Invariants

1. **Quota Enforcement**: CT memory usage ≤ allocated quota
   - Enforced: Allocation checks + periodic monitoring
   - Impact: Prevents memory exhaustion attacks

2. **Capability-Based Access**: Only agents with memory capability can allocate
   - Enforced: Capability check on every alloc
   - Impact: Isolates memory between untrusted agents

3. **Tier Awareness**: Applications don't need to know which tier they're on (transparent)
   - Enforced: Semantic memory API (not address-based)
   - Impact: Flexible memory management without application changes

---

## 4. Dependencies

| Crate | Type | Layer | Why |
|-------|------|-------|-----|
| `ct_lifecycle` | Internal | L0 | Query CT memory quota |
| `capability_engine` | Internal | L0 | Verify memory capability |
| `gpu_accelerator` | Internal | L1 | Coordinate GPU VRAM allocation |

---

## 5. Public API Surface

```rust
/// Allocate memory from semantic memory
pub fn mem_alloc(
    task_id: TaskId,
    size: usize,
    tier_preference: MemoryTier,
) -> CsResult<MemoryHandle>;

/// Free allocated memory
pub fn mem_free(handle: MemoryHandle) -> CsResult<()>;

/// Trigger garbage collection
pub fn mem_gc(strategy: GCStrategy) -> CsResult<()>;

/// Query memory usage
pub fn mem_usage(task_id: TaskId) -> CsResult<MemoryStats>;

pub enum MemoryTier {
    L1,  // Fast (GPU VRAM, L3 cache)
    L2,  // Normal (DRAM)
    L3,  // Slow (NVMe, S3)
}

pub struct MemoryStats {
    pub used: usize,
    pub quota: usize,
    pub tier_distribution: HashMap<MemoryTier, usize>,
}
```

---

## 6. Building & Testing

```bash
cargo build -p semantic_memory
cargo test -p semantic_memory
```

**Key Test Scenarios:**
1. Allocation under quota — Success
2. Allocation over quota — Failure (OOM)
3. Garbage collection — Memory reclamation
4. NUMA placement — Correct socket assignment
5. Capability checks — Unauthorized alloc fails

---

## 7. Design Decisions Log

### 7.1 "Three-Tier Memory vs. Traditional Paging?"

**Decision:** Three explicit tiers (L1/L2/L3) instead of single virtual memory space.

**Alternatives:**
1. Traditional paging — Single VA space, kernel manages page placement
2. Application-managed memory — Apps choose tier explicitly

**Rationale:**
- Explicit tiers allow kernel to optimize placement without complex logic
- Applications can hint tier preference (e.g., LLM activations → L1)
- Simpler than page fault handling for multi-tier systems
- Matches modern GPU memory hierarchies

**Date:** 2026-03-01
**Author:** Engineer 02

### 7.2 "Mark-and-Sweep vs. Generational GC?"

**Decision:** Mark-and-sweep for L0, generational for L1/L2.

**Alternatives:**
1. Generational everywhere — Better for high-allocation workloads
2. Reference counting — Immediate reclamation, no GC pauses

**Rationale:**
- Mark-and-sweep simpler to implement and reason about
- Generational targets long-lived agent objects efficiently
- Reference counting overhead (atomics) not acceptable in L0

**Date:** 2026-03-01
**Author:** Engineer 02

---

## 8. Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `mem_alloc` | O(log n) | n = free blocks |
| `mem_free` | O(log n) | Coalescing free blocks |
| `mem_gc` | O(n) | n = live objects |
| NUMA placement | O(1) | Topology lookup |

---

## 9. Common Pitfalls & Troubleshooting

**Mistake 1: Allocating without checking quota**
```rust
// ✗ WRONG: Assumes infinite memory
let handle = mem_alloc(task_id, 1_000_000_000, MemoryTier::L2)?;

// ✓ RIGHT: Check stats first
let stats = mem_usage(task_id)?;
if stats.used + 1_000_000 > stats.quota {
    return Err(CsError::OutOfMemory);
}
let handle = mem_alloc(task_id, 1_000_000, MemoryTier::L2)?;
```

**Mistake 2: Not freeing memory**
```rust
// ✗ WRONG: Memory leak
let handle = mem_alloc(task_id, 1024, MemoryTier::L2)?;
// Forgot to free!

// ✓ RIGHT: Explicit free or RAII
let handle = mem_alloc(task_id, 1024, MemoryTier::L2)?;
defer!(mem_free(handle));  // Or use drop guard
```

---

## 10. Integration Points

| Module | Integration | Protocol |
|--------|-----------|----------|
| `ct_lifecycle` | Allocate per-CT memory pools | Direct call |
| `gpu_accelerator` | Coordinate GPU VRAM allocation | CSCI wrapper |
| `framework_adapters` (L2) | Request memory for reasoning phases | IPC |

---

## 11. Future Roadmap

**Planned Improvements:**
- Predictive prefetching — Anticipate memory access patterns
- Memory sharing — Share read-only memory between agents
- Swap-to-disk — Overflow to NVMe/S3 on memory pressure

---

## 12. References

- **Three-Tier Memory:** https://arxiv.org/pdf/2109.08609.pdf (GPU memory hierarchies)
- **NUMA Topology:** https://www.kernel.org/doc/html/latest/vm/numa.html

---

**README Version:** 1.0
**Last Updated:** 2026-03-01
**Owner:** Engineer 02
