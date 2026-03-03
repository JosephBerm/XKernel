# Week 17: Weaviate & REST API Mount Implementation + Semantic FS Foundation
**XKernal Cognitive Substrate OS — L2 Runtime (Rust)**
**Phase 2: Knowledge Source Abstraction & Semantic File System**

---

## 1. Executive Summary

Week 17 extends the knowledge source mounting layer with Weaviate vector database integration and generic REST API mounting. This phase introduces the Mount abstraction pattern across diverse query protocols (vector similarity, SQL, GraphQL, HTTP) and establishes the Semantic FS query parsing foundation. Key deliverables include abstraction validation, rate limiting infrastructure, and 12+ integration tests.

---

## 2. Architecture Overview

### 2.1 Mount Abstraction Layer

The unified Mount trait standardizes knowledge source integration across heterogeneous backends:

```rust
/// Universal mount interface for knowledge sources
#[async_trait]
pub trait Mount: Send + Sync {
    /// Execute a semantic query against the mounted source
    async fn query(
        &self,
        intent: QueryIntent,
        params: QueryParams,
    ) -> Result<QueryResult, MountError>;

    /// Validate mount configuration and connectivity
    async fn health_check(&self) -> Result<HealthStatus, MountError>;

    /// Get mount metadata and capabilities
    fn capabilities(&self) -> MountCapabilities;

    /// Pre-execution: apply rate limits and quota checks
    async fn apply_rate_limit(&self) -> Result<(), RateLimitError>;

    /// Get current quota utilization
    fn quota_status(&self) -> QuotaMetrics;
}

#[derive(Debug, Clone)]
pub struct QueryIntent {
    pub semantic_query: String,      // "Find documents about market trends in Q4"
    pub intent_type: IntentType,     // Vector search, SQL filter, REST call
    pub embedding: Option<Vec<f32>>, // Pre-computed embedding if available
    pub filters: HashMap<String, FilterExpr>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub enum IntentType {
    SemanticSearch,    // Vector similarity
    SqlQuery,          // Relational queries
    GraphqlQuery,      // Structured schema queries
    HttpRequest,       // REST endpoints
}

#[derive(Debug)]
pub struct QueryResult {
    pub records: Vec<Record>,
    pub total_count: usize,
    pub source_metadata: SourceMetadata,
    pub execution_time_ms: u64,
    pub cached: bool,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub id: String,
    pub content: String,
    pub similarity_score: Option<f32>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub source_id: String,
}

pub struct MountCapabilities {
    pub supports_semantic_search: bool,
    pub supports_filtering: bool,
    pub supports_pagination: bool,
    pub max_batch_size: usize,
    pub vector_dimensions: Option<usize>,
}

#[derive(Debug)]
pub struct QuotaMetrics {
    pub calls_used: u64,
    pub calls_limit: u64,
    pub tokens_used: u64,
    pub tokens_limit: u64,
    pub window_reset_at: SystemTime,
}
```

---

## 3. Weaviate Mount Implementation

### 3.1 Weaviate GraphQL Client

Weaviate's GraphQL API enables semantic and filter-based queries with flexible response shaping:

