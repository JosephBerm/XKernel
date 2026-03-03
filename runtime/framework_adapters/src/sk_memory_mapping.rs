// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Semantic Kernel Memory to L2/L3 Tier Mapping
//!
//! Maps Semantic Kernel memory buffers and access patterns to Cognitive Substrate
//! memory tier architecture (L2 episodic, L3 semantic). Handles volatile SK buffers
//! to L2 episodic snapshots and persistent stores to L3 semantic storage.
//!
//! SK memory model:
//! - Volatile buffers (conversation, working memory) → L2 episodic snapshots
//! - Persistent KernelMemory (knowledge base) → L3 semantic indexed storage
//! - Vector stores → L3 semantic with vector indexing
//!
//! Sec 4.3: SK Memory → L2/L3 Mapping
//! Sec 3.3: Memory Tier Architecture
//! Sec 4.3: Episodic vs Semantic Storage

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::AdapterResult;
use crate::error::AdapterError;

/// Semantic Kernel memory buffer classification.
/// Sec 4.3: SK Buffer Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkBufferType {
    /// Short-term conversation history (volatile)
    ConversationHistory,
    /// Working memory for current task
    WorkingMemory,
    /// Vector embeddings store (persistent)
    VectorStore,
    /// Knowledge base entries (persistent)
    KnowledgeBase,
    /// Semantic memory cache (mixed persistence)
    SemanticCache,
}

impl SkBufferType {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SkBufferType::ConversationHistory => "conversation_history",
            SkBufferType::WorkingMemory => "working_memory",
            SkBufferType::VectorStore => "vector_store",
            SkBufferType::KnowledgeBase => "knowledge_base",
            SkBufferType::SemanticCache => "semantic_cache",
        }
    }
}

/// Cognitive Substrate memory tier.
/// Sec 3.3: Memory Tier Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CtMemoryTier {
    /// L2: Episodic memory layer (fast, session-lived)
    L2Episodic,
    /// L3: Semantic memory layer (persistent, indexed)
    L3Semantic,
    /// L3 with vector indexing
    L3SemanticIndexed,
}

impl CtMemoryTier {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            CtMemoryTier::L2Episodic => "L2_episodic",
            CtMemoryTier::L3Semantic => "L3_semantic",
            CtMemoryTier::L3SemanticIndexed => "L3_semantic_indexed",
        }
    }

    /// Returns typical capacity in tokens for this tier.
    pub fn typical_capacity_tokens(&self) -> u64 {
        match self {
            CtMemoryTier::L2Episodic => 500_000,       // ~2MB
            CtMemoryTier::L3Semantic => 100_000_000,   // ~400MB
            CtMemoryTier::L3SemanticIndexed => 100_000_000,
        }
    }
}

/// Persistence characteristic of SK buffer.
/// Sec 4.3: Persistence Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkPersistence {
    /// Volatile, cleared at end of session
    Volatile,
    /// Persistent across sessions
    Persistent,
    /// Cached with TTL-based expiration
    CachedWithTtl,
}

impl SkPersistence {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SkPersistence::Volatile => "volatile",
            SkPersistence::Persistent => "persistent",
            SkPersistence::CachedWithTtl => "cached_with_ttl",
        }
    }
}

/// Access pattern hints for memory migration.
/// Sec 4.3: Access Pattern Analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessPattern {
    /// Sequential read/write (typical for conversation)
    Sequential,
    /// Random access (typical for vector similarity search)
    Random,
    /// Hot/cold phases (active then dormant)
    HotCold,
    /// Archive-like (rarely accessed)
    Archive,
}

impl AccessPattern {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessPattern::Sequential => "sequential",
            AccessPattern::Random => "random",
            AccessPattern::HotCold => "hot_cold",
            AccessPattern::Archive => "archive",
        }
    }
}

/// SK memory buffer definition.
/// Sec 4.3: SK Memory Buffer Specification
#[derive(Debug, Clone)]
pub struct SkMemoryBuffer {
    /// Buffer identifier
    pub buffer_id: String,
    /// Buffer type
    pub buffer_type: SkBufferType,
    /// Persistence mode
    pub persistence: SkPersistence,
    /// Capacity in tokens
    pub capacity_tokens: u64,
    /// Access pattern hint
    pub access_pattern: AccessPattern,
    /// Optional serialization format
    pub serialization_format: Option<String>,
}

impl SkMemoryBuffer {
    /// Creates a new SK memory buffer.
    pub fn new(buffer_id: String, buffer_type: SkBufferType) -> Self {
        SkMemoryBuffer {
            buffer_id,
            buffer_type,
            persistence: SkPersistence::Volatile,
            capacity_tokens: 10000,
            access_pattern: AccessPattern::Sequential,
            serialization_format: Some("json".to_string()),
        }
    }
}

