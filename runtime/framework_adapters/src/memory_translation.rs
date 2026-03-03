// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Memory → L2/L3 Translation
//!
//! Translates framework-specific memory models into CSCI memory tier allocations.
//! Maps framework memory concepts (conversation history, knowledge bases, working context) to
//! appropriate kernel memory tiers (L1 working, L2 episodic, L3 long-term).
//!
//! Provides memory syscall generation for kernel memory subsystem invocation.
//!
//! Sec 4.2: Memory Translation Interface
//! Sec 4.2: Memory Tier Assignment

use alloc::{string::String, vec::Vec};
use crate::{AdapterError, framework_type::FrameworkType};

/// Framework memory types requiring translation.
/// Sec 4.2: Framework Memory Type Taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTypeClass {
    /// Conversation history and multi-turn dialogue state
    ConversationBuffer,
    /// Structured entity information and relationships
    EntityMemory,
    /// Vector embeddings for semantic search
    VectorStore,
    /// Semantic knowledge graph or ontology
    KnowledgeBase,
    /// Summary of previous interactions
    Summary,
    /// Persistent user or session configuration
    Config,
    /// Custom framework-specific memory
    Custom,
}

impl MemoryTypeClass {
    /// Returns string representation of the memory type.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryTypeClass::ConversationBuffer => "conversation_buffer",
            MemoryTypeClass::EntityMemory => "entity_memory",
            MemoryTypeClass::VectorStore => "vector_store",
            MemoryTypeClass::KnowledgeBase => "knowledge_base",
            MemoryTypeClass::Summary => "summary",
            MemoryTypeClass::Config => "config",
            MemoryTypeClass::Custom => "custom",
        }
    }
}

/// Framework-specific memory definition requiring translation.
/// Sec 4.2: Framework Memory Definition
#[derive(Debug, Clone)]
pub struct FrameworkMemory {
    /// Memory identifier within the framework
    pub memory_id: String,
    /// Memory type classification
    pub memory_type: MemoryTypeClass,
    /// Memory data (typically JSON or serialized)
    pub data: String,
    /// Optional metadata
    pub metadata: Option<String>,
    /// Estimated capacity in tokens
    pub capacity_tokens: u64,
}

impl FrameworkMemory {
    /// Creates a new framework memory definition.
    pub fn new(
        memory_id: String,
        memory_type: MemoryTypeClass,
        data: String,
        capacity_tokens: u64,
    ) -> Self {
        FrameworkMemory {
            memory_id,
            memory_type,
            data,
            metadata: None,
            capacity_tokens,
        }
    }

    /// Sets optional metadata.
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// CSCI memory tier classification.
/// Sec 4.2: Memory Tier Assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// L1: Working/execution memory (transient, high speed)
    L1Working,
    /// L2: Episodic memory (session/conversation scope)
    L2Episodic,
    /// L3: Long-term memory (persistent knowledge)
    L3LongTerm,
}

impl MemoryTier {
    /// Returns string representation of the memory tier.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryTier::L1Working => "L1_working",
            MemoryTier::L2Episodic => "L2_episodic",
            MemoryTier::L3LongTerm => "L3_longterm",
        }
    }

    /// Returns the approximate retention duration in milliseconds.
    /// Sec 4.2: Tier Retention Characteristics
    pub fn retention_duration_ms(&self) -> u64 {
        match self {
            MemoryTier::L1Working => 300_000,       // 5 minutes
            MemoryTier::L2Episodic => 86_400_000,   // 24 hours
            MemoryTier::L3LongTerm => 7_776_000_000, // 90 days
        }
    }
}

/// Memory syscall types for kernel invocation.
/// Sec 4.2: Memory Syscall Types
#[derive(Debug, Clone)]
pub enum MemorySyscall {
    /// Allocate memory region
    MemAlloc {
        /// Target memory tier
        tier: MemoryTier,
        /// Requested size in bytes
        size: u64,
    },
    /// Write data to memory region
    MemWrite {
        /// Memory region identifier
        region: String,
        /// Data to write
        data: String,
    },
    /// Mount external memory source
    MemMount {
        /// Source identifier
        source: String,
        /// Mount point
        point: String,
    },
    /// Clear memory region
    MemClear {
        /// Region identifier
        region: String,
    },
}