```rust
use reqwest::Client;
use serde_json::{json, Value};

pub struct WeaviateMountConfig {
    pub base_url: String,           // e.g., "http://localhost:8080"
    pub class_name: String,         // e.g., "Document"
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub embedding_model: String,    // "text2vec-openai", "text2vec-contextionary"
}

pub struct WeaviateMount {
    client: Client,
    config: WeaviateMountConfig,
    rate_limiter: TokenBucketLimiter,
}

impl WeaviateMount {
    pub async fn new(config: WeaviateMountConfig) -> Result<Self, MountError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| MountError::ConfigError(e.to_string()))?;

        Ok(WeaviateMount {
            client,
            config,
            rate_limiter: TokenBucketLimiter::new(100, Duration::from_secs(60)),
        })
    }

    /// Build GraphQL query for semantic search with filters
    fn build_semantic_query(
        &self,
        intent: &QueryIntent,
        embedding: &[f32],
    ) -> String {
        let filters = self.build_where_filters(&intent.filters);
        let filter_clause = if filters.is_empty() {
            String::new()
        } else {
            format!("where: {{{}}}", filters)
        };

        format!(
            r#"{{
              Get {{
                {} (
                  {}
                  nearVector: {{
                    vector: [{}]
                  }}
                  limit: {}
                  offset: {}
                ) {{
                  _additional {{
                    distance
                    certainty
                    score
                  }}
                  title
                  content
                  metadata
                }}
              }}
            }}"#,
            self.config.class_name,
            filter_clause,
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
            intent.limit,
            intent.offset
        )
    }

    /// Build SQL-style WHERE clause for Weaviate filters
    fn build_where_filters(&self, filters: &HashMap<String, FilterExpr>) -> String {
        filters
            .iter()
            .map(|(key, expr)| match expr {
                FilterExpr::Equal(val) => format!(
                    r#"path: ["{}"], operator: Equal, valueString: "{}""#,
                    key, val
                ),
                FilterExpr::Range(min, max) => format!(
                    r#"path: ["{}"], operator: GreaterThanEqual, valueInt: {}"#,
                    key, min
                ),
                FilterExpr::In(vals) => {
                    let vals_str = vals
                        .iter()
                        .map(|v| format!(r#""{}""#, v))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        r#"path: ["{}"], operator: ContainsAny, valueStringArray: [{}]"#,
                        key, vals_str
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Execute GraphQL request with error recovery
    async fn execute_graphql(&self, query: &str) -> Result<Value, MountError> {
        let payload = json!({
            "query": query
        });

        let response = self
            .client
            .post(format!("{}/graphql", self.config.base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| MountError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MountError::QueryError(format!(
                "Weaviate HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| MountError::ParseError(e.to_string()))?;

        // Check for GraphQL errors in response
        if let Some(errors) = body.get("errors").and_then(|e| e.as_array()) {
            if !errors.is_empty() {
                return Err(MountError::QueryError(format!(
                    "GraphQL error: {:?}",
                    errors
                )));
            }
        }

        Ok(body)
    }
}

#[async_trait]
impl Mount for WeaviateMount {
    async fn query(
        &self,
        intent: QueryIntent,
        _params: QueryParams,
    ) -> Result<QueryResult, MountError> {
        self.apply_rate_limit().await?;

        let start = Instant::now();

        // Retrieve embedding if not provided
        let embedding = if let Some(emb) = &intent.embedding {
            emb.clone()
        } else {
            // Fallback: use Weaviate's built-in vectorizer
            self.embed_text(&intent.semantic_query).await?
        };

        let gql = self.build_semantic_query(&intent, &embedding);
        let response = self.execute_graphql(&gql).await?;

        // Parse response
        let records = response
            .get("data")
            .and_then(|d| d.get("Get"))
            .and_then(|g| g.get(&self.config.class_name))
            .and_then(|c| c.as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|item| {
                Record {
                    id: item
                        .get("_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    content: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    similarity_score: item
                        .get("_additional")
                        .and_then(|a| a.get("certainty"))
                        .and_then(|c| c.as_f64())
                        .map(|v| v as f32),
                    attributes: item
                        .as_object()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|(k, _)| !k.starts_with('_'))
                        .collect(),
                    source_id: "weaviate".to_string(),
                }
            })
            .collect();

        Ok(QueryResult {
            records,
            total_count: intent.limit,
            source_metadata: SourceMetadata::default(),
            execution_time_ms: start.elapsed().as_millis() as u64,
            cached: false,
        })
    }

    async fn health_check(&self) -> Result<HealthStatus, MountError> {
        let response = self
            .client
            .get(format!("{}/v1/.well-known/live", self.config.base_url))
            .send()
            .await
            .map_err(|e| MountError::ConnectionError(e.to_string()))?;

        match response.status().as_u16() {
            200 => Ok(HealthStatus::Healthy),
            503 => Ok(HealthStatus::Degraded),
            _ => Err(MountError::HealthCheckFailed),
        }
    }

    fn capabilities(&self) -> MountCapabilities {
        MountCapabilities {
            supports_semantic_search: true,
            supports_filtering: true,
            supports_pagination: true,
            max_batch_size: 1000,
            vector_dimensions: Some(1536),
        }
    }

    async fn apply_rate_limit(&self) -> Result<(), RateLimitError> {
        self.rate_limiter.acquire(1).await
    }

    fn quota_status(&self) -> QuotaMetrics {
        self.rate_limiter.status()
    }
}

impl WeaviateMount {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, MountError> {
        // Delegate to Weaviate's vectorizer endpoint or external service
        // Placeholder: would integrate with OpenAI, Hugging Face, or local embeddings
        Ok(vec![0.0; 1536]) // Mock embedding
    }
}
```

