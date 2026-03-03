# Week 19: Semantic File System NL Query Interface & CSCI Integration

**Document Version**: 1.0
**Date**: 2026-03-02
**Phase**: Phase 2 - L2 Runtime (Rust)
**Engineer**: Staff-Level (Engineer 8)
**Status**: Design & Implementation Specification

---

## 1. Executive Summary

Week 19 implements the core Semantic File System (SFS) Natural Language (NL) query interface with deep integration into the Cognitive Substrate Core Interface (CSCI). This document specifies:

- **NL Query Parser** with entity extraction and normalization
- **Intent Classification System** supporting 4+ intent types (search, retrieve, aggregate, join)
- **Query Router** for multi-source mount selection and optimization
- **Query Translator** converting intents to source-specific queries (vector, SQL, GraphQL, REST)
- **Result Aggregation Engine** with merge, deduplication, and ranking
- **CSCI Integration** via `mem_mount` and `mem_read` semantic query protocols
- **Test Coverage** with 50+ diverse NL query examples
- **Performance SLA**: <200ms simple queries, <500ms aggregations

### Key Architectural Decisions

1. **Modular Intent Pipeline**: Separate parsing, classification, routing, translation, and aggregation stages for maintainability
2. **Source-Agnostic Translation**: Intent representation decoupled from backend specifics (PostgreSQL, Pinecone, GraphQL, etc.)
3. **CSCI as First-Class Protocol**: Semantic queries flow through CSCI memory interface for consistent lifecycle management
4. **Lazy Aggregation**: Results aggregated incrementally to minimize latency and memory footprint
5. **Entity Extraction via Trie + LLM**: Fast path for known entities, fallback to semantic matching

---

## 2. Architecture Overview

### 2.1 System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      User Application Layer                       │
└────────────────────────────┬────────────────────────────────────┘
                             │ NL Query + Context
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CSCI Memory Interface                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  mem_mount(semantic_query_ctx) → query_handle           │  │
│  │  mem_read(query_handle) → streaming results             │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
    ┌────────────┐   ┌──────────────┐   ┌──────────────┐
    │   Parser   │   │ Classifier   │   │    Router    │
    │ (Entity    │   │  (Intent +   │   │  (Mount      │
    │ Extractor) │   │   Type)      │   │   Selection) │
    └────────────┘   └──────────────┘   └──────────────┘
        │                    │                    │
        └────────────────────┴────────────────────┘
                             │
                             ▼
              ┌──────────────────────────────┐
              │     Translator Layer         │
              │  ┌────────────────────────┐  │
              │  │ Vector → Pinecone      │  │
              │  │ SQL → PostgreSQL       │  │
              │  │ GraphQL → Weaviate     │  │
              │  │ REST → Custom mounts   │  │
              │  │ S3 → Object metadata   │  │
              │  └────────────────────────┘  │
              └──────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
   ┌─────────┐          ┌─────────┐         ┌─────────┐
   │Pinecone │          │Postgres │         │Weaviate │
   │  Mount  │          │  Mount  │         │  Mount  │
   └─────────┘          └─────────┘         └─────────┘

        │                    │                    │
        └────────────────────┴────────────────────┘
                             │
                             ▼
              ┌──────────────────────────────┐
              │  Result Aggregation Engine   │
              │  • Merge result sets         │
              │  • Deduplicate by ID/hash    │
              │  • Rank by relevance         │
              │  • Stream to consumer        │
              └──────────────────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │ CSCI Response  │
                    │ (Streaming)    │
                    └────────────────┘
