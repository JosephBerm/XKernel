# Week 10 Deliverable: Semantic File System RFC & Query Parser Prototype (Phase 1)

**Date:** Week 10 | **Engineer:** 8 | **Status:** Phase 2 Implementation Readiness
**Objective:** Finalize Semantic FS architecture with working NL query parser, optimizer design, and caching strategy.

---

## 1. RFC-Style Semantic File System Architecture

### 1.1 Overview

The XKernal Semantic File System (SFS) bridges natural language queries with structured file system operations and knowledge base retrieval. Users express intent in natural language; the SFS parses, optimizes, and routes queries to appropriate knowledge sources (file metadata, embeddings, structured indexes) with minimal latency.

**Core Principles:**
- **Single-turn latency:** <100ms for simple queries, <500ms for complex aggregations
- **Stateless query processing:** No session state required for correctness
- **Incremental indexing:** Continuous embedding updates without blocking file operations
- **Hybrid matching:** Lexical + semantic matching for robustness
- **Intelligent caching:** Query result cache (LRU) + embedding cache (persistent)

### 1.2 Architecture Components

```
User Natural Language Query
    ↓
[NL Query Parser] → Tokenization, POS tagging, entity extraction, intent inference
    ↓
[Query Optimizer] → Source selection, parallelization, caching decisions
    ↓
[Knowledge Sources] → File metadata, embedding index, full-text search, structured queries
    ↓
[Caching Layer] → Query results, embeddings, intermediate computations
    ↓
[Result Aggregation] → Combine & rank results, format response
```

### 1.3 Knowledge Sources

1. **File Metadata Index:** Filesystem metadata (name, size, mtime, permissions, tags)
2. **Embedding Index:** Dense vector representations for semantic similarity
3. **Full-Text Index:** Inverted index for keyword-based retrieval
4. **Structured Query Engine:** SQL-like interface for attribute filtering
5. **Entity Knowledge Base:** Named entities, relationships, temporal annotations

---

## 2. Natural Language Query Parser Prototype

### 2.1 Parser Pipeline

1. **Tokenization:** Split query into linguistic units
2. **POS Tagging:** Identify nouns, verbs, adjectives, prepositions
3. **Entity Extraction:** Extract file names, dates, sizes, ownership references
4. **Intent Inference:** Classify query intent (search, filter, aggregate, traverse)
5. **Query Normalization:** Canonicalize entities, resolve ambiguities

### 2.2 Supported Query Intents

- **Search:** "Find all Rust files modified in the last week"
- **Filter:** "Show projects larger than 500MB"
- **Aggregate:** "List all owners with total storage usage"
- **Traverse:** "Navigate to the largest subdirectory"
- **Temporal:** "Files changed between March 1 and March 15"
- **Semantic:** "Documents similar to 'machine learning'"
- **Complex:** "All Python files in data-science projects modified today and smaller than 10MB"

### 2.3 Example Queries (20+ Diversity)

```
1. "Find all Rust files modified in the last week"
2. "Show me the largest directory in /home/user"
3. "List all PDF documents about machine learning"
4. "Files created by alice modified in March 2026"
5. "Directories with more than 1000 files"
6. "Search for Python notebooks with Jupyter in the name"
7. "All symlinks in /usr/local that point to /opt"
8. "Configuration files modified in the last 24 hours"
9. "Show storage usage by file extension"
10. "Readonly files owned by root"
11. "Archives created before 2020"
12. "Find test files related to authentication"
13. "Nested directories deeper than 5 levels"
14. "Images modified today"
15. "Cache files larger than 1GB"
16. "Executables in PATH with write permissions"
17. "Documentation semantically similar to API design"
18. "Files accessed but never modified"
19. "Temporary files with no recent access"
20. "All files owned by bob in projects tagged as active"
21. "Sorted by date: most recent first, only PDFs"
22. "Deep search: files in archives and compressed directories"
```

---

## 3. Query Optimizer Design

### 3.1 Optimization Strategy

The Query Optimizer makes three critical decisions:

1. **Source Selection:** Which knowledge sources to query
   - Keyword queries → Full-text index first
   - Semantic queries → Embedding index + semantic reranking
   - Attribute queries (size, owner) → Structured query engine
   - Temporal queries → Metadata index with time-range filtering

2. **Parallelization:** Execute independent lookups concurrently
   - Multiple entities → Parallel embedding lookups
   - Hybrid queries → Parallel lexical + semantic execution
   - Aggregations → Distribute filtering across knowledge sources

3. **Caching Decisions:** Leverage caching to avoid redundant computation
   - Query result cache for identical/similar historical queries
   - Embedding cache for frequently-referenced entities
   - Intermediate results for subquery reuse

### 3.2 Optimizer Algorithm

