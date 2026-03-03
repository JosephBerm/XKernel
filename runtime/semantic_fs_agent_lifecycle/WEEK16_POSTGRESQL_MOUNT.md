# Week 16: PostgreSQL Semantic Volume Mount Design
## XKernal Cognitive Substrate OS - L2 Runtime Layer (Rust)

**Date:** 2026-03-02
**Phase:** Phase 2 (Knowledge Source Integration)
**Layer:** L2 Runtime
**Team:** Semantic FS & Agent Lifecycle (Engineer 8)

---

## Executive Summary

Week 16 implements PostgreSQL as a queryable semantic volume within the XKernal runtime, extending the knowledge source mounting framework established in Week 15 (Pinecone). This design introduces relational query translation—converting semantic intents into safe, parameterized SQL—alongside connection pooling, schema introspection, and transaction support. The implementation maintains the composable, transparent design principles while ensuring SQL injection prevention and query isolation.

---

## 1. Architecture Overview

### 1.1 Component Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│ Semantic FS (Week 15-16 Unified Mount Layer)                │
└─────────────────────────────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Knowledge Source Router (Mount Dispatcher)                   │
│ - Type Detection (Vector DB, Relational, Document)          │
│ - Credential Routing & Rotation                             │
└─────────────────────────────────────────────────────────────┘
                           ▼
    ┌──────────────────────┬──────────────────────┐
    ▼                      ▼                      ▼
┌─────────────┐    ┌──────────────┐    ┌──────────────────┐
│ Pinecone    │    │ PostgreSQL    │    │ Document Stores  │
│ Mount       │    │ Mount (NEW)   │    │ (Planned)        │
│ (Week 15)   │    │ (Week 16)     │    │                  │
└─────────────┘    └──────────────┘    └──────────────────┘
```

### 1.2 PostgreSQL Mount Lifecycle

```
Mount Request
    ↓
[Schema Introspection] → Metadata Cache
    ↓
[Query Intent Parsing] → Semantic Intent Tree
    ↓
[SQL Translation] → Parameterized SQL
    ↓
[Connection Pool] → Acquire Connection
    ↓
[Query Execution] → Transaction Wrapper
    ↓
[Result Normalization] → Semantic Result Set
    ↓
