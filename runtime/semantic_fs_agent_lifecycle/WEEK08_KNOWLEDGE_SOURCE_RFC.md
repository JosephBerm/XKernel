# Week 8 Deliverable: Knowledge Source Mount Interface RFC (Phase 1)

**Engineer 8: Runtime Semantic FS & Agent Lifecycle**
**XKernal Project — Semantic Filesystem Architecture**
**Prepared:** 2026-03-02
**Phase:** 1 (RFC & Specification)

---

## Executive Summary

This RFC formalizes the Knowledge Source mount interface for XKernal's semantic filesystem. It defines the complete lifecycle for connecting diverse data sources (vector DBs, relational databases, REST APIs, object stores) to the runtime through a unified mount abstraction. This specification is the foundation for Phase 2 implementation.

**Scope:** Query protocols, authentication mechanisms, error handling, and reference architecture for source integration.

---

## 1. RFC-Style Specification: Mount Interface Formalism

### 1.1 Core Abstractions

```
MOUNT = {
  id: String,                          // Unique mount identifier
  source_type: SourceType,             // VECTOR_DB | RELATIONAL | REST_API | OBJECT_STORE
  location: String,                    // Endpoint/connection string
  namespace: String,                   // Virtual namespace in semantic FS
  metadata: {
    created_at: Timestamp,
    last_verified: Timestamp,
    health_status: HealthStatus,
    capabilities: Set<Capability>
  }
}

SourceType ∈ {VECTOR_DB, RELATIONAL, REST_API, OBJECT_STORE}

Capability ∈ {
  SEMANTIC_SEARCH,     // Vector similarity + keyword
  RELATIONAL_QUERY,    // SQL-like structured queries
  STREAMING,           // Long-running result streaming
  TRANSACTIONS,        // ACID compliance
  FILTERING,           // Predicate pushdown
  AGGREGATION,         // Native aggregations
  AUTH_REQUIRED        // Auth enforcement
}
```

### 1.2 Formal Mount Lifecycle

```
State Transitions:
  UNINITIALIZED → CONNECTING → CONNECTED → VERIFIED → [ACTIVE | DEGRADED | UNHEALTHY] → DISCONNECTING → DISCONNECTED

UNINITIALIZED: Mount definition exists, not yet contacted
CONNECTING: Authentication in progress, connection negotiation
CONNECTED: Network connected, capabilities discovered
VERIFIED: Health check passed, ready for queries
ACTIVE: Normal operation, <5% error rate
DEGRADED: Elevated latency or partial failures, <10% error rate
UNHEALTHY: >10% error rate or repeated timeouts
DISCONNECTING: Graceful shutdown initiated
DISCONNECTED: Mount unavailable
```

---

## 2. Query Protocol Specification

### 2.1 Unified Query Interface