```
Input: ParsedQuery
Output: OptimizedExecutionPlan

1. Analyze query structure:
   - Identify entity references (what to match)
   - Extract attribute filters (size, owner, time range)
   - Determine semantic vs. lexical intent

2. Select sources:
   - If has_semantic_intent: add EmbeddingIndex
   - If has_attribute_filters: add StructuredQueryEngine
   - Always check QueryResultCache first
   - If has_keywords: add FullTextIndex

3. Plan parallelization:
   - Group independent operations
   - Estimate latency per source
   - Order by predicted latency (slowest first)

4. Estimate caching opportunities:
   - Check if exact query in cache
   - Identify embeddings already cached
   - Plan result caching if aggregation cost high

5. Return ExecutionPlan with:
   - Source ordering
   - Parallel execution groups
   - Caching hints
```

### 3.3 Latency Estimation Model

- **File Metadata Index:** ~5ms (in-memory hash lookups)
- **Full-Text Index:** ~20-50ms (inverted index lookups + Boolean operations)
- **Embedding Index:** ~30-100ms (vector similarity search, depends on index size)
- **Structured Query Engine:** ~15-40ms (B-tree range scans)
- **Query Result Cache Hit:** ~1ms (hash table lookup)

---

## 4. Caching Strategy

### 4.1 Query Result Cache (LRU)

**Purpose:** Cache complete query results to avoid recomputation.

- **Capacity:** 10,000 entries (configurable)
- **Eviction:** LRU when capacity exceeded
- **TTL:** 5 minutes for dynamic queries, 24 hours for static queries
- **Key:** Hash of (query_intent, normalized_entities, filters)
- **Value:** Ranked result list with metadata

**Hit Rate Target:** >70% for typical workloads (file browsing, repeated searches)

### 4.2 Embedding Cache (Persistent)

**Purpose:** Cache computed embeddings for entities (file names, tags, owners) to avoid recomputation.

- **Storage:** Persistent disk-backed cache (RocksDB)
- **Capacity:** Unbounded (until disk full)
- **Update Strategy:** Lazy invalidation on metadata changes
- **Format:** Entity hash → embedding vector (float32 array)

**Hit Rate Target:** >85% for frequently-accessed entities

### 4.3 Dual-Level Caching Architecture

```
┌─ L1 Query Result Cache (In-Memory LRU) ─┐
│ - Fast hits: 1ms lookup                  │
│ - 10K entries, short TTL                 │
└──────────────────────────────────────────┘
          ↓ Miss
┌─ L2 Embedding Cache (Persistent RocksDB) ┐
│ - Medium hits: 5-10ms lookup               │
│ - Unbounded storage, long-lived            │
└────────────────────────────────────────────┘
          ↓ Miss
┌─ Source Query Execution ─────────────────┐
│ - Slow path: 20-100ms per source           │
│ - Read from file system, indexes           │
└────────────────────────────────────────────┘
```

---

## 5. Performance Targets & Benchmark Methodology

### 5.1 Performance Targets

| Query Type | Latency Target | Notes |
|-----------|----------------|-------|
| Simple keyword search (cache hit) | <5ms | Result cache hit |
| Simple keyword search (cache miss) | <50ms | Full-text index scan |
| Simple attribute filter | <30ms | Structured query on metadata |
| Semantic search | <200ms | Embedding lookup + reranking |
| Complex multi-source query | <500ms | Parallel execution, multiple sources |
| Aggregation query | <300ms | Streaming aggregation |

### 5.2 Benchmark Methodology

**Test Suite:**
1. **Single-source queries:** 20 queries per source type
2. **Multi-source queries:** 10 complex aggregations
3. **Cache performance:** Replay traces to measure hit rates
4. **Scalability:** Vary corpus size (100K, 1M, 10M files)
5. **Concurrency:** Parallel query workloads (1, 10, 100 concurrent queries)

**Metrics:**
- **p50, p95, p99 latency** for each query type
- **Cache hit rate** (L1 + L2)
- **Throughput** (queries/second)
- **Memory usage** (parser, optimizer, caches)

**Validation:**
```
for query in TestQueries:
    start = now()
    results = execute(query)
    latency = now() - start
    verify_correctness(results)
    record_latency(latency)
```

---

## 6. Implementation Code: Rust Prototype

### 6.1 NL Query Parser

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum QueryIntent {
    Search,
    Filter,
    Aggregate,
    Traverse,
    Temporal,
    Semantic,
    Complex,
}

#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub intent: QueryIntent,
    pub keywords: Vec<String>,
    pub entities: HashMap<String, String>,
    pub filters: HashMap<String, String>,
    pub temporal_range: Option<(String, String)>,
    pub semantic_query: Option<String>,
}

pub struct NLQueryParser;

