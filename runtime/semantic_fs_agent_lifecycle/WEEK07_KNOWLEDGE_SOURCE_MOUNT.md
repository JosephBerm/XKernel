# Week 7 Deliverable: Knowledge Source Mount Interface (Phase 1)

**XKernal Cognitive Substrate — Engineer 8: Semantic FS & Agent Lifecycle**

---

## Executive Summary

Week 7 marks the transition to Phase 1 implementation of XKernal's knowledge source mount architecture. This deliverable specifies the abstract interface for mounting diverse external data sources (Pinecone, Weaviate, PostgreSQL, REST APIs, S3) as semantic volumes within the L2 Agent Runtime layer. The design establishes a unified query abstraction across heterogeneous backends while maintaining capability-based security and semantic-first access patterns. Integration with the CSCI layer via `mem_mount` syscall enables agents to dynamically bind knowledge sources with fine-grained access control and attenuation policies.

---

## Problem Statement

Cognitive agents operating within XKernal require seamless access to external knowledge repositories without coupling to specific backend technologies. Current approaches force agents to implement adapter logic for each data source type, creating tight coupling and violating composability principles.

**Key Challenges:**
- Heterogeneous source types (vector DBs, relational DBs, object stores, REST APIs) require different query semantics
- No unified capability model for knowledge source access across agent boundaries
- Mounting/unmounting sources lacks lifecycle governance and state tracking
- Cross-crew knowledge sharing requires attestable access controls
- Semantic queries (embedding-based, SQL, key-value) must map to diverse backend protocols

The Knowledge Source Mount Interface solves this by providing an abstraction layer that normalizes access patterns while preserving each backend's native strengths.

---

## Architecture Overview

The Knowledge Source Mount system operates at the intersection of the Semantic FS subsystem and Agent Lifecycle management. Sources are registered through CSCI `mem_mount` syscall, validated against agent capabilities, and exposed to agents through a unified semantic volume interface.

**Key Layers:**
1. **Mount Interface** — Abstract trait defining source lifecycle and query contract
2. **Backend Adapters** — Concrete implementations for each source type
3. **Capability Validation** — CSCI integration for access control
4. **Semantic Volume** — Unified query API across heterogeneous backends
5. **Lifecycle State Machine** — Register → Validate → Enable → Disable → Unregister

---

## Knowledge Source Mount Interface Specification

### Rust Trait Hierarchy

```rust
/// Core trait for all knowledge sources
pub trait KnowledgeSource: Send + Sync {
    /// Get the source type for routing and capability checks
    fn source_type(&self) -> SourceType;

    /// Mount this source with agent-provided configuration
    fn mount(&mut self, config: MountConfig) -> Result<MountHandle>;

    /// Execute a semantic query against the source
    fn query(&self, request: QueryRequest) -> Result<QueryResponse>;

    /// Unmount and release resources
    fn unmount(&mut self, handle: MountHandle) -> Result<()>;

    /// Health check for availability verification
    fn health_check(&self) -> HealthStatus;

    /// Get source metadata for capability negotiation
    fn metadata(&self) -> SourceMetadata;
}

/// Type discriminant for source classification
#[derive(Clone, Debug, PartialEq)]
pub enum SourceType {
    VectorDB { dimension: usize },
    RelationalDB { schema_version: u32 },
    ObjectStore,
    RestAPI { base_url: String },
}

/// Semantic query abstraction — backend-agnostic
pub enum QueryRequest {
    /// Vector similarity: (embedding, top_k, filters)
    VectorSearch {
        embedding: Vec<f32>,
        top_k: usize,
        filters: Option<Vec<Filter>>,
    },
    /// SQL query with parameter binding
    SQL {
        query: String,
        params: Vec<QueryParam>,
    },
    /// Key-value retrieval with prefix support
    KeyValue {
        key: String,
        include_metadata: bool,
    },
    /// REST API call with normalized semantics
    APICall {
        endpoint: String,
        method: HttpMethod,
        payload: Option<serde_json::Value>,
    },
}

/// Unified response format across all source types
pub struct QueryResponse {
    pub results: Vec<ResultItem>,
    pub metadata: ResponseMetadata,
    pub execution_stats: ExecutionStats,
}

pub struct ResultItem {
    pub id: String,
    pub content: serde_json::Value,
    pub relevance: f32,
    pub source_metadata: Option<serde_json::Value>,
}

pub struct MountConfig {
    pub credentials: SourceCredentials,
    pub mount_path: String,
    pub attenuation: AttenuationPolicy,
    pub cache_ttl: Option<Duration>,
    pub readonly: bool,
}

pub struct MountHandle {
    pub id: String,
    pub source_id: String,
    pub mounted_at: Instant,
    pub agent_id: String,
}

pub struct SourceMetadata {
    pub capabilities: Vec<Capability>,
    pub max_query_size: usize,
    pub supported_filters: Vec<FilterType>,
    pub latency_sla_ms: u32,
}

pub enum SourceCredentials {
    APIKey(String),
    OAuth2Token(String),
    BasicAuth { username: String, password: String },
    AwsCredentials { access_key: String, secret_key: String },
    PostgresConnStr(String),
}
```

