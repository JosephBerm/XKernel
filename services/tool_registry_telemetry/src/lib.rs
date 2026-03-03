// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Tool Registry & Telemetry Service - ToolBinding and CEF Event Format
//!
//! This crate implements the Tool Registry subsystem for the Cognitive Substrate OS,
//! providing tool binding management, effect class enforcement, sandbox configuration,
//! and Cognitive Event Format (CEF) telemetry.
//!
//! # Architecture Overview
//!
//! The Tool Registry manages tool invocation through strongly-typed bindings that
//! declare effect classes, security constraints, and caching behavior:
//!
//! - **ToolBinding**: Binding between tool definition and agent context with all metadata
//! - **Effect Classes**: Declares mutation semantics (ReadOnly, WriteReversible, WriteCompensable, WriteIrreversible)
//! - **Sandbox Configuration**: Per-tool security constraints (network, filesystem, syscalls, limits)
//! - **Response Caching**: Cache policies with TTL, freshness strategies, and key strategies
//! - **Commit Protocol**: Two-phase commit support for transactional safety
//! - **Type Schema**: Input/output validation and serialization specs
//! - **CEF Events**: Distributed tracing aligned with OpenTelemetry standards
//!
//! See Engineering Plan § 2.11 (ToolBinding Entity & Tool Registry) and
//! § 2.12 (Cognitive Event Format & Telemetry).
//!
//! # Safety & Correctness
//!
//! This crate operates as kernel-adjacent code with strict safety guarantees:
//! - `#![forbid(unsafe_code)]` - No unsafe blocks allowed
//! - `#![no_std]` - Runs in kernel mode without standard library
//! - All errors return `Result<T, ToolError>` - No unwrap/expect
//! - Strongly typed identifiers prevent confusion
//! - Validation on all binding creation and configuration

#![forbid(unsafe_code)]

extern crate alloc;

pub mod cache;
pub mod cef;
pub mod cef_format;
pub mod chain_validator;
pub mod commit_protocol;
pub mod compliance;
pub mod correlation;
pub mod cost_attribution;
pub mod cost_calculator;
pub mod cost_validation;
pub mod effect_class;
pub mod effect_enforcement;
pub mod error;
pub mod event_logger;
pub mod event_subscriber;
pub mod ids;
pub mod mock_tools;
pub mod registry_introspection;
pub mod retention;
pub mod sandbox;
pub mod schema;
pub mod serialization;
pub mod streaming;
pub mod telemetry_engine;
pub mod token_counter;
pub mod tool_binding;
pub mod tool_registry;
pub mod tool_registry_integration;
pub mod performance_baselines;
pub mod persistent_event_logger;
pub mod phase0_architecture_doc;
pub mod phase0_integration_tests;
pub mod phase1_transition_plan;
pub mod retention_policy;
pub mod registry;
pub mod telemetry;
pub mod journal;
pub mod compliance_l1;

