// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent start operation - Unit file loading, parsing, and CT spawn integration.
//!
//! Implements the complete agent startup sequence: loading unit file, parsing configuration,
//! validating constraints, extracting config, translating to CT spawn parameters, spawning
//! the CT process, and registering the agent.
//!
//! Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation

use crate::lifecycle_manager::LifecycleManager;
use crate::unit_file::AgentUnitFile;
use crate::{LifecycleError, LifecycleState, Result};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Parameters for starting an agent.
///
/// Encapsulates all information needed to start an agent, including the unit file
/// and timing information.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
#[derive(Debug, Clone)]
pub struct AgentStartParams {
    /// Unique agent identifier.
    pub agent_id: String,

    /// Agent unit file configuration.
    pub unit_file: AgentUnitFile,

    /// Current time in milliseconds since Unix epoch.
    pub current_time_ms: u64,

    /// Startup timeout in milliseconds.
    pub startup_timeout_ms: u64,
}

impl AgentStartParams {
    /// Creates new agent start parameters.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Unique identifier for the agent
    /// - `unit_file`: Agent unit file configuration
    /// - `current_time_ms`: Current time in milliseconds
    /// - `startup_timeout_ms`: Maximum time allowed for startup
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
    pub fn new(
        agent_id: impl Into<String>,
        unit_file: AgentUnitFile,
        current_time_ms: u64,
        startup_timeout_ms: u64,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            unit_file,
            current_time_ms,
            startup_timeout_ms,
        }
    }
}

/// Result of CT spawn operation.
///
/// Contains the process ID and resource information for a spawned CT.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
#[derive(Debug, Clone)]
pub struct CtSpawnResult {
    /// Platform-specific process identifier.
    pub process_id: u64,

    /// Memory limit in bytes if specified.
    pub memory_limit_bytes: Option<u64>,

    /// CPU cores limit if specified.
    pub cpu_cores_limit: Option<f32>,
}

impl CtSpawnResult {
    /// Creates a new CT spawn result.
    ///
    /// # Arguments
    ///
    /// - `process_id`: Platform process ID
    /// - `memory_limit_bytes`: Memory limit in bytes
    /// - `cpu_cores_limit`: CPU cores limit
    pub fn new(process_id: u64, memory_limit_bytes: Option<u64>, cpu_cores_limit: Option<f32>) -> Self {
        Self {
            process_id,
            memory_limit_bytes,
            cpu_cores_limit,
        }
    }
}

/// Agent start operation handler.
///
/// Implements the full startup sequence for agents: validation, configuration extraction,
/// CT spawn translation, process spawn, and lifecycle registration.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
pub struct AgentStartHandler;