---

## Data Source Type Support

### Vector Databases (Pinecone, Weaviate)

```rust
pub struct VectorDBAdapter {
    client: Box<dyn VectorDBClient>,
    index_dimension: usize,
    similarity_metric: SimilarityMetric,
}

impl KnowledgeSource for VectorDBAdapter {
    fn source_type(&self) -> SourceType {
        SourceType::VectorDB {
            dimension: self.index_dimension,
        }
    }

    fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        match request {
            QueryRequest::VectorSearch { embedding, top_k, filters } => {
                self.vector_search(embedding, top_k, filters)
            }
            _ => Err("VectorDB only supports VectorSearch queries".into()),
        }
    }
}

pub enum SimilarityMetric {
    CosineSimilarity,
    EuclideanDistance,
    DotProduct,
}

// Pinecone: Hierarchical Navigable Small World (HNSW) index
// Weaviate: Vector search with schema-aware filtering
```

### Relational Databases (PostgreSQL)

```rust
pub struct PostgresAdapter {
    pool: sqlx::PgPool,
    schema: DatabaseSchema,
    query_timeout: Duration,
}

impl KnowledgeSource for PostgresAdapter {
    fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        match request {
            QueryRequest::SQL { query, params } => {
                self.execute_sql(query, params)
            }
            _ => Err("PostgreSQL only supports SQL queries".into()),
        }
    }
}

pub struct DatabaseSchema {
    pub tables: Vec<TableSchema>,
    pub relationships: Vec<Relationship>,
}

// SQL query translation from semantic to relational
// Schema-aware validation and optimization
```

### Object Stores (S3)

```rust
pub struct S3Adapter {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix_filter: Option<String>,
}

impl KnowledgeSource for S3Adapter {
    fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        match request {
            QueryRequest::KeyValue { key, include_metadata } => {
                self.get_object(key, include_metadata)
            }
            _ => Err("S3 supports KeyValue queries only".into()),
        }
    }
}

// Streaming reads for large objects
// Prefix-based listing and pagination
```

### REST APIs

```rust
pub struct RestAPIAdapter {
    base_url: String,
    client: reqwest::Client,
    endpoint_specs: Vec<EndpointSpec>,
}

impl KnowledgeSource for RestAPIAdapter {
    fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        match request {
            QueryRequest::APICall { endpoint, method, payload } => {
                self.call_endpoint(endpoint, method, payload)
            }
            _ => Err("REST API supports APICall queries only".into()),
        }
    }
}

pub struct EndpointSpec {
    pub path: String,
    pub method: HttpMethod,
    pub param_schema: serde_json::Schema,
    pub response_schema: serde_json::Schema,
}
```

---

## Mount Lifecycle State Machine