```rust
// Core trait: All sources implement this interface
pub trait QueryProtocol: Send + Sync {
  /// Execute typed query and return results
  async fn execute_query(
    &self,
    query: QueryRequest,
    timeout_ms: u64,
  ) -> Result<QueryResponse, QueryError>;

  /// Stream large result sets without buffering in memory
  async fn stream_query(
    &self,
    query: QueryRequest,
    timeout_ms: u64,
  ) -> Result<Box<dyn Stream<Item = QueryRow> + Send>, QueryError>;

  /// Validate query syntax and estimated cost before execution
  async fn explain_query(
    &self,
    query: QueryRequest,
  ) -> Result<QueryExplain, QueryError>;
}

pub struct QueryRequest {
  pub query_type: QueryType,
  pub source: String,              // Mount ID
  pub timeout_ms: u64,
  pub max_results: Option<usize>,
  pub cache_hint: CachePolicy,
  pub credentials_token: Option<String>,
}

pub enum QueryType {
  // Vector database: semantic similarity
  VectorSearch {
    embedding: Vec<f32>,
    top_k: usize,
    distance_metric: DistanceMetric,
    filter: Option<String>,          // Optional metadata filter
  },

  // Relational: SQL subset
  RelationalQuery {
    sql: String,                     // SELECT, WHERE, JOIN, GROUP BY
    params: Vec<QueryValue>,
  },

  // REST API: structured request
  RestCall {
    method: String,                  // GET, POST
    path: String,
    query_params: Map<String, String>,
    body: Option<String>,
  },

  // Object store: path enumeration/retrieval
  ObjectQuery {
    prefix: String,
    recursive: bool,
    metadata_only: bool,
  },

  // Hybrid: semantic + relational
  HybridQuery {
    embedding: Vec<f32>,
    top_k: usize,
    sql_filter: Option<String>,      // Optional SQL WHERE clause
    boost_by_recency: bool,
  },
}

pub enum DistanceMetric {
  Cosine,
  EuclideanL2,
  DotProduct,
  Hamming,
}

pub struct QueryResponse {
  pub rows: Vec<QueryRow>,
  pub metadata: ResponseMetadata,
  pub total_latency_ms: u64,
  pub returned_results: usize,
  pub estimated_total: Option<usize>,
}

pub struct ResponseMetadata {
  pub query_id: String,              // For audit trail
  pub source_latency_ms: u64,
  pub mount_processing_ms: u64,
  pub cache_hit: bool,
  pub data_freshness_seconds: Option<u64>,
}

pub struct QueryExplain {
  pub estimated_cost_tokens: u64,
  pub estimated_latency_ms: u64,
  pub rows_scanned_estimate: usize,
  pub indexes_used: Vec<String>,
  pub warnings: Vec<String>,
}

pub enum QueryError {
  InvalidQuery(String),              // Syntax error, wrong type
  SourceUnavailable(String),         // Connection failed, mount down
  Unauthorized(String),              // Auth failure, insufficient permissions
  Timeout(u64),                      // Exceeded timeout_ms
  QuotaExceeded(String),             // Rate limiting, quota exhausted
  InternalError(String),             // Source or mount bug
  PartialFailure {                   // Partial result available
    rows: Vec<QueryRow>,
    error: String,
  },
}
```

### 2.2 Source-Specific Query Examples

**Vector DB (Pinecone/Weaviate):**
```rust
// Semantic search with metadata filtering
QueryRequest {
  query_type: QueryType::VectorSearch {
    embedding: vec![0.1, 0.2, ..., 0.9],  // 768-dim
    top_k: 10,
    distance_metric: DistanceMetric::Cosine,
    filter: Some("metadata.category = 'enterprise' AND created_at > 2026-01-01"),
  },
  source: "mount:pinecone-prod",
  timeout_ms: 2000,
  max_results: Some(10),
  cache_hint: CachePolicy::Prefer(300),
  credentials_token: Some("pk-xxx"),
}
```

**Relational (PostgreSQL):**
```rust
// Structured query with pushdown
QueryRequest {
  query_type: QueryType::RelationalQuery {
    sql: "SELECT id, name, score FROM users WHERE age > ? AND status = ? LIMIT 50",
    params: vec![QueryValue::Int(25), QueryValue::String("active")],
  },
  source: "mount:postgres-analytics",
  timeout_ms: 5000,
  max_results: None,
  cache_hint: CachePolicy::NoCache,
  credentials_token: Some("role:analyst"),
}
```

**Hybrid (Vector + Relational):**
```rust
// Semantic search constrained by SQL predicate
QueryRequest {
  query_type: QueryType::HybridQuery {
    embedding: vec![...],
    top_k: 20,
    sql_filter: Some("price < 100 AND in_stock = true AND region = 'US'"),
    boost_by_recency: true,
  },
  source: "mount:product-catalog",
  timeout_ms: 3000,
  max_results: Some(20),
  cache_hint: CachePolicy::Prefer(600),
  credentials_token: Some("session-xyz"),
}
```

---

## 3. Authentication & Credential Management

### 3.1 Authentication Provider Trait

