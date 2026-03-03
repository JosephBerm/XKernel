# Engineer 5 — Services: GPU/Accelerator Manager — Week 11

## Phase: 1 (Multi-Model VRAM Management)
## Weekly Objective
Implement multi-model VRAM management: partition VRAM across concurrent agents based on scheduling priority. Enable async model loading with LRU eviction policy. Support simultaneous models in VRAM for inference diversity.

## Document References
- **Primary:** Section 3.3.2 — GPU/Accelerator Manager, VRAM Management subsection
- **Supporting:** Section 6.2 — Phase 1, Weeks 11-14

## Deliverables
- [ ] Multi-model VRAM partitioning scheme (priority-based allocation per agent)
- [ ] VRAM allocation state machine (free, allocated, evicting, loading)
- [ ] Model LRU eviction policy implementation (prioritize eviction of least-recently-used)
- [ ] Async model loading: Background loading without stalling executing agents
- [ ] Model preloading heuristic: Predict next models; preload during idle GPU cycles
- [ ] VRAM fragmentation prevention (defragmentation, compaction strategy)
- [ ] VRAM bound tracking per agent (ensure allocated VRAM matches assigned agents)
- [ ] Integration with Cognitive Scheduler: Priority signals → VRAM allocation decisions
- [ ] Testing: Multi-model workloads (3-5 models, agents rotating), allocation efficiency

## Technical Specifications
- VRAM partitions: Dynamic allocation based on agent priority and model sizes
- Partition size: 20GB VRAM available; models vary 5GB-16GB (multiple concurrent supported)
- LRU policy: Evict model if unused for > 60s and higher-priority model needs space
- Async loading: Model load via DMA in background; ready signal to Cognitive Scheduler
- Preloading: Predict next model from agent state; load if VRAM available
- Fragmentation target: Maintain > 90% useful VRAM allocation (< 10% wasted)
- Eviction latency: Model evict + reload < 2s to minimize agent disruption

## Dependencies
- **Blocked by:** Week 10 (Dynamic right-sizing), Week 8 (TPC scheduling validation)
- **Blocking:** Week 12-13 (KV-cache isolation), Week 13-14 (Multi-GPU support)

## Acceptance Criteria
- [ ] Multi-model VRAM partitioning tested with 3-5 concurrent models
- [ ] LRU eviction policy correctly prioritizes models for eviction
- [ ] Async model loading verified: agent execution continues during model load
- [ ] Model preloading reduces agent latency by 30-50% on model switches
- [ ] VRAM fragmentation maintained below 10%
- [ ] Integration test: Multi-model workload passes correctness and performance tests

## Design Principles Alignment
- **Priority-Driven:** VRAM allocation respects agent priority from Cognitive Scheduler
- **Async-First:** Model loading doesn't block agent execution
- **Efficiency:** Preloading and LRU policy maximize useful VRAM utilization
