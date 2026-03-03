// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Core Translation Layer Architecture
//!
//! Provides the foundational translation infrastructure for converting framework-specific
//! concepts and definitions into CSCI (Cognitive Substrate Core Interface) primitives.
//!
//! The translation layer orchestrates the multi-step process of analyzing framework input,
//! mapping concepts, building dependency graphs, and validating structural constraints.
//!
//! Sec 4.2: Translation Layer Architecture
//! Sec 4.2: Framework-to-CSCI Mapping Process
//! Sec 5.1: Translation Metrics and Fidelity

use std::collections::BTreeMap;
use crate::framework_type::FrameworkType;
use crate::AdapterError;

/// Unique identifier for a Cognitive Task
pub type CTID = u64;

/// Translation step enumeration representing discrete phases in the translation pipeline.
/// Sec 4.2: Translation Pipeline Phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationStep {
    /// Parse framework-specific input format
    ParseFrameworkInput,
    /// Map framework concepts to CSCI entities
    MapConcepts,
    /// Build Cognitive Task Directed Acyclic Graph (CT DAG)
    BuildCtDag,
    /// Validate structural dependencies and constraints
    ValidateDependencies,
    /// Emit executable syscalls for kernel
    EmitSyscalls,
}

impl TranslationStep {
    /// Returns string representation of the translation step.
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslationStep::ParseFrameworkInput => "parse_framework_input",
            TranslationStep::MapConcepts => "map_concepts",
            TranslationStep::BuildCtDag => "build_ct_dag",
            TranslationStep::ValidateDependencies => "validate_dependencies",
            TranslationStep::EmitSyscalls => "emit_syscalls",
        }
    }
}

/// Translation metrics tracking performance and fidelity characteristics.
/// Sec 5.1: Translation Performance Metrics
#[derive(Debug, Clone)]
pub struct TranslationMetrics {
    /// Number of translation steps executed
    pub steps_executed: usize,
    /// Number of framework concepts successfully mapped
    pub concepts_mapped: usize,
    /// Number of syscalls emitted for kernel
    pub syscalls_emitted: usize,
    /// Total translation time in nanoseconds
    pub translation_time_ns: u64,
    /// Overall translation accuracy (0-100)
    pub accuracy: u8,
    /// Source framework type
    pub source_framework: FrameworkType,
}

impl TranslationMetrics {
    /// Creates new translation metrics.
    /// Sec 4.2: Metrics Creation
    pub fn new(
        source_framework: FrameworkType,
        translation_time_ns: u64,
    ) -> Self {
        TranslationMetrics {
            steps_executed: 0,
            concepts_mapped: 0,
            syscalls_emitted: 0,
            translation_time_ns,
            accuracy: 100,
            source_framework,
        }
    }

    /// Returns true if translation achieved high fidelity (accuracy >= 90)
    /// Sec 5.1: Fidelity Assessment
    pub fn is_high_fidelity(&self) -> bool {
        self.accuracy >= 90
    }

    /// Increments the step execution counter.
    pub fn record_step_executed(&mut self) {
        self.steps_executed += 1;
    }

    /// Records a concept mapping event.
    pub fn record_concept_mapped(&mut self) {
        self.concepts_mapped += 1;
    }

    /// Records a syscall emission event.
    pub fn record_syscall_emitted(&mut self) {
        self.syscalls_emitted += 1;
    }

    /// Sets the final accuracy score.
    pub fn set_accuracy(&mut self, accuracy: u8) {
        self.accuracy = accuracy.min(100);
    }
}

/// Translation context providing configuration and state for the translation process.
/// Sec 4.2: Translation Context Structure
#[derive(Debug, Clone)]
pub struct TranslationContext {
    /// Source framework type
    pub source_framework: FrameworkType,
    /// Target syscall identifiers and types
    pub target_syscalls: Vec<String>,
    /// Translation metrics accumulator
    pub metrics: TranslationMetrics,
    /// Optional configuration metadata
    pub config: BTreeMap<String, String>,
}

