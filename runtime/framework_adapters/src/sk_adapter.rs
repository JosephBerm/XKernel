// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Semantic Kernel Advanced Adapter
//!
//! Extended Semantic Kernel adapter implementation for deep framework integration.
//! Provides comprehensive translation between SK Plugin/Skill architecture and CT execution model,
//! SK Planner output to CT spawner directives, and SK memory mechanisms to L2/L3 tiers.
//!
//! Sec 4.2: SK Adapter Architecture
//! Sec 4.3: SK Plugin/Skill Mapping
//! Sec 4.3: SK Planner Translation
//! Sec 4.3: SK Memory Mapping

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::{
    adapter::{CognitiveTaskConfig, IFrameworkAdapter, SemanticChannelConfig, SemanticMemoryConfig, 
              ToolBindingConfig, TranslationResult},
    framework_type::FrameworkType,
    error::AdapterError,
    AdapterResult,
};

/// Semantic Kernel plugin or skill definition.
/// Sec 4.3: SK Plugin/Skill Concept
#[derive(Debug, Clone)]
pub struct SkPlugin {
    /// Unique plugin identifier
    pub plugin_id: String,
    /// Plugin name
    pub name: String,
    /// Plugin description
    pub description: String,
    /// Plugin function definitions (skill name -> SkFunction)
    pub functions: BTreeMap<String, SkFunction>,
}

impl SkPlugin {
    /// Creates a new SK plugin.
    pub fn new(plugin_id: String, name: String) -> Self {
        SkPlugin {
            plugin_id,
            name,
            description: String::new(),
            functions: BTreeMap::new(),
        }
    }

    /// Adds a function to the plugin.
    pub fn add_function(&mut self, name: String, function: SkFunction) {
        self.functions.insert(name, function);
    }
}

/// Semantic Kernel function/skill definition.
/// Sec 4.3: SK Function Specification
#[derive(Debug, Clone)]
pub struct SkFunction {
    /// Function name within the plugin
    pub name: String,
    /// Function description and intent
    pub description: String,
    /// Input parameter schema (JSON schema format)
    pub input_schema: String,
    /// Output parameter schema (JSON schema format)
    pub output_schema: String,
    /// Whether function execution requires authorization
    pub requires_auth: bool,
}

impl SkFunction {
    /// Creates a new SK function.
    pub fn new(name: String) -> Self {
        SkFunction {
            name,
            description: String::new(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_auth: false,
        }
    }
}

/// Semantic Kernel planner step/action.
/// Sec 4.3: SK Planner Output
#[derive(Debug, Clone)]
pub struct SkPlanStep {
    /// Step identifier
    pub step_id: String,
    /// Target plugin and function (format: "PluginName.FunctionName")
    pub function_ref: String,
    /// Input parameters for the function
    pub inputs: BTreeMap<String, String>,
    /// Dependencies on other steps (step IDs)
    pub dependencies: Vec<String>,
    /// Optional conditional predicate
    pub condition: Option<String>,
}

impl SkPlanStep {
    /// Creates a new SK plan step.
    pub fn new(step_id: String, function_ref: String) -> Self {
        SkPlanStep {
            step_id,
            function_ref,
            inputs: BTreeMap::new(),
            dependencies: Vec::new(),
            condition: None,
        }
    }
}

/// Semantic Kernel plan output representation.
/// Sec 4.3: SK Plan Definition
#[derive(Debug, Clone)]
pub struct SkPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Ordered list of steps to execute
    pub steps: Vec<SkPlanStep>,
    /// Initial inputs to the plan
    pub initial_inputs: BTreeMap<String, String>,
    /// Expected outputs
    pub expected_outputs: Vec<String>,
}

impl SkPlan {
    /// Creates a new SK plan.
    pub fn new(plan_id: String) -> Self {
        SkPlan {
            plan_id,
            steps: Vec::new(),
            initial_inputs: BTreeMap::new(),
            expected_outputs: Vec::new(),
        }
    }

    /// Adds a step to the plan.
    pub fn add_step(&mut self, step: SkPlanStep) {
        self.steps.push(step);
    }
}

/// Semantic Kernel memory buffer concept.
/// Sec 4.3: SK Memory Buffers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkMemoryBufferType {
    /// Volatile short-term conversation history
    ShortTerm,
    /// Long-term persistent knowledge base
    LongTerm,
    /// Semantic embeddings and vector store
    Semantic,
}

impl SkMemoryBufferType {
    /// Returns string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SkMemoryBufferType::ShortTerm => "short_term",
            SkMemoryBufferType::LongTerm => "long_term",
            SkMemoryBufferType::Semantic => "semantic",
        }
    }
}

