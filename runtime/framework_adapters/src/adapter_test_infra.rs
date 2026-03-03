// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Adapter Testing Infrastructure
//!
//! Provides comprehensive testing utilities and mocks for adapter lifecycle testing.
//! Includes MockKernelIpc, TestAgent scenarios, assertion helpers, and adapter lifecycle
//! test suite support.
//!
//! Sec 5.2: Adapter Testing Infrastructure

use crate::error::AdapterError;
use crate::AdapterResult;
use crate::adapter_interface_contract::{
    AdapterState, FrameworkAgentConfig, FrameworkChainDefinition, ChainStepDefinition,
    FrameworkResultItem, AdapterErrorInfo,
};
use crate::framework_type::FrameworkType;

/// Mock kernel IPC interface for testing.
/// Sec 5.2: MockKernelIpc Test Double
#[derive(Debug, Clone)]
pub struct MockKernelIpc {
    /// Recorded messages sent to kernel
    sent_messages: Vec<String>,
    /// Canned responses to return
    canned_responses: std::collections::BTreeMap<String, String>,
    /// Failure mode (None = success, Some(error) = always fail)
    failure_mode: Option<String>,
    /// Latency simulation in milliseconds
    latency_ms: u64,
}

impl MockKernelIpc {
    /// Creates a new mock kernel IPC.
    pub fn new() -> Self {
        MockKernelIpc {
            sent_messages: Vec::new(),
            canned_responses: std::collections::BTreeMap::new(),
            failure_mode: None,
            latency_ms: 0,
        }
    }

    /// Registers a canned response for a message pattern.
    pub fn register_response(&mut self, pattern: String, response: String) {
        self.canned_responses.insert(pattern, response);
    }

    /// Sets failure mode - if Some, all operations fail with this error.
    pub fn set_failure_mode(&mut self, error: Option<String>) {
        self.failure_mode = error;
    }

    /// Sets latency simulation.
    pub fn set_latency_ms(&mut self, latency: u64) {
        self.latency_ms = latency;
    }

    /// Gets recorded sent messages.
    pub fn sent_messages(&self) -> &[String] {
        &self.sent_messages
    }

    /// Gets message count.
    pub fn message_count(&self) -> usize {
        self.sent_messages.len()
    }

    /// Sends a message and returns response.
    pub fn send_message(&mut self, message: String) -> AdapterResult<String> {
        if let Some(ref error) = self.failure_mode {
            return Err(AdapterError::KernelIpcError(error.clone()));
        }

        self.sent_messages.push(message.clone());

        // Try to find matching canned response
        for (pattern, response) in &self.canned_responses {
            if message.contains(pattern) {
                return Ok(response.clone());
            }
        }

        // Default response
        Ok("OK".to_string())
    }

    /// Resets recorded state.
    pub fn reset(&mut self) {
        self.sent_messages.clear();
    }
}

impl Default for MockKernelIpc {
    fn default() -> Self {
        Self::new()
    }
}

/// Test agent scenario definitions.
/// Sec 5.2: TestAgent Scenario Builders
#[derive(Debug, Clone)]
pub struct TestAgent {
    /// Agent identifier
    pub agent_id: String,
    /// Agent name
    pub name: String,
    /// Available tools
    pub tools: Vec<String>,
    /// Memory capacity
    pub memory_capacity: u64,
    /// Whether agent execution should succeed
    pub should_succeed: bool,
    /// Optional error to simulate
    pub simulated_error: Option<String>,
}

impl TestAgent {
    /// Creates a simple test agent.
    pub fn simple(name: &str) -> Self {
        TestAgent {
            agent_id: format!("agent-{}", name.to_lowercase()),
            name: name.to_string(),
            tools: Vec::new(),
            memory_capacity: 100000,
            should_succeed: true,
            simulated_error: None,
        }
    }

    /// Adds a tool to the agent.
    pub fn with_tool(mut self, tool_id: String) -> Self {
        self.tools.push(tool_id);
        self
    }

    /// Sets memory capacity.
    pub fn with_memory(mut self, capacity: u64) -> Self {
        self.memory_capacity = capacity;
        self
    }

    /// Sets failure mode.
    pub fn failing(mut self, error: String) -> Self {
        self.should_succeed = false;
        self.simulated_error = Some(error);
        self
    }

    /// Converts to adapter config.
    pub fn to_adapter_config(&self) -> FrameworkAgentConfig {
        FrameworkAgentConfig {
            agent_id: self.agent_id.clone(),
            name: self.name.clone(),
            description: format!("Test agent: {}", self.name),
            system_prompt: "You are a test agent".into(),
            memory_type: "test-memory".into(),
            memory_capacity_tokens: self.memory_capacity,
            tool_ids: self.tools.clone(),
            timeout_ms: 30000,
            is_mandatory: true,
        }
    }
}

