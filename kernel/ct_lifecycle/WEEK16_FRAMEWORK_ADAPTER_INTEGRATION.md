# XKernal Cognitive Substrate OS — Week 16: Framework Adapter Integration
## Technical Design Document

**Date:** March 2, 2026
**Phase:** Phase 2 (Scheduler & Framework Integration)
**Layer:** L0 Microkernel (Rust, no_std)
**Design Principles:** P6 (Framework-Agnostic), P1 (Agent-First)

---

## 1. Executive Summary

Week 16 completes full framework adapter integration for LangChain 4 and Semantic Kernel within the XKernal CT Lifecycle scheduler. This document specifies:

- **LangChain Adapter:** Integration of 4 chain types (SimpleChain, ReActChain, MapReduceChain, RouterChain) into CT execution primitives
- **Semantic Kernel Adapter:** Plugin/planner/memory bridging to CT task graphs
- **End-to-End Execution:** Real agent workflows from both frameworks running on XKernal scheduler
- **Integration Test Suite:** 20+ verification scenarios covering tool integration, memory consistency, and error handling

By week end, external agents (LangChain, Semantic Kernel) will execute at native scheduler performance with full CT-aware deadlock detection, NUMA affinity, and memory isolation.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  LangChain Agent    │   Semantic Kernel Agent       │
│  (External Process) │   (External Process)          │
└──────────┬──────────┴──────────┬─────────────────────┘
           │                     │
        [Adapter FFI]         [Adapter FFI]
           │                     │
┌──────────▼─────────────────────▼──────────────────────┐
│         CT Adapter Layer (Scheduler Bridge)           │
│  - Chain→TaskGraph conversion                          │
│  - Plugin→ComputeNode translation                      │
│  - Memory binding & isolation                          │
│  - Tool dispatch & result aggregation                  │
└──────────┬──────────────────────┬──────────────────────┘
           │                      │
┌──────────▼──────────────────────▼──────────────────────┐
│      CT Lifecycle Scheduler (Week 15 API)             │
│  - ct_spawn_from_adapter()                            │
│  - ct_graph_submit()                                  │
│  - Deadlock detection, NUMA affinity                  │
│  - GPU co-scheduling, memory accounting               │
└──────────┬──────────────────────┬──────────────────────┘
           │                      │
┌──────────▼──────────────────────▼──────────────────────┐
│      4D Scheduler + Runtime (Phase 1 Validated)       │
│  - Task execution, context switching                  │
│  - NUMA-aware memory management                       │
└─────────────────────────────────────────────────────────┘
```

---

## 3. LangChain Adapter Integration

### 3.1 SimpleChain Mapping

**Concept:** Linear sequence of LLM + parser → CT sequential task graph.

```rust
// ct_lifecycle/adapters/langchain_bridge.rs (no_std)

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;

/// Represents a LangChain SimpleChain step
#[derive(Clone)]
pub struct ChainStep {
    pub name: String,
    pub prompt_template: String,
    pub tool_name: Option<String>,
}

/// Converts LangChain SimpleChain to CT task graph
pub struct SimpleChainAdapter;

impl SimpleChainAdapter {
    /// Convert chain steps to ComputeNode tasks
    /// Returns (node_ids, edges) representing task dependencies
    pub fn convert_chain(
        chain_steps: &[ChainStep],
        agent_id: u64,
    ) -> Result<(Vec<u64>, Vec<(u64, u64)>), AdapterError> {
        let mut node_ids = Vec::new();
        let mut edges = Vec::new();

        for (idx, step) in chain_steps.iter().enumerate() {
            // Each step becomes a ComputeNode task
            let node_id = ct_spawn_from_adapter(
                agent_id,
                step.name.as_str(),
                step.prompt_template.as_bytes(),
            )?;

            node_ids.push(node_id);

            // Create dependency edge from previous step
            if idx > 0 {
                edges.push((node_ids[idx - 1], node_id));
            }
        }

        Ok((node_ids, edges))
    }
}
```

**Integration Flow:**
1. Agent calls `agent.run(input)` in LangChain
2. Adapter intercepts chain, extracts steps
3. For each step: create ComputeNode via `ct_spawn_from_adapter()`
4. Wire sequential dependencies via edges
5. Submit task graph via `ct_graph_submit()`
6. Scheduler executes with deadlock detection & NUMA affinity
7. Results aggregated back to LangChain chain

---

### 3.2 ReActChain Mapping

**Concept:** Reasoning + Acting loop → CT cyclic task graph with feedback paths.

```rust
/// ReActChain adapter for reasoning loops
pub struct ReActChainAdapter;