/// Semantic Kernel kernel memory definition.
/// Sec 4.3: SK Kernel Memory
#[derive(Debug, Clone)]
pub struct SkKernelMemory {
    /// Memory buffer identifier
    pub buffer_id: String,
    /// Buffer type classification
    pub buffer_type: SkMemoryBufferType,
    /// Capacity in tokens
    pub capacity_tokens: u64,
    /// Whether buffer data persists across sessions
    pub is_persistent: bool,
}

impl SkKernelMemory {
    /// Creates a new SK kernel memory buffer.
    pub fn new(buffer_id: String, buffer_type: SkMemoryBufferType) -> Self {
        SkKernelMemory {
            buffer_id,
            buffer_type,
            capacity_tokens: 50000,
            is_persistent: false,
        }
    }
}

/// Semantic Kernel context variables.
/// Sec 4.3: SK Context Variables
#[derive(Debug, Clone)]
pub struct SkContextVariables {
    /// Variable mappings (name -> value)
    pub variables: BTreeMap<String, String>,
}

impl SkContextVariables {
    /// Creates new context variables.
    pub fn new() -> Self {
        SkContextVariables {
            variables: BTreeMap::new(),
        }
    }

    /// Sets a context variable.
    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Gets a context variable.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}

impl Default for SkContextVariables {
    fn default() -> Self {
        Self::new()
    }
}

/// Semantic Kernel framework adapter implementation.
/// Sec 4.2: SK Adapter Implementation
#[derive(Debug, Clone)]
pub struct SemanticKernelAdvancedAdapter {
    /// Minimum supported SK version
    min_version: String,
    /// Maximum supported SK version
    max_version: String,
    /// Loaded plugins registry (plugin_id -> SkPlugin)
    loaded_plugins: BTreeMap<String, SkPlugin>,
}

impl SemanticKernelAdvancedAdapter {
    /// Creates a new advanced SK adapter instance.
    /// Sec 4.2: Adapter Initialization
    pub fn new() -> Self {
        SemanticKernelAdvancedAdapter {
            min_version: "0.1.0".to_string(),
            max_version: "1.5.x".to_string(),
            loaded_plugins: BTreeMap::new(),
        }
    }

    /// Registers a plugin in the adapter.
    /// Sec 4.3: Plugin Registration
    pub fn register_plugin(&mut self, plugin: SkPlugin) {
        self.loaded_plugins.insert(plugin.plugin_id.clone(), plugin);
    }

    /// Translates SK Plugin/Skill to ToolBinding.
    /// Sec 4.3: Plugin Translation
    fn translate_plugin_to_tool(&self, plugin: &SkPlugin) -> AdapterResult<ToolBindingConfig> {
        // Aggregate all functions in the plugin as inputs to the tool binding
        let function_names: Vec<String> = plugin.functions.keys().cloned().collect();
        let functions_str = alloc::format!("{:?}", function_names);

        Ok(ToolBindingConfig {
            tool_id: alloc::format!("sk-plugin-{}", plugin.plugin_id),
            name: plugin.name.clone(),
            description: alloc::format!("{}\nFunctions: {}", plugin.description, functions_str),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    /// Translates SK Planner output to CognitiveTask spawner directives.
    /// Sec 4.3: Planner Translation
    pub fn translate_plan_to_spawner(&self, plan: &SkPlan) -> AdapterResult<CtSpawnerDirective> {
        // Create a spawner directive with tasks for each plan step
        let mut spawner = CtSpawnerDirective::new(alloc::format!("sk-spawner-{}", plan.plan_id));
        
        // Build task list from plan steps
        for step in &plan.steps {
            let task = CtSpawnTask {
                task_id: step.step_id.clone(),
                function_ref: step.function_ref.clone(),
                inputs: step.inputs.clone(),
                dependencies: step.dependencies.clone(),
            };
            spawner.add_task(task);
        }

        Ok(spawner)
    }

    /// Maps SK Kernel Memory to L2/L3 tiers.
    /// Sec 4.3: Memory Tier Mapping
    pub fn map_kernel_memory_to_tiers(&self, memory: &SkKernelMemory) -> AdapterResult<MemoryTierMapping> {
        let (target_tier, persistence) = match memory.buffer_type {
            SkMemoryBufferType::ShortTerm => {
                // Volatile short-term → L2 episodic snapshots
                ("L2_episodic".to_string(), "transient".to_string())
            }
            SkMemoryBufferType::LongTerm => {
                // Persistent long-term → L3 semantic storage
                ("L3_semantic".to_string(), "permanent".to_string())
            }
            SkMemoryBufferType::Semantic => {
                // Vector semantics → L3 with semantic indexing
                ("L3_semantic_indexed".to_string(), "permanent".to_string())
            }
        };

        Ok(MemoryTierMapping {
            memory_id: memory.buffer_id.clone(),
            source_buffer_type: memory.buffer_type.as_str().to_string(),
            target_tier,
            capacity_tokens: memory.capacity_tokens,
            persistence_policy: persistence,
        })
    }
}

impl Default for SemanticKernelAdvancedAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFrameworkAdapter for SemanticKernelAdvancedAdapter {
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig> {
        if framework_task.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty framework task definition".to_string(),
            ));
        }

        // Parse as plan definition
        let task_id = alloc::format!("sk-plan-{}", ulid::Ulid::new());
        Ok(CognitiveTaskConfig {
            task_id,
            name: "SemanticKernelPlan".to_string(),
            objective: framework_task.to_string(),
            timeout_ms: 60000,
            is_mandatory: false,
        })
    }

    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult> {
        Ok(TranslationResult {
            artifact_id: task_id.to_string(),
            artifact_type: "semantic_kernel_result".to_string(),
            success: true,
            fidelity: "partial".to_string(),
            translation_notes: "SK plan result with approximate fidelity translation".to_string(),
        })
    }

    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig> {
        if framework_memory.is_empty() {
            return Err(AdapterError::MemoryMappingError(
                "Empty memory definition".to_string(),
            ));
        }

        let memory_id = alloc::format!("sk-mem-{}", ulid::Ulid::new());
        Ok(SemanticMemoryConfig {
            memory_id,
            memory_type: "kernel_memory".to_string(),
            capacity_tokens: 50000,
            serialization_format: "json".to_string(),
        })
    }

