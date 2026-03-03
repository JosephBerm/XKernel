// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Adapters Runtime Subsystem
//!
//! Provides translation and mapping between external framework concepts (LangChain, Semantic Kernel,
//! CrewAI, AutoGen) and Cognitive Substrate Core Interface (CSCI) primitives.
//!
//! This crate implements the foundational concept mapping matrix and adapter types that enable
//! framework interoperability with the cognitive kernel.
//!
//! ## Architecture
//!
//! - **Concept Mapping**: Framework-specific concepts are mapped to CSCI entities via a bidirectional
//!   mapping matrix with fidelity tracking.
//! - **Adapter Protocol**: The `IFrameworkAdapter` trait defines translation methods for converting
//!   framework tasks, memory, tools, and communication patterns to CSCI equivalents.
//! - **Framework Type Enumeration**: Support for LangChain, Semantic Kernel, CrewAI, AutoGen, and
//!   custom frameworks.
//! - **Universal Adapter Pattern**: The `UniversalFrameworkAdapter` trait provides a unified lifecycle
//!   and protocol for all framework implementations.
//! - **SK-Specific Modules**: Advanced Semantic Kernel support with plugin/skill mapping, planner
//!   translation to CT spawners, and memory tier mapping.
//! - **Week 5 Enhancements**: Adapter interface contract, syscall binding, utilities, and testing infrastructure.
//!
//! ## Engineering Plan References
//!
//! - Section 4.2: Framework Adapter Interfaces
//! - Section 4.3: Concept Mapping Matrix
//! - Section 5.1: Translation Fidelity Tracking
//! - Section 5.2: Adapter Interface Contract & Week 5 Deliverables
//! - Week 4: Advanced SK Adapter Architecture
//! - Week 5: Framework Adapter Interface Contracts

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::{string::String, vec::Vec};

pub mod adapter_base;
pub mod adapter_impl;
pub mod translation;
pub mod migration;
pub mod adapter;
pub mod adapter_cache;
pub mod adapter_guide;
pub mod adapter_interface_contract;
pub mod adapter_test_infra;
pub mod adapter_utilities;
pub mod autogen;
pub mod chain_to_dag;
pub mod common_adapter_pattern;
pub mod crewai;
pub mod entity_lifecycle;
pub mod error;
pub mod framework_type;
pub mod ipc_format;
pub mod langchain;
pub mod mapping;
pub mod memory_model;
pub mod memory_translation;
pub mod runtime_adapter_ref;
pub mod semantic_kernel;
pub mod sk_adapter;
pub mod sk_memory_mapping;
pub mod sk_planner_translation;
pub mod syscall_binding;
pub mod tool_translation;
pub mod translation_layer;
pub mod cef_event_integration;
pub mod common_utility_lib;
pub mod langchain_adapter_v2;
pub mod runtime_adapter_ref_v2;
pub mod syscall_binding_layer;

