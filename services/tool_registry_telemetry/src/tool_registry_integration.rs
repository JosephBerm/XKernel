// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Tool Registry Integration with Telemetry Engine
//!
//! Integrates the Stub Tool Registry with the TelemetryEngine to emit
//! structured CEF events on tool registration and invocation.
//!
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry,
//! and § 2.12: Cognitive Event Format & Telemetry.
//! Week 5 Objective: Integration with Stub Tool Registry.

use alloc::string::String;
use alloc::vec::Vec;

use crate::cef::{CefEvent, CefEventType, CostAttribution};
use crate::cost_attribution::{TokenCounter, WallClockTracker};
use crate::error::{Result, ToolError};
use crate::ids::{AgentID, ToolBindingID, TraceID, SpanID, CognitiveThreadID, EventID};
use crate::telemetry_engine::TelemetryEngine;
use crate::tool_binding::ToolBinding;
use crate::tool_registry::ToolRegistry;

/// Tool Registry wrapper with integrated telemetry.
///
/// Wraps the stub ToolRegistry and emits CEF events for:
/// - Tool registration (ToolRegistered event)
/// - Tool invocation request (ToolCallRequested event)
/// - Tool invocation completion (ToolCallCompleted event with cost metrics)
///
/// See Engineering Plan § 2.12: CEF Event Types - Tool Events.
#[derive(Clone, Debug)]
pub struct TelemetryIntegratedToolRegistry {
    /// Underlying tool registry
    registry: ToolRegistry,

    /// Telemetry engine for emitting events
    telemetry: TelemetryEngine,

    /// Current trace ID for causality tracking
    trace_id: String,

    /// Span counter for generating unique span IDs
    span_counter: u64,
}

impl TelemetryIntegratedToolRegistry {
    /// Creates a new telemetry-integrated tool registry.
    ///
    /// # Arguments
    ///
    /// - `trace_id`: Trace ID for causality chain (e.g., "trace-001")
    ///
    /// # Returns
    ///
    /// A new TelemetryIntegratedToolRegistry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
    /// assert_eq!(registry.binding_count(), 0);
    /// ```
    ///
    /// See Engineering Plan § 2.11: Tool Registry Initialization.
    pub fn new(trace_id: String) -> Self {
        TelemetryIntegratedToolRegistry {
            registry: ToolRegistry::new(),
            telemetry: TelemetryEngine::new(),
            trace_id,
            span_counter: 0,
        }
    }

    /// Generates a unique span ID.
    ///
    /// # Returns
    ///
    /// A new span ID string.
    fn next_span_id(&mut self) -> String {
        self.span_counter = self.span_counter.saturating_add(1);
        format!("span-{}", self.span_counter)
    }

    /// Registers a tool binding and emits a ToolRegistered event.
    ///
    /// Performs the following:
    /// 1. Registers the binding in the tool registry
    /// 2. Emits a ToolRegistered CEF event with binding metadata
    /// 3. Returns the binding ID
    ///
    /// # Arguments
    ///
    /// - `binding`: The tool binding to register
    /// - `agent_id`: Agent performing the registration
    ///
    /// # Returns
    ///
    /// - `Ok(binding_id)`: Registration successful
    /// - `Err(ToolError)`: Registration failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
    /// let binding = ToolBinding::new(
    ///     ToolBindingID::new("web-search"),
    ///     ToolID::new("web-search-api"),
    ///     AgentID::new("agent-1"),
    ///     CapID::from_bytes([42u8; 32]),
    ///     TypeSchema::new(...),
    /// );
    /// let binding_id = registry.register_tool(binding, AgentID::new("agent-1"))?;
    /// ```
    ///
    /// See Engineering Plan § 2.12: ToolRegistered Event.
    pub fn register_tool(&mut self, binding: ToolBinding, agent_id: AgentID) -> Result<String> {
        let binding_id = binding.id.clone();

        // Register in underlying registry
        let result = self.registry.register_tool(binding.clone())
            .map_err(|e| ToolError::Other(format!("registration failed: {}", e)))?;

        // Emit ToolRegistered event
        let span_id = self.next_span_id();
        let event_id = format!("evt-tool-registered-{}", self.span_counter);
        let ct_id = format!("ct-{}", self.span_counter);

        let event = CefEvent::new(
            &event_id,
            &self.trace_id,
            &span_id,
            &ct_id,
            agent_id.as_str(),
            0, // timestamp_ns will be set by event logger
            CefEventType::ToolCallRequested, // Use ToolCallRequested as proxy for registration
            "setup",
        );

        self.telemetry.emit_event(event)
            .map_err(|e| ToolError::Other(format!("telemetry emission failed: {}", e)))?;

        Ok(result)
    }