```rust
pub trait AuthProvider: Send + Sync {
  /// Issue capability-based token valid for duration
  async fn issue_token(
    &self,
    principal: String,
    scope: AuthScope,
    ttl_seconds: u64,
  ) -> Result<AuthToken, AuthError>;

  /// Verify token integrity and extract claims
  async fn verify_token(
    &self,
    token: &str,
  ) -> Result<TokenClaims, AuthError>;

  /// Rotate credentials (supports key rotation without downtime)
  async fn rotate_credentials(
    &self,
    old_secret: String,
    new_secret: String,
  ) -> Result<RotationResult, AuthError>;

  /// Check if principal has capability
  async fn authorize(
    &self,
    principal: String,
    capability: &str,
    resource: &str,
  ) -> Result<bool, AuthError>;
}

pub struct AuthToken {
  pub token: String,
  pub token_type: String,            // "Bearer", "Basic", "ApiKey"
  pub expires_at: Timestamp,
  pub claims: TokenClaims,
}

pub struct TokenClaims {
  pub sub: String,                   // Subject (principal ID)
  pub iss: String,                   // Issuer (mount ID)
  pub aud: Vec<String>,              // Audience (allowed operations)
  pub scope: AuthScope,
  pub exp: u64,                      // Expiration timestamp
  pub iat: u64,                      // Issued-at timestamp
}

pub enum AuthScope {
  ReadOnly,                          // SELECT, GET operations
  ReadWrite,                         // INSERT, UPDATE, DELETE
  Admin,                             // Full control
  Custom(Vec<String>),               // Fine-grained capabilities
}

pub struct RotationResult {
  pub old_key_revoked_at: Timestamp,
  pub new_key_valid_from: Timestamp,
  pub grace_period_seconds: u64,
}

pub enum AuthError {
  InvalidCredentials,
  TokenExpired,
  InsufficientPermissions,
  RotationInProgress,
  InvalidScope,
}

// Per-source auth configuration
pub struct AuthConfig {
  pub mechanism: AuthMechanism,
  pub provider: Box<dyn AuthProvider>,
  pub credential_storage: CredentialStorage,
  pub token_cache_ttl_seconds: u64,
  pub refresh_threshold_seconds: u64,  // Refresh token 60s before expiry
}

pub enum AuthMechanism {
  ApiKey(String),                    // Static API key
  OAuth2(OAuth2Config),
  BasicAuth(String, String),         // Username, password
  MutualTls(TlsConfig),
  ServiceAccount(ServiceAccountKey),
}

pub enum CredentialStorage {
  Memory,                            // Ephemeral, no persistence
  EnvironmentVariable(String),       // ENV variable name
  SecretManager(String),             // e.g., AWS Secrets Manager
  VaultKv(VaultConfig),              // HashiCorp Vault
}

pub struct VaultConfig {
  pub addr: String,
  pub engine: String,
  pub path: String,
  pub auth_method: String,
}
```

### 3.2 Credential Lifecycle

```
Issue: principal requests token for operation
  ↓
Authenticate: verify principal credentials
  ↓
Authorize: check principal.capability on resource
  ↓
Token Generation: create JWT with scope + TTL
  ↓
Cache: store in local LRU cache (if enabled)
  ↓
Use: include in QueryRequest.credentials_token
  ↓
Refresh: monitor TTL, auto-refresh before expiry
  ↓
Revoke: immediate invalidation on logout or rotation
```

---

## 4. Error Handling & Fault Tolerance

### 4.1 Error Classification & Retry Policy

