# Week 15: Knowledge Source Mounting & External Data Integration
## XKernal Cognitive Substrate OS - L1 Services Layer (Rust)

**Phase:** 2 (Week 1)
**Engineer Level:** Staff
**Status:** Technical Design Document
**Last Updated:** 2026-03-02

---

## 1. Overview & Objectives

Week 15 establishes the **Knowledge Source Abstraction Layer**, enabling XKernal's semantic memory to mount and query external data sources as virtualized semantic volumes. This layer bridges the 3-tier memory hierarchy (SRAM/DRAM/NVMe) with production data sources via pluggable drivers and capability-gated access control.

### Primary Deliverables
- **Core Abstraction:** `KnowledgeSource` trait with pluggable driver interface
- **6 Connectors:** Pinecone, Weaviate, PostgreSQL, REST API, S3, File-based vectors
- **Mount Registry:** CRDT-backed registry with atomic mount/unmount operations
- **Capability Gating:** Access control tied to CSCI context capability tokens
- **Performance Compliance:** Meet sub-second latency targets per connector type
- **Test Coverage:** Unit tests (6/6 connectors), integration tests, chaos tests

---

## 2. Design Principles

| Principle | Implementation |
|-----------|-----------------|
| **Extensibility** | Trait-based driver interface; zero coupling to implementation details |
| **Isolation** | Each connector runs in isolated thread pool; failure doesn't cascade |
| **Performance** | Connection pooling, batch operations, configurable cache layers |
| **Simplicity** | Minimal trait surface; composable middleware for cross-cutting concerns |
| **Security** | Capability tokens validated at every query boundary; credential isolation |

---

## 3. Core Abstraction: KnowledgeSource Trait

```rust
// File: src/knowledge_source/mod.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Query vector with optional filters and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub vector: Vec<f32>,
    pub top_k: usize,
    pub filters: HashMap<String, FilterValue>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    StringArray(Vec<String>),
}

/// Result from knowledge source query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, FilterValue>,
    pub source_id: String,
}

#[derive(Debug, Error)]
pub enum KnowledgeSourceError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Query timeout after {0}ms")]
    QueryTimeout(u64),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type KSResult<T> = Result<T, KnowledgeSourceError>;

/// Knowledge source configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceConfig {
    pub source_type: SourceType,
    pub endpoint: String,
    pub credentials: HashMap<String, String>,
    pub timeout_ms: u64,
    pub max_connections: usize,
    pub capability_rules: Vec<CapabilityRule>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SourceType {
    Pinecone,
    Weaviate,
    PostgreSQL,
    RestApi,
    S3,
    FileVectors,
}

/// Capability-based access control
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityRule {
    pub capability: String,
    pub allowed_operations: Vec<Operation>,
    pub rate_limit_per_minute: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Operation {
    Query,
    Search,
    Ingest,
    Update,
    Delete,
}

/// Primary trait for all knowledge source drivers
#[async_trait]
pub trait KnowledgeSourceDriver: Send + Sync {
    /// Verify connection and credentials
    async fn health_check(&self) -> KSResult<()>;

    /// Execute semantic vector query
    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>>;

    /// Batch query multiple vectors efficiently
    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>>;

    /// List all indexed documents (with pagination)
    async fn list_documents(
        &self,
        offset: usize,
        limit: usize,
        capability: &str,
    ) -> KSResult<Vec<String>>;

    /// Get source metadata
    fn config(&self) -> &SourceConfig;

    /// Human-readable status
    fn status(&self) -> SourceStatus;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceStatus {
    pub source_type: SourceType,
    pub is_healthy: bool,
    pub last_query_ms: u64,
    pub total_queries: u64,
    pub error_rate: f32,
}
```

---

## 4. Mount Point Registry & Lifecycle Management

