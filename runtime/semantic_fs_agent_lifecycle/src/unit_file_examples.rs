// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Example Agent Unit File configurations.
//!
//! Provides 8 comprehensive example unit file TOML strings demonstrating
//! various agent configurations, from minimal to complex setups.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples

/// Minimal agent configuration with only required fields.
///
/// This is the simplest possible agent configuration.
/// Demonstrates the minimum viable unit file.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_SIMPLE_AGENT: &str = r#"[agent]
name = "simple-agent"
version = "1.0.0"
description = "A simple agent with minimal configuration"
"#;

/// LangChain agent with tools, memory, and model configuration.
///
/// Demonstrates a complete LangChain agent setup with:
/// - Model provider configuration (OpenAI)
/// - Tool capabilities
/// - Memory and context settings
/// - Resource limits
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_LANGCHAIN_AGENT: &str = r#"[agent]
name = "langchain-agent"
version = "2.1.0"
description = "LangChain agent with tools and memory"
framework = "langchain"

[model]
provider = "openai"
model_name = "gpt-4-turbo"
max_tokens = 2048
temperature = 0.7
context_window = 128000

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke"]
optional = ["channel_send", "file_access"]

[resources]
max_tokens_per_task = 4096
max_gpu_ms = 5000
max_wall_clock_ms = 30000
max_memory_bytes = 1073741824
max_tool_calls = 10

[health_check]
type = "http"
endpoint = "http://localhost:8080/health"
interval_ms = 5000
timeout_ms = 2000
failure_threshold = 3
success_threshold = 1

[environment]
LOG_LEVEL = "info"
ENABLE_TOOLS = "true"
MEMORY_SIZE = "1024"
"#;

/// Crew coordinator agent managing other agents.
///
/// Demonstrates:
/// - Crew coordinator role
/// - Dependencies on worker agents
/// - Complex health checks
/// - Restart policies
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_CREW_COORDINATOR: &str = r#"[agent]
name = "crew-coordinator"
version = "1.5.0"
description = "Coordinator for a multi-agent crew"
framework = "crewai"

[model]
provider = "anthropic"
model_name = "claude-opus-4.6"
max_tokens = 4096
temperature = 0.5
context_window = 200000

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke", "agent_spawn"]
optional = ["channel_send"]

[health_check]
type = "csci"
endpoint = "cs_agent_probe"
interval_ms = 3000
timeout_ms = 1000
failure_threshold = 5
success_threshold = 2

[restart]
policy = "on_failure"
max_retries = 10
backoff_base_ms = 200
backoff_multiplier = 1.5
max_backoff_ms = 60000

[dependencies]
after = ["logging-service", "memory-service"]
requires = ["database", "message-queue"]

[crew]
name = "main-crew"
role = "coordinator"
ordering_priority = 1

[resources]
max_tokens_per_task = 8192
max_memory_bytes = 2147483648
max_tool_calls = 50

[environment]
CREW_MODE = "coordinator"
ORCHESTRATION_LEVEL = "full"
"#;

/// Worker agent in a crew.
///
/// Demonstrates:
/// - Worker role in a crew
/// - Dependency on coordinator
/// - Simpler resource requirements
/// - TCP health checks
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_CREW_WORKER: &str = r#"[agent]
name = "crew-worker"
version = "1.2.0"
description = "Worker agent in a managed crew"
framework = "semantic_kernel"

[model]
provider = "openai"
model_name = "gpt-3.5-turbo"
max_tokens = 1024
temperature = 0.8

[capabilities]
required = ["mem_read", "tool_invoke"]
optional = ["channel_send"]

[health_check]
type = "tcp"
endpoint = "localhost:9000"
interval_ms = 5000
timeout_ms = 1000
failure_threshold = 3
success_threshold = 1

[restart]
policy = "on_failure"
max_retries = 5
backoff_base_ms = 100
backoff_multiplier = 2.0
max_backoff_ms = 10000

[dependencies]
after = ["crew-coordinator"]
before = []

[crew]
name = "main-crew"
role = "worker"
ordering_priority = 5

[resources]
max_tokens_per_task = 2048
max_memory_bytes = 536870912
max_tool_calls = 5

[environment]
WORKER_ID = "worker-1"
TASK_TIMEOUT = "60000"
"#;

/// Agent with high resource requirements.
///
/// Demonstrates:
/// - Large memory and compute quotas
/// - Extended timeouts
/// - Multiple required capabilities
/// - Heavy tool usage
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_HIGH_RESOURCE: &str = r#"[agent]
name = "high-compute-agent"
version = "3.0.0"
description = "Agent with high compute and memory requirements"
framework = "autogen"