/// CT memory tier mapping result.
/// Sec 4.3: Memory Tier Mapping
#[derive(Debug, Clone)]
pub struct MemoryTierMap {
    /// Source SK buffer ID
    pub source_buffer_id: String,
    /// Source SK buffer type
    pub source_buffer_type: String,
    /// Target CT memory tier
    pub target_tier: CtMemoryTier,
    /// Capacity mapping (may differ from source)
    pub mapped_capacity_tokens: u64,
    /// Persistence policy in CT model
    pub ct_persistence: String,
    /// Access pattern in CT model
    pub ct_access_pattern: String,
    /// Migration strategy hint
    pub migration_strategy: String,
}

impl MemoryTierMap {
    /// Creates a new memory tier map.
    pub fn new(
        source_buffer_id: String,
        source_buffer_type: String,
        target_tier: CtMemoryTier,
    ) -> Self {
        MemoryTierMap {
            source_buffer_id,
            source_buffer_type,
            target_tier,
            mapped_capacity_tokens: target_tier.typical_capacity_tokens(),
            ct_persistence: "session_lived".to_string(),
            ct_access_pattern: "sequential".to_string(),
            migration_strategy: "stay".to_string(),
        }
    }
}

/// Snapshot of episodic memory in L2.
/// Sec 4.3: L2 Episodic Snapshot
#[derive(Debug, Clone)]
pub struct L2EpisodicSnapshot {
    /// Snapshot identifier
    pub snapshot_id: String,
    /// Source SK buffer ID
    pub source_buffer_id: String,
    /// Snapshot timestamp (nanoseconds since epoch)
    pub timestamp_ns: u64,
    /// Serialized memory contents
    pub contents: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// TTL in milliseconds (None = session-lived)
    pub ttl_ms: Option<u64>,
}

impl L2EpisodicSnapshot {
    /// Creates a new episodic snapshot.
    pub fn new(snapshot_id: String, source_buffer_id: String, timestamp_ns: u64) -> Self {
        L2EpisodicSnapshot {
            snapshot_id,
            source_buffer_id,
            timestamp_ns,
            contents: String::new(),
            size_bytes: 0,
            ttl_ms: None,
        }
    }
}

/// Semantic memory record in L3.
/// Sec 4.3: L3 Semantic Record
#[derive(Debug, Clone)]
pub struct L3SemanticRecord {
    /// Record identifier
    pub record_id: String,
    /// Source SK buffer ID
    pub source_buffer_id: String,
    /// Semantic content
    pub content: String,
    /// Vector embedding (if applicable)
    pub embedding: Option<Vec<f32>>,
    /// Metadata tags for indexing
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_ts_ns: u64,
    /// Last modified timestamp
    pub modified_ts_ns: u64,
}

impl L3SemanticRecord {
    /// Creates a new semantic record.
    pub fn new(record_id: String, source_buffer_id: String, content: String) -> Self {
        let now = 0u64; // In practice, use actual timestamp
        L3SemanticRecord {
            record_id,
            source_buffer_id,
            content,
            embedding: None,
            tags: Vec::new(),
            created_ts_ns: now,
            modified_ts_ns: now,
        }
    }

    /// Adds a tag for semantic indexing.
    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }
}

/// Mapper from SK memory to CT tiers.
/// Sec 4.3: SK-to-CT Memory Mapper
pub struct SkMemoryMapper;

impl SkMemoryMapper {
    /// Maps SK memory buffer to appropriate CT tier.
    /// Sec 4.3: Buffer-to-Tier Mapping
    pub fn map_buffer_to_tier(buffer: &SkMemoryBuffer) -> AdapterResult<MemoryTierMap> {
        let target_tier = match buffer.buffer_type {
            // Volatile buffers → L2 episodic
            SkBufferType::ConversationHistory | SkBufferType::WorkingMemory => {
                CtMemoryTier::L2Episodic
            }
            // Persistent knowledge → L3 semantic
            SkBufferType::KnowledgeBase => CtMemoryTier::L3Semantic,
            // Vector stores → L3 with indexing
            SkBufferType::VectorStore => CtMemoryTier::L3SemanticIndexed,
            // Cache → depends on persistence
            SkBufferType::SemanticCache => {
                match buffer.persistence {
                    SkPersistence::Volatile => CtMemoryTier::L2Episodic,
                    _ => CtMemoryTier::L3Semantic,
                }
            }
        };

        let ct_persistence = match buffer.persistence {
            SkPersistence::Volatile => "session_lived".to_string(),
            SkPersistence::Persistent => "permanent".to_string(),
            SkPersistence::CachedWithTtl => "cached".to_string(),
        };

        let ct_access_pattern = match buffer.access_pattern {
            AccessPattern::Sequential => "sequential".to_string(),
            AccessPattern::Random => "random".to_string(),
            AccessPattern::HotCold => "hot_cold".to_string(),
            AccessPattern::Archive => "archive".to_string(),
        };

        let migration_strategy = match buffer.persistence {
            SkPersistence::Volatile => "stay".to_string(),
            SkPersistence::Persistent => "stay".to_string(),
            SkPersistence::CachedWithTtl => "demote_if_inactive".to_string(),
        };

        let mut map = MemoryTierMap::new(
            buffer.buffer_id.clone(),
            buffer.buffer_type.as_str().to_string(),
            target_tier,
        );

        map.ct_persistence = ct_persistence;
        map.ct_access_pattern = ct_access_pattern;
        map.migration_strategy = migration_strategy;

        // Adjust capacity if mapping to smaller tier
        if target_tier == CtMemoryTier::L2Episodic && buffer.capacity_tokens > CtMemoryTier::L2Episodic.typical_capacity_tokens() {
            map.mapped_capacity_tokens = CtMemoryTier::L2Episodic.typical_capacity_tokens();
        }

        Ok(map)
    }

