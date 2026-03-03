// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Common Adapter Interface Pattern
//!
//! Unified adapter pattern applied to all five supported frameworks:
//! 1. LangChain
//! 2. Semantic Kernel
//! 3. AutoGen
//! 4. CrewAI
//! 5. Custom Framework
//!
//! Demonstrates the universal contract that all framework adapters must implement.
//! Provides consistent lifecycle, initialization, translation, and result handling across all frameworks.
//!
//! Sec 4.2: Universal Adapter Pattern
//! Sec 4.3: Framework Concept Mapping
//! Sec 4.2: Adapter Lifecycle

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::framework_type::FrameworkType;
use crate::{AdapterResult, error::AdapterError};

/// Universal adapter lifecycle state.
/// Sec 4.2: Adapter Lifecycle States
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterLifecycleState {
    /// Adapter created but not initialized
    Uninitialized,
    /// Adapter initialized and ready
    Ready,
    /// Adapter actively translating
    Translating,
    /// Adapter waiting for framework response
    Waiting,
    /// Adapter has encountered an error
    Error,
    /// Adapter shut down cleanly
    Shutdown,
}

impl AdapterLifecycleState {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            AdapterLifecycleState::Uninitialized => "uninitialized",
            AdapterLifecycleState::Ready => "ready",
            AdapterLifecycleState::Translating => "translating",
            AdapterLifecycleState::Waiting => "waiting",
            AdapterLifecycleState::Error => "error",
            AdapterLifecycleState::Shutdown => "shutdown",
        }
    }
}

/// Configuration parameters for adapter initialization.
/// Sec 4.2: Adapter Configuration
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Framework version specification
    pub framework_version: String,
    /// Timeout in milliseconds for operations
    pub timeout_ms: u64,
    /// Maximum retries for failed operations
    pub max_retries: u32,
    /// Whether to enable streaming responses
    pub enable_streaming: bool,
    /// Custom configuration parameters
    pub custom_params: BTreeMap<String, String>,
}

impl AdapterConfig {
    /// Creates a new adapter configuration with defaults.
    pub fn new(framework_version: String) -> Self {
        AdapterConfig {
            framework_version,
            timeout_ms: 30000,
            max_retries: 3,
            enable_streaming: false,
            custom_params: BTreeMap::new(),
        }
    }

    /// Sets a custom parameter.
    pub fn set_param(&mut self, key: String, value: String) {
        self.custom_params.insert(key, value);
    }
}

/// Unified adapter trait implementing the common pattern across all frameworks.
/// Sec 4.2: UniversalFrameworkAdapter Interface
pub trait UniversalFrameworkAdapter {
    /// Initializes the adapter with configuration.
    /// Sec 4.2: Adapter Initialization
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()>;

    /// Loads agent or entity from framework format.
    /// Sec 4.2: Agent Loading
    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String>;

    /// Translates framework plan to CT spawner directives.
    /// Sec 4.2: Plan Translation
    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String>;

    /// Spawns tasks based on translated plan.
    /// Sec 4.2: Task Spawning
    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>>;

    /// Collects results from completed tasks.
    /// Sec 4.2: Result Collection
    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String>;

    /// Returns current adapter state.
    fn get_state(&self) -> AdapterLifecycleState;

    /// Shuts down the adapter cleanly.
    fn shutdown(&mut self) -> AdapterResult<()>;
}

/// LangChain adapter implementing the universal pattern.
/// Sec 4.3: LangChain Adapter Instance
#[derive(Debug, Clone)]
pub struct LangChainUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    loaded_agents: BTreeMap<String, String>,
}

impl LangChainUniversalAdapter {
    /// Creates a new LangChain universal adapter.
    pub fn new() -> Self {
        LangChainUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            loaded_agents: BTreeMap::new(),
        }
    }
}

