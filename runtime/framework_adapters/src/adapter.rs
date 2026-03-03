// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! Base adapter trait and lifecycle management for framework adapters.
//!
//! This module defines the foundational `FrameworkAdapter` trait that all framework adapters
//! must implement, along with core types for adapter lifecycle, configuration, and event translation.


/// Core performance target: P95 latency for adapter operations (milliseconds)
pub const P95_LATENCY_TARGET_MS: u32 = 500;

/// Maximum memory per agent instance (megabytes)
pub const MAX_MEMORY_PER_AGENT_MB: u32 = 15;

/// Unique identifier for an agent instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentHandle {
    /// Internal identifier
    id: u64,
}

impl AgentHandle {
    /// Create a new agent handle with the given ID
    pub fn new(id: u64) -> Self {
        AgentHandle { id }
    }

    /// Get the underlying ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Lifecycle state of an adapter instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterLifecycleState {
    /// Adapter created but not initialized
    Created,
    /// Initialization in progress
    Initializing,
    /// Adapter ready for use
    Initialized,
    /// Spawning an agent
    Spawning,
    /// Agent running
    Running,
    /// Shutdown in progress
    Shutting,
    /// Adapter shut down
    Shutdown,
    /// Error state
    Error,
}

/// Configuration for a framework adapter
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Adapter name/identifier
    pub name: String,
    /// Framework type identifier
    pub framework_type: String,
    /// Maximum concurrent agents
    pub max_agents: usize,
    /// Memory limit per agent in MB
    pub memory_limit_mb: u32,
    /// Timeout for operations (milliseconds)
    pub timeout_ms: u32,
}

impl AdapterConfig {
    /// Create a new adapter configuration
    pub fn new(name: String, framework_type: String) -> Self {
        AdapterConfig {
            name,
            framework_type,
            max_agents: 100,
            memory_limit_mb: MAX_MEMORY_PER_AGENT_MB,
            timeout_ms: P95_LATENCY_TARGET_MS,
        }
    }
}

/// Re-export error types from the canonical error module.
pub use crate::error::{AdapterError, AdapterResult};

/// Main trait for framework adapters
///
/// Implementers provide translation between framework-specific concepts and
/// Cognitive Substrate Core Interface (CSCI) primitives.
pub trait FrameworkAdapter {
    /// Initialize the adapter with the given configuration
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()>;

    /// Spawn an agent within this adapter
    ///
    /// Returns an AgentHandle identifying the spawned agent
    fn spawn_agent(&mut self, config: &AdapterConfig) -> AdapterResult<AgentHandle>;

    /// Translate a framework-specific event to CSCI representation
    fn translate_event(&self, event_data: &[u8]) -> AdapterResult<Vec<u8>>;

    /// Shutdown the adapter and all managed agents
    fn shutdown(&mut self) -> AdapterResult<()>;

    /// Get the current lifecycle state
    fn state(&self) -> AdapterLifecycleState;

    /// Check memory usage (returns megabytes used)
    fn memory_used_mb(&self) -> u32;
}

/// Configuration for a cognitive task translated from a framework concept.
/// Sec 4.3: Cognitive Task Mapping
#[derive(Debug, Clone)]
pub struct CognitiveTaskConfig {
    /// Unique task identifier
    pub task_id: String,
    /// Task name
    pub name: String,
    /// Task objective description
    pub objective: String,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether the task is mandatory
    pub is_mandatory: bool,
}

/// Configuration for a semantic channel translated from a framework communication pattern.
/// Sec 4.3: Channel Mapping
#[derive(Debug, Clone)]
pub struct SemanticChannelConfig {
    /// Unique channel identifier
    pub channel_id: String,
    /// Channel name
    pub name: String,
    /// Participant description
    pub participants: String,
    /// Communication pattern type
    pub communication_pattern: String,
    /// Whether the channel is persistent
    pub is_persistent: bool,
}

/// Configuration for semantic memory translated from framework memory.
/// Sec 4.3: Memory Mapping
#[derive(Debug, Clone)]
pub struct SemanticMemoryConfig {
    /// Unique memory identifier
    pub memory_id: String,
    /// Memory type classification
    pub memory_type: String,
    /// Capacity in tokens
    pub capacity_tokens: u64,
    /// Serialization format
    pub serialization_format: String,
}

/// Configuration for tool binding translated from a framework tool.
/// Sec 4.3: Tool Binding Mapping
#[derive(Debug, Clone)]
pub struct ToolBindingConfig {
    /// Unique tool identifier
    pub tool_id: String,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON string)
    pub input_schema: String,
    /// Output schema (JSON string)
    pub output_schema: String,
    /// Whether authorization is required
    pub requires_authorization: bool,
}

/// Result of a framework-to-CSCI translation operation.
/// Sec 5.1: Translation Fidelity Tracking
#[derive(Debug, Clone)]
pub struct TranslationResult {
    /// Identifier of the translated artifact
    pub artifact_id: String,
    /// Type of the translated artifact
    pub artifact_type: String,
    /// Whether the translation succeeded
    pub success: bool,
    /// Fidelity level of the translation
    pub fidelity: String,
    /// Notes about the translation process
    pub translation_notes: String,
}

/// Trait defining the interface for framework adapters that translate framework
/// concepts to CSCI primitives.
/// Sec 4.2: Framework Adapter Interface
pub trait IFrameworkAdapter {
    /// Translate a framework task definition into a cognitive task configuration.
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig>;

    /// Translate a CSCI task result back to a framework result.
    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult>;

    /// Map framework memory to semantic memory configuration.
    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig>;

    /// Map a framework tool to a tool binding configuration.
    fn map_tool(&self, framework_tool: &str) -> AdapterResult<ToolBindingConfig>;

    /// Map a framework communication pattern to a semantic channel.
    fn map_channel(&self, framework_comm: &str) -> AdapterResult<SemanticChannelConfig>;

    /// Get the framework type this adapter supports.
    fn framework_type(&self) -> crate::framework_type::FrameworkType;

    /// Get the supported version range.
    fn supported_versions(&self) -> &str;

    /// Check if a framework artifact is compatible with this adapter.
    fn is_compatible(&self, artifact: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_handle_creation() {
        let handle = AgentHandle::new(42);
        assert_eq!(handle.id(), 42);
    }

    #[test]
    fn test_adapter_config_creation() {
        let config = AdapterConfig::new("test".into(), "langchain".into());
        assert_eq!(config.name, "test");
        assert_eq!(config.framework_type, "langchain");
        assert_eq!(config.memory_limit_mb, MAX_MEMORY_PER_AGENT_MB);
        assert_eq!(config.timeout_ms, P95_LATENCY_TARGET_MS);
    }

    #[test]
    fn test_lifecycle_state_debug() {
        let state = AdapterLifecycleState::Initialized;
        assert_eq!(state, AdapterLifecycleState::Initialized);
    }

    #[test]
    fn test_adapter_error_display() {
        let err = AdapterError::TranslationError("test error".into());
        assert!(!err.to_string().is_empty());
    }
}