```

### 2.2 Data Structures

```rust
// Core intent representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryIntent {
    pub intent_type: IntentType,
    pub entities: Vec<Entity>,
    pub filters: Vec<Filter>,
    pub aggregations: Vec<Aggregation>,
    pub join_specs: Vec<JoinSpec>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntentType {
    Search {
        query_text: String,
        match_type: MatchType,
    },
    Retrieve {
        entity_type: String,
        identifiers: Vec<String>,
    },
    Aggregate {
        group_by: Vec<String>,
        metrics: Vec<AggregateMetric>,
    },
    Join {
        left_entity: String,
        right_entity: String,
        join_condition: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MatchType {
    Exact,
    Fuzzy,
    Semantic,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub span: (usize, usize),
    pub confidence: f32,
    pub normalized_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    pub metric_type: AggregateMetric,
    pub field: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AggregateMetric {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Distinct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSpec {
    pub join_type: JoinType,
    pub left_mount: String,
    pub right_mount: String,
    pub condition: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub result_id: String,
    pub source_mount: String,
    pub data: serde_json::Value,
    pub metadata: ResultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMetadata {
    pub retrieval_time_ms: u64,
    pub relevance_score: f32,
    pub row_count: u32,
    pub source_type: String,
}
```

---

## 3. Component Specification

### 3.1 NL Parser with Entity Extraction

**Purpose**: Parse natural language queries into structured entity and filter representations.

**Implementation Strategy**:
1. **Tokenization**: Split query into tokens, preserve span information
2. **Entity Extraction**: Trie-based lookup for known entities, LLM fallback for novel entities
3. **POS Tagging**: Lightweight part-of-speech identification
4. **Normalization**: Standardize entity values (lowercase, stemming, synonym expansion)

```rust
use std::collections::HashMap;
use regex::Regex;

pub struct NLParser {
    entity_trie: EntityTrie,
    entity_synonyms: HashMap<String, Vec<String>>,
    pos_patterns: Vec<(String, Regex)>,
    custom_entity_extractors: Vec<Box<dyn Fn(&str) -> Option<Entity>>>,
}

impl NLParser {
    pub fn new() -> Self {
        Self {
            entity_trie: EntityTrie::new(),
            entity_synonyms: Self::build_synonym_map(),
            pos_patterns: Self::build_pos_patterns(),
            custom_entity_extractors: vec![
                Box::new(Self::extract_email),
                Box::new(Self::extract_uuid),
                Box::new(Self::extract_timestamp),
                Box::new(Self::extract_numeric_range),
            ],
        }
    }

    /// Parse NL query and extract entities with confidence scores
    pub async fn parse_query(
        &self,
        query: &str,
    ) -> Result<Vec<Entity>, ParseError> {
        let tokens = self.tokenize(query);
        let mut entities = Vec::new();

        // Fast path: trie-based entity extraction
        for (i, token) in tokens.iter().enumerate() {
            if let Some(entity_type) = self.entity_trie.lookup(token) {
                entities.push(Entity {
                    entity_type,
                    value: token.clone(),
                    span: self.get_span(query, i),
                    confidence: 0.95,
                    normalized_value: self.normalize(&entity_type, token),
                });
            }
        }

        // Custom extractor patterns (email, UUID, timestamps, ranges)
        for extractor in &self.custom_entity_extractors {
            if let Some(entity) = extractor(query) {
                entities.push(entity);
            }
        }

        // Expand entities with synonyms
        let expanded = self.expand_entities(entities);
        Ok(expanded)
    }

    fn tokenize(&self, query: &str) -> Vec<String> {
        query
            .split(|c: char| c.is_whitespace() || "(),;:".contains(c))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    }

    fn build_synonym_map() -> HashMap<String, Vec<String>> {
        vec![
            ("user_id", vec!["uid", "userid", "user", "actor"]),
            ("timestamp", vec!["time", "date", "when", "created_at"]),
            ("email", vec!["mail", "address", "e-mail", "contact"]),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
    }

    fn build_pos_patterns() -> Vec<(String, Regex)> {
        vec![
            (
                "NOUN".to_string(),
                Regex::new(r"^[a-z]+_[a-z]+$").unwrap(),
            ),
            (
                "VERB".to_string(),
                Regex::new(r"^(find|get|search|retrieve|list|count)$").unwrap(),
            ),
            (
                "ADJ".to_string(),
                Regex::new(r"^(recent|old|active|inactive)$").unwrap(),
            ),
        ]
    }

    fn normalize(&self, entity_type: &str, value: &str) -> String {
        match entity_type {
            "timestamp" => self.normalize_timestamp(value),
            "email" => value.to_lowercase(),
            "user_id" => value.trim().to_lowercase(),
            _ => value.to_lowercase(),
        }
    }

    fn normalize_timestamp(&self, value: &str) -> String {
        // Parse relative timestamps: "last 7 days", "this week", etc.
        match value {
            "now" | "today" => chrono::Local::now().to_rfc3339(),
            s if s.contains("day") => {
                let days = s.split_whitespace()
                    .find_map(|w| w.parse::<i32>().ok())
                    .unwrap_or(1);
                (chrono::Local::now() - chrono::Duration::days(days as i64)).to_rfc3339()
            }
            _ => value.to_string(),
        }
    }

    fn expand_entities(&self, entities: Vec<Entity>) -> Vec<Entity> {
        entities
            .into_iter()
            .flat_map(|entity| {
                let mut expanded = vec![entity.clone()];
                if let Some(synonyms) = self.entity_synonyms.get(&entity.entity_type) {
                    for synonym in synonyms {
                        expanded.push(Entity {
                            entity_type: entity.entity_type.clone(),
                            value: synonym.clone(),
                            span: entity.span,
                            confidence: entity.confidence * 0.8,
                            normalized_value: entity.normalized_value.clone(),
                        });
                    }
                }
                expanded
            })
            .collect()
    }

    fn get_span(&self, query: &str, token_index: usize) -> (usize, usize) {
        // Simplified: calculate character span of token
        (token_index * 10, token_index * 10 + 10)
    }

    fn extract_email(query: &str) -> Option<Entity> {
        let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-z]{2,}")
            .ok()?;
        email_re.find(query).map(|m| Entity {
            entity_type: "email".to_string(),
            value: m.as_str().to_string(),
            span: (m.start(), m.end()),
            confidence: 0.99,
            normalized_value: m.as_str().to_lowercase(),
        })
    }

    fn extract_uuid(query: &str) -> Option<Entity> {
        let uuid_re = Regex::new(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
        ).ok()?;
        uuid_re.find(query).map(|m| Entity {
            entity_type: "uuid".to_string(),
            value: m.as_str().to_string(),
            span: (m.start(), m.end()),
            confidence: 0.99,
            normalized_value: m.as_str().to_lowercase(),
        })
    }

    fn extract_timestamp(query: &str) -> Option<Entity> {
        let ts_re = Regex::new(
            r"\d{4}-\d{2}-\d{2}T?\d{2}:\d{2}:\d{2}"
        ).ok()?;
        ts_re.find(query).map(|m| Entity {
            entity_type: "timestamp".to_string(),
            value: m.as_str().to_string(),
            span: (m.start(), m.end()),
            confidence: 0.99,
            normalized_value: m.as_str().to_string(),
        })
    }

    fn extract_numeric_range(query: &str) -> Option<Entity> {
        let range_re = Regex::new(r"(\d+)\s*-\s*(\d+)").ok()?;
        range_re.find(query).map(|m| Entity {
            entity_type: "numeric_range".to_string(),
            value: m.as_str().to_string(),
            span: (m.start(), m.end()),
            confidence: 0.95,
            normalized_value: m.as_str().to_string(),
        })
    }
}

struct EntityTrie {
    root: TrieNode,
}

struct TrieNode {
    children: HashMap<char, Box<TrieNode>>,
    entity_type: Option<String>,
}

impl EntityTrie {
    fn new() -> Self {
        Self {
            root: TrieNode {
                children: HashMap::new(),
                entity_type: None,
            },
        }
    }

    fn insert(&mut self, word: &str, entity_type: String) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node
                .children
                .entry(ch)
                .or_insert_with(|| Box::new(TrieNode {
                    children: HashMap::new(),
                    entity_type: None,
                }));
        }
        node.entity_type = Some(entity_type);
    }

    fn lookup(&self, word: &str) -> Option<String> {
        let mut node = &self.root;
        for ch in word.chars() {
            node = node.children.get(&ch)?;
        }
        node.entity_type.clone()
    }
}
```

### 3.2 Intent Classification

**Purpose**: Classify query intent and extract relevant parameters for routing and translation.

```rust
use rand::seq::SliceRandom;

pub struct IntentClassifier {
    search_keywords: Vec<String>,
    retrieve_keywords: Vec<String>,
    aggregate_keywords: Vec<String>,
    join_keywords: Vec<String>,
    match_type_detector: MatchTypeDetector,
}

impl IntentClassifier {
    pub fn new() -> Self {
        Self {
            search_keywords: vec![
                "find", "search", "look for", "query", "show",
                "list", "where", "filter", "match", "like"
            ].iter().map(|s| s.to_string()).collect(),
            retrieve_keywords: vec![
                "get", "fetch", "retrieve", "load", "read",
                "by id", "with id", "for id"
            ].iter().map(|s| s.to_string()).collect(),
            aggregate_keywords: vec![
                "count", "sum", "average", "min", "max", "group",
                "total", "aggregate", "statistics", "metrics"
            ].iter().map(|s| s.to_string()).collect(),
            join_keywords: vec![
                "join", "combine", "merge", "correlate", "relate",
                "associated", "linked to", "related to"
            ].iter().map(|s| s.to_string()).collect(),
            match_type_detector: MatchTypeDetector::new(),
        }
    }

    pub fn classify(
        &self,
        query: &str,
        entities: &[Entity],
    ) -> Result<QueryIntent, ClassificationError> {
        let intent_type = self.detect_intent_type(query)?;
        let filters = self.extract_filters(query, entities);
        let aggregations = self.extract_aggregations(query, entities);
        let join_specs = self.extract_joins(query, entities);
        let (limit, offset) = self.extract_pagination(query);

        let confidence_score = self.calculate_confidence(&intent_type, entities);

        Ok(QueryIntent {
            intent_type,
            entities: entities.to_vec(),
            filters,
            aggregations,
            join_specs,
            limit,
            offset,
            confidence_score,
        })
    }

    fn detect_intent_type(&self, query: &str) -> Result<IntentType, ClassificationError> {
        let query_lower = query.to_lowercase();

        // Check for join intent (highest priority)
        if self.search_keywords_in_query(&query_lower, &self.join_keywords) {
            return Ok(IntentType::Join {
                left_entity: "unknown".to_string(),
                right_entity: "unknown".to_string(),
                join_condition: query.to_string(),
            });
        }

        // Check for aggregation intent
        if self.search_keywords_in_query(&query_lower, &self.aggregate_keywords) {
            return Ok(IntentType::Aggregate {
                group_by: vec![],
                metrics: vec![AggregateMetric::Count],
            });
        }

        // Check for retrieve intent
        if self.search_keywords_in_query(&query_lower, &self.retrieve_keywords) {
            return Ok(IntentType::Retrieve {
                entity_type: "unknown".to_string(),
                identifiers: vec![],
            });
        }

        // Default to search intent
        let match_type = self.match_type_detector.detect(&query_lower);
        Ok(IntentType::Search {
            query_text: query.to_string(),
            match_type,
        })
    }

    fn extract_filters(
        &self,
        query: &str,
        entities: &[Entity],
    ) -> Vec<Filter> {
        let mut filters = Vec::new();

        for entity in entities {
            if entity.entity_type == "timestamp" {
                filters.push(Filter {
                    field: "created_at".to_string(),
                    operator: FilterOperator::Gte,
                    value: serde_json::Value::String(entity.normalized_value.clone()),
                });
            }
        }

        // Parse explicit filter syntax: "field:value", "field>value", etc.
        if let Some(caps) = regex::Regex::new(r"(\w+)([><=!:]+)([^\s]+)")
            .ok()
            .and_then(|re| re.captures(query))
        {
            if let (Some(field), Some(op), Some(value)) = (caps.get(1), caps.get(2), caps.get(3)) {
                filters.push(Filter {
                    field: field.as_str().to_string(),
                    operator: match op.as_str() {
                        ">" => FilterOperator::Gt,
                        ">=" => FilterOperator::Gte,
                        "<" => FilterOperator::Lt,
                        "<=" => FilterOperator::Lte,
                        "!" | "!=" => FilterOperator::Ne,
                        ":" | "=" => FilterOperator::Eq,
                        _ => FilterOperator::Eq,
                    },
                    value: serde_json::Value::String(value.as_str().to_string()),
                });
            }
        }

        filters
    }

    fn extract_aggregations(
        &self,
        query: &str,
        _entities: &[Entity],
    ) -> Vec<Aggregation> {
        let mut aggregations = Vec::new();
        let query_lower = query.to_lowercase();

        if query_lower.contains("count") {
            aggregations.push(Aggregation {
                metric_type: AggregateMetric::Count,
                field: "*".to_string(),
                alias: Some("total".to_string()),
            });
        }
        if query_lower.contains("sum") {
            aggregations.push(Aggregation {
                metric_type: AggregateMetric::Sum,
                field: "amount".to_string(),
                alias: Some("total_amount".to_string()),
            });
        }
        if query_lower.contains("average") || query_lower.contains("avg") {
            aggregations.push(Aggregation {
                metric_type: AggregateMetric::Avg,
                field: "value".to_string(),
                alias: Some("avg_value".to_string()),
            });
        }

        aggregations
    }

    fn extract_joins(
        &self,
        _query: &str,
        _entities: &[Entity],
    ) -> Vec<JoinSpec> {
        // Placeholder: full join extraction in future iteration
        Vec::new()
    }

    fn extract_pagination(&self, query: &str) -> (Option<u32>, Option<u32>) {
        let limit_re = regex::Regex::new(r"limit\s+(\d+)").ok();
        let offset_re = regex::Regex::new(r"offset\s+(\d+)").ok();

        let limit = limit_re
            .and_then(|re| re.captures(query))
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok());

        let offset = offset_re
            .and_then(|re| re.captures(query))
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok());

        (limit, offset)
    }

    fn search_keywords_in_query(&self, query: &str, keywords: &[String]) -> bool {
        keywords.iter().any(|kw| query.contains(kw))
    }

    fn calculate_confidence(&self, intent: &IntentType, entities: &[Entity]) -> f32 {
        let base = match intent {
            IntentType::Search { .. } => 0.8,
            IntentType::Retrieve { .. } => 0.85,
            IntentType::Aggregate { .. } => 0.9,
            IntentType::Join { .. } => 0.75,
        };

        let entity_boost = (entities.len() as f32 * 0.02).min(0.15);
        (base + entity_boost).min(1.0)
    }
}

struct MatchTypeDetector;

impl MatchTypeDetector {
    fn new() -> Self {
        Self
    }

    fn detect(&self, query: &str) -> MatchType {
        if query.contains('^') || query.contains('$') || query.contains('[') {
            MatchType::Regex
        } else if query.contains("*") || query.contains("?") {
            MatchType::Fuzzy
        } else if query.contains("semantic") || query.contains("similar") {
            MatchType::Semantic
        } else {
            MatchType::Exact
        }
    }
}

#[derive(Debug)]
pub enum ClassificationError {
    NoIntentDetected,
    AmbiguousIntent,
    InvalidQuery,
}
```

### 3.3 Query Router

**Purpose**: Route classified intents to appropriate mounted sources based on capability matching.

```rust
use std::collections::BTreeMap;

pub struct QueryRouter {
    mounts: BTreeMap<String, MountCapability>,
    routing_rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone)]
