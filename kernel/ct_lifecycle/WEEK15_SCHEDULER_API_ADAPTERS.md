# Week 15: Scheduler API Adapters for Runtime Stream

**Phase:** 2 (Runtime Integration)
**Timeline:** Week 15 (Phase 2, Week 1)
**Objective:** Expose scheduler APIs for framework adapters; enable LangChain and Semantic Kernel to spawn CTs correctly.

---

## 1. Executive Summary

Week 15 bridges the L0 Microkernel Scheduler (Phase 1) with the Runtime Stream (Engineers 7, 8). We expose a public scheduler API surface that allows high-level framework adapters (LangChain, Semantic Kernel) to spawn Cognitive Tasks (CTs) and submit CT graphs without knowledge of internal scheduling complexity. This design maintains kernel abstraction boundaries while enabling framework-agnostic agent runtimes to leverage the 4D Cognitive Priority Scheduler's deadlock detection, crew-aware NUMA scheduling, and dual-resource co-scheduling.

**Key Deliverables:**
1. Public scheduler API with `ct_spawn_from_adapter`, `ct_graph_submit`, adapter context propagation
2. LangChain and Semantic Kernel adapter integration patterns
3. Performance baseline measurements (spawn latency, throughput, context switch overhead)
4. Comprehensive test suite for cross-adapter scenarios

---

## 2. Design Principles

**P6: Framework-Agnostic Agent Runtime**
The scheduler API abstracts away framework-specific details (chain steps, agent loops, tool calls) into a unified CT spawning interface. LangChain's `Chain.invoke()` and Semantic Kernel's `Kernel.RunAsync()` both compile to the same internal representation: CT task graphs with dependency annotations.

**P2: Cognitive Primitives as Kernel Abstractions**
CTs are the fundamental execution unit. Framework adapters never directly manipulate threads, queues, or scheduling decisions—they request CT creation with semantic hints (priority, resource affinity, streaming context), and the kernel's 4D scheduler handles the rest.

---

## 3. Public Scheduler API Surface

### 3.1 Core Data Structures

```rust
// Defined in kernel/ct_lifecycle/scheduler.rs
// no_std compatible; uses static allocation with bounded types

/// Adapter context: carries framework-specific metadata through CT lifecycle
#[derive(Clone, Copy)]
pub struct AdapterContext {
    /// Framework identifier: "langchain" | "semantic_kernel" | "custom"
    pub adapter_id: u8,
    /// Chain/workflow step index in the original framework DAG
    pub chain_step_idx: u16,
    /// User-provided opaque context (framework-specific state reference)
    pub user_context_ptr: *const u8,
    /// Priority hint: 0 (low) to 15 (critical); scheduler respects but may override
    pub priority_hint: u8,
    /// Resource affinity: CPU core preference (-1 for auto), GPU index, NUMA node
    pub affinity: ResourceAffinity,
}

/// Resource affinity for NUMA and GPU awareness
#[derive(Clone, Copy)]
pub struct ResourceAffinity {
    pub preferred_cpu_core: i8,    // -1 = auto
    pub preferred_gpu_idx: i8,     // -1 = none
    pub preferred_numa_node: u8,
}

/// CT spawn request from adapters
pub struct CtSpawnRequest {
    pub adapter_ctx: AdapterContext,
    pub entry_fn: *const dyn Fn(*const u8) -> CtResult,
    pub stack_size_bytes: usize,
    pub timeout_ms: u32,
}

/// Graph submission for multi-CT workflows
pub struct CtGraph {
    pub nodes: &'static [CtGraphNode],
    pub edges: &'static [CtGraphEdge],
    pub adapter_ctx: AdapterContext,
}

pub struct CtGraphNode {
    pub node_id: u16,
    pub entry_fn: *const dyn Fn(*const u8) -> CtResult,
    pub resource_hints: ResourceHints,
}

pub struct CtGraphEdge {
    pub from_node: u16,
    pub to_node: u16,
    pub dependency_type: DependencyType, // Data, Control, Fairness-Aware
}

pub enum DependencyType {
    Data,
    Control,
    FairnessAware, // Scheduler-managed; respects crew fairness
}

pub struct ResourceHints {
    pub min_cpu_affinity_strength: u8,  // 0 = soft hint, 255 = hard constraint
    pub gpu_compute_fraction: u8,       // 0-255; 0 = CPU-only, 255 = GPU-primary
    pub estimated_duration_ms: u32,
}

pub enum CtResult {
    Success(u64),      // Return value from entry_fn
    Error(CtErrorCode),
    Timeout,
}

pub enum CtErrorCode {
    InvalidAdapter,
    MemoryExhausted,
    DeadlockDetected,
    SchedulingFailed,
}
```

