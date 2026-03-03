// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Kernel CT spawn integration - Agent config to CT spawn parameter translation.
//!
//! Provides translation layer between agent unit file configuration and kernel
//! CT (Capability Thread) spawn parameters. Enforces resource quotas and translates
//! agent-level constraints to CT-level capabilities and limits.
//!
//! Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration

use crate::unit_file::AgentUnitFile;
use crate::{LifecycleError, Result};
use std::collections::BTreeMap;

/// Resource quota enforcement policy.
///
/// Determines how resource limits are enforced when creating CT processes.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaPolicy {
    /// Strictly enforce all limits, fail if exceeded.
    Strict,

    /// Warn on limit violations but proceed.
    Warn,

    /// Allow limits to be exceeded (for trusted agents).
    Permissive,
}

/// CT spawn parameters for kernel invocation.
///
/// Represents the parameters passed to the kernel CT spawn mechanism.
/// Maps agent configuration to CT-level capabilities and constraints.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
#[derive(Debug, Clone)]
pub struct CtSpawnParams {
    /// Agent identifier.
    pub agent_id: String,

    /// Memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,

    /// CPU cores limit.
    pub cpu_cores_limit: Option<f32>,

    /// List of capabilities to grant.
    pub capabilities: Vec<String>,

    /// Environment variables to set.
    pub environment: BTreeMap<String, String>,

    /// Custom CT parameters.
    pub custom_params: BTreeMap<String, String>,
}

