# Engineer 4 — Services: Semantic Memory Manager — Week 15

## Phase: 2 — Extended Capabilities & Optimization
## Weekly Objective
Implement Knowledge Source mounting — integrate external data sources (Pinecone, Weaviate, PostgreSQL, REST APIs, S3) as semantic volumes. Enable querying through CSCI interface with capability-gated access.

## Document References
- **Primary:** Section 6.3 — Phase 2, Week 17-20 (Semantic FS with external mounts)
- **Supporting:** Section 2.5 — SemanticMemory, Section 3.3.1 — Semantic Memory Manager

## Deliverables
- [ ] Knowledge source abstraction layer (pluggable drivers)
- [ ] Pinecone connector with vector search support
- [ ] Weaviate connector for semantic search
- [ ] PostgreSQL connector for relational data
- [ ] REST API connector for generic remote sources
- [ ] S3 connector for object/document storage
- [ ] Mount point registry and lifecycle management
- [ ] Capability-based access control for mounted sources
- [ ] Unit tests for each connector type
- [ ] Integration test: mount external source, search via CSCI interface

## Technical Specifications
- Create mount abstraction: (source_type, endpoint, credentials, capability_rules)
- Implement unified query interface across all source types
- Pinecone: query by vector, support k-NN with metadata filtering
- Weaviate: GraphQL-based semantic search support
- PostgreSQL: SQL query translation, result caching
- REST APIs: JSON-RPC or gRPC protocol adapters
- S3: object listing, content fetching with lazy loading
- Capability model: CT gets read/query capability on mounted sources
- Support federation: query multiple sources with merged results
- Implement result caching (reduce redundant source queries)

## Dependencies
- **Blocked by:** Week 14 (CRDT complete, Phase 1 stable)
- **Blocking:** Week 16 (knowledge source testing), Week 18 (prefetch optimization)

## Acceptance Criteria
- [ ] All six source types successfully mounted and queryable
- [ ] Pinecone k-NN search <500ms latency
- [ ] Weaviate semantic queries <1s latency
- [ ] PostgreSQL queries <100ms latency
- [ ] Capability control prevents unauthorized source access
- [ ] Integration test: query federated sources, merge results
- [ ] Result caching improves repeated queries by 10x

## Design Principles Alignment
- **Extensibility:** Pluggable drivers support multiple source types
- **Isolation:** Capability control maintains privacy across sources
- **Performance:** Caching reduces latency for hot queries
- **Simplicity:** Unified query interface abstracts source heterogeneity
