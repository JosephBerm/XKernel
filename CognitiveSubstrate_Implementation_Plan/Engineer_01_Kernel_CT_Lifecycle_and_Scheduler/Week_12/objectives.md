# Engineer 1 — Kernel: CT Lifecycle & Scheduler — Week 12

## Phase: PHASE 1 — Core Services + Multi-Agent (Weeks 7-14)

## Weekly Objective
Complete GPU Manager integration and begin Phase 1 stability work. Ensure dual-resource scheduler works end-to-end with GPU Manager. Focus on production reliability, testing, and documentation.

## Document References
- **Primary:** Section 11-12 (Weeks 11-12: Integration with GPU Manager (Engineer 5) — coordinate CPU scheduling with TPC allocation)
- **Supporting:** Section 3.2.2 (GPU Scheduling overview), Section 3.3.2 (GPU Manager implementation details)

## Deliverables
- [ ] GPU Manager integration completion — resolve any remaining interfaces with Engineer 5
- [ ] End-to-end test — spawn 5 CTs, verify dual-resource allocation (CPU + GPU) works
- [ ] TPC allocation/deallocation stress test — rapidly allocate/deallocate TPCs, verify no leaks
- [ ] Latency modeling validation — profile actual inference latencies vs predictions
- [ ] Documentation — scheduler design document (architecture, algorithms, examples)
- [ ] Code review — full dual-resource scheduler reviewed and signed off
- [ ] Performance baseline — measure scheduler overhead, target <1% CPU for scheduling decisions
- [ ] Robustness — handle GPU Manager failures gracefully (timeouts, allocation denials)

## Technical Specifications
**Integration Handshake (with Engineer 5's GPU Manager):**
- GPU Manager exports: request_tpc(count, priority) → Option<GpuAllocation>
- Scheduler calls this when CT enters reason phase
- GPU Manager tracks TPC ownership, enforces quotas
- On CT preemption/termination, scheduler calls release_tpc(allocation)

**Stress Testing Scenarios:**
- Allocate/deallocate 1000 TPCs in 10ms intervals → verify no leaks, correct accounting
- Kill CT mid-inference → verify TPCs released immediately
- Request more TPCs than available → verify graceful queue/backoff

**Latency Modeling Validation:**
- Profile 100 inference runs with different TPC allocations
- Compare actual latency vs predicted latency
- Target: prediction error <20% p99

**Scheduler Overhead Measurement:**
- Instrument scheduling decision loop
- Measure time spent in priority calculation, GPU allocation request, context switch
- Target: <1% of total execution time (50-500ms inference phase → <5ms scheduler overhead)

**Documentation Content:**
- Architecture overview (dual-resource scheduling model)
- Priority scoring algorithm and formulas
- GPU TPC allocation strategy
- Deadlock prevention and wait-for graph
- Crew-aware scheduling affinity
- Examples with real workload scenarios

## Dependencies
- **Blocked by:** Week 11 (GPU Manager interface), Engineer 5 GPU Manager implementation
- **Blocking:** Week 13-14 (demo preparation with multi-agent crew)

## Acceptance Criteria
- [ ] GPU Manager integration complete and tested
- [ ] Dual-resource scheduler passes end-to-end tests
- [ ] TPC allocation stress test passes (1000 allocations/deallocations, no leaks)
- [ ] Latency modeling prediction error <20% p99
- [ ] Scheduler overhead <1% CPU
- [ ] Graceful failure handling when GPU Manager unavailable
- [ ] Comprehensive documentation written
- [ ] Code review completed and approved
- [ ] All integration tests pass

## Design Principles Alignment
- **P7 — Production-Grade from Phase 1:** End-to-end integration and stress testing ensure production readiness
- **P5 — Observable by Default:** Documentation enables visibility into scheduling decisions