```rust
pub trait ErrorHandler: Send + Sync {
  /// Determine if error is retryable and suggest backoff
  fn classify_error(&self, error: &QueryError) -> ErrorClassification;

  /// Execute query with automatic retry and exponential backoff
  async fn execute_with_retry(
    &self,
    query: QueryRequest,
    retry_config: RetryConfig,
  ) -> Result<QueryResponse, QueryError>;
}

pub struct ErrorClassification {
  pub category: ErrorCategory,
  pub is_retryable: bool,
  pub suggested_backoff_ms: u64,
  pub should_cascade: bool,          // Failover to replica
}

pub enum ErrorCategory {
  ClientError,                       // 4xx: Invalid query, auth failure
  TransientFailure,                 // Temporary: timeout, connection reset
  PermanentFailure,                 // 5xx: Service down, quota exceeded
  PartialFailure,                   // Partial results available
  UnknownError,                      // Should not proceed
}

pub struct RetryConfig {
  pub max_attempts: u32,
  pub initial_backoff_ms: u64,
  pub max_backoff_ms: u64,
  pub backoff_multiplier: f64,       // Usually 2.0
  pub jitter_enabled: bool,
  pub retryable_errors: Vec<String>,
}

// Circuit breaker pattern
pub struct CircuitBreaker {
  pub failure_threshold: u32,        // Failures before opening
  pub success_threshold: u32,        // Successes before closing
  pub timeout_seconds: u64,          // Duration of open state
  pub state: CircuitState,
}

pub enum CircuitState {
  Closed,                            // Normal operation
  Open,                              // Reject requests
  HalfOpen,                          // Allow test requests
}

impl CircuitBreaker {
  pub async fn execute<F, T>(
    &mut self,
    operation: F,
  ) -> Result<T, QueryError>
  where
    F: std::future::Future<Output = Result<T, QueryError>>,
  {
    match self.state {
      CircuitState::Closed => operation.await,
      CircuitState::Open => {
        if self.timeout_expired() {
          self.state = CircuitState::HalfOpen;
          operation.await
        } else {
          Err(QueryError::SourceUnavailable("Circuit open".to_string()))
        }
      },
      CircuitState::HalfOpen => {
        match operation.await {
          Ok(result) => {
            self.success_count += 1;
            if self.success_count >= self.success_threshold {
              self.state = CircuitState::Closed;
              self.failure_count = 0;
            }
            Ok(result)
          },
          Err(e) => {
            self.failure_count += 1;
            if self.failure_count >= self.failure_threshold {
              self.state = CircuitState::Open;
            }
            Err(e)
          }
        }
      }
    }
  }
}

// Fault tolerance strategies
pub enum FallbackStrategy {
  None,
  ReplicaMountId(String),            // Switch to replica mount
  CachedResult(u64),                 // Serve stale data if available (seconds)
  DegradedMode {                     // Reduced functionality
    max_results: usize,
    timeout_ms: u64,
  },
}
```

### 4.2 Timeout & Unavailability Handling

```rust
pub struct MountHealthChecker {
  pub mount_id: String,
  pub check_interval_seconds: u64,
  pub consecutive_failures_threshold: u32,
  pub circuit_breaker: CircuitBreaker,
}

impl MountHealthChecker {
  /// Periodic health check (runs every check_interval_seconds)
  pub async fn health_check(&mut self) -> HealthStatus {
    match tokio::time::timeout(
      Duration::from_secs(5),
      self.probe_endpoint(),
    ).await {
      Ok(Ok(_)) => {
        self.consecutive_failures = 0;
        HealthStatus::Healthy
      },
      Ok(Err(_)) => {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.consecutive_failures_threshold {
          HealthStatus::Unhealthy
        } else {
          HealthStatus::Degraded
        }
      },
      Err(_) => {
        self.consecutive_failures += 1;
        HealthStatus::Unhealthy
      }
    }
  }

  async fn probe_endpoint(&self) -> Result<(), String> {
    // Lightweight ping: connectivity + latency check
    // Does not execute actual queries
    todo!()
  }
}

pub enum HealthStatus {
  Healthy,                           // <50ms latency, 0% error rate
  Degraded,                          // 50-200ms latency, <5% error rate
  Unhealthy,                         // >200ms latency, >10% error rate
  Disconnected,                      // No connectivity
}

// Timeout handling
pub const DEFAULT_QUERY_TIMEOUT_MS: u64 = 5000;
pub const MAX_QUERY_TIMEOUT_MS: u64 = 60000;

pub async fn execute_with_timeout(
  query_future: impl std::future::Future<Output = Result<QueryResponse, QueryError>>,
  timeout_ms: u64,
) -> Result<QueryResponse, QueryError> {
  let clamped_timeout = timeout_ms.min(MAX_QUERY_TIMEOUT_MS);
  match tokio::time::timeout(
    Duration::from_millis(clamped_timeout),
    query_future,
  ).await {
    Ok(result) => result,
    Err(_) => Err(QueryError::Timeout(clamped_timeout)),
  }
}
```

