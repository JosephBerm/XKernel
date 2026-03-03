# Engineer 1 — Kernel: CT Lifecycle & Scheduler — 36-Week Implementation Plan

## Overview

This folder contains comprehensive weekly implementation objectives for Engineer 1, who owns the **CognitiveTask (CT) Lifecycle State Machine** and **Cognitive Priority Scheduler** within the L0 Microkernel of Cognitive Substrate.

Engineer 1 is responsible for building the fundamental scheduler that controls how AI agents' reasoning tasks are executed across CPU and GPU resources in a production-grade, bare-metal operating system.

## 36-Week Roadmap

### Phase 0: Domain Model + Kernel Skeleton (Weeks 1-6)
- **Week 01-02:** Formalize 12 domain model entities in Rust types; begin CT lifecycle state machine
- **Week 03-04:** Implement round-robin scheduler and boot on QEMU; add CT dependency DAG with cycle detection
- **Week 05-06:** Integrate with capability engine; complete Phase 0 integration testing

**Exit Criteria:** Bootable microkernel, spawn 100 CTs, enforce capabilities with policies, handle exceptions, checkpoint and restore, detect dependency cycles.

### Phase 1: Core Services + Multi-Agent (Weeks 7-14)
- **Week 07-08:** Implement 4-dimensional Cognitive Priority Scheduler (Chain Criticality, Resource Efficiency, Deadline Pressure, Capability Cost)
- **Week 09-10:** Add crew-aware NUMA scheduling; implement runtime deadlock detection with wait-for graphs
- **Week 11-12:** Integrate with GPU Manager for dual-resource scheduling (CPU + GPU)
- **Week 13-14:** Prepare and execute Phase 1 demo (3-agent crew scenario with fault tolerance)

**Exit Criteria:** AgentCrew of 3 agents collaborating with full fault tolerance demonstrated; all failure scenarios handled.

### Phase 2: Agent Runtime + SDKs (Weeks 15-24)
- **Week 15-16:** Support runtime stream; expose scheduler APIs for framework adapters (LangChain, Semantic Kernel)
- **Week 17-20:** Performance profiling and optimization (priority calculation caching, context switch <1µs, cold start <50ms, IPC sub-microsecond)
- **Week 21-22:** Run 10 real-world agent scenarios; validate scaling to 500 concurrent agents
- **Week 23-24:** Complete Phase 2 exit criteria; verify all performance targets met

**Exit Criteria:** 10 real-world agent scenarios running with measured perf vs Linux+Docker; CSCI v1.0 published; cs-pkg with 10+ packages; all 5 debug tools functional.

### Phase 3: Production Hardening + Launch (Weeks 25-36)
- **Week 25-28:** Comprehensive benchmarking across 4 reference workloads at 10/50/100/500 concurrent agents
- **Week 29-30:** Fuzz testing and adversarial testing (capability escalation, priority inversion, resource exhaustion, deadlock bypass)
- **Week 31-32:** Fix all critical/high findings; begin paper writing for OSDI/SOSP/COLM
- **Week 33-36:** Final security audit; OS completeness re-audit; open-source launch

**Exit Criteria:** 3-5x throughput improvement demonstrated; paper submitted; OS completeness audit 100%; open-source release live on GitHub.

## File Structure

Each `Week_XX/objectives.md` file contains:

1. **Phase** — which of 4 phases this week belongs to
2. **Weekly Objective** — clear goal for the week
3. **Document References** — precise section citations from Engineering Plan v2.5
4. **Deliverables** — specific artifacts to create (modules, tests, documentation)
5. **Technical Specifications** — algorithms, data structures, performance targets
6. **Dependencies** — what blocks this week, what this week blocks
7. **Acceptance Criteria** — how to verify the week's work is complete
8. **Design Principles Alignment** — which of 8 design principles this week satisfies (P1-P8)

## Key Responsibilities

### CT Lifecycle Management
- Implement `CTPhase` enum: spawn → plan → reason → act → reflect → yield → complete
- Enforce all 6 invariants on CognitiveTask entity (capabilities subset, budget, dependencies, phase transitions, DAG acyclic, watchdog enforcement)
- Type-safe state machine using Rust type-state pattern to prevent illegal transitions at compile time