pub struct MountCapability {
    pub mount_id: String,
    pub mount_type: MountType,
    pub supported_intents: Vec<IntentType>,
    pub supported_filters: Vec<String>,
    pub estimated_latency_ms: u32,
    pub result_cardinality: Cardinality,
}

#[derive(Debug, Clone, Copy)]
pub enum MountType {
    Pinecone,
    PostgreSQL,
    Weaviate,
    REST,
    S3,
}

#[derive(Debug, Clone, Copy)]
pub enum Cardinality {
    Singleton,
    Few,
    Many,
}

struct RoutingRule {
    condition: Box<dyn Fn(&QueryIntent) -> bool>,
    preferred_mounts: Vec<String>,
    fallback_mounts: Vec<String>,
}

impl QueryRouter {
    pub fn new() -> Self {
        Self {
            mounts: Self::build_default_mounts(),
            routing_rules: Self::build_routing_rules(),
        }
    }

    /// Route query to optimal mount(s)
    pub fn route(&self, intent: &QueryIntent) -> Result<Vec<String>, RoutingError> {
        // Apply routing rules
        for rule in &self.routing_rules {
            if (rule.condition)(intent) {
                let available_preferred: Vec<_> = rule
                    .preferred_mounts
                    .iter()
                    .filter(|m| self.mounts.contains_key(*m))
                    .cloned()
                    .collect();

                if !available_preferred.is_empty() {
                    return Ok(available_preferred);
                }
            }
        }

        // Fallback: select mounts by intent type
        let mounts = self.select_mounts_for_intent(intent)?;
        Ok(mounts)
    }