/// Test scenario for chain/plan execution.
/// Sec 5.2: TestChain Scenario Builder
#[derive(Debug, Clone)]
pub struct TestChain {
    /// Chain identifier
    pub chain_id: String,
    /// Number of steps
    pub step_count: u32,
    /// Whether execution should succeed
    pub should_succeed: bool,
}

impl TestChain {
    /// Creates a simple sequential chain.
    pub fn sequential(steps: u32) -> Self {
        TestChain {
            chain_id: format!("chain-seq-{}", steps),
            step_count: steps,
            should_succeed: true,
        }
    }

    /// Creates a failing chain.
    pub fn failing(steps: u32) -> Self {
        TestChain {
            chain_id: format!("chain-fail-{}", steps),
            step_count: steps,
            should_succeed: false,
        }
    }

    /// Converts to framework definition.
    pub fn to_framework_definition(&self) -> FrameworkChainDefinition {
        let mut steps = Vec::new();
        for i in 0..self.step_count {
            steps.push(ChainStepDefinition {
                step_id: format!("step-{}", i),
                name: format!("Step {}", i),
                action: "execute".into(),
                input_schema: "{}".into(),
                output_schema: "{}".into(),
                timeout_ms: 5000,
                depends_on: if i > 0 {
                    vec![format!("step-{}", i - 1)]
                } else {
                    vec![]
                },
            });
        }

        FrameworkChainDefinition {
            chain_id: self.chain_id.clone(),
            name: self.chain_id.clone(),
            chain_type: "sequential".into(),
            steps,
            timeout_ms: 30000,
        }
    }
}

/// Assertion helpers for adapter testing.
/// Sec 5.2: Adapter Assertion Helpers
pub struct AdapterAssertions;

impl AdapterAssertions {
    /// Asserts adapter is in expected state.
    pub fn assert_state(actual: AdapterState, expected: AdapterState, context: &str) {
        if actual != expected {
            panic!(
                "State assertion failed ({}): expected {}, got {}",
                context,
                expected.as_str(),
                actual.as_str()
            );
        }
    }

    /// Asserts result is successful.
    pub fn assert_success<T: std::fmt::Debug>(result: &AdapterResult<T>, context: &str) {
        if result.is_err() {
            panic!("Expected success but got error ({}): {:?}", context, result);
        }
    }

    /// Asserts result is an error.
    pub fn assert_error<T>(result: &AdapterResult<T>, context: &str) {
        if result.is_ok() {
            panic!("Expected error but got success ({})", context);
        }
    }

    /// Asserts result error contains substring.
    pub fn assert_error_contains(result: &AdapterResult<()>, substring: &str, context: &str) {
        match result {
            Err(e) => {
                let error_str = e.to_string();
                if !error_str.contains(substring) {
                    panic!(
                        "Error message doesn't contain '{}' ({}): {}",
                        substring, context, error_str
                    );
                }
            }
            Ok(_) => panic!("Expected error but got success ({})", context),
        }
    }

    /// Asserts two strings are equal.
    pub fn assert_equal(actual: &str, expected: &str, context: &str) {
        if actual != expected {
            panic!(
                "Assertion failed ({}): expected '{}', got '{}'",
                context, expected, actual
            );
        }
    }

    /// Asserts string contains substring.
    pub fn assert_contains(text: &str, substring: &str, context: &str) {
        if !text.contains(substring) {
            panic!(
                "String doesn't contain substring '{}' ({}): {}",
                substring, context, text
            );
        }
    }
}

/// Adapter lifecycle test scenario runner.
/// Sec 5.2: Adapter Lifecycle Test Suite
#[derive(Debug, Clone)]
pub struct AdapterLifecycleTestScenario {
    /// Test name
    pub name: String,
    /// Test agent
    pub agent: TestAgent,
    /// Test chain
    pub chain: TestChain,
    /// Should test initialization
    pub test_init: bool,
    /// Should test agent loading
    pub test_load_agent: bool,
    /// Should test chain translation
    pub test_translate_chain: bool,
    /// Should test task spawning
    pub test_spawn_tasks: bool,
    /// Should test result collection
    pub test_collect_results: bool,
    /// Should test error handling
    pub test_error_handling: bool,
}

impl AdapterLifecycleTestScenario {
    /// Creates a full lifecycle test scenario.
    pub fn full_lifecycle() -> Self {
        AdapterLifecycleTestScenario {
            name: "full-lifecycle".into(),
            agent: TestAgent::simple("TestAgent"),
            chain: TestChain::sequential(3),
            test_init: true,
            test_load_agent: true,
            test_translate_chain: true,
            test_spawn_tasks: true,
            test_collect_results: true,
            test_error_handling: true,
        }
    }

    /// Creates a minimal lifecycle test scenario.
    pub fn minimal() -> Self {
        AdapterLifecycleTestScenario {
            name: "minimal".into(),
            agent: TestAgent::simple("TestAgent"),
            chain: TestChain::sequential(1),
            test_init: true,
            test_load_agent: true,
            test_translate_chain: true,
            test_spawn_tasks: false,
            test_collect_results: false,
            test_error_handling: false,
        }
    }