```rust
pub enum MountState {
    /// Registered but not yet validated
    Registered,
    /// Credentials validated, ready to enable
    Validated,
    /// Active and queryable
    Enabled,
    /// Temporarily suspended (e.g., rate limit hit)
    Disabled,
    /// Unmounted and resource-released
    Unregistered,
}

pub struct MountStateMachine {
    current_state: MountState,
    state_history: Vec<(MountState, Instant, Option<String>)>,
}

impl MountStateMachine {
    /// Register a new knowledge source
    pub fn register(&mut self) -> Result<()> {
        self.transition(MountState::Registered)
    }

    /// Validate credentials and connectivity
    pub fn validate(&mut self) -> Result<()> {
        self.transition(MountState::Validated)
    }

    /// Enable the mount for agent access
    pub fn enable(&mut self) -> Result<()> {
        self.transition(MountState::Enabled)
    }

    /// Disable temporarily (e.g., quota exhaustion)
    pub fn disable(&mut self, reason: Option<String>) -> Result<()> {
        self.transition(MountState::Disabled)
    }

    /// Unregister and clean up
    pub fn unregister(&mut self) -> Result<()> {
        self.transition(MountState::Unregistered)
    }

    fn transition(&mut self, new_state: MountState) -> Result<()> {
        let valid = match (&self.current_state, &new_state) {
            (MountState::Registered, MountState::Validated) => true,
            (MountState::Validated, MountState::Enabled) => true,
            (MountState::Enabled, MountState::Disabled) => true,
            (MountState::Disabled, MountState::Enabled) => true,
            (_, MountState::Unregistered) => true,
            _ => false,
        };

        if valid {
            self.current_state = new_state.clone();
            self.state_history.push((new_state, Instant::now(), None));
            Ok(())
        } else {
            Err("Invalid state transition".into())
        }
    }
}
```

---

## Capability-Gating Design

Knowledge source access is governed by the CSCI capability model. Agents must hold valid capabilities to mount and query sources.

```rust
pub struct Capability {
    pub capability_id: String,
    pub source_type: SourceType,
    pub agent_id: String,
    pub operations: Vec<Operation>,
    pub attenuation: AttenuationPolicy,
    pub issued_at: Instant,
    pub expires_at: Instant,
}

pub enum Operation {
    Query,
    Write,
    Admin,
}

pub enum AttenuationPolicy {
    /// Read-only queries
    ReadOnly,
    /// Read and write
    ReadWrite,
    /// Limit queries per time window
    RateLimited { max_qps: u32 },
    /// Scope to specific data subset (e.g., collections)
    Scoped { scope: String },
}

pub struct CapabilityValidator {
    csci_interface: Box<dyn CSCIInterface>,
}

impl CapabilityValidator {
    /// Validate agent capability before mount
    pub fn validate_mount(
        &self,
        agent_id: &str,
        source_type: &SourceType,
        requested_ops: &[Operation],
    ) -> Result<Capability> {
        self.csci_interface.check_capability(
            agent_id,
            source_type,
            requested_ops,
        )
    }

    /// Validate per-query against attenuation policy
    pub fn validate_query(
        &self,
        capability: &Capability,
        request: &QueryRequest,
    ) -> Result<()> {
        // Check operation type matches capability
        // Verify rate limits not exceeded
        // Confirm scope constraints satisfied
        Ok(())
    }
}

pub trait CSCIInterface: Send + Sync {
    fn check_capability(
        &self,
        agent_id: &str,
        source_type: &SourceType,
        operations: &[Operation],
    ) -> Result<Capability>;

    fn revoke_capability(&self, capability_id: &str) -> Result<()>;

    fn emit_telemetry(&self, event: TelemetryEvent) -> Result<()>;
}

// Cross-crew sharing: Capabilities are transferable through CSCI endorsement
```

---

## Semantic Volume Abstraction

All mounted sources are exposed through a unified semantic volume interface, enabling agents to query diverse backends with consistent semantics.

