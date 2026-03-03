# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 20

## Phase: PHASE 2 — Agent Runtime + SDKs (Weeks 15-24)

## Weekly Objective
Optimize cold start latency. Target <50ms from agent definition to first CT execution. Focus on agent initialization, CT allocation, and early scheduling.

## Document References
- **Primary:** Section 7 (Cold Start target: <50ms from agent definition to first CT execution), Section 3.4.3 (Agent Lifecycle Manager)
- **Supporting:** Section 3.2.1 (Boot Sequence), Section 3.3.1 (Semantic Memory Manager allocation)

## Deliverables
- [ ] Cold start profiling — measure time from agent unit file loaded to first CT executing
- [ ] Agent initialization path profiling — identify bottlenecks in agent startup
- [ ] CT allocation optimization — pre-allocate CT structures, reduce allocation latency
- [ ] Memory allocation optimization — use slab allocators for L1/L2 context windows
- [ ] Scheduler insertion optimization — O(1) insertion into priority heap
- [ ] Framework adapter startup optimization — minimize adapter initialization overhead
- [ ] Benchmark: cold start latency for LangChain and Semantic Kernel agents
- [ ] Target: <50ms end-to-end

## Technical Specifications
**Cold Start Timeline:**
1. Agent unit file loaded: agent_lifecycle_manager reads config (expect: 1-2ms)
2. Agent initialized: create Agent struct, allocate capabilities (expect: 2-3ms)
3. First CT spawned: ct_spawn called by framework adapter (expect: 5-10ms)
4. First CT scheduled: CT enters runqueue, context switch happens (expect: <1ms)
5. First instruction executed: CT code begins running (expect: <1ms)
- Total budget: 50ms (currently ~20ms, room for growth)

**Agent Initialization Bottlenecks:**
- Capability graph creation — might walk entire capability tree O(n)
  - Optimization: copy parent Agent's capabilities directly (O(1) for CT inherit)
- Memory allocation — allocate L1, L2, L3 references
  - Optimization: pre-reserve memory pools at boot, avoid slow malloc
- Framework adapter initialization — adapter loads models, plugins, etc.
  - Optimization: lazy loading (defer until first CT executes)

**CT Allocation Optimization:**
- Current: allocate CT struct, initialize all fields
- Optimization: pre-allocate pool of 1000 CT structures at boot
- Implementation: slab allocator, allocate from pool in O(1)
- Expected improvement: 2-3ms reduction

**Memory Allocation Optimization:**
- Current: allocate L1/L2/L3 memory references separately
- Optimization: batch allocate memory at CT spawn time
- Implementation: Memory Manager provides batch allocation API
- Expected improvement: 1-2ms reduction

**Scheduler Insertion Optimization:**
- Current: insert CT into priority heap O(log n)
- Optimization: priority heap already optimal, but can reduce constant factors
- Implementation: use faster heap implementation (Fibonacci heap? No, too complex)
- Expected improvement: <0.1ms (already negligible)

**Framework Adapter Optimization:**
- Current: adapter loads all plugins/tools at agent init
- Optimization: lazy load plugins only when first CT uses them
- Implementation: defer tool binding until first tool_invoke call
- Expected improvement: 5-10ms reduction (depends on adapter)

## Dependencies
- **Blocked by:** Week 19 (performance infrastructure from earlier weeks), Engineer 7, 8 (Agent Lifecycle Manager interface)
- **Blocking:** Week 21-24 (overall performance targets), Phase 2 exit criteria

## Acceptance Criteria
- [ ] Cold start profiling complete (timeline broken down per phase)
- [ ] Agent initialization bottlenecks identified
- [ ] CT allocation optimization implemented (pool-based allocation)
- [ ] Memory allocation optimization implemented (batch allocation)
- [ ] Framework adapter optimization implemented (lazy loading)
- [ ] Cold start latency <50ms for LangChain and Semantic Kernel agents
- [ ] Backward compatible (no correctness issues)
- [ ] Benchmarks documented

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Cold start <50ms is production requirement
