// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File RFC-style specification.
//!
//! Provides the complete RFC-style specification for the Agent Unit File format.
//! This module serves as the authoritative definition of the format, including:
//!
//! - Abstract and motivation
//! - Formal specification of all sections and fields
//! - Type definitions and constraints
//! - Default values and deprecation rules
//! - Security considerations
//! - Backward compatibility notes
//!
//! This is both documentation and executable specification that can be used
//! for format validation and documentation generation.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC Specification

use alloc::string::String;

/// Agent Unit File RFC Specification - Abstract
///
/// The Agent Unit File (AUF) is a declarative configuration format for specifying
/// autonomous agents in the Cognitive Substrate OS. It provides a complete,
/// human-readable definition of an agent's capabilities, constraints, dependencies,
/// and operational parameters.
///
/// The format is inspired by systemd unit files and Kubernetes manifests, providing
/// a familiar, battle-tested approach to agent lifecycle management.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Abstract
pub const RFC_ABSTRACT: &str = r#"
ABSTRACT

The Agent Unit File (AUF) is a declarative configuration format for specifying
autonomous agents in the Cognitive Substrate OS. It enables:

1. Code-as-configuration: Agent specifications as version-controlled TOML files
2. GitOps integration: Easy deployment and rollback of agent configurations
3. Standardized format: Familiar TOML syntax reduces adoption friction
4. Complete specification: All lifecycle and operational parameters in one place
5. Strong typing: Validation ensures configuration correctness before deployment

The format draws inspiration from:
- systemd unit files (.service, .socket specifications)
- Kubernetes manifests (Pod and Deployment specs)
- Docker Compose (Multi-container configurations)

This provides a balance between expressiveness and simplicity.
"#;

/// RFC Motivation and Problem Statement
///
/// Clearly articulates why the Agent Unit File format is necessary.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Motivation
pub const RFC_MOTIVATION: &str = r#"
MOTIVATION

Current state of agent configuration:
- Agents are configured via environment variables, scattered across multiple files
- Dependencies are implicit, making deployments error-prone
- Health checks are not standardized, forcing ad-hoc implementations
- Resource limits lack enforcement mechanisms
- No clear path for version management and gradual rollout

The Agent Unit File solves these problems by providing:

1. Unified specification: All agent configuration in a single, versioned file
2. Explicit dependencies: Clear ordering and service requirements
3. Standardized health checks: Common patterns for readiness/liveness probes
4. Strong typing: Type-safe configuration with validation before deployment
5. Declarative approach: Focus on "what" not "how" for agent configuration
6. GitOps friendly: Version control and CI/CD integration

This enables autonomous agents to be deployed with the same rigor as traditional
microservices, while providing domain-specific features for agent lifecycle management.
"#;

/// Agent Unit File Field Specification
///
/// Documents all fields, their types, defaults, and constraints.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Specification
pub const RFC_SPECIFICATION: &str = r#"
SPECIFICATION

1. AGENT SECTION [agent]

    name (string, required)
        Unique agent identifier within the deployment.
        Pattern: ^[a-z][a-z0-9]*(-[a-z0-9]+)*$
        Example: "http-server", "data-processor"
        Length: 1-255 characters

    version (string, required)
        Semantic version of the agent (SemVer 2.0.0).
        Pattern: ^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$
        Example: "1.0.0", "2.1.0-beta.1"

    description (string, required)
        Human-readable description of the agent's purpose.
        Length: 1-1024 characters
        Example: "REST API server for agent communication"

    framework (string, optional)
        Agent framework being used.
        Values: "langchain", "semantic_kernel", "crewai", "autogen", "custom"
        Default: None
        Determines which SDK and tooling is required for deployment.

    author (string, optional)
        Team or person responsible for this agent.
        Default: None
        Example: "Platform Team"

    tags (array of strings, optional)
        Classification tags for discovery and organization.
        Default: Empty array
        Examples: ["network", "critical", "stateless", "batch-processor"]

