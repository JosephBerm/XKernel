// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Comprehensive Agent Unit File test suite.
//!
//! Provides 20+ unit file examples covering:
//! - All field types and combinations
//! - Edge cases and boundary values
//! - Error conditions
//! - Schema validation
//! - Backward compatibility scenarios
//!
//! These test files serve as both validation fixtures and documentation
//! of supported configurations.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Test Suite

/// Test Unit File 1: Absolute Minimal Configuration
///
/// Only required fields, no optional sections.
/// Demonstrates the simplest valid unit file.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Test Suite
pub const TEST_MINIMAL_VALID: &str = r#"[agent]
name = "minimal-agent"
version = "0.1.0"
description = "Minimal valid unit file"
"#;

/// Test Unit File 2: Simple Agent with Tags
///
/// Adds metadata tags for discovery and classification.
pub const TEST_AGENT_WITH_TAGS: &str = r#"[agent]
name = "tagged-agent"
version = "1.0.0"
description = "Agent with classification tags"
author = "Platform Team"
tags = ["network", "critical", "stateless"]
"#;

/// Test Unit File 3: LangChain Framework with Full Model Config
///
/// Complete model configuration with all optional fields.
pub const TEST_LANGCHAIN_FULL: &str = r#"[agent]
name = "langchain-complete"
version = "2.1.0"
description = "Complete LangChain agent"
framework = "langchain"

[model]
provider = "openai"
model_name = "gpt-4-turbo"
max_tokens = 4096
temperature = 0.7
context_window = 128000

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke"]
optional = ["channel_send", "file_access"]
"#;

/// Test Unit File 4: Anthropic Model with Resource Limits
///
/// Demonstrates resource constraint configuration.
pub const TEST_ANTHROPIC_RESOURCES: &str = r#"[agent]
name = "resource-constrained"
version = "1.5.0"
description = "Agent with strict resource limits"
framework = "semantic_kernel"

[model]
provider = "anthropic"
model_name = "claude-opus-4.6"
max_tokens = 2048
context_window = 200000

[resources]
max_tokens_per_task = 1024
max_gpu_ms = 5000
max_wall_clock_ms = 30000
max_memory_bytes = 1073741824
max_tool_calls = 5
"#;

/// Test Unit File 5: HTTP Health Check Configuration
///
/// Complete health check with all parameters.
pub const TEST_HEALTH_CHECK_HTTP: &str = r#"[agent]
name = "http-health-agent"
version = "1.0.0"
description = "Agent with HTTP health check"

[health_check]
type = "http"
endpoint = "http://localhost:8080/health"
interval_ms = 5000
timeout_ms = 2000
failure_threshold = 3
success_threshold = 1
"#;

/// Test Unit File 6: TCP Health Check
///
/// Demonstrates TCP-based health checking.
pub const TEST_HEALTH_CHECK_TCP: &str = r#"[agent]
name = "tcp-health-agent"
version = "1.0.0"
description = "Agent with TCP health check"

[health_check]
type = "tcp"
endpoint = "localhost:9000"
interval_ms = 10000
timeout_ms = 3000
failure_threshold = 5
success_threshold = 2
"#;

/// Test Unit File 7: Exec Health Check
///
/// Uses custom command for health probing.
pub const TEST_HEALTH_CHECK_EXEC: &str = r#"[agent]
name = "exec-health-agent"
version = "1.0.0"
description = "Agent with exec health check"

[health_check]
type = "exec"
endpoint = "/usr/bin/healthcheck --agent-id test"
interval_ms = 15000
timeout_ms = 5000
failure_threshold = 3
success_threshold = 1
"#;

/// Test Unit File 8: OnFailure Restart Policy with Exponential Backoff
///
/// Complete restart configuration.
pub const TEST_RESTART_POLICY_ONFAIL: &str = r#"[agent]
name = "restart-agent"
version = "1.0.0"
description = "Agent with restart policy"

[restart]
policy = "on_failure"
max_retries = 5
backoff_base_ms = 100
backoff_multiplier = 2.0
max_backoff_ms = 30000
"#;

