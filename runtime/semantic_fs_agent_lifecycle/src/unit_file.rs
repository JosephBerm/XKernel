// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File specifications.
//!
//! Defines the declarative agent unit file format, similar to systemd unit files.
//! Agent unit files provide complete agent configuration in a standardized format,
//! enabling code-as-configuration and GitOps-style agent management.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
//! Week 03: TOML schema implementation with model, resources, and health check configuration

use crate::{CrewMembership, DependencySpec, LifecycleConfig};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Metadata about an agent unit file.
///
/// Provides identification and descriptive information for an agent.
///
/// # Fields
///
/// - `name`: Unique agent name (e.g., "http-server")
/// - `version`: Semantic version string (e.g., "1.0.0")
/// - `description`: Human-readable description
/// - `author`: Author or team responsible for this agent
/// - `tags`: Classification tags for discovery and organization
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
#[derive(Debug, Clone)]
pub struct UnitFileMetadata {
    /// Unique agent name.
    pub name: String,

    /// Semantic version of the agent.
    pub version: String,

    /// Human-readable description of the agent's purpose.
    pub description: String,

    /// Author or team responsible for this agent.
    pub author: Option<String>,

    /// Classification tags (e.g., ["network", "critical", "stateless"]).
    pub tags: Vec<String>,
}

impl UnitFileMetadata {
    /// Creates new agent metadata.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            author: None,
            tags: Vec::new(),
        }
    }

    /// Sets the author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Adds a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Adds multiple tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Model configuration for agent inference.
///
/// Specifies LLM provider, model selection, and inference parameters.
/// Used by the agent to configure AI model interactions.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Model Configuration
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model provider (e.g., "openai", "anthropic", "ollama").
    pub provider: Option<String>,

    /// Model name (e.g., "gpt-4", "claude-opus-4.6").
    pub model_name: Option<String>,

    /// Maximum tokens for completion.
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0-2.0).
    pub temperature: Option<f32>,

    /// Context window size in tokens.
    pub context_window: Option<u32>,
}

impl ModelConfig {
    /// Creates a new empty model configuration.
    pub fn new() -> Self {
        Self {
            provider: None,
            model_name: None,
            max_tokens: None,
            temperature: None,
            context_window: None,
        }
    }

    /// Sets the provider.
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Sets the model name.
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Sets max tokens.
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Sets temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Sets context window.
    pub fn with_context_window(mut self, tokens: u32) -> Self {
        self.context_window = Some(tokens);
        self
    }

    /// Checks if any model config is specified.
    pub fn has_config(&self) -> bool {
        self.provider.is_some()
            || self.model_name.is_some()
            || self.max_tokens.is_some()
            || self.temperature.is_some()
            || self.context_window.is_some()
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource limits and quotas for agent execution.
///
/// Specifies resource constraints for an agent including memory, compute,
/// and execution time limits.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Resource Limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
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

    /// Memory requirement in megabytes (legacy field).
    pub memory_mb: Option<u64>,

    /// CPU requirement in cores (legacy field).
    pub cpu_cores: Option<f64>,
}

impl ResourceLimits {
    /// Creates a new empty resource limits configuration.
    pub fn new() -> Self {
        Self {
            max_tokens_per_task: None,
            max_gpu_ms: None,
            max_wall_clock_ms: None,
            max_memory_bytes: None,
            max_tool_calls: None,
            memory_mb: None,
            cpu_cores: None,
        }
    }

    /// Sets maximum tokens per task.
    pub fn with_max_tokens_per_task(mut self, tokens: u32) -> Self {
        self.max_tokens_per_task = Some(tokens);
        self
    }

    /// Sets maximum GPU milliseconds.
    pub fn with_max_gpu_ms(mut self, ms: u64) -> Self {
        self.max_gpu_ms = Some(ms);
        self
    }

    /// Sets maximum wall clock milliseconds.
    pub fn with_max_wall_clock_ms(mut self, ms: u64) -> Self {
        self.max_wall_clock_ms = Some(ms);
        self
    }

    /// Sets maximum memory in bytes.
    pub fn with_max_memory_bytes(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = Some(bytes);
        self
    }

    /// Sets maximum tool calls.
    pub fn with_max_tool_calls(mut self, calls: u32) -> Self {
        self.max_tool_calls = Some(calls);
        self
    }