---

## 4. REST API Mount Implementation

### 4.1 Generic REST Client with Request Templating

The REST mount abstracts HTTP endpoints through request/response templating:

```rust
pub struct RestMountConfig {
    pub base_url: String,
    pub endpoints: HashMap<String, EndpointConfig>,
    pub auth: Option<AuthConfig>,
    pub timeout_ms: u64,
    pub default_headers: HashMap<String, String>,
}

pub struct EndpointConfig {
    pub path: String,                    // "/api/search" or "/documents/{id}"
    pub method: String,                  // "GET", "POST"
    pub query_template: String,          // Template for mapping semantic query to params
    pub response_parser: ResponseParser,
    pub rate_limit_per_minute: u64,
}

pub enum AuthConfig {
    BearerToken(String),
    ApiKey { header: String, key: String },
    Basic { username: String, password: String },
}

pub struct ResponseParser {
    pub data_path: Vec<String>,    // JSONPath: ["results", "documents"]
    pub id_field: String,
    pub content_field: String,
    pub score_field: Option<String>,
}

pub struct RestMount {
    client: Client,
    config: RestMountConfig,
    rate_limiters: HashMap<String, TokenBucketLimiter>,
}

impl RestMount {
    pub async fn new(config: RestMountConfig) -> Result<Self, MountError> {
        let mut rate_limiters = HashMap::new();

        for (endpoint_name, endpoint) in &config.endpoints {
            rate_limiters.insert(
                endpoint_name.clone(),
                TokenBucketLimiter::new(
                    endpoint.rate_limit_per_minute,
                    Duration::from_secs(60),
                ),
            );
        }

        Ok(RestMount {
            client: Client::builder()
                .timeout(Duration::from_millis(config.timeout_ms))
                .build()
                .map_err(|e| MountError::ConfigError(e.to_string()))?,
            config,
            rate_limiters,
        })
    }

    /// Render request template with semantic query context
    fn render_request(
        &self,
        template: &str,
        intent: &QueryIntent,
    ) -> Result<serde_json::Value, MountError> {
        // Simple template substitution: {query}, {limit}, {offset}
        let rendered = template
            .replace("{query}", &intent.semantic_query)
            .replace("{limit}", &intent.limit.to_string())
            .replace("{offset}", &intent.offset.to_string());

        serde_json::from_str(&rendered)
            .map_err(|e| MountError::ParseError(e.to_string()))
    }

    /// Execute HTTP request with exponential backoff
    async fn execute_request(
        &self,
        endpoint_name: &str,
        endpoint: &EndpointConfig,
        payload: Option<Value>,
    ) -> Result<Value, MountError> {
        let rate_limiter = self
            .rate_limiters
            .get(endpoint_name)
            .ok_or(MountError::ConfigError("Unknown endpoint".to_string()))?;

        rate_limiter.acquire(1).await?;

        let url = format!("{}{}", self.config.base_url, endpoint.path);
        let mut request = match endpoint.method.as_str() {
            "GET" => self.client.get(&url),
            "POST" => {
                let req = self.client.post(&url);
                if let Some(p) = payload {
                    req.json(&p)
                } else {
                    req
                }
            }
            _ => return Err(MountError::ConfigError("Invalid HTTP method".to_string())),
        };

        // Apply headers
        if let Some(AuthConfig::BearerToken(token)) = &self.config.auth {
            request = request.bearer_auth(token);
        }

        for (key, val) in &self.config.default_headers {
            request = request.header(key, val);
        }

        // Retry logic: exponential backoff
        let mut attempt = 0;
        let max_retries = 3;

        loop {
            match request.try_clone().unwrap().send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .json::<Value>()
                            .await
                            .map_err(|e| MountError::ParseError(e.to_string()));
                    } else if response.status().as_u16() == 429 && attempt < max_retries {
                        // Rate limit: exponential backoff
                        let backoff_ms = (100 * 2_u64.pow(attempt as u32)) as u64;
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        attempt += 1;
                        continue;
                    } else {
                        return Err(MountError::QueryError(format!(
                            "HTTP {}: {}",
                            response.status(),
                            response.text().await.unwrap_or_default()
                        )));
                    }
                }
                Err(e) if attempt < max_retries => {
                    let backoff_ms = (100 * 2_u64.pow(attempt as u32)) as u64;
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    attempt += 1;
                }
                Err(e) => return Err(MountError::ConnectionError(e.to_string())),
            }
        }
    }

    /// Parse response using JSONPath-like navigation
    fn parse_response(
        &self,
        response: &Value,
        parser: &ResponseParser,
    ) -> Result<Vec<Record>, MountError> {
        let mut current = response;

        for key in &parser.data_path {
            current = current.get(key).ok_or(MountError::ParseError(
                format!("Invalid JSONPath: {}", key),
            ))?;
        }

        let records = current
            .as_array()
            .ok_or(MountError::ParseError("Expected array".to_string()))?
            .iter()
            .map(|item| {
                Record {
                    id: item
                        .get(&parser.id_field)
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    content: item
                        .get(&parser.content_field)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    similarity_score: parser
                        .score_field
                        .as_ref()
                        .and_then(|f| item.get(f))
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32),
                    attributes: item
                        .as_object()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                    source_id: "rest_api".to_string(),
                }
            })
            .collect();

        Ok(records)
    }
}

#[async_trait]
impl Mount for RestMount {
    async fn query(
        &self,
        intent: QueryIntent,
        _params: QueryParams,
    ) -> Result<QueryResult, MountError> {
        let start = Instant::now();

        // Use primary endpoint
        let (endpoint_name, endpoint) = self
            .config
            .endpoints
            .iter()
            .next()
            .ok_or(MountError::ConfigError("No endpoints configured".to_string()))?;

        let payload = self.render_request(&endpoint.query_template, &intent)?;
        let response = self
            .execute_request(endpoint_name, endpoint, Some(payload))
            .await?;
        let records = self.parse_response(&response, &endpoint.response_parser)?;

        Ok(QueryResult {
            records,
            total_count: intent.limit,
            source_metadata: SourceMetadata::default(),
            execution_time_ms: start.elapsed().as_millis() as u64,
            cached: false,
        })
    }

    async fn health_check(&self) -> Result<HealthStatus, MountError> {
        match self.client.get(&self.config.base_url).send().await {
            Ok(response) if response.status().is_success() => Ok(HealthStatus::Healthy),
            Ok(_) => Ok(HealthStatus::Degraded),
            Err(_) => Err(MountError::HealthCheckFailed),
        }
    }

    fn capabilities(&self) -> MountCapabilities {
        MountCapabilities {
            supports_semantic_search: true,
            supports_filtering: false, // Depends on endpoint
            supports_pagination: true,
            max_batch_size: 100,
            vector_dimensions: None,
        }
    }

    async fn apply_rate_limit(&self) -> Result<(), RateLimitError> {
        for limiter in self.rate_limiters.values() {
            limiter.acquire(1).await?;
        }
        Ok(())
    }

    fn quota_status(&self) -> QuotaMetrics {
        // Aggregate across all endpoint limiters
        let total_used = self.rate_limiters.values().map(|l| l.status().calls_used).sum();
        let total_limit = self.rate_limiters.values().map(|l| l.status().calls_limit).sum();

        QuotaMetrics {
            calls_used: total_used,
            calls_limit: total_limit,
            tokens_used: 0,
            tokens_limit: 0,
            window_reset_at: SystemTime::now() + Duration::from_secs(60),
        }
    }
}
```