### 3.2 Scheduler API Functions

```rust
/// Spawn a single CT from adapter context.
/// Returns CT handle for tracking/awaiting.
///
/// Safety: Caller must ensure entry_fn is valid and thread-safe.
/// Adapter context is captured; no aliasing of user_context_ptr during execution.
pub extern "C" fn ct_spawn_from_adapter(
    req: &CtSpawnRequest,
) -> Result<CtHandle, CtErrorCode> {
    // Phase 2 implementation:
    // 1. Validate adapter_id; confirm adapter is registered
    // 2. Construct internal CtDescriptor with adapter_ctx embedded
    // 3. Run preemptive deadlock detection (Phase 1 subsystem)
    // 4. Assign to 4D scheduler queue based on priority_hint + crew fairness
    // 5. Enqueue for immediate or deferred launch
    // 6. Return CtHandle for tracking
    unimplemented!()
}

/// Submit a multi-node CT graph (e.g., LangChain chain steps as CT nodes).
/// Scheduler validates DAG, assigns node ranks, builds dependency order.
///
/// Returns graph execution context for streaming results.
pub extern "C" fn ct_graph_submit(
    graph: &CtGraph,
) -> Result<CtGraphExecutionContext, CtErrorCode> {
    // Phase 2 implementation:
    // 1. Validate graph structure (DAG property, no cycles)
    // 2. For each node: allocate CtDescriptor, embed FairnessAware edges
    // 3. Compute topological sort (node execution order)
    // 4. Build dependency graph in scheduler's internal representation
    // 5. Enqueue nodes respecting DependencyType semantics
    // 6. Return CtGraphExecutionContext for streaming/awaiting
    unimplemented!()
}

/// Register a framework adapter with the scheduler.
/// Must be called once per adapter before any ct_spawn_from_adapter.
pub extern "C" fn scheduler_register_adapter(
    adapter_id: u8,
    adapter_name: &[u8],  // utf-8 name
) -> Result<(), CtErrorCode> {
    // Phase 2 implementation:
    // 1. Validate adapter_id not already registered
    // 2. Store adapter metadata for future validation
    // 3. Initialize adapter-specific statistics counters
    unimplemented!()
}

/// Retrieve adapter context from a running CT.
/// Used by framework adapters to recover their state during CT execution.
pub extern "C" fn ct_current_adapter_context() -> Option<AdapterContext> {
    // Phase 2 implementation:
    // 1. Query scheduler TLS (thread-local storage) for current CtDescriptor
    // 2. Extract and return AdapterContext
    // 3. Return None if called outside CT execution or from non-adapter code
    unimplemented!()
}

/// Wait for CT completion with timeout. Blocks caller.
pub extern "C" fn ct_await(
    handle: CtHandle,
    timeout_ms: u32,
) -> Result<CtResult, CtErrorCode> {
    unimplemented!()
}

/// Non-blocking poll for CT completion status.
pub extern "C" fn ct_poll(
    handle: CtHandle,
) -> Option<CtResult> {
    unimplemented!()
}

/// Retrieve performance metrics for a completed CT.
pub extern "C" fn ct_get_metrics(
    handle: CtHandle,
) -> Option<CtMetrics> {
    unimplemented!()
}

pub struct CtMetrics {
    pub wall_time_us: u64,
    pub cpu_time_us: u64,
    pub gpu_time_us: u64,
    pub spawn_to_start_us: u64,
    pub context_switches: u32,
    pub numa_migrations: u16,
}
```

---

## 4. Adapter Context Propagation

### 4.1 Propagation Mechanics

When an adapter calls `ct_spawn_from_adapter()`, the kernel embeds the `AdapterContext` into the internal `CtDescriptor`. During CT execution:

1. **TLS Injection**: The scheduler inserts the `AdapterContext` into task-local storage (TLS).
2. **Framework Callback**: The adapter's entry function calls `ct_current_adapter_context()` to recover its state.
3. **Nested CTs**: If a CT spawns child CTs (e.g., LangChain tool calls within a step), child CTs inherit the parent's `adapter_ctx` with modified `chain_step_idx`.

```rust
/// Example: LangChain adapter's entry function
extern "C" fn langchain_step_entry(user_ctx_ptr: *const u8) -> CtResult {
    unsafe {
        let adapter_ctx = ct_current_adapter_context().unwrap();
        let step_state: &LangChainStepState = std::mem::transmute(user_ctx_ptr);

        // Execute step logic with access to adapter context
        let priority = adapter_ctx.priority_hint;
        match step_state.invoke(priority) {
            Ok(result) => CtResult::Success(result as u64),
            Err(e) => CtResult::Error(CtErrorCode::SchedulingFailed),
        }
    }
}
```

### 4.2 Adapter Registration and Validation

Adapters must register with the scheduler before spawning CTs:

```rust
// Phase 2: Initialization code in LangChain adapter
pub fn initialize_langchain_adapter() -> Result<(), CtErrorCode> {
    const LANGCHAIN_ADAPTER_ID: u8 = 1;
    scheduler_register_adapter(LANGCHAIN_ADAPTER_ID, b"langchain")?;
    Ok(())
}
```

---

## 5. Framework Adapter Integration Patterns

### 5.1 LangChain Adapter (Chain Steps to CT Graph)

LangChain `Chain.invoke()` with a 3-step chain (Input → Tool Call → Output) maps to:

```rust
pub struct LangChainAdapterContext {
    pub chain_id: u32,
    pub user_priority: u8,
}

/// Converts LangChain chain to CT graph
pub fn langchain_chain_to_ct_graph(
    chain: &LangChainChain,
    user_priority: u8,
) -> CtGraph {
    let nodes = [
        CtGraphNode {
            node_id: 0,
            entry_fn: &langchain_step_input as *const _,
            resource_hints: ResourceHints {
                min_cpu_affinity_strength: 64,
                gpu_compute_fraction: 0,
                estimated_duration_ms: 10,
            },
        },
        CtGraphNode {
            node_id: 1,
            entry_fn: &langchain_step_tool_call as *const _,
            resource_hints: ResourceHints {
                min_cpu_affinity_strength: 128,
                gpu_compute_fraction: 200, // Suggest GPU for ML inference
                estimated_duration_ms: 100,
            },
        },
        CtGraphNode {
            node_id: 2,
            entry_fn: &langchain_step_output as *const _,
            resource_hints: ResourceHints {
                min_cpu_affinity_strength: 64,
                gpu_compute_fraction: 0,
                estimated_duration_ms: 5,
            },
        },
    ];

    let edges = [
        CtGraphEdge {
            from_node: 0,
            to_node: 1,
            dependency_type: DependencyType::Data,
        },
        CtGraphEdge {
            from_node: 1,
            to_node: 2,
            dependency_type: DependencyType::Data,
        },
    ];

    CtGraph {
        nodes: &nodes,
        edges: &edges,
        adapter_ctx: AdapterContext {
            adapter_id: 1,                      // LANGCHAIN_ADAPTER_ID
            chain_step_idx: 0,
            user_context_ptr: &chain.state as *const _ as *const u8,
            priority_hint: user_priority,
            affinity: ResourceAffinity {
                preferred_cpu_core: -1,
                preferred_gpu_idx: 0,
                preferred_numa_node: 0,
            },
        },
    }
}
```

### 5.2 Semantic Kernel Adapter (Semantic Functions to CT Graph)

Semantic Kernel's `Kernel.RunAsync()` with a function pipeline:

```rust
pub struct SemanticKernelAdapterContext {
    pub kernel_id: u32,
    pub skill_name: &'static str,
}

/// Converts Semantic Kernel skill pipeline to CT graph
pub fn semantic_kernel_skill_to_ct_graph(
    kernel: &SemanticKernel,
    skill_pipeline: &[SemanticFunction],
) -> CtGraph {
    let mut nodes = heapless::Vec::<CtGraphNode, 32>::new();
    let mut edges = heapless::Vec::<CtGraphEdge, 64>::new();

    for (idx, func) in skill_pipeline.iter().enumerate() {
        nodes.push(CtGraphNode {
            node_id: idx as u16,
            entry_fn: &semantic_kernel_function_wrapper as *const _,
            resource_hints: ResourceHints {
                min_cpu_affinity_strength: 80,
                gpu_compute_fraction: if func.requires_gpu { 255 } else { 0 },
                estimated_duration_ms: func.avg_duration_ms,
            },
        }).unwrap();

        if idx > 0 {
            edges.push(CtGraphEdge {
                from_node: (idx - 1) as u16,
                to_node: idx as u16,
                dependency_type: DependencyType::FairnessAware,
            }).unwrap();
        }
    }

    CtGraph {
        nodes: nodes.as_slice(),
        edges: edges.as_slice(),
        adapter_ctx: AdapterContext {
            adapter_id: 2,                      // SEMANTIC_KERNEL_ADAPTER_ID
            chain_step_idx: 0,
            user_context_ptr: kernel as *const _ as *const u8,
            priority_hint: 8,                   // Medium priority
            affinity: ResourceAffinity {
                preferred_cpu_core: -1,
                preferred_gpu_idx: if skill_pipeline.iter().any(|f| f.requires_gpu) { 0 } else { -1 },
                preferred_numa_node: 0,
            },
        },
    }
}
```

---

## 6. CT Spawn from Adapter Context

### 6.1 Single CT Spawn Flow

```rust
/// Example: LangChain adapter spawning a tool call CT
pub fn langchain_spawn_tool_call(
    tool_name: &str,
    input: &str,
    chain_step_idx: u16,
) -> Result<CtHandle, CtErrorCode> {
    let adapter_ctx = AdapterContext {
        adapter_id: 1,
        chain_step_idx,
        user_context_ptr: input.as_ptr(),
        priority_hint: 10,
        affinity: ResourceAffinity {
            preferred_cpu_core: -1,
            preferred_gpu_idx: -1,
            preferred_numa_node: 0,
        },
    };

    let req = CtSpawnRequest {
        adapter_ctx,
        entry_fn: &langchain_tool_wrapper as *const _,
        stack_size_bytes: 8192,
        timeout_ms: 5000,
    };

    ct_spawn_from_adapter(&req)
}

extern "C" fn langchain_tool_wrapper(user_ctx_ptr: *const u8) -> CtResult {
    let adapter_ctx = ct_current_adapter_context().expect("adapter context");
    let tool_input = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(user_ctx_ptr, 256))
            .unwrap_or("")
    };

    // Invoke tool with scheduler awareness
    match invoke_tool(tool_input) {
        Ok(output) => CtResult::Success(output as u64),
        Err(_) => CtResult::Error(CtErrorCode::SchedulingFailed),
    }
}
```

### 6.2 Nested CT Spawning (Parent-Child Relationships)

When a CT spawns child CTs (e.g., parallel tool calls), the kernel preserves crew-aware fairness:

```rust
extern "C" fn langchain_parallel_tools(user_ctx_ptr: *const u8) -> CtResult {
    let parent_ctx = ct_current_adapter_context().unwrap();

    // Spawn 4 child CTs for parallel tool invocations
    let mut child_handles = heapless::Vec::<CtHandle, 8>::new();

    for tool_idx in 0..4 {
        let child_ctx = AdapterContext {
            adapter_id: parent_ctx.adapter_id,
            chain_step_idx: parent_ctx.chain_step_idx,
            user_context_ptr: parent_ctx.user_context_ptr, // Shared state
            priority_hint: parent_ctx.priority_hint,
            affinity: ResourceAffinity {
                preferred_cpu_core: (tool_idx % NUM_CORES) as i8,
                ..parent_ctx.affinity
            },
        };

        let req = CtSpawnRequest {
            adapter_ctx: child_ctx,
            entry_fn: &langchain_tool_wrapper as *const _,
            stack_size_bytes: 4096,
            timeout_ms: 3000,
        };

        if let Ok(handle) = ct_spawn_from_adapter(&req) {
            child_handles.push(handle).unwrap();
        }
    }

    // Await all children with crew fairness guarantees
    for handle in child_handles.iter() {
        let _ = ct_await(*handle, 5000);
    }

    CtResult::Success(4u64) // Spawned 4 children
}
```

---

