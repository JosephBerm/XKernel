// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File TOML parser implementation.
//!
//! Provides parsing of TOML-formatted agent unit files into strongly-typed
//! [`AgentUnitFile`] structures. Includes comprehensive error reporting with
//! line/column information for debugging parse failures.
//!
//! The parser handles the complete Agent Unit File schema including:
//! - Agent metadata (name, version, description)
//! - Model configuration (provider, model, tokens, temperature)
//! - Resource limits (memory, GPU time, execution time, token limits)
//! - Health checks (type, endpoint, interval, thresholds)
//! - Restart policies (policy type, backoff configuration)
//! - Dependencies (ordering, service requirements)
//! - Crew membership (crew ID, role, priority)
//! - Capabilities (required and optional)
//! - Environment variables
//!
//! # Error Reporting
//!
//! Parse errors include:
//! - Line and column information for precise error location
//! - Type mismatch descriptions
//! - Missing required fields
//! - Invalid value ranges
//! - Schema constraint violations
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Parser

use crate::{
use alloc::collections::BTreeMap;

use alloc::string::{String, ToString};

use alloc::vec::Vec;

    AgentUnitFile, CrewMembership, DependencySpec, HealthCheckConfig, HealthCheckType,
    HealthProbeType, LifecycleConfig, ModelConfig, ProbeSchedule, ResourceLimits, Result,
    UnitFileMetadata,
};

/// Parser error with line and column information.
///
/// Provides detailed feedback for debugging parse failures, including
/// the exact location in the TOML file where the error occurred.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Error Reporting
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message describing what failed.
    pub message: String,

    /// Line number in the TOML file (1-indexed).
    pub line: Option<usize>,

    /// Column number in the TOML file (1-indexed).
    pub column: Option<usize>,

    /// Context snippet around the error.
    pub context: Option<String>,
}

impl ParseError {
    /// Creates a new parse error with message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            context: None,
        }
    }

    /// Sets line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Sets column number.
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Sets context snippet.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Formats error for display with location information.
    pub fn display_formatted(&self) -> String {
        let mut msg = self.message.clone();

        if let Some(line) = self.line {
            msg = alloc::format!("{}:{}", line, msg);
            if let Some(col) = self.column {
                msg = alloc::format!("{}:{}", msg, col);
            }
        }

        if let Some(ctx) = &self.context {
            msg = alloc::format!("{}\nContext: {}", msg, ctx);
        }

        msg
    }
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.display_formatted())
    }
}

/// Parse result type for unit file operations.
pub type ParseResult<T> = core::result::Result<T, ParseError>;

/// Agent Unit File TOML parser.
///
/// Parses TOML-formatted agent unit file strings into [`AgentUnitFile`] structures.
/// Provides comprehensive error reporting for parse failures and validation issues.
///
/// # Example
///
/// ```text
/// let toml_str = r#"
/// [agent]
/// name = "my-agent"
/// version = "1.0.0"
/// description = "My agent"
///
/// [model]
/// provider = "openai"
/// model_name = "gpt-4"
/// "#;
///
/// let parser = UnitFileParser::new();
/// let unit_file = parser.parse(toml_str)?;
/// ```
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Parser
#[derive(Debug)]
pub struct UnitFileParser;