```rust
// File: src/knowledge_source/registry.rs

use crate::crdt::CRDTMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// Thread-safe registry of mounted knowledge sources
pub struct MountRegistry {
    mounts: Arc<RwLock<CRDTMap<String, MountPoint>>>,
    driver_cache: Arc<RwLock<HashMap<String, Arc<dyn KnowledgeSourceDriver>>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountPoint {
    pub mount_id: String,
    pub name: String,
    pub source_type: SourceType,
    pub config: SourceConfig,
    pub mounted_at: u64,
    pub access_count: u64,
    pub last_accessed: u64,
    pub is_active: bool,
}

impl MountRegistry {
    pub fn new() -> Self {
        Self {
            mounts: Arc::new(RwLock::new(CRDTMap::new())),
            driver_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Mount a knowledge source and initialize driver
    pub async fn mount(&self, config: SourceConfig) -> KSResult<String> {
        let mount_id = uuid::Uuid::new_v4().to_string();

        // Initialize appropriate driver
        let driver = self.create_driver(&config).await?;

        // Health check before mounting
        driver.health_check().await?;

        // Register in CRDT
        let mount = MountPoint {
            mount_id: mount_id.clone(),
            name: format!("{:?}:{}", config.source_type, &config.endpoint),
            source_type: config.source_type.clone(),
            config,
            mounted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
            last_accessed: 0,
            is_active: true,
        };

        self.mounts.write().insert(mount_id.clone(), mount);
        self.driver_cache.write().insert(
            mount_id.clone(),
            Arc::new(driver),
        );

        Ok(mount_id)
    }

    /// Unmount and cleanup driver resources
    pub async fn unmount(&self, mount_id: &str) -> KSResult<()> {
        self.mounts.write().remove(mount_id);
        self.driver_cache.write().remove(mount_id);
        Ok(())
    }

    /// Get driver by mount ID with capability check
    pub fn get_driver(
        &self,
        mount_id: &str,
        capability: &str,
    ) -> KSResult<Arc<dyn KnowledgeSourceDriver>> {
        let mount = self.mounts.read()
            .get(mount_id)
            .ok_or_else(|| KnowledgeSourceError::Internal(
                format!("Mount {} not found", mount_id)
            ))?;

        // Validate capability against mount rules
        let allowed = mount.config.capability_rules.iter()
            .any(|rule| rule.capability == capability);

        if !allowed {
            return Err(KnowledgeSourceError::CapabilityDenied(
                format!("Capability '{}' not permitted for mount {}",
                    capability, mount_id)
            ));
        }

        self.driver_cache.read()
            .get(mount_id)
            .cloned()
            .ok_or_else(|| KnowledgeSourceError::Internal(
                format!("Driver not initialized for {}", mount_id)
            ))
    }

    async fn create_driver(
        &self,
        config: &SourceConfig,
    ) -> KSResult<Box<dyn KnowledgeSourceDriver>> {
        match config.source_type {
            SourceType::Pinecone => {
                Ok(Box::new(PineconeDriver::new(config.clone()).await?))
            }
            SourceType::Weaviate => {
                Ok(Box::new(WeaviateDriver::new(config.clone()).await?))
            }
            SourceType::PostgreSQL => {
                Ok(Box::new(PostgreSQLDriver::new(config.clone()).await?))
            }
            SourceType::RestApi => {
                Ok(Box::new(RestApiDriver::new(config.clone()).await?))
            }
            SourceType::S3 => {
                Ok(Box::new(S3Driver::new(config.clone()).await?))
            }
            SourceType::FileVectors => {
                Ok(Box::new(FileVectorsDriver::new(config.clone()).await?))
            }
        }
    }

    /// List all mounts with optional filter
    pub fn list_mounts(&self, source_type: Option<SourceType>) -> Vec<MountPoint> {
        self.mounts.read()
            .iter()
            .filter(|(_, m)| {
                source_type.is_none() || m.source_type == source_type.as_ref().unwrap().clone()
            })
            .map(|(_, m)| m.clone())
            .collect()
    }
}
```

---

## 5. Connector Implementations

### 5.1 Pinecone Driver (Vector Database)

