// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File TOML schema specification.
//!
//! Defines the `AgentUnitFileSchema` struct that maps to the TOML-based agent unit file format.
//! Provides parsing and validation of agent unit file specifications from TOML strings.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § TOML Schema

use std::collections::BTreeMap;
use core::fmt;

/// TOML schema errors.
///
/// Represents all possible errors that can occur during TOML parsing and validation.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Error Handling
#[derive(Debug, Clone)]
pub enum UnitFileError {
    /// TOML parsing error.
    ParseError {
        /// Description of what failed to parse.
        message: String,
    },

    /// Required field is missing from the unit file.
    MissingRequired {
        /// Name of the missing field.
        field: String,
    },

    /// Field value is invalid according to schema.
    InvalidValue {
        /// Name of the invalid field.
        field: String,
        /// Description of what makes it invalid.
        reason: String,
    },

    /// Schema constraint violated.
    SchemaViolation {
        /// Description of the violation.
        message: String,
    },
}

impl fmt::Display for UnitFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError { message } => write!(f, "Parse error: {}", message),
            Self::MissingRequired { field } => write!(f, "Missing required field: {}", field),
            Self::InvalidValue { field, reason } => {
                write!(f, "Invalid value for field '{}': {}", field, reason)
            }
            Self::SchemaViolation { message } => write!(f, "Schema violation: {}", message),
        }
    }
}

/// Result type for unit file operations.
///
/// Convenience type alias for operations that may return [`UnitFileError`].
pub type UnitFileResult<T> = core::result::Result<T, UnitFileError>;

/// Agent framework type.
///
/// Specifies which agent framework is being used.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Framework Types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentFramework {
    /// LangChain agent framework.
    LangChain,
    /// Semantic Kernel agent framework.
    SemanticKernel,
    /// CrewAI agent framework.
    CrewAI,
    /// AutoGen agent framework.
    AutoGen,
    /// Custom agent framework.
    Custom,
}

impl AgentFramework {
    /// Parses framework from string.
    pub fn from_str(s: &str) -> UnitFileResult<Self> {
        match s.to_lowercase().as_str() {
            "langchain" => Ok(Self::LangChain),
            "semantic_kernel" | "semantickernel" => Ok(Self::SemanticKernel),
            "crewai" => Ok(Self::CrewAI),
            "autogen" => Ok(Self::AutoGen),
            "custom" => Ok(Self::Custom),
            other => Err(UnitFileError::InvalidValue {
                field: "framework".to_string(),
                reason: format!("Unknown framework: {}", other),
            }),
        }
    }

    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LangChain => "langchain",
            Self::SemanticKernel => "semantic_kernel",
            Self::CrewAI => "crewai",
            Self::AutoGen => "autogen",
            Self::Custom => "custom",
        }
    }
}

/// Health check type used in unit file schema.
///
/// Specifies the type of health check to perform.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Health Checks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitFileHealthCheckType {
    /// HTTP health check.
    Http,
    /// TCP health check.
    Tcp,
    /// Command execution health check.
    Exec,
    /// CSCI syscall health check.
    Csci,
}

impl UnitFileHealthCheckType {
    /// Parses health check type from string.
    pub fn from_str(s: &str) -> UnitFileResult<Self> {
        match s.to_lowercase().as_str() {
            "http" => Ok(Self::Http),
            "tcp" => Ok(Self::Tcp),
            "exec" => Ok(Self::Exec),
            "csci" => Ok(Self::Csci),
            other => Err(UnitFileError::InvalidValue {
                field: "health_check_type".to_string(),
                reason: format!("Unknown health check type: {}", other),
            }),
        }
    }

    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Tcp => "tcp",
            Self::Exec => "exec",
            Self::Csci => "csci",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RestartPolicyType {
    /// Always restart the agent.
    Always,
    /// Restart only on failure.
    OnFailure,
    /// Never restart the agent.
    Never,
}

impl RestartPolicyType {
    /// Parses restart policy type from string.
    pub fn from_str(s: &str) -> UnitFileResult<Self> {
        match s.to_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "on_failure" | "onfailure" => Ok(Self::OnFailure),
            "never" => Ok(Self::Never),
            other => Err(UnitFileError::InvalidValue {
                field: "restart_policy".to_string(),
                reason: format!("Unknown restart policy: {}", other),
            }),
        }
    }

    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::OnFailure => "on_failure",
            Self::Never => "never",
        }
    }
}

/// Agent section in unit file.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Sections
#[derive(Debug, Clone)]
pub struct AgentSection {
    /// Unique agent name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Agent framework type.
    pub framework: Option<String>,
}

/// Model section in unit file.
///
/// Configures the LLM model and inference parameters.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Model Configuration
#[derive(Debug, Clone)]
pub struct ModelSection {
    /// Model provider (e.g., "openai", "anthropic", "ollama").
    pub provider: Option<String>,
    /// Model name (e.g., `gpt-4`, `claude-opus`).
    pub model_name: Option<String>,
    /// Maximum tokens for completion.
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0-2.0).
    pub temperature: Option<f32>,
    /// Context window size in tokens.
    pub context_window: Option<u32>,
}