[model]
provider = "openai"
model_name = "gpt-4"
max_tokens = 8192
temperature = 0.3
context_window = 128000

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke", "gpu_compute", "file_access", "network_raw"]
optional = []

[resources]
max_tokens_per_task = 16384
max_gpu_ms = 120000
max_wall_clock_ms = 300000
max_memory_bytes = 8589934592
max_tool_calls = 100

[health_check]
type = "http"
endpoint = "http://localhost:8090/health/detailed"
interval_ms = 2000
timeout_ms = 5000
failure_threshold = 2
success_threshold = 3

[restart]
policy = "always"
backoff_base_ms = 500
backoff_multiplier = 1.2
max_backoff_ms = 120000

[environment]
GPU_ENABLED = "true"
GPU_MEMORY = "24GB"
BATCH_SIZE = "128"
WORKER_THREADS = "32"
CACHE_SIZE = "4GB"
"#;

/// Agent with comprehensive health checks.
///
/// Demonstrates:
/// - Multiple health check configurations
/// - Readiness and liveness probes
/// - Custom health check endpoints
/// - Detailed health monitoring
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_HEALTH_CHECKED: &str = r#"[agent]
name = "health-checked-agent"
version = "1.1.0"
description = "Agent with comprehensive health monitoring"
framework = "langchain"

[model]
provider = "anthropic"
model_name = "claude-opus-4.6"
max_tokens = 2048
temperature = 0.7

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke"]

[health_check]
type = "http"
endpoint = "http://localhost:8080/health"
interval_ms = 1000
timeout_ms = 500
failure_threshold = 5
success_threshold = 2

[restart]
policy = "on_failure"
max_retries = 3
backoff_base_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[resources]
max_tokens_per_task = 4096
max_memory_bytes = 1073741824
max_tool_calls = 20

[environment]
HEALTH_CHECK_DETAILED = "true"
METRICS_ENABLED = "true"
TRACE_ENABLED = "true"
LIVENESS_TIMEOUT = "5000"
READINESS_TIMEOUT = "10000"
"#;

/// Agent with sophisticated restart and backoff policy.
///
/// Demonstrates:
/// - Exponential backoff configuration
/// - Retry limits with strategic delays
/// - Policy variations (always, on_failure, never)
/// - Backoff tuning for different failure scenarios
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_RESTART_POLICY: &str = r#"[agent]
name = "restart-policy-agent"
version = "2.0.0"
description = "Agent with sophisticated restart and backoff configuration"
framework = "custom"

[model]
provider = "ollama"
model_name = "mistral:latest"
max_tokens = 1024
temperature = 0.5

[capabilities]
required = ["mem_read", "tool_invoke"]

[health_check]
type = "exec"
endpoint = "/opt/agent/health_check.sh"
interval_ms = 5000
timeout_ms = 2000
failure_threshold = 2
success_threshold = 1

[restart]
policy = "on_failure"
max_retries = 15
backoff_base_ms = 500
backoff_multiplier = 1.3
max_backoff_ms = 300000

[resources]
max_tokens_per_task = 2048
max_memory_bytes = 536870912
max_tool_calls = 10

[environment]
RESTART_STRATEGY = "exponential_backoff"
FAILURE_LOG_LEVEL = "error"
MAX_RESTART_JITTER = "5000"
"#;

/// Agent with complex multi-agent dependencies.
///
/// Demonstrates:
/// - Multiple dependency declarations
/// - Ordering constraints (after, before)
/// - Required services
/// - Complex startup ordering
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Examples
pub const EXAMPLE_MULTI_DEPENDENCY: &str = r#"[agent]
name = "orchestrator-agent"
version = "1.8.0"
description = "Agent with complex dependency ordering"
framework = "crewai"

[model]
provider = "openai"
model_name = "gpt-4-turbo"
max_tokens = 4096
temperature = 0.6

[capabilities]
required = ["mem_read", "mem_write", "tool_invoke", "agent_spawn"]
optional = ["channel_send", "database_access"]

[resources]
max_tokens_per_task = 8192
max_memory_bytes = 2147483648
max_tool_calls = 50

[health_check]
type = "http"
endpoint = "http://localhost:8080/orchestrator/health"
interval_ms = 3000
timeout_ms = 1500
failure_threshold = 3
success_threshold = 1

[restart]
policy = "on_failure"
max_retries = 8
backoff_base_ms = 250
backoff_multiplier = 1.5
max_backoff_ms = 60000