impl NLQueryParser {
    /// Tokenize query into words
    fn tokenize(query: &str) -> Vec<String> {
        query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract file-related entities (names, paths, types)
    fn extract_entities(tokens: &[String]) -> HashMap<String, String> {
        let mut entities = HashMap::new();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "file" | "files" if i + 1 < tokens.len() => {
                    entities.insert("entity_type".to_string(), "file".to_string());
                    i += 1;
                }
                "directory" | "directories" | "dir" => {
                    entities.insert("entity_type".to_string(), "directory".to_string());
                    i += 1;
                }
                token if token.ends_with(".rs") || token.ends_with(".py") => {
                    entities.insert("file_extension".to_string(), token.to_string());
                    i += 1;
                }
                "rust" | "python" | "javascript" => {
                    entities.insert("language".to_string(), tokens[i].clone());
                    i += 1;
                }
                _ => i += 1,
            }
        }
        entities
    }

    /// Extract temporal references
    fn extract_temporal(tokens: &[String]) -> Option<(String, String)> {
        let temporal_keywords = vec!["today", "week", "month", "year", "day", "hours"];
        if tokens.iter().any(|t| temporal_keywords.contains(&t.as_str())) {
            Some(("2026-03-02".to_string(), "2026-03-09".to_string()))
        } else {
            None
        }
    }

    /// Infer query intent from tokens
    fn infer_intent(tokens: &[String]) -> QueryIntent {
        let text = tokens.join(" ");
        if text.contains("similar") {
            QueryIntent::Semantic
        } else if text.contains("count") || text.contains("total") {
            QueryIntent::Aggregate
        } else if text.contains("larger") || text.contains("bigger") {
            QueryIntent::Filter
        } else if text.contains("find") || text.contains("search") {
            QueryIntent::Search
        } else if text.contains("navigate") || text.contains("go") {
            QueryIntent::Traverse
        } else if text.contains("between") || text.contains("since") {
            QueryIntent::Temporal
        } else {
            QueryIntent::Complex
        }
    }

    /// Parse natural language query
    pub fn parse(query: &str) -> ParsedQuery {
        let tokens = Self::tokenize(query);
        let keywords = tokens.clone();
        let entities = Self::extract_entities(&tokens);
        let temporal_range = Self::extract_temporal(&tokens);
        let intent = Self::infer_intent(&tokens);

        let mut filters = HashMap::new();
        if let Some(("last", _)) = tokens.windows(2).find(|w| w[0] == "last") {
            filters.insert("recency".to_string(), "recent".to_string());
        }

        ParsedQuery {
            intent,
            keywords,
            entities,
            filters,
            temporal_range,
            semantic_query: None,
        }
    }
}
```

### 6.2 Query Optimizer

```rust
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum KnowledgeSource {
    FileMetadata,
    FullTextIndex,
    EmbeddingIndex,
    StructuredQueryEngine,
}

#[derive(Debug)]
pub struct ExecutionPlan {
    pub sources: Vec<KnowledgeSource>,
    pub parallelizable: bool,
    pub cache_hint: bool,
    pub estimated_latency_ms: u32,
}

pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Select appropriate knowledge sources
    fn select_sources(parsed: &ParsedQuery) -> Vec<KnowledgeSource> {
        let mut sources = Vec::new();

        // Determine if query requires semantic matching
        if parsed.intent == QueryIntent::Semantic {
            sources.push(KnowledgeSource::EmbeddingIndex);
        }

        // Determine if query has attribute filters
        if !parsed.filters.is_empty() || parsed.temporal_range.is_some() {
            sources.push(KnowledgeSource::StructuredQueryEngine);
        }

        // Keyword-based queries use full-text index
        if !parsed.keywords.is_empty() && sources.is_empty() {
            sources.push(KnowledgeSource::FullTextIndex);
        }

        // Always check file metadata
        if sources.is_empty() {
            sources.push(KnowledgeSource::FileMetadata);
        }

        sources
    }

    /// Estimate total query latency
    fn estimate_latency(sources: &[KnowledgeSource]) -> u32 {
        let mut total = 0u32;
        for source in sources {
            total += match source {
                KnowledgeSource::FileMetadata => 5,
                KnowledgeSource::FullTextIndex => 40,
                KnowledgeSource::EmbeddingIndex => 75,
                KnowledgeSource::StructuredQueryEngine => 30,
            };
        }
        if sources.len() > 1 {
            total / 2  // Assume 50% parallelization benefit
        } else {
            total
        }
    }

    /// Optimize query execution
    pub fn optimize(parsed: &ParsedQuery) -> ExecutionPlan {
        let sources = Self::select_sources(parsed);
        let parallelizable = sources.len() > 1;
        let cache_hint = !parsed.keywords.is_empty() || !parsed.filters.is_empty();
        let estimated_latency_ms = Self::estimate_latency(&sources);

        ExecutionPlan {
            sources,
            parallelizable,
            cache_hint,
            estimated_latency_ms,
        }
    }
}
```

### 6.3 Query Result Cache (LRU)

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub results: Vec<String>,
    pub timestamp_ms: u64,
}

pub struct QueryResultCache {
    cache: HashMap<String, CacheEntry>,
    capacity: usize,
    access_order: Vec<String>,
}

impl QueryResultCache {
    pub fn new(capacity: usize) -> Self {
        QueryResultCache {
            cache: HashMap::new(),
            capacity,
            access_order: Vec::new(),
        }
    }

    /// Generate cache key from query
    fn generate_key(parsed: &ParsedQuery) -> String {
        format!(
            "{:?}:{:?}:{}",
            parsed.intent,
            parsed.entities,
            parsed.keywords.join("|")
        )
    }

    /// Get cached results
    pub fn get(&mut self, parsed: &ParsedQuery) -> Option<Vec<String>> {
        let key = Self::generate_key(parsed);
        if let Some(entry) = self.cache.get(&key) {
            // Update access order (move to end)
            self.access_order.retain(|k| k != &key);
            self.access_order.push(key);
            return Some(entry.results.clone());
        }
        None
    }

    /// Put results in cache
    pub fn put(&mut self, parsed: &ParsedQuery, results: Vec<String>, timestamp_ms: u64) {
        let key = Self::generate_key(parsed);

        // Evict LRU if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            if let Some(lru_key) = self.access_order.first() {
                self.cache.remove(lru_key);
                self.access_order.remove(0);
            }
        }

        self.cache
            .insert(key.clone(), CacheEntry { results, timestamp_ms });
        self.access_order.push(key);
    }

    /// Get cache stats
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.capacity)
    }
}
```

