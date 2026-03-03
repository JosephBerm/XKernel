// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Concept Mapping Matrix
//!
//! Bidirectional mapping between framework-specific concepts and CSCI (Cognitive Substrate Core Interface)
//! entities. Defines the translation semantics and fidelity characteristics of each mapping.
//!
//! Sec 4.3: Concept Mapping Matrix
//! Sec 5.1: Translation Fidelity Tracking

use alloc::{string::String, vec::Vec};

/// Framework-specific concepts that require mapping to CSCI entities.
/// Sec 4.3: Framework Concept Taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkConcept {
    /// Executable workflow or chain of operations
    Chain,
    /// Pluggable capability or skill
    Tool,
    /// State or knowledge persistence mechanism
    Memory,
    /// Autonomous entity with behavior and state
    Agent,
    /// Decision-making or execution orchestration
    Planner,
    /// Reusable semantic function
    Skill,
    /// Extension or module
    Plugin,
    /// Multi-turn dialogue or interaction
    Conversation,
    /// Agent identity or responsibility specification
    Role,
    /// Multi-agent collective
    Crew,
    /// Callable function or method
    Function,
    /// Unit of work or execution
    Task,
}

impl FrameworkConcept {
    /// Returns string representation of the concept.
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameworkConcept::Chain => "chain",
            FrameworkConcept::Tool => "tool",
            FrameworkConcept::Memory => "memory",
            FrameworkConcept::Agent => "agent",
            FrameworkConcept::Planner => "planner",
            FrameworkConcept::Skill => "skill",
            FrameworkConcept::Plugin => "plugin",
            FrameworkConcept::Conversation => "conversation",
            FrameworkConcept::Role => "role",
            FrameworkConcept::Crew => "crew",
            FrameworkConcept::Function => "function",
            FrameworkConcept::Task => "task",
        }
    }
}

/// CSCI (Cognitive Substrate Core Interface) entity types.
/// Sec 4.3: CSCI Entity Taxonomy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsciEntity {
    /// Cognitive task: atomic unit of cognitive work
    CognitiveTask,
    /// Agent: autonomous entity with goals and capabilities
    Agent,
    /// Agent crew: coordinated multi-agent collective
    AgentCrew,
    /// Capability: specialized capability or behavior
    Capability,
    /// Semantic memory: persistent knowledge representation
    SemanticMemory,
    /// Semantic channel: communication pathway with semantic constraints
    SemanticChannel,
    /// Cognitive exception: error condition in cognitive processing
    CognitiveException,
    /// Cognitive signal: control signal or notification
    CognitiveSignal,
    /// Cognitive checkpoint: savepoint in execution state
    CognitiveCheckpoint,
    /// Mandatory capability policy: non-negotiable capability requirements
    MandatoryCapabilityPolicy,
    /// Tool binding: interface to external tool or capability
    ToolBinding,
    /// Watchdog configuration: monitoring and safety policy
    WatchdogConfig,
}

impl CsciEntity {
    /// Returns string representation of the CSCI entity.
    pub fn as_str(&self) -> &'static str {
        match self {
            CsciEntity::CognitiveTask => "cognitive_task",
            CsciEntity::Agent => "agent",
            CsciEntity::AgentCrew => "agent_crew",
            CsciEntity::Capability => "capability",
            CsciEntity::SemanticMemory => "semantic_memory",
            CsciEntity::SemanticChannel => "semantic_channel",
            CsciEntity::CognitiveException => "cognitive_exception",
            CsciEntity::CognitiveSignal => "cognitive_signal",
            CsciEntity::CognitiveCheckpoint => "cognitive_checkpoint",
            CsciEntity::MandatoryCapabilityPolicy => "mandatory_capability_policy",
            CsciEntity::ToolBinding => "tool_binding",
            CsciEntity::WatchdogConfig => "watchdog_config",
        }
    }
}