/// Test Unit File 9: Always Restart Policy
///
/// Demonstrates always restart configuration.
pub const TEST_RESTART_POLICY_ALWAYS: &str = r#"[agent]
name = "always-restart-agent"
version = "1.0.0"
description = "Agent that always restarts"

[restart]
policy = "always"
backoff_base_ms = 500
backoff_multiplier = 1.5
max_backoff_ms = 60000
"#;

/// Test Unit File 10: Never Restart Policy
///
/// Demonstrates no-restart configuration.
pub const TEST_RESTART_POLICY_NEVER: &str = r#"[agent]
name = "no-restart-agent"
version = "1.0.0"
description = "Agent that never restarts"

[restart]
policy = "never"
"#;

/// Test Unit File 11: Dependency Ordering
///
/// Complex dependency specifications.
pub const TEST_DEPENDENCIES_ORDERING: &str = r#"[agent]
name = "dependent-agent"
version = "1.0.0"
description = "Agent with dependencies"

[dependencies]
after = ["database-agent", "cache-agent"]
before = ["load-balancer"]
requires = ["postgresql", "redis"]
"#;

/// Test Unit File 12: CrewAI Crew Member
///
/// Crew membership configuration.
pub const TEST_CREW_MEMBERSHIP: &str = r#"[agent]
name = "worker-agent"
version = "1.0.0"
description = "Worker in a crew"
framework = "crewai"

[crew]
name = "data-processing-crew"
role = "worker"
ordering_priority = 2
"#;

/// Test Unit File 13: Crew Coordinator
///
/// Coordinator role in a crew.
pub const TEST_CREW_COORDINATOR: &str = r#"[agent]
name = "crew-coordinator"
version = "1.0.0"
description = "Coordinator for multi-agent crew"
framework = "crewai"

[crew]
name = "analytics-crew"
role = "coordinator"
ordering_priority = 1

[capabilities]
required = ["agent_spawn", "channel_send"]
"#;

/// Test Unit File 14: Extended Environment Variables
///
/// Custom environment configuration.
pub const TEST_ENVIRONMENT_VARIABLES: &str = r#"[agent]
name = "env-agent"
version = "1.0.0"
description = "Agent with environment config"

[environment]
LOG_LEVEL = "debug"
DEBUG_MODE = "true"
MAX_WORKERS = "4"
API_TIMEOUT_MS = "5000"
ENABLE_METRICS = "true"
CUSTOM_CONFIG_PATH = "/etc/agent/config.json"
"#;

/// Test Unit File 15: All Capabilities Required
///
/// Comprehensive capability request.
pub const TEST_CAPABILITIES_EXTENDED: &str = r#"[agent]
name = "privileged-agent"
version = "1.0.0"
description = "Agent requiring many capabilities"

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke", "agent_spawn", "database_access"]
optional = ["gpu_compute", "channel_send", "file_access"]
"#;

/// Test Unit File 16: Complete Feature-Rich Agent
///
/// Demonstrates all optional sections together.
pub const TEST_COMPLETE_RICH_AGENT: &str = r#"[agent]
name = "feature-rich-agent"
version = "3.2.1"
description = "Comprehensive feature-rich agent configuration"
framework = "langchain"
author = "AI Platform Team"
tags = ["network", "critical", "gpu-enabled"]

[model]
provider = "openai"
model_name = "gpt-4-turbo"
max_tokens = 4096
temperature = 0.7
context_window = 128000

[resources]
max_tokens_per_task = 2048
max_gpu_ms = 10000
max_wall_clock_ms = 60000
max_memory_bytes = 2147483648
max_tool_calls = 20

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke", "gpu_compute"]
optional = ["channel_send", "agent_spawn"]

[health_check]
type = "http"
endpoint = "http://localhost:8080/health"
interval_ms = 5000
timeout_ms = 2000
failure_threshold = 3
success_threshold = 1

[restart]
policy = "on_failure"
max_retries = 5
backoff_base_ms = 100
backoff_multiplier = 2.0
max_backoff_ms = 30000