    /// Sets memory in megabytes (legacy).
    pub fn with_memory_mb(mut self, mb: u64) -> Self {
        self.memory_mb = Some(mb);
        self
    }

    /// Sets CPU cores (legacy).
    pub fn with_cpu_cores(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Checks if any resource limits are specified.
    pub fn has_limits(&self) -> bool {
        self.max_tokens_per_task.is_some()
            || self.max_gpu_ms.is_some()
            || self.max_wall_clock_ms.is_some()
            || self.max_memory_bytes.is_some()
            || self.max_tool_calls.is_some()
            || self.memory_mb.is_some()
            || self.cpu_cores.is_some()
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete agent unit file specification.
///
/// Declarative configuration for an agent, analogous to systemd unit files or
/// Kubernetes pod specs. Provides all information needed to instantiate, manage,
/// and operate an agent throughout its lifecycle.
///
/// This struct now includes full TOML schema support with model configuration,
/// resource limits, and extended capabilities.
///
/// # Fields
///
/// - `metadata`: Agent identification and metadata
/// - `lifecycle_config`: Startup, shutdown, health probe configuration
/// - `model_config`: LLM provider and inference parameters
/// - `resource_limits`: Memory, CPU, and execution time limits
/// - `dependencies`: Required agents and services, ordering constraints
/// - `capabilities_required`: Security and feature capabilities needed
/// - `crew_membership`: Optional crew that this agent belongs to
/// - `environment`: Environment variables and configuration
///
/// # Example
///
/// ```text
/// [agent]
/// name = "api-server"
/// version = "1.2.0"
/// description = "REST API server for agent communication"
///
/// [model]
/// provider = "openai"
/// model_name = "gpt-4"
/// max_tokens = 4096
/// context_window = 8192
///
/// [resources]
/// max_tokens_per_task = 2048
/// max_memory_bytes = 536870912
/// max_tool_calls = 10
/// ```
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files
#[derive(Debug, Clone)]
pub struct AgentUnitFile {
    /// Agent metadata (name, version, description, etc.).
    pub metadata: UnitFileMetadata,

    /// Lifecycle configuration (timeouts, health checks, restart policy).
    pub lifecycle_config: LifecycleConfig,

    /// Model configuration for LLM inference.
    pub model_config: ModelConfig,

    /// Resource limits and quotas.
    pub resource_limits: ResourceLimits,

    /// Dependency specifications (required agents/services, ordering).
    pub dependencies: Option<DependencySpec>,

    /// Memory requirement in megabytes (legacy - use resource_limits).
    pub memory_mb: Option<u64>,

    /// CPU requirement in cores (legacy - use resource_limits).
    pub cpu_cores: Option<f64>,

    /// Security capabilities required (e.g., "net_admin", "sys_resource").
    pub capabilities_required: Vec<String>,

    /// Crew membership information if agent belongs to a crew.
    pub crew_membership: Option<CrewMembership>,

    /// Environment variables and configuration.
    pub environment: BTreeMap<String, String>,
}

impl AgentUnitFile {
    /// Creates a new agent unit file with minimal configuration.
    ///
    /// Starts with default lifecycle, model, and resource configurations.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            metadata: UnitFileMetadata::new(name, version, description),
            lifecycle_config: LifecycleConfig::default(),
            model_config: ModelConfig::default(),
            resource_limits: ResourceLimits::default(),
            dependencies: None,
            memory_mb: None,
            cpu_cores: None,
            capabilities_required: Vec::new(),
            crew_membership: None,
            environment: BTreeMap::new(),
        }
    }

    /// Sets the author in metadata.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_author(author);
        self
    }

    /// Adds a tag to metadata.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_tag(tag);
        self
    }

    /// Sets the lifecycle configuration.
    pub fn with_lifecycle_config(mut self, config: LifecycleConfig) -> Self {
        self.lifecycle_config = config;
        self
    }

    /// Sets the model configuration.
    pub fn with_model_config(mut self, config: ModelConfig) -> Self {
        self.model_config = config;
        self
    }

    /// Sets the resource limits.
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    /// Sets the dependency specification.
    pub fn with_dependencies(mut self, deps: DependencySpec) -> Self {
        self.dependencies = Some(deps);
        self
    }

    /// Sets memory requirement in megabytes.
    pub fn with_memory_mb(mut self, mb: u64) -> Self {
        self.memory_mb = Some(mb);
        self
    }