/// Fidelity level of a concept mapping.
/// Sec 5.1: Translation Fidelity Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingFidelity {
    /// Complete semantic equivalence; no loss of expressiveness
    Full,
    /// Most semantics preserved; minor aspects may degrade
    Partial,
    /// Significant semantic approximation; core functionality preserved
    Approximate,
    /// No direct mapping; requires custom translation or simulation
    NoDirectMapping,
}

impl MappingFidelity {
    /// Returns string representation of the fidelity level.
    pub fn as_str(&self) -> &'static str {
        match self {
            MappingFidelity::Full => "full",
            MappingFidelity::Partial => "partial",
            MappingFidelity::Approximate => "approximate",
            MappingFidelity::NoDirectMapping => "no_direct_mapping",
        }
    }

    /// Returns true if mapping fidelity is acceptable (not NoDirectMapping).
    pub fn is_acceptable(&self) -> bool {
        !matches!(self, MappingFidelity::NoDirectMapping)
    }
}

/// Mapping of a single framework concept to a CSCI entity.
/// Sec 4.3: Concept Mapping Entry
#[derive(Debug, Clone)]
pub struct ConceptMapping {
    /// Framework concept being mapped
    pub framework_concept: FrameworkConcept,
    /// Target CSCI entity
    pub csci_entity: CsciEntity,
    /// Translation fidelity level
    pub fidelity: MappingFidelity,
    /// Notes on translation semantics and constraints
    pub translation_notes: String,
}

impl ConceptMapping {
    /// Creates a new concept mapping.
    /// Sec 4.3: Mapping Construction
    pub fn new(
        framework_concept: FrameworkConcept,
        csci_entity: CsciEntity,
        fidelity: MappingFidelity,
        translation_notes: &str,
    ) -> Self {
        ConceptMapping {
            framework_concept,
            csci_entity,
            fidelity,
            translation_notes: String::from(translation_notes),
        }
    }
}

/// Complete mapping matrix for all framework concepts to CSCI entities.
/// Sec 4.3: Complete Concept Mapping Matrix (5×12)
pub struct MappingMatrix {
    mappings: Vec<ConceptMapping>,
}

impl MappingMatrix {
    /// Creates a new mapping matrix with all standard mappings.
    /// Sec 4.3: Matrix Initialization
    pub fn new() -> Self {
        let mut mappings = Vec::new();

        // LangChain mappings
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Chain,
            CsciEntity::CognitiveTask,
            MappingFidelity::Full,
            "LangChain chain is a sequence of operations mapping directly to CognitiveTask",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Tool,
            CsciEntity::ToolBinding,
            MappingFidelity::Full,
            "LangChain tool maps to ToolBinding for capability invocation",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Memory,
            CsciEntity::SemanticMemory,
            MappingFidelity::Partial,
            "LangChain memory is approximated by SemanticMemory; schema translation required",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Agent,
            CsciEntity::Agent,
            MappingFidelity::Full,
            "LangChain agent maps directly to CSCI Agent with goal pursuit and autonomy",
        ));

        // Semantic Kernel mappings
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Skill,
            CsciEntity::ToolBinding,
            MappingFidelity::Full,
            "Semantic Kernel skill is a semantic function mapping to ToolBinding",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Plugin,
            CsciEntity::ToolBinding,
            MappingFidelity::Partial,
            "Semantic Kernel plugin (container) maps to ToolBinding with aggregation semantics",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Planner,
            CsciEntity::CognitiveTask,
            MappingFidelity::Approximate,
            "Semantic Kernel planner orchestrates execution; approximated as CognitiveTask",
        ));

        // CrewAI mappings
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Crew,
            CsciEntity::AgentCrew,
            MappingFidelity::Full,
            "CrewAI crew maps directly to AgentCrew for coordinated multi-agent execution",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Task,
            CsciEntity::CognitiveTask,
            MappingFidelity::Full,
            "CrewAI task maps directly to CognitiveTask",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Role,
            CsciEntity::Agent,
            MappingFidelity::Full,
            "CrewAI role specifies agent identity and responsibilities, mapping to Agent",
        ));

        // AutoGen mappings
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Agent,
            CsciEntity::Agent,
            MappingFidelity::Full,
            "AutoGen agent maps directly to CSCI Agent",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Function,
            CsciEntity::ToolBinding,
            MappingFidelity::Full,
            "AutoGen function maps to ToolBinding for execution",
        ));
        mappings.push(ConceptMapping::new(
            FrameworkConcept::Conversation,
            CsciEntity::SemanticChannel,
            MappingFidelity::Partial,
            "AutoGen conversation approximated as SemanticChannel; turn-taking semantics translate imperfectly",
        ));

        MappingMatrix { mappings }
    }

    /// Looks up a mapping by framework concept.
    /// Sec 4.3: Mapping Lookup
    pub fn find_by_concept(&self, concept: FrameworkConcept) -> Option<&ConceptMapping> {
        self.mappings.iter().find(|m| m.framework_concept == concept)
    }

    /// Looks up mappings by CSCI entity.
    /// Sec 4.3: Reverse Lookup
    pub fn find_by_entity(&self, entity: CsciEntity) -> Vec<&ConceptMapping> {
        self.mappings.iter().filter(|m| m.csci_entity == entity).collect()
    }

    /// Returns all mappings.
    pub fn all(&self) -> &[ConceptMapping] {
        &self.mappings
    }

    /// Returns the number of mappings in the matrix.
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Returns true if the matrix is empty.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

