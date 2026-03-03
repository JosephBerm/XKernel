// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Adapter Interface Contract
//!
//! Defines the final RuntimeAdapterRef interface contract with method signatures,
//! error handling, and state management for framework adapters.
//!
//! This module finalizes the adapter interface specification, including:
//! - Core adapter methods (load_agent, translate_chain, spawn_tasks, collect_results, on_error)
//! - Adapter states (Initialized, AgentLoaded, PlanTranslated, TasksSpawned, ResultsCollected, Failed)
//! - Error types specific to adapter operations
//! - Configuration types and validation
//!
//! Sec 4.2: RuntimeAdapterRef Interface Contract
//! Sec 5.2: Adapter Interface Contract Specification

use alloc::{string::String, vec::Vec, boxed::Box};
use crate::framework_type::FrameworkType;
use crate::AdapterResult;
use crate::error::AdapterError;

/// Adapter state machine states.
/// Sec 5.2: Adapter Lifecycle States
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterState {
    /// Adapter created but not configured
    Initialized,
    /// Agent configuration loaded successfully
    AgentLoaded,
    /// Framework plan translated to CT DAG
    PlanTranslated,
    /// Tasks spawned on kernel
    TasksSpawned,
    /// Results collected from kernel
    ResultsCollected,
    /// Error occurred during processing
    Failed,
}

impl AdapterState {
    /// Returns string representation of adapter state.
    pub fn as_str(&self) -> &'static str {
        match self {
            AdapterState::Initialized => "initialized",
            AdapterState::AgentLoaded => "agent_loaded",
            AdapterState::PlanTranslated => "plan_translated",
            AdapterState::TasksSpawned => "tasks_spawned",
            AdapterState::ResultsCollected => "results_collected",
            AdapterState::Failed => "failed",
        }
    }

    /// Returns true if adapter is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AdapterState::ResultsCollected | AdapterState::Failed
        )
    }
}

/// Framework-specific agent configuration.
/// Sec 5.2: Framework Agent Configuration
#[derive(Debug, Clone)]
pub struct FrameworkAgentConfig {
    /// Agent identifier from framework
    pub agent_id: String,
    /// Agent name
    pub name: String,
    /// Agent description/role
    pub description: String,
    /// Agent system prompt or initialization
    pub system_prompt: String,
    /// Memory type (e.g., "conversation", "entity")
    pub memory_type: String,
    /// Maximum memory capacity in tokens
    pub memory_capacity_tokens: u64,
    /// Tool identifiers available to agent
    pub tool_ids: Vec<String>,
    /// Timeout for agent operations in milliseconds
    pub timeout_ms: u64,
    /// Whether agent execution is mandatory
    pub is_mandatory: bool,
}

/// Framework-specific chain/plan definition.
/// Sec 5.2: Framework Chain Definition
#[derive(Debug, Clone)]
pub struct FrameworkChainDefinition {
    /// Chain/plan identifier
    pub chain_id: String,
    /// Chain name
    pub name: String,
    /// Chain type (sequential, conditional, parallel, etc.)
    pub chain_type: String,
    /// Step definitions
    pub steps: Vec<ChainStepDefinition>,
    /// Total timeout for chain execution
    pub timeout_ms: u64,
}

/// Single step within a framework chain/plan.
/// Sec 5.2: Chain Step Definition
#[derive(Debug, Clone)]
pub struct ChainStepDefinition {
    /// Step identifier
    pub step_id: String,
    /// Step name
    pub name: String,
    /// Step action/operation
    pub action: String,
    /// Input requirements
    pub input_schema: String,
    /// Output specification
    pub output_schema: String,
    /// Timeout for this step
    pub timeout_ms: u64,
    /// Dependencies on other steps
    pub depends_on: Vec<String>,
}

/// Framework-specific result item.
/// Sec 5.2: Framework Result Item
#[derive(Debug, Clone)]
pub struct FrameworkResultItem {
    /// Result identifier
    pub result_id: String,
    /// Result source (task_id)
    pub source_id: String,
    /// Result data (serialized)
    pub data: String,
    /// Result status
    pub status: String,
    /// Human-readable result
    pub output: String,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Adapter error information for on_error callback.
/// Sec 5.2: Adapter Error Information
#[derive(Debug, Clone)]
pub struct AdapterErrorInfo {
    /// Error identifier
    pub error_id: String,
    /// Error type
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Error source (agent_id, task_id, etc.)
    pub source_id: String,
    /// Stack trace or diagnostic info
    pub diagnostic_info: String,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Whether error is recoverable
    pub is_recoverable: bool,
}

/// Error recovery action recommended by adapter.
/// Sec 5.2: Error Recovery Actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRecoveryAction {
    /// Retry the failed operation
    Retry,
    /// Skip this step and continue
    Skip,
    /// Escalate to parent handler
    Escalate,
    /// Abort entire operation
    Abort,
}

