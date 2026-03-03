# Engineer 4 — Services: Semantic Memory Manager — Week 32

## Phase: 3 — Production Validation & Hardening
## Weekly Objective
Validate NUMA-aware memory allocation correctness. Verify L1 placement on GPU-local DRAM, L2 balancing across NUMA nodes, L3 replication distribution. Optimize for NUMA topology.

## Document References
- **Primary:** Section 2.5 — SemanticMemory (three-tier placement)
- **Supporting:** Section 3.3 — L1 Kernel Services (process architecture)

## Deliverables
- [ ] NUMA topology detection and characterization
- [ ] L1 allocation NUMA affinity verification
- [ ] L2 NUMA placement policy (local-first)
- [ ] L3 replication distribution verification
- [ ] Memory access latency profiling (local vs. remote)
- [ ] NUMA optimization opportunities identification
- [ ] Performance impact analysis (NUMA-aware vs. unaware)
- [ ] NUMA validation report

## Technical Specifications
- Topology detection: use numactl, /proc/meminfo to understand NUMA layout
- L1 NUMA affinity: allocate on GPU-local node (if applicable)
- L2 policy: prefer local NUMA node for CT's home node
- L3 replication: distribute replicas across NUMA nodes for availability
- Latency profiling: measure access time to pages on each NUMA node
- Performance analysis: compare bandwidth local vs. remote
- Acceptable variance: remote access <3x slower than local
- Optimization: rebalance pages, tune prefetch priorities by NUMA distance

## Dependencies
- **Blocked by:** Week 31 (memory leak detection complete)
- **Blocking:** Week 33 (paper writing)

## Acceptance Criteria
- [ ] L1 NUMA placement correct (GPU-local if available)
- [ ] L2 shows NUMA locality (pages near CT home node)
- [ ] L3 replication distributed across nodes
- [ ] Access latency ratio within 3x for remote access
- [ ] NUMA-aware version performs better than unaware
- [ ] Optimization opportunities documented
- [ ] NUMA validation approved

## Design Principles Alignment
- **Performance:** NUMA-aware placement improves latency
- **Scalability:** Multi-NUMA systems supported efficiently
- **Correctness:** Proper placement prevents correctness bugs
- **Observability:** Latency profiling validates design