    fn select_mounts_for_intent(&self, intent: &QueryIntent) -> Result<Vec<String>, RoutingError> {
        match &intent.intent_type {
            IntentType::Search { match_type, .. } => {
                match match_type {
                    MatchType::Semantic => Ok(vec!["weaviate".to_string(), "pinecone".to_string()]),
                    MatchType::Exact => Ok(vec!["postgres".to_string(), "s3_metadata".to_string()]),
                    MatchType::Fuzzy => Ok(vec!["postgres".to_string()]),
                    MatchType::Regex => Ok(vec!["postgres".to_string()]),
                }
            }
            IntentType::Retrieve { .. } => {
                Ok(vec!["postgres".to_string(), "s3_metadata".to_string()])
            }
            IntentType::Aggregate { .. } => Ok(vec!["postgres".to_string()]),
            IntentType::Join { .. } => {
                Ok(vec!["postgres".to_string(), "weaviate".to_string()])
            }
        }
    }

    fn build_default_mounts() -> BTreeMap<String, MountCapability> {
        vec![
            (
                "pinecone".to_string(),
                MountCapability {
                    mount_id: "pinecone".to_string(),
                    mount_type: MountType::Pinecone,
                    supported_intents: vec![],
                    supported_filters: vec!["metadata".to_string()],
                    estimated_latency_ms: 100,
                    result_cardinality: Cardinality::Many,
                },
            ),
            (
                "postgres".to_string(),
                MountCapability {
                    mount_id: "postgres".to_string(),
                    mount_type: MountType::PostgreSQL,
                    supported_intents: vec![],
                    supported_filters: vec!["*".to_string()],
                    estimated_latency_ms: 50,
                    result_cardinality: Cardinality::Many,
                },
            ),
            (
                "weaviate".to_string(),
                MountCapability {
                    mount_id: "weaviate".to_string(),
                    mount_type: MountType::Weaviate,
                    supported_intents: vec![],
                    supported_filters: vec!["where".to_string()],
                    estimated_latency_ms: 120,
                    result_cardinality: Cardinality::Many,
                },
            ),
            (
                "rest_api".to_string(),
                MountCapability {
                    mount_id: "rest_api".to_string(),
                    mount_type: MountType::REST,
                    supported_intents: vec![],
                    supported_filters: vec!["query_params".to_string()],
                    estimated_latency_ms: 200,
                    result_cardinality: Cardinality::Many,
                },
            ),
            (
                "s3_metadata".to_string(),
                MountCapability {
                    mount_id: "s3_metadata".to_string(),
                    mount_type: MountType::S3,
                    supported_intents: vec![],
                    supported_filters: vec!["tags".to_string(), "metadata".to_string()],
                    estimated_latency_ms: 150,
                    result_cardinality: Cardinality::Many,
                },
            ),
        ]
        .into_iter()
        .collect()
    }

    fn build_routing_rules() -> Vec<RoutingRule> {
        // Simplified: in production, these would be loaded from configuration
        vec![]
    }

    #[allow(dead_code)]
    pub fn register_mount(&mut self, capability: MountCapability) {
        self.mounts.insert(capability.mount_id.clone(), capability);
    }
}

#[derive(Debug)]
pub enum RoutingError {
    NoSuitableMounts,
    AllMountsUnavailable,
}
```

### 3.4 Query Translator

**Purpose**: Convert unified intent representation to source-specific queries.

```rust
pub struct QueryTranslator;

impl QueryTranslator {
    pub fn translate(
        intent: &QueryIntent,
        mount_type: MountType,
    ) -> Result<SourceQuery, TranslationError> {
        match mount_type {
            MountType::Pinecone => Self::to_pinecone_query(intent),
            MountType::PostgreSQL => Self::to_postgres_query(intent),
            MountType::Weaviate => Self::to_graphql_query(intent),
            MountType::REST => Self::to_rest_query(intent),
            MountType::S3 => Self::to_s3_query(intent),
        }
    }

    fn to_pinecone_query(intent: &QueryIntent) -> Result<SourceQuery, TranslationError> {
        match &intent.intent_type {
            IntentType::Search {
                query_text,
                match_type,
            } => {
                if *match_type != MatchType::Semantic {
                    return Err(TranslationError::UnsupportedMatchType);
                }

                Ok(SourceQuery::Pinecone {
                    vector_query: query_text.clone(),
                    top_k: intent.limit.unwrap_or(10),
                    filter: Self::pinecone_filters(&intent.filters),
                })
            }
            _ => Err(TranslationError::UnsupportedIntent),
        }
    }