impl Default for LangChainUniversalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalFrameworkAdapter for LangChainUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty agent definition".into()));
        }
        Ok(alloc::format!("lc-agent-{}", agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plan definition".into()));
        }
        Ok(alloc::format!("lc-spawner-{}", plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        let task_id = alloc::format!("lc-task-{}", spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs provided".into()));
        }
        Ok(alloc::format!("lc-result-{}", task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_agents.clear();
        Ok(())
    }
}

/// Semantic Kernel adapter implementing the universal pattern.
/// Sec 4.3: Semantic Kernel Adapter Instance
#[derive(Debug, Clone)]
pub struct SemanticKernelUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    loaded_plugins: BTreeMap<String, String>,
}

impl SemanticKernelUniversalAdapter {
    /// Creates a new Semantic Kernel universal adapter.
    pub fn new() -> Self {
        SemanticKernelUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            loaded_plugins: BTreeMap::new(),
        }
    }
}

impl Default for SemanticKernelUniversalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalFrameworkAdapter for SemanticKernelUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plugin definition".into()));
        }
        Ok(alloc::format!("sk-plugin-{}", agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plan definition".into()));
        }
        Ok(alloc::format!("sk-spawner-{}", plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        let task_id = alloc::format!("sk-task-{}", spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs provided".into()));
        }
        Ok(alloc::format!("sk-result-{}", task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_plugins.clear();
        Ok(())
    }
}

/// AutoGen adapter implementing the universal pattern.
/// Sec 4.3: AutoGen Adapter Instance
#[derive(Debug, Clone)]
pub struct AutoGenUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    loaded_agents: BTreeMap<String, String>,
}

impl AutoGenUniversalAdapter {
    /// Creates a new AutoGen universal adapter.
    pub fn new() -> Self {
        AutoGenUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            loaded_agents: BTreeMap::new(),
        }
    }
}

impl Default for AutoGenUniversalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalFrameworkAdapter for AutoGenUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty agent definition".into()));
        }
        Ok(alloc::format!("ag-agent-{}", agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plan definition".into()));
        }
        Ok(alloc::format!("ag-spawner-{}", plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        let task_id = alloc::format!("ag-task-{}", spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs provided".into()));
        }
        Ok(alloc::format!("ag-result-{}", task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_agents.clear();
        Ok(())
    }
}

/// CrewAI adapter implementing the universal pattern.
/// Sec 4.3: CrewAI Adapter Instance
#[derive(Debug, Clone)]
pub struct CrewAIUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    loaded_crews: BTreeMap<String, String>,
}

impl CrewAIUniversalAdapter {
    /// Creates a new CrewAI universal adapter.
    pub fn new() -> Self {
        CrewAIUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            loaded_crews: BTreeMap::new(),
        }
    }
}

impl Default for CrewAIUniversalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalFrameworkAdapter for CrewAIUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty agent definition".into()));
        }
        Ok(alloc::format!("crew-agent-{}", agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty task definition".into()));
        }
        Ok(alloc::format!("crew-spawner-{}", plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        let task_id = alloc::format!("crew-task-{}", spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs provided".into()));
        }
        Ok(alloc::format!("crew-result-{}", task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_crews.clear();
        Ok(())
    }
}

/// Custom Framework adapter implementing the universal pattern.
/// Sec 4.3: Custom Framework Adapter Instance
#[derive(Debug, Clone)]
pub struct CustomFrameworkUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    framework_name: String,
    loaded_entities: BTreeMap<String, String>,
}

impl CustomFrameworkUniversalAdapter {
    /// Creates a new custom framework universal adapter.
    pub fn new(framework_name: String) -> Self {
        CustomFrameworkUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            framework_name,
            loaded_entities: BTreeMap::new(),
        }
    }
}

impl UniversalFrameworkAdapter for CustomFrameworkUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty entity definition".into()));
        }
        Ok(alloc::format!("custom-{}-entity-{}", self.framework_name, agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plan definition".into()));
        }
        Ok(alloc::format!("custom-{}-spawner-{}", self.framework_name, plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        let task_id = alloc::format!("custom-{}-task-{}", self.framework_name, spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs provided".into()));
        }
        Ok(alloc::format!("custom-{}-result-{}", self.framework_name, task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_entities.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_adapter_lifecycle_state_as_str() {
        assert_eq!(AdapterLifecycleState::Uninitialized.as_str(), "uninitialized");
        assert_eq!(AdapterLifecycleState::Ready.as_str(), "ready");
        assert_eq!(AdapterLifecycleState::Shutdown.as_str(), "shutdown");
    }

    #[test]
    fn test_adapter_config_creation() {
        let config = AdapterConfig::new("1.0.0".into());
        assert_eq!(config.framework_version, "1.0.0");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_adapter_config_custom_params() {
        let mut config = AdapterConfig::new("1.0.0".into());
        config.set_param("key1".into(), "value1".into());
        assert_eq!(config.custom_params.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_langchain_adapter_initialization() {
        let mut adapter = LangChainUniversalAdapter::new();
        let config = AdapterConfig::new("0.1.0".into());
        let result = adapter.initialize(config);
        
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_langchain_adapter_load_agent() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.load_agent("agent_def");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("lc-agent"));
    }

    #[test]
    fn test_langchain_adapter_translate_plan() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.translate_plan("plan_def");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("lc-spawner"));
    }

    #[test]
    fn test_langchain_adapter_spawn_tasks() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.spawn_tasks("directive");
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_langchain_adapter_collect_results() {
        let adapter = LangChainUniversalAdapter::new();
        let task_ids = vec!["task-1".into(), "task-2".into()];
        let result = adapter.collect_results(&task_ids);
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("lc-result"));
    }

    #[test]
    fn test_langchain_adapter_shutdown() {
        let mut adapter = LangChainUniversalAdapter::new();
        let config = AdapterConfig::new("0.1.0".into());
        let _ = adapter.initialize(config);
        
        let result = adapter.shutdown();
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Shutdown);
    }

    #[test]
    fn test_semantic_kernel_adapter_initialization() {
        let mut adapter = SemanticKernelUniversalAdapter::new();
        let config = AdapterConfig::new("1.0.0".into());
        let result = adapter.initialize(config);
        
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_semantic_kernel_adapter_load_agent() {
        let adapter = SemanticKernelUniversalAdapter::new();
        let result = adapter.load_agent("plugin_def");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("sk-plugin"));
    }

    #[test]
    fn test_autogen_adapter_initialization() {
        let mut adapter = AutoGenUniversalAdapter::new();
        let config = AdapterConfig::new("0.20.0".into());
        let result = adapter.initialize(config);
        
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_crewai_adapter_initialization() {
        let mut adapter = CrewAIUniversalAdapter::new();
        let config = AdapterConfig::new("0.1.0".into());
        let result = adapter.initialize(config);
        
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_custom_framework_adapter_initialization() {
        let mut adapter = CustomFrameworkUniversalAdapter::new("MyFramework".into());
        let config = AdapterConfig::new("1.0.0".into());
        let result = adapter.initialize(config);
        
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_custom_framework_adapter_load_agent() {
        let adapter = CustomFrameworkUniversalAdapter::new("MyFramework".into());
        let result = adapter.load_agent("entity_def");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("MyFramework"));
    }

    #[test]
    fn test_empty_agent_definition_error() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.load_agent("");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plan_definition_error() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.translate_plan("");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_task_ids_error() {
        let adapter = LangChainUniversalAdapter::new();
        let result = adapter.collect_results(&[]);
        assert!(result.is_err());
    }
}