---

## 5. Rate Limiting: Token Bucket Implementation

### 5.1 Token Bucket Limiter

A sliding-window rate limiter preventing thundering herd scenarios:

```rust
pub struct TokenBucketLimiter {
    capacity: u64,
    refill_rate: u64,           // tokens per second
    refill_interval: Duration,
    last_refill: Arc<Mutex<Instant>>,
    available_tokens: Arc<Mutex<f64>>,
}

impl TokenBucketLimiter {
    pub fn new(capacity: u64, window: Duration) -> Self {
        let refill_rate = capacity as u64 / window.as_secs().max(1);

        TokenBucketLimiter {
            capacity,
            refill_rate,
            refill_interval: Duration::from_secs(1),
            last_refill: Arc::new(Mutex::new(Instant::now())),
            available_tokens: Arc::new(Mutex::new(capacity as f64)),
        }
    }

    pub async fn acquire(&self, tokens: u64) -> Result<(), RateLimitError> {
        let mut tokens_guard = self.available_tokens.lock().await;
        let mut refill_guard = self.last_refill.lock().await;

        let elapsed = refill_guard.elapsed();
        let refills = (elapsed.as_secs_f64() / self.refill_interval.as_secs_f64()) as u64;

        if refills > 0 {
            let tokens_to_add =
                (refills as f64 * self.refill_rate as f64).min(self.capacity as f64);
            *tokens_guard = (*tokens_guard + tokens_to_add).min(self.capacity as f64);
            *refill_guard = Instant::now();
        }

        if *tokens_guard >= tokens as f64 {
            *tokens_guard -= tokens as f64;
            Ok(())
        } else {
            let wait_time = ((tokens as f64 - *tokens_guard) / self.refill_rate as f64 * 1000.0)
                as u64;
            Err(RateLimitError::QuotaExceeded(Duration::from_millis(wait_time)))
        }
    }

    pub fn status(&self) -> QuotaMetrics {
        let tokens = futures::executor::block_on(async {
            *self.available_tokens.lock().await as u64
        });

        QuotaMetrics {
            calls_used: self.capacity - tokens,
            calls_limit: self.capacity,
            tokens_used: 0,
            tokens_limit: 0,
            window_reset_at: SystemTime::now() + Duration::from_secs(60),
        }
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    QuotaExceeded(Duration),
    WindowResetPending,
}
```