## 7. Performance Baseline Measurements

### 7.1 Benchmarking Methodology

**Test Environment:** 16-core x86-64, 2x NVIDIA A100 GPUs, 4x NUMA nodes
**Workloads:**
- Single CT spawn (empty loop)
- 3-node CT graph (LangChain-like)
- 8-node CT graph (Semantic Kernel-like)
- Nested spawning (1 parent + 16 children)

### 7.2 Baseline Metrics (Phase 2 Target)

| Metric | Single CT | 3-Node Graph | 8-Node Graph | Nested (1+16) |
|--------|-----------|--------------|--------------|---------------|
| **Spawn Latency (µs)** | 45-65 | 120-150 | 250-300 | 800-950 |
| **Throughput (CTs/ms)** | 18-22 | 6-8 | 3-4 | 1-1.5 |
| **Context Switch Overhead (µs)** | 12-18 | 15-22 | 18-28 | 25-40 |
| **Memory per CT (bytes)** | 512-1024 | - | - | 512-1024 |
| **NUMA Migration Rate (%)** | <2 | <3 | <5 | <8 |

**Acceptance Criteria:**
- Spawn latency < 100µs for single CTs
- Graph validation < 200µs for 8-node graphs
- Context switch overhead < 50µs in contention scenarios
- NUMA migrations < 10% in multi-node scenarios