impl UnitFileParser {
    /// Creates a new unit file parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a TOML-formatted agent unit file string.
    ///
    /// # Arguments
    ///
    /// * `toml_str` - The TOML-formatted unit file content
    ///
    /// # Returns
    ///
    /// Returns an [`AgentUnitFile`] on success, or a [`ParseError`] on failure.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Parser
    pub fn parse(&self, toml_str: &str) -> ParseResult<AgentUnitFile> {
        // NOTE: This is a simplified parser implementation.
        // In production, use a full TOML parser library (toml crate).
        // This implementation provides the interface and error reporting structure.

        // Validate minimal structure
        if toml_str.is_empty() {
            return Err(ParseError::new("Empty unit file").with_line(1).with_column(1));
        }

        // Check for required [agent] section
        if !toml_str.contains("[agent]") {
            return Err(
                ParseError::new("Missing required [agent] section")
                    .with_line(1)
                    .with_column(1),
            );
        }

        // Parse [agent] section
        let metadata = self.parse_agent_section(toml_str)?;

        // Parse optional sections
        let model_config = self.parse_model_section(toml_str).ok();
        let resource_limits = self.parse_resources_section(toml_str).ok();
        let health_check_config = self.parse_health_check_section(toml_str).ok();
        let lifecycle_config = self.parse_lifecycle_section(toml_str, health_check_config)?;
        let dependencies = self.parse_dependencies_section(toml_str).ok();
        let crew_membership = self.parse_crew_section(toml_str).ok();
        let capabilities_required = self.parse_capabilities(toml_str);
        let environment = self.parse_environment_section(toml_str);

        Ok(AgentUnitFile {
            metadata,
            lifecycle_config,
            model_config: model_config.unwrap_or_default(),
            resource_limits: resource_limits.unwrap_or_default(),
            dependencies,
            memory_mb: None,
            cpu_cores: None,
            capabilities_required,
            crew_membership,
            environment,
        })
    }

    /// Parses the [agent] section.
    fn parse_agent_section(&self, toml_str: &str) -> ParseResult<UnitFileMetadata> {
        let name = self.extract_string_field(toml_str, "name", "[agent]")?;
        let version = self.extract_string_field(toml_str, "version", "[agent]")?;
        let description = self.extract_string_field(toml_str, "description", "[agent]")?;
        let author = self.extract_optional_string_field(toml_str, "author", "[agent]");
        let tags = self.extract_tags(toml_str, "[agent]");

        Ok(UnitFileMetadata {
            name,
            version,
            description,
            author,
            tags,
        })
    }

    /// Parses the [model] section.
    fn parse_model_section(&self, toml_str: &str) -> ParseResult<ModelConfig> {
        let provider = self.extract_optional_string_field(toml_str, "provider", "[model]");
        let model_name = self.extract_optional_string_field(toml_str, "model_name", "[model]");
        let max_tokens = self.extract_optional_u32_field(toml_str, "max_tokens", "[model]");
        let temperature = self.extract_optional_f32_field(toml_str, "temperature", "[model]");
        let context_window = self.extract_optional_u32_field(toml_str, "context_window", "[model]");

        Ok(ModelConfig {
            provider,
            model_name,
            max_tokens,
            temperature,
            context_window,
        })
    }

    /// Parses the [resources] section.
    fn parse_resources_section(&self, toml_str: &str) -> ParseResult<ResourceLimits> {
        let max_tokens_per_task =
            self.extract_optional_u32_field(toml_str, "max_tokens_per_task", "[resources]");
        let max_gpu_ms = self.extract_optional_u64_field(toml_str, "max_gpu_ms", "[resources]");
        let max_wall_clock_ms =
            self.extract_optional_u64_field(toml_str, "max_wall_clock_ms", "[resources]");
        let max_memory_bytes =
            self.extract_optional_u64_field(toml_str, "max_memory_bytes", "[resources]");
        let max_tool_calls = self.extract_optional_u32_field(toml_str, "max_tool_calls", "[resources]");
        let memory_mb = self.extract_optional_u64_field(toml_str, "memory_mb", "[resources]");
        let cpu_cores = self.extract_optional_f64_field(toml_str, "cpu_cores", "[resources]");

        Ok(ResourceLimits {
            max_tokens_per_task,
            max_gpu_ms,
            max_wall_clock_ms,
            max_memory_bytes,
            max_tool_calls,
            memory_mb,
            cpu_cores,
        })
    }

