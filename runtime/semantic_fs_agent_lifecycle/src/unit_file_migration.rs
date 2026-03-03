// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File migration utilities.
//!
//! Provides tools for migrating agent configurations from legacy formats
//! (environment-based configuration, older schema versions) to the modern
//! Agent Unit File format.
//!
//! Supports:
//! - Environment variable mapping to unit file sections
//! - Legacy field conversion (memory_mb → max_memory_bytes)
//! - Schema version upgrades
//! - Compatibility mapping
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration

use crate::AgentUnitFile;
use std::collections::BTreeMap;

/// Migration result type.
pub type MigrationResult<T> = core::result::Result<T, MigrationError>;

/// Migration error type.
///
/// Represents errors that occur during agent configuration migration.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration
#[derive(Debug, Clone)]
pub enum MigrationError {
    /// Source configuration is invalid.
    InvalidSource {
        /// Description of what's invalid.
        reason: String,
    },

    /// Required field is missing from source.
    MissingField {
        /// Name of missing field.
        field: String,
    },

    /// Field value cannot be converted to target format.
    ConversionFailed {
        /// Name of the field.
        field: String,
        /// Description of why conversion failed.
        reason: String,
    },

    /// Semantic violation during migration.
    SemanticError {
        /// Description of the violation.
        message: String,
    },
}

impl core::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSource { reason } => write!(f, "Invalid source: {}", reason),
            Self::MissingField { field } => write!(f, "Missing field: {}", field),
            Self::ConversionFailed { field, reason } => {
                write!(f, "Conversion failed for field '{}': {}", field, reason)
            }
            Self::SemanticError { message } => write!(f, "Semantic error: {}", message),
        }
    }
}

/// Environment-based configuration (legacy format).
///
/// Represents the old way of configuring agents using environment variables.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration
#[derive(Debug, Clone)]
pub struct LegacyEnvConfig {
    /// Raw environment variables.
    pub env: BTreeMap<String, String>,
}

impl LegacyEnvConfig {
    /// Creates a new legacy config from environment variables.
    pub fn new(env: BTreeMap<String, String>) -> Self {
        Self { env }
    }

    /// Gets a required environment variable.
    fn get_required(&self, key: &str) -> MigrationResult<String> {
        self.env
            .get(key)
            .cloned()
            .ok_or_else(|| MigrationError::MissingField {
                field: key.to_string(),
            })
    }

    /// Gets an optional environment variable.
    fn get_optional(&self, key: &str) -> Option<String> {
        self.env.get(key).cloned()
    }
}

/// Migrates legacy environment-based configuration to Agent Unit File format.
///
/// Performs semantic conversion from environment variables to structured
/// unit file configuration, including field mapping and validation.
///
/// # Environment Variable Mapping
///
/// Legacy environment variables map to unit file sections:
///
/// - AGENT_NAME → agent.name
/// - AGENT_VERSION → agent.version
/// - AGENT_DESCRIPTION → agent.description
/// - FRAMEWORK → agent.framework
/// - MODEL_PROVIDER → model.provider
/// - MODEL_NAME → model.model_name
/// - MODEL_MAX_TOKENS → model.max_tokens
/// - MEMORY_MB → resources.max_memory_bytes (converted)
/// - CPU_CORES → (deprecated, ignored)
/// - HEALTH_CHECK_TYPE → health_check.type
/// - HEALTH_CHECK_ENDPOINT → health_check.endpoint
/// - MAX_RETRIES → restart.max_retries
/// - (all other vars) → environment section
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration
#[derive(Debug)]
pub struct EnvironmentMigrator;

impl EnvironmentMigrator {
    /// Creates a new environment migrator.
    pub fn new() -> Self {
        Self
    }