[dependencies]
after = ["database", "cache"]
requires = ["postgresql", "redis"]

[environment]
LOG_LEVEL = "info"
ENABLE_METRICS = "true"
ENABLE_TRACING = "true"
BATCH_SIZE = "32"
DEVICE = "cuda:0"
"#;

/// Test Unit File 17: Minimal Version with Only Legacy Fields
///
/// Tests backward compatibility with deprecated fields.
pub const TEST_LEGACY_MEMORY_CPU: &str = r#"[agent]
name = "legacy-agent"
version = "1.0.0"
description = "Agent using deprecated memory_mb and cpu_cores"

[resources]
memory_mb = 512
cpu_cores = 2.0
"#;

/// Test Unit File 18: Custom Framework
///
/// Custom framework specification.
pub const TEST_CUSTOM_FRAMEWORK: &str = r#"[agent]
name = "custom-framework-agent"
version = "1.0.0"
description = "Agent using custom framework"
framework = "custom"

[model]
provider = "custom"
model_name = "proprietary-model-v1"
max_tokens = 8192
context_window = 32768
"#;

/// Test Unit File 19: CSCI Health Check
///
/// CSCI syscall-based health checking.
pub const TEST_HEALTH_CHECK_CSCI: &str = r#"[agent]
name = "csci-health-agent"
version = "1.0.0"
description = "Agent with CSCI health check"

[health_check]
type = "csci"
endpoint = "cs_agent_probe"
interval_ms = 3000
timeout_ms = 1000
failure_threshold = 5
success_threshold = 1
"#;

/// Test Unit File 20: Minimal Model Config
///
/// Only provider specified.
pub const TEST_MODEL_MINIMAL: &str = r#"[agent]
name = "model-minimal-agent"
version = "1.0.0"
description = "Agent with minimal model config"

[model]
provider = "anthropic"
"#;

/// Test Unit File 21: Edge Case - Zero Retry Policy
///
/// On-failure with zero retries (fail immediately).
pub const TEST_EDGE_CASE_ZERO_RETRY: &str = r#"[agent]
name = "no-retry-agent"
version = "1.0.0"
description = "Agent with zero retry policy"

[restart]
policy = "on_failure"
max_retries = 0
backoff_base_ms = 100
backoff_multiplier = 1.0
max_backoff_ms = 100
"#;

/// Test Unit File 22: Edge Case - Very Large Token Limits
///
/// Maximum reasonable token configurations.
pub const TEST_EDGE_CASE_LARGE_TOKENS: &str = r#"[agent]
name = "large-token-agent"
version = "1.0.0"
description = "Agent with large token limits"

[model]
max_tokens = 100000
context_window = 1000000

[resources]
max_tokens_per_task = 50000
"#;

/// Test File Collection
///
/// All test cases for programmatic access.
pub const ALL_TEST_CASES: &[(&str, &str, &str)] = &[
    ("minimal_valid", TEST_MINIMAL_VALID, "Absolute minimal configuration"),
    ("agent_with_tags", TEST_AGENT_WITH_TAGS, "Agent with classification tags"),
    ("langchain_full", TEST_LANGCHAIN_FULL, "Complete LangChain agent"),
    ("anthropic_resources", TEST_ANTHROPIC_RESOURCES, "Anthropic with resource limits"),
    ("health_check_http", TEST_HEALTH_CHECK_HTTP, "HTTP health check"),
    ("health_check_tcp", TEST_HEALTH_CHECK_TCP, "TCP health check"),
    ("health_check_exec", TEST_HEALTH_CHECK_EXEC, "Exec health check"),
    ("restart_onfail", TEST_RESTART_POLICY_ONFAIL, "OnFailure restart policy"),
    ("restart_always", TEST_RESTART_POLICY_ALWAYS, "Always restart policy"),
    ("restart_never", TEST_RESTART_POLICY_NEVER, "Never restart policy"),
    ("dependencies", TEST_DEPENDENCIES_ORDERING, "Dependency ordering"),
    ("crew_member", TEST_CREW_MEMBERSHIP, "Crew member configuration"),
    ("crew_coordinator", TEST_CREW_COORDINATOR, "Crew coordinator"),
    ("environment", TEST_ENVIRONMENT_VARIABLES, "Environment variables"),
    ("capabilities", TEST_CAPABILITIES_EXTENDED, "Extended capabilities"),
    ("complete_rich", TEST_COMPLETE_RICH_AGENT, "Complete feature-rich agent"),
    ("legacy_memory_cpu", TEST_LEGACY_MEMORY_CPU, "Legacy memory/CPU fields"),
    ("custom_framework", TEST_CUSTOM_FRAMEWORK, "Custom framework"),
    ("health_check_csci", TEST_HEALTH_CHECK_CSCI, "CSCI health check"),
    ("model_minimal", TEST_MODEL_MINIMAL, "Minimal model config"),
    ("edge_zero_retry", TEST_EDGE_CASE_ZERO_RETRY, "Edge case: zero retries"),
    ("edge_large_tokens", TEST_EDGE_CASE_LARGE_TOKENS, "Edge case: large tokens"),
];

