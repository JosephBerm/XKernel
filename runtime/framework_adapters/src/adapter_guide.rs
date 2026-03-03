// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Adapter Development Guide
//!
//! Comprehensive guide for implementing custom framework adapters for the Cognitive Substrate OS.
//! This module documents the adapter architecture, patterns, and best practices.
//!
//! ## How to Implement a Framework Adapter
//!
//! A framework adapter bridges between an external framework's concepts and the Cognitive Substrate
//! Core Interface (CSCI) execution model. The adapter implements the UniversalFrameworkAdapter trait.
//!
//! ### Step 1: Understand Your Framework
//! - Identify key concepts: tasks, tools, memory, channels
//! - Document the framework's execution model
//! - Map framework concepts to CSCI entities
//!
//! ### Step 2: Implement the Adapter Struct
//! - Create a new struct implementing UniversalFrameworkAdapter
//! - Track adapter state and loaded entities
//! - Maintain configuration
//!
//! ### Step 3: Implement Lifecycle Methods
//! - `initialize()`: Set up adapter with configuration
//! - `load_agent()`: Load framework entities into adapter
//! - `translate_plan()`: Convert framework plans to CT spawner directives
//! - `spawn_tasks()`: Request task execution
//! - `collect_results()`: Gather and format results
//! - `get_state()` and `shutdown()`: State management
//!
//! ### Step 4: Handle Error Cases
//! - Use AdapterError for framework-specific errors
//! - Validate input at adapter boundaries
//! - Provide clear error messages for debugging
//!
//! ### Step 5: Test Your Implementation
//! - Test each lifecycle method
//! - Test error cases
//! - Verify round-trip translation accuracy
//!
//! Sec 4.2: Adapter Implementation Pattern
//! Sec 4.2: Universal Adapter Interface
//! Sec 4.3: Framework Concept Mapping

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::error::AdapterError;
use crate::AdapterResult;

/// Adapter implementation documentation and reference patterns.
/// Sec 4.2: Adapter Development Guide
#[derive(Debug, Clone)]
pub struct AdapterImplementationGuide {
    /// Framework name
    pub framework_name: String,
    /// Implementation version
    pub version: String,
    /// Key concept mappings
    pub concept_mappings: Vec<AdapterGuideConceptMapping>,
    /// Implementation notes
    pub implementation_notes: String,
}

impl AdapterImplementationGuide {
    /// Creates a new adapter implementation guide.
    pub fn new(framework_name: String) -> Self {
        AdapterImplementationGuide {
            framework_name,
            version: "1.0.0".to_string(),
            concept_mappings: Vec::new(),
            implementation_notes: String::new(),
        }
    }

    /// Adds a concept mapping to the guide.
    pub fn add_concept_mapping(&mut self, mapping: AdapterGuideConceptMapping) {
        self.concept_mappings.push(mapping);
    }

    /// Sets the implementation notes.
    pub fn set_notes(&mut self, notes: String) {
        self.implementation_notes = notes;
    }
}

/// Documentation of a single concept mapping.
/// Sec 4.3: Concept Mapping Documentation
#[derive(Debug, Clone)]
pub struct AdapterGuideConceptMapping {
    /// Framework concept name
    pub framework_concept: String,
    /// CSCI entity type this maps to
    pub csci_entity: String,
    /// Mapping fidelity level
    pub fidelity: MappingFidelityLevel,
    /// Description of the mapping
    pub description: String,
    /// Example framework input
    pub example_input: Option<String>,
    /// Example CSCI output
    pub example_output: Option<String>,
}

impl AdapterGuideConceptMapping {
    /// Creates a new concept mapping.
    pub fn new(
        framework_concept: String,
        csci_entity: String,
        fidelity: MappingFidelityLevel,
    ) -> Self {
        AdapterGuideConceptMapping {
            framework_concept,
            csci_entity,
            fidelity,
            description: String::new(),
            example_input: None,
            example_output: None,
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Sets example input.
    pub fn with_example_input(mut self, input: String) -> Self {
        self.example_input = Some(input);
        self
    }

    /// Sets example output.
    pub fn with_example_output(mut self, output: String) -> Self {
        self.example_output = Some(output);
        self
    }
}

/// Mapping fidelity classification.
/// Sec 4.3: Fidelity Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MappingFidelityLevel {
    /// Full semantic preservation during mapping
    Full,
    /// Most semantics preserved, minor adaptations
    High,
    /// Reasonable semantic coverage
    Moderate,
    /// Some information loss during mapping
    Low,
    /// Significant information loss
    Partial,
}

impl MappingFidelityLevel {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MappingFidelityLevel::Full => "full",
            MappingFidelityLevel::High => "high",
            MappingFidelityLevel::Moderate => "moderate",
            MappingFidelityLevel::Low => "low",
            MappingFidelityLevel::Partial => "partial",
        }
    }

    /// Returns fidelity percentage (0-100).
    pub fn percentage(&self) -> u8 {
        match self {
            MappingFidelityLevel::Full => 100,
            MappingFidelityLevel::High => 90,
            MappingFidelityLevel::Moderate => 75,
            MappingFidelityLevel::Low => 60,
            MappingFidelityLevel::Partial => 40,
        }
    }
}