### 4-Dimensional Priority Scheduling
- **Chain Criticality (0.4 weight):** CTs unblocking most downstream work get highest CPU priority
- **Resource Efficiency (0.25 weight):** Batch-ready CTs co-scheduled for GPU inference efficiency
- **Deadline Pressure (0.2 weight):** Priority escalates as wall-clock deadline approaches
- **Capability Cost (0.15 weight):** GPU-heavy inference phases yield CPU to CPU-heavy reflection phases

### GPU Scheduling
- Dual-resource scheduling: simultaneously allocate CPU cores and GPU TPCs
- TPC-level spatial scheduling (LithOS-inspired): manage TPCs like CPU cores
- Kernel atomization: split long-running kernels into atoms without app changes
- Dynamic right-sizing: allocate minimal TPCs to meet latency SLO

### Fault Tolerance
- Implement exception engine: handle ContextOverflow, ToolCallFailed, CapabilityExpired, BudgetExhausted, ReasoningDiverged, DeadlineExceeded, DependencyCycleDetected
- Signal dispatch: deliver SIG_CTXOVERFLOW, SIG_CAPREVOKED, SIG_PRIORITY_CHANGE, SIG_DEADLINE_WARN, SIG_BUDGET_WARN, SIG_CREW_UPDATE, SIG_TERMINATE, SIG_CHECKPOINT
- Checkpointing: copy-on-write for CPU state, concurrent GPU state via PhoenixOS-style speculation
- Deadlock prevention: static DAG checking + runtime wait-for graph with preemption-based resolution

## Performance Targets (Section 7 of Engineering Plan)

- **IPC Latency:** sub-microsecond for request-response between co-located agents
- **Capability Check Overhead:** <100ns per local handle check
- **Cold Start:** <50ms from agent definition to first CT execution
- **Fault Recovery:** <100ms from exception to resumed execution via checkpoint
- **Context Switch:** <1µs
- **Scheduler Overhead:** <1% CPU time
- **Multi-Agent Throughput:** 3-5x improvement vs Linux+Docker at 100+ concurrent agents

## Design Principles (Section 1.2)

Engineer 1's work directly implements these principles:

- **P1 (Agent-First):** CT scheduler is for agents, not human processes
- **P2 (Cognitive Primitives as Kernel Abstractions):** Scheduler is kernel responsibility, not library
- **P3 (Capability-Based Security):** Capabilities enforced via page tables, not software checks
- **P7 (Production-Grade from Phase 1):** Every component targets production from start
- **P8 (Fault-Tolerant by Design):** Exception handling, signals, checkpointing prove fault tolerance

## Cross-Team Dependencies

- **Engineer 2 (Capability Engine):** Week 05 — capability validation on CT spawn
- **Engineer 5 (GPU Manager):** Weeks 11-12 — dual-resource scheduling coordination
- **Engineers 7-8 (Runtime):** Weeks 15-16 — framework adapter integration with scheduler APIs
- **Engineers 9-10 (SDK):** Weeks 20+ — CSCI syscall validation, debug tools

## Key Technologies

- **Language:** Rust (memory safety, ownership maps to capability semantics)
- **Kernel Model:** seL4-inspired microkernel (20-50K lines Rust, real bare-metal, MMU-backed isolation)
- **Scheduling Algorithm:** 4-dimensional priority heap (O(log n) insertion/deletion)
- **Serialization:** Cap'n Proto (zero-copy, schema-first)
- **GPU Control:** Direct MMIO + command submission queue interface (bypasses CUDA/ROCm)
- **Testing:** Fuzz testing, adversarial testing, security audit

## References

- Engineering Plan v2.5 (Cognitive Substrate)
- seL4 Microkernel (formal verification, OCap model)
- AIOS (LLM Agent OS with 2.1x performance)
- LithOS (SOSP 2025, kernel atomization, 13x lower tail latency)
- PhoenixOS (SOSP 2025, concurrent GPU checkpoint/restore)
- SchedCP (arXiv:2509.01245, autonomous eBPF scheduler generation)

## Success Metrics (36-Month Horizon)

- 3-5x throughput improvement for 100+ concurrent agents
- Sub-microsecond IPC latency
- <50ms cold start
- <100ms fault recovery
- Paper accepted at OSDI/SOSP/COLM
- 1000+ GitHub stars within 3 months of launch
- OS completeness audit passes at 100%