    /// Converts SK buffer to L2 episodic snapshot.
    /// Sec 4.3: Buffer-to-Snapshot Conversion
    pub fn buffer_to_l2_snapshot(
        buffer: &SkMemoryBuffer,
        contents: String,
    ) -> AdapterResult<L2EpisodicSnapshot> {
        let snapshot_id = alloc::format!("l2-snap-{}", buffer.buffer_id);
        let size_bytes = contents.len() as u64;
        
        let mut snapshot = L2EpisodicSnapshot::new(
            snapshot_id,
            buffer.buffer_id.clone(),
            0, // Use actual timestamp in practice
        );
        snapshot.contents = contents;
        snapshot.size_bytes = size_bytes;

        // Set TTL based on persistence
        if buffer.persistence == SkPersistence::Volatile {
            snapshot.ttl_ms = Some(3600000); // 1 hour default
        }

        Ok(snapshot)
    }

    /// Converts SK buffer to L3 semantic record.
    /// Sec 4.3: Buffer-to-SemanticRecord Conversion
    pub fn buffer_to_l3_record(
        buffer: &SkMemoryBuffer,
        content: String,
    ) -> AdapterResult<L3SemanticRecord> {
        let record_id = alloc::format!("l3-rec-{}", buffer.buffer_id);
        
        let mut record = L3SemanticRecord::new(
            record_id,
            buffer.buffer_id.clone(),
            content,
        );

        // Add semantic tags based on buffer type
        match buffer.buffer_type {
            SkBufferType::KnowledgeBase => {
                record.add_tag("knowledge_base".to_string());
                record.add_tag("indexed".to_string());
            }
            SkBufferType::VectorStore => {
                record.add_tag("vector_searchable".to_string());
                record.add_tag("semantic".to_string());
            }
            SkBufferType::ConversationHistory => {
                record.add_tag("conversation".to_string());
            }
            _ => {}
        }

        Ok(record)
    }