impl TranslationContext {
    /// Creates a new translation context.
    /// Sec 4.2: Context Initialization
    pub fn new(source_framework: FrameworkType, translation_time_ns: u64) -> Self {
        TranslationContext {
            source_framework,
            target_syscalls: Vec::new(),
            metrics: TranslationMetrics::new(source_framework, translation_time_ns),
            config: BTreeMap::new(),
        }
    }

    /// Adds a target syscall to the translation context.
    pub fn add_target_syscall(&mut self, syscall: String) {
        self.target_syscalls.push(syscall);
    }

    /// Sets a configuration parameter.
    pub fn set_config(&mut self, key: String, value: String) {
        self.config.insert(key, value);
    }

    /// Gets a configuration parameter.
    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }
}

/// Common adapter interface pattern describing the universal translation contract.
/// Sec 4.2: Universal Adapter Pattern
#[derive(Debug, Clone)]
pub struct CommonAdapterInterfacePattern {
    /// The universal pattern name
    pub pattern_name: String,
    /// Description of the pattern
    pub description: String,
    /// List of required input stages
    pub required_input_stages: Vec<String>,
    /// List of required output stages
    pub required_output_stages: Vec<String>,
    /// Fidelity constraints
    pub min_fidelity_percentage: u8,
    /// Whether this pattern supports streaming
    pub supports_streaming: bool,
}

impl CommonAdapterInterfacePattern {
    /// Creates a new common adapter interface pattern.
    /// Sec 4.2: Pattern Definition
    pub fn new(pattern_name: String) -> Self {
        CommonAdapterInterfacePattern {
            pattern_name,
            description: String::new(),
            required_input_stages: Vec::new(),
            required_output_stages: Vec::new(),
            min_fidelity_percentage: 90,
            supports_streaming: false,
        }
    }

    /// Sets the description of the pattern.
    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    /// Adds a required input stage.
    pub fn add_required_input_stage(&mut self, stage: String) {
        self.required_input_stages.push(stage);
    }

    /// Adds a required output stage.
    pub fn add_required_output_stage(&mut self, stage: String) {
        self.required_output_stages.push(stage);
    }

    /// Sets streaming capability.
    pub fn set_streaming(&mut self, supports: bool) {
        self.supports_streaming = supports;
    }

    /// Returns true if the pattern is valid according to constraints.
    pub fn is_valid(&self) -> bool {
        !self.required_input_stages.is_empty() && !self.required_output_stages.is_empty()
    }
}

/// Translation pipeline trait defining the contract for translation implementations.
/// Sec 4.2: TranslationPipeline Interface
pub trait TranslationPipeline {
    /// Translates a framework chain definition to a CT DAG.
    /// Sec 4.2: Chain Translation Method
    ///
    /// # Arguments
    /// * `chain_def` - The framework-specific chain definition
    /// * `context` - The translation context
    ///
    /// # Returns
    /// The translated CT DAG or an AdapterError
    fn translate_chain(&self, chain_def: &str, context: &mut TranslationContext) -> Result<String, AdapterError>;

    /// Returns the framework type this pipeline handles.
    /// Sec 4.2: Pipeline Type Declaration
    fn framework_type(&self) -> FrameworkType;

    /// Returns the translation metrics for the last operation.
    /// Sec 5.1: Metrics Retrieval
    fn get_metrics(&self) -> TranslationMetrics;
}

#[cfg(test)]
mod tests {
    use super::*;
use std::collections::BTreeMap;

    #[test]
    fn test_translation_step_as_str() {
        assert_eq!(TranslationStep::ParseFrameworkInput.as_str(), "parse_framework_input");
        assert_eq!(TranslationStep::MapConcepts.as_str(), "map_concepts");
        assert_eq!(TranslationStep::BuildCtDag.as_str(), "build_ct_dag");
        assert_eq!(TranslationStep::ValidateDependencies.as_str(), "validate_dependencies");
        assert_eq!(TranslationStep::EmitSyscalls.as_str(), "emit_syscalls");
    }