/// Capabilities section in unit file.
///
/// Specifies required and optional capabilities.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Capabilities
#[derive(Debug, Clone)]
pub struct CapabilitiesSection {
    /// Required capabilities (must be granted).
    pub required: Option<Vec<String>>,
    /// Optional capabilities (nice to have).
    pub optional: Option<Vec<String>>,
}

/// Resources section in unit file.
///
/// Specifies resource quotas and limits.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Resource Limits
#[derive(Debug, Clone)]
pub struct ResourcesSection {
    /// Maximum tokens per task.
    pub max_tokens_per_task: Option<u32>,
    /// Maximum GPU milliseconds.
    pub max_gpu_ms: Option<u64>,
    /// Maximum wall clock milliseconds.
    pub max_wall_clock_ms: Option<u64>,
    /// Maximum memory in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Maximum tool invocations per task.
    pub max_tool_calls: Option<u32>,
}

/// Health check section in unit file.
///
/// Configures health monitoring for the agent.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Health Checks
#[derive(Debug, Clone)]
pub struct HealthCheckSection {
    /// Health check type.
    pub check_type: Option<String>,
    /// Health check endpoint or command.
    pub endpoint: Option<String>,
    /// Probe interval in milliseconds.
    pub interval_ms: Option<u64>,
    /// Probe timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Consecutive failures before marking unhealthy.
    pub failure_threshold: Option<u32>,
    /// Consecutive successes before marking healthy.
    pub success_threshold: Option<u32>,
}

/// Restart policy section in unit file.
///
/// Configures restart behavior and backoff.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Restart Policies
#[derive(Debug, Clone)]
pub struct RestartSection {
    /// Restart policy type.
    pub policy: Option<String>,
    /// Maximum number of retries.
    pub max_retries: Option<u32>,
    /// Backoff base in milliseconds.
    pub backoff_base_ms: Option<u64>,
    /// Backoff multiplier (exponential).
    pub backoff_multiplier: Option<f32>,
    /// Maximum backoff in milliseconds.
    pub max_backoff_ms: Option<u64>,
}

/// Dependencies section in unit file.
///
/// Specifies agent dependencies and ordering constraints.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Dependencies
#[derive(Debug, Clone)]
pub struct DependenciesSection {
    /// Agents that must start after this agent.
    pub after: Option<Vec<String>>,
    /// Agents that must start before this agent.
    pub before: Option<Vec<String>>,
    /// Required services that must be available.
    pub requires: Option<Vec<String>>,
}

/// Crew section in unit file.
///
/// Specifies crew membership and role information.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Crew Configuration
#[derive(Debug, Clone)]
pub struct CrewSection {
    /// Crew identifier.
    pub name: Option<String>,
    /// Agent role in the crew (e.g., "coordinator", "worker", "specialist").
    pub role: Option<String>,
    /// Ordering priority within the crew (lower starts first).
    pub ordering_priority: Option<u32>,
}

/// Agent Unit File TOML schema.
///
/// Complete representation of an agent configuration in TOML format.
/// Provides all configuration sections for agent definition.
///
/// # TOML Format
///
/// ```text
/// [agent]
/// name = "my_agent"
/// version = "1.0.0"
/// description = "An example agent configuration"
/// framework = "langchain"
///
/// [model]
/// provider = "openai"
/// model_name = "gpt-4"
/// max_tokens = 2048
/// temperature = 0.7
/// context_window = 8192
///
/// [capabilities]
/// required = ["mem_read", "mem_write"]
/// optional = ["channel_send"]
///
/// [resources]
/// max_tokens_per_task = 4096
/// max_memory_bytes = 536870912
///
/// [health_check]
/// type = "http"
/// endpoint = "http://localhost:8080/health"
/// interval_ms = 5000
/// timeout_ms = 1000
/// failure_threshold = 3
/// success_threshold = 1
///
/// [restart]
/// policy = "on_failure"
/// max_retries = 5
/// backoff_base_ms = 100
/// backoff_multiplier = 2.0
/// max_backoff_ms = 30000
///
/// [dependencies]
/// after = ["agent-a", "agent-b"]
/// requires = ["memory-service"]
///
/// [crew]
/// name = "team-a"
/// role = "worker"
/// ordering_priority = 2
/// ```
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
#[derive(Debug, Clone)]
pub struct AgentUnitFileSchema {
    /// Agent section (required).
    pub agent: AgentSection,
    /// Model section (optional).
    pub model: Option<ModelSection>,
    /// Capabilities section (optional).
    pub capabilities: Option<CapabilitiesSection>,
    /// Resources section (optional).
    pub resources: Option<ResourcesSection>,
    /// Health check section (optional).
    pub health_check: Option<HealthCheckSection>,
    /// Restart policy section (optional).
    pub restart: Option<RestartSection>,
    /// Dependencies section (optional).
    pub dependencies: Option<DependenciesSection>,
    /// Crew membership section (optional).
    pub crew: Option<CrewSection>,
    /// Custom environment variables.
    pub environment: Option<BTreeMap<String, String>>,
}

