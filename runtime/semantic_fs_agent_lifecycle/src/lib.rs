// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # CS Semantic FS Agent Lifecycle - Agent Lifecycle Management
//!
//! This crate implements the semantic filesystem foundation and agent lifecycle management
//! for the Cognitive Substrate OS. It provides strongly-typed domain models for agent
//! lifecycle configuration, health checks, restart policies, dependency ordering, and
//! agent unit file specifications.
//!
//! ## No-std Environment
//!
//! This crate is `#![no_std]` for use in kernel contexts. All heap allocations use
//! `alloc` crate collections.
//!
//! ## Design Principles
//!
//! - **Type Safety**: Illegal states are unrepresentable via Rust's type system
//! - **Declarative Configuration**: Agent Unit Files provide declarative agent specifications
//! - **Health Probes**: Readiness and liveness probes for continuous health monitoring
//! - **Dependency Resolution**: Graph-based dependency ordering with cycle detection
//! - **Kubernetes Patterns**: Restart policies and backoff strategies align with K8s

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod semantic_fs;
pub mod agent_lifecycle;
pub mod mounts;
pub mod cli;
pub mod agent_start;
pub mod agent_stop;
pub mod ct_spawn_integration;
pub mod cs_agentctl_stub;
pub mod dependency;
pub mod error;
pub mod health_check;
pub mod health_status;
pub mod lifecycle;
pub mod lifecycle_integration_tests;
pub mod lifecycle_logging;
pub mod lifecycle_manager;
pub mod phase1_readiness;
pub mod restart_policy;
pub mod synthesis;
pub mod unit_file;
pub mod unit_file_examples;
pub mod unit_file_migration;
pub mod unit_file_parser;
pub mod unit_file_rfc;
pub mod unit_file_schema;
pub mod unit_file_test_suite;
pub mod unit_file_validator;

// Core exports from agent_start module
pub use agent_start::{AgentStartHandler, AgentStartParams, CtSpawnResult};

// Core exports from agent_stop module
pub use agent_stop::{AgentStopHandler, AgentStopParams, SignalResult, TerminationSignal};

// Core exports from ct_spawn_integration module
pub use ct_spawn_integration::{CtSpawnParams, CtSpawnTranslator, QuotaPolicy};

// Core exports from cs_agentctl_stub module
pub use cs_agentctl_stub::{
    AgentStatusSummary, CliArgs, CliCommandProcessor, Subcommand,
};

// Core exports from dependency module
pub use dependency::{
    CrewDependencyContext, CrewMembership, DependencyGraph, DependencySpec, DependencyState,
    OrderingConstraint, ParallelStartGroups,
};

// Core exports from error module
pub use error::{LifecycleError, Result};

// Core exports from semantic_fs module
pub use semantic_fs::{QueryEngine, TagSystem, PathResolver, MountManager};

// Core exports from agent_lifecycle module
pub use agent_lifecycle::{AgentUnit, AgentState, HealthProbe, RestartPolicy, StateMachine};

// Core exports from mounts module
pub use mounts::{MountProvider, LocalMount, HttpMount, S3Mount, DatabaseMount, CustomPluginMount};

// Core exports from cli module
pub use cli::AgentCtl;

// Core exports from health_check module
pub use health_check::{
    AgentHealthStatus, HealthCheckConfig, HealthCheckType, HealthEndpoint, HealthHistory,
    HealthProbe, HealthProbeType, ProbeResult, ProbeSchedule,
};

// Core exports from health_status module
pub use health_status::{
    AgentHealthStatus as AgentHealthStatus2, HealthEvent, HealthMetrics, HealthState,
    HealthStatusAggregator,
};

// Core exports from lifecycle module
pub use lifecycle::{LifecycleConfig, LifecycleState};

// Core exports from lifecycle_logging module
pub use lifecycle_logging::{
    EventType, LifecycleLogger, LogEntry, LogLevel, RotationPolicy,
};

// Core exports from lifecycle_manager module
pub use lifecycle_manager::{AgentLifecycleState, LifecycleManager};