    /// Sets CPU requirement in cores.
    pub fn with_cpu_cores(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Adds a required capability.
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities_required.push(capability.into());
        self
    }

    /// Adds multiple required capabilities.
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities_required.extend(capabilities);
        self
    }

    /// Sets crew membership.
    pub fn with_crew_membership(mut self, membership: CrewMembership) -> Self {
        self.crew_membership = Some(membership);
        self
    }

    /// Sets an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Sets multiple environment variables.
    pub fn with_environment(mut self, env: BTreeMap<String, String>) -> Self {
        self.environment = env;
        self
    }

    /// Checks if this agent is part of a crew.
    pub fn is_crew_member(&self) -> bool {
        self.crew_membership.is_some()
    }

    /// Gets the agent's crew ID if it's a crew member.
    pub fn crew_id(&self) -> Option<&str> {
        self.crew_membership.as_ref().map(|m| m.crew_id.as_str())
    }

    /// Gets the agent's role in its crew if applicable.
    pub fn crew_role(&self) -> Option<&str> {
        self.crew_membership.as_ref().map(|m| m.role.as_str())
    }

    /// Checks if this agent has resource limits specified.
    pub fn has_resource_limits(&self) -> bool {
        self.memory_mb.is_some()
            || self.cpu_cores.is_some()
            || self.resource_limits.has_limits()
    }

    /// Checks if this agent has model configuration specified.
    pub fn has_model_config(&self) -> bool {
        self.model_config.has_config()
    }

    /// Checks if this agent requires specific capabilities.
    pub fn requires_capabilities(&self) -> bool {
        !self.capabilities_required.is_empty()
    }

    /// Gets a string representation suitable for logging/display.
    pub fn identifier(&self) -> String {
        alloc::format!("{}/{}", self.metadata.name, self.metadata.version)
    }
}

impl Default for AgentUnitFile {
    fn default() -> Self {
        Self::new("unnamed", "0.0.0", "Unnamed agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_unit_file_metadata_new() {
        let metadata = UnitFileMetadata::new("test-agent", "1.0.0", "A test agent");
        assert_eq!(metadata.name, "test-agent");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, "A test agent");
        assert!(metadata.author.is_none());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_unit_file_metadata_builder() {
        let metadata = UnitFileMetadata::new("api-server", "2.1.0", "REST API")
            .with_author("Core Team")
            .with_tag("network")
            .with_tag("critical");

        assert_eq!(metadata.author, Some("Core Team".to_string()));
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains(&"network".to_string()));
        assert!(metadata.tags.contains(&"critical".to_string()));
    }

    #[test]
    fn test_unit_file_new() {
        let unit = AgentUnitFile::new("my-agent", "1.0.0", "Test agent");
        assert_eq!(unit.metadata.name, "my-agent");
        assert_eq!(unit.metadata.version, "1.0.0");
        assert!(unit.dependencies.is_none());
        assert!(unit.memory_mb.is_none());
        assert!(unit.cpu_cores.is_none());
        assert!(unit.crew_membership.is_none());
        assert!(unit.environment.is_empty());
    }

    #[test]
    fn test_unit_file_builder() {
        let unit = AgentUnitFile::new("http-server", "1.0.0", "HTTP server")
            .with_author("Web Team")
            .with_tag("network")
            .with_memory_mb(512)
            .with_cpu_cores(2.0)
            .with_capability("net_bind_service")
            .with_env("PORT", "8080")
            .with_env("LOG_LEVEL", "info");

        assert_eq!(unit.metadata.author, Some("Web Team".to_string()));
        assert_eq!(unit.memory_mb, Some(512));
        assert_eq!(unit.cpu_cores, Some(2.0));
        assert_eq!(unit.capabilities_required.len(), 1);
        assert_eq!(unit.environment.len(), 2);
    }

    #[test]
    fn test_unit_file_with_dependencies() {
        let deps = DependencySpec::new()
            .with_required_service("database")
            .after("cache");

        let unit = AgentUnitFile::new("app", "1.0.0", "App")
            .with_dependencies(deps);

        assert!(unit.dependencies.is_some());
    }