    /// Produces migration recommendations for memory.
    /// Sec 4.3: Migration Recommendations
    pub fn recommend_migration(map: &MemoryTierMap) -> String {
        match map.migration_strategy.as_str() {
            "stay" => "Keep in current tier".to_string(),
            "demote_if_inactive" => {
                alloc::format!(
                    "Consider demoting to {} if access becomes infrequent",
                    map.target_tier.as_str()
                )
            }
            "promote_if_active" => {
                "Promote to faster tier if currently accessed frequently".to_string()
            }
            _ => "No specific migration recommendation".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_sk_buffer_type_as_str() {
        assert_eq!(SkBufferType::ConversationHistory.as_str(), "conversation_history");
        assert_eq!(SkBufferType::VectorStore.as_str(), "vector_store");
    }

    #[test]
    fn test_ct_memory_tier_as_str() {
        assert_eq!(CtMemoryTier::L2Episodic.as_str(), "L2_episodic");
        assert_eq!(CtMemoryTier::L3Semantic.as_str(), "L3_semantic");
    }

    #[test]
    fn test_ct_memory_tier_capacity() {
        assert_eq!(CtMemoryTier::L2Episodic.typical_capacity_tokens(), 500_000);
        assert_eq!(CtMemoryTier::L3Semantic.typical_capacity_tokens(), 100_000_000);
    }

    #[test]
    fn test_sk_memory_buffer_creation() {
        let buffer = SkMemoryBuffer::new("buf-1".into(), SkBufferType::ConversationHistory);
        assert_eq!(buffer.buffer_id, "buf-1");
        assert_eq!(buffer.buffer_type, SkBufferType::ConversationHistory);
        assert_eq!(buffer.persistence, SkPersistence::Volatile);
        assert_eq!(buffer.access_pattern, AccessPattern::Sequential);
    }

    #[test]
    fn test_memory_tier_map_creation() {
        let map = MemoryTierMap::new(
            "buf-1".into(),
            "conversation_history".into(),
            CtMemoryTier::L2Episodic,
        );
        assert_eq!(map.source_buffer_id, "buf-1");
        assert_eq!(map.target_tier, CtMemoryTier::L2Episodic);
    }

    #[test]
    fn test_l2_episodic_snapshot_creation() {
        let snapshot = L2EpisodicSnapshot::new("snap-1".into(), "buf-1".into(), 12345);
        assert_eq!(snapshot.snapshot_id, "snap-1");
        assert_eq!(snapshot.source_buffer_id, "buf-1");
        assert_eq!(snapshot.timestamp_ns, 12345);
    }

    #[test]
    fn test_l3_semantic_record_creation() {
        let record = L3SemanticRecord::new(
            "rec-1".into(),
            "buf-1".into(),
            "content".into(),
        );
        assert_eq!(record.record_id, "rec-1");
        assert_eq!(record.content, "content");
        assert!(record.tags.is_empty());
    }

    #[test]
    fn test_l3_semantic_record_add_tag() {
        let mut record = L3SemanticRecord::new(
            "rec-1".into(),
            "buf-1".into(),
            "content".into(),
        );
        record.add_tag("knowledge".to_string());
        assert_eq!(record.tags.len(), 1);
    }

    #[test]
    fn test_mapper_conversation_to_l2() {
        let buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::ConversationHistory,
        );
        
        let result = SkMemoryMapper::map_buffer_to_tier(&buffer);
        assert!(result.is_ok());
        
        let map = result.unwrap();
        assert_eq!(map.target_tier, CtMemoryTier::L2Episodic);
        assert_eq!(map.ct_persistence, "session_lived");
    }

    #[test]
    fn test_mapper_knowledge_base_to_l3() {
        let buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::KnowledgeBase,
        );
        
        let result = SkMemoryMapper::map_buffer_to_tier(&buffer);
        assert!(result.is_ok());
        
        let map = result.unwrap();
        assert_eq!(map.target_tier, CtMemoryTier::L3Semantic);
        assert_eq!(map.ct_persistence, "permanent");
    }

    #[test]
    fn test_mapper_vector_store_to_l3_indexed() {
        let buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::VectorStore,
        );
        
        let result = SkMemoryMapper::map_buffer_to_tier(&buffer);
        assert!(result.is_ok());
        
        let map = result.unwrap();
        assert_eq!(map.target_tier, CtMemoryTier::L3SemanticIndexed);
    }

    #[test]
    fn test_mapper_buffer_to_l2_snapshot() {
        let buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::ConversationHistory,
        );
        
        let result = SkMemoryMapper::buffer_to_l2_snapshot(&buffer, "test content".into());
        assert!(result.is_ok());
        
        let snapshot = result.unwrap();
        assert_eq!(snapshot.contents, "test content");
        assert!(snapshot.ttl_ms.is_some());
    }

    #[test]
    fn test_mapper_buffer_to_l3_record() {
        let buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::KnowledgeBase,
        );
        
        let result = SkMemoryMapper::buffer_to_l3_record(&buffer, "knowledge".into());
        assert!(result.is_ok());
        
        let record = result.unwrap();
        assert_eq!(record.content, "knowledge");
        assert!(!record.tags.is_empty());
    }

    #[test]
    fn test_migration_recommendations() {
        let map = MemoryTierMap::new(
            "buf-1".into(),
            "conversation_history".into(),
            CtMemoryTier::L2Episodic,
        );
        
        let recommendation = SkMemoryMapper::recommend_migration(&map);
        assert!(!recommendation.is_empty());
    }

    #[test]
    fn test_persistence_volatile_mapping() {
        let mut buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::ConversationHistory,
        );
        buffer.persistence = SkPersistence::Volatile;
        
        let map = SkMemoryMapper::map_buffer_to_tier(&buffer).unwrap();
        assert_eq!(map.ct_persistence, "session_lived");
    }

    #[test]
    fn test_persistence_persistent_mapping() {
        let mut buffer = SkMemoryBuffer::new(
            "buf-1".into(),
            SkBufferType::KnowledgeBase,
        );
        buffer.persistence = SkPersistence::Persistent;
        
        let map = SkMemoryMapper::map_buffer_to_tier(&buffer).unwrap();
        assert_eq!(map.ct_persistence, "permanent");
    }
}