---

## 5. Reference Architecture Diagrams

### 5.1 Source → Mount → CSCI Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent Lifecycle Manager                       │
│                   (Request Orchestration)                        │
└────────────┬────────────────────────────────────────────────────┘
             │ QueryRequest with credentials_token
             ↓
┌─────────────────────────────────────────────────────────────────┐
│               Semantic Filesystem Router (CSCI)                  │
│  • Resolves mount from namespace query                           │
│  • Enforces capability-based access control                      │
│  • Applies query transformations (optimization)                  │
│  • Manages result caching layer                                  │
└────────────┬────────────────────────────────────────────────────┘
             │ Routed QueryRequest
             ↓
   ┌─────────────────────────────────────────┐
   │    Mount Abstraction Layer (LAYER)      │
   │                                         │
   │ ┌──────────────────────────────────┐   │
   │ │  QueryProtocol Implementation    │   │
   │ │  • Type-specific query adapters  │   │
   │ │  • Connection pooling            │   │
   │ │  • Circuit breaker               │   │
   │ └──────────────────────────────────┘   │
   │                                         │
   │ ┌──────────────────────────────────┐   │
   │ │  AuthProvider Implementation      │   │
   │ │  • Token validation               │   │
   │ │  • Per-capability filtering       │   │
   │ │  • Credential rotation            │   │
   │ └──────────────────────────────────┘   │
   │                                         │
   │ ┌──────────────────────────────────┐   │
   │ │  MountHealthChecker               │   │
   │ │  • Periodic health probes         │   │
   │ │  • State transitions              │   │
   │ │  • Failover coordination          │   │
   │ └──────────────────────────────────┘   │
   └────────────┬────────────────────────────┘
                │ Source-specific Query
                ↓
   ┌─────────────────────────────────────────┐
   │        Data Source Layer                │
   │                                         │
   │  ┌────────────┐ ┌────────────┐         │
   │  │ Pinecone   │ │ Weaviate   │ ...     │
   │  │ (Vector)   │ │ (Vector)   │         │
   │  └────────────┘ └────────────┘         │
   │                                         │
   │  ┌────────────┐ ┌────────────┐         │
   │  │ PostgreSQL │ │ MySQL      │ ...     │
   │  │(Relational)│ │(Relational)│         │
   │  └────────────┘ └────────────┘         │
   │                                         │
   │  ┌────────────┐ ┌────────────┐         │
   │  │ REST API   │ │ S3         │ ...     │
   │  │ (HTTP)     │ │ (ObjectSt.)│         │
   │  └────────────┘ └────────────┘         │
   └─────────────────────────────────────────┘
```

### 5.2 Network Stack Integration

```
Application Layer
  │
  ├─ Query Execution Engine
  │   ├─ Query Planner
  │   └─ Result Aggregator
  │
  ├─ Cache Layer (Redis/Memcached)
  │   ├─ Query Result Cache
  │   └─ Token Cache
  │
Mount Layer
  │
  ├─ Connection Management
  │   ├─ TCP/HTTP pools
  │   └─ Reconnection logic
  │
  ├─ Protocol Translation
  │   ├─ Vector → REST
  │   ├─ SQL → REST (for REST sources)
  │   └─ Type conversions
  │
  ├─ Auth Enforcement
  │   ├─ Token validation
  │   └─ Capability checks
  │
  ├─ Error Recovery
  │   ├─ Retry logic
  │   ├─ Circuit breaker
  │   └─ Fallback dispatch
  │
Transport Layer
  │
  ├─ TLS 1.3
  ├─ HTTP/2 multiplexing
  ├─ Keep-alive management
  └─ Timeout enforcement

Source Layer
  │
  ├─ TCP connections
  ├─ Request/response serialization
  └─ Credentials
```

---

## 6. Data Source Coverage

### 6.1 Pinecone (Vector DB)

```rust
pub struct PineconeMount {
  endpoint: String,              // "api.pinecone.io"
  index_name: String,
  environment: String,
  api_key: String,
}

// Supported operations:
// - VectorSearch: similarity search with metadata filtering
// - Upsert: insert/update embeddings
// - Delete: remove by ID or filter
// - Hybrid Search: vector + metadata predicates