Result Stream (to Agent Layer)
```

---

## 2. Semantic Intent → SQL Translation

### 2.1 Intent Grammar & Parsing

The semantic intent parser converts natural language queries into an intermediate representation (IR) before SQL generation. This decoupling ensures safety and enables cross-database support.

```rust
/// Semantic intent IR for relational queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalIntent {
    /// Canonical table/relation name (validated against schema)
    pub relation: String,

    /// Projected columns; empty = SELECT *
    pub projections: Vec<ColumnProjection>,

    /// WHERE clause constraints
    pub filters: Vec<FilterExpr>,

    /// JOIN specifications
    pub joins: Vec<JoinSpec>,

    /// Aggregation operations (GROUP BY, COUNT, SUM, etc.)
    pub aggregations: Option<AggregationSpec>,

    /// LIMIT and OFFSET (safety-bounded)
    pub limit: Option<u32>,
    pub offset: Option<u32>,

    /// Query intent metadata (for observability)
    pub intent_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProjection {
    pub column: String,
    pub alias: Option<String>,
    pub transform: Option<ColumnTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnTransform {
    ToLowercase,
    ToUppercase,
    Cast(String),
    Coalesce(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterExpr {
    pub column: String,
    pub operator: FilterOp,
    pub value: FilterValue,
    pub is_parameterized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
    Between,
    IsNull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<FilterValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSpec {
    pub join_type: JoinType,
    pub right_table: String,
    pub left_column: String,
    pub right_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationSpec {
    pub group_by: Vec<String>,
    pub aggregates: Vec<AggregateFunc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunc {
    Count(Option<String>),
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
}
```

### 2.2 Safe SQL Generation with Parameterization

```rust
/// SQL generator with strict parameterization enforcement
pub struct SafeSqlGenerator {
    schema_cache: Arc<SchemaCache>,
    config: SqlGenerationConfig,
}

#[derive(Clone)]
pub struct SqlGenerationConfig {
    /// Maximum row limit to prevent expensive queries
    pub max_limit: u32,
    /// Allowed operations (for policy enforcement)
    pub allowed_operations: HashSet<String>,
    /// Column filtering rules (PII redaction)
    pub column_filters: HashMap<String, ColumnAccessPolicy>,
}

impl SafeSqlGenerator {
    pub async fn generate(
        &self,
        intent: &RelationalIntent,
    ) -> Result<PreparedQuery, SqlGenerationError> {
        // Validation phase: check intent against schema
        self.validate_intent(intent).await?;

        // Build query components
        let (select_clause, projections) = self.build_select(intent)?;
        let (from_clause, tables) = self.build_from(intent)?;
        let (where_clause, params) = self.build_where(intent).await?;
        let join_clause = self.build_joins(intent).await?;
        let groupby_clause = self.build_group_by(intent)?;
        let limit_clause = self.build_limit(intent)?;

        // Compose final SQL
        let mut sql = format!(
            "{}\n{}\n{}\n{}",
            select_clause, from_clause, join_clause, where_clause
        );

        if let Some(gb) = groupby_clause {
            sql.push('\n');
            sql.push_str(&gb);
        }

        sql.push('\n');
        sql.push_str(&limit_clause);

        // Trace for observability
        tracing::debug!(
            sql = sql,
            params_count = params.len(),
            intent = ?intent,
            "Generated parameterized SQL"
        );

        Ok(PreparedQuery {
            sql: Arc::new(sql),
            parameters: params,
            projections,
            tables,
        })
    }

    async fn validate_intent(&self, intent: &RelationalIntent) -> Result<(), SqlGenerationError> {
        let schema = self.schema_cache.get(&intent.relation).await?;

        // Validate projections
        for proj in &intent.projections {
            if proj.column != "*" && !schema.columns.contains_key(&proj.column) {
                return Err(SqlGenerationError::ColumnNotFound(proj.column.clone()));
            }
        }

        // Validate filters
        for filter in &intent.filters {
            if !schema.columns.contains_key(&filter.column) {
                return Err(SqlGenerationError::ColumnNotFound(filter.column.clone()));
            }
            // Type coercion validation
            let col_type = &schema.columns[&filter.column].data_type;
            self.validate_filter_type_compatibility(col_type, &filter.value)?;
        }

        // Validate joins
        for join in &intent.joins {
            if !self.schema_cache.contains(&join.right_table).await {
                return Err(SqlGenerationError::TableNotFound(join.right_table.clone()));
            }
        }

        Ok(())
    }

    fn build_select(&self, intent: &RelationalIntent) -> Result<(String, Vec<String>), SqlGenerationError> {
        let mut projections = Vec::new();

        if intent.projections.is_empty() {
            return Ok(("SELECT *".to_string(), vec!["*".to_string()]));
        }

        let proj_parts: Result<Vec<_>, _> = intent
            .projections
            .iter()
            .map(|p| {
                projections.push(p.column.clone());

                let mut clause = p.column.clone();

                if let Some(transform) = &p.transform {
                    clause = self.apply_transform(&p.column, transform)?;
                }

                if let Some(alias) = &p.alias {
                    clause = format!("{} AS {}", clause, alias);
                }

                Ok(clause)
            })
            .collect();

        let select_str = format!("SELECT {}", proj_parts?.join(", "));
        Ok((select_str, projections))
    }

    fn build_where(
        &self,
        intent: &RelationalIntent,
    ) -> Result<(String, Vec<QueryParam>), SqlGenerationError> {
        if intent.filters.is_empty() {
            return Ok(("".to_string(), Vec::new()));
        }

        let mut params = Vec::new();
        let mut clauses = Vec::new();

        for (idx, filter) in intent.filters.iter().enumerate() {
            let param_idx = params.len();

            let clause = match filter.operator {
                FilterOp::Equals => {
                    params.push(QueryParam::from_value(&filter.value)?);
                    format!("{} = ${}", filter.column, param_idx + 1)
                }
                FilterOp::NotEquals => {
                    params.push(QueryParam::from_value(&filter.value)?);
                    format!("{} != ${}", filter.column, param_idx + 1)
                }
                FilterOp::GreaterThan => {
                    params.push(QueryParam::from_value(&filter.value)?);
                    format!("{} > ${}", filter.column, param_idx + 1)
                }
                FilterOp::Contains => {
                    params.push(QueryParam::from_value(&filter.value)?);
                    format!("{} ILIKE '%' || ${} || '%'", filter.column, param_idx + 1)
                }
                FilterOp::In => {
                    match &filter.value {
                        FilterValue::Array(vals) => {
                            let placeholders: Vec<_> = vals
                                .iter()
                                .map(|v| {
                                    params.push(QueryParam::from_value(v)?);
                                    format!("${}", params.len())
                                })
                                .collect::<Result<_, _>>()?;
                            format!("{} IN ({})", filter.column, placeholders.join(", "))
                        }
                        _ => return Err(SqlGenerationError::InvalidFilterValue),
                    }
                }
                FilterOp::Between => {
                    match &filter.value {
                        FilterValue::Array(vals) if vals.len() == 2 => {
                            params.push(QueryParam::from_value(&vals[0])?);
                            params.push(QueryParam::from_value(&vals[1])?);
                            format!(
                                "{} BETWEEN ${} AND ${}",
                                filter.column,
                                params.len() - 1,
                                params.len()
                            )
                        }
                        _ => return Err(SqlGenerationError::InvalidFilterValue),
                    }
                }
                FilterOp::IsNull => {
                    format!("{} IS NULL", filter.column)
                }
                _ => return Err(SqlGenerationError::UnsupportedOperation),
            };

            clauses.push(clause);
        }

        Ok((format!("WHERE {}", clauses.join(" AND ")), params))
    }

    fn build_from(&self, intent: &RelationalIntent) -> Result<(String, Vec<String>), SqlGenerationError> {
        Ok((
            format!("FROM {}", intent.relation),
            vec![intent.relation.clone()],
        ))
    }

    async fn build_joins(&self, intent: &RelationalIntent) -> Result<String, SqlGenerationError> {
        if intent.joins.is_empty() {
            return Ok(String::new());
        }

        let mut join_parts = Vec::new();

        for join in &intent.joins {
            let join_type = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };

            let clause = format!(
                "{} {} ON {}.{} = {}.{}",
                join_type,
                join.right_table,
                intent.relation,
                join.left_column,
                join.right_table,
                join.right_column
            );

            join_parts.push(clause);
        }

        Ok(join_parts.join("\n"))
    }

    fn build_limit(&self, intent: &RelationalIntent) -> String {
        let limit = intent.limit.unwrap_or(100).min(self.config.max_limit);
        let offset = intent.offset.unwrap_or(0);

        if offset > 0 {
            format!("LIMIT {} OFFSET {}", limit, offset)
        } else {
            format!("LIMIT {}", limit)
        }
    }

    fn build_group_by(&self, intent: &RelationalIntent) -> Result<Option<String>, SqlGenerationError> {
        if let Some(agg) = &intent.aggregations {
            if agg.group_by.is_empty() {
                return Ok(None);
            }

            let groupby_cols = agg.group_by.join(", ");
            let aggregate_clauses: Result<Vec<_>, _> = agg
                .aggregates
                .iter()
                .map(|a| self.render_aggregate(a))
                .collect();

            let select_part = format!("{}, {}", groupby_cols, aggregate_clauses?.join(", "));

            return Ok(Some(format!("GROUP BY {}", groupby_cols)));
        }

        Ok(None)
    }

    fn render_aggregate(&self, agg: &AggregateFunc) -> Result<String, SqlGenerationError> {
        match agg {
            AggregateFunc::Count(col) => {
                let col_expr = col.as_deref().unwrap_or("*");
                Ok(format!("COUNT({})", col_expr))
            }
            AggregateFunc::Sum(col) => Ok(format!("SUM({})", col)),
            AggregateFunc::Avg(col) => Ok(format!("AVG({})", col)),
            AggregateFunc::Min(col) => Ok(format!("MIN({})", col)),
            AggregateFunc::Max(col) => Ok(format!("MAX({})", col)),
        }
    }

    fn apply_transform(&self, column: &str, transform: &ColumnTransform) -> Result<String, SqlGenerationError> {
        match transform {
            ColumnTransform::ToLowercase => Ok(format!("LOWER({})", column)),
            ColumnTransform::ToUppercase => Ok(format!("UPPER({})", column)),
            ColumnTransform::Cast(type_name) => Ok(format!("{}::{}", column, type_name)),
            ColumnTransform::Coalesce(cols) => {
                let col_list = cols.join(", ");
                Ok(format!("COALESCE({})", col_list))
            }
        }
    }
}
```

---

## 3. Connection Pooling & Transaction Support

### 3.1 Connection Pool Architecture

```rust
use deadpool_postgres::{Config, Pool, Manager};
use tokio_postgres::{IsolationLevel, Transaction as PgTransaction};

/// PostgreSQL connection pool with transaction lifecycle management
pub struct PostgresConnectionPool {
    pool: Arc<Pool>,
    config: PoolConfig,
    metrics: Arc<PoolMetrics>,
}

#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout (seconds)
    pub connection_timeout: u64,
    /// Idle timeout (seconds)
    pub idle_timeout: u64,
    /// Query timeout (seconds)
    pub query_timeout: u64,
}

#[derive(Clone, Default)]
pub struct PoolMetrics {
    pub connections_acquired: Arc<std::sync::atomic::AtomicU64>,
    pub connections_released: Arc<std::sync::atomic::AtomicU64>,
    pub query_errors: Arc<std::sync::atomic::AtomicU64>,
    pub transaction_commits: Arc<std::sync::atomic::AtomicU64>,
    pub transaction_rollbacks: Arc<std::sync::atomic::AtomicU64>,
}

impl PostgresConnectionPool {
    pub async fn new(
        connection_string: &str,
        config: PoolConfig,
    ) -> Result<Self, PoolError> {
        let mut cfg = Config::from_str(connection_string)?;
        cfg.manager = Some(Manager::new(
            tokio_postgres::Config::from_str(connection_string)?,
            tokio_native_tls::native_tls::TlsConnector::new()?,
        ));

        let pool = cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            cfg.clone(),
        )?;

        Ok(PostgresConnectionPool {
            pool: Arc::new(pool),
            config,
            metrics: Arc::new(PoolMetrics::default()),
        })
    }

    pub async fn execute_query(
        &self,
        query: &PreparedQuery,
    ) -> Result<QueryResult, ExecutionError> {
        let client = self.acquire_connection().await?;

        let params_ref: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = query
            .parameters
            .iter()
            .map(|p| p.as_ref())
            .collect();

        let timeout = tokio::time::Duration::from_secs(self.config.query_timeout);

        let rows = tokio::time::timeout(
            timeout,
            client.query(query.sql.as_str(), params_ref.as_slice()),
        )
        .await
        .map_err(|_| ExecutionError::QueryTimeout)??;

        self.metrics
            .connections_released
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(QueryResult {
            rows: Arc::new(rows),
            projections: query.projections.clone(),
        })
    }

    pub async fn begin_transaction(
        &self,
        isolation_level: TransactionIsolation,
    ) -> Result<ManagedTransaction, TransactionError> {
        let client = self.acquire_connection().await?;

        let pg_level = match isolation_level {
            TransactionIsolation::ReadCommitted => IsolationLevel::ReadCommitted,
            TransactionIsolation::RepeatableRead => IsolationLevel::RepeatableRead,
            TransactionIsolation::Serializable => IsolationLevel::Serializable,
        };

        let transaction = client.transaction_with_isolation(pg_level).await?;

        Ok(ManagedTransaction {
            inner: transaction,
            metrics: Arc::clone(&self.metrics),
            is_committed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    async fn acquire_connection(&self) -> Result<deadpool_postgres::Client, ExecutionError> {
        let timeout = tokio::time::Duration::from_secs(self.config.connection_timeout);

        let client = tokio::time::timeout(
            timeout,
            self.pool.get(),
        )
        .await
        .map_err(|_| ExecutionError::PoolTimeout)??;

        self.metrics
            .connections_acquired
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(client)
    }
}

#[derive(Debug, Clone)]
pub enum TransactionIsolation {
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

pub struct ManagedTransaction {
    inner: PgTransaction<'static>,
    metrics: Arc<PoolMetrics>,
    is_committed: Arc<std::sync::atomic::AtomicBool>,
}

impl ManagedTransaction {
    pub async fn query(
        &self,
        query: &PreparedQuery,
    ) -> Result<QueryResult, ExecutionError> {
        let params_ref: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = query
            .parameters
            .iter()
            .map(|p| p.as_ref())
            .collect();

        let rows = self
            .inner
            .query(query.sql.as_str(), params_ref.as_slice())
            .await?;

        Ok(QueryResult {
            rows: Arc::new(rows),
            projections: query.projections.clone(),
        })
    }

    pub async fn commit(self) -> Result<(), TransactionError> {
        self.inner.commit().await?;
        self.is_committed.store(true, std::sync::atomic::Ordering::Release);
        self.metrics
            .transaction_commits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub async fn rollback(self) -> Result<(), TransactionError> {
        self.inner.rollback().await?;
        self.is_committed.store(false, std::sync::atomic::Ordering::Release);
        self.metrics
            .transaction_rollbacks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
```

---

## 4. Schema Introspection & Metadata Caching

### 4.1 Schema Cache Implementation

```rust
use dashmap::DashMap;
use std::time::{SystemTime, Duration};

/// Cached PostgreSQL schema metadata
#[derive(Clone, Debug)]
pub struct SchemaMetadata {
    pub relation_name: String,
    pub columns: HashMap<String, ColumnMetadata>,
    pub primary_keys: Vec<String>,
    pub indexes: Vec<IndexMetadata>,
    pub cached_at: SystemTime,
}

#[derive(Clone, Debug)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: PostgresDataType,
    pub is_nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostgresDataType {
    SmallInt,
    Integer,
    BigInt,
    Decimal,
    Numeric,
    Real,
    DoublePrecision,
    Text,
    Varchar,
    Char,
    Boolean,
    Date,
    Time,
    Timestamp,
    TimestampWithTimezone,
    UUID,
    Json,
    Jsonb,
    ByteA,
    Array(Box<PostgresDataType>),
    Custom(String),
}

#[derive(Clone, Debug)]
pub struct IndexMetadata {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

pub struct SchemaCache {
    cache: Arc<DashMap<String, SchemaMetadata>>,
    pool: Arc<PostgresConnectionPool>,
    ttl: Duration,
}

impl SchemaCache {
    pub fn new(pool: Arc<PostgresConnectionPool>, ttl_seconds: u64) -> Self {
        SchemaCache {
            cache: Arc::new(DashMap::new()),
            pool,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn get(&self, table_name: &str) -> Result<SchemaMetadata, CacheError> {
        // Check cache first
        if let Some(entry) = self.cache.get(table_name) {
            let elapsed = entry.cached_at.elapsed().unwrap_or(Duration::MAX);
            if elapsed < self.ttl {
                tracing::debug!(table = table_name, "Schema cache hit");
                return Ok(entry.clone());
            }
        }

        // Cache miss or expired; introspect schema
        tracing::debug!(table = table_name, "Schema cache miss; introspecting");
        let metadata = self.introspect_table(table_name).await?;
        self.cache.insert(table_name.to_string(), metadata.clone());

        Ok(metadata)
    }

    async fn introspect_table(&self, table_name: &str) -> Result<SchemaMetadata, CacheError> {
        // Query information_schema for columns
        let column_query = r#"
            SELECT column_name, data_type, is_nullable, column_default
            FROM information_schema.columns
            WHERE table_name = $1
            ORDER BY ordinal_position
        "#;

        let prepared = PreparedQuery {
            sql: Arc::new(column_query.to_string()),
            parameters: vec![QueryParam::String(table_name.to_string())],
            projections: vec!["column_name".to_string(), "data_type".to_string()],
            tables: vec!["information_schema.columns".to_string()],
        };

        let result = self.pool.execute_query(&prepared).await?;

        let mut columns = HashMap::new();
        for row in result.rows.iter() {
            let col_name: String = row.get(0);
            let data_type: String = row.get(1);
            let is_nullable: String = row.get(2);
            let col_default: Option<String> = row.get(3);

            columns.insert(
                col_name.clone(),
                ColumnMetadata {
                    name: col_name,
                    data_type: Self::parse_pg_type(&data_type)?,
                    is_nullable: is_nullable == "YES",
                    default_value: col_default,
                },
            );
        }

        // Query for primary keys
        let pk_query = r#"
            SELECT a.attname
            FROM pg_index i
            JOIN pg_attribute a ON a.attrelid = i.indrelid
            WHERE i.indisprimary AND i.indrelid = $1::regclass
        "#;

        let pk_prepared = PreparedQuery {
            sql: Arc::new(pk_query.to_string()),
            parameters: vec![QueryParam::String(table_name.to_string())],
            projections: vec!["attname".to_string()],
            tables: vec![],
        };

        let pk_result = self.pool.execute_query(&pk_prepared).await?;
        let primary_keys: Vec<String> = pk_result
            .rows
            .iter()
            .map(|row| row.get(0))
            .collect();

        Ok(SchemaMetadata {
            relation_name: table_name.to_string(),
            columns,
            primary_keys,
            indexes: Vec::new(),
            cached_at: SystemTime::now(),
        })
    }

    fn parse_pg_type(type_str: &str) -> Result<PostgresDataType, CacheError> {
        match type_str {
            "smallint" => Ok(PostgresDataType::SmallInt),
            "integer" => Ok(PostgresDataType::Integer),
            "bigint" => Ok(PostgresDataType::BigInt),
            "text" => Ok(PostgresDataType::Text),
            "varchar" => Ok(PostgresDataType::Varchar),
            "boolean" => Ok(PostgresDataType::Boolean),
            "timestamp with time zone" => Ok(PostgresDataType::TimestampWithTimezone),
            "timestamp" => Ok(PostgresDataType::Timestamp),
            "date" => Ok(PostgresDataType::Date),
            "uuid" => Ok(PostgresDataType::UUID),
            "jsonb" => Ok(PostgresDataType::Jsonb),
            "json" => Ok(PostgresDataType::Json),
            s if s.contains("[]") => {
                let inner = s.trim_end_matches("[]");
                let inner_type = Box::new(Self::parse_pg_type(inner)?);
                Ok(PostgresDataType::Array(inner_type))
            }
            _ => Ok(PostgresDataType::Custom(type_str.to_string())),
        }
    }

    pub async fn contains(&self, table_name: &str) -> bool {
        match self.get(table_name).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn invalidate(&self, table_name: &str) {
        self.cache.remove(table_name);
        tracing::debug!(table = table_name, "Schema cache invalidated");
    }

    pub fn clear(&self) {
        self.cache.clear();
        tracing::debug!("Schema cache cleared");
    }
}
```

---

## 5. Query Result Normalization

### 5.1 Result Transformation Pipeline

```rust
use serde_json::{json, Value};

/// Normalized semantic result set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedResultSet {
    pub rows: Vec<SemanticRow>,
    pub schema: ResultSchema,
    pub metadata: ResultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRow {
    pub columns: HashMap<String, SemanticValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SemanticValue {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Timestamp(String),
    Json(serde_json::Value),
    Array(Vec<SemanticValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSchema {
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub semantic_type: SemanticDataType,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticDataType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Identifier,
    Json,
    Array(Box<SemanticDataType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMetadata {
    pub row_count: usize,
    pub execution_time_ms: u64,
    pub source_relation: String,
}

pub struct ResultNormalizer {
    schema_cache: Arc<SchemaCache>,
}

impl ResultNormalizer {
    pub fn new(schema_cache: Arc<SchemaCache>) -> Self {
        ResultNormalizer { schema_cache }
    }

    pub async fn normalize(
        &self,
        query_result: QueryResult,
        intent: &RelationalIntent,
        execution_time_ms: u64,
    ) -> Result<NormalizedResultSet, NormalizationError> {
        let row_count = query_result.rows.len();

        // Build result schema from projections and type metadata
        let schema = self
            .build_result_schema(&query_result, intent)
            .await?;

        // Transform each row
        let normalized_rows: Result<Vec<_>, _> = query_result
            .rows
            .iter()
            .enumerate()
            .map(|(idx, row)| self.normalize_row(row, &schema, idx))
            .collect();

        Ok(NormalizedResultSet {
            rows: normalized_rows?,
            schema,
            metadata: ResultMetadata {
                row_count,
                execution_time_ms,
                source_relation: intent.relation.clone(),
            },
        })
    }

    async fn build_result_schema(
        &self,
        result: &QueryResult,
        intent: &RelationalIntent,
    ) -> Result<ResultSchema, NormalizationError> {
        let table_schema = self.schema_cache.get(&intent.relation).await?;

        let mut columns = Vec::new();

        for (col_idx, col_name) in result.projections.iter().enumerate() {
            if col_name == "*" {
                // Expand * into individual columns from schema
                for (col_key, col_meta) in &table_schema.columns {
                    columns.push(ColumnSchema {
                        name: col_key.clone(),
                        semantic_type: self.map_pg_to_semantic(&col_meta.data_type),
                        nullable: col_meta.is_nullable,
                    });
                }
            } else {
                let col_meta = table_schema
                    .columns
                    .get(col_name)
                    .ok_or(NormalizationError::ColumnNotFound(col_name.clone()))?;

                columns.push(ColumnSchema {
                    name: col_name.clone(),
                    semantic_type: self.map_pg_to_semantic(&col_meta.data_type),
                    nullable: col_meta.is_nullable,
                });
            }
        }

        Ok(ResultSchema { columns })
    }

    fn normalize_row(
        &self,
        pg_row: &tokio_postgres::Row,
        schema: &ResultSchema,
        _row_idx: usize,
    ) -> Result<SemanticRow, NormalizationError> {
        let mut columns = HashMap::new();

        for (col_idx, col_schema) in schema.columns.iter().enumerate() {
            let value = self.normalize_value(pg_row, col_idx, &col_schema.semantic_type)?;
            columns.insert(col_schema.name.clone(), value);
        }

        Ok(SemanticRow { columns })
    }

    fn normalize_value(
        &self,
        row: &tokio_postgres::Row,
        col_idx: usize,
        semantic_type: &SemanticDataType,
    ) -> Result<SemanticValue, NormalizationError> {
        if let Ok(None) = row.try_get::<_, Option<String>>(col_idx) {
            return Ok(SemanticValue::Null);
        }

        match semantic_type {
            SemanticDataType::String => {
                let val: String = row.get(col_idx);
                Ok(SemanticValue::String(val))
            }
            SemanticDataType::Integer => {
                let val: i64 = row.get(col_idx);
                Ok(SemanticValue::Integer(val))
            }
            SemanticDataType::Float => {
                let val: f64 = row.get(col_idx);
                Ok(SemanticValue::Float(val))
            }
            SemanticDataType::Boolean => {
                let val: bool = row.get(col_idx);
                Ok(SemanticValue::Boolean(val))
            }
            SemanticDataType::DateTime => {
                let val: chrono::DateTime<chrono::Utc> = row.get(col_idx);
                Ok(SemanticValue::Timestamp(val.to_rfc3339()))
            }
            SemanticDataType::Json => {
                let val: serde_json::Value = row.get(col_idx);
                Ok(SemanticValue::Json(val))
            }
            _ => Err(NormalizationError::UnsupportedType),
        }
    }

    fn map_pg_to_semantic(&self, pg_type: &PostgresDataType) -> SemanticDataType {
        match pg_type {
            PostgresDataType::SmallInt
            | PostgresDataType::Integer
            | PostgresDataType::BigInt => SemanticDataType::Integer,
            PostgresDataType::Real | PostgresDataType::DoublePrecision => SemanticDataType::Float,
            PostgresDataType::Text
            | PostgresDataType::Varchar
            | PostgresDataType::Char => SemanticDataType::String,
            PostgresDataType::Boolean => SemanticDataType::Boolean,
            PostgresDataType::Timestamp
            | PostgresDataType::TimestampWithTimezone
            | PostgresDataType::Date
            | PostgresDataType::Time => SemanticDataType::DateTime,
            PostgresDataType::Json | PostgresDataType::Jsonb => SemanticDataType::Json,
            PostgresDataType::Array(inner) => {
                SemanticDataType::Array(Box::new(self.map_pg_to_semantic(inner)))
            }
            _ => SemanticDataType::String,
        }
    }
}
```

---

## 6. Integration with Semantic FS Mount Layer

### 6.1 PostgreSQL Mount Handler

```rust
/// PostgreSQL mount implementation within unified mount dispatcher
#[async_trait::async_trait]
pub trait SemanticVolumeMount: Send + Sync {
    async fn query(&self, intent: &RelationalIntent) -> Result<NormalizedResultSet, MountError>;
    async fn mount(&self) -> Result<MountMetadata, MountError>;
    async fn unmount(&self) -> Result<(), MountError>;
}

pub struct PostgresVolumeMount {
    mount_point: String,
    pool: Arc<PostgresConnectionPool>,
    schema_cache: Arc<SchemaCache>,
    sql_generator: Arc<SafeSqlGenerator>,
    result_normalizer: Arc<ResultNormalizer>,
    metrics: Arc<PostgresMountMetrics>,
}

#[derive(Default)]
pub struct PostgresMountMetrics {
    pub queries_executed: Arc<std::sync::atomic::AtomicU64>,
    pub queries_failed: Arc<std::sync::atomic::AtomicU64>,
    pub total_rows_returned: Arc<std::sync::atomic::AtomicU64>,
    pub avg_query_time_ms: Arc<std::sync::atomic::AtomicU64>,
}

#[async_trait::async_trait]
impl SemanticVolumeMount for PostgresVolumeMount {
    async fn query(&self, intent: &RelationalIntent) -> Result<NormalizedResultSet, MountError> {
        let start = std::time::Instant::now();

        // Generate parameterized SQL from semantic intent
        let prepared_query = self
            .sql_generator
            .generate(intent)
            .await
            .map_err(MountError::SqlGeneration)?;

        // Execute query with timeout
        let result = self
            .pool
            .execute_query(&prepared_query)
            .await
            .map_err(MountError::QueryExecution)?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Normalize result to semantic representation
        let normalized = self
            .result_normalizer
            .normalize(result, intent, execution_time_ms)
            .await
            .map_err(MountError::Normalization)?;

        // Update metrics
        self.metrics
            .queries_executed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .total_rows_returned
            .fetch_add(normalized.metadata.row_count as u64, std::sync::atomic::Ordering::Relaxed);

        tracing::info!(
            relation = intent.relation,
            row_count = normalized.metadata.row_count,
            execution_time_ms = execution_time_ms,
            "Query executed successfully"
        );

        Ok(normalized)
    }

    async fn mount(&self) -> Result<MountMetadata, MountError> {
        // Verify connectivity and readiness
        let schema_count = self.schema_cache.introspect_all().await?.len();

        Ok(MountMetadata {
            mount_point: self.mount_point.clone(),
            source_type: "postgresql".to_string(),
            available_tables: schema_count,
            mounted_at: SystemTime::now(),
        })
    }

    async fn unmount(&self) -> Result<(), MountError> {
        self.schema_cache.clear();
        self.pool.close().await?;
        tracing::info!(mount_point = self.mount_point, "PostgreSQL mount unmounted");
        Ok(())
    }
}
```

---

## 7. Error Handling & Observability

```rust
#[derive(Debug, thiserror::Error)]
pub enum SqlGenerationError {
    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Unsupported filter operation")]
    UnsupportedOperation,

    #[error("Invalid filter value")]
    InvalidFilterValue,

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Query timeout")]
    QueryTimeout,

    #[error("Connection pool timeout")]
    PoolTimeout,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Parameter binding error: {0}")]
    ParameterError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Transaction already committed")]
    AlreadyCommitted,

    #[error("Transaction isolation violation")]
    IsolationViolation,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NormalizationError {
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Unsupported type conversion")]
    UnsupportedType,

    #[error("Schema error: {0}")]
    SchemaError(String),
}
```

---

## 8. Test Suite Requirements

### 8.1 Test Coverage Areas

1. **Semantic Intent Parsing** - NL → RelationalIntent translation
2. **SQL Generation** - Parameterized SQL with injection prevention
3. **Connection Pooling** - Concurrency, timeout, resource cleanup
4. **Transaction Semantics** - Isolation levels, commit/rollback
5. **Schema Introspection** - Metadata caching and invalidation
6. **Result Normalization** - Type mapping, NULL handling
7. **Error Scenarios** - Timeout, constraint violations, schema mismatch
8. **Integration Tests** - Full query pipeline with real PostgreSQL

---

## 9. Deployment & Credential Management

### 9.1 Secure Credential Handling

- PostgreSQL connection strings stored in XKernal secrets manager
- Credential rotation coordinated with connection pool lifecycle
- TLS enforcement for remote connections
- Read-only query credentials for safety

---

## 10. Roadmap & Future Extensions

- **Week 17-18:** Document store mounting (MongoDB, Elasticsearch)
- **Week 19-20:** Query federation across multiple mounts
- **Phase 3:** Agent-driven schema exploration and optimization
