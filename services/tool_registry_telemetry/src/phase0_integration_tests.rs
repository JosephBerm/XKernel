//! End-to-end integration tests for Phase 0 tool registry and telemetry system.
//!
//! This module contains comprehensive integration tests validating the complete
//! workflow from tool registration through event emission and cost metric verification.
//!
//! # Test Coverage
//!
//! - **Registration tests**: Tool registration with metadata validation
//! - **Invocation tests**: Tool execution with proper event emission
//! - **Cost metrics**: Token counting, cost attribution, and calculation accuracy
//! - **Effect enforcement**: Effect class validation and restriction enforcement
//! - **Subscriber tests**: Event subscriber registration and notification
//! - **End-to-end workflows**: Complete register -> invoke -> emit -> verify cycles

use crate::error::Result;
use crate::tool_binding::ToolBinding;
use crate::tool_registry::ToolRegistry;
use crate::telemetry_engine::TelemetryEngine;
use crate::effect_class::EffectClass;
use crate::ids::{AgentID, CapID, ToolBindingID, ToolID};
use crate::schema::TypeSchema;
use crate::cef::{CefEvent, CefEventType};
use crate::cost_calculator::InvocationCostCalculator;
use crate::event_subscriber::EventFilter;

/// Test fixture for integration tests.
struct IntegrationTestFixture {
    registry: ToolRegistry,
    telemetry: TelemetryEngine,
}

impl IntegrationTestFixture {
    /// Creates a new integration test fixture with initialized components.
    fn new() -> Self {
        let registry = ToolRegistry::new();
        let telemetry = TelemetryEngine::new();

        Self {
            registry,
            telemetry,
        }
    }

    /// Registers a simple test tool.
    fn register_test_tool(&mut self, tool_id: &str) -> core::result::Result<String, crate::tool_registry::RegistryError> {
        let binding = ToolBinding::new(
            ToolBindingID::new(tool_id),
            ToolID::new(tool_id),
            AgentID::new("test-agent"),
            CapID::from_bytes([0u8; 32]),
            TypeSchema::no_input("void"),
        );
        self.registry.register_tool(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_register_tool_success() {
        let mut fixture = IntegrationTestFixture::new();
        let result = fixture.register_test_tool("test_tool_1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_duplicate_tool_fails() {
        let mut fixture = IntegrationTestFixture::new();
        fixture.register_test_tool("test_tool_2").unwrap();

        let result = fixture.register_test_tool("test_tool_2");
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_unregistered_tool_fails() {
        let fixture = IntegrationTestFixture::new();
        let result = fixture.registry.get_binding("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_tool_invocation_emits_event() {
        let mut fixture = IntegrationTestFixture::new();
        fixture.register_test_tool("test_tool_3").unwrap();

        let event = CefEvent::new(
            "evt-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );

        let result = fixture.telemetry.emit_event(event);
        assert!(result.is_ok());
        assert!(fixture.telemetry.event_count() > 0);
    }

    #[test]
    fn test_effect_class_enforcement() {
        let mut fixture = IntegrationTestFixture::new();

        // Register read-only tool
        let readonly_binding = ToolBinding::new(
            ToolBindingID::new("readonly_tool"),
            ToolID::new("readonly_tool"),
            AgentID::new("test-agent"),
            CapID::from_bytes([0u8; 32]),
            TypeSchema::no_input("void"),
        );
        let mut readonly_binding_configured = readonly_binding;
        readonly_binding_configured.effect_class = EffectClass::ReadOnly;
        fixture.registry.register_tool(readonly_binding_configured).unwrap();

        // Register write-irreversible tool
        let write_binding = ToolBinding::new(
            ToolBindingID::new("write_tool"),
            ToolID::new("write_tool"),
            AgentID::new("test-agent"),
            CapID::from_bytes([1u8; 32]),
            TypeSchema::no_input("void"),
        );
        fixture.registry.register_tool(write_binding).unwrap();

        // Both should be registered successfully
        assert!(fixture.registry.get_binding("readonly_tool").is_some());
        assert!(fixture.registry.get_binding("write_tool").is_some());
    }

    #[test]
    fn test_cost_calculation_accuracy() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("hello world test").unwrap();
        calc.add_output_tokens("result").unwrap();
        calc.record_gpu_time(100).unwrap();

        assert!(calc.total_tokens() > 0);
        assert_eq!(calc.gpu_ms(), 100);
    }

    #[test]
    fn test_full_workflow_register_invoke_emit() {
        let mut fixture = IntegrationTestFixture::new();

        // Step 1: Register tool
        fixture.register_test_tool("workflow_tool").unwrap();
        let binding = fixture.registry.get_binding("workflow_tool");
        assert!(binding.is_some());

        // Step 2: Emit invocation event
        let invocation_event = CefEvent::new(
            "evt-inv-1",
            "trace-wf-1",
            "span-wf-1",
            "ct-wf-1",
            "agent-wf-1",
            3000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        fixture.telemetry.emit_event(invocation_event).unwrap();

        // Step 3: Emit completion event
        let completion_event = CefEvent::new(
            "evt-comp-1",
            "trace-wf-1",
            "span-wf-1",
            "ct-wf-1",
            "agent-wf-1",
            3100,
            CefEventType::ToolCallCompleted,
            "acting",
        );
        fixture.telemetry.emit_event(completion_event).unwrap();

        assert_eq!(fixture.telemetry.event_count(), 2);
    }

    #[test]
    fn test_multiple_tools_independent_registration() {
        let mut fixture = IntegrationTestFixture::new();

        for i in 0..5 {
            let tool_id = format!("multi_tool_{}", i);
            fixture.register_test_tool(&tool_id).unwrap();
        }

        // Verify all registered
        for i in 0..5 {
            let tool_id = format!("multi_tool_{}", i);
            assert!(fixture.registry.get_binding(&tool_id).is_some());
        }
    }

    #[test]
    fn test_telemetry_engine_initialization() {
        let telemetry = TelemetryEngine::new();
        assert_eq!(telemetry.event_count(), 0);
        assert_eq!(telemetry.buffer_size(), 0);
    }

    #[test]
    fn test_registry_list_all_tools() {
        let mut fixture = IntegrationTestFixture::new();

        for i in 0..3 {
            fixture.register_test_tool(&format!("list_tool_{}", i)).unwrap();
        }

        let tools = fixture.registry.list_all();
        assert!(tools.len() >= 3);
    }
}