impl ReActChainAdapter {
    /// ReAct loop: Think → Act → Observe → Repeat
    pub fn convert_react_loop(
        initial_input: &str,
        max_iterations: usize,
        agent_id: u64,
    ) -> Result<u64, AdapterError> {
        // Create master task that coordinates the loop
        let loop_task_id = ct_spawn_from_adapter(
            agent_id,
            "react_coordinator",
            initial_input.as_bytes(),
        )?;

        let mut prev_task_id = loop_task_id;

        for iteration in 0..max_iterations {
            // Thought step
            let thought_task = ct_spawn_from_adapter(
                agent_id,
                &format!("react_thought_{}", iteration),
                b"",
            )?;
            ct_schedule_edge(prev_task_id, thought_task)?;

            // Action step
            let action_task = ct_spawn_from_adapter(
                agent_id,
                &format!("react_action_{}", iteration),
                b"",
            )?;
            ct_schedule_edge(thought_task, action_task)?;

            // Observation step
            let obs_task = ct_spawn_from_adapter(
                agent_id,
                &format!("react_observe_{}", iteration),
                b"",
            )?;
            ct_schedule_edge(action_task, obs_task)?;

            prev_task_id = obs_task;
        }

        Ok(loop_task_id)
    }
}

/// External C FFI for scheduler edge creation
extern "C" {
    fn ct_schedule_edge(from_id: u64, to_id: u64) -> i32;
}
```

**Key Features:**
- Coordinator task governs loop termination logic
- Thought/Action/Observe stages become separate ComputeNodes
- Feedback path (Observe→Thought) creates cycle detection (scheduler deadlock detector validates)
- Scheduler enforces memory isolation between iterations

---

### 3.3 MapReduceChain Mapping

**Concept:** Parallel map phase → reduce aggregation → CT DAG with fan-out/fan-in.

```rust
/// MapReduceChain parallel execution
pub struct MapReduceChainAdapter;

impl MapReduceChainAdapter {
    /// Convert map-reduce to parallel task DAG
    pub fn convert_map_reduce(
        input_chunks: &[String],  // Partitioned input
        mapper_fn: &str,          // LLM prompt for map phase
        reducer_fn: &str,         // LLM prompt for reduce
        agent_id: u64,
    ) -> Result<u64, AdapterError> {
        // Create map coordinator
        let map_coordinator = ct_spawn_from_adapter(
            agent_id,
            "mapreduce_map_coordinator",
            b"",
        )?;

        let mut map_task_ids = Vec::new();

        // Spawn parallel map tasks
        for (idx, chunk) in input_chunks.iter().enumerate() {
            let map_task = ct_spawn_from_adapter(
                agent_id,
                &format!("map_task_{}", idx),
                chunk.as_bytes(),
            )?;

            // Declare as parallel child of coordinator
            // Scheduler will parallelize without dependency
            map_task_ids.push(map_task);
        }

        // Reduce phase: join all map outputs
        let reduce_task = ct_spawn_from_adapter(
            agent_id,
            "mapreduce_reduce",
            reducer_fn.as_bytes(),
        )?;

        // Create fan-in: all map tasks must complete before reduce
        for map_id in &map_task_ids {
            ct_schedule_edge(*map_id, reduce_task)?;
        }

        Ok(reduce_task)
    }
}
```

**Scheduler Interaction:**
- Map coordinator declares parallelism constraint
- Scheduler places map tasks on separate NUMA nodes when possible
- Reduce task waits for all map outputs (implicit synchronization point)
- Memory accounting accumulates across map tasks, checked before reduce allocation

---

### 3.4 RouterChain Mapping

**Concept:** Conditional branching (if-then-else) → CT task graph with choice nodes.

```rust
/// RouterChain conditional execution
pub struct RouterChainAdapter;

#[derive(Clone)]
pub struct RoutingDecision {
    pub condition: String,
    pub true_chain: Vec<ChainStep>,
    pub false_chain: Vec<ChainStep>,
}

