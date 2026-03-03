# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 25

## Phase: PHASE 3 — Production Hardening + Launch (Weeks 25-36)

## Weekly Objective
Begin comprehensive benchmarking across reference workloads. Measure scheduler performance at scale (10, 50, 100, 500 concurrent agents) on all four reference workloads.

## Document References
- **Primary:** Section 6.4 (Phase 3 Weeks 25-28: Benchmark suite across 10, 50, 100, 500 concurrent agents on 4 reference workloads)
- **Supporting:** Section 7 (Benchmark Strategy with four reference workloads and eight measurement dimensions)

## Deliverables
- [ ] Benchmark infrastructure setup — automated test harness, result collection, graphing
- [ ] Benchmark 1: Enterprise Research Team (50 agents: 10 research, 10 writing, 10 analysis, 10 review, 10 coordination)
- [ ] Benchmark 2: Autonomous Code Review (100 agents: 50 analysis, 25 test generation, 25 documentation)
- [ ] Benchmark 3: Real-Time Customer Support (200 agents: concurrent conversations with shared knowledge)
- [ ] Benchmark 4: Scientific Discovery (20 agents: GPU-heavy iterative hypothesis-experiment-analysis)
- [ ] Scale tests: run each workload at 10, 50, 100, 500 concurrent agents
- [ ] Measurement collection — all 8 dimensions (throughput, efficiency, memory, IPC, overhead, cost, cold start, fault recovery)
- [ ] Benchmark results documentation — save for publication

## Technical Specifications
**Four Reference Workloads (Section 7):**

1. **Enterprise Research Team (50 agents):**
   - 10 research agents (web search, document reading)
   - 10 writing agents (draft composition, editing)
   - 10 analysis agents (data processing, insight extraction)
   - 10 review agents (fact checking, quality review)
   - 10 coordination agents (task assignment, result aggregation)
   - Shared L3 knowledge base (research findings)
   - Expected: tests crew scheduling, memory sharing, deadline management

2. **Autonomous Code Review (100 agents):**
   - 50 analysis agents (static analysis, logic review)
   - 25 test generation agents (test case creation, verification)
   - 25 documentation agents (code documentation, API docs)
   - High concurrency, mixed reasoning and tool use
   - Expected: tests scheduling throughput, tool call overhead

3. **Real-Time Customer Support (200 agents):**
   - Concurrent conversations (10-50 concurrent)
   - Shared knowledge (company policies, product FAQ)
   - Escalation (route to human agent when needed)
   - Real-time monitoring (deadline-driven)
   - Expected: tests deadline scheduling, interactive latency

4. **Scientific Discovery (20 agents GPU-heavy):**
   - Iterative hypothesis generation (2 agents)
   - Experiment simulation (10 agents, GPU-heavy inference)
   - Analysis (5 agents)
   - Aggregation (3 agents)
   - Expected: tests GPU scheduling, long-running CT checkpointing, inference batching

**Eight Measurement Dimensions (Section 7):**
1. Multi-Agent Throughput: CTs/sec at 10, 50, 100, 500 agents (target: 3-5x vs Linux)
2. Inference Efficiency: GPU-ms per reasoning chain (target: 30-60% reduction)
3. Memory Efficiency: working set per agent (target: 40-60% reduction)
4. IPC Latency: request-response latency (target: sub-microsecond)
5. Security Overhead: capability check latency (target: <100ns)
6. Cost Attribution: accuracy (target: >99%)
7. Cold Start: agent def to first execution (target: <50ms)
8. Fault Recovery: exception to resumed execution (target: <100ms)

## Dependencies
- **Blocked by:** Phase 2 complete (Week 24)
- **Blocking:** Week 26-28 (continue benchmarking), Phase 3 exit criteria

## Acceptance Criteria
- [ ] Benchmark infrastructure operational and validated
- [ ] All 4 reference workloads executable
- [ ] Throughput measured at all 4 scales (10/50/100/500 agents)
- [ ] All 8 measurement dimensions collected
- [ ] No crashes or memory leaks during benchmarking
- [ ] Benchmark results saved for analysis and publication

## Benchmark Percentile Definitions

**Percentile Reporting Requirements (Addendum v2.5.1 — Correction 2: Benchmark Methodology)**

All benchmark measurements MUST report the following percentiles:
- p50, p95, p99, p99.9 for all latency metrics

**Statistical Rigor Requirements:**
- Minimum 100 runs per measurement
- 95% confidence interval required
- First 10 warmup runs discarded from results

**IPC Latency Targets:**
- p50 < 500ns
- p95 < 1µs
- p99 < 5µs
- p99.9 < 50µs

**Cold Start Targets:**
- p50 < 30ms
- p95 < 50ms
- p99 < 100ms
- p99.9 < 500ms

**Fault Recovery Targets:**
- p50 < 50ms
- p95 < 100ms
- p99 < 250ms
- p99.9 < 1s

**Linux+Docker Baseline Specification:**
- OS: Ubuntu 24.04 LTS
- Kernel: 6.8+
- Docker: 27.x
- GPU: Same hardware for all comparisons
- Python/ML Framework: LangChain 0.3.x / SK 1.x

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Comprehensive benchmarking validates production readiness