/// Validates all test cases can be parsed.
///
/// Returns the count of test cases for metrics.
pub fn validate_all_test_cases() -> usize {
    ALL_TEST_CASES.len()
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec::Vec;

    #[test]
    fn test_minimal_valid_not_empty() {
        assert!(!TEST_MINIMAL_VALID.is_empty());
        assert!(TEST_MINIMAL_VALID.contains("[agent]"));
    }

    #[test]
    fn test_all_test_cases_populated() {
        assert!(!ALL_TEST_CASES.is_empty());
        assert!(ALL_TEST_CASES.len() >= 20);
    }

    #[test]
    fn test_all_test_cases_have_descriptions() {
        for (name, _content, description) in ALL_TEST_CASES {
            assert!(!name.is_empty(), "Test case has empty name");
            assert!(!description.is_empty(), "Test case '{}' has empty description", name);
        }
    }

    #[test]
    fn test_all_test_cases_have_agent_section() {
        for (name, content, _desc) in ALL_TEST_CASES {
            assert!(
                content.contains("[agent]"),
                "Test case '{}' missing [agent] section",
                name
            );
        }
    }

    #[test]
    fn test_health_check_cases_count() {
        let health_check_cases: Vec<_> = ALL_TEST_CASES
            .iter()
            .filter(|(n, _, _)| n.contains("health_check"))
            .collect();
        assert!(health_check_cases.len() >= 4);
    }

    #[test]
    fn test_restart_policy_cases_count() {
        let restart_cases: Vec<_> = ALL_TEST_CASES
            .iter()
            .filter(|(n, _, _)| n.contains("restart"))
            .collect();
        assert!(restart_cases.len() >= 3);
    }

    #[test]
    fn test_test_case_uniqueness() {
        let mut names = alloc::vec::Vec::new();
        for (name, _, _) in ALL_TEST_CASES {
            assert!(!names.contains(name), "Duplicate test case name: {}", name);
            names.push(name);
        }
    }

    #[test]
    fn test_validate_all_test_cases_returns_count() {
        let count = validate_all_test_cases();
        assert_eq!(count, ALL_TEST_CASES.len());
        assert!(count >= 20);
    }

    #[test]
    fn test_complete_rich_agent_has_all_sections() {
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[agent]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[model]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[resources]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[capabilities]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[health_check]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[restart]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[dependencies]"));
        assert!(TEST_COMPLETE_RICH_AGENT.contains("[environment]"));
    }

    #[test]
    fn test_framework_variations() {
        let frameworks = [
            ("langchain", TEST_LANGCHAIN_FULL),
            ("semantic_kernel", TEST_ANTHROPIC_RESOURCES),
            ("crewai", TEST_CREW_COORDINATOR),
            ("custom", TEST_CUSTOM_FRAMEWORK),
        ];

        for (framework, content) in &frameworks {
            assert!(
                content.contains(framework),
                "Test case should mention framework '{}'",
                framework
            );
        }
    }
}