impl MemorySyscall {
    /// Returns the syscall name for kernel invocation.
    pub fn syscall_name(&self) -> &'static str {
        match self {
            MemorySyscall::MemAlloc { .. } => "mem_alloc",
            MemorySyscall::MemWrite { .. } => "mem_write",
            MemorySyscall::MemMount { .. } => "mem_mount",
            MemorySyscall::MemClear { .. } => "mem_clear",
        }
    }
}

/// Indexing configuration for memory tier.
/// Sec 4.2: Memory Indexing Strategy
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Whether to create full-text index
    pub full_text_index: bool,
    /// Whether to create semantic index
    pub semantic_index: bool,
    /// Optional custom index specification
    pub custom_index: Option<String>,
}

impl IndexingConfig {
    /// Creates a default indexing configuration.
    pub fn default() -> Self {
        IndexingConfig {
            full_text_index: true,
            semantic_index: false,
            custom_index: None,
        }
    }

    /// Creates a semantic-focused indexing configuration.
    pub fn semantic() -> Self {
        IndexingConfig {
            full_text_index: false,
            semantic_index: true,
            custom_index: None,
        }
    }

    /// Creates a comprehensive indexing configuration.
    pub fn comprehensive() -> Self {
        IndexingConfig {
            full_text_index: true,
            semantic_index: true,
            custom_index: None,
        }
    }
}

/// Memory mapping with syscall sequence for tier assignment.
/// Sec 4.2: Memory Mapping Structure
#[derive(Debug, Clone)]
pub struct MemoryMapping {
    /// Original framework memory identifier
    pub source_memory_id: String,
    /// Target memory tier
    pub target_tier: MemoryTier,
    /// Syscalls to execute for memory setup
    pub syscall_sequence: Vec<MemorySyscall>,
    /// Indexing configuration for the tier
    pub indexing_config: IndexingConfig,
}

impl MemoryMapping {
    /// Creates a new memory mapping.
    pub fn new(
        source_memory_id: String,
        target_tier: MemoryTier,
        indexing_config: IndexingConfig,
    ) -> Self {
        MemoryMapping {
            source_memory_id,
            target_tier,
            syscall_sequence: Vec::new(),
            indexing_config,
        }
    }

    /// Adds a syscall to the sequence.
    pub fn add_syscall(&mut self, syscall: MemorySyscall) {
        self.syscall_sequence.push(syscall);
    }

    /// Generates a complete syscall sequence for memory provisioning.
    pub fn generate_provisioning_sequence(&mut self, size_bytes: u64) {
        self.syscall_sequence.clear();
        self.syscall_sequence.push(MemorySyscall::MemAlloc {
            tier: self.target_tier,
            size: size_bytes,
        });
    }
}

/// Translates framework memory to CSCI memory tier with syscall sequence.
/// Sec 4.2: Memory Translation Method
///
/// # Tier Assignment Rules
/// - ConversationBuffer, Summary → L2 Episodic
/// - VectorStore, KnowledgeBase → L3 Long-term
/// - EntityMemory, Config → L3 Long-term
/// - Custom → Configurable (default L2)
pub fn translate_memory(
    framework_memory: &FrameworkMemory,
    framework_type: FrameworkType,
) -> Result<MemoryMapping, AdapterError> {
    let target_tier = assign_memory_tier(framework_memory.memory_type);
    let indexing_config = select_indexing_strategy(framework_memory.memory_type, framework_type);

    let mut mapping = MemoryMapping::new(
        framework_memory.memory_id.clone(),
        target_tier,
        indexing_config,
    );

    // Generate provisioning syscalls
    let size_bytes = estimate_memory_size(framework_memory);
    mapping.generate_provisioning_sequence(size_bytes);

    // Add data write syscall
    mapping.add_syscall(MemorySyscall::MemWrite {
        region: format!("{}_region", framework_memory.memory_id),
        data: framework_memory.data.clone(),
    });

    Ok(mapping)
}

/// Assigns memory tier based on memory type.
/// Sec 4.2: Tier Assignment Logic
fn assign_memory_tier(memory_type: MemoryTypeClass) -> MemoryTier {
    match memory_type {
        MemoryTypeClass::ConversationBuffer | MemoryTypeClass::Summary => MemoryTier::L2Episodic,
        MemoryTypeClass::VectorStore
        | MemoryTypeClass::KnowledgeBase
        | MemoryTypeClass::EntityMemory
        | MemoryTypeClass::Config => MemoryTier::L3LongTerm,
        MemoryTypeClass::Custom => MemoryTier::L2Episodic, // Default for custom
    }
}