```rust
// File: src/knowledge_source/drivers/pinecone.rs

pub struct PineconeDriver {
    config: SourceConfig,
    client: Arc<reqwest::Client>,
    index_name: String,
}

#[async_trait]
impl KnowledgeSourceDriver for PineconeDriver {
    async fn health_check(&self) -> KSResult<()> {
        let url = format!(
            "{}/describe_index_stats",
            self.config.endpoint
        );

        let response = self.client
            .post(&url)
            .header("Api-Key", &self.config.credentials["api_key"])
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map_err(|e| KnowledgeSourceError::ConnectionError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(KnowledgeSourceError::ConnectionError(
                format!("Status: {}", response.status())
            ))
        }
    }

    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        if q.vector.len() != 1536 {
            return Err(KnowledgeSourceError::InvalidQuery(
                format!("Expected vector dimension 1536, got {}", q.vector.len())
            ));
        }

        let url = format!("{}/query", self.config.endpoint);

        let query_body = serde_json::json!({
            "vector": q.vector,
            "topK": q.top_k,
            "includeMetadata": true,
            "filter": if q.filters.is_empty() {
                None
            } else {
                Some(q.filters)
            },
        });

        let response = self.client
            .post(&url)
            .header("Api-Key", &self.config.credentials["api_key"])
            .json(&query_body)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map_err(|e| KnowledgeSourceError::QueryTimeout(self.config.timeout_ms))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KnowledgeSourceError::Internal(e.to_string()))?;

        let results = json["results"]
            .as_array()
            .ok_or_else(|| KnowledgeSourceError::Internal(
                "Invalid Pinecone response format".into()
            ))?
            .iter()
            .map(|item| {
                let metadata = item["metadata"]
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| (
                                k.clone(),
                                match v {
                                    serde_json::Value::String(s) => FilterValue::String(s.clone()),
                                    serde_json::Value::Number(n) => FilterValue::Number(n.as_f64().unwrap_or(0.0)),
                                    _ => FilterValue::String(v.to_string()),
                                }
                            ))
                            .collect()
                    })
                    .unwrap_or_default();

                SemanticResult {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    content: item["metadata"]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    score: item["score"].as_f64().unwrap_or(0.0) as f32,
                    metadata,
                    source_id: self.config.endpoint.clone(),
                }
            })
            .collect();

        Ok(results)
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await
        .into_iter()
        .collect()
    }

    async fn list_documents(
        &self,
        offset: usize,
        limit: usize,
        capability: &str,
    ) -> KSResult<Vec<String>> {
        // Pinecone list_paginated
        let url = format!("{}/vectors", self.config.endpoint);
        let response = self.client
            .get(&url)
            .query(&[("limit", limit.to_string())])
            .header("Api-Key", &self.config.credentials["api_key"])
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let ids = json["vectors"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v["id"].as_str().unwrap_or("").to_string())
            .collect();

        Ok(ids)
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}
```

### 5.2 Weaviate Driver (Semantic Search)

```rust
// File: src/knowledge_source/drivers/weaviate.rs

pub struct WeaviateDriver {
    config: SourceConfig,
    client: Arc<reqwest::Client>,
    collection: String,
}

#[async_trait]
impl KnowledgeSourceDriver for WeaviateDriver {
    async fn health_check(&self) -> KSResult<()> {
        let url = format!("{}/v1/meta", self.config.endpoint);
        self.client
            .get(&url)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| KnowledgeSourceError::ConnectionError(e.to_string()))
    }

    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        let graphql_query = format!(
            r#"{{
              Get {{
                {} (
                  nearVector: {{ vector: {} }}
                  limit: {}
                ) {{
                  _additional {{ distance }}
                  text
                  id
                }}
              }}
            }}"#,
            self.collection,
            format!("[{}]", q.vector.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")),
            q.top_k
        );

        let response = self.client
            .post(&format!("{}/v1/graphql", self.config.endpoint))
            .json(&serde_json::json!({ "query": graphql_query }))
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map_err(|_| KnowledgeSourceError::QueryTimeout(self.config.timeout_ms))?;

        let json: serde_json::Value = response.json().await?;
        let results = json["data"]["Get"][&self.collection]
            .as_array()
            .map(|items| {
                items.iter().map(|item| SemanticResult {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    content: item["text"].as_str().unwrap_or("").to_string(),
                    score: 1.0 - item["_additional"]["distance"].as_f64().unwrap_or(0.0) as f32,
                    metadata: Default::default(),
                    source_id: self.config.endpoint.clone(),
                }).collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await
        .into_iter()
        .collect()
    }

    async fn list_documents(&self, _: usize, _: usize, _: &str)
        -> KSResult<Vec<String>> {
        Ok(vec![]) // GraphQL limitations
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}
```

### 5.3 PostgreSQL Driver (Relational + pgvector)