impl RouterChainAdapter {
    /// Convert conditional chain routing to branching task DAG
    pub fn convert_router_chain(
        routing_decisions: &[RoutingDecision],
        agent_id: u64,
    ) -> Result<u64, AdapterError> {
        let mut decision_nodes = Vec::new();

        for (idx, route) in routing_decisions.iter().enumerate() {
            // Decision node evaluates condition
            let decision_node = ct_spawn_from_adapter(
                agent_id,
                &format!("router_decision_{}", idx),
                route.condition.as_bytes(),
            )?;
            decision_nodes.push(decision_node);

            // True branch
            let true_root = ct_spawn_from_adapter(
                agent_id,
                &format!("router_true_branch_{}", idx),
                b"",
            )?;
            ct_schedule_edge(decision_node, true_root)?;

            // Chain true branch steps
            let mut prev = true_root;
            for step in &route.true_chain {
                let step_node = ct_spawn_from_adapter(
                    agent_id,
                    &step.name,
                    step.prompt_template.as_bytes(),
                )?;
                ct_schedule_edge(prev, step_node)?;
                prev = step_node;
            }

            // False branch (similarly)
            let false_root = ct_spawn_from_adapter(
                agent_id,
                &format!("router_false_branch_{}", idx),
                b"",
            )?;
            ct_schedule_edge(decision_node, false_root)?;
            // ... chain false branch steps ...
        }

        Ok(decision_nodes[0])
    }
}
```

**Scheduler Guarantees:**
- Decision node executes deterministically (no race conditions)
- Only one branch path executes (mutual exclusion enforced)
- Scheduler prunes unexecuted branch tasks from memory accounting

---

## 4. Semantic Kernel Adapter Integration

### 4.1 Plugin System Mapping

**Concept:** SK Plugins (native functions + LLM prompts) → CT ComputeNode instances.

```rust
// ct_lifecycle/adapters/semantic_kernel_bridge.rs

/// Semantic Kernel plugin representation
#[derive(Clone)]
pub struct SKPlugin {
    pub name: String,
    pub functions: Vec<SKFunction>,
}

#[derive(Clone)]
pub struct SKFunction {
    pub name: String,
    pub function_type: FunctionType,  // Native or LLM
    pub input_spec: String,
    pub output_spec: String,
}

#[derive(Clone, Copy)]
pub enum FunctionType {
    Native,  // Rust function
    Llm,     // Prompt-based
}

/// Convert SK plugins to ComputeNodes
pub struct PluginAdapter;

impl PluginAdapter {
    pub fn register_plugin(
        plugin: &SKPlugin,
        agent_id: u64,
    ) -> Result<Vec<u64>, AdapterError> {
        let mut function_ids = Vec::new();

        for func in &plugin.functions {
            let node_id = ct_spawn_from_adapter(
                agent_id,
                &format!("{}_{}", plugin.name, func.name),
                func.input_spec.as_bytes(),
            )?;

            function_ids.push(node_id);
        }

        Ok(function_ids)
    }
}
```

---

### 4.2 Planner Integration

**Concept:** SK Planners (sequential, stepwise) → CT task graph generation.

```rust
/// Semantic Kernel planner execution
pub struct PlannerAdapter;

/// SK Step-by-Step planner output
#[derive(Clone)]
pub struct SKPlan {
    pub goal: String,
    pub steps: Vec<SKPlanStep>,
}

#[derive(Clone)]
pub struct SKPlanStep {
    pub plugin_name: String,
    pub function_name: String,
    pub parameters: String,
}

impl PlannerAdapter {
    /// Execute SK plan within CT scheduler
    pub fn execute_plan(
        plan: &SKPlan,
        plugin_registry: &[SKPlugin],
        agent_id: u64,
    ) -> Result<u64, AdapterError> {
        let mut prev_task_id = None;

        for step in &plan.steps {
            // Locate plugin function in registry
            let func_signature = format!("{}_{}", step.plugin_name, step.function_name);

            // Create task for this step
            let task_id = ct_spawn_from_adapter(
                agent_id,
                &func_signature,
                step.parameters.as_bytes(),
            )?;

            // Chain to previous step
            if let Some(prev_id) = prev_task_id {
                ct_schedule_edge(prev_id, task_id)?;
            }

            prev_task_id = Some(task_id);
        }

        Ok(prev_task_id.unwrap_or(0))
    }
}
```

---

### 4.3 Memory Integration

**Concept:** SK semantic memory (embeddings + vectors) → CT isolated memory regions with NUMA affinity.

```rust
/// Semantic Kernel memory binding
pub struct MemoryAdapter;

