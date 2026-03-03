# Engineer 4 — Services: Semantic Memory Manager — Week 9

## Phase: 1 — Three-Tier Implementation
## Weekly Objective
Implement L2 Episodic Memory (Host DRAM) with semantic indexing via embedded vector index. Establish per-agent indexed storage without external pgvector dependency. Build store, retrieve, and search operations with vector-based lookups.

## Document References
- **Primary:** Section 6.2 — Phase 1, Week 7-10 (Three-tier with prefetch, CRDT, OOC handler)
- **Supporting:** Section 2.5 — SemanticMemory, Section 3.3.1 — Semantic Memory Manager

## Deliverables
- [ ] L2 memory allocator for Host DRAM with per-agent bucket isolation
- [ ] Embedded vector index implementation (no external pgvector server)
- [ ] Semantic store operation (store vector + metadata in L2)
- [ ] Semantic retrieve operation (lookup by key, get vector + metadata)
- [ ] Semantic search operation (k-nearest neighbors via embedded index)
- [ ] Unit tests for vector operations at scale (10K-100K vectors)
- [ ] Performance benchmarks for semantic search latency

## Technical Specifications
- Create L2 storage structure: per-agent DRAM region with vector index
- Implement embedded vector database (dimensions configurable, e.g., 1024D for embeddings)
- Use fast indexing (e.g., LSH, IVF, or approximate ANN) for k-NN search
- Support exact match retrieval by semantic key/handle
- Store metadata alongside vectors (timestamp, source, confidence, semantic tags)
- Implement vector quantization for memory efficiency (float32 → int8/int16)
- Define L2 size budgets per agent (typically 1-10GB per CT)
- Support batch store/search operations for efficiency

## Dependencies
- **Blocked by:** Week 7 (L1 allocator provides foundation)
- **Blocking:** Week 10 (spill-first eviction), Week 11 (compactor)

## Acceptance Criteria
- [ ] L2 can store and retrieve 100K vectors with <1ms latency
- [ ] k-NN search (k=20) completes in <50ms for 100K vectors
- [ ] Vector quantization reduces per-vector memory to <512 bytes
- [ ] Embedded index works without external services
- [ ] Per-agent isolation enforced (one CT cannot see another's L2)
- [ ] Integration test: store vectors, search by similarity

## Design Principles Alignment
- **Simplicity:** Embedded indexing eliminates external service dependency
- **Efficiency:** Vector quantization and indexing reduce memory footprint
- **Isolation:** Per-agent buckets ensure privacy and isolation
- **Performance:** Fast search enables semantic reasoning at CT scale