impl ErrorRecoveryAction {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorRecoveryAction::Retry => "retry",
            ErrorRecoveryAction::Skip => "skip",
            ErrorRecoveryAction::Escalate => "escalate",
            ErrorRecoveryAction::Abort => "abort",
        }
    }
}

/// Core adapter interface contract.
/// Sec 5.2: Final Adapter Interface Contract
pub trait RuntimeAdapterContract {
    /// Returns the framework type this adapter handles.
    /// Sec 5.2: Framework Type Declaration
    fn framework_type(&self) -> FrameworkType;

    /// Returns current adapter state.
    /// Sec 5.2: State Query
    fn current_state(&self) -> AdapterState;

    /// Loads and validates framework agent configuration.
    /// Sec 5.2: load_agent() Method
    ///
    /// Transitions: Initialized → AgentLoaded
    ///
    /// # Arguments
    /// * `config` - Framework agent configuration to load
    ///
    /// # Returns
    /// Success with agent identifier, or error
    fn load_agent(&mut self, config: FrameworkAgentConfig) -> AdapterResult<String>;

    /// Translates framework chain/plan to cognitive task DAG.
    /// Sec 5.2: translate_chain() Method
    ///
    /// Transitions: AgentLoaded → PlanTranslated
    ///
    /// # Arguments
    /// * `chain` - Framework chain/plan definition
    ///
    /// # Returns
    /// Success with DAG identifier, or error
    fn translate_chain(&mut self, chain: FrameworkChainDefinition) -> AdapterResult<String>;

    /// Spawns cognitive tasks on kernel based on translated DAG.
    /// Sec 5.2: spawn_tasks() Method
    ///
    /// Transitions: PlanTranslated → TasksSpawned
    ///
    /// # Arguments
    /// * `dag_id` - Identifier of the translated DAG
    ///
    /// # Returns
    /// Success with task identifiers, or error
    fn spawn_tasks(&mut self, dag_id: &str) -> AdapterResult<Vec<String>>;

    /// Collects results from spawned cognitive tasks.
    /// Sec 5.2: collect_results() Method
    ///
    /// Transitions: TasksSpawned → ResultsCollected
    ///
    /// # Arguments
    /// * `task_ids` - Identifiers of tasks to collect results from
    ///
    /// # Returns
    /// Success with result items, or error
    fn collect_results(&mut self, task_ids: &[String]) -> AdapterResult<Vec<FrameworkResultItem>>;

    /// Handles errors from kernel or framework operations.
    /// Sec 5.2: on_error() Method
    ///
    /// Can occur in any state. Does not change state unless transition to Failed.
    ///
    /// # Arguments
    /// * `error` - Error information
    ///
    /// # Returns
    /// Recommended recovery action, or error if unrecoverable
    fn on_error(&mut self, error: AdapterErrorInfo) -> AdapterResult<ErrorRecoveryAction>;

    /// Resets adapter to initial state.
    /// Sec 5.2: Reset Method
    ///
    /// Clears all internal state and returns to Initialized.
    fn reset(&mut self) -> AdapterResult<()>;
}

/// Builder for creating adapter instances with validated configuration.
/// Sec 5.2: Adapter Builder Pattern
#[derive(Debug, Clone)]
pub struct AdapterBuilder {
    framework_type: Option<FrameworkType>,
    config_options: Vec<(String, String)>,
}

impl AdapterBuilder {
    /// Creates a new adapter builder.
    pub fn new() -> Self {
        AdapterBuilder {
            framework_type: None,
            config_options: Vec::new(),
        }
    }

    /// Sets the framework type.
    pub fn framework_type(mut self, ft: FrameworkType) -> Self {
        self.framework_type = Some(ft);
        self
    }

    /// Adds a configuration option.
    pub fn with_option(mut self, key: String, value: String) -> Self {
        self.config_options.push((key, value));
        self
    }

    /// Builds and validates the configuration.
    pub fn build(&self) -> AdapterResult<AdapterConfig> {
        let framework_type = self
            .framework_type
            .ok_or_else(|| AdapterError::ConfigurationError("Framework type not specified".into()))?;

        Ok(AdapterConfig {
            framework_type,
            options: self.config_options.clone(),
            validated: true,
        })
    }
}

impl Default for AdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Validated adapter configuration.
/// Sec 5.2: Adapter Configuration
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Framework type
    pub framework_type: FrameworkType,
    /// Configuration options
    pub options: Vec<(String, String)>,
    /// Whether configuration has been validated
    pub validated: bool,
}

impl AdapterConfig {
    /// Validates configuration correctness.
    pub fn validate(&self) -> AdapterResult<()> {
        if !self.validated {
            return Err(AdapterError::ConfigurationError(
                "Configuration not validated".into(),
            ));
        }
        Ok(())
    }