/// SK semantic memory collection
#[derive(Clone)]
pub struct SKMemoryCollection {
    pub collection_name: String,
    pub embedding_dim: usize,
    pub max_entries: usize,
}

#[repr(C)]
pub struct CTMemoryRegion {
    pub region_id: u64,
    pub base_addr: *mut u8,
    pub size_bytes: usize,
    pub numa_node: u32,
    pub isolation_level: u32,  // Task isolation boundary
}

impl MemoryAdapter {
    /// Allocate isolated memory for SK collection
    pub fn allocate_collection(
        collection: &SKMemoryCollection,
        preferred_numa_node: u32,
    ) -> Result<CTMemoryRegion, AdapterError> {
        // Calculate required size (embeddings + metadata)
        let embedding_bytes = collection.embedding_dim * 4;  // f32
        let total_bytes = collection.max_entries * embedding_bytes;

        // Allocate via CT memory manager with NUMA affinity
        let region = ct_allocate_memory(
            total_bytes,
            preferred_numa_node,
        )?;

        Ok(CTMemoryRegion {
            region_id: region.id,
            base_addr: region.ptr as *mut u8,
            size_bytes: region.size,
            numa_node: region.numa_node,
            isolation_level: 1,  // Task-level isolation
        })
    }

    /// Bind memory region to task execution context
    pub fn bind_to_context(
        task_id: u64,
        region: &CTMemoryRegion,
    ) -> Result<(), AdapterError> {
        ct_bind_memory_region(task_id, region.region_id)
    }
}

extern "C" {
    fn ct_allocate_memory(
        size_bytes: usize,
        numa_node: u32,
    ) -> Result<AllocResult, i32>;

    fn ct_bind_memory_region(task_id: u64, region_id: u64) -> i32;
}

#[repr(C)]
pub struct AllocResult {
    pub id: u64,
    pub ptr: *const u8,
    pub size: usize,
    pub numa_node: u32,
}
```

---

## 5. Tool Integration

### 5.1 Tool Registration & Dispatch

Both LangChain and Semantic Kernel agents call external tools. The adapter dispatcher routes these through CT:

```rust
/// Tool dispatcher for both frameworks
pub struct ToolDispatcher {
    registry: alloc::collections::BTreeMap<String, ToolDefinition>,
}

#[derive(Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub input_schema: String,
    pub executor_fn: *const u8,  // Function pointer
}

impl ToolDispatcher {
    pub fn register_tool(&mut self, def: ToolDefinition) -> Result<(), AdapterError> {
        if self.registry.contains_key(&def.name) {
            return Err(AdapterError::DuplicateTool);
        }
        self.registry.insert(def.name.clone(), def);
        Ok(())
    }

    /// Execute tool within CT context (creates sub-task)
    pub fn execute_tool(
        &self,
        tool_name: &str,
        input: &str,
        parent_task_id: u64,
    ) -> Result<String, AdapterError> {
        let tool = self.registry.get(tool_name)
            .ok_or(AdapterError::ToolNotFound)?;

        // Create child task for tool execution
        let tool_task_id = ct_spawn_from_adapter(
            parent_task_id,  // Parent is calling task
            tool_name,
            input.as_bytes(),
        )?;

        // Wait for completion (scheduler handles synchronization)
        let result = ct_wait_task(tool_task_id)?;

        Ok(String::from_utf8_lossy(&result).to_string())
    }
}

extern "C" {
    fn ct_wait_task(task_id: u64) -> Result<Vec<u8>, i32>;
}
```

---

## 6. Error Handling & Recovery

```rust
#[derive(Clone, Copy, Debug)]
pub enum AdapterError {
    ChainConversionFailed,
    TaskSpawnFailed,
    EdgeSchedulingFailed,
    MemoryAllocationFailed,
    ToolNotFound,
    DuplicateTool,
    ContextBindingFailed,
    DeadlockDetected,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DeadlockDetected => write!(f, "Deadlock detected in task graph"),
            Self::ToolNotFound => write!(f, "Tool not registered in dispatcher"),
            _ => write!(f, "Adapter error"),
        }
    }
}