impl QueryProtocol for PineconeMount {
  async fn execute_query(&self, query: QueryRequest, _: u64) -> Result<QueryResponse, QueryError> {
    match query.query_type {
      QueryType::VectorSearch { embedding, top_k, filter, .. } => {
        // POST /query with embedding, top_k, filter
        // Returns: {matches: [{id, score, metadata, values}]}
        todo!()
      },
      _ => Err(QueryError::InvalidQuery("Pinecone only supports VectorSearch".into())),
    }
  }
}
```

### 6.2 Weaviate (Vector DB + Hybrid)

```rust
pub struct WeaviateMount {
  endpoint: String,              // "https://weaviate.example.com"
  auth_token: String,
}

// Supported operations:
// - VectorSearch: nearVector with semantic search
// - HybridQuery: combine vector + BM25 keyword search
// - Metadata filtering: complex WHERE clauses
// - GraphQL queries

impl QueryProtocol for WeaviateMount {
  async fn execute_query(&self, query: QueryRequest, _: u64) -> Result<QueryResponse, QueryError> {
    match query.query_type {
      QueryType::VectorSearch { embedding, top_k, filter, .. } => {
        // GraphQL: { Get { ClassName(nearVector: {...}, where: {...}) } }
        todo!()
      },
      QueryType::HybridQuery { embedding, top_k, sql_filter, .. } => {
        // HybridSearch with nearVector + bm25
        todo!()
      },
      _ => Err(QueryError::InvalidQuery("Unsupported for Weaviate".into())),
    }
  }
}
```

### 6.3 PostgreSQL (Relational)

```rust
pub struct PostgresMount {
  connection_string: String,
  pool_size: u32,
  ssl_mode: String,
}

// Supported operations:
// - RelationalQuery: full SQL SELECT with transactions
// - Aggregations: GROUP BY, ORDER BY, LIMIT
// - Joins: INNER, LEFT, RIGHT, FULL OUTER
// - Indexing: leverage native indexes

impl QueryProtocol for PostgresMount {
  async fn execute_query(&self, query: QueryRequest, timeout_ms: u64) -> Result<QueryResponse, QueryError> {
    match query.query_type {
      QueryType::RelationalQuery { sql, params } => {
        // Execute SQL with parameter binding (prepared statements)
        // Row results serialized to JSON
        todo!()
      },
      _ => Err(QueryError::InvalidQuery("PostgreSQL requires RelationalQuery".into())),
    }
  }
}
```

### 6.4 REST API (Generic)

```rust
pub struct RestApiMount {
  base_url: String,
  auth_header: String,             // e.g., "Authorization: Bearer token"
  rate_limit_per_second: u32,
}

// Supported operations:
// - RestCall: GET/POST to configured endpoints
// - Pagination: follow rel=next links
// - Response parsing: JSON, CSV, XML

impl QueryProtocol for RestApiMount {
  async fn execute_query(&self, query: QueryRequest, timeout_ms: u64) -> Result<QueryResponse, QueryError> {
    match query.query_type {
      QueryType::RestCall { method, path, query_params, body } => {
        // HTTP request with auth + timeout + retry logic
        // Parse response body as QueryRows
        todo!()
      },
      _ => Err(QueryError::InvalidQuery("REST mounts only support RestCall".into())),
    }
  }
}
```

### 6.5 S3 (Object Store)

```rust
pub struct S3Mount {
  bucket: String,
  region: String,
  aws_access_key: String,
  aws_secret_key: String,
}

// Supported operations:
// - ObjectQuery: list objects by prefix
// - Metadata: retrieve object tags, size, etag
// - Streaming: get object body with multipart download