impl Default for MappingMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::vec::Vec;

    #[test]
    fn test_framework_concept_as_str() {
        assert_eq!(FrameworkConcept::Chain.as_str(), "chain");
        assert_eq!(FrameworkConcept::Tool.as_str(), "tool");
        assert_eq!(FrameworkConcept::Agent.as_str(), "agent");
    }

    #[test]
    fn test_csci_entity_as_str() {
        assert_eq!(CsciEntity::CognitiveTask.as_str(), "cognitive_task");
        assert_eq!(CsciEntity::Agent.as_str(), "agent");
        assert_eq!(CsciEntity::ToolBinding.as_str(), "tool_binding");
    }

    #[test]
    fn test_mapping_fidelity_is_acceptable() {
        assert!(MappingFidelity::Full.is_acceptable());
        assert!(MappingFidelity::Partial.is_acceptable());
        assert!(MappingFidelity::Approximate.is_acceptable());
        assert!(!MappingFidelity::NoDirectMapping.is_acceptable());
    }

    #[test]
    fn test_concept_mapping_creation() {
        let mapping = ConceptMapping::new(
            FrameworkConcept::Chain,
            CsciEntity::CognitiveTask,
            MappingFidelity::Full,
            "Test mapping",
        );
        assert_eq!(mapping.framework_concept, FrameworkConcept::Chain);
        assert_eq!(mapping.csci_entity, CsciEntity::CognitiveTask);
        assert_eq!(mapping.fidelity, MappingFidelity::Full);
    }

    #[test]
    fn test_mapping_matrix_initialization() {
        let matrix = MappingMatrix::new();
        assert!(!matrix.is_empty());
        assert!(matrix.len() > 0);
    }

    #[test]
    fn test_mapping_matrix_find_by_concept() {
        let matrix = MappingMatrix::new();
        let mapping = matrix.find_by_concept(FrameworkConcept::Chain);
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().csci_entity, CsciEntity::CognitiveTask);
    }

    #[test]
    fn test_mapping_matrix_find_by_entity() {
        let matrix = MappingMatrix::new();
        let mappings = matrix.find_by_entity(CsciEntity::ToolBinding);
        assert!(!mappings.is_empty());
    }

    #[test]
    fn test_mapping_matrix_default() {
        let matrix = MappingMatrix::default();
        assert!(!matrix.is_empty());
    }

    #[test]
    fn test_mapping_matrix_complete() {
        let matrix = MappingMatrix::new();
        // Verify we have at least one mapping for each major framework
        assert!(matrix.find_by_concept(FrameworkConcept::Chain).is_some());
        assert!(matrix.find_by_concept(FrameworkConcept::Crew).is_some());
        assert!(matrix.find_by_concept(FrameworkConcept::Conversation).is_some());
    }
}