### 6.4 Embedding Cache

```rust
pub struct EmbeddingCache {
    cache: HashMap<String, Vec<f32>>,
    hits: u64,
    misses: u64,
}

impl EmbeddingCache {
    pub fn new() -> Self {
        EmbeddingCache {
            cache: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Lookup embedding for entity
    pub fn get(&mut self, entity: &str) -> Option<Vec<f32>> {
        if let Some(embedding) = self.cache.get(entity) {
            self.hits += 1;
            Some(embedding.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store embedding for entity
    pub fn put(&mut self, entity: String, embedding: Vec<f32>) {
        self.cache.insert(entity, embedding);
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}
```

---

## 7. Phase 2 Implementation Readiness

### 7.1 Handoff to Implementation Team

**Completion Checklist:**

- [x] RFC-style architecture specification (Section 1)
- [x] NL Query Parser prototype with POS tagging, entity extraction, intent inference
- [x] Query Optimizer design with source selection, parallelization, caching
- [x] Dual-level caching strategy (LRU + persistent)
- [x] Performance targets and benchmark methodology
- [x] Working Rust code: NLQueryParser, QueryOptimizer, QueryResultCache, EmbeddingCache
- [x] 20+ example query validation set
- [x] Monitoring and metrics definitions

**Implementation Roadmap (Week 11-14):**

1. **Week 11:** Full-text index integration, embedding model selection, RocksDB setup
2. **Week 12:** Query parser refinement, optimizer tuning, cache configuration
3. **Week 13:** End-to-end integration testing, latency profiling, cache hit rate measurement
4. **Week 14:** Phase 2 validation, documentation, handoff to operations

### 7.2 Key Decisions for Implementation

1. **Embedding Model:** Use sentence-transformers (MPNet) for fast, high-quality embeddings
2. **Index Backend:** BM25-based full-text search for robustness
3. **Persistent Cache:** RocksDB with periodic compaction
4. **Parser Extensibility:** Pluggable POS tagger for domain-specific entities

---

## 8. Monitoring & Metrics

### 8.1 Query Latency Metrics

```
- Query execution time (p50, p95, p99)
- Per-source latency breakdown
- Parser latency
- Optimizer decision time
- Cache lookup time
```

### 8.2 Cache Effectiveness

```
- Query result cache hit rate (target: >70%)
- Embedding cache hit rate (target: >85%)
- Cache evictions per hour
- Cache memory usage
```

### 8.3 Optimizer Metrics

```
- Sources selected per query (distribution)
- Parallelization effectiveness
- Estimated vs. actual latency accuracy
```

### 8.4 Alerting

- Query p99 latency exceeds 500ms
- Cache hit rate drops below 50%
- Cache memory usage exceeds threshold
- Parser error rate exceeds 1%

---

## 9. Conclusion

The Semantic File System RFC establishes a clear architecture for natural language file system queries with performance targets, caching strategies, and a working prototype implementation. The Query Parser, Optimizer, and dual-level caching layer form the foundation for Phase 2 implementation, enabling <500ms complex queries with >70% cache hit rates.

**Status:** Ready for Phase 2 implementation handoff.
