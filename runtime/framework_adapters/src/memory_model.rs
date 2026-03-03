// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Memory Model
//!
//! Defines the mapping of framework-specific memory concepts to Cognitive Substrate
//! memory tiers (L1/L2/L3). Implements tier-specific persistence, sharing, and
//! migration strategies for each framework.
//!
//! Sec 3.3: Memory Tier Architecture
//! Sec 4.3: Framework Memory Mapping
//! Sec 5.4: Memory Migration Heuristics

use crate::framework_type::FrameworkType;

/// Framework-agnostic memory type classifications
/// Sec 4.3: Framework Memory Type Taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkMemoryType {
    /// Multi-turn conversation history or dialogue
    ConversationHistory,
    /// Vector embeddings and similarity search stores
    VectorStore,
    /// Document or text corpus storage
    DocumentStore,
    /// Short-term working memory buffer
    ShortTermBuffer,
    /// Long-term knowledge base
    LongTermKnowledge,
}

impl FrameworkMemoryType {
    /// Returns string representation of the memory type
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameworkMemoryType::ConversationHistory => "conversation_history",
            FrameworkMemoryType::VectorStore => "vector_store",
            FrameworkMemoryType::DocumentStore => "document_store",
            FrameworkMemoryType::ShortTermBuffer => "short_term_buffer",
            FrameworkMemoryType::LongTermKnowledge => "long_term_knowledge",
        }
    }
}

/// Memory tier classification within Cognitive Substrate
/// Sec 3.3: Memory Tier Specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// L1: Register/cache layer, ultra-fast access, minimal capacity
    L1,
    /// L2: Main memory layer, fast access, moderate capacity
    L2,
    /// L3: Persistent storage layer, slower access, high capacity
    L3,
}

impl MemoryTier {
    /// Returns string representation of the tier
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryTier::L1 => "L1",
            MemoryTier::L2 => "L2",
            MemoryTier::L3 => "L3",
        }
    }

    /// Returns typical maximum capacity in tokens for this tier
    /// Sec 3.3: Tier Capacity Characteristics
    pub fn typical_capacity_tokens(&self) -> u64 {
        match self {
            MemoryTier::L1 => 10_000,           // ~40KB
            MemoryTier::L2 => 1_000_000,        // ~4MB
            MemoryTier::L3 => 100_000_000,      // ~400MB
        }
    }
}

/// Data sharing mode for memory across entities
/// Sec 4.3: Memory Sharing Semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SharingMode {
    /// Private to a single entity
    Private,
    /// Shared among agents in same crew
    CrewLocal,
    /// Shared across all entities in the runtime
    Global,
}

impl SharingMode {
    /// Returns string representation of the sharing mode
    pub fn as_str(&self) -> &'static str {
        match self {
            SharingMode::Private => "private",
            SharingMode::CrewLocal => "crew_local",
            SharingMode::Global => "global",
        }
    }
}

/// Persistence policy for memory data
/// Sec 4.3: Memory Persistence Policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersistencePolicy {
    /// Data persists for the lifetime of the runtime session
    SessionLived,
    /// Data persists indefinitely
    Permanent,
    /// Data persists only during task execution
    Transient,
}

impl PersistencePolicy {
    /// Returns string representation of the persistence policy
    pub fn as_str(&self) -> &'static str {
        match self {
            PersistencePolicy::SessionLived => "session_lived",
            PersistencePolicy::Permanent => "permanent",
            PersistencePolicy::Transient => "transient",
        }
    }
}

/// Hint for when memory should migrate between tiers
/// Sec 5.4: Memory Migration Heuristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MigrationHint {
    /// Data should stay in current tier
    Stay,
    /// Data should migrate to faster tier if needed
    PromoteIfActive,
    /// Data should migrate to slower tier to free faster tier space
    DemoteIfInactive,
    /// Data should be demoted immediately to make room
    DemoteNow,
}