impl CtSpawnParams {
    /// Creates new CT spawn parameters.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            memory_limit_bytes: None,
            cpu_cores_limit: None,
            capabilities: Vec::new(),
            environment: BTreeMap::new(),
            custom_params: BTreeMap::new(),
        }
    }

    /// Sets memory limit in bytes.
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit_bytes = Some(bytes);
        self
    }

    /// Sets CPU cores limit.
    pub fn with_cpu_limit(mut self, cores: f32) -> Self {
        self.cpu_cores_limit = Some(cores);
        self
    }

    /// Adds a capability.
    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    /// Adds multiple capabilities.
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities.extend(caps);
        self
    }

    /// Sets environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Adds custom parameter.
    pub fn with_custom_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_params.insert(key.into(), value.into());
        self
    }

    /// Validates CT spawn parameters.
    ///
    /// Ensures parameters are valid for CT spawn invocation.
    pub fn validate(&self) -> Result<()> {
        // Agent ID must not be empty
        if self.agent_id.is_empty() {
            return Err(LifecycleError::GenericError(
                "Agent ID cannot be empty".to_string(),
            ));
        }

        // Memory limit must be positive if set
        if let Some(mem) = self.memory_limit_bytes {
            if mem == 0 {
                return Err(LifecycleError::GenericError(
                    "Memory limit must be greater than 0".to_string(),
                ));
            }
        }

        // CPU limit must be positive if set
        if let Some(cpu) = self.cpu_cores_limit {
            if cpu <= 0.0 {
                return Err(LifecycleError::GenericError(
                    "CPU limit must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Serializes CT spawn parameters to command-line format.
    ///
    /// Produces a string suitable for passing to kernel CT spawn mechanism.
    pub fn serialize(&self) -> String {
        let mut result = String::new();

        result.push_str("id=");
        result.push_str(&self.agent_id);
        result.push_str(" ");

        if let Some(mem) = self.memory_limit_bytes {
            result.push_str("memory_limit=");
            result.push_str(&mem.to_string());
            result.push_str(" ");
        }

        if let Some(cpu) = self.cpu_cores_limit {
            result.push_str("cpu_limit=");
            result.push_str(&cpu.to_string());
            result.push_str(" ");
        }

        for cap in &self.capabilities {
            result.push_str("cap=");
            result.push_str(cap);
            result.push_str(" ");
        }

        for (key, value) in &self.environment {
            result.push_str("env_");
            result.push_str(key);
            result.push_str("=");
            result.push_str(value);
            result.push_str(" ");
        }

        result
    }
}

/// CT spawn integration translator.
///
/// Translates agent unit file configuration into CT spawn parameters,
/// enforcing resource quotas and constraint mappings.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
pub struct CtSpawnTranslator;

impl CtSpawnTranslator {
    /// Validates resource quotas against system limits.
    ///
    /// Checks that requested resources don't exceed system maximums.
    ///
    /// # Arguments
    ///
    /// - `memory_mb`: Requested memory in MB
    /// - `cpu_cores`: Requested CPU cores
    /// - `policy`: Quota enforcement policy
    ///
    /// # Returns
    ///
    /// - `Ok(())` if quotas are acceptable
    /// - `Err(LifecycleError::...)` if quotas exceed limits and policy is Strict
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
    pub fn validate_resource_quotas(
        memory_mb: Option<u32>,
        cpu_cores: Option<f32>,
        policy: QuotaPolicy,
    ) -> Result<()> {
        // System limits (example values)
        const MAX_MEMORY_MB: u32 = 65536;
        const MAX_CPU_CORES: f32 = 256.0;

        if let Some(mem) = memory_mb {
            if mem > MAX_MEMORY_MB && policy == QuotaPolicy::Strict {
                return Err(LifecycleError::GenericError(format!(
                    "Memory {} MB exceeds system limit {} MB",
                    mem, MAX_MEMORY_MB
                )));
            }
        }

        if let Some(cpu) = cpu_cores {
            if cpu > MAX_CPU_CORES && policy == QuotaPolicy::Strict {
                return Err(LifecycleError::GenericError(format!(
                    "CPU {} cores exceeds system limit {} cores",
                    cpu, MAX_CPU_CORES
                )));
            }
        }

        Ok(())
    }

    /// Translates agent unit file to CT spawn parameters.
    ///
    /// Converts agent configuration into CT spawn parameters, including
    /// resource limits, capabilities, and environment setup.
    ///
    /// # Arguments
    ///
    /// - `unit_file`: Agent unit file configuration
    /// - `policy`: Resource quota enforcement policy
    ///
    /// # Returns
    ///
    /// - `Ok(CtSpawnParams)` on successful translation
    /// - `Err(LifecycleError::...)` if translation fails
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § CT Spawn Integration
    pub fn translate(unit_file: &AgentUnitFile, policy: QuotaPolicy) -> Result<CtSpawnParams> {
        // Validate resource quotas first
        Self::validate_resource_quotas(
            unit_file.memory_mb.map(|m| m as u32),
            unit_file.cpu_cores.map(|c| c as f32),
            policy,
        )?;

        let mut params = CtSpawnParams::new(unit_file.metadata.name.clone());

        // Translate memory limit
        if let Some(memory_mb) = unit_file.memory_mb {
            params = params.with_memory_limit(memory_mb as u64 * 1024 * 1024);
        }

        // Translate CPU limit
        if let Some(cpu_cores) = unit_file.cpu_cores {
            params = params.with_cpu_limit(cpu_cores as f32);
        }

        // Translate capabilities
        for cap in &unit_file.capabilities_required {
            params = params.with_capability(cap.clone());
        }

        // Translate environment variables
        for (key, value) in &unit_file.environment {
            params = params.with_env(key.clone(), value.clone());
        }

        // Validate translated parameters
        params.validate()?;

        Ok(params)
    }

    /// Maps agent tags to CT capabilities.
    ///
    /// Translates agent classification tags to CT security capabilities.
    ///
    /// # Arguments
    ///
    /// - `tags`: Agent tags
    ///
    /// # Returns
    ///
    /// Vector of CT capabilities to grant.
    pub fn map_tags_to_capabilities(tags: &[String]) -> Vec<String> {
        let mut capabilities = Vec::new();

        for tag in tags {
            match tag.as_str() {
                "network" => capabilities.push("net_bind_service".to_string()),
                "privileged" => {
                    capabilities.push("cap_sys_admin".to_string());
                    capabilities.push("cap_net_admin".to_string());
                }
                "database" => {
                    capabilities.push("cap_net_bind_service".to_string());
                }
                "stateless" => {
                    // No special capabilities needed
                }
                _ => {
                    // Unknown tag, no mapping
                }
            }
        }

        capabilities
    }

    /// Calculates effective CT resource limits from agent config.
    ///
    /// Determines final resource limits considering agent requests and
    /// system defaults.
    ///
    /// # Arguments
    ///
    /// - `memory_mb`: Requested memory in MB
    /// - `cpu_cores`: Requested CPU cores
    ///
    /// # Returns
    ///
    /// Tuple of (memory_bytes, cpu_cores) for CT spawn.
    pub fn calculate_resource_limits(memory_mb: Option<u32>, cpu_cores: Option<f32>) -> (Option<u64>, Option<f32>) {
        let memory_bytes = memory_mb.map(|mb| mb as u64 * 1024 * 1024);
        (memory_bytes, cpu_cores)
    }

    /// Validates CT spawn parameter consistency.
    ///
    /// Ensures CT parameters form a consistent, valid configuration
    /// for kernel invocation.
    ///
    /// # Arguments
    ///
    /// - `params`: CT spawn parameters to validate
    ///
    /// # Returns
    ///
    /// - `Ok(())` if parameters are valid
    /// - `Err(LifecycleError::...)` if validation fails
    pub fn validate_ct_params(params: &CtSpawnParams) -> Result<()> {
        params.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_unit_file() -> AgentUnitFile {
        AgentUnitFile::new("test-agent", "1.0.0", "Test agent")
    }

    #[test]
    fn test_ct_spawn_params_creation() {
        let params = CtSpawnParams::new("agent-1");

        assert_eq!(params.agent_id, "agent-1");
        assert!(params.memory_limit_bytes.is_none());
        assert!(params.cpu_cores_limit.is_none());
        assert!(params.capabilities.is_empty());
    }

    #[test]
    fn test_ct_spawn_params_builder() {
        let params = CtSpawnParams::new("agent-1")
            .with_memory_limit(1024 * 1024 * 512)
            .with_cpu_limit(2.0)
            .with_capability("net_bind_service");

        assert_eq!(params.memory_limit_bytes, Some(1024 * 1024 * 512));
        assert_eq!(params.cpu_cores_limit, Some(2.0));
        assert_eq!(params.capabilities.len(), 1);
    }

    #[test]
    fn test_ct_spawn_params_validate_valid() {
        let params = CtSpawnParams::new("agent-1")
            .with_memory_limit(1024 * 1024)
            .with_cpu_limit(1.0);

        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_ct_spawn_params_validate_empty_id() {
        let params = CtSpawnParams::new("");
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_ct_spawn_params_validate_zero_memory() {
        let mut params = CtSpawnParams::new("agent-1");
        params.memory_limit_bytes = Some(0);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_ct_spawn_params_validate_zero_cpu() {
        let mut params = CtSpawnParams::new("agent-1");
        params.cpu_cores_limit = Some(0.0);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_ct_spawn_params_serialize() {
        let params = CtSpawnParams::new("agent-1")
            .with_memory_limit(1024 * 1024 * 512)
            .with_cpu_limit(2.0)
            .with_capability("net_bind_service");

        let serialized = params.serialize();

        assert!(serialized.contains("id=agent-1"));
        assert!(serialized.contains("memory_limit="));
        assert!(serialized.contains("cpu_limit=2"));
        assert!(serialized.contains("cap=net_bind_service"));
    }

    #[test]
    fn test_validate_resource_quotas_strict_memory_exceeded() {
        let result = CtSpawnTranslator::validate_resource_quotas(
            Some(65537),
            None,
            QuotaPolicy::Strict,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_resource_quotas_strict_cpu_exceeded() {
        let result = CtSpawnTranslator::validate_resource_quotas(
            None,
            Some(257.0),
            QuotaPolicy::Strict,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_resource_quotas_permissive_exceeded() {
        let result = CtSpawnTranslator::validate_resource_quotas(
            Some(65537),
            Some(257.0),
            QuotaPolicy::Permissive,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_translate_basic() {
        let unit_file = create_test_unit_file();
        let params = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);

        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.agent_id, "test-agent");
    }

    #[test]
    fn test_translate_with_resources() {
        let unit_file = create_test_unit_file()
            .with_memory_mb(512)
            .with_cpu_cores(2.0);

        let params = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);

        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.memory_limit_bytes, Some(512 * 1024 * 1024));
        assert_eq!(p.cpu_cores_limit, Some(2.0));
    }

    #[test]
    fn test_translate_with_capabilities() {
        let unit_file = create_test_unit_file()
            .with_capability("net_admin")
            .with_capability("sys_resource");

        let params = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);

        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.capabilities.len(), 2);
        assert!(p.capabilities.contains(&"net_admin".to_string()));
    }

    #[test]
    fn test_translate_with_environment() {
        let unit_file = create_test_unit_file()
            .with_env("PORT", "8080")
            .with_env("LOG_LEVEL", "info");

        let params = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);

        assert!(params.is_ok());
        let p = params.unwrap();
        assert_eq!(p.environment.get("PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn test_translate_exceeds_quota() {
        let unit_file = create_test_unit_file()
            .with_memory_mb(65537);

        let params = CtSpawnTranslator::translate(&unit_file, QuotaPolicy::Strict);

        assert!(params.is_err());
    }

    #[test]
    fn test_map_tags_to_capabilities_network() {
        let tags = vec!["network".to_string()];
        let caps = CtSpawnTranslator::map_tags_to_capabilities(&tags);

        assert!(caps.contains(&"net_bind_service".to_string()));
    }

    #[test]
    fn test_map_tags_to_capabilities_privileged() {
        let tags = vec!["privileged".to_string()];
        let caps = CtSpawnTranslator::map_tags_to_capabilities(&tags);

        assert!(caps.contains(&"cap_sys_admin".to_string()));
        assert!(caps.contains(&"cap_net_admin".to_string()));
    }

    #[test]
    fn test_calculate_resource_limits() {
        let (mem, cpu) = CtSpawnTranslator::calculate_resource_limits(Some(512), Some(2.0));

        assert_eq!(mem, Some(512 * 1024 * 1024));
        assert_eq!(cpu, Some(2.0));
    }

    #[test]
    fn test_calculate_resource_limits_none() {
        let (mem, cpu) = CtSpawnTranslator::calculate_resource_limits(None, None);

        assert!(mem.is_none());
        assert!(cpu.is_none());
    }

    #[test]
    fn test_validate_ct_params() {
        let params = CtSpawnParams::new("agent-1");
        assert!(CtSpawnTranslator::validate_ct_params(&params).is_ok());
    }
}