/// Graceful error recovery
pub fn recover_from_adapter_error(
    error: AdapterError,
    agent_id: u64,
) -> Result<(), AdapterError> {
    match error {
        AdapterError::DeadlockDetected => {
            // Scheduler already detected; clean up dependent tasks
            ct_abort_agent(agent_id)?;
            Ok(())
        }
        AdapterError::MemoryAllocationFailed => {
            // Retry with smaller allocation or different NUMA node
            Err(AdapterError::MemoryAllocationFailed)
        }
        _ => Err(error),
    }
}

extern "C" {
    fn ct_abort_agent(agent_id: u64) -> i32;
}
```

---

## 7. End-to-End Test Scenarios

### Test Scenario 1: LangChain ReActChain with Web Search

**Objective:** Verify real LangChain ReAct agent (Wikipedia search + reasoning) executes on XKernal scheduler with proper deadlock detection.

```
Agent Query: "What are the top 3 Python ML frameworks and their release dates?"

1. Coordinator Task: Initialize ReAct loop (iteration limit: 5)
2. Thought Task (Iteration 0): "I need to search for Python ML frameworks"
3. Action Task (Iteration 0): Call Wikipedia search tool
4. Tool Task: Execute search (web I/O → ComputeNode)
5. Observe Task (Iteration 0): Parse search results
6. Thought Task (Iteration 1): "Found TensorFlow, PyTorch, scikit-learn. Get release dates"
7. Action Task (Iteration 1): Call Wikipedia search for each
8. Tool Tasks (3x): Parallel web searches (scheduler parallelizes)
9. Observe Task (Iteration 1): Aggregate results
10. Final Task: Format answer

VERIFICATION POINTS:
- All tasks created via ct_spawn_from_adapter() in proper dependency order
- Parallel tool tasks verified by scheduler (no deadlock, NUMA affinity logged)
- Memory accounting: ~512 KB per iteration (embeddings + context)
- Result: Correct answer with 3 frameworks + dates

DEADLINE: Task completes within 8 seconds (scheduler accounts for I/O blocking)
```

### Test Scenario 2: Semantic Kernel Multi-Plugin Orchestration

**Objective:** Verify SK multi-plugin workflow (summarization → translation → analysis) executes with memory isolation.

```
Agent Goal: Summarize a research paper, translate to Spanish, analyze sentiment

1. Plugin Registration: TextSummarizer, Translator, SentimentAnalyzer
2. Planner generates 3-step plan:
   - Step 1: TextSummarizer.Summarize(paper_content)
   - Step 2: Translator.Translate(summary, "Spanish")
   - Step 3: SentimentAnalyzer.Analyze(translated_summary)

EXECUTION FLOW:
1. Task A: Summarize (allocate 256 KB memory on NUMA node 0)
2. Task B: Translate (waits for A, allocate 128 KB on NUMA node 1)
3. Task C: Analyze (waits for B, allocate 64 KB on NUMA node 0)

VERIFICATION POINTS:
- Memory regions properly isolated (no cross-task contamination)
- NUMA affinity switches honored (B on different node than A)
- Semantic memory collections pre-allocated before execution
- Sequential dependency chain enforced (no premature execution)
- Result: Correct summary in Spanish with sentiment scores