impl MigrationHint {
    /// Returns string representation of the migration hint
    pub fn as_str(&self) -> &'static str {
        match self {
            MigrationHint::Stay => "stay",
            MigrationHint::PromoteIfActive => "promote_if_active",
            MigrationHint::DemoteIfInactive => "demote_if_inactive",
            MigrationHint::DemoteNow => "demote_now",
        }
    }
}

/// Mapping of a framework memory concept to CSCI memory tier
/// Sec 4.3: TierMapping Structure
#[derive(Debug, Clone)]
pub struct TierMapping {
    /// The framework memory type
    pub framework_type: FrameworkMemoryType,
    /// Target CSCI memory tier
    pub target_tier: MemoryTier,
    /// Data persistence policy
    pub persistence: PersistencePolicy,
    /// Sharing mode across entities
    pub sharing_mode: SharingMode,
    /// Migration policy for this memory type
    pub migration_hint: MigrationHint,
    /// Notes on translation characteristics
    pub translation_notes: String,
}

impl TierMapping {
    /// Creates a new tier mapping
    /// Sec 4.3: TierMapping Construction
    pub fn new(
        framework_type: FrameworkMemoryType,
        target_tier: MemoryTier,
        persistence: PersistencePolicy,
        sharing_mode: SharingMode,
        migration_hint: MigrationHint,
        translation_notes: &str,
    ) -> Self {
        TierMapping {
            framework_type,
            target_tier,
            persistence,
            sharing_mode,
            migration_hint,
            translation_notes: String::from(translation_notes),
        }
    }
}

/// LangChain memory concept mappings
/// Sec 4.3: LangChain Memory Mapping
pub struct LangChainMemoryMapping;

impl LangChainMemoryMapping {
    /// Returns the memory mappings for LangChain framework
    /// Sec 4.3: LangChain Mapping Rules
    pub fn mappings() -> [TierMapping; 5] {
        [
            // ConversationBufferMemory → L1 (fast access for active conversation)
            TierMapping::new(
                FrameworkMemoryType::ConversationHistory,
                MemoryTier::L1,
                PersistencePolicy::SessionLived,
                SharingMode::Private,
                MigrationHint::PromoteIfActive,
                "LangChain ConversationBufferMemory maps to L1 for low-latency access during active conversation",
            ),
            // VectorStoreRetriever → L2 (moderate-speed vector search)
            TierMapping::new(
                FrameworkMemoryType::VectorStore,
                MemoryTier::L2,
                PersistencePolicy::Permanent,
                SharingMode::CrewLocal,
                MigrationHint::Stay,
                "LangChain VectorStoreRetriever maps to L2 for efficient similarity search",
            ),
            // DocumentStore (e.g., MongoDBDocumentStore) → L3 (persistent storage)
            TierMapping::new(
                FrameworkMemoryType::DocumentStore,
                MemoryTier::L3,
                PersistencePolicy::Permanent,
                SharingMode::Global,
                MigrationHint::DemoteIfInactive,
                "LangChain DocumentStore maps to L3 for archival and long-term knowledge storage",
            ),
            // Working memory → L1 (for intermediate computations)
            TierMapping::new(
                FrameworkMemoryType::ShortTermBuffer,
                MemoryTier::L1,
                PersistencePolicy::Transient,
                SharingMode::Private,
                MigrationHint::DemoteIfInactive,
                "LangChain working memory for intermediate values maps to L1 transient storage",
            ),
            // KnowledgeBase → L3 (reference material)
            TierMapping::new(
                FrameworkMemoryType::LongTermKnowledge,
                MemoryTier::L3,
                PersistencePolicy::Permanent,
                SharingMode::CrewLocal,
                MigrationHint::Stay,
                "LangChain knowledge base maps to L3 persistent storage with crew-local sharing",
            ),
        ]
    }
}

/// Semantic Kernel memory concept mappings
/// Sec 4.3: Semantic Kernel Memory Mapping
pub struct SemanticKernelMemoryMapping;