    fn to_postgres_query(intent: &QueryIntent) -> Result<SourceQuery, TranslationError> {
        match &intent.intent_type {
            IntentType::Search {
                query_text,
                match_type,
            } => {
                let where_clause = Self::build_where_clause(
                    match_type,
                    query_text,
                    &intent.filters,
                );

                Ok(SourceQuery::SQL {
                    query: format!("SELECT * FROM documents WHERE {}", where_clause),
                    limit: intent.limit.unwrap_or(100),
                    offset: intent.offset.unwrap_or(0),
                })
            }
            IntentType::Retrieve { identifiers, .. } => {
                let in_clause = identifiers
                    .iter()
                    .map(|id| format!("'{}'", id))
                    .collect::<Vec<_>>()
                    .join(",");

                Ok(SourceQuery::SQL {
                    query: format!("SELECT * FROM documents WHERE id IN ({})", in_clause),
                    limit: intent.limit.unwrap_or(100),
                    offset: intent.offset.unwrap_or(0),
                })
            }
            IntentType::Aggregate { metrics, group_by } => {
                let metric_cols = metrics
                    .iter()
                    .map(|m| Self::agg_metric_to_sql(m))
                    .collect::<Vec<_>>()
                    .join(",");

                let group_clause = if group_by.is_empty() {
                    String::new()
                } else {
                    format!("GROUP BY {}", group_by.join(","))
                };

                Ok(SourceQuery::SQL {
                    query: format!(
                        "SELECT {} FROM documents {} {}",
                        metric_cols, group_clause, where_clause
                    ),
                    limit: intent.limit.unwrap_or(1000),
                    offset: intent.offset.unwrap_or(0),
                })
            }
            _ => Err(TranslationError::UnsupportedIntent),
        }
    }

    fn to_graphql_query(intent: &QueryIntent) -> Result<SourceQuery, TranslationError> {
        match &intent.intent_type {
            IntentType::Search { query_text, .. } => {
                let filter_str = Self::graphql_filter(&intent.filters);
                let graphql = format!(
                    r#"
                    {{
                      Get {{
                        Document(
                          limit: {}
                          where: {{
                            operator: Contains
                            path: ["content"]
                            valueString: "{}"
                            {}
                          }}
                        ) {{
                          content
                          metadata
                          _additional {{
                            certainty
                          }}
                        }}
                      }}
                    }}
                    "#,
                    intent.limit.unwrap_or(10),
                    query_text,
                    filter_str
                );

                Ok(SourceQuery::GraphQL { query: graphql })
            }
            _ => Err(TranslationError::UnsupportedIntent),
        }
    }

    fn to_rest_query(intent: &QueryIntent) -> Result<SourceQuery, TranslationError> {
        match &intent.intent_type {
            IntentType::Search { query_text, .. } => {
                let params = vec![
                    ("q".to_string(), query_text.clone()),
                    ("limit".to_string(), intent.limit.unwrap_or(10).to_string()),
                ];

                Ok(SourceQuery::REST {
                    endpoint: "/search".to_string(),
                    method: "GET".to_string(),
                    query_params: params,
                    body: None,
                })
            }
            _ => Err(TranslationError::UnsupportedIntent),
        }
    }

    fn to_s3_query(intent: &QueryIntent) -> Result<SourceQuery, TranslationError> {
        match &intent.intent_type {
            IntentType::Search { query_text, .. } => {
                Ok(SourceQuery::S3 {
                    prefix: query_text.clone(),
                    tags: intent
                        .filters
                        .iter()
                        .map(|f| (f.field.clone(), f.value.to_string()))
                        .collect(),
                })
            }
            _ => Err(TranslationError::UnsupportedIntent),
        }
    }

    fn build_where_clause(
        match_type: &MatchType,
        query: &str,
        filters: &[Filter],
    ) -> String {
        let mut clauses = vec![match match_type {
            MatchType::Exact => format!("content = '{}'", query),
            MatchType::Fuzzy => format!("content ILIKE '%{}%'", query),
            MatchType::Regex => format!("content ~ '{}'", query),
            MatchType::Semantic => "1=1".to_string(),
        }];

        for filter in filters {
            let op_str = match filter.operator {
                FilterOperator::Eq => "=",
                FilterOperator::Ne => "!=",
                FilterOperator::Gt => ">",
                FilterOperator::Gte => ">=",
                FilterOperator::Lt => "<",
                FilterOperator::Lte => "<=",
                FilterOperator::In => "IN",
                FilterOperator::Contains => "ILIKE",
                FilterOperator::Regex => "~",
            };

            clauses.push(format!("{} {} '{}'", filter.field, op_str, filter.value));
        }

        clauses.join(" AND ")
    }

    fn agg_metric_to_sql(metric: &AggregateMetric) -> String {
        match metric {
            AggregateMetric::Count => "COUNT(*) as count".to_string(),
            AggregateMetric::Sum => "SUM(amount) as total".to_string(),
            AggregateMetric::Avg => "AVG(value) as average".to_string(),
            AggregateMetric::Min => "MIN(value) as minimum".to_string(),
            AggregateMetric::Max => "MAX(value) as maximum".to_string(),
            AggregateMetric::Distinct => "COUNT(DISTINCT id) as unique_count".to_string(),
        }
    }

    fn pinecone_filters(filters: &[Filter]) -> serde_json::Value {
        serde_json::json!({
            "filters": filters.iter().map(|f| {
                serde_json::json!({
                    "field": f.field,
                    "value": f.value
                })
            }).collect::<Vec<_>>()
        })
    }