2. MODEL SECTION [model] (optional)

    provider (string, optional)
        LLM provider identifier.
        Valid values: "openai", "anthropic", "ollama", "local", "custom"
        Default: None
        Used to select appropriate SDK and authentication.

    model_name (string, optional)
        Specific model identifier.
        Examples: "gpt-4", "claude-opus-4.6", "llama2:70b"
        Default: None
        Must be compatible with provider.

    max_tokens (u32, optional)
        Maximum tokens for model completion.
        Range: 1-1000000
        Default: None (use provider default)
        Example: 4096

    temperature (f32, optional)
        Sampling temperature for model output.
        Range: 0.0-2.0
        Default: None (use provider default)
        Semantic: 0.0 = deterministic, 1.0 = balanced, >1.0 = creative

    context_window (u32, optional)
        Effective context window size in tokens.
        Range: 128-1000000
        Default: None (use model's maximum)
        Example: 8192 for gpt-4, 200000 for claude-opus-4.6

3. CAPABILITIES SECTION [capabilities] (optional)

    required (array of strings, optional)
        Capabilities that MUST be granted for agent to function.
        Valid values: Standard capability set (see Section 4)
        Default: None
        Example: ["mem_read", "mem_write", "tool_invoke"]

    optional (array of strings, optional)
        Capabilities that MAY be granted to enhance functionality.
        Valid values: Standard capability set
        Default: None
        Example: ["channel_send", "file_access"]

4. STANDARD CAPABILITIES

    mem_read: Read memory of other agents
    mem_write: Write to memory of other agents
    tool_invoke: Invoke external tools and APIs
    channel_send: Send messages on communication channels
    file_access: Read/write files in agent workspace
    network_raw: Raw network access
    gpu_compute: GPU computation access
    agent_spawn: Spawn new agents dynamically
    database_access: Direct database queries
    sys_resource: System resource queries
    sys_ptrace: Process tracing
    net_admin: Network administration
    net_bind_service: Bind to service ports

5. RESOURCES SECTION [resources] (optional)

    max_tokens_per_task (u32, optional)
        Maximum tokens per individual task execution.
        Range: 1-100000
        Default: None
        Provides fine-grained token budgeting per task.

    max_gpu_ms (u64, optional)
        Maximum GPU compute time in milliseconds.
        Range: 1-3600000 (1 hour)
        Default: None
        Enforces GPU utilization limits.

    max_wall_clock_ms (u64, optional)
        Maximum wall-clock execution time in milliseconds.
        Range: 1-86400000 (24 hours)
        Default: None (no limit)
        Constraint: Must be >= max_gpu_ms if both specified.

    max_memory_bytes (u64, optional)
        Maximum memory available to agent in bytes.
        Range: 1-17179869184 (16GB)
        Default: None
        Example: 1073741824 (1GB)

    max_tool_calls (u32, optional)
        Maximum tool invocations per task.
        Range: 1-1000
        Default: None
        Prevents runaway tool invocation chains.

6. HEALTH CHECK SECTION [health_check] (optional)

    type (string, optional)
        Type of health check to perform.
        Values: "http", "tcp", "exec", "csci"
        Default: "http"
        Determines probe mechanism.

    endpoint (string, optional)
        Health check endpoint or command.
        Format depends on type:
          - http: URL (e.g., "http://localhost:8080/health")
          - tcp: "host:port" (e.g., "localhost:8080")
          - exec: command (e.g., "/usr/bin/healthcheck")
          - csci: CSCI syscall identifier
        Default: None

    interval_ms (u64, optional)
        Probe interval in milliseconds.
        Range: 100-3600000
        Default: 10000 (10 seconds)
        Constraint: Must be > timeout_ms

    timeout_ms (u64, optional)
        Probe timeout in milliseconds.
        Range: 10-60000
        Default: 5000 (5 seconds)
        Constraint: Must be < interval_ms

    failure_threshold (u32, optional)
        Consecutive failures before marking unhealthy.
        Range: 1-100
        Default: 3

    success_threshold (u32, optional)
        Consecutive successes before marking healthy.
        Range: 1-100
        Default: 1

7. RESTART SECTION [restart] (optional)

    policy (string, optional)
        Restart policy type.
        Values: "always", "on_failure", "never"
        Default: "on_failure"

    max_retries (u32, optional)
        Maximum number of restart attempts.
        Range: 0-1000
        Default: 5
        Constraint: Must be > 0 for on_failure policy.

    backoff_base_ms (u64, optional)
        Base backoff delay in milliseconds.
        Range: 10-60000
        Default: 100
        Used for exponential backoff calculation.

    backoff_multiplier (f32, optional)
        Exponential backoff multiplier.
        Range: 1.0-10.0
        Default: 2.0
        Next delay = current_delay * multiplier

    max_backoff_ms (u64, optional)
        Maximum backoff delay in milliseconds.
        Range: 100-3600000
        Default: 30000 (30 seconds)
        Caps exponential backoff growth.

8. DEPENDENCIES SECTION [dependencies] (optional)

    after (array of strings, optional)
        Agents that must start after this agent.
        Example: ["cache", "database"]
        Used to establish startup ordering.

    before (array of strings, optional)
        Agents that must start before this agent.
        Example: ["load-balancer"]
        Alternative way to express ordering.

    requires (array of strings, optional)
        External services that must be available.
        Example: ["postgresql", "redis", "opensearch"]
        Services not managed by this deployment.

9. CREW SECTION [crew] (optional)

    name (string, optional)
        Crew identifier if agent belongs to a crew.
        Pattern: Same as agent name
        Example: "data-processing-crew"

    role (string, optional)
        Agent's role within the crew.
        Values: "coordinator", "worker", "specialist", "observer"
        Default: "member"

    ordering_priority (u32, optional)
        Start order within crew (lower = earlier).
        Range: 0-1000
        Default: 100
        Used for coordinated crew startup.

10. ENVIRONMENT SECTION [environment] (optional)

    Arbitrary key=value pairs for agent configuration.
    Keys must be valid POSIX environment variable names.
    Values can be any string (will be converted to strings).
    Example:
        LOG_LEVEL = "info"
        ENABLE_DEBUG = "false"
        MAX_RETRIES = "5"

CONSTRAINTS

Global constraints that apply across all sections:

1. Names and identifiers must match: ^[a-z][a-z0-9]*(-[a-z0-9]+)*$
2. Numeric ranges must be enforced at validation time
3. If health_check.timeout_ms is specified, it must be < interval_ms
4. If max_gpu_ms is specified, it must be < max_wall_clock_ms
5. For on_failure restart policy, max_retries must be > 0
6. Dependency graph must be acyclic (validated at deployment)
7. All timestamps are in milliseconds (ms)
8. All memory sizes are in bytes
9. All token counts are unbounded integers (u32/u64 as appropriate)
"#;

/// Backward Compatibility Rules
///
/// Documents deprecation timeline and migration path for format changes.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Backward Compatibility
pub const RFC_BACKWARD_COMPATIBILITY: &str = r#"
BACKWARD COMPATIBILITY

Legacy Fields (Deprecated v2.0, Removed v3.0)

The following fields are deprecated in favor of the [resources] section but
are still supported for backward compatibility:

    memory_mb (u64, deprecated)
        DEPRECATED: Use max_memory_bytes in [resources] instead.
        Conversion: max_memory_bytes = memory_mb * 1024 * 1024
        Timeline: Supported until v3.0, warning in v2.x

    cpu_cores (f64, deprecated)
        DEPRECATED: No direct replacement (resource limits are advisory).
        Timeline: Supported until v3.0, warning in v2.x
        Note: Not enforced by runtime, only for documentation.

Framework Detection

For agents without explicit framework specification:
- If model.provider is specified → assume LangChain
- If crew.name is specified → assume CrewAI
- Otherwise → framework agnostic

Migration Guide

From v0.1 (environment-based) to v1.0 (unit files):

1. Extract all env vars into [environment] section
2. Move memory_mb to [resources].max_memory_bytes
3. Move cpu_cores to documentation (not enforced)
4. Explicit dependency specifications in [dependencies]
5. Health checks move to [health_check]
6. Restart policies move to [restart]

Version Compatibility

- v1.x agents can be deployed alongside v1.y agents (y > x)
- v1.x agents should not coexist with v2.x agents
- v2.x may break v1.x format (with deprecation period)
- All format versions are documented in comments

Update Strategy

1. Add new field as optional with default behavior
2. Maintain old field for at least 2 minor versions
3. Issue warnings when deprecated fields are used
4. Remove in next major version
5. Document migration path clearly
"#;

/// Security Considerations
///
/// Addresses security implications of the format and runtime.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Security
pub const RFC_SECURITY: &str = r#"
SECURITY CONSIDERATIONS

1. CAPABILITY-BASED SECURITY

Capabilities restrict what agents can do:
- Explicit allow-list model: Agents have no capabilities by default
- Fine-grained control: Separate capabilities for read vs. write
- Crew isolation: Crew members cannot exceed crew's collective capabilities
- Audit trail: All capability grants are logged and auditable

2. RESOURCE LIMITS AS DOS PREVENTION

Resource limits prevent denial-of-service:
- max_tokens_per_task: Prevents token explosion attacks
- max_gpu_ms: Prevents GPU hogging
- max_wall_clock_ms: Prevents long-running computations
- max_memory_bytes: Prevents memory exhaustion
- max_tool_calls: Prevents API call floods

3. DEPENDENCY VALIDATION

Dependency validation prevents deployment issues:
- Acyclic check: Prevents deadlock situations
- Service existence check: Required services must be available
- Circular dependency detection: Prevents startup hangs

4. MODEL CREDENTIAL SECURITY

Model credentials are never stored in unit files:
- provider field specifies provider, not credentials
- Credentials come from secure secret management
- API keys injected at runtime via [environment]
- Unit files can be safely version controlled

5. ENVIRONMENT VARIABLE HANDLING

Environment variables require careful handling:
- Secrets should be injected from secret managers
- Sensitive values should not appear in version control
- Pre-deployment checks can flag suspicious values
- Default values should not be secrets

6. HEALTH CHECK ENDPOINT VALIDATION

Health check endpoints must be validated:
- HTTP endpoints must use verified HTTPS in production
- TCP health checks should use private network
- Exec health checks must reference available binaries
- CSCI syscalls must be validated against security policies

7. RESTART POLICY LIMITS

Restart policies prevent resource exhaustion:
- max_retries limits restart attempts
- backoff prevents thundering herd
- max_backoff_ms provides upper bound on delay

Recommendations:
- Use "on_failure" with reasonable max_retries
- Never use "always" without max_retries limits
- Monitor restart events for anomalies

8. FILE PERMISSIONS

Unit files should have appropriate permissions:
- Unit files: 644 (world-readable, owner-writable)
- Credentials (if any): 600 (owner-readable only)
- Deployment directories: 755 (world-readable, owner-writable)

9. VALIDATION BEFORE DEPLOYMENT

Pre-deployment validation must include:
- Schema validation (all required fields present)
- Constraint validation (values within bounds)
- Dependency cycle detection
- Capability validity checks
- Resource limit reasonableness
- Model provider accessibility
"#;

/// Field Descriptions for Documentation
///
/// Detailed descriptions for each field, suitable for tool-generated docs.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Field Reference
pub const FIELD_DESCRIPTIONS: &[(&str, &str)] = &[
    ("agent.name", "Unique agent identifier (required)"),
    ("agent.version", "Semantic version following SemVer 2.0.0 (required)"),
    ("agent.description", "Human-readable description of agent purpose (required)"),
    ("agent.framework", "Agent framework: langchain, semantic_kernel, crewai, autogen, custom"),
    ("agent.author", "Team or person responsible for this agent"),
    ("agent.tags", "Classification tags for discovery and organization"),
    ("model.provider", "LLM provider: openai, anthropic, ollama, local, custom"),
    ("model.model_name", "Specific model identifier (e.g., gpt-4, claude-opus-4.6)"),
    ("model.max_tokens", "Maximum tokens for model completion (1-1000000)"),
    ("model.temperature", "Sampling temperature (0.0-2.0, 0=deterministic, 1=balanced)"),
    ("model.context_window", "Effective context window in tokens"),
    ("capabilities.required", "Capabilities that MUST be granted"),
    ("capabilities.optional", "Capabilities that MAY be granted"),
    ("resources.max_tokens_per_task", "Maximum tokens per task execution (1-100000)"),
    ("resources.max_gpu_ms", "Maximum GPU time in milliseconds (1-3600000)"),
    ("resources.max_wall_clock_ms", "Maximum execution time in milliseconds (1-86400000)"),
    ("resources.max_memory_bytes", "Maximum memory in bytes (1-17179869184)"),
    ("resources.max_tool_calls", "Maximum tool invocations per task (1-1000)"),
    ("health_check.type", "Health check type: http, tcp, exec, csci"),
    ("health_check.endpoint", "Health check endpoint or command"),
    ("health_check.interval_ms", "Probe interval in milliseconds (100-3600000, default 10000)"),
    ("health_check.timeout_ms", "Probe timeout in milliseconds (10-60000, default 5000)"),
    ("health_check.failure_threshold", "Failures before unhealthy (1-100, default 3)"),
    ("health_check.success_threshold", "Successes before healthy (1-100, default 1)"),
    ("restart.policy", "Restart policy: always, on_failure, never (default on_failure)"),
    ("restart.max_retries", "Maximum restart attempts (0-1000, default 5)"),
    ("restart.backoff_base_ms", "Base backoff delay in milliseconds (10-60000, default 100)"),
    ("restart.backoff_multiplier", "Exponential backoff multiplier (1.0-10.0, default 2.0)"),
    ("restart.max_backoff_ms", "Maximum backoff delay in milliseconds (100-3600000, default 30000)"),
    ("dependencies.after", "Agents that must start after this agent"),
    ("dependencies.before", "Agents that must start before this agent"),
    ("dependencies.requires", "External services that must be available"),
    ("crew.name", "Crew identifier if agent belongs to a crew"),
    ("crew.role", "Agent role: coordinator, worker, specialist, observer"),
    ("crew.ordering_priority", "Start order within crew (0-1000, lower=earlier, default 100)"),
];

/// Default Values Reference
///
/// Authoritative source for all default values in the format.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC § Defaults
pub const DEFAULTS: &[(&str, &str)] = &[
    ("health_check.type", "http"),
    ("health_check.interval_ms", "10000"),
    ("health_check.timeout_ms", "5000"),
    ("health_check.failure_threshold", "3"),
    ("health_check.success_threshold", "1"),
    ("restart.policy", "on_failure"),
    ("restart.max_retries", "5"),
    ("restart.backoff_base_ms", "100"),
    ("restart.backoff_multiplier", "2.0"),
    ("restart.max_backoff_ms", "30000"),
    ("crew.role", "member"),
    ("crew.ordering_priority", "100"),
];

/// Complete RFC specification document
///
/// Combines all sections into a cohesive RFC document.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § RFC
pub fn complete_rfc() -> String {
    alloc::format!(
        r#"
COGNITIVE SUBSTRATE AGENT UNIT FILE SPECIFICATION
RFC 2026-001

{}

{}

{}

{}

{}

---
Field Descriptions:

{}

---
Default Values:

{}
"#,
        RFC_ABSTRACT,
        RFC_MOTIVATION,
        RFC_SPECIFICATION,
        RFC_SECURITY,
        RFC_BACKWARD_COMPATIBILITY,
        FIELD_DESCRIPTIONS
            .iter()
            .map(|(field, desc)| alloc::format!("{}: {}", field, desc))
            .collect::<Vec<_>>()
            .join("\n"),
        DEFAULTS
            .iter()
            .map(|(field, value)| alloc::format!("{} = {}", field, value))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::vec::Vec;

    #[test]
    fn test_rfc_abstract_not_empty() {
        assert!(!RFC_ABSTRACT.is_empty());
        assert!(RFC_ABSTRACT.contains("Abstract"));
    }

    #[test]
    fn test_rfc_motivation_not_empty() {
        assert!(!RFC_MOTIVATION.is_empty());
        assert!(RFC_MOTIVATION.contains("MOTIVATION"));
    }

    #[test]
    fn test_rfc_specification_not_empty() {
        assert!(!RFC_SPECIFICATION.is_empty());
        assert!(RFC_SPECIFICATION.contains("AGENT SECTION"));
    }

    #[test]
    fn test_rfc_security_not_empty() {
        assert!(!RFC_SECURITY.is_empty());
        assert!(RFC_SECURITY.contains("SECURITY CONSIDERATIONS"));
    }

    #[test]
    fn test_field_descriptions_populated() {
        assert!(!FIELD_DESCRIPTIONS.is_empty());
        assert!(FIELD_DESCRIPTIONS.len() > 10);
    }

    #[test]
    fn test_defaults_populated() {
        assert!(!DEFAULTS.is_empty());
        assert!(DEFAULTS.iter().any(|(k, _)| k.contains("policy")));
    }

    #[test]
    fn test_complete_rfc_generates() {
        let rfc = complete_rfc();
        assert!(!rfc.is_empty());
        assert!(rfc.contains("AGENT SECTION"));
        assert!(rfc.contains("SECURITY"));
    }

    #[test]
    fn test_field_descriptions_have_agent_fields() {
        let names: Vec<_> = FIELD_DESCRIPTIONS
            .iter()
            .filter(|(k, _)| k.starts_with("agent."))
            .collect();
        assert!(!names.is_empty());
    }

    #[test]
    fn test_defaults_have_timeouts() {
        let timeout_defaults: Vec<_> = DEFAULTS
            .iter()
            .filter(|(k, _)| k.contains("ms"))
            .collect();
        assert!(!timeout_defaults.is_empty());
    }
}