impl SemanticKernelMemoryMapping {
    /// Returns the memory mappings for Semantic Kernel framework
    /// Sec 4.3: Semantic Kernel Mapping Rules
    pub fn mappings() -> [TierMapping; 5] {
        [
            // VolatileMemoryStore → L1 (ephemeral runtime storage)
            TierMapping::new(
                FrameworkMemoryType::ShortTermBuffer,
                MemoryTier::L1,
                PersistencePolicy::Transient,
                SharingMode::Private,
                MigrationHint::DemoteNow,
                "Semantic Kernel VolatileMemoryStore maps to L1 transient storage for rapid iteration",
            ),
            // SemanticTextMemory → L2 (indexed text search)
            TierMapping::new(
                FrameworkMemoryType::DocumentStore,
                MemoryTier::L2,
                PersistencePolicy::SessionLived,
                SharingMode::CrewLocal,
                MigrationHint::PromoteIfActive,
                "Semantic Kernel SemanticTextMemory maps to L2 for semantic search capabilities",
            ),
            // SqliteMemoryStore → L3 (persistent relational storage)
            TierMapping::new(
                FrameworkMemoryType::DocumentStore,
                MemoryTier::L3,
                PersistencePolicy::Permanent,
                SharingMode::Global,
                MigrationHint::Stay,
                "Semantic Kernel SqliteMemoryStore maps to L3 for persistent structured storage",
            ),
            // ContextVariables (conversation state) → L1
            TierMapping::new(
                FrameworkMemoryType::ConversationHistory,
                MemoryTier::L1,
                PersistencePolicy::SessionLived,
                SharingMode::Private,
                MigrationHint::PromoteIfActive,
                "Semantic Kernel ContextVariables map to L1 for low-latency state access",
            ),
            // EmbeddingStore → L2 (vector similarity)
            TierMapping::new(
                FrameworkMemoryType::VectorStore,
                MemoryTier::L2,
                PersistencePolicy::Permanent,
                SharingMode::CrewLocal,
                MigrationHint::Stay,
                "Semantic Kernel embedding stores map to L2 for vector similarity search",
            ),
        ]
    }
}

/// CrewAI memory concept mappings
/// Sec 4.3: CrewAI Memory Mapping
pub struct CrewAIMemoryMapping;

impl CrewAIMemoryMapping {
    /// Returns the memory mappings for CrewAI framework
    /// Sec 4.3: CrewAI Mapping Rules
    pub fn mappings() -> [TierMapping; 4] {
        [
            // Crew shared memory (team context) → L2
            TierMapping::new(
                FrameworkMemoryType::ConversationHistory,
                MemoryTier::L2,
                PersistencePolicy::SessionLived,
                SharingMode::CrewLocal,
                MigrationHint::PromoteIfActive,
                "CrewAI crew shared memory maps to L2 with crew-local sharing for team context",
            ),
            // Agent memory (role-specific context) → L1
            TierMapping::new(
                FrameworkMemoryType::ShortTermBuffer,
                MemoryTier::L1,
                PersistencePolicy::SessionLived,
                SharingMode::Private,
                MigrationHint::PromoteIfActive,
                "CrewAI agent-specific memory maps to L1 for rapid role context access",
            ),
            // Task results archive → L3
            TierMapping::new(
                FrameworkMemoryType::DocumentStore,
                MemoryTier::L3,
                PersistencePolicy::Permanent,
                SharingMode::CrewLocal,
                MigrationHint::DemoteIfInactive,
                "CrewAI task result archive maps to L3 for historical reference",
            ),
            // Knowledge base (tools, references) → L2/L3 hybrid
            TierMapping::new(
                FrameworkMemoryType::LongTermKnowledge,
                MemoryTier::L2,
                PersistencePolicy::Permanent,
                SharingMode::CrewLocal,
                MigrationHint::PromoteIfActive,
                "CrewAI knowledge base starts in L2, demotes to L3 if unused",
            ),
        ]
    }
}

/// AutoGen memory concept mappings
/// Sec 4.3: AutoGen Memory Mapping
pub struct AutoGenMemoryMapping;

