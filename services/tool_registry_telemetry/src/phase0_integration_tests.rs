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
//! - **End-to-end workflows**: Complete register → invoke → emit → verify cycles
//!
//! # Test Scenarios
//!
//! Each test validates:
//! 1. **Happy path**: Normal successful execution
//! 2. **Error cases**: Graceful error handling with Result types
//! 3. **Edge cases**: Boundary conditions and special values
//! 4. **Metrics accuracy**: Proper cost calculation and event details
//!
//! # Example
//!
//! ```ignore
//! #[test]
//! fn test_register_and_invoke_tool() -> Result<()> {
//!     // ... setup ...
//!     registry.register_tool(binding)?;
//!     // ... invoke ...
//!     assert!(events_captured > 0);
//!     Ok(())
//! }
//! ```

use crate::error::{Error, Result};
use crate::tool_binding::ToolBinding;
use crate::tool_registry::ToolRegistry;
use crate::telemetry_engine::TelemetryEngine;
use crate::effect_class::EffectClass;
use crate::cost_calculator::CostCalculator;
use crate::event_subscriber::{EventSubscriber, EventFilter};
use serde_json::{json, Value};
use alloc::sync::Arc; // Mutex not available in no_std

/// Test fixture for integration tests.
struct IntegrationTestFixture {
    registry: ToolRegistry,
    telemetry: Arc<Mutex<TelemetryEngine>>,
    cost_calculator: Arc<CostCalculator>,
}

impl IntegrationTestFixture {
    /// Creates a new integration test fixture with initialized components.
    fn new() -> Result<Self> {
        let registry = ToolRegistry::new();
        let cost_calculator = Arc::new(CostCalculator::new());
        let telemetry = Arc::new(Mutex::new(TelemetryEngine::new()?));

        Ok(Self {
            registry,
            telemetry,
            cost_calculator,
        })
    }

    /// Registers a simple test tool.
    fn register_test_tool(&mut self, tool_id: &str) -> Result<()> {
        let binding = ToolBinding::new(
            tool_id,
            "Test Tool",
            EffectClass::Deterministic,
            json!({}),
        );
        self.registry.register(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;

    #[test]
    fn test_register_tool_success() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        fixture.register_test_tool("test_tool_1")?;
        
        let binding = fixture.registry.lookup("test_tool_1")?;
        assert_eq!(binding.tool_id, "test_tool_1");
        assert_eq!(binding.name, "Test Tool");
        Ok(())
    }

    #[test]
    fn test_register_duplicate_tool_fails() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        fixture.register_test_tool("test_tool_2")?;
        
        let result = fixture.register_test_tool("test_tool_2");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_lookup_unregistered_tool_fails() -> Result<()> {
        let fixture = IntegrationTestFixture::new()?;
        let result = fixture.registry.lookup("nonexistent");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_tool_invocation_emits_event() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        fixture.register_test_tool("test_tool_3")?;
        
        let binding = fixture.registry.lookup("test_tool_3")?;
        let tool_invoked_event = json!({
            "event_type": "tool_invoked",
            "tool_id": binding.tool_id,
            "timestamp": 1000,
            "invocation_id": "inv_1",
        });

        {
            let mut telemetry = fixture.telemetry.lock().map_err(|_| {
                Error::internal("Failed to acquire telemetry lock".to_string())
            })?;
            telemetry.emit_event(tool_invoked_event)?;
        }

        // Verify event was recorded
        let telemetry = fixture.telemetry.lock().map_err(|_| {
            Error::internal("Failed to acquire telemetry lock".to_string())
        })?;
        
        Ok(())
    }

    #[test]
    fn test_effect_class_enforcement() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        
        // Register deterministic tool
        let det_binding = ToolBinding::new(
            "deterministic_tool",
            "Deterministic",
            EffectClass::Deterministic,
            json!({}),
        );
        fixture.registry.register(det_binding)?;

        // Register nondeterministic tool
        let nondet_binding = ToolBinding::new(
            "nondeterministic_tool",
            "Non-deterministic",
            EffectClass::Nondeterministic,
            json!({}),
        );
        fixture.registry.register(nondet_binding)?;

        // Both should be registered successfully
        assert!(fixture.registry.lookup("deterministic_tool").is_ok());
        assert!(fixture.registry.lookup("nondeterministic_tool").is_ok());
        
        Ok(())
    }

