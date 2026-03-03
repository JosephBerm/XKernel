# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 28

## Phase: PHASE 3 — Production Hardening + Launch (Weeks 25-36)

## Weekly Objective
Complete benchmarking of Customer Support and Scientific Discovery workloads. Validate scheduler performance across full spectrum of workload types.

## Document References
- **Primary:** Section 7 (Reference Workloads: Real-Time Customer Support, Scientific Discovery), Section 6.4 (Weeks 26-30)
- **Supporting:** Section 3.2.2 (GPU scheduling for Science Discovery), Section 3.2.8 (Reasoning Watchdog for deadline enforcement)

## Deliverables
- [ ] Real-Time Customer Support benchmark (200 agents, concurrent conversations)
- [ ] Measure: response latency (p50, p99), conversation context accuracy, escalation efficiency
- [ ] Scientific Discovery benchmark (20 agents GPU-heavy)
- [ ] Measure: inference latency, GPU utilization, checkpoint overhead, long-running CT handling
- [ ] Detailed analysis — understand how scheduler handles deadline-driven and GPU-bound workloads
- [ ] Comparison — vs Linux baseline
- [ ] Final benchmark results compilation — all 4 workloads at all scales

## Technical Specifications
**Real-Time Customer Support Scenario (Section 7):**

Workload Structure:
- 200 agents, each handling concurrent conversations
- Multiple concurrent conversations (10-50 at peak)
- Shared knowledge: company policies, product FAQ, previous interactions
- Tool access: knowledge search, escalation to human agent, ticket creation
- Deadline: each conversation turn must complete <500ms (interactive SLO)
- Dynamic: agents added/removed as conversation load changes

Key Scheduler Tests:
- Deadline enforcement: are all turns completing within SLO?
- Priority escalation: do agents approaching deadline get higher priority?
- Shared memory contention: does knowledge base access scale?
- Interactive latency: what's p99 latency for a conversation turn?

Expected Metrics:
- Response latency p50: <100ms (vs Linux: 150-200ms)
- Response latency p99: <500ms (vs Linux: 800-1200ms)
- Escalation latency: <1s (human should take over quickly)
- Knowledge base throughput: 1000+ lookups/second

**Scientific Discovery Scenario (Section 7):**

Workload Structure:
- 20 agents, GPU-heavy iterative loops
- Hypothesis generation (2 agents, CPU-heavy reasoning)
- Simulation/inference (10 agents, GPU-heavy)
- Analysis (5 agents, mixed CPU/GPU)
- Aggregation (3 agents)
- Long-running: each agent may run for hours
- Checkpointing: each agent checkpoints every 60 seconds

Key Scheduler Tests:
- GPU batching: can concurrent inference requests be batched?
- Long-running CT handling: do agents complete without timeout?
- Checkpoint efficiency: how much overhead from periodic checkpointing?
- GPU memory management: are KV-caches isolated correctly?

Expected Metrics:
- Inference batching benefit: 40-50% latency reduction via kernel batching
- Checkpoint overhead: <5% of execution time
- Long-running CT success: all 20 agents complete task (no timeouts)
- GPU utilization: 85-95% during inference phase

## Dependencies
- **Blocked by:** Week 27 (Enterprise and Code Review benchmarks complete)
- **Blocking:** Week 29+ (fuzz testing and security work)

## Acceptance Criteria
- [ ] Customer Support benchmark completes successfully
- [ ] Scientific Discovery benchmark completes successfully
- [ ] All metrics collected
- [ ] Performance meets or exceeds targets
- [ ] Comparison vs Linux documented
- [ ] Scheduler behavior analysis complete
- [ ] All 4 workloads benchmarked (complete Phase 3 Week 25-28 goal)

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Validation across full workload spectrum