// Re-export commonly used types
pub use cache::CacheConfig;
pub use cef::{
    CefEvent, CefEventType, CheckpointCreatedData, CostAttribution, CTPhase, DataClassification,
    DeliveryStatus, ExceptionRaisedData, ExceptionSeverity, IpcMessageData, MemoryAccessData,
    MemoryAccessType, PhaseTransitionData, PolicyDecisionData, PolicyOutcome,
    SignalDeliveryMode, SignalDispatchedData, ThoughtStepData, ToolCallCompletedData,
    ToolCallRequestedData,
};
pub use cef_format::{
    BinaryCefEncoder, CefDecoder, CefEncoder, CefFormatVersion, CompressedEvent,
    CompressionStrategy, JsonCefEncoder, SchemaRegistry, encode_with_compression,
};
pub use chain_validator::{ChainAnalysis, ChainValidator, ExecutionChain, ChainValidationError};
pub use commit_protocol::{CommitProtocol, CommitType, RollbackStrategy};
pub use compliance::{AuditRecord, RedactionPolicy, RetentionPolicy as ComplianceRetentionPolicy};
pub use correlation::{CausalityChain, TraceContext, TraceFlags};
pub use cost_attribution::{
    AccuracyReport, AccuracyValidator, AggregatedCost, CostAggregator, CostCalculator,
    GpuCostCalculator, OperationCost, TokenCounter as CostTokenCounter, TpcCalculator, WallClockTracker,
};
pub use cost_calculator::InvocationCostCalculator;
pub use cost_validation::{
    DailyReconciliation, GroundTruthSource, OutlierEvent, SamplingStrategy, ValidationFramework,
    ValidationReport, run_validation,
};
pub use effect_class::EffectClass;
pub use effect_enforcement::{EffectEnforcer, EffectViolationAudit, ExecutionContext};
pub use error::{Result, ToolError};
pub use event_logger::EventLogger;
pub use event_subscriber::{EventFilter, EventSubscription, SubscriptionManager};
pub use ids::{
    AgentID, CognitiveThreadID, CrewID, EventID, PolicyID, SpanID, ToolBindingID,
    ToolID, TraceID, Ulid,
};
pub use mock_tools::{MockDatabaseTool, MockEmailTool, MockToolFactory, MockWebSearchTool};
pub use registry_introspection::{RegistryIntrospection, RegistryStats, ToolQuery};
pub use retention::{
    DefaultRetentionManager, EventLifecycleManager, RedactionRule, RetentionManager,
    RetentionPolicy, RetentionStats, RetentionTier, StorageDecision, TierMigration,
};
pub use sandbox::{FsPolicy, NetworkPolicy, SandboxConfig};
pub use schema::{FieldDefinition, SchemaDefinition, TypeSchema, ValidationRule};
pub use serialization::{
    CapnProtoEventSerializer, EventDeserializer, EventSerializer, JsonEventSerializer,
    ParquetEventSerializer, ProtobufEventSerializer, SerializationFormat, SerializerFactory,
};
pub use streaming::{
    DeliveryMode, EventBuffer, EventFilter as StreamingEventFilter, InMemoryStreamingEngine, MessageOrdering,
    OverflowPolicy, StreamingEngine, StreamingMetrics, Subscription, SubscriptionId,
};
pub use telemetry_engine::TelemetryEngine;
pub use token_counter::{TokenCounter, TokenCountSnapshot};
pub use tool_binding::ToolBinding;
pub use tool_registry::{RegistryError, ToolRegistry};
pub use tool_registry_integration::TelemetryIntegratedToolRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::AgentID;

    #[test]
    fn test_crate_compiles() {
        // Basic smoke test to ensure crate structure is valid
        let binding_id = ToolBindingID::new("test-binding");
        assert_eq!(binding_id.as_str(), "test-binding");
    }

    #[test]
    fn test_no_std_environment() {
        // Verify we can use Vec without std
        let mut vec = Vec::new();
        vec.push(1);
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn test_effect_class_defaults() {
        assert_eq!(EffectClass::default(), EffectClass::WriteIrreversible);
    }

    #[test]
    fn test_sandbox_defaults() {
        let restrictive = SandboxConfig::restrictive();
        assert!(!restrictive.network_access.allows_network());
    }

    #[test]
    fn test_cef_event_creation() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallRequested,
            "acting",
        );
        assert_eq!(event.event_id, "event-1");
        assert!(event.is_tool_event());
    }

    #[test]
    fn test_tool_registry_basic() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.binding_count(), 0);
    }

    #[test]
    fn test_execution_chain_basic() {
        let chain = ExecutionChain::new();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_effect_enforcer_basic() {
        let enforcer = EffectEnforcer::new();
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_mock_tools_creation() {
        let agent = AgentID::new("test-agent");
        let bindings = MockToolFactory::create_all_bindings(agent);
        assert_eq!(bindings.len(), 3);
    }

    #[test]
    fn test_registry_introspection_query() {
        let registry = ToolRegistry::new();
        let query = ToolQuery::new().read_only_only();
        let results = registry.query(query);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_registry_stats() {
        let registry = ToolRegistry::new();
        let stats = RegistryStats::from_registry(&registry);
        assert_eq!(stats.total_bindings, 0);
    }

    #[test]
    fn test_telemetry_engine_creation() {
        let engine = TelemetryEngine::new();
        assert_eq!(engine.buffer_size(), 0);
        assert_eq!(engine.event_count(), 0);
    }

    #[test]
    fn test_event_logger_formatting() {
        let event = CefEvent::new(
            "event-1",
            "trace-1",
            "span-1",
            "ct-1",
            "agent-1",
            1000,
            CefEventType::ToolCallCompleted,
            "acting",
        );
        let log_entry = EventLogger::log(&event).unwrap();
        assert!(log_entry.contains("event-1"));
    }

    #[test]
    fn test_invocation_cost_calculator() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("test").unwrap();
        calc.add_output_tokens("result").unwrap();
        calc.record_gpu_time(100).unwrap();
        
        assert!(calc.total_tokens() > 0);
        assert_eq!(calc.gpu_ms(), 100);
    }

    #[test]
    fn test_telemetry_integrated_registry() {
        let registry = TelemetryIntegratedToolRegistry::new("trace-001".to_string());
        assert_eq!(registry.binding_count(), 0);
        assert_eq!(registry.telemetry().event_count(), 0);
    }

    #[test]
    fn test_token_counter_creation() {
        let counter = TokenCounter::new();
        assert_eq!(counter.input_total(), 0);
        assert_eq!(counter.output_total(), 0);
    }

    #[test]
    fn test_token_counter_accumulation() {
        let counter = TokenCounter::new();
        let input_total = counter.add_input_tokens("hello world").unwrap();
        assert!(input_total > 0);
        
        let output_total = counter.add_output_tokens("result data").unwrap();
        assert!(output_total > 0);
        
        assert_eq!(counter.total_tokens(), input_total + output_total);
    }

    #[test]
    fn test_token_count_snapshot() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("test").unwrap();
        counter.add_output_tokens("result").unwrap();
        
        let snap = counter.snapshot();
        assert!(snap.total_tokens() > 0);
    }

    #[test]
    fn test_event_filter_creation() {
        let filter = EventFilter::accept_all();
        assert!(filter.event_types.is_empty());
        assert!(filter.actor_filter.is_none());
    }

    #[test]
    fn test_event_subscription_creation() {
        let filter = EventFilter::accept_all();
        let sub = EventSubscription::new(1, filter, "channel-1".to_string(), 1000);
        assert_eq!(sub.subscription_id, 1);
        assert!(sub.is_active);
    }

    #[test]
    fn test_subscription_manager_creation() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.subscription_count(), 0);
    }

    #[test]
    fn test_subscription_manager_subscribe() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        let sub_id = manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();
        assert_eq!(manager.subscription_count(), 1);
    }

    #[test]
    fn test_event_subscriber_with_routing() {
        let mut manager = SubscriptionManager::new();
        let filter = EventFilter::accept_all();
        manager.subscribe(filter, "channel-1".to_string(), 1000).unwrap();

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
        
        let count = manager.route_event(&event).unwrap();
        assert_eq!(count, 1);
    }
}