    fn map_tool(&self, framework_tool: &str) -> AdapterResult<ToolBindingConfig> {
        if framework_tool.is_empty() {
            return Err(AdapterError::ToolBindingError(
                "Empty tool definition".to_string(),
            ));
        }

        let tool_id = alloc::format!("sk-tool-{}", ulid::Ulid::new());
        Ok(ToolBindingConfig {
            tool_id,
            name: "SemanticKernelSkill".to_string(),
            description: framework_tool.to_string(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    fn map_channel(&self, framework_comm: &str) -> AdapterResult<SemanticChannelConfig> {
        if framework_comm.is_empty() {
            return Err(AdapterError::ChannelMappingError(
                "Empty communication definition".to_string(),
            ));
        }

        let channel_id = alloc::format!("sk-ch-{}", ulid::Ulid::new());
        Ok(SemanticChannelConfig {
            channel_id,
            name: "SemanticKernelChannel".to_string(),
            participants: "kernel".to_string(),
            communication_pattern: "request-reply".to_string(),
            is_persistent: false,
        })
    }

    fn framework_type(&self) -> FrameworkType {
        FrameworkType::SemanticKernel
    }

    fn supported_versions(&self) -> &str {
        ">=0.1.0,<2.0.0"
    }

    fn is_compatible(&self, _artifact: &str) -> bool {
        // In production, validate schema and structure
        true
    }
}

/// Cognitive Task spawner directive for plan execution.
/// Sec 4.3: CT Spawner Model
#[derive(Debug, Clone)]
pub struct CtSpawnerDirective {
    /// Spawner identifier
    pub spawner_id: String,
    /// Tasks to spawn and execute
    pub tasks: Vec<CtSpawnTask>,
}

impl CtSpawnerDirective {
    /// Creates a new spawner directive.
    pub fn new(spawner_id: String) -> Self {
        CtSpawnerDirective {
            spawner_id,
            tasks: Vec::new(),
        }
    }

    /// Adds a task to the spawner.
    pub fn add_task(&mut self, task: CtSpawnTask) {
        self.tasks.push(task);
    }
}

/// Individual task in a spawner directive.
/// Sec 4.3: CT Spawn Task
#[derive(Debug, Clone)]
pub struct CtSpawnTask {
    /// Task identifier
    pub task_id: String,
    /// Function reference (PluginName.FunctionName)
    pub function_ref: String,
    /// Input parameters
    pub inputs: BTreeMap<String, String>,
    /// Task dependencies (task IDs)
    pub dependencies: Vec<String>,
}

/// Memory tier mapping from SK to CT architecture.
/// Sec 4.3: Memory Tier Mapping
#[derive(Debug, Clone)]
pub struct MemoryTierMapping {
    /// Original SK memory buffer ID
    pub memory_id: String,
    /// Source SK buffer type
    pub source_buffer_type: String,
    /// Target CT memory tier (L2/L3)
    pub target_tier: String,
    /// Capacity in tokens
    pub capacity_tokens: u64,
    /// Persistence policy in CT model
    pub persistence_policy: String,
}

#[cfg(test)]
mod tests {
    use super::*;
use ulid::Ulid;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_sk_plugin_creation() {
        let plugin = SkPlugin::new("plugin-1".into(), "TestPlugin".into());
        assert_eq!(plugin.plugin_id, "plugin-1");
        assert_eq!(plugin.name, "TestPlugin");
        assert!(plugin.functions.is_empty());
    }

    #[test]
    fn test_sk_function_creation() {
        let func = SkFunction::new("analyze".into());
        assert_eq!(func.name, "analyze");
        assert!(!func.requires_auth);
    }

    #[test]
    fn test_sk_plan_step_creation() {
        let step = SkPlanStep::new("step-1".into(), "Plugin.Function".into());
        assert_eq!(step.step_id, "step-1");
        assert_eq!(step.function_ref, "Plugin.Function");
        assert!(step.dependencies.is_empty());
    }

    #[test]
    fn test_sk_plan_creation() {
        let plan = SkPlan::new("plan-1".into());
        assert_eq!(plan.plan_id, "plan-1");
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_sk_kernel_memory_creation() {
        let mem = SkKernelMemory::new("mem-1".into(), SkMemoryBufferType::ShortTerm);
        assert_eq!(mem.buffer_id, "mem-1");
        assert_eq!(mem.buffer_type, SkMemoryBufferType::ShortTerm);
        assert!(!mem.is_persistent);
    }

    #[test]
    fn test_sk_context_variables() {
        let mut ctx = SkContextVariables::new();
        ctx.set("key1".into(), "value1".into());
        assert_eq!(ctx.get("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.get("nonexistent"), None);
    }

    #[test]
    fn test_advanced_adapter_creation() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        assert_eq!(adapter.framework_type(), FrameworkType::SemanticKernel);
        assert!(adapter.loaded_plugins.is_empty());
    }

    #[test]
    fn test_plugin_registration() {
        let mut adapter = SemanticKernelAdvancedAdapter::new();
        let plugin = SkPlugin::new("plugin-1".into(), "TestPlugin".into());
        adapter.register_plugin(plugin);
        assert_eq!(adapter.loaded_plugins.len(), 1);
    }

    #[test]
    fn test_plugin_translation_to_tool() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        let mut plugin = SkPlugin::new("plugin-1".into(), "TestPlugin".into());
        plugin.add_function("func1".into(), SkFunction::new("func1".into()));
        
        let result = adapter.translate_plugin_to_tool(&plugin);
        assert!(result.is_ok());
        let tool = result.unwrap();
        assert_eq!(tool.name, "TestPlugin");
    }

    #[test]
    fn test_plan_to_spawner_translation() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        let mut plan = SkPlan::new("plan-1".into());
        let step = SkPlanStep::new("step-1".into(), "Plugin.Func".into());
        plan.add_step(step);

        let result = adapter.translate_plan_to_spawner(&plan);
        assert!(result.is_ok());
        let spawner = result.unwrap();
        assert_eq!(spawner.tasks.len(), 1);
    }

    #[test]
    fn test_kernel_memory_to_tier_mapping() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        
        // Test short-term to L2
        let short_term = SkKernelMemory::new("mem-1".into(), SkMemoryBufferType::ShortTerm);
        let result = adapter.map_kernel_memory_to_tiers(&short_term);
        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.target_tier, "L2_episodic");
        assert_eq!(mapping.persistence_policy, "transient");

        // Test long-term to L3
        let long_term = SkKernelMemory::new("mem-2".into(), SkMemoryBufferType::LongTerm);
        let result = adapter.map_kernel_memory_to_tiers(&long_term);
        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.target_tier, "L3_semantic");
        assert_eq!(mapping.persistence_policy, "permanent");
    }

    #[test]
    fn test_ct_spawner_directive_creation() {
        let mut spawner = CtSpawnerDirective::new("spawner-1".into());
        let task = CtSpawnTask {
            task_id: "task-1".into(),
            function_ref: "Plugin.Func".into(),
            inputs: BTreeMap::new(),
            dependencies: Vec::new(),
        };
        spawner.add_task(task);
        assert_eq!(spawner.tasks.len(), 1);
    }

    #[test]
    fn test_advanced_adapter_translate_to_ct() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        let result = adapter.translate_to_ct("test plan definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout_ms, 60000);
    }

    #[test]
    fn test_advanced_adapter_map_memory() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        let result = adapter.map_memory("kernel memory");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.memory_type, "kernel_memory");
    }

    #[test]
    fn test_advanced_adapter_empty_input_error() {
        let adapter = SemanticKernelAdvancedAdapter::new();
        assert!(adapter.translate_to_ct("").is_err());
        assert!(adapter.map_memory("").is_err());
        assert!(adapter.map_tool("").is_err());
        assert!(adapter.map_channel("").is_err());
    }
}