    /// Creates an error handling test scenario.
    pub fn error_handling() -> Self {
        AdapterLifecycleTestScenario {
            name: "error-handling".into(),
            agent: TestAgent::simple("TestAgent").failing("Simulated failure".into()),
            chain: TestChain::failing(2),
            test_init: true,
            test_load_agent: true,
            test_translate_chain: true,
            test_spawn_tasks: true,
            test_collect_results: true,
            test_error_handling: true,
        }
    }

    /// Gets list of tests to run.
    pub fn tests_to_run(&self) -> Vec<String> {
        let mut tests = Vec::new();
        if self.test_init {
            tests.push("init".into());
        }
        if self.test_load_agent {
            tests.push("load_agent".into());
        }
        if self.test_translate_chain {
            tests.push("translate_chain".into());
        }
        if self.test_spawn_tasks {
            tests.push("spawn_tasks".into());
        }
        if self.test_collect_results {
            tests.push("collect_results".into());
        }
        if self.test_error_handling {
            tests.push("error_handling".into());
        }
        tests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::collections::BTreeMap;

    #[test]
    fn test_mock_kernel_ipc_send_message() {
        let mut ipc = MockKernelIpc::new();
        let result = ipc.send_message("test message".into());
        assert!(result.is_ok());
        assert_eq!(ipc.message_count(), 1);
    }

    #[test]
    fn test_mock_kernel_ipc_failure_mode() {
        let mut ipc = MockKernelIpc::new();
        ipc.set_failure_mode(Some("Simulated failure".into()));
        let result = ipc.send_message("test message".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_kernel_ipc_canned_response() {
        let mut ipc = MockKernelIpc::new();
        ipc.register_response("pattern".into(), "response".into());
        let result = ipc.send_message("test pattern here".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "response");
    }

    #[test]
    fn test_test_agent_simple() {
        let agent = TestAgent::simple("TestAgent");
        assert_eq!(agent.name, "TestAgent");
        assert!(agent.should_succeed);
    }

    #[test]
    fn test_test_agent_with_tools() {
        let agent = TestAgent::simple("TestAgent")
            .with_tool("tool-1".into())
            .with_tool("tool-2".into());
        assert_eq!(agent.tools.len(), 2);
    }

    #[test]
    fn test_test_agent_failing() {
        let agent = TestAgent::simple("TestAgent").failing("error".into());
        assert!(!agent.should_succeed);
        assert!(agent.simulated_error.is_some());
    }

    #[test]
    fn test_test_agent_to_adapter_config() {
        let agent = TestAgent::simple("TestAgent").with_memory(50000);
        let config = agent.to_adapter_config();
        assert_eq!(config.memory_capacity_tokens, 50000);
    }

    #[test]
    fn test_test_chain_sequential() {
        let chain = TestChain::sequential(3);
        assert_eq!(chain.step_count, 3);
        assert!(chain.should_succeed);
    }

    #[test]
    fn test_test_chain_failing() {
        let chain = TestChain::failing(2);
        assert!(!chain.should_succeed);
    }

    #[test]
    fn test_test_chain_to_definition() {
        let chain = TestChain::sequential(2);
        let def = chain.to_framework_definition();
        assert_eq!(def.steps.len(), 2);
    }

    #[test]
    fn test_adapter_assertions_state() {
        AdapterAssertions::assert_state(
            AdapterState::Initialized,
            AdapterState::Initialized,
            "test",
        );
    }

    #[test]
    fn test_adapter_assertions_success() {
        let result: AdapterResult<()> = Ok(());
        AdapterAssertions::assert_success(&result, "test");
    }

    #[test]
    fn test_adapter_assertions_error() {
        let result: AdapterResult<()> = Err(AdapterError::ConfigurationError("test".into()));
        AdapterAssertions::assert_error(&result, "test");
    }

    #[test]
    fn test_adapter_assertions_equal() {
        AdapterAssertions::assert_equal("test", "test", "test");
    }

    #[test]
    fn test_adapter_assertions_contains() {
        AdapterAssertions::assert_contains("test string", "string", "test");
    }

    #[test]
    fn test_adapter_lifecycle_full() {
        let scenario = AdapterLifecycleTestScenario::full_lifecycle();
        let tests = scenario.tests_to_run();
        assert_eq!(tests.len(), 6);
    }

    #[test]
    fn test_adapter_lifecycle_minimal() {
        let scenario = AdapterLifecycleTestScenario::minimal();
        let tests = scenario.tests_to_run();
        assert_eq!(tests.len(), 3);
    }

    #[test]
    fn test_adapter_lifecycle_error_handling() {
        let scenario = AdapterLifecycleTestScenario::error_handling();
        assert!(!scenario.agent.should_succeed);
        assert!(!scenario.chain.should_succeed);
    }
}