/// Best practices and anti-patterns for adapter implementation.
/// Sec 4.2: Adapter Best Practices
#[derive(Debug, Clone)]
pub struct AdapterBestPractices {
    /// Do implement proper error handling
    pub do_error_handling: String,
    /// Do validate inputs at boundaries
    pub do_input_validation: String,
    /// Do use Result types exclusively
    pub do_use_results: String,
    /// Do track adapter state
    pub do_track_state: String,
    /// Don't use unwrap() or expect()
    pub dont_panic_macros: String,
    /// Don't ignore error cases
    pub dont_ignore_errors: String,
    /// Don't create circular dependencies
    pub dont_circular_deps: String,
}

impl Default for AdapterBestPractices {
    fn default() -> Self {
        AdapterBestPractices {
            do_error_handling: alloc::string::String::from(
                "Always return Result<T, AdapterError> from public methods"
            ),
            do_input_validation: alloc::string::String::from(
                "Validate framework artifacts against expected schemas at adapter boundaries"
            ),
            do_use_results: alloc::string::String::from(
                "Use Result type for all fallible operations, avoid unwrap() and expect()"
            ),
            do_track_state: alloc::string::String::from(
                "Maintain adapter state transitions through lifecycle methods"
            ),
            dont_panic_macros: alloc::string::String::from(
                "Never use unwrap(), expect(), panic!() - return Err instead"
            ),
            dont_ignore_errors: alloc::string::String::from(
                "Every Result must be handled - never ignore errors silently"
            ),
            dont_circular_deps: alloc::string::String::from(
                "Avoid circular dependencies between concepts and entities"
            ),
        }
    }
}

/// Complete template for a new framework adapter.
/// Sec 4.2: Adapter Implementation Template
pub struct AdapterImplementationTemplate;

impl AdapterImplementationTemplate {
    /// Returns a template for a new adapter struct.
    pub fn struct_template() -> &'static str {
        r#"
/// My Framework adapter implementing the universal pattern.
#[derive(Debug, Clone)]
pub struct MyFrameworkUniversalAdapter {
    state: AdapterLifecycleState,
    config: Option<AdapterConfig>,
    loaded_entities: BTreeMap<String, String>,
}

impl MyFrameworkUniversalAdapter {
    /// Creates a new adapter instance.
    pub fn new() -> Self {
        MyFrameworkUniversalAdapter {
            state: AdapterLifecycleState::Uninitialized,
            config: None,
            loaded_entities: BTreeMap::new(),
        }
    }
}
"#
    }

    /// Returns a template for implementing the trait.
    pub fn trait_impl_template() -> &'static str {
        r#"
impl UniversalFrameworkAdapter for MyFrameworkUniversalAdapter {
    fn initialize(&mut self, config: AdapterConfig) -> AdapterResult<()> {
        // Validate configuration
        // Set up internal state
        self.config = Some(config);
        self.state = AdapterLifecycleState::Ready;
        Ok(())
    }