impl AgentUnitFileSchema {
    /// Creates a new unit file schema with minimal configuration.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            agent: AgentSection {
                name: name.into(),
                version: version.into(),
                description: description.into(),
                framework: None,
            },
            model: None,
            capabilities: None,
            resources: None,
            health_check: None,
            restart: None,
            dependencies: None,
            crew: None,
            environment: None,
        }
    }

    /// Sets the agent framework.
    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.agent.framework = Some(framework.into());
        self
    }

    /// Sets the model section.
    pub fn with_model(mut self, model: ModelSection) -> Self {
        self.model = Some(model);
        self
    }

    /// Sets the capabilities section.
    pub fn with_capabilities(mut self, capabilities: CapabilitiesSection) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Sets the resources section.
    pub fn with_resources(mut self, resources: ResourcesSection) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Sets the health check section.
    pub fn with_health_check(mut self, health_check: HealthCheckSection) -> Self {
        self.health_check = Some(health_check);
        self
    }

    /// Sets the restart policy section.
    pub fn with_restart(mut self, restart: RestartSection) -> Self {
        self.restart = Some(restart);
        self
    }

    /// Sets the dependencies section.
    pub fn with_dependencies(mut self, dependencies: DependenciesSection) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    /// Sets the crew membership section.
    pub fn with_crew(mut self, crew: CrewSection) -> Self {
        self.crew = Some(crew);
        self
    }

    /// Sets environment variables.
    pub fn with_environment(mut self, env: BTreeMap<String, String>) -> Self {
        self.environment = Some(env);
        self
    }

    /// Gets the agent name.
    pub fn name(&self) -> &str {
        &self.agent.name
    }

    /// Gets the agent version.
    pub fn version(&self) -> &str {
        &self.agent.version
    }

    /// Gets the agent description.
    pub fn description(&self) -> &str {
        &self.agent.description
    }

    /// Gets the framework type if specified.
    pub fn framework(&self) -> Option<&str> {
        self.agent.framework.as_deref()
    }

    /// Gets the agent identifier string.
    pub fn identifier(&self) -> String {
        format!("{}/{}", self.agent.name, self.agent.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_framework_parsing() {
        assert_eq!(
            AgentFramework::from_str("langchain").unwrap(),
            AgentFramework::LangChain
        );
        assert_eq!(
            AgentFramework::from_str("semantic_kernel").unwrap(),
            AgentFramework::SemanticKernel
        );
        assert_eq!(
            AgentFramework::from_str("crewai").unwrap(),
            AgentFramework::CrewAI
        );
        assert_eq!(
            AgentFramework::from_str("autogen").unwrap(),
            AgentFramework::AutoGen
        );
        assert_eq!(
            AgentFramework::from_str("custom").unwrap(),
            AgentFramework::Custom
        );

        // Test error case
        assert!(AgentFramework::from_str("unknown").is_err());
    }

    #[test]
    fn test_health_check_type_parsing() {
        assert_eq!(
            UnitFileHealthCheckType::from_str("http").unwrap(),
            UnitFileHealthCheckType::Http
        );
        assert_eq!(
            UnitFileHealthCheckType::from_str("tcp").unwrap(),
            UnitFileHealthCheckType::Tcp
        );
        assert_eq!(
            UnitFileHealthCheckType::from_str("exec").unwrap(),
            UnitFileHealthCheckType::Exec
        );
        assert_eq!(
            UnitFileHealthCheckType::from_str("csci").unwrap(),
            UnitFileHealthCheckType::Csci
        );

        assert!(UnitFileHealthCheckType::from_str("unknown").is_err());
    }

    #[test]
    fn test_restart_policy_type_parsing() {
        assert_eq!(
            RestartPolicyType::from_str("always").unwrap(),
            RestartPolicyType::Always
        );
        assert_eq!(
            RestartPolicyType::from_str("on_failure").unwrap(),
            RestartPolicyType::OnFailure
        );
        assert_eq!(
            RestartPolicyType::from_str("never").unwrap(),
            RestartPolicyType::Never
        );

        assert!(RestartPolicyType::from_str("unknown").is_err());
    }

    #[test]
    fn test_unit_file_schema_new() {
        let schema = AgentUnitFileSchema::new("test-agent", "1.0.0", "Test agent");
        assert_eq!(schema.name(), "test-agent");
        assert_eq!(schema.version(), "1.0.0");
        assert_eq!(schema.description(), "Test agent");
        assert_eq!(schema.identifier(), "test-agent/1.0.0");
    }

    #[test]
    fn test_unit_file_schema_builder() {
        let schema = AgentUnitFileSchema::new("api-server", "2.0.0", "REST API server")
            .with_framework("langchain");

        assert_eq!(schema.framework(), Some("langchain"));
    }

    #[test]
    fn test_unit_file_error_display() {
        let err = UnitFileError::MissingRequired {
            field: "name".to_string(),
        };
        assert_eq!(format!("{}", err), "Missing required field: name");

        let err = UnitFileError::ParseError {
            message: "Invalid TOML syntax".to_string(),
        };
        assert!(format!("{}", err).contains("Parse error"));
    }
}