impl AutoGenMemoryMapping {
    /// Returns the memory mappings for AutoGen framework
    /// Sec 4.3: AutoGen Mapping Rules
    pub fn mappings() -> [TierMapping; 3] {
        [
            // Conversation history (agent interactions) → L2
            TierMapping::new(
                FrameworkMemoryType::ConversationHistory,
                MemoryTier::L2,
                PersistencePolicy::SessionLived,
                SharingMode::CrewLocal,
                MigrationHint::PromoteIfActive,
                "AutoGen conversation history maps to L2 for multi-turn dialogue management",
            ),
            // Message buffer (for groupchat) → L1
            TierMapping::new(
                FrameworkMemoryType::ShortTermBuffer,
                MemoryTier::L1,
                PersistencePolicy::Transient,
                SharingMode::CrewLocal,
                MigrationHint::DemoteIfInactive,
                "AutoGen message buffer maps to L1 for rapid message throughput",
            ),
            // User feedback / execution results → L3
            TierMapping::new(
                FrameworkMemoryType::DocumentStore,
                MemoryTier::L3,
                PersistencePolicy::Permanent,
                SharingMode::Global,
                MigrationHint::Stay,
                "AutoGen execution results and user feedback map to L3 for audit trail",
            ),
        ]
    }
}

/// Comprehensive memory mapping registry
/// Sec 4.3: Memory Mapping Registry
pub struct MemoryMappingRegistry;

impl MemoryMappingRegistry {
    /// Gets all tier mappings for a specific framework
    /// Sec 4.3: Framework-Specific Mapping Lookup
    pub fn get_mappings(framework: FrameworkType) -> Vec<TierMapping> {
        match framework {
            FrameworkType::LangChain => LangChainMemoryMapping::mappings().to_vec(),
            FrameworkType::SemanticKernel => SemanticKernelMemoryMapping::mappings().to_vec(),
            FrameworkType::CrewAI => CrewAIMemoryMapping::mappings().to_vec(),
            FrameworkType::AutoGen => AutoGenMemoryMapping::mappings().to_vec(),
        }
    }

    /// Looks up the tier mapping for a specific framework memory type
    /// Sec 4.3: Memory Type Lookup
    pub fn lookup(
        framework: FrameworkType,
        memory_type: FrameworkMemoryType,
    ) -> Option<TierMapping> {
        Self::get_mappings(framework)
            .into_iter()
            .find(|m| m.framework_type == memory_type)
    }