### 7.3 Profiling Code

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;

    #[test]
    fn bench_single_ct_spawn() {
        let mut total_us = 0u64;
        const ITERATIONS: usize = 1000;

        for _ in 0..ITERATIONS {
            let start = rdtsc();
            let req = CtSpawnRequest {
                adapter_ctx: AdapterContext {
                    adapter_id: 1,
                    chain_step_idx: 0,
                    user_context_ptr: core::ptr::null(),
                    priority_hint: 8,
                    affinity: ResourceAffinity {
                        preferred_cpu_core: -1,
                        preferred_gpu_idx: -1,
                        preferred_numa_node: 0,
                    },
                },
                entry_fn: &empty_ct_fn as *const _,
                stack_size_bytes: 4096,
                timeout_ms: 1000,
            };
            let _ = ct_spawn_from_adapter(&req);
            let end = rdtsc();
            total_us += (end - start) / CPU_FREQ_GHZ;
        }

        let avg_us = total_us / ITERATIONS as u64;
        println!("Single CT spawn: {} µs (avg)", avg_us);
        assert!(avg_us < 100, "Spawn latency exceeded 100µs");
    }

    extern "C" fn empty_ct_fn(_ctx: *const u8) -> CtResult {
        CtResult::Success(0)
    }
}
```

---

## 8. Test Suite Strategy

### 8.1 Core Test Categories

1. **Adapter Registration & Validation**
   - Register multiple adapters; verify isolation
   - Reject duplicate adapter IDs
   - Recover from registration failures

2. **Single CT Spawning**
   - Spawn with various priority hints
   - Verify adapter context recovery in entry function
   - Timeout behavior on long-running CTs

3. **CT Graph Submission**
   - DAG validation (reject cycles)
   - Topological sort correctness
   - Dependency propagation (Data, Control, FairnessAware)

4. **Nested Spawning**
   - Parent-child context inheritance
   - Fairness constraints on child pools
   - Deadlock detection with nested graphs

5. **Cross-Adapter Scenarios**
   - LangChain + Semantic Kernel interleaving
   - Resource contention and scheduling decisions
   - Metrics isolation per adapter

6. **Resource Affinity**
   - NUMA-aware scheduling
   - GPU assignment correctness
   - CPU core pinning enforcement

### 8.2 Example Test: LangChain-Semantic Kernel Interleaving

```rust
#[test]
fn test_interleaved_adapters() {
    // Initialize both adapters
    scheduler_register_adapter(1, b"langchain").unwrap();
    scheduler_register_adapter(2, b"semantic_kernel").unwrap();

    // Spawn LangChain CT
    let lc_req = CtSpawnRequest {
        adapter_ctx: AdapterContext {
            adapter_id: 1,
            chain_step_idx: 0,
            user_context_ptr: core::ptr::null(),
            priority_hint: 8,
            affinity: ResourceAffinity {
                preferred_cpu_core: 0,
                preferred_gpu_idx: 0,
                preferred_numa_node: 0,
            },
        },
        entry_fn: &langchain_step_entry as *const _,
        stack_size_bytes: 8192,
        timeout_ms: 2000,
    };
    let lc_handle = ct_spawn_from_adapter(&lc_req).unwrap();

    // Spawn Semantic Kernel CT with different affinity
    let sk_req = CtSpawnRequest {
        adapter_ctx: AdapterContext {
            adapter_id: 2,
            chain_step_idx: 0,
            user_context_ptr: core::ptr::null(),
            priority_hint: 9,
            affinity: ResourceAffinity {
                preferred_cpu_core: 4,
                preferred_gpu_idx: 1,
                preferred_numa_node: 1,
            },
        },
        entry_fn: &semantic_kernel_function_wrapper as *const _,
        stack_size_bytes: 8192,
        timeout_ms: 2000,
    };
    let sk_handle = ct_spawn_from_adapter(&sk_req).unwrap();

    // Await both; verify no deadlock
    let lc_result = ct_await(lc_handle, 3000).unwrap();
    let sk_result = ct_await(sk_handle, 3000).unwrap();

    // Retrieve and validate metrics
    if let Some(lc_metrics) = ct_get_metrics(lc_handle) {
        assert!(lc_metrics.wall_time_us < 2_000_000);
        assert!(lc_metrics.numa_migrations < 5);
    }
    if let Some(sk_metrics) = ct_get_metrics(sk_handle) {
        assert!(sk_metrics.wall_time_us < 2_000_000);
        assert!(sk_metrics.numa_migrations < 5);
    }
}
```

---

## 9. Integration with Phase 1 Subsystems

### 9.1 Deadlock Detection Integration

Each `ct_spawn_from_adapter()` triggers Phase 1's preemptive deadlock detector:

```rust
// Internal: called from ct_spawn_from_adapter
fn preemptive_deadlock_check(
    adapter_ctx: &AdapterContext,
    resource_hints: &ResourceHints,
) -> Result<(), CtErrorCode> {
    // Phase 1 subsystem query
    let held_resources = query_crew_held_resources(adapter_ctx.adapter_id)?;
    let requested_resources = extract_resource_needs(resource_hints);

    if deadlock_detector::would_form_cycle(&held_resources, &requested_resources) {
        return Err(CtErrorCode::DeadlockDetected);
    }
    Ok(())
}
```

### 9.2 4D Scheduler Queue Assignment

Adapter priority hints inform but do not override crew-aware NUMA scheduling:

```rust
fn assign_to_scheduler_queue(
    adapter_ctx: &AdapterContext,
    resource_hints: &ResourceHints,
) -> QueueAssignment {
    let base_priority = adapt_priority_to_crew(
        adapter_ctx.priority_hint,
        adapter_ctx.adapter_id,
    );

    let numa_queue = if resource_hints.gpu_compute_fraction > 128 {
        assign_gpu_affinity_queue(adapter_ctx.affinity.preferred_gpu_idx)
    } else {
        assign_numa_queue(adapter_ctx.affinity.preferred_numa_node)
    };

    QueueAssignment {
        priority: base_priority,
        queue: numa_queue,
        cpu_core_pinning: adapter_ctx.affinity.preferred_cpu_core >= 0,
    }
}
```

---

## 10. Success Criteria (Phase 2 Exit)

✓ **API Documentation**: Public scheduler API fully documented with examples
✓ **LangChain Integration**: 3-step chain example completes < 200ms spawn-to-completion
✓ **Semantic Kernel Integration**: 4-skill pipeline executes with correct dependency order
✓ **Adapter Context Propagation**: Framework state recovered in CT entry functions
✓ **Performance**: Single CT spawn < 100µs; 8-node graph < 300µs
✓ **Test Coverage**: 12+ integration tests; zero race conditions detected
✓ **Deadlock Safety**: No deadlock scenarios in nested spawning tests

---

## 11. Future Work (Phase 3+)

- **Streaming Results**: Async callback interface for CT result streaming (Engineers 7)
- **Adaptive Scheduling**: Framework adapters provide dynamic priority adjustment based on chain state
- **Custom Adapter SDK**: Template for third-party framework integrations
- **Metrics Export**: Prometheus-compatible scheduler metrics endpoint