    fn load_agent(&self, agent_definition: &str) -> AdapterResult<String> {
        if agent_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty definition".into()));
        }
        // Parse and translate framework agent
        Ok(format!("mf-agent-{}", agent_definition))
    }

    fn translate_plan(&self, plan_definition: &str) -> AdapterResult<String> {
        if plan_definition.is_empty() {
            return Err(AdapterError::TranslationError("Empty plan".into()));
        }
        // Convert framework plan to CT spawner
        Ok(format!("mf-spawner-{}", plan_definition))
    }

    fn spawn_tasks(&self, spawn_directive: &str) -> AdapterResult<Vec<String>> {
        // Request kernel to spawn tasks
        let task_id = format!("mf-task-{}", spawn_directive);
        Ok(vec![task_id])
    }

    fn collect_results(&self, task_ids: &[String]) -> AdapterResult<String> {
        if task_ids.is_empty() {
            return Err(AdapterError::TranslationError("No task IDs".into()));
        }
        // Gather results from completed tasks
        Ok(format!("mf-result-{}", task_ids.len()))
    }

    fn get_state(&self) -> AdapterLifecycleState {
        self.state
    }

    fn shutdown(&mut self) -> AdapterResult<()> {
        self.state = AdapterLifecycleState::Shutdown;
        self.config = None;
        self.loaded_entities.clear();
        Ok(())
    }
}
"#
    }

    /// Returns a template for test module.
    pub fn test_template() -> &'static str {
        r#"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_initialization() {
        let mut adapter = MyFrameworkUniversalAdapter::new();
        let config = AdapterConfig::new("1.0.0".into());
        let result = adapter.initialize(config);
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Ready);
    }

    #[test]
    fn test_load_agent() {
        let adapter = MyFrameworkUniversalAdapter::new();
        let result = adapter.load_agent("agent_def");
        assert!(result.is_ok());
    }

    #[test]
    fn test_translate_plan() {
        let adapter = MyFrameworkUniversalAdapter::new();
        let result = adapter.translate_plan("plan_def");
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_input_errors() {
        let adapter = MyFrameworkUniversalAdapter::new();
        assert!(adapter.load_agent("").is_err());
        assert!(adapter.translate_plan("").is_err());
    }

    #[test]
    fn test_shutdown() {
        let mut adapter = MyFrameworkUniversalAdapter::new();
        let config = AdapterConfig::new("1.0.0".into());
        let _ = adapter.initialize(config);
        let result = adapter.shutdown();
        assert!(result.is_ok());
        assert_eq!(adapter.get_state(), AdapterLifecycleState::Shutdown);
    }
}
"#
    }

    /// Returns common pitfalls and solutions.
    pub fn pitfalls_and_solutions() -> Vec<(String, String)> {
        vec![
            (
                "Not validating input at adapter boundary".into(),
                "Always check for empty/invalid input and return Err with descriptive message".into(),
            ),
            (
                "Using unwrap()/expect() instead of Result".into(),
                "Use AdapterError::TranslationError or framework-specific error variant".into(),
            ),
            (
                "Losing fidelity without documenting".into(),
                "Document mapping fidelity in comments and test round-trip accuracy".into(),
            ),
            (
                "Not handling state transitions".into(),
                "Track state in lifecycle methods, validate state in operations".into(),
            ),
            (
                "Ignoring error propagation".into(),
                "Use ? operator or explicit match to propagate errors up the call stack".into(),
            ),
            (
                "Hardcoding framework-specific paths".into(),
                "Use configuration parameters for framework-specific settings".into(),
            ),
            (
                "Not testing error cases".into(),
                "Write tests for empty inputs, invalid states, and expected errors".into(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_adapter_implementation_guide_creation() {
        let guide = AdapterImplementationGuide::new("TestFramework".into());
        assert_eq!(guide.framework_name, "TestFramework");
        assert!(guide.concept_mappings.is_empty());
    }

    #[test]
    fn test_concept_mapping_creation() {
        let mapping = AdapterGuideConceptMapping::new(
            "Task".into(),
            "CognitiveTask".into(),
            MappingFidelityLevel::Full,
        );
        assert_eq!(mapping.framework_concept, "Task");
        assert_eq!(mapping.csci_entity, "CognitiveTask");
        assert_eq!(mapping.fidelity, MappingFidelityLevel::Full);
    }

    #[test]
    fn test_concept_mapping_builder_pattern() {
        let mapping = AdapterGuideConceptMapping::new(
            "Plugin".into(),
            "ToolBinding".into(),
            MappingFidelityLevel::High,
        )
        .with_description("Maps plugins to tool bindings".into())
        .with_example_input("plugin_def".into())
        .with_example_output("tool_binding_config".into());

        assert_eq!(mapping.description, "Maps plugins to tool bindings");
        assert!(mapping.example_input.is_some());
        assert!(mapping.example_output.is_some());
    }

    #[test]
    fn test_fidelity_level_as_str() {
        assert_eq!(MappingFidelityLevel::Full.as_str(), "full");
        assert_eq!(MappingFidelityLevel::Partial.as_str(), "partial");
    }

    #[test]
    fn test_fidelity_level_percentage() {
        assert_eq!(MappingFidelityLevel::Full.percentage(), 100);
        assert_eq!(MappingFidelityLevel::High.percentage(), 90);
        assert_eq!(MappingFidelityLevel::Moderate.percentage(), 75);
        assert_eq!(MappingFidelityLevel::Low.percentage(), 60);
        assert_eq!(MappingFidelityLevel::Partial.percentage(), 40);
    }

    #[test]
    fn test_best_practices_default() {
        let practices = AdapterBestPractices::default();
        assert!(!practices.do_error_handling.is_empty());
        assert!(!practices.dont_panic_macros.is_empty());
    }

    #[test]
    fn test_adapter_templates_exist() {
        let struct_tmpl = AdapterImplementationTemplate::struct_template();
        assert!(struct_tmpl.contains("pub struct"));

        let trait_tmpl = AdapterImplementationTemplate::trait_impl_template();
        assert!(trait_tmpl.contains("impl UniversalFrameworkAdapter"));

        let test_tmpl = AdapterImplementationTemplate::test_template();
        assert!(test_tmpl.contains("#[test]"));
    }

    #[test]
    fn test_pitfalls_and_solutions() {
        let pitfalls = AdapterImplementationTemplate::pitfalls_and_solutions();
        assert!(!pitfalls.is_empty());
        assert!(pitfalls.iter().all(|(p, s)| !p.is_empty() && !s.is_empty()));
    }

    #[test]
    fn test_guide_add_concept_mapping() {
        let mut guide = AdapterImplementationGuide::new("TestFramework".into());
        let mapping = AdapterGuideConceptMapping::new(
            "Entity".into(),
            "CsciEntity".into(),
            MappingFidelityLevel::Full,
        );
        guide.add_concept_mapping(mapping);
        assert_eq!(guide.concept_mappings.len(), 1);
    }

    #[test]
    fn test_guide_set_notes() {
        let mut guide = AdapterImplementationGuide::new("TestFramework".into());
        guide.set_notes("Implementation notes here".into());
        assert_eq!(guide.implementation_notes, "Implementation notes here");
    }
}