/// Selects indexing strategy based on memory type and framework.
fn select_indexing_strategy(
    memory_type: MemoryTypeClass,
    _framework_type: FrameworkType,
) -> IndexingConfig {
    match memory_type {
        MemoryTypeClass::ConversationBuffer => IndexingConfig::default(),
        MemoryTypeClass::VectorStore => IndexingConfig::semantic(),
        MemoryTypeClass::KnowledgeBase => IndexingConfig::comprehensive(),
        MemoryTypeClass::EntityMemory => IndexingConfig::comprehensive(),
        MemoryTypeClass::Summary => IndexingConfig::default(),
        MemoryTypeClass::Config => IndexingConfig::default(),
        MemoryTypeClass::Custom => IndexingConfig::default(),
    }
}

/// Estimates memory size from framework memory.
fn estimate_memory_size(framework_memory: &FrameworkMemory) -> u64 {
    // Estimate based on data size and capacity
    let data_size = framework_memory.data.len() as u64;
    let capacity_bytes = (framework_memory.capacity_tokens / 4) as u64; // Rough estimate: 1 token ≈ 4 bytes

    data_size.max(capacity_bytes).saturating_mul(2) // Add 2x buffer for overhead
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

    #[test]
    fn test_memory_type_class_as_str() {
        assert_eq!(MemoryTypeClass::ConversationBuffer.as_str(), "conversation_buffer");
        assert_eq!(MemoryTypeClass::EntityMemory.as_str(), "entity_memory");
        assert_eq!(MemoryTypeClass::VectorStore.as_str(), "vector_store");
        assert_eq!(MemoryTypeClass::KnowledgeBase.as_str(), "knowledge_base");
    }

    #[test]
    fn test_framework_memory_creation() {
        let memory = FrameworkMemory::new(
            "mem1".into(),
            MemoryTypeClass::ConversationBuffer,
            "{}".into(),
            5000,
        );
        assert_eq!(memory.memory_id, "mem1");
        assert_eq!(memory.capacity_tokens, 5000);
    }

    #[test]
    fn test_framework_memory_with_metadata() {
        let memory = FrameworkMemory::new(
            "mem1".into(),
            MemoryTypeClass::ConversationBuffer,
            "{}".into(),
            5000,
        ).with_metadata("metadata".into());

        assert_eq!(memory.metadata, Some("metadata".into()));
    }

    #[test]
    fn test_memory_tier_as_str() {
        assert_eq!(MemoryTier::L1Working.as_str(), "L1_working");
        assert_eq!(MemoryTier::L2Episodic.as_str(), "L2_episodic");
        assert_eq!(MemoryTier::L3LongTerm.as_str(), "L3_longterm");
    }

    #[test]
    fn test_memory_tier_retention() {
        assert_eq!(MemoryTier::L1Working.retention_duration_ms(), 300_000);
        assert_eq!(MemoryTier::L2Episodic.retention_duration_ms(), 86_400_000);
        assert!(MemoryTier::L3LongTerm.retention_duration_ms() > 86_400_000);
    }

    #[test]
    fn test_memory_syscall_names() {
        let alloc = MemorySyscall::MemAlloc {
            tier: MemoryTier::L1Working,
            size: 1024,
        };
        assert_eq!(alloc.syscall_name(), "mem_alloc");

        let write = MemorySyscall::MemWrite {
            region: "reg1".into(),
            data: "data".into(),
        };
        assert_eq!(write.syscall_name(), "mem_write");

        let mount = MemorySyscall::MemMount {
            source: "src".into(),
            point: "pnt".into(),
        };
        assert_eq!(mount.syscall_name(), "mem_mount");

        let clear = MemorySyscall::MemClear {
            region: "reg1".into(),
        };
        assert_eq!(clear.syscall_name(), "mem_clear");
    }

    #[test]
    fn test_indexing_config_default() {
        let config = IndexingConfig::default();
        assert!(config.full_text_index);
        assert!(!config.semantic_index);
    }

    #[test]
    fn test_indexing_config_semantic() {
        let config = IndexingConfig::semantic();
        assert!(!config.full_text_index);
        assert!(config.semantic_index);
    }

    #[test]
    fn test_indexing_config_comprehensive() {
        let config = IndexingConfig::comprehensive();
        assert!(config.full_text_index);
        assert!(config.semantic_index);
    }

    #[test]
    fn test_memory_mapping_creation() {
        let mapping = MemoryMapping::new(
            "mem1".into(),
            MemoryTier::L2Episodic,
            IndexingConfig::default(),
        );
        assert_eq!(mapping.source_memory_id, "mem1");
        assert_eq!(mapping.target_tier, MemoryTier::L2Episodic);
    }

    #[test]
    fn test_memory_mapping_add_syscall() {
        let mut mapping = MemoryMapping::new(
            "mem1".into(),
            MemoryTier::L2Episodic,
            IndexingConfig::default(),
        );
        mapping.add_syscall(MemorySyscall::MemAlloc {
            tier: MemoryTier::L2Episodic,
            size: 1024,
        });

        assert_eq!(mapping.syscall_sequence.len(), 1);
    }

    #[test]
    fn test_memory_mapping_generate_provisioning() {
        let mut mapping = MemoryMapping::new(
            "mem1".into(),
            MemoryTier::L2Episodic,
            IndexingConfig::default(),
        );
        mapping.generate_provisioning_sequence(2048);

        assert_eq!(mapping.syscall_sequence.len(), 1);
        match &mapping.syscall_sequence[0] {
            MemorySyscall::MemAlloc { tier, size } => {
                assert_eq!(*tier, MemoryTier::L2Episodic);
                assert_eq!(*size, 2048);
            }
            _ => panic!("Expected MemAlloc syscall"),
        }
    }

    #[test]
    fn test_assign_memory_tier_conversation() {
        assert_eq!(
            assign_memory_tier(MemoryTypeClass::ConversationBuffer),
            MemoryTier::L2Episodic
        );
        assert_eq!(
            assign_memory_tier(MemoryTypeClass::Summary),
            MemoryTier::L2Episodic
        );
    }

    #[test]
    fn test_assign_memory_tier_persistent() {
        assert_eq!(
            assign_memory_tier(MemoryTypeClass::KnowledgeBase),
            MemoryTier::L3LongTerm
        );
        assert_eq!(
            assign_memory_tier(MemoryTypeClass::EntityMemory),
            MemoryTier::L3LongTerm
        );
        assert_eq!(
            assign_memory_tier(MemoryTypeClass::VectorStore),
            MemoryTier::L3LongTerm
        );
    }

    #[test]
    fn test_translate_memory_conversation() {
        let memory = FrameworkMemory::new(
            "conv1".into(),
            MemoryTypeClass::ConversationBuffer,
            "{}".into(),
            10000,
        );
        let mapping =
            translate_memory(&memory, FrameworkType::LangChain).expect("translation failed");

        assert_eq!(mapping.target_tier, MemoryTier::L2Episodic);
        assert!(!mapping.syscall_sequence.is_empty());
    }

    #[test]
    fn test_translate_memory_knowledge_base() {
        let memory = FrameworkMemory::new(
            "kb1".into(),
            MemoryTypeClass::KnowledgeBase,
            "{}".into(),
            50000,
        );
        let mapping =
            translate_memory(&memory, FrameworkType::SemanticKernel).expect("translation failed");

        assert_eq!(mapping.target_tier, MemoryTier::L3LongTerm);
        assert!(mapping.indexing_config.semantic_index);
    }

    #[test]
    fn test_estimate_memory_size() {
        let memory = FrameworkMemory::new(
            "mem1".into(),
            MemoryTypeClass::ConversationBuffer,
            "test data".into(),
            1000,
        );
        let size = estimate_memory_size(&memory);
        assert!(size > 0);
    }

    #[test]
    fn test_select_indexing_strategy_conversation() {
        let config = select_indexing_strategy(MemoryTypeClass::ConversationBuffer, FrameworkType::LangChain);
        assert!(config.full_text_index);
        assert!(!config.semantic_index);
    }

    #[test]
    fn test_select_indexing_strategy_vector_store() {
        let config = select_indexing_strategy(MemoryTypeClass::VectorStore, FrameworkType::LangChain);
        assert!(!config.full_text_index);
        assert!(config.semantic_index);
    }

    #[test]
    fn test_select_indexing_strategy_knowledge_base() {
        let config = select_indexing_strategy(MemoryTypeClass::KnowledgeBase, FrameworkType::LangChain);
        assert!(config.full_text_index);
        assert!(config.semantic_index);
    }
}