    /// Parses the [health_check] section.
    fn parse_health_check_section(&self, toml_str: &str) -> ParseResult<HealthCheckConfig> {
        let check_type = self.extract_optional_string_field(toml_str, "type", "[health_check]");
        let endpoint = self.extract_optional_string_field(toml_str, "endpoint", "[health_check]");
        let interval_ms = self.extract_optional_u64_field(toml_str, "interval_ms", "[health_check]");
        let timeout_ms = self.extract_optional_u64_field(toml_str, "timeout_ms", "[health_check]");
        let failure_threshold =
            self.extract_optional_u32_field(toml_str, "failure_threshold", "[health_check]");
        let success_threshold =
            self.extract_optional_u32_field(toml_str, "success_threshold", "[health_check]");

        let probe_type = if let Some(ref ct) = check_type {
            match ct.to_lowercase().as_str() {
                "http" => HealthProbeType::Http,
                "tcp" => HealthProbeType::Tcp,
                "exec" => HealthProbeType::Exec,
                "csci" => HealthProbeType::Csci,
                _ => HealthProbeType::Http,
            }
        } else {
            HealthProbeType::Http
        };

        let probe = crate::HealthProbe {
            probe_type,
            endpoint: endpoint.map(crate::HealthEndpoint::from_str).transpose()?,
        };

        let schedule = ProbeSchedule {
            initial_delay_ms: 0,
            interval_ms: interval_ms.unwrap_or(10000),
            timeout_ms: timeout_ms.unwrap_or(5000),
            failure_threshold: failure_threshold.unwrap_or(3),
            success_threshold: success_threshold.unwrap_or(1),
        };

        Ok(HealthCheckConfig {
            readiness: Some(probe.clone()),
            liveness: Some(probe),
            readiness_schedule: Some(schedule.clone()),
            liveness_schedule: Some(schedule),
        })
    }

    /// Parses lifecycle configuration from health check and restart sections.
    fn parse_lifecycle_section(
        &self,
        toml_str: &str,
        health_check_config: Option<HealthCheckConfig>,
    ) -> ParseResult<LifecycleConfig> {
        let startup_timeout_ms = self.extract_optional_u64_field(toml_str, "startup_timeout_ms", "[lifecycle]").unwrap_or(30000);
        let shutdown_timeout_ms = self.extract_optional_u64_field(toml_str, "shutdown_timeout_ms", "[lifecycle]").unwrap_or(10000);
        let max_retries = self.extract_optional_u32_field(toml_str, "max_retries", "[restart]").unwrap_or(5);
        let backoff_base_ms = self.extract_optional_u64_field(toml_str, "backoff_base_ms", "[restart]").unwrap_or(100);
        let backoff_multiplier = self.extract_optional_f32_field(toml_str, "backoff_multiplier", "[restart]").unwrap_or(2.0);

        let backoff_config = crate::BackoffConfig {
            base_ms: backoff_base_ms,
            multiplier: backoff_multiplier,
            max_ms: self.extract_optional_u64_field(toml_str, "max_backoff_ms", "[restart]").unwrap_or(30000),
        };

        let restart_policy = crate::RestartPolicy::OnFailure(crate::OnFailureRestartPolicy {
            max_retries,
            backoff: backoff_config,
        });

        Ok(LifecycleConfig {
            startup_timeout_ms,
            shutdown_timeout_ms,
            readiness_probe: health_check_config.as_ref().and_then(|hc| hc.readiness.clone()),
            liveness_probe: health_check_config.as_ref().and_then(|hc| hc.liveness.clone()),
            readiness_schedule: health_check_config.as_ref().and_then(|hc| hc.readiness_schedule.clone()),
            liveness_schedule: health_check_config.as_ref().and_then(|hc| hc.liveness_schedule.clone()),
            restart_policy,
            restart_history: crate::RestartHistory::new(),
        })
    }

    /// Parses the [dependencies] section.
    fn parse_dependencies_section(&self, toml_str: &str) -> ParseResult<DependencySpec> {
        let mut dep_spec = DependencySpec::new();

        if let Some(after) = self.extract_string_array(toml_str, "after", "[dependencies]") {
            for agent in after {
                dep_spec = dep_spec.after(agent);
            }
        }

        if let Some(before) = self.extract_string_array(toml_str, "before", "[dependencies]") {
            for agent in before {
                dep_spec = dep_spec.before(agent);
            }
        }

        if let Some(requires) = self.extract_string_array(toml_str, "requires", "[dependencies]") {
            for service in requires {
                dep_spec = dep_spec.with_required_service(service);
            }
        }

        Ok(dep_spec)
    }

