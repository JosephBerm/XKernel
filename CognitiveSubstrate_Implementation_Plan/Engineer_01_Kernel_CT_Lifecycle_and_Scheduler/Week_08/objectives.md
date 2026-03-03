# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 08

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Complete Cognitive Priority Scheduler implementation by adding Deadline Pressure (0.2 weight) and Capability Cost (0.15 weight) dimensions. Implement full 4-dimensional scoring and inference batching support.

## Document References
- **Primary:** Section 3.2.2 (Cognitive Priority Scheduler — all four dimensions complete, including Capability Cost for GPU-heavy phases)
- **Supporting:** Section 2.12 (WatchdogConfig with deadline_ms), Section 2.1 (CognitivePriority struct with all four components)

## Deliverables
- [ ] Deadline Pressure scorer (0.2 weight) — escalates as wall-clock deadline approaches
- [ ] Capability Cost scorer (0.15 weight) — GPU-heavy reason phases yield CPU to reflect phases
- [ ] Full 4-dimensional priority formula: priority = 0.4*chain + 0.25*efficiency + 0.2*deadline + 0.15*cost
- [ ] Inference batching detection — identify CTs ready for concurrent execution on GPU
- [ ] Batch-ready CT co-scheduling logic — schedule batch-ready CTs together on CPU
- [ ] GPU-ready signal to GPU Manager — notify which CTs can batch for next GPU inference cycle
- [ ] Test suite — 20+ test cases covering all four dimensions, deadline escalation, batching scenarios

## Technical Specifications
**Deadline Pressure (0.2 weight) — Section 3.2.2:**
- For each CT, track deadline (from watchdog_config.deadline_ms)
- Score = (deadline_elapsed_ms) / (deadline_total_ms), normalized to [0, 1]
- As deadline approaches (80%, 90%, 95%), score increases
- CTs approaching deadline get higher priority
- Example: at 80% of deadline elapsed, deadline_pressure_score = 0.8

**Capability Cost (0.15 weight) — Section 3.2.2:**
- Track which CT phase (plan, reason, act, reflect, yield) uses GPU vs CPU
- reason phase (inference) is GPU-heavy → lower CPU priority while GPU-bound
- reflect phase is CPU-heavy → higher CPU priority while CPU-intensive
- If GPU Manager indicates CT_A is running inference (GPU-bound), lower CPU priority; elevate reflect-phase CTs
- Score = (cpu_bound_factor) ∈ [0, 1] where GPU-bound CTs → 0.0, CPU-heavy CTs → 1.0

**Inference Batching (Section 3.2.2):**
- Batch-ready: two CTs ready for reason phase, same LLM model, compatible sequence lengths
- Co-schedule batch-ready CTs to run on same CPU cores in same time window
- This allows GPU Manager to batch their inference requests together
- Batching benefit: 30-60% reduction in inference latency per token (from compiler paper)

**Priority Recalculation:**
- Deadline Pressure score updated every 100ms (or at context switch)
- As CT progresses toward deadline, priority increases automatically
- No manual intervention needed

## Dependencies
- **Blocked by:** Week 07 (Chain Criticality, Resource Efficiency), Engineer 5 must provide GPU Manager interface by Week 08
- **Blocking:** Week 09 (Crew-aware scheduling), Week 11-12 (GPU integration)

## Acceptance Criteria
- [ ] Deadline Pressure scorer correctly escalates as deadline approaches
- [ ] Capability Cost scorer correctly identifies GPU-bound vs CPU-bound phases
- [ ] Full 4-dimensional priority formula correctly calculated
- [ ] Priority scores [0, 1] normalized and weighted correctly
- [ ] Inference batching detection identifies batch-ready CT pairs
- [ ] Batch-ready CTs co-scheduled together (same CPU cores, same time window)
- [ ] All 20+ test cases pass
- [ ] Integration test: 100 CTs with deadlines, verify deadline-driven priority escalation
- [ ] Batching test: spawn 10 CTs with same model, verify co-scheduling for GPU efficiency

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** Full 4-dimensional scheduling addresses production complexity
- **P2 — Cognitive Primitives as Kernel Abstractions:** Cognitive priority is kernel core, not framework library