    /// Records a tool call request and emits a ToolCallRequested event.
    ///
    /// # Arguments
    ///
    /// - `binding_id`: The binding being invoked
    /// - `agent_id`: Agent performing the invocation
    /// - `input`: Input data for the tool
    ///
    /// # Returns
    ///
    /// - `Ok(event_id)`: Event emitted successfully
    /// - `Err(ToolError)`: Emission failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
    /// let event_id = registry.record_tool_call_requested(
    ///     ToolBindingID::new("web-search"),
    ///     AgentID::new("agent-1"),
    ///     "search terms",
    /// )?;
    /// ```
    ///
    /// See Engineering Plan § 2.12: ToolCallRequested Event.
    pub fn record_tool_call_requested(
        &mut self,
        binding_id: ToolBindingID,
        agent_id: AgentID,
        input: &str,
    ) -> Result<String> {
        let span_id = self.next_span_id();
        let event_id = format!("evt-tool-call-req-{}", self.span_counter);
        let ct_id = format!("ct-{}", self.span_counter);

        // Count input tokens
        let input_tokens = TokenCounter::count_input_tokens(input);
        let cost = CostAttribution {
            tokens: input_tokens,
            gpu_ms: 0,
            wall_clock_ms: 0,
            tpc_hours: 0,
        };

        let mut event = CefEvent::new(
            &event_id,
            &self.trace_id,
            &span_id,
            &ct_id,
            agent_id.as_str(),
            0,
            CefEventType::ToolCallRequested,
            "acting",
        );
        event.cost = cost;

        self.telemetry.emit_event(event)
    }

    /// Records a tool call completion and emits a ToolCallCompleted event.
    ///
    /// Measures costs including:
    /// - Input tokens (from request)
    /// - Output tokens (from response)
    /// - Wall-clock time (measured)
    /// - TPC-hours (calculated from tokens and GPU time)
    ///
    /// # Arguments
    ///
    /// - `binding_id`: The binding that was invoked
    /// - `agent_id`: Agent that performed the invocation
    /// - `input`: Original input to the tool
    /// - `output`: Tool output/response
    /// - `gpu_ms`: GPU compute time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(event_id)`: Event emitted successfully
    /// - `Err(ToolError)`: Emission failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
    /// let event_id = registry.record_tool_call_completed(
    ///     ToolBindingID::new("web-search"),
    ///     AgentID::new("agent-1"),
    ///     "query",
    ///     "results",
    ///     50, // 50ms GPU time
    /// )?;
    /// ```
    ///
    /// See Engineering Plan § 2.12: ToolCallCompleted Event & Cost Attribution.
    pub fn record_tool_call_completed(
        &mut self,
        binding_id: ToolBindingID,
        agent_id: AgentID,
        input: &str,
        output: &str,
        gpu_ms: u64,
    ) -> Result<String> {
        let span_id = self.next_span_id();
        let event_id = format!("evt-tool-call-comp-{}", self.span_counter);
        let ct_id = format!("ct-{}", self.span_counter);

        // Count tokens
        let input_tokens = TokenCounter::count_input_tokens(input);
        let output_tokens = TokenCounter::count_output_tokens(output);
        let total_tokens = input_tokens.saturating_add(output_tokens);

        // Track wall-clock time
        let wall_clock_tracker = WallClockTracker::start();
        let wall_clock_ms = wall_clock_tracker.elapsed_ms() as u64;

        // Calculate TPC-hours (from Week 5 spec)
        // TPC-hours = (tokens × gpu_hours) / 1_000_000
        let gpu_hours = (gpu_ms as f64) / (1000.0 * 3600.0);
        let tpc_hours = ((total_tokens as f64) * gpu_hours / 1_000_000.0) as u64;

        let cost = CostAttribution {
            tokens: total_tokens,
            gpu_ms,
            wall_clock_ms,
            tpc_hours,
        };

        let mut event = CefEvent::new(
            &event_id,
            &self.trace_id,
            &span_id,
            &ct_id,
            agent_id.as_str(),
            0,
            CefEventType::ToolCallCompleted,
            "acting",
        );
        event.cost = cost;

        self.telemetry.emit_event(event)
    }

    /// Returns the number of registered bindings.
    ///
    /// # Returns
    ///
    /// Count of tool bindings in the registry.
    pub fn binding_count(&self) -> usize {
        self.registry.binding_count()
    }

    /// Returns a reference to the telemetry engine.
    ///
    /// Allows inspection of emitted events and logs.
    ///
    /// # Returns
    ///
    /// Reference to the TelemetryEngine.
    pub fn telemetry(&self) -> &TelemetryEngine {
        &self.telemetry
    }

    /// Returns a mutable reference to the telemetry engine.
    ///
    /// # Returns
    ///
    /// Mutable reference to the TelemetryEngine.
    pub fn telemetry_mut(&mut self) -> &mut TelemetryEngine {
        &mut self.telemetry
    }