    #[test]
    fn test_translation_metrics_creation() {
        let metrics = TranslationMetrics::new(FrameworkType::LangChain, 5000);
        assert_eq!(metrics.steps_executed, 0);
        assert_eq!(metrics.concepts_mapped, 0);
        assert_eq!(metrics.syscalls_emitted, 0);
        assert_eq!(metrics.translation_time_ns, 5000);
        assert_eq!(metrics.accuracy, 100);
        assert!(metrics.is_high_fidelity());
    }

    #[test]
    fn test_translation_metrics_recording() {
        let mut metrics = TranslationMetrics::new(FrameworkType::LangChain, 5000);
        metrics.record_step_executed();
        metrics.record_concept_mapped();
        metrics.record_syscall_emitted();

        assert_eq!(metrics.steps_executed, 1);
        assert_eq!(metrics.concepts_mapped, 1);
        assert_eq!(metrics.syscalls_emitted, 1);
    }

    #[test]
    fn test_translation_metrics_fidelity_threshold() {
        let mut high_fidelity = TranslationMetrics::new(FrameworkType::LangChain, 1000);
        high_fidelity.set_accuracy(90);
        assert!(high_fidelity.is_high_fidelity());

        let mut low_fidelity = TranslationMetrics::new(FrameworkType::LangChain, 1000);
        low_fidelity.set_accuracy(75);
        assert!(!low_fidelity.is_high_fidelity());
    }

    #[test]
    fn test_translation_metrics_accuracy_clamping() {
        let mut metrics = TranslationMetrics::new(FrameworkType::LangChain, 1000);
        metrics.set_accuracy(150);
        assert_eq!(metrics.accuracy, 100);
    }

    #[test]
    fn test_translation_context_creation() {
        let ctx = TranslationContext::new(FrameworkType::LangChain, 5000);
        assert_eq!(ctx.source_framework, FrameworkType::LangChain);
        assert!(ctx.target_syscalls.is_empty());
        assert!(ctx.config.is_empty());
    }

    #[test]
    fn test_translation_context_syscall_management() {
        let mut ctx = TranslationContext::new(FrameworkType::LangChain, 5000);
        ctx.add_target_syscall("ct_create".into());
        ctx.add_target_syscall("tool_invoke".into());

        assert_eq!(ctx.target_syscalls.len(), 2);
        assert_eq!(ctx.target_syscalls[0], "ct_create");
    }

    #[test]
    fn test_translation_context_config_management() {
        let mut ctx = TranslationContext::new(FrameworkType::LangChain, 5000);
        ctx.set_config("timeout_ms".into(), "5000".into());
        ctx.set_config("retries".into(), "3".into());

        assert_eq!(ctx.get_config("timeout_ms"), Some(&"5000".into()));
        assert_eq!(ctx.get_config("retries"), Some(&"3".into()));
        assert_eq!(ctx.get_config("nonexistent"), None);
    }

    #[test]
    fn test_common_adapter_interface_pattern_creation() {
        let pattern = CommonAdapterInterfacePattern::new("chain_to_dag".into());
        assert_eq!(pattern.pattern_name, "chain_to_dag");
        assert_eq!(pattern.min_fidelity_percentage, 90);
        assert!(!pattern.supports_streaming);
    }

    #[test]
    fn test_common_adapter_interface_pattern_configuration() {
        let mut pattern = CommonAdapterInterfacePattern::new("chain_to_dag".into());
        pattern.set_description("Translates linear chains to DAGs".into());
        pattern.add_required_input_stage("parse".into());
        pattern.add_required_output_stage("validate".into());
        pattern.set_streaming(true);

        assert_eq!(pattern.description, "Translates linear chains to DAGs");
        assert_eq!(pattern.required_input_stages.len(), 1);
        assert_eq!(pattern.required_output_stages.len(), 1);
        assert!(pattern.supports_streaming);
        assert!(pattern.is_valid());
    }

    #[test]
    fn test_common_adapter_interface_pattern_validity() {
        let mut pattern = CommonAdapterInterfacePattern::new("test_pattern".into());
        assert!(!pattern.is_valid());

        pattern.add_required_input_stage("input".into());
        assert!(!pattern.is_valid());

        pattern.add_required_output_stage("output".into());
        assert!(pattern.is_valid());
    }
}