impl AgentStartHandler {
    /// Validates agent unit file for startup.
    ///
    /// Checks required fields and resource constraints before attempting to start.
    ///
    /// # Arguments
    ///
    /// - `unit_file`: Agent unit file to validate
    ///
    /// # Returns
    ///
    /// - `Ok(())` if unit file is valid for startup
    /// - `Err(LifecycleError::...)` if validation fails
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
    pub fn validate_unit_file(unit_file: &AgentUnitFile) -> Result<()> {
        // Check agent has a name
        if unit_file.metadata.name.is_empty() {
            return Err(LifecycleError::LifecycleError(
                "Agent name cannot be empty".to_string(),
            ));
        }

        // Check agent has a version
        if unit_file.metadata.version.is_empty() {
            return Err(LifecycleError::LifecycleError(
                "Agent version cannot be empty".to_string(),
            ));
        }

        // Validate memory limits if specified
        if let Some(memory) = unit_file.memory_mb {
            if memory == 0 {
                return Err(LifecycleError::LifecycleError(
                    "Memory limit must be greater than 0".to_string(),
                ));
            }
        }

        // Validate CPU limits if specified
        if let Some(cpu) = unit_file.cpu_cores {
            if cpu <= 0.0 {
                return Err(LifecycleError::LifecycleError(
                    "CPU cores limit must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Extracts configuration from unit file.
    ///
    /// Transforms the agent unit file into startup configuration, including
    /// environment variables, resource limits, and dependencies.
    ///
    /// # Arguments
    ///
    /// - `unit_file`: Agent unit file
    ///
    /// # Returns
    ///
    /// Extracted configuration as a map for CT spawn translation.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
    pub fn extract_config(unit_file: &AgentUnitFile) -> alloc::collections::BTreeMap<String, String> {
        let mut config = alloc::collections::BTreeMap::new();

        // Extract metadata
        config.insert("agent_id".to_string(), unit_file.metadata.name.clone());
        config.insert("agent_version".to_string(), unit_file.metadata.version.clone());
        config.insert(
            "agent_description".to_string(),
            unit_file.metadata.description.clone(),
        );

        // Extract resource limits
        if let Some(memory) = unit_file.memory_mb {
            config.insert("memory_mb".to_string(), memory.to_string());
        }

        if let Some(cpu) = unit_file.cpu_cores {
            config.insert("cpu_cores".to_string(), cpu.to_string());
        }

        // Extract environment variables
        for (key, value) in &unit_file.environment {
            config.insert(format!("env_{}", key), value.clone());
        }

        // Extract model config if present
        if let Some(model) = &unit_file.model_config {
            if let Some(provider) = &model.provider {
                config.insert("model_provider".to_string(), provider.clone());
            }
            if let Some(model_name) = &model.model_name {
                config.insert("model_name".to_string(), model_name.clone());
            }
        }

        config
    }

    /// Translates agent config to CT spawn parameters.
    ///
    /// Converts agent configuration into kernel CT spawn parameters, mapping
    /// resource limits and configuration to CT capabilities.
    ///
    /// # Arguments
    ///
    /// - `config`: Extracted agent configuration
    /// - `unit_file`: Original unit file for additional metadata
    ///
    /// # Returns
    ///
    /// CT spawn parameters ready for kernel invocation.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
    pub fn translate_to_ct_spawn_params(
        config: &alloc::collections::BTreeMap<String, String>,
        unit_file: &AgentUnitFile,
    ) -> alloc::string::String {
        let mut params = alloc::string::String::new();

        // Add agent identification
        if let Some(agent_id) = config.get("agent_id") {
            params.push_str("id=");
            params.push_str(agent_id);
            params.push_str(" ");
        }

        // Add resource limits
        if let Some(memory) = config.get("memory_mb") {
            params.push_str("memory_mb=");
            params.push_str(memory);
            params.push_str(" ");
        }

        if let Some(cpu) = config.get("cpu_cores") {
            params.push_str("cpu_cores=");
            params.push_str(cpu);
            params.push_str(" ");
        }

        // Add capabilities
        for cap in &unit_file.capabilities_required {
            params.push_str("capability=");
            params.push_str(cap);
            params.push_str(" ");
        }

        params
    }

    /// Spawns CT process for agent.
    ///
    /// Simulates CT process spawn with resource quota enforcement. In real implementation,
    /// this would invoke kernel CT spawn mechanisms.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `spawn_params`: CT spawn parameters
    /// - `unit_file`: Agent unit file
    ///
    /// # Returns
    ///
    /// - `Ok(CtSpawnResult)` with process ID and resource info
    /// - `Err(LifecycleError::...)` if spawn failed
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
    pub fn spawn_ct_process(
        agent_id: &str,
        _spawn_params: &str,
        unit_file: &AgentUnitFile,
    ) -> Result<CtSpawnResult> {
        // Validate agent_id is not empty
        if agent_id.is_empty() {
            return Err(LifecycleError::LifecycleError(
                "Cannot spawn CT with empty agent_id".to_string(),
            ));
        }

        // In a real implementation, this would call kernel CT spawn mechanisms
        // For now, we simulate with a deterministic but unique process ID
        let process_id = Self::generate_process_id(agent_id);

        Ok(CtSpawnResult::new(
            process_id,
            unit_file.memory_mb.map(|mb| mb as u64 * 1024 * 1024),
            unit_file.cpu_cores,
        ))
    }

    /// Generates a process ID from agent ID.
    ///
    /// Deterministic process ID generation for testing. In production, the kernel
    /// would assign actual process IDs.
    fn generate_process_id(agent_id: &str) -> u64 {
        // Simple hash of agent_id for deterministic but unique ID
        let mut hash = 5381u64;
        for byte in agent_id.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as u64);
        }
        // Ensure it's a valid process ID (> 0)
        (hash & 0x7FFFFFFF) + 1000
    }

    /// Performs complete agent start operation.
    ///
    /// Orchestrates the full startup sequence: validation, configuration extraction,
    /// CT spawn, and lifecycle registration.
    ///
    /// # Arguments
    ///
    /// - `lifecycle_manager`: Lifecycle manager for registration
    /// - `params`: Agent start parameters
    ///
    /// # Returns
    ///
    /// - `Ok(CtSpawnResult)` on successful startup
    /// - `Err(LifecycleError::...)` if any step fails
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Start Operation
    pub fn start_agent(
        lifecycle_manager: &crate::lifecycle_manager::LifecycleManager,
        params: AgentStartParams,
    ) -> Result<CtSpawnResult> {
        // Step 1: Validate unit file
        Self::validate_unit_file(&params.unit_file)?;

        // Step 2: Register agent in Initializing state
        lifecycle_manager.register_agent(
            params.agent_id.clone(),
            params.unit_file.clone(),
            params.current_time_ms,
        )?;

        // Step 3: Transition to Loading state (simulated as Starting)
        lifecycle_manager.transition_agent(&params.agent_id, LifecycleState::Starting, params.current_time_ms)?;

        // Step 4: Extract configuration
        let config = Self::extract_config(&params.unit_file);

        // Step 5: Translate to CT spawn parameters
        let ct_params = Self::translate_to_ct_spawn_params(&config, &params.unit_file);

        // Step 6: Spawn CT process
        let spawn_result = Self::spawn_ct_process(&params.agent_id, &ct_params, &params.unit_file)?;

        // Step 7: Update lifecycle manager with process ID
        lifecycle_manager.set_agent_ct_process_id(&params.agent_id, spawn_result.process_id)?;

        // Step 8: Transition to Running state
        lifecycle_manager.transition_agent(&params.agent_id, LifecycleState::Running, params.current_time_ms)?;

        Ok(spawn_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_file::AgentUnitFile;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    fn create_test_unit_file() -> AgentUnitFile {
        AgentUnitFile::new("test-agent", "1.0.0", "Test agent")
    }

    #[test]
    fn test_agent_start_params_creation() {
        let unit_file = create_test_unit_file();
        let params = AgentStartParams::new("agent-1", unit_file, 1000, 30000);

        assert_eq!(params.agent_id, "agent-1");
        assert_eq!(params.current_time_ms, 1000);
        assert_eq!(params.startup_timeout_ms, 30000);
    }

    #[test]
    fn test_ct_spawn_result_creation() {
        let result = CtSpawnResult::new(12345, Some(1024 * 1024 * 512), Some(2.0));

        assert_eq!(result.process_id, 12345);
        assert_eq!(result.memory_limit_bytes, Some(1024 * 1024 * 512));
        assert_eq!(result.cpu_cores_limit, Some(2.0));
    }

    #[test]
    fn test_validate_unit_file_valid() {
        let unit_file = create_test_unit_file();
        assert!(AgentStartHandler::validate_unit_file(&unit_file).is_ok());
    }

    #[test]
    fn test_validate_unit_file_empty_name() {
        let mut unit_file = create_test_unit_file();
        unit_file.metadata.name = String::new();
        assert!(AgentStartHandler::validate_unit_file(&unit_file).is_err());
    }

    #[test]
    fn test_validate_unit_file_empty_version() {
        let mut unit_file = create_test_unit_file();
        unit_file.metadata.version = String::new();
        assert!(AgentStartHandler::validate_unit_file(&unit_file).is_err());
    }

    #[test]
    fn test_validate_unit_file_zero_memory() {
        let mut unit_file = create_test_unit_file();
        unit_file.memory_mb = Some(0);
        assert!(AgentStartHandler::validate_unit_file(&unit_file).is_err());
    }

    #[test]
    fn test_validate_unit_file_zero_cpu() {
        let mut unit_file = create_test_unit_file();
        unit_file.cpu_cores = Some(0.0);
        assert!(AgentStartHandler::validate_unit_file(&unit_file).is_err());
    }

    #[test]
    fn test_extract_config_basic() {
        let unit_file = AgentUnitFile::new("test", "1.0.0", "Test");
        let config = AgentStartHandler::extract_config(&unit_file);

        assert_eq!(config.get("agent_id"), Some(&"test".to_string()));
        assert_eq!(config.get("agent_version"), Some(&"1.0.0".to_string()));
    }

    #[test]
    fn test_extract_config_with_resources() {
        let unit_file = AgentUnitFile::new("test", "1.0.0", "Test")
            .with_memory_mb(512)
            .with_cpu_cores(2.0);

        let config = AgentStartHandler::extract_config(&unit_file);

        assert_eq!(config.get("memory_mb"), Some(&"512".to_string()));
        assert_eq!(config.get("cpu_cores"), Some(&"2".to_string()));
    }

    #[test]
    fn test_extract_config_with_environment() {
        let unit_file = AgentUnitFile::new("test", "1.0.0", "Test")
            .with_env("PORT", "8080")
            .with_env("LOG_LEVEL", "info");

        let config = AgentStartHandler::extract_config(&unit_file);

        assert_eq!(config.get("env_PORT"), Some(&"8080".to_string()));
        assert_eq!(config.get("env_LOG_LEVEL"), Some(&"info".to_string()));
    }

    #[test]
    fn test_translate_to_ct_spawn_params() {
        let unit_file = AgentUnitFile::new("test", "1.0.0", "Test")
            .with_memory_mb(512)
            .with_cpu_cores(2.0);

        let config = AgentStartHandler::extract_config(&unit_file);
        let params = AgentStartHandler::translate_to_ct_spawn_params(&config, &unit_file);

        assert!(params.contains("id=test"));
        assert!(params.contains("memory_mb=512"));
        assert!(params.contains("cpu_cores=2"));
    }

    #[test]
    fn test_spawn_ct_process_valid() {
        let unit_file = create_test_unit_file();
        let result = AgentStartHandler::spawn_ct_process("agent-1", "test_params", &unit_file);

        assert!(result.is_ok());
        let spawn_result = result.unwrap();
        assert!(spawn_result.process_id > 0);
    }

    #[test]
    fn test_spawn_ct_process_empty_agent_id() {
        let unit_file = create_test_unit_file();
        let result = AgentStartHandler::spawn_ct_process("", "test_params", &unit_file);

        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_ct_process_deterministic_pid() {
        let unit_file = create_test_unit_file();

        let result1 = AgentStartHandler::spawn_ct_process("agent-1", "params", &unit_file);
        let result2 = AgentStartHandler::spawn_ct_process("agent-1", "params", &unit_file);

        assert_eq!(result1.unwrap().process_id, result2.unwrap().process_id);
    }

    #[test]
    fn test_start_agent_success() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();
        let params = AgentStartParams::new("agent-1", unit_file, 1000, 30000);

        let result = AgentStartHandler::start_agent(&manager, params);
        assert!(result.is_ok());

        // Verify agent is in Running state
        assert_eq!(
            manager.get_agent_state("agent-1").unwrap(),
            LifecycleState::Running
        );

        // Verify CT process ID is set
        let info = manager.get_agent_info("agent-1").unwrap();
        assert!(info.ct_process_id.is_some());
    }

    #[test]
    fn test_start_agent_validation_failure() {
        let manager = LifecycleManager::new();
        let mut unit_file = create_test_unit_file();
        unit_file.metadata.name = String::new();

        let params = AgentStartParams::new("agent-1", unit_file, 1000, 30000);
        let result = AgentStartHandler::start_agent(&manager, params);

        assert!(result.is_err());
        // Agent should not be registered on validation failure
        assert_eq!(manager.total_agents(), 0);
    }
}