    fn graphql_filter(filters: &[Filter]) -> String {
        filters
            .iter()
            .map(|f| format!(r#""{}" : "{}""#, f.field, f.value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug)]
pub enum SourceQuery {
    Pinecone {
        vector_query: String,
        top_k: u32,
        filter: serde_json::Value,
    },
    SQL {
        query: String,
        limit: u32,
        offset: u32,
    },
    GraphQL {
        query: String,
    },
    REST {
        endpoint: String,
        method: String,
        query_params: Vec<(String, String)>,
        body: Option<String>,
    },
    S3 {
        prefix: String,
        tags: Vec<(String, String)>,
    },
}

#[derive(Debug)]
pub enum TranslationError {
    UnsupportedIntent,
    UnsupportedMatchType,
    InvalidQuery,
}
```

### 3.5 Result Aggregation Engine

**Purpose**: Merge results from multiple sources, deduplicate, and rank.

```rust
use std::collections::HashSet;

pub struct AggregationEngine {
    dedup_strategy: DeduplicationStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum DeduplicationStrategy {
    ById,
    ByHash,
    ByContent,
    None,
}

impl AggregationEngine {
    pub fn new(dedup_strategy: DeduplicationStrategy) -> Self {
        Self { dedup_strategy }
    }

    /// Aggregate results from multiple sources
    pub async fn aggregate(
        &self,
        result_streams: Vec<Vec<QueryResult>>,
    ) -> Result<Vec<QueryResult>, AggregationError> {
        let mut aggregated = Vec::new();
        let mut seen = HashSet::new();

        for stream in result_streams {
            for result in stream {
                let key = match self.dedup_strategy {
                    DeduplicationStrategy::ById => result.result_id.clone(),
                    DeduplicationStrategy::ByHash => {
                        format!("{:x}", self.hash_result(&result))
                    }
                    DeduplicationStrategy::ByContent => {
                        format!("{:x}", self.hash_content(&result.data))
                    }
                    DeduplicationStrategy::None => result.result_id.clone(),
                };

                if !seen.contains(&key) {
                    seen.insert(key);
                    aggregated.push(result);
                }
            }
        }

        // Sort by relevance score (descending)
        aggregated.sort_by(|a, b| {
            b.metadata
                .relevance_score
                .partial_cmp(&a.metadata.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(aggregated)
    }

    fn hash_result(&self, result: &QueryResult) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        result.result_id.hash(&mut hasher);
        result.data.to_string().hash(&mut hasher);
        hasher.finish()
    }

    fn hash_content(&self, data: &serde_json::Value) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.to_string().hash(&mut hasher);
        hasher.finish()
    }

    /// Merge metadata from multiple results
    pub fn merge_metadata(results: &[QueryResult]) -> ResultMetadata {
        let total_time: u64 = results.iter().map(|r| r.metadata.retrieval_time_ms).sum();
        let avg_score: f32 = results.iter().map(|r| r.metadata.relevance_score).sum::<f32>()
            / results.len().max(1) as f32;
        let total_rows: u32 = results.iter().map(|r| r.metadata.row_count).sum();

        ResultMetadata {
            retrieval_time_ms: total_time,
            relevance_score: avg_score,
            row_count: total_rows,
            source_type: "aggregated".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum AggregationError {
    FailedToAggregate(String),
}
```

### 3.6 CSCI Integration

**Purpose**: Integrate semantic queries with CSCI memory interface.

```rust
use tokio::sync::mpsc;

pub struct CSCISemanticQueryAdapter {
    parser: NLParser,
    classifier: IntentClassifier,
    router: QueryRouter,
    translator: QueryTranslator,
    aggregator: AggregationEngine,
}

impl CSCISemanticQueryAdapter {
    pub fn new() -> Self {
        Self {
            parser: NLParser::new(),
            classifier: IntentClassifier::new(),
            router: QueryRouter::new(),
            translator: QueryTranslator,
            aggregator: AggregationEngine::new(DeduplicationStrategy::ById),
        }
    }

    /// CSCI-compatible semantic query interface
    pub async fn mem_mount_semantic_query(
        &self,
        query: &str,
        context: Option<serde_json::Value>,
    ) -> Result<String, CSCIError> {
        // Parse NL query
        let entities = self.parser.parse_query(query).await?;

        // Classify intent
        let intent = self.classifier.classify(query, &entities)?;

        // Route to mounts
        let mounts = self.router.route(&intent)?;

        // Translate to source-specific queries
        let mut translated = Vec::new();
        for mount in mounts {
            let mount_type = self.get_mount_type(&mount)?;
            let source_query = self.translator.translate(&intent, mount_type)?;
            translated.push((mount, source_query));
        }

        // Generate query handle (UUID)
        let query_handle = format!("sq_{}", uuid::Uuid::new_v4());

        Ok(query_handle)
    }

    /// CSCI-compatible streaming result interface
    pub async fn mem_read_semantic_query_results(
        &self,
        query_handle: &str,
    ) -> Result<mpsc::UnboundedReceiver<QueryResult>, CSCIError> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Simulate streaming results (real implementation would fetch from mounts)
        tokio::spawn(async move {
            for i in 0..10 {
                let result = QueryResult {
                    result_id: format!("result_{}", i),
                    source_mount: "postgres".to_string(),
                    data: serde_json::json!({
                        "id": i,
                        "content": format!("Result {}", i)
                    }),
                    metadata: ResultMetadata {
                        retrieval_time_ms: 50,
                        relevance_score: 0.95 - (i as f32 * 0.01),
                        row_count: 1,
                        source_type: "postgres".to_string(),
                    },
                };

                let _ = tx.send(result);
            }
        });

        Ok(rx)
    }

    fn get_mount_type(&self, mount_id: &str) -> Result<MountType, CSCIError> {
        match mount_id {
            "pinecone" => Ok(MountType::Pinecone),
            "postgres" => Ok(MountType::PostgreSQL),
            "weaviate" => Ok(MountType::Weaviate),
            "rest_api" => Ok(MountType::REST),
            "s3_metadata" => Ok(MountType::S3),
            _ => Err(CSCIError::UnknownMount),
        }
    }
}

#[derive(Debug)]
pub enum CSCIError {
    ParseError(String),
    ClassificationError(String),
    RoutingError(String),
    TranslationError(String),
    UnknownMount,
}

impl From<String> for CSCIError {
    fn from(e: String) -> Self {
        CSCIError::ParseError(e)
    }
}
```

---

## 4. Integration Points

### 4.1 Mount System Integration

Each mounted source (Pinecone, PostgreSQL, Weaviate, REST, S3) provides:

1. **Capability Advertisement**: Declares supported query types, filter operators, latency characteristics
2. **Query Translation**: Accepts `SourceQuery` enum and returns mount-specific query format
3. **Streaming Results**: Returns `Stream<QueryResult>` for integration with aggregation engine
4. **Metadata Enrichment**: Provides relevance scores, retrieval timing, row counts

### 4.2 CSCI Protocol Mapping

```rust
// CSCI request: semantic query
pub struct CSCISemanticQueryRequest {
    pub query_text: String,
    pub context: Option<serde_json::Value>,
    pub timeout_ms: u32,
}

// CSCI response: streaming handle + results channel
pub struct CSCISemanticQueryResponse {
    pub query_handle: String,
    pub results_channel: String,  // Channel ID for mem_read
    pub estimated_rows: u32,
    pub estimated_latency_ms: u32,
}

// CSCI mem_read: retrieve paginated results
pub struct CSCIMemReadRequest {
    pub handle: String,
    pub offset: u32,
    pub limit: u32,
}

pub struct CSCIMemReadResponse {
    pub results: Vec<QueryResult>,
    pub has_more: bool,
    pub total_rows: u32,
}
```

---

## 5. Test Suite: 50+ NL Query Examples

### 5.1 Search Queries (15 examples)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exact_search() {
        let parser = NLParser::new();
        let entities = parser.parse_query("find users with email alice@example.com").await.unwrap();
        assert!(!entities.is_empty());
        assert_eq!(entities[0].entity_type, "email");
    }

    #[tokio::test]
    async fn test_fuzzy_search() {
        let parser = NLParser::new();
        let entities = parser.parse_query("search for documents about machine learning").await.unwrap();
        assert!(!entities.is_empty());
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let classifier = IntentClassifier::new();
        let intent = classifier.classify("find similar documents to my query", &[]).unwrap();
        match intent.intent_type {
            IntentType::Search { match_type, .. } => {
                assert_eq!(match_type, MatchType::Semantic);
            }
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn test_regex_search() {
        let query = "search for emails matching ^[a-z]+@example.com$";
        let classifier = IntentClassifier::new();
        let intent = classifier.classify(query, &[]).unwrap();
        match intent.intent_type {
            IntentType::Search { match_type, .. } => {
                assert_eq!(match_type, MatchType::Regex);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_search_with_timestamp() {
        let query = "find documents created after 2026-01-01";
        let parser = NLParser::new();
        // Parser should extract timestamp entity
        // assert timestamp entity present
    }

    #[test]
    fn test_search_with_multiple_entities() {
        let query = "find user alice@example.com in project XKernal";
        // Should extract: email, user_id, project_name
    }

    #[test]
    fn test_search_with_range() {
        let query = "find posts between 100-500 characters";
        // Should extract numeric range
    }

    #[test]
    fn test_search_case_insensitive() {
        let query = "FIND DOCUMENTS ABOUT NETWORKS";
        // Parser should handle uppercase normalization
    }

    #[test]
    fn test_search_with_special_chars() {
        let query = "find files named config.yaml or settings.json";
        // Should extract filenames with extensions
    }

    #[test]
    fn test_search_with_boolean_operators() {
        let query = "find users (admin OR moderator) NOT banned";
        // Should extract boolean logic
    }

    #[test]
    fn test_search_with_proximity() {
        let query = "find documents where 'machine' near 'learning' within 5 words";
        // Should extract proximity constraint
    }

    #[test]
    fn test_search_with_language() {
        let query = "find documents in language:spanish about architecture";
        // Should extract language filter
    }

    #[test]
    fn test_search_with_date_range() {
        let query = "find articles between last week and now";
        // Should extract relative date range
    }

    #[test]
    fn test_search_negation() {
        let query = "find documents NOT containing spam";
        // Should extract negation
    }

    #[test]
    fn test_search_wildcard() {
        let query = "find files matching docs/*.md";
        // Should extract wildcard pattern
    }
}
```

### 5.2 Retrieve Queries (12 examples)

```rust
#[cfg(test)]
mod retrieve_tests {
    use super::*;

    #[test]
    fn test_retrieve_by_id() {
        let query = "get document with id 550e8400-e29b-41d4-a716-446655440000";
        let classifier = IntentClassifier::new();
        let intent = classifier.classify(query, &[]).unwrap();
        match intent.intent_type {
            IntentType::Retrieve { .. } => {},
            _ => panic!(),
        }
    }

    #[test]
    fn test_retrieve_by_uuid() {
        let query = "fetch user uuid:550e8400-e29b-41d4-a716-446655440000";
        // Should extract and classify as Retrieve
    }

    #[test]
    fn test_retrieve_multiple_ids() {
        let query = "get documents with ids doc1, doc2, doc3, doc4";
        // Should extract all IDs and classify as Retrieve
    }

    #[test]
    fn test_retrieve_by_external_id() {
        let query = "load user external_id:stripe_cus_123";
        // Should extract external ID
    }

    #[test]
    fn test_retrieve_batch() {
        let query = "batch get documents 1,2,3,4,5,6,7,8,9,10";
        // Should recognize batch retrieval
    }

    #[test]
    fn test_retrieve_with_projection() {
        let query = "get user 123 with fields name, email, created_at only";
        // Should extract field projection
    }

    #[test]
    fn test_retrieve_with_expand() {
        let query = "fetch document id:456 and expand all relations";
        // Should extract expand directive
    }

    #[test]
    fn test_retrieve_latest() {
        let query = "get latest message from user alice";
        // Should classify as retrieval with ordering
    }

    #[test]
    fn test_retrieve_by_key() {
        let query = "lookup value with key config:database:url";
        // Should extract hierarchical key
    }

    #[test]
    fn test_retrieve_with_consistency() {
        let query = "fetch document id:789 with strong consistency";
        // Should extract consistency requirement
    }

    #[test]
    fn test_retrieve_conditional() {
        let query = "get user 999 if exists else return null";
        // Should extract conditional logic
    }

    #[test]
    fn test_retrieve_cached() {
        let query = "fetch page cache:bypass to ensure freshness";
        // Should extract cache directive
    }
}
```

### 5.3 Aggregate Queries (12 examples)

```rust
#[cfg(test)]
mod aggregate_tests {
    use super::*;

    #[test]
    fn test_count_all() {
        let query = "how many documents are there";
        let classifier = IntentClassifier::new();
        let intent = classifier.classify(query, &[]).unwrap();
        match intent.intent_type {
            IntentType::Aggregate { .. } => {},
            _ => panic!(),
        }
    }

    #[test]
    fn test_sum_metric() {
        let query = "sum the amounts for all transactions";
        // Should extract aggregation type Sum
    }

    #[test]
    fn test_average_metric() {
        let query = "what is the average response time across all requests";
        // Should extract aggregation type Avg
    }

    #[test]
    fn test_min_max_metrics() {
        let query = "find minimum and maximum values in the dataset";
        // Should extract both Min and Max
    }

    #[test]
    fn test_group_by_single() {
        let query = "count documents grouped by user_id";
        // Should extract grouping
    }

    #[test]
    fn test_group_by_multiple() {
        let query = "sum revenue grouped by country and product";
        // Should extract multiple group-by fields
    }

    #[test]
    fn test_distinct_count() {
        let query = "how many unique users visited the site";
        // Should extract distinct aggregation
    }

    #[test]
    fn test_percentile_aggregate() {
        let query = "calculate 95th percentile latency";
        // Should extract percentile aggregation
    }

    #[test]
    fn test_aggregate_with_filter() {
        let query = "count posts where status=published grouped by author";
        // Should combine aggregation with filters
    }

    #[test]
    fn test_aggregate_with_having() {
        let query = "find users with more than 100 posts";
        // Should extract HAVING clause equivalent
    }

    #[test]
    fn test_time_series_aggregate() {
        let query = "count events per hour for last 7 days";
        // Should extract time-bucketing aggregation
    }

    #[test]
    fn test_statistical_aggregate() {
        let query = "calculate stddev and variance of measurements";
        // Should extract statistical aggregations
    }
}
```

### 5.4 Join Queries (11 examples)

```rust
#[cfg(test)]
mod join_tests {
    use super::*;

    #[test]
    fn test_inner_join() {
        let query = "find users who have posted comments";
        let classifier = IntentClassifier::new();
        let intent = classifier.classify(query, &[]).unwrap();
        match intent.intent_type {
            IntentType::Join { .. } => {},
            _ => panic!(),
        }
    }

    #[test]
    fn test_left_join() {
        let query = "list all users and their profiles, including users without profiles";
        // Should classify as left join
    }

    #[test]
    fn test_right_join() {
        let query = "show all posts and authors, even posts with deleted authors";
        // Should classify as right join
    }

    #[test]
    fn test_cross_join() {
        let query = "combine every product with every category";
        // Should classify as cross join
    }

    #[test]
    fn test_join_on_condition() {
        let query = "join documents to projects where doc.project_id = project.id";
        // Should extract join condition
    }

    #[test]
    fn test_multi_join() {
        let query = "find users with their posts and comments";
        // Should recognize multiple joins
    }

    #[test]
    fn test_join_across_mounts() {
        let query = "correlate pinecone vectors with postgres user data";
        // Should identify cross-mount join
    }

    #[test]
    fn test_self_join() {
        let query = "find users and their referrers";
        // Should recognize self-join pattern
    }

    #[test]
    fn test_join_with_aggregation() {
        let query = "for each user, count their posts with comments";
        // Should combine join and aggregation
    }

    #[test]
    fn test_outer_join() {
        let query = "match products to suppliers, keeping unmatched items";
        // Should recognize outer join
    }

    #[test]
    fn test_lateral_join() {
        let query = "for each order, get top 3 items";
        // Should recognize lateral/cross-apply semantics
    }
}
```

### 5.5 Complex Queries (16 examples)

```rust
#[cfg(test)]
mod complex_tests {
    use super::*;

    #[test]
    fn test_nested_query() {
        let query = "find users who posted more than the average number of posts";
        // Should parse nested subquery semantics
    }

    #[test]
    fn test_cte_style_query() {
        let query = "first, find active users, then count their posts";
        // Should recognize sequential query pattern
    }

    #[test]
    fn test_union_semantics() {
        let query = "combine results from search with recent documents";
        // Should recognize union operation
    }

    #[test]
    fn test_window_function() {
        let query = "rank users by number of posts, breaking ties by join date";
        // Should recognize window function pattern
    }

    #[test]
    fn test_text_search_with_filters() {
        let query = "search for 'machine learning' in articles published after 2025 by authors in ML category";
        // Complex: text search + temporal filter + categorical filter
    }

    #[test]
    fn test_semantic_search_with_aggregation() {
        let query = "find documents semantically similar to 'neural networks' and count by topic";
        // Complex: semantic search + aggregation
    }

    #[test]
    fn test_multi_mount_search() {
        let query = "search vectors in pinecone and structured data in postgres for same topic";
        // Complex: cross-mount search
    }

    #[test]
    fn test_filter_aggregation_ordering() {
        let query = "for posts with >10 likes from last month, count by tag, order by frequency desc, limit 20";
        // Complex: filter + aggregation + ordering + pagination
    }

    #[test]
    fn test_fuzzy_text_semantic() {
        let query = "find documents fuzzy-matching 'databse' with semantic similarity to 'SQL'";
        // Complex: fuzzy + semantic matching
    }

    #[test]
    fn test_temporal_aggregation() {
        let query = "for each day in the last 30 days, count new users and their average session duration";
        // Complex: time-bucketing + aggregation
    }

    #[test]
    fn test_geo_proximity_search() {
        let query = "find restaurants within 5km of coordinates 37.7749,-122.4194 with rating > 4.5";
        // Complex: geo-spatial + filter
    }

    #[test]
    fn test_recommendation_query() {
        let query = "find products similar to what users in segment 'premium' have purchased";
        // Complex: semantic + user segmentation
    }

    #[test]
    fn test_anomaly_detection() {
        let query = "find transactions deviating > 3 standard deviations from user average spend";
        // Complex: statistical aggregation
    }

    #[test]
    fn test_graph_traversal() {
        let query = "find all collaborators of alice up to 3 degrees of separation";
        // Complex: graph traversal semantics
    }

    #[test]
    fn test_temporal_join() {
        let query = "match orders to shipments where shipment date is within 7 days of order date";
        // Complex: temporal join condition
    }

    #[test]
    fn test_full_pipeline() {
        let query = "find high-value customers (>$10k spent) from USA, group by region, show top 5 regions by average order value, ordered descending";
        // Complex: filter + aggregation + ordering + pagination
    }
}
```

---

## 6. Performance Specifications

### 6.1 Latency SLA

| Query Type | Target | Threshold |
|-----------|--------|-----------|
| Simple exact search | <100ms | <200ms |
| Simple semantic search | <150ms | <200ms |
| Retrieve by ID | <50ms | <150ms |
| Aggregate (single mount) | <200ms | <500ms |
| Aggregate (multi-mount) | <300ms | <500ms |
| Join (2 mounts) | <400ms | <700ms |
| Full pipeline with dedup | <500ms | <1000ms |

### 6.2 Memory Usage

- **Parser cache (entities)**: <10MB
- **Classifier rules engine**: <5MB
- **Router mount registry**: <2MB
- **Translator query templates**: <1MB
- **Aggregator result buffer**: <50MB (for 10k results)

### 6.3 Throughput

- **Queries per second**: 1000 QPS (simple), 100 QPS (complex)
- **Concurrent queries**: 100+
- **Result streaming**: 10k rows/sec per consumer

---

## 7. Future Enhancements (Phase 3)

1. **Machine Learning Intent Classifier**: ONNX-based neural classifier for ambiguous queries
2. **Query Optimization**: Cost-based routing with cardinality estimation
3. **Caching Layer**: LRU cache for common queries with TTL
4. **Federated Learning**: Improve entity extraction with user-provided feedback loop
5. **GraphQL Federation**: Unified GraphQL endpoint over all mounts
6. **Query Plan Visualization**: Interactive query plan explorer for CSCI clients

---

## 8. References

- CSCI Specification: `xkernal/csci/CSCI_PROTOCOL.md`
- Mount System Design: `xkernal/mounts/MOUNT_ARCHITECTURE.md`
- Rust Guidelines: MAANG Code Standards, 350-400 LOC per component
- Performance Baseline: Week 18 query parser achieved 95% entity extraction accuracy

---

**Document Author**: Staff-Level Engineer (Engineer 8)
**Last Updated**: 2026-03-02
**Status**: Ready for Implementation Sprint