    /// Gets configuration option by key.
    pub fn get_option(&self, key: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_adapter_state_as_str() {
        assert_eq!(AdapterState::Initialized.as_str(), "initialized");
        assert_eq!(AdapterState::AgentLoaded.as_str(), "agent_loaded");
        assert_eq!(AdapterState::PlanTranslated.as_str(), "plan_translated");
        assert_eq!(AdapterState::TasksSpawned.as_str(), "tasks_spawned");
        assert_eq!(AdapterState::ResultsCollected.as_str(), "results_collected");
        assert_eq!(AdapterState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_adapter_state_is_terminal() {
        assert!(!AdapterState::Initialized.is_terminal());
        assert!(!AdapterState::AgentLoaded.is_terminal());
        assert!(!AdapterState::PlanTranslated.is_terminal());
        assert!(!AdapterState::TasksSpawned.is_terminal());
        assert!(AdapterState::ResultsCollected.is_terminal());
        assert!(AdapterState::Failed.is_terminal());
    }

    #[test]
    fn test_error_recovery_action_as_str() {
        assert_eq!(ErrorRecoveryAction::Retry.as_str(), "retry");
        assert_eq!(ErrorRecoveryAction::Skip.as_str(), "skip");
        assert_eq!(ErrorRecoveryAction::Escalate.as_str(), "escalate");
        assert_eq!(ErrorRecoveryAction::Abort.as_str(), "abort");
    }

    #[test]
    fn test_framework_agent_config_creation() {
        let config = FrameworkAgentConfig {
            agent_id: "agent-001".into(),
            name: "TestAgent".into(),
            description: "Test agent".into(),
            system_prompt: "You are helpful".into(),
            memory_type: "conversation".into(),
            memory_capacity_tokens: 100000,
            tool_ids: vec!["tool-1".into(), "tool-2".into()],
            timeout_ms: 30000,
            is_mandatory: true,
        };
        assert_eq!(config.agent_id, "agent-001");
        assert_eq!(config.tool_ids.len(), 2);
        assert!(config.is_mandatory);
    }

    #[test]
    fn test_framework_chain_definition_creation() {
        let chain = FrameworkChainDefinition {
            chain_id: "chain-001".into(),
            name: "TestChain".into(),
            chain_type: "sequential".into(),
            steps: vec![],
            timeout_ms: 60000,
        };
        assert_eq!(chain.chain_id, "chain-001");
        assert_eq!(chain.chain_type, "sequential");
    }

    #[test]
    fn test_chain_step_definition_creation() {
        let step = ChainStepDefinition {
            step_id: "step-001".into(),
            name: "ProcessData".into(),
            action: "process".into(),
            input_schema: "{}".into(),
            output_schema: "{}".into(),
            timeout_ms: 5000,
            depends_on: vec![],
        };
        assert_eq!(step.step_id, "step-001");
        assert_eq!(step.timeout_ms, 5000);
    }

    #[test]
    fn test_framework_result_item_creation() {
        let result = FrameworkResultItem {
            result_id: "result-001".into(),
            source_id: "task-001".into(),
            data: "{}".into(),
            status: "success".into(),
            output: "Operation succeeded".into(),
            timestamp_ms: 1234567890,
        };
        assert_eq!(result.result_id, "result-001");
        assert_eq!(result.status, "success");
    }

    #[test]
    fn test_adapter_error_info_creation() {
        let error = AdapterErrorInfo {
            error_id: "err-001".into(),
            error_type: "TimeoutError".into(),
            message: "Operation timed out".into(),
            source_id: "task-001".into(),
            diagnostic_info: "Exceeded 30000ms".into(),
            timestamp_ms: 1234567890,
            is_recoverable: true,
        };
        assert_eq!(error.error_id, "err-001");
        assert!(error.is_recoverable);
    }

    #[test]
    fn test_adapter_builder_success() {
        let config = AdapterBuilder::new()
            .framework_type(FrameworkType::LangChain)
            .with_option("timeout".into(), "30000".into())
            .build();

        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.framework_type, FrameworkType::LangChain);
        assert_eq!(cfg.get_option("timeout"), Some("30000"));
    }

    #[test]
    fn test_adapter_builder_missing_framework() {
        let config = AdapterBuilder::new()
            .with_option("timeout".into(), "30000".into())
            .build();

        assert!(config.is_err());
        match config {
            Err(AdapterError::ConfigurationError(msg)) => {
                assert!(msg.contains("Framework type"));
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }

    #[test]
    fn test_adapter_config_validation() {
        let config = AdapterConfig {
            framework_type: FrameworkType::LangChain,
            options: vec![],
            validated: true,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_adapter_config_get_option() {
        let config = AdapterConfig {
            framework_type: FrameworkType::LangChain,
            options: vec![
                ("key1".into(), "value1".into()),
                ("key2".into(), "value2".into()),
            ],
            validated: true,
        };
        assert_eq!(config.get_option("key1"), Some("value1"));
        assert_eq!(config.get_option("key3"), None);
    }
}
