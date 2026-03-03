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