    #[test]
    fn test_unit_file_with_crew_membership() {
        let membership = CrewMembership::new("crew-1", "agent-1", "leader");
        let unit = AgentUnitFile::new("agent-1", "1.0.0", "Agent in crew")
            .with_crew_membership(membership);

        assert!(unit.is_crew_member());
        assert_eq!(unit.crew_id(), Some("crew-1"));
        assert_eq!(unit.crew_role(), Some("leader"));
    }

    #[test]
    fn test_unit_file_identifier() {
        let unit = AgentUnitFile::new("my-service", "2.1.0", "Service");
        assert_eq!(unit.identifier(), "my-service/2.1.0");
    }

    #[test]
    fn test_unit_file_has_resource_limits() {
        let unit1 = AgentUnitFile::new("agent-1", "1.0.0", "Test");
        assert!(!unit1.has_resource_limits());

        let unit2 = AgentUnitFile::new("agent-2", "1.0.0", "Test")
            .with_memory_mb(256);
        assert!(unit2.has_resource_limits());

        let unit3 = AgentUnitFile::new("agent-3", "1.0.0", "Test")
            .with_cpu_cores(1.0);
        assert!(unit3.has_resource_limits());
    }

    #[test]
    fn test_unit_file_requires_capabilities() {
        let unit1 = AgentUnitFile::new("agent-1", "1.0.0", "Test");
        assert!(!unit1.requires_capabilities());

        let unit2 = AgentUnitFile::new("agent-2", "1.0.0", "Test")
            .with_capability("net_admin");
        assert!(unit2.requires_capabilities());
    }

    #[test]
    fn test_unit_file_default() {
        let unit = AgentUnitFile::default();
        assert_eq!(unit.metadata.name, "unnamed");
        assert_eq!(unit.metadata.version, "0.0.0");
        assert!(!unit.is_crew_member());
        assert!(!unit.requires_capabilities());
    }

    #[test]
    fn test_unit_file_with_multiple_capabilities() {
        let caps = alloc::vec![
            "net_admin".to_string(),
            "sys_resource".to_string(),
            "sys_ptrace".to_string(),
        ];
        let unit = AgentUnitFile::new("privileged-agent", "1.0.0", "Test")
            .with_capabilities(caps);

        assert_eq!(unit.capabilities_required.len(), 3);
        assert!(unit.capabilities_required.contains(&"net_admin".to_string()));
        assert!(unit.capabilities_required.contains(&"sys_ptrace".to_string()));
    }

    #[test]
    fn test_model_config_new() {
        let config = ModelConfig::new();
        assert!(config.provider.is_none());
        assert!(config.model_name.is_none());
        assert!(!config.has_config());
    }

    #[test]
    fn test_model_config_builder() {
        let config = ModelConfig::new()
            .with_provider("openai")
            .with_model_name("gpt-4")
            .with_max_tokens(4096)
            .with_temperature(0.7)
            .with_context_window(8192);

        assert_eq!(config.provider, Some("openai".to_string()));
        assert_eq!(config.model_name, Some("gpt-4".to_string()));
        assert_eq!(config.max_tokens, Some(4096));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.context_window, Some(8192));
        assert!(config.has_config());
    }

    #[test]
    fn test_resource_limits_new() {
        let limits = ResourceLimits::new();
        assert!(limits.max_tokens_per_task.is_none());
        assert!(limits.max_memory_bytes.is_none());
        assert!(!limits.has_limits());
    }

    #[test]
    fn test_resource_limits_builder() {
        let limits = ResourceLimits::new()
            .with_max_tokens_per_task(2048)
            .with_max_memory_bytes(1073741824)
            .with_max_tool_calls(10);

        assert_eq!(limits.max_tokens_per_task, Some(2048));
        assert_eq!(limits.max_memory_bytes, Some(1073741824));
        assert_eq!(limits.max_tool_calls, Some(10));
        assert!(limits.has_limits());
    }

    #[test]
    fn test_unit_file_with_model_config() {
        let model = ModelConfig::new().with_provider("anthropic");
        let unit = AgentUnitFile::new("agent", "1.0.0", "Test").with_model_config(model);

        assert!(unit.has_model_config());
    }

    #[test]
    fn test_unit_file_with_resource_limits() {
        let limits = ResourceLimits::new().with_max_memory_bytes(536870912);
        let unit = AgentUnitFile::new("agent", "1.0.0", "Test")
            .with_resource_limits(limits);

        assert!(unit.has_resource_limits());
    }
}