    /// Migrates legacy environment-based config to Agent Unit File.
    ///
    /// # Arguments
    ///
    /// * `legacy_config` - Legacy environment-based configuration
    ///
    /// # Returns
    ///
    /// Returns an [`AgentUnitFile`] on success, or a [`MigrationError`] on failure.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration
    pub fn migrate(&self, legacy_config: &LegacyEnvConfig) -> MigrationResult<AgentUnitFile> {
        // Extract required fields
        let name = legacy_config.get_required("AGENT_NAME")?;
        let version = legacy_config.get_required("AGENT_VERSION")?;
        let description = legacy_config.get_required("AGENT_DESCRIPTION")?;

        // Create base unit file
        let mut unit_file = AgentUnitFile::new(name, version, description);

        // Migrate optional agent fields
        if let Some(framework) = legacy_config.get_optional("FRAMEWORK") {
            unit_file.metadata = unit_file.metadata.with_author(framework);
        }

        if let Some(author) = legacy_config.get_optional("AUTHOR") {
            unit_file = unit_file.with_author(author);
        }

        // Migrate model configuration
        if legacy_config.env.contains_key("MODEL_PROVIDER")
            || legacy_config.env.contains_key("MODEL_NAME")
        {
            let mut model_config = crate::ModelConfig::new();

            if let Some(provider) = legacy_config.get_optional("MODEL_PROVIDER") {
                model_config = model_config.with_provider(provider);
            }

            if let Some(model_name) = legacy_config.get_optional("MODEL_NAME") {
                model_config = model_config.with_model_name(model_name);
            }

            if let Some(max_tokens_str) = legacy_config.get_optional("MODEL_MAX_TOKENS") {
                if let Ok(tokens) = max_tokens_str.parse::<u32>() {
                    model_config = model_config.with_max_tokens(tokens);
                }
            }

            if let Some(temp_str) = legacy_config.get_optional("TEMPERATURE") {
                if let Ok(temp) = temp_str.parse::<f32>() {
                    model_config = model_config.with_temperature(temp);
                }
            }

            unit_file = unit_file.with_model_config(model_config);
        }

        // Migrate resource limits
        if legacy_config.env.contains_key("MEMORY_MB")
            || legacy_config.env.contains_key("CPU_CORES")
        {
            let mut resource_limits = crate::ResourceLimits::new();

            // Convert legacy memory_mb to max_memory_bytes
            if let Some(memory_mb_str) = legacy_config.get_optional("MEMORY_MB") {
                if let Ok(mb) = memory_mb_str.parse::<u64>() {
                    let bytes = mb * 1024 * 1024;
                    resource_limits = resource_limits.with_max_memory_bytes(bytes);
                    unit_file.memory_mb = Some(mb);
                }
            }

            // Note: CPU_CORES is deprecated and not enforced
            if let Some(_cpu_cores_str) = legacy_config.get_optional("CPU_CORES") {
                // CPU cores is deprecated - documented but not enforced
                // unit_file.cpu_cores can be set for backward compat
            }

            unit_file = unit_file.with_resource_limits(resource_limits);
        }

        // Migrate environment variables (preserve non-standard ones)
        let mut env_vars = BTreeMap::new();
        let standard_keys = [
            "AGENT_NAME",
            "AGENT_VERSION",
            "AGENT_DESCRIPTION",
            "FRAMEWORK",
            "AUTHOR",
            "MODEL_PROVIDER",
            "MODEL_NAME",
            "MODEL_MAX_TOKENS",
            "TEMPERATURE",
            "MEMORY_MB",
            "CPU_CORES",
            "HEALTH_CHECK_TYPE",
            "HEALTH_CHECK_ENDPOINT",
            "MAX_RETRIES",
        ];

        for (key, value) in &legacy_config.env {
            if !standard_keys.contains(&key.as_str()) {
                env_vars.insert(key.clone(), value.clone());
            }
        }

        unit_file = unit_file.with_environment(env_vars);

        // Migrate health check configuration
        if legacy_config.env.contains_key("HEALTH_CHECK_TYPE") {
            let check_type = legacy_config.get_optional("HEALTH_CHECK_TYPE");
            let endpoint = legacy_config.get_optional("HEALTH_CHECK_ENDPOINT")
                .unwrap_or_default();

            let check_type_enum = match check_type.as_deref() {
                Some("http") => crate::HealthCheckType::Http(endpoint.clone()),
                Some("tcp") => crate::HealthCheckType::Tcp(endpoint.parse::<u16>().unwrap_or(8080)),
                Some("exec") => crate::HealthCheckType::Exec(endpoint.clone()),
                Some("csci") => crate::HealthCheckType::CsciSyscall(endpoint.clone()),
                _ => crate::HealthCheckType::Http(endpoint.clone()),
            };

            let health_config = crate::HealthCheckConfig {
                check_type: check_type_enum,
                interval_ms: 10000,
                timeout_ms: 5000,
                failure_threshold: 3,
                success_threshold: 1,
            };

            let probe = crate::HealthProbe {
                config: health_config,
                initial_delay_ms: 0,
            };

            let mut lifecycle = crate::LifecycleConfig::default();
            lifecycle.readiness_probe = Some(probe.clone());
            lifecycle.liveness_probe = Some(probe);

            unit_file = unit_file.with_lifecycle_config(lifecycle);
        }

        // Migrate restart configuration
        if let Some(_max_retries_str) = legacy_config.get_optional("MAX_RETRIES") {
            let restart_policy = crate::RestartPolicy::OnFailure;

            let mut lifecycle = unit_file.lifecycle_config.clone();
            lifecycle.restart_policy = restart_policy;
            unit_file = unit_file.with_lifecycle_config(lifecycle);
        }

        Ok(unit_file)
    }
}