pub use adapter_base::{AgentHandle, AdapterConfig, AdapterLifecycleState, AdapterError, AdapterResult, FrameworkAdapter, P95_LATENCY_TARGET_MS, MAX_MEMORY_PER_AGENT_MB};
pub use adapter_impl::{LangChainAdapter, SemanticKernelAdapter, AutoGenAdapter, CrewAIAdapter, CustomAdapter};
pub use translation::{CefEventTranslator, CapabilityMapping};
pub use migration::{AdapterMigrationTool, MigrationResult};
pub use adapter::IFrameworkAdapter;
pub use adapter_cache::{AdapterCache, CachedTranslation, CacheStats};
pub use adapter_guide::{
    AdapterImplementationGuide, ConceptMapping, MappingFidelityLevel, AdapterBestPractices,
    AdapterImplementationTemplate,
};
pub use adapter_interface_contract::{
    RuntimeAdapterContract, AdapterState, FrameworkAgentConfig, FrameworkChainDefinition,
    ChainStepDefinition, FrameworkResultItem, AdapterErrorInfo, ErrorRecoveryAction,
    AdapterBuilder, AdapterConfig,
};
pub use adapter_test_infra::{
    MockKernelIpc, TestAgent, TestChain, AdapterAssertions, AdapterLifecycleTestScenario,
};
pub use adapter_utilities::{
    ResultAggregator, SerializationHelper, ValidationHelper, TranslationMetricsHelper,
    ErrorHandlingHelper,
};
pub use chain_to_dag::{
    ChainDefinition, ChainStep, ChainType, CtDag, CtNode, DagEdge, EdgeType,
    translate_sequential, translate_router, translate_map_reduce, validate_dag,
};
pub use common_adapter_pattern::{
    UniversalFrameworkAdapter, AdapterLifecycleState, AdapterConfig as AdapterConfigLegacy,
    LangChainUniversalAdapter, SemanticKernelUniversalAdapter, AutoGenUniversalAdapter,
    CrewAIUniversalAdapter, CustomFrameworkUniversalAdapter,
};
pub use entity_lifecycle::{
    EntityLifecycle, CTLifecycleState, AgentLifecycleState, CrewLifecycleState,
    ChannelLifecycleState, CapabilityLifecycleState, MemoryLifecycleState,
};
pub use error::{AdapterError, AdapterResult};
pub use framework_type::FrameworkType;
pub use ipc_format::{AdapterMessage, KernelResponse, MessageEnvelope, SerializationHint};
pub use mapping::{ConceptMapping, CsciEntity, FrameworkConcept, MappingMatrix, MappingFidelity};
pub use memory_model::{FrameworkMemoryType, MemoryTier, MemoryMappingRegistry, TierMapping};
pub use memory_translation::{
    FrameworkMemory, MemoryTypeClass, MemoryMapping, MemorySyscall, IndexingConfig,
    translate_memory,
};
pub use runtime_adapter_ref::{RuntimeAdapterRef, CTPhase, CTConfig, MemoryConfig, ToolBindingConfig, IpcConfig, TranslationMetrics};
pub use semantic_kernel::SemanticKernelAdapter;
pub use sk_adapter::{
    SemanticKernelAdvancedAdapter, SkPlugin, SkFunction, SkPlan, SkPlanStep,
    SkKernelMemory, SkMemoryBufferType, SkContextVariables,
    CtSpawnerDirective, CtSpawnTask, MemoryTierMapping,
};
pub use sk_memory_mapping::{
    SkMemoryMapper, SkMemoryBuffer, SkBufferType, SkPersistence, AccessPattern,
    CtMemoryTier, MemoryTierMap, L2EpisodicSnapshot, L3SemanticRecord,
};
pub use sk_planner_translation::{
    SkPlannerTranslator, PlannerStep, CtSpawnRequest, TaskDag, DependencyEdge, StepId,
};
pub use syscall_binding::{
    CsciSyscallId, SyscallRequest, SyscallResponse, SyscallBinding, MockSyscallBinding,
};
pub use tool_translation::{
    FrameworkTool, ToolBindingConfig as ToolBindingConfigFromTool, SandboxConfig, EffectClass,
    translate_tool, LangChainToolTranslator, SemanticKernelToolTranslator, CrewAIToolTranslator,
    AutoGenToolTranslator, ArgumentSerializer, ResultDeserializer,
};
pub use translation_layer::{
    TranslationContext, TranslationMetrics as TranslationLayerMetrics, TranslationStep,
    CommonAdapterInterfacePattern, TranslationPipeline,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_adapter_module_exports() {
        // Verify public API is accessible
        let _: AdapterResult<()> = Ok(());
    }

    #[test]
    fn test_adapter_interface_contract_exports() {
        // Verify adapter interface contract exports
        let _state = AdapterState::Initialized;
        let _action = ErrorRecoveryAction::Retry;
    }

    #[test]
    fn test_adapter_utilities_exports() {
        // Verify adapter utilities are exported
        let _agg: ResultAggregator<String> = ResultAggregator::new(true);
        let _metrics = TranslationMetricsHelper::new();
    }

    #[test]
    fn test_syscall_binding_exports() {
        // Verify syscall binding exports
        let _syscall = CsciSyscallId::MemWrite;
        let _mock = MockSyscallBinding::new();
    }

    #[test]
    fn test_adapter_test_infra_exports() {
        // Verify test infrastructure exports
        let _ipc = MockKernelIpc::new();
        let _agent = TestAgent::simple("Test");
        let _chain = TestChain::sequential(1);
        let _scenario = AdapterLifecycleTestScenario::full_lifecycle();
    }

    #[test]
    fn test_universal_adapter_pattern_exports() {
        // Verify all framework adapters are exported
        let _lc = LangChainUniversalAdapter::new();
        let _sk = SemanticKernelUniversalAdapter::new();
        let _ag = AutoGenUniversalAdapter::new();
        let _crew = CrewAIUniversalAdapter::new();
        let _custom = CustomFrameworkUniversalAdapter::new("test".into());
    }

    #[test]
    fn test_sk_adapter_exports() {
        // Verify SK-specific exports
        let _advanced = SemanticKernelAdvancedAdapter::new();
        let _translator = SkPlannerTranslator;
        let _mapper = SkMemoryMapper;
    }

    #[test]
    fn test_framework_types_exportable() {
        let _ft = FrameworkType::LangChain;
        let _ft = FrameworkType::SemanticKernel;
        let _ft = FrameworkType::CrewAI;
        let _ft = FrameworkType::AutoGen;
    }

    #[test]
    fn test_adapter_error_types() {
        use error::AdapterError;
use alloc::string::String;
use alloc::string::ToString;
        
        let err1 = AdapterError::TranslationError("test".into());
        let err2 = AdapterError::FrameworkCompatibilityError("test".into());
        let err3 = AdapterError::KernelIpcError("test".into());
        
        assert!(!err1.to_string().is_empty());
        assert!(!err2.to_string().is_empty());
        assert!(!err3.to_string().is_empty());
    }
}