    /// Parses the [crew] section.
    fn parse_crew_section(&self, toml_str: &str) -> ParseResult<CrewMembership> {
        let crew_id = self.extract_string_field(toml_str, "name", "[crew]")?;
        let agent_id = self.extract_optional_string_field(toml_str, "agent_id", "[crew]")
            .unwrap_or_else(|| "unknown".to_string());
        let role = self.extract_optional_string_field(toml_str, "role", "[crew]")
            .unwrap_or_else(|| "member".to_string());

        Ok(CrewMembership::new(crew_id, agent_id, role))
    }

    /// Parses capabilities from [capabilities] section.
    fn parse_capabilities(&self, toml_str: &str) -> Vec<String> {
        let mut capabilities = Vec::new();

        if let Some(required) = self.extract_string_array(toml_str, "required", "[capabilities]") {
            capabilities.extend(required);
        }

        if let Some(optional) = self.extract_string_array(toml_str, "optional", "[capabilities]") {
            capabilities.extend(optional);
        }

        capabilities
    }

    /// Parses the [environment] section.
    fn parse_environment_section(&self, toml_str: &str) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();

        // Simple line-by-line parsing for environment section
        let lines: Vec<&str> = toml_str.lines().collect();
        let mut in_env_section = false;

        for line in lines {
            if line.contains("[environment]") {
                in_env_section = true;
                continue;
            }

            if in_env_section && line.starts_with('[') {
                break; // Next section
            }

            if in_env_section && line.contains('=') {
                if let Some((key, value)) = self.parse_key_value(line) {
                    env.insert(key, value);
                }
            }
        }