impl Default for EnvironmentMigrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Field mapping for reference.
///
/// Documents the mapping between legacy environment variables and unit file fields.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Migration
pub const LEGACY_FIELD_MAPPING: &[(&str, &str, &str)] = &[
    ("AGENT_NAME", "agent.name", "Required: agent identifier"),
    (
        "AGENT_VERSION",
        "agent.version",
        "Required: semantic version",
    ),
    (
        "AGENT_DESCRIPTION",
        "agent.description",
        "Required: agent description",
    ),
    (
        "FRAMEWORK",
        "agent.framework",
        "Optional: framework type",
    ),
    ("AUTHOR", "agent.author", "Optional: author/team name"),
    (
        "MODEL_PROVIDER",
        "model.provider",
        "Optional: LLM provider",
    ),
    (
        "MODEL_NAME",
        "model.model_name",
        "Optional: model identifier",
    ),
    (
        "MODEL_MAX_TOKENS",
        "model.max_tokens",
        "Optional: max tokens (u32)",
    ),
    (
        "TEMPERATURE",
        "model.temperature",
        "Optional: temperature (f32)",
    ),
    (
        "MEMORY_MB",
        "resources.max_memory_bytes",
        "Legacy: converted (× 1024 × 1024)",
    ),
    (
        "CPU_CORES",
        "resources (deprecated)",
        "Deprecated: no direct replacement",
    ),
    (
        "HEALTH_CHECK_TYPE",
        "health_check.type",
        "Optional: check type",
    ),
    (
        "HEALTH_CHECK_ENDPOINT",
        "health_check.endpoint",
        "Optional: check endpoint",
    ),
    (
        "MAX_RETRIES",
        "restart.max_retries",
        "Optional: max retries",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_migrator_simple() {
        let mut env = BTreeMap::new();
        env.insert("AGENT_NAME".to_string(), "test-agent".to_string());
        env.insert("AGENT_VERSION".to_string(), "1.0.0".to_string());
        env.insert("AGENT_DESCRIPTION".to_string(), "Test agent".to_string());

        let legacy_config = LegacyEnvConfig::new(env);
        let migrator = EnvironmentMigrator::new();
        let result = migrator.migrate(&legacy_config);

        assert!(result.is_ok());
        let unit = result.unwrap();
        assert_eq!(unit.metadata.name, "test-agent");
        assert_eq!(unit.metadata.version, "1.0.0");
    }

    #[test]
    fn test_environment_migrator_with_model() {
        let mut env = BTreeMap::new();
        env.insert("AGENT_NAME".to_string(), "api-agent".to_string());
        env.insert("AGENT_VERSION".to_string(), "2.0.0".to_string());
        env.insert("AGENT_DESCRIPTION".to_string(), "API agent".to_string());
        env.insert("MODEL_PROVIDER".to_string(), "openai".to_string());
        env.insert("MODEL_NAME".to_string(), "gpt-4".to_string());
        env.insert("MODEL_MAX_TOKENS".to_string(), "4096".to_string());

        let legacy_config = LegacyEnvConfig::new(env);
        let migrator = EnvironmentMigrator::new();
        let result = migrator.migrate(&legacy_config);

        assert!(result.is_ok());
        let unit = result.unwrap();
        assert_eq!(unit.model_config.provider, Some("openai".to_string()));
        assert_eq!(unit.model_config.model_name, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_environment_migrator_memory_conversion() {
        let mut env = BTreeMap::new();
        env.insert("AGENT_NAME".to_string(), "mem-agent".to_string());
        env.insert("AGENT_VERSION".to_string(), "1.0.0".to_string());
        env.insert("AGENT_DESCRIPTION".to_string(), "Memory agent".to_string());
        env.insert("MEMORY_MB".to_string(), "512".to_string());

        let legacy_config = LegacyEnvConfig::new(env);
        let migrator = EnvironmentMigrator::new();
        let result = migrator.migrate(&legacy_config);

        assert!(result.is_ok());
        let unit = result.unwrap();
        // 512 MB = 512 * 1024 * 1024 = 536870912 bytes
        assert_eq!(unit.resource_limits.max_memory_bytes, Some(536870912));
    }

    #[test]
    fn test_environment_migrator_missing_required_field() {
        let env = BTreeMap::new();
        let legacy_config = LegacyEnvConfig::new(env);
        let migrator = EnvironmentMigrator::new();
        let result = migrator.migrate(&legacy_config);

        assert!(result.is_err());
    }

    #[test]
    fn test_legacy_field_mapping_not_empty() {
        assert!(!LEGACY_FIELD_MAPPING.is_empty());
        assert!(LEGACY_FIELD_MAPPING.len() > 5);
    }

    #[test]
    fn test_migration_error_display() {
        let err = MigrationError::MissingField {
            field: "AGENT_NAME".to_string(),
        };
        assert!(format!("{}", err).contains("AGENT_NAME"));
    }
}