DEADLINE: ~2 seconds end-to-end (LLM calls dominate)
```

---

## 8. Integration Test Suite (20+ Tests)

### Category A: Chain Conversion Tests (5 tests)

| Test ID | Name | Scope |
|---------|------|-------|
| A1 | SimpleChain with 3 steps | Convert, verify dependency edges |
| A2 | ReActChain max-iteration enforcement | Check loop termination logic |
| A3 | MapReduceChain parallelism | Verify 4 parallel map tasks created |
| A4 | RouterChain conditional branching | Both branches instantiated (only 1 executes) |
| A5 | Nested chains | Convert chain-of-chains with 2 levels |

### Category B: Plugin & Memory Tests (6 tests)

| Test ID | Name | Scope |
|---------|------|-------|
| B1 | Plugin registration | Register 5 plugins, verify all accessible |
| B2 | Memory allocation NUMA affinity | Allocate 1MB on NUMA 1, verify placement |
| B3 | Memory isolation between tasks | 2 tasks, verify no memory bleed |
| B4 | Collection pre-allocation | SK collection allocated before planner execution |
| B5 | Memory region binding | Bind region to task context, verify in execution |
| B6 | Memory exhaustion handling | Exceed available memory, verify graceful error |

### Category C: Tool Integration Tests (4 tests)

| Test ID | Name | Scope |
|---------|------|-------|
| C1 | Tool registration | Register 10 tools, verify dispatcher index |
| C2 | Sequential tool calls | Parent task calls tool A, then B (order enforced) |
| C3 | Parallel tool dispatch | Parent spawns 3 parallel tool tasks |
| C4 | Tool result propagation | Tool output correctly returned to caller |

### Category D: Deadlock & Error Handling Tests (3 tests)

| Test ID | Name | Scope |
|---------|------|-------|
| D1 | Cyclic dependency detection | ReAct with circular reference → deadlock detected |
| D2 | Task timeout recovery | Task hangs, scheduler timeout triggers abort |
| D3 | Memory cleanup on error | Failed task releases allocated memory |

### Category E: End-to-End Integration Tests (2 tests)

| Test ID | Name | Scope |
|---------|------|-------|
| E1 | LangChain ReAct on scheduler | Complete scenario 1 execution |
| E2 | Semantic Kernel multi-plugin | Complete scenario 2 execution |

---

## 9. Implementation Checklist

- [ ] LangChain FFI bindings (adapters/langchain_bridge.rs): 200 lines
  - [ ] SimpleChainAdapter::convert_chain()
  - [ ] ReActChainAdapter::convert_react_loop()
  - [ ] MapReduceChainAdapter::convert_map_reduce()
  - [ ] RouterChainAdapter::convert_router_chain()

- [ ] Semantic Kernel FFI bindings (adapters/semantic_kernel_bridge.rs): 180 lines
  - [ ] PluginAdapter::register_plugin()
  - [ ] PlannerAdapter::execute_plan()
  - [ ] MemoryAdapter::allocate_collection()
  - [ ] MemoryAdapter::bind_to_context()

- [ ] Tool dispatcher (adapters/tool_dispatcher.rs): 80 lines
  - [ ] ToolDispatcher registration & lookup
  - [ ] Execute tool within CT context

- [ ] Error handling (adapters/error.rs): 40 lines
  - [ ] AdapterError enum
  - [ ] Recovery strategies

- [ ] Integration tests (tests/adapter_integration_tests.rs): 250 lines
  - [ ] 20+ test functions covering all categories

- [ ] Real agent test harnesses: 100 lines
  - [ ] Scenario 1 & 2 executable demonstrations

---

## 10. Success Criteria (Week 16 Exit)

1. **LangChain Adapter Complete:** All 4 chain types (SimpleChain, ReActChain, MapReduceChain, RouterChain) convert to CT task graphs and execute on scheduler without deadlock.

2. **Semantic Kernel Adapter Complete:** Plugins, planners, and memory collections integrate with CT execution; task dependencies enforced; memory isolation verified.

3. **End-to-End Agents:** Both Scenario 1 (ReAct web search) and Scenario 2 (SK multi-plugin) execute successfully with correct results.

4. **Tool Integration:** Tools callable from both frameworks; dispatcher routes correctly; results propagate to parent tasks.

5. **Memory Integration:** SK semantic memory pre-allocated, isolated, NUMA-affine; no cross-task contamination.

6. **Error Handling:** Deadlock detection catches cycles; memory exhaustion handled gracefully; tool not-found returns proper errors.

7. **Integration Test Suite:** All 20+ tests pass; coverage includes chain conversion, plugins, tools, deadlock scenarios, end-to-end workflows.

---

## 11. References

- **Week 15 Deliverables:** Scheduler API (ct_spawn_from_adapter, ct_graph_submit)
- **Phase 1 Foundations:** 4D scheduler, NUMA management, deadlock detection
- **Design Principles:** P6 (Framework-Agnostic), P1 (Agent-First)
- **External Frameworks:** LangChain 4, Semantic Kernel (latest stable)

---

## 12. Appendix: Key Rust Patterns (no_std)

```rust
// Safe string handling without std
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// Error handling without ? operator in some contexts
match ct_spawn_from_adapter(...) {
    Ok(id) => { /* continue */ }
    Err(e) => { /* handle */ }
}

// Extern C FFI (C calling convention, no mangling)
extern "C" {
    fn ct_spawn_from_adapter(
        agent_id: u64,
        task_name: *const u8,
        task_name_len: usize,
        input: *const u8,
        input_len: usize,
    ) -> i64;  // Returns task_id on success, -1 on error
}
```

---

**Document Version:** 1.0
**Last Updated:** March 2, 2026
**Status:** Ready for Week 16 Implementation