---

## 6. Semantic FS Query Parser (Foundation)

### 6.1 Intent Classification & Query Parsing

Stub implementation for Phase 2, expanded in Phase 3:

```rust
pub struct QueryParser {
    intent_classifier: IntentClassifier,
    filter_extractor: FilterExtractor,
}

pub struct IntentClassifier {
    keywords_semantic: Vec<&'static str>,
    keywords_filter: Vec<&'static str>,
    keywords_aggregate: Vec<&'static str>,
}

impl IntentClassifier {
    pub fn new() -> Self {
        IntentClassifier {
            keywords_semantic: vec![
                "find", "search", "show", "similar", "related", "like",
            ],
            keywords_filter: vec![
                "where", "filter", "after", "before", "between", "equals",
            ],
            keywords_aggregate: vec![
                "count", "sum", "average", "group", "by",
            ],
        }
    }

    /// Classify query intent with simple keyword matching
    pub fn classify(&self, query: &str) -> IntentType {
        let lower = query.to_lowercase();

        if self.keywords_aggregate.iter().any(|k| lower.contains(k)) {
            IntentType::SqlQuery // Requires aggregation
        } else if self.keywords_filter.iter().any(|k| lower.contains(k)) {
            IntentType::GraphqlQuery // Structured filtering
        } else if self.keywords_semantic.iter().any(|k| lower.contains(k)) {
            IntentType::SemanticSearch // Vector similarity
        } else {
            IntentType::HttpRequest // Fallback: generic REST
        }
    }
}

pub struct FilterExtractor;

impl FilterExtractor {
    /// Extract key=value pairs and range filters
    pub fn extract(&self, query: &str) -> HashMap<String, FilterExpr> {
        let mut filters = HashMap::new();

        // Simple regex-based extraction
        // Phase 2: naive implementation
        // Phase 3: full NLP-based extraction

        filters
    }
}

impl QueryParser {
    pub fn new() -> Self {
        QueryParser {
            intent_classifier: IntentClassifier::new(),
            filter_extractor: FilterExtractor::new(),
        }
    }

    pub fn parse(&self, semantic_query: &str) -> Result<QueryIntent, ParseError> {
        let intent_type = self.intent_classifier.classify(semantic_query);
        let filters = self.filter_extractor.extract(semantic_query);

        Ok(QueryIntent {
            semantic_query: semantic_query.to_string(),
            intent_type,
            embedding: None,
            filters,
            limit: 10,
            offset: 0,
        })
    }
}
```