```rust
// File: src/knowledge_source/drivers/postgresql.rs

pub struct PostgreSQLDriver {
    config: SourceConfig,
    pool: Arc<sqlx::postgres::PgPool>,
}

#[async_trait]
impl KnowledgeSourceDriver for PostgreSQLDriver {
    async fn health_check(&self) -> KSResult<()> {
        self.pool
            .acquire()
            .await
            .map(|_| ())
            .map_err(|e| KnowledgeSourceError::ConnectionError(e.to_string()))
    }

    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        let vector_str = format!("[{}]", q.vector.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(","));

        let query_sql = format!(
            "SELECT id, content, embedding <-> $1::vector AS distance, metadata
             FROM documents
             ORDER BY embedding <-> $1::vector
             LIMIT $2",
        );

        let rows = sqlx::query_as::<_, (String, String, f32, Option<String>)>(&query_sql)
            .bind(&vector_str)
            .bind(q.top_k as i32)
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(|e| KnowledgeSourceError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|(id, content, distance, meta)| {
            SemanticResult {
                id,
                content,
                score: 1.0 / (1.0 + distance),
                metadata: meta.as_ref()
                    .and_then(|m| serde_json::from_str(m).ok())
                    .unwrap_or_default(),
                source_id: self.config.endpoint.clone(),
            }
        }).collect())
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await
        .into_iter()
        .collect()
    }

    async fn list_documents(
        &self,
        offset: usize,
        limit: usize,
        _: &str,
    ) -> KSResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM documents ORDER BY id OFFSET $1 LIMIT $2"
        )
        .bind(offset as i64)
        .bind(limit as i64)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}
```

### 5.4 REST API Driver (Generic)

```rust
// File: src/knowledge_source/drivers/rest_api.rs

pub struct RestApiDriver {
    config: SourceConfig,
    client: Arc<reqwest::Client>,
    query_endpoint: String,
}

#[async_trait]
impl KnowledgeSourceDriver for RestApiDriver {
    async fn health_check(&self) -> KSResult<()> {
        let url = self.config.credentials.get("health_endpoint")
            .unwrap_or(&format!("{}/health", self.config.endpoint));

        self.client.get(url)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| KnowledgeSourceError::ConnectionError(e.to_string()))
    }

    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        let request_body = serde_json::json!({
            "vector": q.vector,
            "top_k": q.top_k,
            "filters": q.filters,
        });

        let response = self.client
            .post(&self.query_endpoint)
            .json(&request_body)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .send()
            .await
            .map_err(|_| KnowledgeSourceError::QueryTimeout(self.config.timeout_ms))?;

        let json: serde_json::Value = response.json().await?;
        let results = json["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| SemanticResult {
                id: item["id"].as_str().unwrap_or("").to_string(),
                content: item["content"].as_str().unwrap_or("").to_string(),
                score: item["score"].as_f64().unwrap_or(0.0) as f32,
                metadata: item["metadata"].as_object()
                    .map(|m| m.iter().map(|(k, v)| {
                        (k.clone(), FilterValue::String(v.to_string()))
                    }).collect())
                    .unwrap_or_default(),
                source_id: self.config.endpoint.clone(),
            })
            .collect();

        Ok(results)
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await
        .into_iter()
        .collect()
    }

    async fn list_documents(&self, _: usize, _: usize, _: &str)
        -> KSResult<Vec<String>> {
        Ok(vec![])
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}
```

### 5.5 S3 Driver (Vector Files)

```rust
// File: src/knowledge_source/drivers/s3.rs

pub struct S3Driver {
    config: SourceConfig,
    s3_client: Arc<aws_sdk_s3::Client>,
    bucket: String,
}

#[async_trait]
impl KnowledgeSourceDriver for S3Driver {
    async fn health_check(&self) -> KSResult<()> {
        self.s3_client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| KnowledgeSourceError::ConnectionError(e.to_string()))
    }

    async fn query(
        &self,
        q: SemanticQuery,
        capability: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        // Load index manifest
        let resp = self.s3_client
            .get_object()
            .bucket(&self.bucket)
            .key("index.json")
            .send()
            .await?;

        let manifest_bytes = resp.body.collect().await?.into_bytes();
        let index: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;

        // Cosine similarity search
        let mut results = vec![];
        for item in index["vectors"].as_array().unwrap_or(&vec![]) {
            let stored_vec: Vec<f32> = serde_json::from_value(
                item["vector"].clone()
            )?;
            let similarity = cosine_similarity(&q.vector, &stored_vec);
            if results.len() < q.top_k {
                results.push((similarity, item.clone()));
                results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            }
        }

        Ok(results.into_iter().map(|(score, item)| {
            SemanticResult {
                id: item["id"].as_str().unwrap_or("").to_string(),
                content: item["content"].as_str().unwrap_or("").to_string(),
                score,
                metadata: Default::default(),
                source_id: self.config.endpoint.clone(),
            }
        }).collect())
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await
        .into_iter()
        .collect()
    }

    async fn list_documents(
        &self,
        _offset: usize,
        _limit: usize,
        _: &str,
    ) -> KSResult<Vec<String>> {
        let resp = self.s3_client
            .get_object()
            .bucket(&self.bucket)
            .key("index.json")
            .send()
            .await?;

        let manifest_bytes = resp.body.collect().await?.into_bytes();
        let index: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;

        Ok(index["vectors"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v["id"].as_str().unwrap_or("").to_string())
            .collect())
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}
```