impl QueryProtocol for S3Mount {
  async fn execute_query(&self, query: QueryRequest, timeout_ms: u64) -> Result<QueryResponse, QueryError> {
    match query.query_type {
      QueryType::ObjectQuery { prefix, recursive, metadata_only } => {
        // S3 ListObjectsV2 with prefix
        // Optional: recursively traverse directory structure
        // Returns: {objects: [{key, size, etag, last_modified}]}
        todo!()
      },
      _ => Err(QueryError::InvalidQuery("S3 only supports ObjectQuery".into())),
    }
  }
}
```

---

## 7. Implementation Readiness Checklist

### Phase 2 Implementation Tasks

- [ ] **QueryProtocol Implementation**
  - [ ] Pinecone adapter with gRPC + REST fallback
  - [ ] Weaviate GraphQL adapter
  - [ ] PostgreSQL async driver (sqlx) integration
  - [ ] Generic REST client with retry logic
  - [ ] S3 client (AWS SDK) integration
  - [ ] Query validation & optimization layer

- [ ] **AuthProvider Implementation**
  - [ ] In-memory token cache with LRU eviction
  - [ ] OAuth2 token exchange (authorization code flow)
  - [ ] API key management and rotation
  - [ ] Vault integration for secret storage
  - [ ] Per-capability authorization checks
  - [ ] Token refresh background task

- [ ] **Error Handling & Fault Tolerance**
  - [ ] Circuit breaker state machine
  - [ ] Exponential backoff with jitter
  - [ ] Timeout enforcement (tokio::time::timeout)
  - [ ] Partial failure handling and result merging
  - [ ] Fallback strategy dispatcher
  - [ ] Health check task (runs every 30s)

- [ ] **Mount Lifecycle Management**
  - [ ] State machine (UNINITIALIZED → DISCONNECTED)
  - [ ] Connection pooling (with keep-alive)
  - [ ] Credential rotation without downtime
  - [ ] Graceful degradation (circuit open)
  - [ ] Mount registration/deregistration
  - [ ] Metadata persistence (mount configs)

- [ ] **Testing & Validation**
  - [ ] Unit tests: each QueryProtocol impl
  - [ ] Integration tests: source connectivity
  - [ ] Fault injection: timeouts, auth failures, unavailable sources
  - [ ] Load testing: connection pool saturation
  - [ ] Chaos testing: random failures + circuit breaker recovery
  - [ ] Benchmark: query latency vs result size

- [ ] **Observability**
  - [ ] Metrics: query latency, error rates, cache hit %
  - [ ] Distributed tracing: mount → source call stacks
  - [ ] Health dashboard: per-mount status + circuit state
  - [ ] Audit logs: credentials issued, capabilities used
  - [ ] Alerts: high error rate, circuit breaker open

- [ ] **Documentation**
  - [ ] Mount configuration guide (YAML examples)
  - [ ] Query examples for each source type
  - [ ] Troubleshooting runbook
  - [ ] Performance tuning guide
  - [ ] Security best practices

---

## 8. Key Design Decisions

1. **Unified Query Interface:** All sources implement QueryProtocol, enabling polymorphic query execution and consistent error handling across diverse backends.

2. **Capability-Based Auth:** Fine-grained authorization tied to operations (SEMANTIC_SEARCH, RELATIONAL_QUERY) and resources (mount ID, database name), not just principals.

3. **Circuit Breaker Pattern:** Automatic fail-fast for cascading failures, with configurable thresholds for state transitions.

4. **Typed Query Requests:** QueryType enum ensures type safety; invalid operations rejected at plan time, not runtime.

5. **Async/Await Everywhere:** Non-blocking I/O enables high concurrency and responsive timeout enforcement.

6. **Pluggable AuthProvider:** Supports multiple auth mechanisms (API key, OAuth2, TLS) via trait, enabling per-source customization.

---

## 9. Next Steps

1. **Phase 2 Kick-off:** Engineer 8 leads implementation of QueryProtocol adapters for all 5 source types.
2. **Prototype:** Build reference implementation for Pinecone + PostgreSQL mounts.
3. **Integration Testing:** Validate mount lifecycle and error recovery scenarios.
4. **Performance Tuning:** Optimize connection pooling, timeout thresholds, cache TTLs.
5. **Readiness Review:** Acceptance criteria before Phase 2 completion.

---

**Document Status:** Ready for Phase 2 Implementation
**Last Updated:** 2026-03-02
**Next Review:** Post-Phase-2-Completion