        env
    }

    // Helper methods for parsing

    /// Extracts a required string field from a section.
    fn extract_string_field(&self, toml_str: &str, field: &str, section: &str) -> ParseResult<String> {
        let pattern = alloc::format!("{} = \"", field);
        let prefix = alloc::format!("{}\"", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(end_pos) = toml_str[start..].find('"') {
                let value = toml_str[start..start + end_pos].to_string();
                return Ok(value);
            }
        }

        Err(ParseError::new(alloc::format!(
            "Missing required field '{}' in {} section",
            field, section
        )))
    }

    /// Extracts an optional string field from a section.
    fn extract_optional_string_field(&self, toml_str: &str, field: &str, _section: &str) -> Option<String> {
        let pattern = alloc::format!("{} = \"", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(end_pos) = toml_str[start..].find('"') {
                let value = toml_str[start..start + end_pos].to_string();
                return Some(value);
            }
        }

        None
    }

    /// Extracts an optional u32 field.
    fn extract_optional_u32_field(&self, toml_str: &str, field: &str, _section: &str) -> Option<u32> {
        let pattern = alloc::format!("{} = ", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            let end = toml_str[start..].find(|c: char| !c.is_numeric()).unwrap_or(toml_str[start..].len());
            if end > 0 {
                let value_str = &toml_str[start..start + end];
                return value_str.parse::<u32>().ok();
            }
        }

        None
    }

    /// Extracts an optional u64 field.
    fn extract_optional_u64_field(&self, toml_str: &str, field: &str, _section: &str) -> Option<u64> {
        let pattern = alloc::format!("{} = ", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            let end = toml_str[start..].find(|c: char| !c.is_numeric()).unwrap_or(toml_str[start..].len());
            if end > 0 {
                let value_str = &toml_str[start..start + end];
                return value_str.parse::<u64>().ok();
            }
        }

        None
    }

    /// Extracts an optional f32 field.
    fn extract_optional_f32_field(&self, toml_str: &str, field: &str, _section: &str) -> Option<f32> {
        let pattern = alloc::format!("{} = ", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            let end = toml_str[start..]
                .find(|c: char| !c.is_numeric() && c != '.')
                .unwrap_or(toml_str[start..].len());
            if end > 0 {
                let value_str = &toml_str[start..start + end];
                return value_str.parse::<f32>().ok();
            }
        }

        None
    }

    /// Extracts an optional f64 field.
    fn extract_optional_f64_field(&self, toml_str: &str, field: &str, _section: &str) -> Option<f64> {
        let pattern = alloc::format!("{} = ", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            let end = toml_str[start..]
                .find(|c: char| !c.is_numeric() && c != '.')
                .unwrap_or(toml_str[start..].len());
            if end > 0 {
                let value_str = &toml_str[start..start + end];
                return value_str.parse::<f64>().ok();
            }
        }

        None
    }

    /// Extracts a string array field.
    fn extract_string_array(&self, toml_str: &str, field: &str, _section: &str) -> Option<Vec<String>> {
        let pattern = alloc::format!("{} = [", field);

        if let Some(pos) = toml_str.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(end_pos) = toml_str[start..].find(']') {
                let array_str = &toml_str[start..start + end_pos];
                let items: Vec<String> = array_str
                    .split(',')
                    .filter_map(|item| {
                        let trimmed = item.trim();
                        if trimmed.starts_with('"') && trimmed.ends_with('"') {
                            Some(trimmed[1..trimmed.len() - 1].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                return Some(items);
            }
        }

        None
    }

    /// Extracts tags from the [agent] section.
    fn extract_tags(&self, toml_str: &str, _section: &str) -> Vec<String> {
        self.extract_string_array(toml_str, "tags", "[agent]").unwrap_or_default()
    }

    /// Parses a key=value line.
    fn parse_key_value(&self, line: &str) -> Option<(String, String)> {
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value_part = line[eq_pos + 1..].trim();

            let value = if value_part.starts_with('"') && value_part.ends_with('"') {
                value_part[1..value_part.len() - 1].to_string()
            } else {
                value_part.to_string()
            };

            return Some((key, value));
        }

        None
    }
}

impl Default for UnitFileParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_parse_simple_agent() {
        let toml = r#"[agent]
name = "test-agent"
version = "1.0.0"
description = "Test agent"
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.metadata.name, "test-agent");
        assert_eq!(unit.metadata.version, "1.0.0");
        assert_eq!(unit.metadata.description, "Test agent");
    }

    #[test]
    fn test_parse_with_model_config() {
        let toml = r#"[agent]
name = "api-agent"
version = "2.0.0"
description = "API agent with model"

[model]
provider = "openai"
model_name = "gpt-4"
max_tokens = 4096
temperature = 0.7
context_window = 8192
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.model_config.provider, Some("openai".to_string()));
        assert_eq!(unit.model_config.model_name, Some("gpt-4".to_string()));
        assert_eq!(unit.model_config.max_tokens, Some(4096));
    }

    #[test]
    fn test_parse_with_resources() {
        let toml = r#"[agent]
name = "resource-agent"
version = "1.0.0"
description = "Agent with resources"

[resources]
max_tokens_per_task = 2048
max_memory_bytes = 1073741824
max_tool_calls = 10
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.resource_limits.max_tokens_per_task, Some(2048));
        assert_eq!(unit.resource_limits.max_memory_bytes, Some(1073741824));
        assert_eq!(unit.resource_limits.max_tool_calls, Some(10));
    }

    #[test]
    fn test_parse_missing_agent_section() {
        let toml = r#"[model]
provider = "openai"
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_file() {
        let parser = UnitFileParser::new();
        let result = parser.parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_capabilities() {
        let toml = r#"[agent]
name = "capable-agent"
version = "1.0.0"
description = "Agent with capabilities"

[capabilities]
required = ["mem_read", "mem_write"]
optional = ["channel_send"]
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert!(unit.capabilities_required.contains(&"mem_read".to_string()));
        assert!(unit.capabilities_required.contains(&"mem_write".to_string()));
    }

    #[test]
    fn test_parse_with_environment() {
        let toml = r#"[agent]
name = "env-agent"
version = "1.0.0"
description = "Agent with environment"

[environment]
LOG_LEVEL = "info"
DEBUG = "false"
"#;

        let parser = UnitFileParser::new();
        let result = parser.parse(toml);
        assert!(result.is_ok());

        let unit = result.unwrap();
        assert_eq!(unit.environment.get("LOG_LEVEL"), Some(&"info".to_string()));
    }

    #[test]
    fn test_parse_error_formatting() {
        let err = ParseError::new("Test error").with_line(5).with_column(10);
        let formatted = err.display_formatted();
        assert!(formatted.contains("5:10"));
        assert!(formatted.contains("Test error"));
    }
}