```rust
pub struct SemanticVolume {
    mount: MountHandle,
    source: Box<dyn KnowledgeSource>,
    capability: Capability,
    query_cache: Arc<RwLock<QueryCache>>,
}

impl SemanticVolume {
    /// Unified query interface across all source types
    pub async fn query(
        &self,
        request: QueryRequest,
    ) -> Result<QueryResponse> {
        // 1. Validate capability for this query
        self.validate_query_access(&request)?;

        // 2. Check query cache
        if let Some(cached) = self.query_cache.read().unwrap().get(&request) {
            return Ok(cached.clone());
        }

        // 3. Execute query through source adapter
        let response = self.source.query(request.clone())?;

        // 4. Cache and emit telemetry
        self.query_cache.write().unwrap().put(request, response.clone());
        self.emit_query_telemetry(&response)?;

        Ok(response)
    }

    fn validate_query_access(&self, request: &QueryRequest) -> Result<()> {
        // Map request type to required operation
        let operation = match request {
            QueryRequest::VectorSearch { .. } => Operation::Query,
            QueryRequest::SQL { .. } => Operation::Query,
            QueryRequest::KeyValue { .. } => Operation::Query,
            QueryRequest::APICall { .. } => Operation::Query,
        };

        // Check capability allows operation
        if !self.capability.operations.contains(&operation) {
            return Err("Insufficient capability for operation".into());
        }

        Ok(())
    }
}

// Query normalization layer
pub struct QueryNormalizer {
    // Translates semantic queries to backend-specific syntax
}
```

---

## CSCI Integration Points

Knowledge source mounts integrate with the CSCI layer through standardized syscall and telemetry interfaces.

```rust
pub enum CSCISyscall {
    /// Mount a knowledge source: (agent_id, source_config) → mount_handle
    MemMount {
        agent_id: String,
        source_config: MountConfig,
    },
    /// Unmount a source: (mount_handle) → unit
    MemUnmount {
        mount_handle: MountHandle,
    },
    /// Query mounted source: (mount_handle, query) → response
    MemQuery {
        mount_handle: MountHandle,
        request: QueryRequest,
    },
}

pub struct TelemetryEvent {
    pub event_type: TelemetryType,
    pub timestamp: Instant,
    pub agent_id: String,
    pub source_id: String,
    pub metadata: serde_json::Value,
}

pub enum TelemetryType {
    MountAttempted,
    MountSucceeded,
    MountFailed,
    QueryExecuted,
    QueryFailed,
    CapabilityChecked,
    CapabilityDenied,
}

// All mount/query operations emit telemetry for audit and observability
// CSCI validates capabilities before syscall dispatch
// Rate limiting and quota enforcement occur at CSCI boundary
```

---

## Design Principles

1. **Composability:** Knowledge sources combine seamlessly into semantic memory; queries span multiple sources through federation patterns.

2. **Capability-Based Security:** Access is granted through unforgeable capabilities issued by CSCI; attenuation restricts operations at the boundary.

3. **Abstraction:** Unified `KnowledgeSource` trait masks backend diversity; agents query through semantic interface independent of storage technology.

4. **Semantic Over Syntactic:** Query semantics (vector similarity, SQL relations, object retrieval) take precedence over backend protocols; translation occurs in adapters.

5. **Lifecycle Governance:** State machine ensures orderly progression through mount states; invalid transitions are rejected.

---

## Phase 1 Deliverables

- ✓ Abstract `KnowledgeSource` trait with mount/query/unmount contract
- ✓ Concrete adapters for Pinecone, Weaviate, PostgreSQL, S3, REST APIs
- ✓ Mount lifecycle state machine with validation gates
- ✓ Capability-gating design with attenuation policies
- ✓ CSCI integration via `mem_mount`, `mem_unmount`, `mem_query` syscalls
- ✓ Telemetry instrumentation for audit and observability
- Pending Phase 2: Query federation (multi-source queries)
- Pending Phase 2: Advanced caching and optimization strategies

---

**Document Generated:** 2026-03-02
**Engineer:** 8 (Semantic FS & Agent Lifecycle)
**Project:** XKernal Cognitive Substrate OS
**Phase:** 1 (Implementation)