    /// Suggests a migration action based on framework type and memory type
    /// Sec 5.4: Migration Suggestion
    pub fn suggest_migration(
        framework: FrameworkType,
        memory_type: FrameworkMemoryType,
    ) -> MigrationHint {
        Self::lookup(framework, memory_type)
            .map(|m| m.migration_hint)
            .unwrap_or(MigrationHint::Stay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_memory_type_as_str() {
        assert_eq!(FrameworkMemoryType::ConversationHistory.as_str(), "conversation_history");
        assert_eq!(FrameworkMemoryType::VectorStore.as_str(), "vector_store");
        assert_eq!(FrameworkMemoryType::DocumentStore.as_str(), "document_store");
        assert_eq!(FrameworkMemoryType::ShortTermBuffer.as_str(), "short_term_buffer");
        assert_eq!(FrameworkMemoryType::LongTermKnowledge.as_str(), "long_term_knowledge");
    }

    #[test]
    fn test_memory_tier_as_str() {
        assert_eq!(MemoryTier::L1.as_str(), "L1");
        assert_eq!(MemoryTier::L2.as_str(), "L2");
        assert_eq!(MemoryTier::L3.as_str(), "L3");
    }

    #[test]
    fn test_memory_tier_capacity() {
        assert_eq!(MemoryTier::L1.typical_capacity_tokens(), 10_000);
        assert_eq!(MemoryTier::L2.typical_capacity_tokens(), 1_000_000);
        assert_eq!(MemoryTier::L3.typical_capacity_tokens(), 100_000_000);
    }

    #[test]
    fn test_sharing_mode_as_str() {
        assert_eq!(SharingMode::Private.as_str(), "private");
        assert_eq!(SharingMode::CrewLocal.as_str(), "crew_local");
        assert_eq!(SharingMode::Global.as_str(), "global");
    }

    #[test]
    fn test_persistence_policy_as_str() {
        assert_eq!(PersistencePolicy::SessionLived.as_str(), "session_lived");
        assert_eq!(PersistencePolicy::Permanent.as_str(), "permanent");
        assert_eq!(PersistencePolicy::Transient.as_str(), "transient");
    }

    #[test]
    fn test_migration_hint_as_str() {
        assert_eq!(MigrationHint::Stay.as_str(), "stay");
        assert_eq!(MigrationHint::PromoteIfActive.as_str(), "promote_if_active");
        assert_eq!(MigrationHint::DemoteIfInactive.as_str(), "demote_if_inactive");
        assert_eq!(MigrationHint::DemoteNow.as_str(), "demote_now");
    }

    #[test]
    fn test_tier_mapping_creation() {
        let mapping = TierMapping::new(
            FrameworkMemoryType::ConversationHistory,
            MemoryTier::L1,
            PersistencePolicy::SessionLived,
            SharingMode::Private,
            MigrationHint::PromoteIfActive,
            "Test mapping",
        );
        assert_eq!(mapping.framework_type, FrameworkMemoryType::ConversationHistory);
        assert_eq!(mapping.target_tier, MemoryTier::L1);
    }

    #[test]
    fn test_langchain_memory_mappings() {
        let mappings = LangChainMemoryMapping::mappings();
        assert_eq!(mappings.len(), 5);
        assert_eq!(mappings[0].target_tier, MemoryTier::L1);
        assert_eq!(mappings[1].target_tier, MemoryTier::L2);
        assert_eq!(mappings[2].target_tier, MemoryTier::L3);
    }

    #[test]
    fn test_semantic_kernel_memory_mappings() {
        let mappings = SemanticKernelMemoryMapping::mappings();
        assert_eq!(mappings.len(), 5);
        assert_eq!(mappings[0].target_tier, MemoryTier::L1);
    }

    #[test]
    fn test_crewai_memory_mappings() {
        let mappings = CrewAIMemoryMapping::mappings();
        assert_eq!(mappings.len(), 4);
    }

    #[test]
    fn test_autogen_memory_mappings() {
        let mappings = AutoGenMemoryMapping::mappings();
        assert_eq!(mappings.len(), 3);
    }

    #[test]
    fn test_memory_mapping_registry_langchain() {
        let mappings = MemoryMappingRegistry::get_mappings(FrameworkType::LangChain);
        assert!(!mappings.is_empty());
    }

    #[test]
    fn test_memory_mapping_registry_lookup() {
        let mapping = MemoryMappingRegistry::lookup(
            FrameworkType::LangChain,
            FrameworkMemoryType::ConversationHistory,
        );
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().target_tier, MemoryTier::L1);
    }

    #[test]
    fn test_memory_mapping_registry_lookup_all_frameworks() {
        let frameworks = [
            FrameworkType::LangChain,
            FrameworkType::SemanticKernel,
            FrameworkType::CrewAI,
            FrameworkType::AutoGen,
        ];
        for framework in &frameworks {
            let mappings = MemoryMappingRegistry::get_mappings(*framework);
            assert!(!mappings.is_empty());
        }
    }

    #[test]
    fn test_memory_mapping_registry_suggest_migration() {
        let hint = MemoryMappingRegistry::suggest_migration(
            FrameworkType::LangChain,
            FrameworkMemoryType::ConversationHistory,
        );
        assert_eq!(hint, MigrationHint::PromoteIfActive);
    }

    #[test]
    fn test_memory_mapping_registry_unknown_type() {
        // Verify that unknown lookups don't panic and return defaults
        let hint =
            MemoryMappingRegistry::suggest_migration(FrameworkType::AutoGen, FrameworkMemoryType::ConversationHistory);
        assert_eq!(hint, MigrationHint::PromoteIfActive);
    }
}