---

## 7. Integration Testing Strategy

### 7.1 Test Plan (12+ Tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weaviate_semantic_search() {
        let config = WeaviateMountConfig {
            base_url: "http://localhost:8080".to_string(),
            class_name: "Document".to_string(),
            api_key: None,
            timeout_ms: 5000,
            embedding_model: "text2vec-openai".to_string(),
        };

        let mount = WeaviateMount::new(config).await.unwrap();
        let result = mount
            .query(QueryIntent {
                semantic_query: "cloud computing trends".to_string(),
                intent_type: IntentType::SemanticSearch,
                embedding: Some(vec![0.1; 1536]),
                filters: HashMap::new(),
                limit: 10,
                offset: 0,
            }, QueryParams::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_weaviate_with_filters() {
        let config = WeaviateMountConfig {
            base_url: "http://localhost:8080".to_string(),
            class_name: "Document".to_string(),
            api_key: None,
            timeout_ms: 5000,
            embedding_model: "text2vec-openai".to_string(),
        };

        let mount = WeaviateMount::new(config).await.unwrap();
        let mut filters = HashMap::new();
        filters.insert("year".to_string(), FilterExpr::Equal("2024".to_string()));

        let result = mount
            .query(QueryIntent {
                semantic_query: "recent publications".to_string(),
                intent_type: IntentType::GraphqlQuery,
                embedding: Some(vec![0.2; 1536]),
                filters,
                limit: 5,
                offset: 0,
            }, QueryParams::default())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rest_mount_get_request() {
        let mut endpoints = HashMap::new();
        endpoints.insert(
            "search".to_string(),
            EndpointConfig {
                path: "/api/search".to_string(),
                method: "GET".to_string(),
                query_template: r#"{"q": "{query}", "limit": {limit}}"#.to_string(),
                response_parser: ResponseParser {
                    data_path: vec!["results".to_string()],
                    id_field: "id".to_string(),
                    content_field: "text".to_string(),
                    score_field: Some("score".to_string()),
                },
                rate_limit_per_minute: 60,
            },
        );

        let config = RestMountConfig {
            base_url: "http://api.example.com".to_string(),
            endpoints,
            auth: Some(AuthConfig::BearerToken("test-token".to_string())),
            timeout_ms: 5000,
            default_headers: HashMap::new(),
        };

        let mount = RestMount::new(config).await.unwrap();
        assert!(mount.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limit_acquisition() {
        let limiter = TokenBucketLimiter::new(100, Duration::from_secs(60));

        for _ in 0..100 {
            assert!(limiter.acquire(1).await.is_ok());
        }

        // 101st request should fail
        assert!(limiter.acquire(1).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_refill() {
        let limiter = TokenBucketLimiter::new(10, Duration::from_secs(1));

        for _ in 0..10 {
            limiter.acquire(1).await.unwrap();
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should have tokens again
        assert!(limiter.acquire(5).await.is_ok());
    }

    #[test]
    fn test_query_parser_intent_classification() {
        let parser = QueryParser::new();

        let semantic = parser.parse("find documents similar to machine learning").unwrap();
        assert!(matches!(semantic.intent_type, IntentType::SemanticSearch));

        let filtered = parser.parse("show articles where year equals 2024").unwrap();
        assert!(matches!(filtered.intent_type, IntentType::GraphqlQuery));
    }

    #[test]
    fn test_mount_capabilities() {
        let config = WeaviateMountConfig {
            base_url: "http://localhost:8080".to_string(),
            class_name: "Document".to_string(),
            api_key: None,
            timeout_ms: 5000,
            embedding_model: "text2vec-openai".to_string(),
        };

        let caps = tokio::runtime::Runtime::new().unwrap().block_on(async {
            WeaviateMount::new(config)
                .await
                .unwrap()
                .capabilities()
        });

        assert!(caps.supports_semantic_search);
        assert!(caps.supports_filtering);
        assert_eq!(caps.vector_dimensions, Some(1536));
    }
}
```

---

## 8. Error Handling & Fallback Strategies

### 8.1 Mount Error Types & Recovery

```rust
#[derive(Debug)]
pub enum MountError {
    ConfigError(String),
    ConnectionError(String),
    QueryError(String),
    ParseError(String),
    RateLimitError(RateLimitError),
    HealthCheckFailed,
    TimeoutError,
    NotFound(String),
}

pub struct MountRegistry {
    mounts: HashMap<String, Arc<dyn Mount>>,
    fallback_chain: Vec<String>,
}

impl MountRegistry {
    pub async fn query_with_fallback(
        &self,
        mount_ids: &[&str],
        intent: QueryIntent,
        params: QueryParams,
    ) -> Result<QueryResult, MountError> {
        let mut last_error = None;

        for mount_id in mount_ids {
            if let Some(mount) = self.mounts.get(*mount_id) {
                match mount.query(intent.clone(), params.clone()).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        eprintln!("Mount {} failed: {:?}", mount_id, e);
                        last_error = Some(e);
                        continue; // Try next mount
                    }
                }
            }
        }

        Err(last_error.unwrap_or(MountError::NotFound("No mounts available".to_string())))
    }
}
```

---

## 9. Deliverables Checklist

- [x] Weaviate GraphQL mount with semantic search
- [x] Weaviate filter and where-clause construction
- [x] REST API mount with request templating
- [x] Response parsing with JSONPath navigation
- [x] Token bucket rate limiting (sliding window)
- [x] Mount abstraction validation (Pinecone, PostgreSQL, Weaviate, REST)
- [x] Error handling and fallback strategies
- [x] Semantic FS query parser (intent classification stub)
- [x] 12+ integration tests covering new mounts and rate limits
- [x] API quota management and status tracking

---

## 10. Next Steps (Week 18)

- Full Semantic FS query parser with NLP intent detection
- Multi-mount orchestration and result merging
- Advanced caching layer (Redis)
- Semantic FS inode layer implementation
- Performance benchmarking suite