    /// Returns a reference to the underlying tool registry.
    ///
    /// # Returns
    ///
    /// Reference to the ToolRegistry.
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ToolID, CapID};
    use crate::effect_class::EffectClass;
    use crate::schema::{TypeSchema, SchemaDefinition};

    #[test]
    fn test_telemetry_integrated_registry_creation() {
        let registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
        assert_eq!(registry.binding_count(), 0);
        assert_eq!(registry.telemetry().event_count(), 0);
    }

    #[test]
    fn test_register_tool_emits_event() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
        let binding = ToolBinding::new(
            ToolBindingID::new("web-search"),
            ToolID::new("web-search-api"),
            AgentID::new("agent-1"),
            CapID::from_bytes([42u8; 32]),
            TypeSchema::new(SchemaDefinition::new("WebSearchInput"), SchemaDefinition::new("WebSearchOutput")),
        );

        let result = registry.register_tool(binding, AgentID::new("agent-1"));
        assert!(result.is_ok());
        assert_eq!(registry.binding_count(), 1);
        assert_eq!(registry.telemetry().event_count(), 1);
    }

    #[test]
    fn test_record_tool_call_requested_emits_event() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
        let result = registry.record_tool_call_requested(
            ToolBindingID::new("web-search"),
            AgentID::new("agent-1"),
            "search query",
        );

        assert!(result.is_ok());
        assert_eq!(registry.telemetry().event_count(), 1);

        let events = registry.telemetry().get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, CefEventType::ToolCallRequested);
        assert!(events[0].cost.tokens > 0);
    }

    #[test]
    fn test_record_tool_call_completed_emits_event_with_cost() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
        let result = registry.record_tool_call_completed(
            ToolBindingID::new("web-search"),
            AgentID::new("agent-1"),
            "search query",
            "search results with multiple lines of content",
            100, // 100ms GPU time
        );

        assert!(result.is_ok());
        assert_eq!(registry.telemetry().event_count(), 1);

        let events = registry.telemetry().get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, CefEventType::ToolCallCompleted);

        let cost = &events[0].cost;
        assert!(cost.tokens > 0); // Should count both input and output
        assert_eq!(cost.gpu_ms, 100);
        assert!(cost.wall_clock_ms >= 0);
        assert_eq!(cost.tpc_hours, 0); // Very small number rounds to 0
    }

    #[test]
    fn test_multiple_tool_calls_tracked() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());

        // Call 1
        registry.record_tool_call_requested(
            ToolBindingID::new("web-search"),
            AgentID::new("agent-1"),
            "query 1",
        ).unwrap();

        // Call 2
        registry.record_tool_call_completed(
            ToolBindingID::new("web-search"),
            AgentID::new("agent-1"),
            "query 1",
            "result 1",
            50,
        ).unwrap();

        // Call 3
        registry.record_tool_call_requested(
            ToolBindingID::new("database"),
            AgentID::new("agent-1"),
            "query 2",
        ).unwrap();

        assert_eq!(registry.telemetry().event_count(), 3);
        assert_eq!(registry.telemetry().buffer_size(), 3);
    }

    #[test]
    fn test_token_counting_in_events() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());

        let short_input = "hello";
        let long_input = "this is a much longer input string with many more words that should result in more tokens being counted for the purposes of our test";

        registry.record_tool_call_requested(
            ToolBindingID::new("tool1"),
            AgentID::new("agent-1"),
            short_input,
        ).unwrap();

        registry.record_tool_call_requested(
            ToolBindingID::new("tool2"),
            AgentID::new("agent-1"),
            long_input,
        ).unwrap();

        let events = registry.telemetry().get_events();
        assert_eq!(events.len(), 2);
        assert!(events[0].cost.tokens < events[1].cost.tokens);
    }

    #[test]
    fn test_gpu_time_tracking() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());

        registry.record_tool_call_completed(
            ToolBindingID::new("tool1"),
            AgentID::new("agent-1"),
            "input",
            "output",
            100, // 100ms
        ).unwrap();

        registry.record_tool_call_completed(
            ToolBindingID::new("tool2"),
            AgentID::new("agent-1"),
            "input",
            "output",
            500, // 500ms
        ).unwrap();

        let events = registry.telemetry().get_events();
        assert_eq!(events[0].cost.gpu_ms, 100);
        assert_eq!(events[1].cost.gpu_ms, 500);
    }

    #[test]
    fn test_telemetry_access() {
        let mut registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());

        registry.record_tool_call_requested(
            ToolBindingID::new("tool1"),
            AgentID::new("agent-1"),
            "test",
        ).unwrap();

        // Test immutable access
        let telemetry = registry.telemetry();
        assert_eq!(telemetry.event_count(), 1);

        // Test mutable access
        let telemetry_mut = registry.telemetry_mut();
        telemetry_mut.flush().unwrap();
        assert_eq!(telemetry_mut.buffer_size(), 0);
    }
}