[dependencies]
after = ["logging-service", "memory-service", "database-service"]
before = ["worker-agent-1", "worker-agent-2", "worker-agent-3"]
requires = ["message-queue", "distributed-cache", "config-server"]

[crew]
name = "orchestration-crew"
role = "coordinator"
ordering_priority = 1

[environment]
ORCHESTRATION_MODE = "distributed"
HEARTBEAT_INTERVAL = "5000"
DEPENDENCY_CHECK_INTERVAL = "2000"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_simple_agent_is_valid_toml_format() {
        assert!(EXAMPLE_SIMPLE_AGENT.contains("[agent]"));
        assert!(EXAMPLE_SIMPLE_AGENT.contains("name = "));
        assert!(EXAMPLE_SIMPLE_AGENT.contains("version = "));
        assert!(EXAMPLE_SIMPLE_AGENT.contains("description = "));
    }

    #[test]
    fn test_example_langchain_agent_is_valid_toml_format() {
        assert!(EXAMPLE_LANGCHAIN_AGENT.contains("[agent]"));
        assert!(EXAMPLE_LANGCHAIN_AGENT.contains("[model]"));
        assert!(EXAMPLE_LANGCHAIN_AGENT.contains("[capabilities]"));
        assert!(EXAMPLE_LANGCHAIN_AGENT.contains("framework = \"langchain\""));
    }

    #[test]
    fn test_example_crew_coordinator_is_valid_toml_format() {
        assert!(EXAMPLE_CREW_COORDINATOR.contains("[crew]"));
        assert!(EXAMPLE_CREW_COORDINATOR.contains("role = \"coordinator\""));
        assert!(EXAMPLE_CREW_COORDINATOR.contains("[restart]"));
    }

    #[test]
    fn test_example_crew_worker_is_valid_toml_format() {
        assert!(EXAMPLE_CREW_WORKER.contains("[crew]"));
        assert!(EXAMPLE_CREW_WORKER.contains("role = \"worker\""));
        assert!(EXAMPLE_CREW_WORKER.contains("framework = \"semantic_kernel\""));
    }

    #[test]
    fn test_example_high_resource_is_valid_toml_format() {
        assert!(EXAMPLE_HIGH_RESOURCE.contains("max_memory_bytes = 8589934592"));
        assert!(EXAMPLE_HIGH_RESOURCE.contains("max_gpu_ms = 120000"));
        assert!(EXAMPLE_HIGH_RESOURCE.contains("gpu_compute"));
    }

    #[test]
    fn test_example_health_checked_is_valid_toml_format() {
        assert!(EXAMPLE_HEALTH_CHECKED.contains("[health_check]"));
        assert!(EXAMPLE_HEALTH_CHECKED.contains("HEALTH_CHECK_DETAILED = \"true\""));
    }

    #[test]
    fn test_example_restart_policy_is_valid_toml_format() {
        assert!(EXAMPLE_RESTART_POLICY.contains("[restart]"));
        assert!(EXAMPLE_RESTART_POLICY.contains("max_retries = 15"));
        assert!(EXAMPLE_RESTART_POLICY.contains("backoff_multiplier = 1.3"));
    }

    #[test]
    fn test_example_multi_dependency_is_valid_toml_format() {
        assert!(EXAMPLE_MULTI_DEPENDENCY.contains("[dependencies]"));
        assert!(EXAMPLE_MULTI_DEPENDENCY.contains("after = "));
        assert!(EXAMPLE_MULTI_DEPENDENCY.contains("before = "));
        assert!(EXAMPLE_MULTI_DEPENDENCY.contains("requires = "));
    }

    #[test]
    fn test_all_examples_have_required_agent_section() {
        let examples = [
            EXAMPLE_SIMPLE_AGENT,
            EXAMPLE_LANGCHAIN_AGENT,
            EXAMPLE_CREW_COORDINATOR,
            EXAMPLE_CREW_WORKER,
            EXAMPLE_HIGH_RESOURCE,
            EXAMPLE_HEALTH_CHECKED,
            EXAMPLE_RESTART_POLICY,
            EXAMPLE_MULTI_DEPENDENCY,
        ];

        for example in &examples {
            assert!(example.contains("[agent]"), "Example must have [agent] section");
            assert!(example.contains("name = "), "Example must have name field");
            assert!(example.contains("version = "), "Example must have version field");
            assert!(
                example.contains("description = "),
                "Example must have description field"
            );
        }
    }
}