// Core exports from phase1_readiness module
pub use phase1_readiness::{
    ComponentDependency, MigrationChecklistItem, Phase0Completion, Phase1ReadinessAssessment,
    ReadinessGap, ReadinessStatus, RiskItem,
};

// Core exports from restart_policy module
pub use restart_policy::{
    AlwaysRestartPolicy, BackoffConfig, FailureInfo, NeverRestartPolicy, OnFailureRestartPolicy,
    RestartDecision, RestartDecisionOutcome, RestartHistory, RestartPolicy, RestartPolicyEngine,
};

// Core exports from synthesis module
pub use synthesis::{
    FeatureMapping, FeatureParityMatrix, GapAnalysis, MappingType, UnitFileRequirements,
};

// Core exports from unit_file module
pub use unit_file::{AgentUnitFile, ModelConfig, ResourceLimits, UnitFileMetadata};

// Core exports from unit_file_schema module
pub use unit_file_schema::{
    AgentFramework, AgentSection, AgentUnitFileSchema, CapabilitiesSection, CrewSection,
    DependenciesSection, HealthCheckSection, HealthCheckType, ModelSection, ResourcesSection,
    RestartPolicyType, RestartSection, UnitFileError, UnitFileResult,
};

// Core exports from unit_file_examples module
pub use unit_file_examples::{
    EXAMPLE_CREW_COORDINATOR, EXAMPLE_CREW_WORKER, EXAMPLE_HEALTH_CHECKED, EXAMPLE_HIGH_RESOURCE,
    EXAMPLE_LANGCHAIN_AGENT, EXAMPLE_MULTI_DEPENDENCY, EXAMPLE_RESTART_POLICY, EXAMPLE_SIMPLE_AGENT,
};

// Core exports from unit_file_parser module
pub use unit_file_parser::{ParseError, ParseResult, UnitFileParser};

// Core exports from unit_file_rfc module
pub use unit_file_rfc::{
    complete_rfc, DEFAULTS, FIELD_DESCRIPTIONS, RFC_ABSTRACT, RFC_BACKWARD_COMPATIBILITY,
    RFC_MOTIVATION, RFC_SECURITY, RFC_SPECIFICATION,
};

// Core exports from unit_file_migration module
pub use unit_file_migration::{
    EnvironmentMigrator, LegacyEnvConfig, MigrationError, MigrationResult, LEGACY_FIELD_MAPPING,
};

// Core exports from unit_file_test_suite module
pub use unit_file_test_suite::{
    validate_all_test_cases, ALL_TEST_CASES, TEST_AGENT_WITH_TAGS, TEST_ANTHROPIC_RESOURCES,
    TEST_CAPABILITIES_EXTENDED, TEST_COMPLETE_RICH_AGENT, TEST_CREW_COORDINATOR, TEST_CREW_MEMBERSHIP,
    TEST_CUSTOM_FRAMEWORK, TEST_EDGE_CASE_LARGE_TOKENS, TEST_EDGE_CASE_ZERO_RETRY,
    TEST_ENVIRONMENT_VARIABLES, TEST_HEALTH_CHECK_CSCI, TEST_HEALTH_CHECK_EXEC,
    TEST_HEALTH_CHECK_HTTP, TEST_HEALTH_CHECK_TCP, TEST_LANGCHAIN_FULL, TEST_LEGACY_MEMORY_CPU,
    TEST_MINIMAL_VALID, TEST_MODEL_MINIMAL, TEST_RESTART_POLICY_ALWAYS, TEST_RESTART_POLICY_ONFAIL,
    TEST_RESTART_POLICY_NEVER, TEST_DEPENDENCIES_ORDERING,
};

// Core exports from unit_file_validator module
pub use unit_file_validator::{
    CapabilityExistenceRule, DependencyConsistencyRule, HealthCheckRule, RequiredFieldsRule,
    ResourceLimitsRule, RestartPolicyRule, ValidationEngine, ValidationError, ValidationResult,
    ValidationRule,
};