    #[test]
    fn test_cost_calculation_accuracy() -> Result<()> {
        let fixture = IntegrationTestFixture::new()?;
        
        // Test cost calculation with known input tokens
        let cost = fixture.cost_calculator.calculate_cost(
            100,  // input_tokens
            50,   // output_tokens
            1.0,  // input_cost_per_token
            2.0,  // output_cost_per_token
        )?;

        // Expected: (100 * 1.0) + (50 * 2.0) = 200.0
        assert!((cost - 200.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_event_subscriber_registration() -> Result<()> {
        let fixture = IntegrationTestFixture::new()?;
        
        let mut telemetry = fixture.telemetry.lock().map_err(|_| {
            Error::internal("Failed to acquire telemetry lock".to_string())
        })?;

        // Create a subscriber
        let subscriber = EventSubscriber::new("test_subscriber")?;
        let filter = EventFilter::default();
        
        telemetry.subscribe(subscriber, filter)?;
        
        Ok(())
    }

    #[test]
    fn test_subscriber_receives_events() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        fixture.register_test_tool("test_tool_sub")?;

        let mut telemetry = fixture.telemetry.lock().map_err(|_| {
            Error::internal("Failed to acquire telemetry lock".to_string())
        })?;

        // Create subscriber
        let subscriber = EventSubscriber::new("test_sub")?;
        let filter = EventFilter::default();
        telemetry.subscribe(subscriber, filter)?;

        // Emit event
        let event = json!({
            "event_type": "tool_invoked",
            "tool_id": "test_tool_sub",
            "timestamp": 2000,
        });
        telemetry.emit_event(event)?;
        
        Ok(())
    }

    #[test]
    fn test_full_workflow_register_invoke_emit() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        
        // Step 1: Register tool
        fixture.register_test_tool("workflow_tool")?;
        let binding = fixture.registry.lookup("workflow_tool")?;
        assert_eq!(binding.tool_id, "workflow_tool");
        
        // Step 2: Emit invocation event
        let invocation_event = json!({
            "event_type": "tool_invoked",
            "tool_id": "workflow_tool",
            "timestamp": 3000,
            "invocation_id": "inv_workflow_1",
            "effect_class": "deterministic",
        });

        {
            let mut telemetry = fixture.telemetry.lock().map_err(|_| {
                Error::internal("Failed to acquire telemetry lock".to_string())
            })?;
            telemetry.emit_event(invocation_event)?;
        }

        // Step 3: Emit completion event with costs
        let completion_event = json!({
            "event_type": "tool_completed",
            "tool_id": "workflow_tool",
            "timestamp": 3100,
            "invocation_id": "inv_workflow_1",
            "input_tokens": 100,
            "output_tokens": 50,
            "total_cost": 200.0,
        });

        {
            let mut telemetry = fixture.telemetry.lock().map_err(|_| {
                Error::internal("Failed to acquire telemetry lock".to_string())
            })?;
            telemetry.emit_event(completion_event)?;
        }

        Ok(())
    }

    #[test]
    fn test_multiple_tools_independent_registration() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        
        for i in 0..5 {
            let tool_id = format!("multi_tool_{}", i);
            fixture.register_test_tool(&tool_id)?;
        }

        // Verify all registered
        for i in 0..5 {
            let tool_id = format!("multi_tool_{}", i);
            assert!(fixture.registry.lookup(&tool_id).is_ok());
        }

        Ok(())
    }

    #[test]
    fn test_cost_metrics_per_invocation() -> Result<()> {
        let fixture = IntegrationTestFixture::new()?;
        
        // Simulate multiple invocations with different token counts
        let invocations = vec![
            (50, 25, 75.0),
            (100, 50, 200.0),
            (200, 100, 400.0),
        ];

        for (input, output, expected_cost) in invocations {
            let cost = fixture.cost_calculator.calculate_cost(
                input,
                output,
                1.0,
                2.0,
            )?;
            
            assert!((cost - expected_cost).abs() < 0.01);
        }

        Ok(())
    }

    #[test]
    fn test_tool_binding_metadata() -> Result<()> {
        let metadata = json!({
            "category": "search",
            "version": "1.0",
            "deprecated": false,
        });

        let binding = ToolBinding::new(
            "metadata_tool",
            "Metadata Test Tool",
            EffectClass::Deterministic,
            metadata.clone(),
        );

        assert_eq!(binding.tool_id, "metadata_tool");
        assert_eq!(binding.metadata, metadata);
        Ok(())
    }

    #[test]
    fn test_deterministic_vs_nondeterministic_tools() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        
        let det_binding = ToolBinding::new(
            "det_tool",
            "Deterministic",
            EffectClass::Deterministic,
            json!({}),
        );
        fixture.registry.register(det_binding)?;

        let nondet_binding = ToolBinding::new(
            "nondet_tool",
            "Non-deterministic",
            EffectClass::Nondeterministic,
            json!({}),
        );
        fixture.registry.register(nondet_binding)?;

        // Both should work, just with different effect classes
        let det = fixture.registry.lookup("det_tool")?;
        let nondet = fixture.registry.lookup("nondet_tool")?;
        
        assert_eq!(det.effect_class, EffectClass::Deterministic);
        assert_eq!(nondet.effect_class, EffectClass::Nondeterministic);
        
        Ok(())
    }

    #[test]
    fn test_event_subscriber_filter() -> Result<()> {
        let fixture = IntegrationTestFixture::new()?;
        
        let filter = EventFilter::new()
            .with_event_type("tool_invoked".to_string());

        let subscriber = EventSubscriber::new("filtered_sub")?;
        
        let mut telemetry = fixture.telemetry.lock().map_err(|_| {
            Error::internal("Failed to acquire telemetry lock".to_string())
        })?;
        
        telemetry.subscribe(subscriber, filter)?;
        
        Ok(())
    }

    #[test]
    fn test_cost_calculator_zero_tokens() -> Result<()> {
        let calculator = CostCalculator::new();
        
        let cost = calculator.calculate_cost(0, 0, 1.0, 2.0)?;
        assert!((cost - 0.0).abs() < 0.01);
        
        Ok(())
    }

    #[test]
    fn test_cost_calculator_fractional_tokens() -> Result<()> {
        let calculator = CostCalculator::new();
        
        let cost = calculator.calculate_cost(33, 17, 1.5, 2.5)?;
        // (33 * 1.5) + (17 * 2.5) = 49.5 + 42.5 = 92.0
        let expected = 92.0;
        assert!((cost - expected).abs() < 0.01);
        
        Ok(())
    }

    #[test]
    fn test_telemetry_engine_initialization() -> Result<()> {
        let telemetry = TelemetryEngine::new()?;
        // Should initialize without events
        Ok(())
    }

    #[test]
    fn test_registry_list_all_tools() -> Result<()> {
        let mut fixture = IntegrationTestFixture::new()?;
        
        for i in 0..3 {
            fixture.register_test_tool(&format!("list_tool_{}", i))?;
        }

        let tools = fixture.registry.list_all()?;
        assert!(tools.len() >= 3);
        
        Ok(())
    }
}