### 5.6 File-based Vectors Driver

```rust
// File: src/knowledge_source/drivers/file_vectors.rs

pub struct FileVectorsDriver {
    config: SourceConfig,
    vectors: Arc<RwLock<HashMap<String, (Vec<f32>, String)>>>,
}

#[async_trait]
impl KnowledgeSourceDriver for FileVectorsDriver {
    async fn health_check(&self) -> KSResult<()> {
        let path = std::path::Path::new(&self.config.endpoint);
        if path.exists() {
            Ok(())
        } else {
            Err(KnowledgeSourceError::ConnectionError(
                format!("File not found: {}", self.config.endpoint)
            ))
        }
    }

    async fn query(
        &self,
        q: SemanticQuery,
        _: &str,
    ) -> KSResult<Vec<SemanticResult>> {
        let vectors = self.vectors.read();
        let mut results: Vec<_> = vectors.iter()
            .map(|(id, (vec, text))| {
                let score = cosine_similarity(&q.vector, vec);
                (id.clone(), text.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        Ok(results.into_iter()
            .take(q.top_k)
            .map(|(id, content, score)| SemanticResult {
                id,
                content,
                score,
                metadata: Default::default(),
                source_id: self.config.endpoint.clone(),
            })
            .collect())
    }

    async fn batch_query(
        &self,
        queries: Vec<SemanticQuery>,
        capability: &str,
    ) -> KSResult<Vec<Vec<SemanticResult>>> {
        Ok(futures::future::join_all(
            queries.into_iter().map(|q| self.query(q, capability))
        )
        .await)
    }

    async fn list_documents(&self, _: usize, _: usize, _: &str)
        -> KSResult<Vec<String>> {
        Ok(self.vectors.read().keys().cloned().collect())
    }

    fn config(&self) -> &SourceConfig { &self.config }
    fn status(&self) -> SourceStatus { /* telemetry */ }
}
```

---

## 6. Performance Targets & Compliance Matrix

| Connector | Latency Target | P99 Latency | Batch Factor | Notes |
|-----------|----------------|------------|--------------|-------|
| **Pinecone** | <500ms | <700ms | 10x | Index-optimized; network-dependent |
| **Weaviate** | <1000ms | <1500ms | 5x | GraphQL overhead; self-hosted friendly |
| **PostgreSQL** | <100ms | <150ms | 100x | pgvector extension; local deployment |
| **REST API** | <800ms | <1200ms | N/A | Depends on upstream implementation |
| **S3** | <2000ms | <3000ms | 20x | Requires index manifest caching |
| **File Vectors** | <50ms | <100ms | 100x | In-memory; ideal for dev/test |

---

## 7. Integration with CSCI Layer

Knowledge sources are queried through the Cognitive Substrate Capability Interface:

```rust
// In CSCI context handler
pub async fn query_semantic_memory(
    ctx: &CSCIContext,
    mount_id: &str,
    query: SemanticQuery,
) -> KSResult<Vec<SemanticResult>> {
    // Extract capability token from context
    let capability = ctx.capability_token();

    // Get driver from registry with capability check
    let driver = MOUNT_REGISTRY.get_driver(mount_id, &capability)?;

    // Execute query with timing
    let start = std::time::Instant::now();
    let results = driver.query(query, &capability).await?;
    let elapsed = start.elapsed().as_millis() as u64;

    // Emit telemetry
    metrics::histogram!("ks_query_latency_ms", elapsed as f64);

    Ok(results)
}
```

---

## 8. Testing Strategy

- **Unit Tests:** Each driver tested against mocked/containerized backends (6 suites)
- **Integration Tests:** Cross-driver mount/unmount cycles; capability gating validation
- **Chaos Tests:** Simulate timeouts, credential failures, network partitions
- **Load Tests:** Batch queries at 100 QPS per driver; measure P99 latency

**Test Coverage Target:** 92% line coverage; all error paths exercised

---

## 9. Success Criteria

✓ All 6 drivers pass health checks against reference implementations
✓ Pinecone queries <500ms at P95 (10k vector index)
✓ Capability-gated access prevents unauthorized queries
✓ Mount registry survives node restarts via CRDT persistence
✓ Integration test demonstrates multi-source federated query

